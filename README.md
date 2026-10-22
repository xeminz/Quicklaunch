# QuickLaunch

Ein schneller Launcher für Windows, inspiriert von der Spotlight-Suche auf dem Mac. Mit **Alt + Leertaste** öffnet sich überall ein kleines Suchfenster. Gebaut mit **Rust + Tauri v2**, dadurch braucht es sehr wenig Arbeitsspeicher.

## Was kann es?

- Apps starten
- Dateien und Ordner auf dem PC finden und öffnen
- Rechnen und im Web suchen
- Eigene Befehle, zum Beispiel mehrere Apps mit einem Befehl starten
- Startet automatisch mit Windows
- Updates direkt über QuickLaunch installieren

Mit einem Zeichen am Anfang suchst du gezielt: `>` Befehle, `@` Apps, `#` Dateien, `!` eigene Automationen.

## Installation

QuickLaunch ist bereits fertig kompiliert. Du musst keine Entwickler-Tools installieren und nichts selbst bauen.

1. Öffne die [Releases-Seite](https://github.com/xeminz/Quicklaunch/releases).
2. Wähle die neueste Version aus.
3. Lade **`quicklaunch.exe`** aus dem Bereich **Assets** herunter.
4. Starte die heruntergeladene Datei.

Das war's! QuickLaunch ist jetzt startbereit.

**Wichtig:** Lade bei der ersten Installation immer die neueste Version von GitHub herunter.

## Updates

QuickLaunch wird regelmäßig aktualisiert. Neue Versionen werden auf der [GitHub-Releases-Seite](https://github.com/xeminz/Quicklaunch/releases) veröffentlicht.

Unten rechts in QuickLaunch siehst du deine aktuelle Versionsnummer. Sobald eine neue Version verfügbar ist, erscheint neben der Versionsnummer ein **blauer Punkt**.

Klicke auf die Versionsnummer, um das Update direkt in QuickLaunch zu installieren. Du musst die neue Version dafür nicht manuell von GitHub herunterladen.

## Automatischer Start mit Windows

QuickLaunch startet automatisch mit Windows, sodass du es nicht jedes Mal manuell öffnen musst.

Nach dem Hochfahren und deiner Windows-Anmeldung läuft QuickLaunch im Hintergrund. Mit **Alt + Leertaste** kannst du den Launcher jederzeit öffnen.

## Benutzung

- **Alt + Leertaste** drücken und lostippen
- Mit den Pfeiltasten auswählen und mit **Enter** öffnen
- Beim ersten Start wird der Suchindex aufgebaut. Es kann kurz dauern, bis alle Dateien gefunden werden.

## Hilfe bei Problemen

### Windows blockiert das Programm

Da das Programm möglicherweise nicht digital signiert ist, kann Windows SmartScreen eine Warnung anzeigen.

Prüfe zuerst, ob du die Datei aus dem offiziellen QuickLaunch-Repository heruntergeladen hast. Führe sie nur aus, wenn du der Quelle vertraust.

### QuickLaunch startet nicht automatisch mit Windows

- Prüfe im Task-Manager unter **Autostart-Apps**, ob QuickLaunch aktiviert ist.
- Stelle sicher, dass die Anwendung korrekt installiert und der Autostart eingerichtet ist.
- Starte QuickLaunch einmal manuell und prüfe die Einstellungen.

### Alt + Leertaste öffnet nichts

- Prüfe, ob `quicklaunch.exe` im Task-Manager läuft.
- Eine andere Anwendung könnte dieselbe Tastenkombination verwenden.
- Starte QuickLaunch erneut.

### Dateien oder Apps werden nicht gefunden

Warte kurz, bis der Suchindex aufgebaut oder aktualisiert wurde. Neu angelegte Dateien können einen Moment benötigen, bis sie erscheinen.

---

Gebaut von [xeminz](https://github.com/xeminz)
