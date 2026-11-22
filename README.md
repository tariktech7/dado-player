# Dado Player

**0.8.0 Beta 1** ist der gemeinsame Stand für Windows, Android, iOS und Samsung Tizen.
Die Beta ist über [GitHub Releases](https://github.com/tariktech7/dado-player/releases/tag/v0.8.0-beta) für berechtigte Konten verfügbar. Die stabilen Updatekanäle bleiben davon unabhängig.
Versionsquelle ist `release.json`; `python scripts/release_version.py` synchronisiert die Ziele,
`--check` verhindert abweichende Metadaten. Funktionsvertrag und bewusste Plattformunterschiede:
[Plattformabgleich](docs/PLATFORM-PARITY.md). Aktuelle Paketdateien und Prüfsummen stehen in Releases/AKTUELL.json.
Windows und Samsung enthalten die Lizenz-Beta mit 7 Tagen Testzeit,
eigenem Sperrbildschirm, MAC/Geräte-ID und manuellem Freischaltabruf. Dauerlizenzen werden
offline geprüft und lösen keine automatischen Lizenz-Serverabrufe aus. Android/iOS bleiben
ohne diese Kaufsperre. Betrieb und Einrichtung: [Lizenzverwaltung](licensing/README.md).
Nur „7 Tage kostenlos testen“ startet einen Trial. „Freischaltung abrufen“ liefert ausschließlich
eine genehmigte Dauerlizenz; ohne Freigabe erscheint eine entsprechende Meldung. Während des Trials
stehen verbleibende Tage unter dem App-Namen, bei eingeklappter Navigation als kurze Anzeige.
Alle Apps verwenden die gleiche Spaltennavigation und ein virtualisiertes Poster-Raster. Auf Telefonen
und Tablets ist die Oberfläche fürs Querformat mit Touch und sicheren Bildschirmrändern ausgelegt.
Unter **Einstellungen → Allgemein → Oberfläche** stehen **Modern** und **Klassisch** zur Wahl.
Klassisch ist die Voreinstellung. Die App speichert die gewählte Ansicht.
Klassisch öffnet eine Bibliotheksübersicht mit großen Medienkacheln und nutzt eine kompakte Senderliste
sowie größere Poster. Beide Modi verwenden dieselbe Wiedergabe und dieselben Profile.
Die Detailspalte erscheint bei Filmen und Serien nach der Auswahl. In Live-TV ist die Vorschau dauerhaft
sichtbar. In Seriendetails lassen sich die Staffeln horizontal und die Folgen vertikal durchblättern.
Der Play-Button setzt die zuletzt gesehene Folge fort und geht bei höchstens 60 Sekunden Rest
zur nächsten Folge über. Favoriten und Fortsetzen haben getrennte Rubriken für Live-TV, Serien
und Filme. Die Startseite kombiniert Fortsetzen mit Artwork und dem aktuellen Favoritenprogramm.
Pro Serie erscheint dort und unter Fortsetzen die zuletzt angesehene Folge. Der Folgenverlauf bleibt erhalten.
Auf Geräten mit Farbtasten öffnet Rot die Elternsperre. Unter Windows entspricht das F1.
Im Live-Vollbild schalten Hoch/Runter die Sender, 0 wechselt zum vorherigen Sender; die eigene
Player-Steuerung blendet sich nach zwei Sekunden aus.
OK im Live-Vollbild öffnet die Senderliste und setzt den Fokus auf den laufenden Sender.
Der Live-Player bündelt Sender und aktuelles Programm unten. Filme und Folgen zeigen dort Play/Pause,
Zeiten und Spulleiste. Tonspuren und die nächste verfügbare Folge sind direkt erreichbar.

Der iOS-Build wird lokal unter Windows/WSL mit
xtool-Darwin-SDK, Rust und Swift als Geräte-IPA gebaut. Installation über AltStore/Sideloadly
mit eigener Apple-ID; Wiedergabe und Layout auf einem echten iPhone bleiben separat zu prüfen.
Der lokale Buildweg steht in [ios/README.md](ios/README.md).
LG-Vorbereitung und TV-Verträge: [SmartTV/README.md](SmartTV/README.md).

Der Rust-Kern unter `src/` enthält App-Zustand, IPTV-Client, Speicherung, Datenmodell und egui-Oberfläche. Alle drei Apps verwenden dieselbe Crate und dieselbe Cargo.lock.

| Ziel | Einstieg und Wiedergabe | Build aus dem Projektordner | Ergebnis |
| --- | --- | --- | --- |
| Windows | src/main.rs → src/platform/windows, libmpv | `./windows/build.ps1` in PowerShell | target/release/Dado-Player.exe |
| Android | android/ (Gradle), src/platform/android, JNI und libmpv | `bash android/build.sh` unter Windows mit Git Bash | Dado-Player-arm64-debug.apk |
| iOS | ios/Sources (UIKit, AVPlayer), src/platform/ios (Rust und C-ABI) | `bash ios/build.sh --ipa` mit iOS-SDK | Dado-Player-unsigned.ipa |
| Samsung Tizen 6.5+ | SmartTV/client (Preact/TypeScript, AVPlay) | `./SmartTV/client/build.ps1` | Dado-Player-Tizen-0.6.0-unsigned.wgt; TV-Zertifikat für Installation erforderlich |

Windows bündelt seine Laufzeit-DLLs in `dist/`. Das separate portable Paket wird mit `packaging/build_portable.py` aus dem Inhalt von dist gebaut. Der Cargo-Build aktualisiert dist nicht. Android benötigt JDK, Android SDK, NDK, cargo-ndk und für den lokalen signierten Build den vorhandenen Projektschlüssel. Details zum Swift-Host stehen in [ios/README.md](ios/README.md).

[AGENTS.md](AGENTS.md) beschreibt Zuständigkeiten und Prüfungen. [SmartTV/README.md](SmartTV/README.md) enthält die gemeinsame UI- und Technikvorbereitung für LG webOS und Samsung Tizen. references/ enthält Vergleichsmaterial. probe/ und .workbuddy-ai/ enthalten Prüfwerkzeuge und historische Aufnahmen. Frühere Baupläne und Prüfberichte stehen unter docs/history/.

## Portable Windows-Updates

Aktuelle Pakete liegen direkt in `Releases/`; die Windows-Datei heißt immer **Dado-Player.exe**, ohne Versionsnummer. `windows/package.ps1` ersetzt sie über `packaging/organize_releases.py --install-windows`, prüft die Kopie und sichert die Vorgängerdatei im Archiv. Eine geöffnete/gesperrte EXE führt zu einem klaren Fehler statt einer zweiten aktuellen Datei. `README.md` und `AKTUELL.json` dort nennen Plattform, Version und SHA-256. Frühere Pakete stehen unter `Releases/Archiv/`. `python packaging/organize_releases.py` archiviert frühere Windows-/Tizen-Dateinamen und veraltete IPAs. Der Android-Build prüft die Signatur der APK, der iOS-Build erzeugt die IPA für die anschließende Gerätesignierung.

Zum Paketieren `./windows/package.ps1` mit einer Python-Installation mit Pillow ausführen (optional `-Python <Pfad>`). Die portable EXE steht danach in `Releases/Dado-Player.exe`.

Unter **Einstellungen → Auf GitHub-Updates prüfen** sucht die portable App in den veröffentlichten GitHub-Releases, einschließlich Betas. Für das private Repository verwendet sie vorhandene Git-Zugangsdaten oder einen eingegebenen Token mit Leserecht. Nach „Installieren und neu starten“ prüft die App das heruntergeladene Paket. Der Launcher ersetzt seine EXE am ursprünglichen Ort, sichert die letzte Datei als `.previous` und startet die neue Version.

### Windows 10/11 und Herausgebersignatur

Ab portable 0.5.2 werden keine aus dem Build-Windows kopierten `d3dcompiler_47.dll` und `msvcp_win.dll` mehr gepackt. Windows 10/11 laden ihre eigenen Systemkomponenten. Die app-lokalen VC-Runtime-Dateien bleiben enthalten. Historische lokale Kopien sind unter dist/legacy-system-dlls archiviert und nicht Teil des Pakets. `python -m unittest discover -s packaging -p test_runtime_policy.py` prüft die Paketregel. Der Launcher prüft vor Wiederverwendung des Laufzeitcaches auch die CRC, nicht nur die Dateigröße.

Für eine öffentliche Herausgebersignatur ist ein gültiges, von einer öffentlichen CA ausgestelltes Code-Signing-Zertifikat mit privatem Schlüssel im Benutzer-Zertifikatsspeicher erforderlich. Dann `./windows/package.ps1 -SigningThumbprint <Thumbprint>` ausführen (alternativ DADO_SIGN_CERT_THUMBPRINT setzen). Der Build signiert die enthaltene App nach den Icon-Ressourcenänderungen und die fertige portable EXE nach Anhängen des Pakets; SignTool prüft SHA-256, Zertifikatsvertrauen und RFC-3161-Zeitstempel. Launcher und Updater berücksichtigen die Authenticode-Zertifikatstabelle hinter dem Paket.

Ohne konfigurierte Identität bleiben die Pakete unsigniert. Ein selbstsigniertes Zertifikat beseitigt „Unbekannter Herausgeber“ auf fremden Rechnern nicht. Eine gültige Signatur gibt den geprüften Herausgeber an, garantiert aber keine sofortige SmartScreen-Reputation. Offizielle Grundlagen: [Microsoft Code Signing](https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options), [SmartScreen-Reputation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation). Die Paketierung verändert keine SmartScreen-/Defender-Einstellungen.

### GitHub-Updates veröffentlichen

Android übergibt das geprüfte APK beim Klick auf „Installieren“ direkt an Androids Paketinstaller. Je nach Android-Version und Gerätekonfiguration ist eine Systembestätigung oder die Berechtigung „Unbekannte Apps installieren“ erforderlich. Auf iOS die geprüfte IPA in Dateien sichern und in AltStore Classic über My Apps → + mit demselben Apple-Konto signieren und als Update installieren. Für das private GitHub-Repository benötigen mobile Geräte einen kurzlebigen Fine-grained Token mit Contents-Leserecht für dieses Repo.

Für Samsung am eingerichteten Entwickler-PC `./SmartTV/client/deploy.ps1 -FromGitHub` ausführen. Mit `-Tizen5` und `-TvIp` lässt sich das Paket für einen älteren autorisierten Fernseher auswählen. Der Befehl lädt das signierte WGT, prüft Größe und SHA-256 und installiert es über Tizen Studio. Das Paket muss zum DUID des Fernsehers passen.

Vor jedem Release die Buildnummern der betroffenen Plattformen in `release.json` erhöhen, `python scripts/release_version.py` ausführen und die Pakete bauen. Anschließend `python packaging/organize_releases.py` ausführen und die kanonischen Dateien im GitHub-Release `vX.Y.Z-beta` oder `vX.Y.Z` mit den jeweiligen `assetLabel`-Werten aus `Releases/AKTUELL.json` veröffentlichen. Die Updater erwarten diese Labels und prüfen Buildnummer, Dateigröße und SHA-256. Gleichbleibende Versionsnummern sind mit einer höheren Buildnummer möglich.

## D7-Oberfläche und Wiedergabe

Die Zeichenform in assets/d7.svg ist die Vorlage für App-Icons und das gemeinsame Markenzeichen. Die Farbpalette liegt in src/theme.rs. Windows, Android, iOS und Samsung teilen Navigation, Katalogspalten und Player-Bedienung. Android und iOS starten im Querformat. Nach der Anmeldung öffnet die App beim nächsten Start das letzte Profil. Abmelden setzt diesen Wiedereinstieg aus.

Auf iOS laufen Streams mit AVPlayer. Xtream verwendet zunächst HLS, bereitgestellte M3U-URLs bleiben erhalten. Der lokale Wiedergabetest nutzt ein mitgeliefertes H.264/AAC-MP4. Der macOS-Workflow baut die Geräte-IPA und startet zusätzlich einen Simulator mit diesem Clip. Für eine Freigabe auf Hardware Live-HLS, VOD, Ton, Rotation und Eingabe auf einem iPhone prüfen.

### Mobile 0.2.3 / Windows 0.5.1
Die iOS-Zeichenfläche verwendet die vollständigen Fenstermaße. Die beiden lokalen egui-winit-/egui-wgpu-Patches umgehen dafür Winit 0.29s Safe-Area-bereinigte inner_size; Safe Areas werden ausschließlich im Inhaltslayout berücksichtigt. Der Simulator prüft zusätzlich, dass UIKit-Fenster und egui-Fläche übereinstimmen.

Dado-Steuerung ist auf iOS wieder Standard: AVPlayerLayer liegt hinter der transparenten Metal-Oberfläche. Einstellungen → Wiedergabe → Player-Oberfläche bietet weiterhin Apple Player als Option, wirksam ab der nächsten Wiedergabe. Tonspuren und Untertitel werden über AVFoundation an die gemeinsamen Menüs angebunden, soweit der Stream sie bereitstellt.

Die Startseite zeigt auf allen Plattformen kompakte Senderkarten mit Logos. Der separate Bereich „Das läuft auf deinen Favoriten“ liest ausschließlich favorisierte Sender und deren aktuelle EPG-Sendung. Auf Telefonen startet ein Tipp die Wiedergabe direkt. Alle Plattformen bündeln Source Sans 3 in festen Gewichten; Quelle und OFL-Lizenz liegen unter assets/fonts.

**Prüfstand 13.11.2026:** Der neue iOS-Gerätebuild und die Simulator-Bildprüfung sind noch nicht durchgeführt: GitHub Actions verweigert bereits den Runnerstart wegen Kontozahlung/Ausgabenlimit (Run 43489126211). Ein Host-Typecheck ersetzt diese Prüfungen nicht. Die zuvor veröffentlichte IPA bleibt Build 4 / 0.2.1; nicht als neue IPA ausgeben.

### Bedienkorrekturen 0.2.1 / Windows 0.4.1
Startseitenkarten setzen gespeicherte Wiedergabe direkt fort. Die Windows-Version setzt die Startposition vor dem asynchronen Laden. Live-Sender starten auf mobilen Geräten beim Antippen der Zeile. Leere Codec-/Auflösungsanzeigen werden ausgeblendet, erfundene Programmfortschritte entfernt. Navigation und Startseite teilen sich durchscheinende Oberflächen mit Lichtkanten und abgestimmten Hintergründen.

### Live-TV-Spaltenansicht
Windows und der TV-Modus verwenden dieselbe Ansicht mit getrennt scrollbaren Kategorien, Sendern und einer Vorschau. Senderzeilen zeigen das aktuelle und das nächste Programm. Die erste Auswahl startet die interne Vorschau, die zweite Auswahl desselben Senders oder die Aktivierung des Vorschaubilds öffnet den großen Player. Auf Fernsehern bewegt das Steuerkreuz den Fokus zwischen den Spalten. Die Android-Vorschau wird in einen eigenen OpenGL-Puffer gerendert und anschließend in die Vorschaufläche kopiert.

### Android TV 0.2.4

Die TV-Aufteilung ist auf die übliche Arbeitsfläche von 960 × 540 dp ausgelegt. 1920 × 1080 bei 320 dpi und 3840 × 2160 bei 640 dpi behalten dieselbe Größe der Bedienelemente. Die physische Diagonale (etwa 55 oder 77 Zoll) ist kein Layout-Breakpoint. Die Schrift wird bei 4K nicht halbiert. Navigation, Kategorien, Sender und Vorschau passen nebeneinander; Einstellungen bleiben im sicheren Bildschirmbereich. Der TV-Kopf verwendet das gemeinsame D7-Logo.

Auf Android-TV-Geräten aktiviert die Geräteerkennung den TV-Modus automatisch. Zum Ausprobieren auf anderen Android-Geräten unter Einstellungen → Fernsehen → TV-Modus „Immer“ wählen. Pfeile navigieren, OK startet Vorschau bzw. Player, Zurück schließt zuerst den Player bzw. die Vorschau. Am Hauptmenü legt Zurück die native Hauptaktivität in den Hintergrund, damit das Wiederöffnen keinen zweiten winit-Eventloop anfordert.

Die Abnahme dieses Stands erfolgte im Android-Emulator mit Demodaten und Fernbedienungstasten, nicht auf echter 55-/77-Zoll-Hardware. Lesbarkeit hängt außerdem vom Sitzabstand und der Geräteskalierung ab. Die letzten iOS-Änderungen sind weiterhin mangels gestartetem macOS-Runner nicht in einer neuen IPA bestätigt.

### Android TV 0.2.5: kompakte Ansicht

Die TV-Navigation ist schmaler, Schrift, Logos und Karten sind kleiner. Der Kategoriename („Alle Sender“) liegt in der oberen Markenleiste. Kategorienüberschrift, Suchfeld und Vorschauüberschrift teilen sich die zweite Kopfzeile. Ein zusätzlicher Überschriften-/Senderzählerblock über der Liste entfällt.

Die Liste verwendet die restliche Höhe für mindestens sieben vollständige Zeilen mit geöffneter Vorschau. Die Zeilenhöhe wird aus `(Listenhöhe - 6 × 4 dp Abstand) / 7` berechnet und auf 48–58 dp begrenzt. Die drei Textzeilen werden nach den tatsächlichen Schriftmetriken platziert. Auf größeren logischen Arbeitsflächen bleiben zusätzliche Zeilen sichtbar. Menü und Kategorien verwenden kompakte Zeilen und einen dezenteren Fernbedienungsfokus.

Der Layouttest prüft mindestens sieben Sender sowie die Ausrichtung von Marken-/Kategorientitel und Kategorien-/Suchzeile. Full-HD- und 4K-Aufnahmen mit Vorschau und Demodaten bestätigen sieben vollständige Zeilen im Emulator. Android-Version 0.2.5, Build 16; Windows und iOS wurden für diesen TV-Änderungssatz nicht neu paketiert.

## Dado Connect

M3U-URLs und Xtream-Zugänge nach manueller Kontogenehmigung und einmaliger Gerätezuordnung per MAC-/Geräteadresse an Windows, Android, iOS und Samsung senden. Am Gerät mit Aktualisieren speichern. [Portal](https://dado-connect.dado-licensing.workers.dev), [Verwaltung](https://dado-admin.dado-licensing.workers.dev/admin/connect), [Anleitung](connect/README.md). Keine Hintergrundabrufe; Kontogenehmigung und Kauflizenz sind unabhängig.

## Darstellung und Player ab 0.7.2

Standard ist Blau: gedeckte stahlblaue Flächen und klare blaue Akzente. Dunkel verwendet weiche Mitternachtsblautöne, Hell bleibt weiß. Alle Auswahlen werden gespeichert. Der alte interne Warm-Wert bleibt kompatibel und bezeichnet jetzt Blau; ausdrücklich gespeicherte Dunkel-/Hell-Auswahlen bleiben erhalten.

Android und Android TV öffnen optional die installierte MX-Player-App im Expertenmodus. Automatisch verwendet weiterhin den eingebauten Player. Es gibt kein eingebettetes MX-SDK in diesem Projekt; alte irreführende Optionen sind entfernt und kompatibel migriert. Windows verwendet mpv, iOS AVPlayer/VLC, Samsung AVPlay/HTML. Anbieter-Kataloge bleiben nach Inhaltstyp getrennt; Datensätze mit fremdem Identifier oder ausdrücklich falschem Streamtyp werden nicht übernommen.
Elternsperre ab 0.7.4: Unter **Einstellungen → Elternsperre** eine vierstellige PIN
einrichten und Sender, Filme oder ganze Serien sperren. Weitere Informationen:
[Elternsperre](docs/PARENTAL-CONTROLS.md).

### Gemeinsame Oberfläche ab 0.7.4
Windows übernimmt die Samsung-Spalten, kompakte Navigation und deren 220-ms-Bewegung.
Gemeinsame Geometrie: `shared/shell-appearance.json`; normale Blaupalette:
`shared/normal-palette.json`. Blau ist der Standard, Dunkel verwendet weiche Mitternachtsblautöne,
Hell bleibt weiß. Logos und Online-Portale verwenden dieselbe blaue D7-Marke.
Einstellungen stehen in gleich hohen Karten pro Zeile; lange Live-EPG-Texte bleiben in der Vorschau.
Telefon-/Tablet-Ansichten behalten ihre eigene Touch-Navigation.


### Einstellungen ab 0.7.5

Allgemein öffnet sich mit App-Sprache, Zeitformat und Farbschema. Wiedergabe, Ton und
Untertitel, Profile, Elternsperre, Updates und Lizenz haben getrennte Bereiche.
Technische Regler erscheinen unter Erweitert nach Einschalten des Expertenmodus.
Das Zeitformat gilt für Uhrzeiten im EPG, Wiedergabepositionen bleiben Laufzeiten.
Englisch ist für Navigation, die neue Einstellungsstruktur und zentrale Player- und
Live-Texte verfügbar. Ältere Dialoge und technische Meldungen können deutsch bleiben.
