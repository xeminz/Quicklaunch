# QuickLaunch

Ein schneller Launcher für Windows. Mit **Alt + Leertaste** öffnet sich überall ein kleines Suchfenster. Gebaut mit **Rust + Tauri v2**, dadurch braucht es sehr wenig Arbeitsspeicher.

## Was kann es?

- Apps starten
- Dateien und Ordner auf dem PC finden und öffnen
- Rechnen und im Web suchen
- Eigene Befehle, zum Beispiel mehrere Apps mit einem Befehl starten
- Startet auf Wunsch mit Windows

Mit einem Zeichen am Anfang suchst du gezielt: `>` Befehle, `@` Apps, `#` Dateien, `!` eigene Automationen.

## Installation

**1. Einmalig installieren**
- Rust von [rustup.rs](https://rustup.rs) (Standard-Einstellungen)
- Visual Studio Build Tools mit **„Desktopentwicklung mit C++"** (Rust fragt danach)

**2. Herunterladen**
- Oben auf **Code** und dann **Download ZIP** klicken und entpacken

**3. Bauen und starten**
- **`BUILD.bat` doppelklicken** und warten (beim ersten Mal ein paar Minuten)
- Fertig: Das Programm liegt in `%LOCALAPPDATA%\QuickLaunch\quicklaunch.exe`, startet automatisch und läuft ab jetzt mit Windows

**4. Benutzen**
- **Alt + Leertaste** drücken und lostippen

## Hilfe

- **BUILD.bat bricht ab:** Prüfen, ob Rust und die Build Tools installiert sind.
- **Alt + Leertaste geht nicht:** `BUILD.bat` noch einmal ausführen.

Gebaut von [xeminz](https://github.com/xeminz)
