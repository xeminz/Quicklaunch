#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;
use std::fs;
use std::io::{BufRead, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use notify::{EventKind, RecursiveMode, Watcher};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const NO_WINDOW: u32 = 0x0800_0000;
const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 200_000;
const WIN_WIDTH: f64 = 700.0;
const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = "xeminz/Quicklaunch";

// ---------- Index (im Speicher, mit Cache auf der Festplatte) ----------

struct Meta {
    lower: String,
    dir: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct AppEntry {
    name: String,
    id: String,
}

#[derive(Default)]
struct Index {
    files: HashMap<String, Meta>, // Pfad -> Meta
    apps: Vec<AppEntry>,
    dirty: bool,
}

struct Shared {
    index: Mutex<Index>,
    icons: Mutex<HashMap<String, String>>,
    update: Mutex<Option<UpdateInfo>>,
    last_check: Mutex<Option<Instant>>,
    pending: AtomicBool,
    closing: AtomicBool,
}

fn lock(sh: &Shared) -> MutexGuard<'_, Index> {
    sh.index.lock().unwrap_or_else(|e| e.into_inner())
}

fn bad_name(n: &str) -> bool {
    n.starts_with('.') || n == "node_modules" || n == "AppData" || n == "$RECYCLE.BIN"
}

fn skipped(p: &Path) -> bool {
    p.components().any(|c| bad_name(&c.as_os_str().to_string_lossy()))
}

fn entry(p: &Path, dir: bool) -> Option<(String, Meta)> {
    let name = p.file_name()?.to_string_lossy().to_lowercase();
    Some((p.to_string_lossy().to_string(), Meta { lower: name, dir }))
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<(String, Meta)>) {
    if depth > MAX_DEPTH || out.len() > MAX_ENTRIES {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if bad_name(&e.file_name().to_string_lossy()) {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        let p = e.path();
        if ft.is_dir() {
            if let Some(x) = entry(&p, true) {
                out.push(x);
            }
            walk(&p, depth + 1, out);
        } else if ft.is_file() {
            if let Some(x) = entry(&p, false) {
                out.push(x);
            }
        }
    }
}

fn known_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = [
        dirs::desktop_dir(),
        dirs::document_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
        dirs::video_dir(),
        dirs::audio_dir(),
        dirs::home_dir().map(|h| h.join("OneDrive")),
    ]
    .into_iter()
    .flatten()
    .filter(|p| p.exists())
    .collect();
    v.sort();
    v.dedup();
    v
}

// nur die obersten Ordner scannen (keine doppelte Arbeit, wenn z. B. Desktop in OneDrive liegt)
fn top_level(all: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in all {
        let inside = all.iter().any(|q| q != p && p.starts_with(q));
        if !inside {
            out.push(p.clone());
        }
    }
    out
}

fn full_scan() -> HashMap<String, Meta> {
    let all = known_roots();
    let mut out = Vec::new();
    for r in &all {
        if let Some(x) = entry(r, true) {
            out.push(x);
        }
    }
    for r in top_level(&all) {
        walk(&r, 0, &mut out);
    }
    out.into_iter().collect()
}

fn load_apps() -> Vec<AppEntry> {
    let cmd = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-StartApps | ConvertTo-Json";
    let Ok(out) = Command::new("powershell")
        .args(["-NoProfile", "-Command", cmd])
        .creation_flags(NO_WINDOW)
        .output()
    else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}')) else {
        return vec![];
    };
    let items = match v {
        serde_json::Value::Array(a) => a,
        o @ serde_json::Value::Object(_) => vec![o],
        _ => vec![],
    };
    items
        .iter()
        .filter_map(|x| {
            Some(AppEntry {
                name: x.get("Name")?.as_str()?.to_string(),
                id: x.get("AppID")?.as_str()?.to_string(),
            })
        })
        .collect()
}

