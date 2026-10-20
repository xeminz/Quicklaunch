# QuickLaunch

Ein schneller Launcher für Windows: Mit **Alt + Leertaste** öffnet sich von überall ein kleines Command-Center-Fenster. Tippen, Enter drücken, fertig.

Gebaut mit **Rust + Tauri v2**. Dadurch braucht QuickLaunch deutlich weniger Arbeitsspeicher als eine Electron-App, weil kein komplettes Chrome mitgeliefert wird.

## Was kann QuickLaunch?

- **Apps starten:** Alle installierten Programme finden und per Enter öffnen
- **Dateien und Ordner suchen:** Findet Dateien und Ordner auf dem ganzen PC und öffnet sie direkt
- **Rechnen:** Rechenaufgaben einfach eintippen, das Ergebnis erscheint sofort
- **Websuche:** Suchbegriff oder Website eingeben, dazu gibt es Vorschläge
- **Eigene Befehle und Automationen:** Zum Beispiel mehrere Apps mit einem einzigen Befehl starten
- **Tastatursteuerung:** Alles lässt sich ohne Maus bedienen
- **Autostart:** Startet auf Wunsch mit Windows und ist immer bereit

### Präfixe

Mit einem Zeichen am Anfang schränkst du die Suche ein:

| Präfix | Bedeutung |
|--------|-----------|
| `>` | Befehle |
| `@` | Apps |
| `#` | Dateien |
| `!` | Eigene Automationen |

### Schnelle Suche durch Index

Beim Start wird ein Cache geladen und im Hintergrund neu gescannt. Ein Dateisystem-Watcher hält den Index live aktuell. Dadurch wird nicht bei jeder Suche der ganze PC neu durchsucht.

## Installation (Schritt für Schritt)

QuickLaunch wird auf deinem eigenen PC gebaut. Das dauert beim ersten Mal ein paar Minuten, danach läuft es von selbst.

### 1. Voraussetzungen (einmalig)

1. **Rust installieren:** Auf [rustup.rs](https://rustup.rs) die Datei `rustup-init.exe` herunterladen, starten und die Standard-Einstellungen nehmen.
2. **Visual Studio Build Tools:** Beim Installieren von Rust wirst du danach gefragt. Wichtig ist der Haken bei **„Desktopentwicklung mit C++"**.
3. **WebView2:** Ist unter Windows 10 und 11 normalerweise schon vorhanden, du musst nichts tun.

### 2. Projekt herunterladen

**Variante A: ohne Git (am einfachsten)**

1. Oben auf dieser Seite auf den grünen Button **Code** klicken.
2. **Download ZIP** wählen.
3. Die ZIP-Datei entpacken, zum Beispiel auf den Desktop.

**Variante B: mit Git**

```
git clone https://github.com/xeminz/quicklaunch-rust.git
```

### 3. Bauen und starten

1. In den entpackten Ordner gehen.
2. **`BUILD.bat` doppelklicken** und warten, bis das Fenster fertig ist (beim ersten Mal dauert es ein paar Minuten).
3. Das fertige Programm liegt danach hier:
```
   %LOCALAPPDATA%\QuickLaunch\quicklaunch.exe
```
   Es startet automatisch und läuft ab jetzt mit Windows.

### 4. Benutzen

- **Alt + Leertaste** drücken, um QuickLaunch zu öffnen.
- Etwas eintippen, zum Beispiel einen App-Namen, und mit Enter bestätigen.
- Mit den Präfixen `>`, `@`, `#` und `!` gezielt suchen.

## Wo liegen meine Daten?

Der Such-Index wird lokal auf deinem PC gespeichert:

```
%APPDATA%\me.emincviko.quicklaunch\files.tsv
```

Es wird nichts ins Internet hochgeladen.

## Projektstruktur

```
quicklaunch-rust/
├── src-tauri/   Rust-Backend (Tauri v2)
├── ui/          Oberfläche (HTML)
├── BUILD.bat    Baut das Programm und installiert es
└── README.md
```

## Hinweis zu Smart App Control

Wenn Windows **Smart App Control** aktiv ist, kann es selbst gebaute Programme blockieren. In dem Fall hilft es, QuickLaunch mit den oben genannten Voraussetzungen lokal zu bauen. Die Smart App Control musst du dafür nicht ausschalten.

## Fehlerhilfe

- **BUILD.bat bricht ab:** Prüfen, ob Rust und die Visual Studio Build Tools mit „Desktopentwicklung mit C++" installiert sind.
- **Alt + Leertaste öffnet nichts:** Prüfen, ob `quicklaunch.exe` im Task-Manager läuft, oder `BUILD.bat` noch einmal ausführen.
- **Neue Dateien werden nicht gefunden:** Kurz warten, der Index aktualisiert sich im Hintergrund.

## Autor

Gebaut von [xeminz](https://github.com/xeminz).
