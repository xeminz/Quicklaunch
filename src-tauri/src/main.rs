#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

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
        .invoke_handler(tauri::generate_handler![search, launch, open_url, suggest, ready, hide_window, quit])
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                request_hide(window.app_handle());
            }
        })
        .setup(|app| {
            let cache_dir = app.path().app_data_dir()?;
            let mut idx = Index::default();
            load_cache(&cache_dir, &mut idx);
            let sh = Arc::new(Shared {
                index: Mutex::new(idx),
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
