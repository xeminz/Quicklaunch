# QuickLaunch

Ein schneller Launcher für Windows, inspiriert von der Spotlight-Suche auf dem Mac. Mit **Alt + Leertaste** öffnet sich überall ein kleines Suchfenster. Gebaut mit **Rust + Tauri v2**, dadurch braucht es sehr wenig Arbeitsspeicher.

## Was kann es?

- Apps starten (mit den Original-Icons aus dem Startmenü)
- Dateien und Ordner auf dem PC finden und öffnen
- Rechnen und im Web suchen
- Eigene Befehle, zum Beispiel mehrere Apps mit einem Befehl starten
- Prüft selbst auf Updates und installiert sie mit einem Klick
- Startet auf Wunsch mit Windows

Mit einem Zeichen am Anfang suchst du gezielt: `>` Befehle, `@` Apps, `#` Dateien und Ordner, `!` eigene Automationen.

## Installation

Du hast zwei Möglichkeiten. Du brauchst Windows 10 oder 11.

### Variante A: Fertige Version herunterladen

1. Öffne die Seite [Releases](https://github.com/xeminz/Quicklaunch/releases) und lade bei der neuesten Version die Datei `quicklaunch.exe` herunter.
2. Lege sie in einen festen Ordner, zum Beispiel `%LOCALAPPDATA%\QuickLaunch`, und verschiebe sie danach nicht mehr.
3. Doppelklicke die Datei. Beim ersten Start richtet sich der Autostart ein, und das Fenster öffnet sich einmal.

Windows kann das Programm blockieren, siehe "Hilfe bei Problemen".

### Variante B: Selbst bauen

QuickLaunch wird auf deinem eigenen PC gebaut. Das dauert beim ersten Mal ein paar Minuten. Du brauchst mehrere GB freien Speicherplatz für die Entwickler-Tools.

**1. Einmalig: Tools installieren**

1. **Rust:** Lade `rustup-init.exe` von [rustup.rs](https://rustup.rs) herunter, starte sie und nimm die Standard-Einstellungen.
2. **Visual Studio Build Tools:** Rust fragt bei der Installation danach. Wichtig ist der Haken bei **"Desktopentwicklung mit C++"**.
3. Schließe danach offene Terminals, damit sie Rust kennen.
4. **WebView2** ist unter Windows 10 und 11 schon vorhanden.

**2. Projekt herunterladen**

- **Ohne Git:** Oben auf der GitHub-Seite auf **Code**, dann **Download ZIP** klicken und entpacken.
- **Mit Git:** `git clone https://github.com/xeminz/Quicklaunch.git`

**3. Bauen und starten**

1. Öffne den Projektordner und doppelklicke **`BUILD.bat`**.
2. Das fertige Programm liegt danach in `%LOCALAPPDATA%\QuickLaunch\quicklaunch.exe`. Es startet automatisch und läuft ab jetzt mit Windows.

## Benutzen

- **Alt + Leertaste** drücken und lostippen
- Mit den Pfeiltasten auswählen und mit **Enter** öffnen
- Mit `!neu` legst du eigene Automationen an, mit `exit` beendest du QuickLaunch
- Beim ersten Start wird der Such-Index aufgebaut. Direkt danach kann es kurz dauern, bis alle Dateien gefunden werden.

## Updates

Unten rechts im Fenster steht die Versionsnummer. Gibt es eine neuere Version, erscheint dort **"● Update auf vX.Y.Z"**. Ein Klick lädt sie herunter, prüft die Prüfsumme, ersetzt das Programm und startet es neu. QuickLaunch fragt höchstens einmal am Tag bei GitHub nach. Mit dem Befehl `update` suchst du sofort.

## Hilfe bei Problemen

**Windows blockiert das Programm (Smart App Control oder SmartScreen)**
Selbst gebaute Programme sind nicht digital signiert, deshalb können Windows-Schutzfunktionen sie blockieren.

- **SmartScreen** ("Der Computer wurde durch Windows geschützt"): Auf **Weitere Informationen** und dann auf **Trotzdem ausführen** klicken.
- **Smart App Control:** Das Programm wird ohne Hinweis geblockt oder startet nicht. Öffne **Windows-Sicherheit → App- & Browsersteuerung → Smart App Control-Einstellungen**. Wenn du es ausschaltest, ist der Schutz nicht mehr aktiv, und je nach Windows-Version lässt er sich nicht mehr einschalten. Prüfe das vorher.
- Falls ein Antivirus-Programm meldet, dass die Datei verdächtig ist: Es ist ein neues, unsigniertes Programm. Du kannst den Ordner `%LOCALAPPDATA%\QuickLaunch` als Ausnahme hinzufügen.

**BUILD.bat bricht ab**

- Rust und die Visual Studio Build Tools müssen installiert sein, inklusive "Desktopentwicklung mit C++".
- Fehler wie `link.exe not found` oder `cargo not found` bedeuten meistens: Tools fehlen, oder das Fenster muss einmal neu geöffnet werden.
- Prüfen kannst du es mit `rustc --version`.

**Alt + Leertaste öffnet nichts**

- Läuft `quicklaunch.exe` im Task-Manager? Wenn nicht, starte sie aus `%LOCALAPPDATA%\QuickLaunch`.
- Eine andere App belegt vielleicht dieselbe Tastenkombination (zum Beispiel PowerToys Run). Probiere **Strg + Alt + Leertaste**, das ist die Ersatz-Taste.

**Dateien oder Apps werden nicht gefunden**
Kurz warten, der Index aktualisiert sich im Hintergrund. Neu angelegte Dateien erscheinen nach einem Moment automatisch.

## Für Entwickler: neue Version veröffentlichen

Eine neue Version entsteht über einen Tag. GitHub Actions baut dann die `quicklaunch.exe` samt Prüfsumme und legt sie unter Releases ab:

```
git add .
git commit -m "Beschreibung der Änderung"
git push
git tag v0.2.2
git push origin v0.2.2
```

Die Versionsnummer im Tag muss höher sein als die vorherige. Alle laufenden QuickLaunch zeigen danach den Update-Knopf an.

Gebaut von [xeminz](https://github.com/xeminz)
