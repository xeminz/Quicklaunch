# QuickLaunch

Ein schneller Launcher für Windows, inspiriert von der Spotlight-Suche auf dem Mac. Mit **Alt + Leertaste** öffnet sich überall ein kleines Suchfenster. Gebaut mit **Rust + Tauri v2**, dadurch braucht es sehr wenig Arbeitsspeicher.

## Was kann es?

- Apps starten
- Dateien und Ordner auf dem PC finden und öffnen
- Rechnen und im Web suchen
- Eigene Befehle, zum Beispiel mehrere Apps mit einem Befehl starten
- Startet auf Wunsch mit Windows

Mit einem Zeichen am Anfang suchst du gezielt: `>` Befehle, `@` Apps, `#` Dateien, `!` eigene Automationen.

## Installation

QuickLaunch wird auf deinem eigenen PC gebaut. Das dauert beim ersten Mal ein paar Minuten, danach ist es sofort startklar. Du brauchst Windows 10 oder 11 und mehrere GB freien Speicherplatz für die Entwickler-Tools.

### 1. Einmalig: Tools installieren

1. **Rust:** Lade `rustup-init.exe` von [rustup.rs](https://rustup.rs) herunter, starte sie und nimm die Standard-Einstellungen.
2. **Visual Studio Build Tools:** Rust fragt bei der Installation danach. Wichtig ist der Haken bei **„Desktopentwicklung mit C++"**.
3. **Neustart des Terminals:** Wenn du ein Terminal offen hattest, schließe es danach, damit es Rust kennt.
4. **WebView2** ist unter Windows 10 und 11 schon vorhanden, du musst nichts tun.

### 2. Projekt herunterladen

- **Ohne Git:** Oben auf dieser Seite auf **Code**, dann **Download ZIP** klicken und den Ordner entpacken (zum Beispiel auf den Desktop).
- **Mit Git:** `git clone https://github.com/xeminz/quicklaunch-rust.git`

### 3. Bauen und starten

1. Öffne den entpackten Ordner.
2. **`BUILD.bat` doppelklicken** und warten, bis das Fenster fertig ist. Beim ersten Mal kann es einige Minuten dauern, weil alles neu gebaut wird.
3. Das fertige Programm liegt danach hier:
   `%LOCALAPPDATA%\QuickLaunch\quicklaunch.exe`
4. Es startet automatisch und läuft ab jetzt mit Windows.

### 4. Benutzen

- **Alt + Leertaste** drücken und lostippen
- Mit den Pfeiltasten auswählen und mit **Enter** öffnen
- Beim ersten Start wird der Such-Index aufgebaut. Direkt danach kann es kurz dauern, bis alle Dateien gefunden werden.

## Hilfe bei Problemen

**Windows blockiert das Programm (Smart App Control oder SmartScreen)**
Selbst gebaute Programme sind nicht digital signiert, deshalb können Windows-Schutzfunktionen sie blockieren.
- **SmartScreen** („Der Computer wurde durch Windows geschützt"): Auf **Weitere Informationen** und dann auf **Trotzdem ausführen** klicken.
- **Smart App Control:** Das Programm wird ohne Hinweis geblockt oder startet nicht. Öffne **Windows-Sicherheit → App- & Browsersteuerung → Smart App Control-Einstellungen**. Wenn du es ausschaltest, ist der Schutz nicht mehr aktiv, und je nach Windows-Version lässt er sich nicht mehr einschalten. Prüfe das vorher.
- Falls ein Antivirus-Programm meldet, dass die Datei verdächtig ist: Es ist ein neues, unsigniertes Programm. Du kannst den Ordner `%LOCALAPPDATA%\QuickLaunch` als Ausnahme hinzufügen.

**BUILD.bat bricht ab**
- Rust und die Visual Studio Build Tools müssen installiert sein, inklusive „Desktopentwicklung mit C++".
- Fehler wie `link.exe not found` oder `cargo not found` bedeuten meistens: Tools fehlen, oder das Fenster muss einmal neu geöffnet werden.
- Prüfen kannst du es mit `rustc --version`. Wenn dort eine Versionsnummer erscheint, ist Rust da.

**Alt + Leertaste öffnet nichts**
- Läuft `quicklaunch.exe` im Task-Manager? Wenn nicht, starte sie aus `%LOCALAPPDATA%\QuickLaunch`.
- Eine andere App belegt vielleicht dieselbe Tastenkombination. Schließe sie zum Test.
- Hilft nichts: `BUILD.bat` noch einmal ausführen.

**Dateien oder Apps werden nicht gefunden**
Kurz warten, der Index aktualisiert sich im Hintergrund. Neu angelegte Dateien erscheinen nach einem Moment automatisch.

Gebaut von [xeminz](https://github.com/xeminz)