fn snapshot(i: &Index) -> (String, String) {
    let mut tsv = String::with_capacity(i.files.len() * 80);
    for (p, m) in &i.files {
        tsv.push(if m.dir { 'D' } else { 'F' });
        tsv.push('\t');
        tsv.push_str(p);
        tsv.push('\n');
    }
    (tsv, serde_json::to_string(&i.apps).unwrap_or_default())
}

fn write_cache(dir: &Path, tsv: &str, apps: &str) {
    let _ = fs::create_dir_all(dir);
    let tmp = dir.join("files.tsv.tmp");
    if fs::write(&tmp, tsv).is_ok() {
        let _ = fs::rename(&tmp, dir.join("files.tsv"));
    }
    let _ = fs::write(dir.join("apps.json"), apps);
}

fn load_cache(dir: &Path, i: &mut Index) {
    if let Ok(f) = fs::File::open(dir.join("files.tsv")) {
        for line in std::io::BufReader::new(f).lines().flatten() {
            if let Some((k, p)) = line.split_once('\t') {
                if let Some((key, m)) = entry(Path::new(p), k == "D") {
                    i.files.insert(key, m);
                }
            }
        }
    }
    if let Ok(s) = fs::read_to_string(dir.join("apps.json")) {
        if let Ok(a) = serde_json::from_str::<Vec<AppEntry>>(&s) {
            i.apps = a;
        }
    }
}

fn spawn_background(sh: Arc<Shared>, dir: PathBuf) {
    // Apps: beim Start und danach alle 10 Minuten
    let s = sh.clone();
    std::thread::spawn(move || loop {
        let apps = load_apps();
        if !apps.is_empty() {
            let mut i = lock(&s);
            i.apps = apps;
            i.dirty = true;
        }
        std::thread::sleep(Duration::from_secs(600));
    });
    // Dateien: Vollscan im Hintergrund beim Start, danach alle 6 Stunden zur Sicherheit
    let s = sh.clone();
    std::thread::spawn(move || loop {
        let files = full_scan();
        {
            let mut i = lock(&s);
            i.files = files;
            i.dirty = true;
        }
        std::thread::sleep(Duration::from_secs(6 * 3600));
    });
    // Cache auf die Festplatte schreiben, wenn sich etwas geaendert hat
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        let snap = {
            let mut i = lock(&sh);
            if i.dirty {
                i.dirty = false;
                Some(snapshot(&i))
            } else {
                None
            }
        };
        if let Some((t, a)) = snap {
            write_cache(&dir, &t, &a);
        }
    });
}

// Live-Updates: neue, geloeschte und umbenannte Dateien werden sofort im Index nachgetragen
fn start_watcher(sh: Arc<Shared>, roots: Vec<PathBuf>) {
    std::thread::spawn(move || {
        let handler = move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            if !matches!(
                ev.kind,
                EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(notify::event::ModifyKind::Name(_))
            ) {
                return;
            }
            let mut adds: Vec<(String, Meta)> = Vec::new();
            let mut gone: Vec<String> = Vec::new();
            for p in &ev.paths {
                if skipped(p) {
                    continue;
                }
                if p.exists() {
                    let dir = p.is_dir();
                    if let Some(x) = entry(p, dir) {
                        adds.push(x);
                    }
                    if dir {
                        walk(p, MAX_DEPTH - 3, &mut adds);
                    }
                } else {
                    gone.push(p.to_string_lossy().to_string());
                }
            }
            let mut i = lock(&sh);
            for k in gone {
                let was_dir = i.files.remove(&k).map(|m| m.dir).unwrap_or(false);
                if was_dir {
                    let pre = format!("{}\\", k);
                    i.files.retain(|x, _| !x.starts_with(&pre));
                }
            }
            for (k, m) in adds {
                i.files.insert(k, m);
            }
            i.dirty = true;
        };
        let Ok(mut w) = notify::recommended_watcher(handler) else { return };
        for r in &roots {
            let _ = w.watch(r, RecursiveMode::Recursive);
        }
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    });
}

