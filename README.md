# QuickLaunch (Rust + Tauri v2)

Voraussetzungen (einmalig):
1. Rust: https://rustup.rs  (rustup-init.exe, Standard-Einstellungen)
2. Visual Studio Build Tools mit "Desktopentwicklung mit C++" (die rustup-Installation fragt danach)
3. WebView2 ist unter Windows 10/11 schon vorhanden

Bauen und starten: BUILD.bat doppelklicken.
Das fertige Programm liegt danach in %LOCALAPPDATA%\QuickLaunch\quicklaunch.exe und startet mit Windows.

Index: Beim Start wird der Cache (%APPDATA%\me.emincviko.quicklaunch\files.tsv) geladen,
im Hintergrund neu gescannt und per Dateisystem-Watcher live aktualisiert.