// ---------- Befehle fuer die Oberflaeche ----------

#[derive(Serialize)]
struct Item {
    kind: String,
    tag: String,
    name: String,
    sub: String,
    id: Option<String>,
    path: Option<String>,
    s: f32,
}

fn name_of(p: &str) -> String {
    Path::new(p)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string())
}

fn parent_of(p: &str) -> String {
    Path::new(p)
        .parent()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command(async)]
fn search(app: AppHandle, q: String, mode: String) -> Vec<Item> {
    let sh = app.state::<Arc<Shared>>();
    let q = q.to_lowercase();
    let sc = |n: &str| -> f32 {
        if q.is_empty() {
            1.0
        } else if n.starts_with(&q) {
            4.0
        } else if n.contains(&q) {
            3.0
        } else {
            0.0
        }
    };
    let idx = lock(&sh);
    let mut out: Vec<Item> = Vec::new();
    if mode != "#" {
        for a in &idx.apps {
            let s = sc(&a.name.to_lowercase());
            if s > 0.0 {
                out.push(Item {
                    kind: "app".into(),
                    tag: "App".into(),
                    name: a.name.clone(),
                    sub: "App".into(),
                    id: Some(a.id.clone()),
                    path: None,
                    s: s + 0.1,
                });
            }
        }
    }
    if mode != "@" && q.chars().count() > 1 {
        let mut hits: Vec<(f32, usize, &String, bool)> = Vec::new();
        for (p, m) in idx.files.iter() {
            let s = sc(&m.lower);
            if s > 0.0 {
                hits.push((s + if m.dir { 0.3 } else { 0.0 }, m.lower.len(), p, m.dir));
            }
        }
        hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        for (s, _, p, dir) in hits.into_iter().take(8) {
            out.push(Item {
                kind: String::from(if dir { "folder" } else { "file" }),
                tag: String::from(if dir { "Ordner" } else { "Datei" }),
                name: name_of(p),
                sub: parent_of(p),
                id: None,
                path: Some(p.clone()),
                s,
            });
        }
    }
    out.sort_by(|a, b| b.s.partial_cmp(&a.s).unwrap().then(a.name.len().cmp(&b.name.len())));
    out.truncate(8);
    out
}

#[derive(Deserialize)]
struct LaunchReq {
    kind: String,
    id: Option<String>,
    path: Option<String>,
}

fn explorer(arg: &str) {
    let _ = Command::new("explorer.exe").arg(arg).creation_flags(NO_WINDOW).spawn();
}

#[tauri::command(async)]
fn launch(item: LaunchReq) {
    match item.kind.as_str() {
        "app" => {
            if let Some(id) = item.id {
                explorer(&format!("shell:AppsFolder\\{}", id));
            }
        }
        "file" | "folder" => {
            if let Some(p) = item.path {
                explorer(&p);
            }
        }
        _ => {}
    }
}

#[tauri::command(async)]
fn open_url(url: String) {
    if url.starts_with("https://") || url.starts_with("http://") {
        explorer(&url);
    }
}

fn urlencode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{:02X}", b)),
        }
    }
    o
}

#[tauri::command(async)]
fn suggest(q: String) -> Vec<String> {
    let url = format!(
        "https://suggestqueries.google.com/complete/search?client=firefox&hl=de&q={}",
        urlencode(&q)
    );
    let Ok(resp) = ureq::get(&url).timeout(Duration::from_millis(2500)).call() else {
        return vec![];
    };
    let mut buf = Vec::new();
    if resp.into_reader().take(1_000_000).read_to_end(&mut buf).is_err() {
        return vec![];
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![];
    };
    v.get(1)
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).take(5).collect())
        .unwrap_or_default()
}

// ---------- Updates ueber GitHub-Releases ----------

#[derive(Clone, Serialize)]
struct UpdateInfo {
    version: String,
    url: String,
    sha_url: String,
}

fn parse_ver(s: &str) -> Vec<u32> {
    s.trim_start_matches('v')
        .split('.')
        .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
        .collect()
}

fn newer(a: &str, b: &str) -> bool {
    let (mut x, mut y) = (parse_ver(a), parse_ver(b));
    let n = x.len().max(y.len());
    x.resize(n, 0);
    y.resize(n, 0);
    x > y
}

// None = Netzwerkfehler, Some(None) = kein Update, Some(Some(..)) = Update verfuegbar
fn fetch_update() -> Option<Option<UpdateInfo>> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let resp = match ureq::get(&url)
        .set("User-Agent", "QuickLaunch")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(6))
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => return Some(None), // noch kein Release
        Err(_) => return None,
    };
    let v: serde_json::Value = serde_json::from_reader(resp.into_reader().take(2_000_000)).ok()?;
    let tag = v.get("tag_name")?.as_str()?;
    let clean: String = tag
        .trim_start_matches('v')
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
        .collect();
    if !newer(&clean, VERSION) {
        return Some(None);
    }
    let assets = v.get("assets")?.as_array()?;
    let find = |name: &str| -> Option<String> {
        assets
            .iter()
            .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(name))
            .and_then(|a| a.get("browser_download_url"))
            .and_then(|u| u.as_str())
            .map(String::from)
    };
    match (find("quicklaunch.exe"), find("quicklaunch.exe.sha256")) {
        (Some(u), Some(s)) => Some(Some(UpdateInfo { version: clean, url: u, sha_url: s })),
        _ => Some(None),
    }
}

#[tauri::command]
fn version() -> String {
    VERSION.to_string()
}

#[tauri::command(async)]
fn check_update(app: AppHandle, force: bool) -> Option<UpdateInfo> {
    let sh = app.state::<Arc<Shared>>();
    if !force {
        let last = *sh.last_check.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = last {
            if t.elapsed() < Duration::from_secs(86400) {
                return sh.update.lock().unwrap_or_else(|e| e.into_inner()).clone();
            }
        }
    }
    let found = fetch_update()?;
    *sh.last_check.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
    *sh.update.lock().unwrap_or_else(|e| e.into_inner()) = found.clone();
    found
}

fn download(url: &str, max: u64) -> Result<Vec<u8>, String> {
    let r = ureq::get(url)
        .set("User-Agent", "QuickLaunch")
        .timeout(Duration::from_secs(120))
        .call()
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    r.into_reader().take(max).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command(async)]
fn install_update(app: AppHandle) -> Result<(), String> {
    let sh = app.state::<Arc<Shared>>();
    let info = sh.update.lock().unwrap_or_else(|e| e.into_inner()).clone().ok_or("Kein Update bekannt")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytes = download(&info.url, 150_000_000)?;
    let sha_txt = String::from_utf8_lossy(&download(&info.sha_url, 4096)?).to_string();
    let want: String = sha_txt.trim().chars().take(64).collect::<String>().to_lowercase();
    let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{:02x}", b)).collect();
    if want != got {
        return Err("Pruefsumme stimmt nicht".into());
    }
    if bytes.len() < 1_000_000 || &bytes[..2] != b"MZ" {
        return Err("Keine gueltige Programmdatei".into());
    }
    let new = exe.with_file_name("quicklaunch.new");
    let old = exe.with_file_name("quicklaunch.old.exe");
    fs::write(&new, &bytes).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&old);
    // Eine laufende exe kann man umbenennen, aber nicht ueberschreiben
    fs::rename(&exe, &old).map_err(|e| e.to_string())?;
    if let Err(e) = fs::rename(&new, &exe) {
        let _ = fs::rename(&old, &exe);
        return Err(e.to_string());
    }
    // Die neue Version startet erst, wenn dieses Programm beendet ist (sonst greift die Einzelinstanz-Sperre)
    let cmd = format!("/C \"ping -n 3 127.0.0.1 >nul & start \"\" \"{}\"\"", exe.display());
    let _ = Command::new("cmd").raw_arg(cmd).creation_flags(NO_WINDOW).spawn();
    app.exit(0);
    Ok(())
}

// ---------- Original-Icons (wie im Windows-Startmenue) ----------

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let n = ((c[0] as u32) << 16) | ((*c.get(1).unwrap_or(&0) as u32) << 8) | (*c.get(2).unwrap_or(&0) as u32);
        o.push(T[((n >> 18) & 63) as usize] as char);
        o.push(T[((n >> 12) & 63) as usize] as char);
        o.push(if c.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        o.push(if c.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    o
}

fn fnv(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// Holt das Icon direkt von der Windows-Shell: funktioniert fuer klassische Programme,
// Store-Apps ("shell:AppsFolder\...") sowie fuer Dateien und Ordner.
fn shell_icon_png(parsing_name: &str) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, SIZE};
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::System::Com::{CoInitializeEx, IBindCtx, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let wide: Vec<u16> = parsing_name.encode_utf16().chain(std::iter::once(0)).collect();
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None::<&IBindCtx>).ok()?;
        let hbm = factory
            .GetImage(SIZE { cx: 64, cy: 64 }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .ok()?;
        let mut bm = BITMAP::default();
        if GetObjectW(HGDIOBJ(hbm.0), size_of::<BITMAP>() as i32, Some(&mut bm as *mut BITMAP as *mut c_void)) == 0 {
            let _ = DeleteObject(HGDIOBJ(hbm.0));
            return None;
        }
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let hdc = GetDC(HWND::default());
        let lines = GetDIBits(hdc, hbm, 0, h as u32, Some(buf.as_mut_ptr() as *mut c_void), &mut bi, DIB_RGB_COLORS);
        ReleaseDC(HWND::default(), hdc);
        let _ = DeleteObject(HGDIOBJ(hbm.0));
        if lines == 0 {
            return None;
        }
        for p in buf.chunks_exact_mut(4) {
            p.swap(0, 2); // BGRA -> RGBA
        }
        if buf.chunks_exact(4).all(|p| p[3] == 0) {
            for p in buf.chunks_exact_mut(4) {
                p[3] = 255; // Bild ohne Alpha-Kanal
            }
        }
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().ok()?;
            wr.write_image_data(&buf).ok()?;
        }
        Some(out)
    }
}

#[tauri::command(async)]
fn icon(app: AppHandle, key: String, persist: bool) -> Option<String> {
    let sh = app.state::<Arc<Shared>>();
    if let Some(v) = sh.icons.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Some(v.clone());
    }
    let dir = app.path().app_data_dir().ok()?.join("icons");
    let file = dir.join(format!("{:016x}.png", fnv(&key)));
    let cached = if persist { fs::read(&file).ok() } else { None };
    let png = match cached {
        Some(p) => p,
        None => {
            let p = shell_icon_png(&key)?;
            if persist {
                let _ = fs::create_dir_all(&dir);
                let _ = fs::write(&file, &p);
            }
            p
        }
    };
    let url = format!("data:image/png;base64,{}", b64(&png));
    let mut m = sh.icons.lock().unwrap_or_else(|e| e.into_inner());
    if m.len() > 400 {
        m.clear();
    }
    m.insert(key, url.clone());
    Some(url)
}

// ---------- Fenster: vorbereiten -> anzeigen -> ausblenden ----------

fn position(w: &WebviewWindow) {
    let monitors = w.available_monitors().unwrap_or_default();
    let cursor = w.cursor_position().ok();
    let mon = cursor
        .and_then(|c| {
            monitors
                .iter()
                .find(|m| {
                    let p = m.position();
                    let s = m.size();
                    c.x >= p.x as f64
                        && c.x < p.x as f64 + s.width as f64
                        && c.y >= p.y as f64
                        && c.y < p.y as f64 + s.height as f64
                })
                .cloned()
        })
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = mon {
        let scale = m.scale_factor();
        let p = m.position();
        let s = m.size();
        let x = p.x as f64 + (s.width as f64 - WIN_WIDTH * scale) / 2.0;
        let y = p.y as f64 + s.height as f64 * 0.15;
        let _ = w.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
}

fn show_now(app: &AppHandle) {
    let sh = app.state::<Arc<Shared>>();
    if !sh.pending.swap(false, Ordering::SeqCst) {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit("open", ());
    }
}

fn request_hide(app: &AppHandle) {
    let sh = app.state::<Arc<Shared>>();
    let Some(w) = app.get_webview_window("main") else { return };
    if !w.is_visible().unwrap_or(false) || sh.closing.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = app.emit("close", ());
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        app2.state::<Arc<Shared>>().closing.store(false, Ordering::SeqCst);
        if let Some(w) = app2.get_webview_window("main") {
            if w.is_visible().unwrap_or(false) {
                let _ = w.hide();
            }
        }
    });
}

fn toggle(app: &AppHandle) {
    let sh = app.state::<Arc<Shared>>();
    let Some(w) = app.get_webview_window("main") else { return };
    if w.is_visible().unwrap_or(false) {
        request_hide(app);
        return;
    }
    if sh.pending.swap(true, Ordering::SeqCst) {
        return;
    }
    position(&w);
    let _ = app.emit("prepare", ());
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        show_now(&app2);
    });
}

#[tauri::command]
fn ready(app: AppHandle) {
    show_now(&app);
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    app.state::<Arc<Shared>>().closing.store(false, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| toggle(app)))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![search, icon, check_update, install_update, version, launch, open_url, suggest, ready, hide_window, quit])
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                request_hide(window.app_handle());
            }
        })
        .setup(|app| {
            let cache_dir = app.path().app_data_dir()?;
            if let Ok(p) = std::env::current_exe() {
                let _ = fs::remove_file(p.with_file_name("quicklaunch.old.exe")); // Rest eines frueheren Updates
            }
            let mut idx = Index::default();
            load_cache(&cache_dir, &mut idx);
            let sh = Arc::new(Shared {
                index: Mutex::new(idx),
                icons: Mutex::new(HashMap::new()),
                update: Mutex::new(None),
                last_check: Mutex::new(None),
                pending: AtomicBool::new(false),
                closing: AtomicBool::new(false),
            });
            app.manage(sh.clone());
            spawn_background(sh.clone(), cache_dir.clone());
            start_watcher(sh.clone(), top_level(&known_roots()));

            let open_i = MenuItem::with_id(app, "open", "Öffnen (Alt+Leertaste)", true, None::<&str>)?;
            let auto_i = MenuItem::with_id(app, "auto", "Autostart an/aus", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &auto_i, &quit_i])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("QuickLaunch (Alt+Leertaste)")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => toggle(app),
                    "auto" => {
                        let m = app.autolaunch();
                        if m.is_enabled().unwrap_or(false) {
                            let _ = m.disable();
                        } else {
                            let _ = m.enable();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle(tray.app_handle());
                    }
                })
                .build(app)?;

            let primary = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            if app.global_shortcut().register(primary).is_err() {
                let _ = app
                    .global_shortcut()
                    .register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space));
            }

            // Beim allerersten Start: Autostart aktivieren und das Fenster einmal zeigen
            let flag = cache_dir.join("first-run");
            if !flag.exists() {
                let _ = fs::create_dir_all(&cache_dir);
                let _ = fs::write(&flag, "1");
                let _ = app.autolaunch().enable();
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(1500));
                    toggle(&h);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("QuickLaunch konnte nicht starten");
}
