# Dado Player

[Downloads für Windows, Android und iOS auf GitHub Releases](https://github.com/tariktech7/dado-player/releases/latest)

Der Rust-Kern unter `src/` enthält App-Zustand, IPTV-Client, Speicherung, Datenmodell und egui-Oberfläche. Alle drei Apps verwenden dieselbe Crate und dieselbe Cargo.lock.

| Ziel | Einstieg und Wiedergabe | Build aus dem Projektordner | Ergebnis |
| --- | --- | --- | --- |
| Windows | src/main.rs → src/platform/windows, libmpv | `./windows/build.ps1` in PowerShell | target/release/Dado-Player.exe |
| Android | android/ (Gradle), src/platform/android, JNI und libmpv | `bash android/build.sh` unter Windows mit Git Bash | Dado-Player-arm64-debug.apk |
| iOS | ios/Sources (UIKit, AVPlayer), src/platform/ios (Rust und C-ABI) | `bash ios/build.sh --ipa` mit iOS-SDK | Dado-Player-unsigned.ipa |

Windows bündelt seine Laufzeit-DLLs in `dist/`. Das separate portable Paket wird mit `packaging/build_portable.py` aus dem Inhalt von dist gebaut. Der Cargo-Build aktualisiert dist nicht. Android benötigt JDK, Android SDK, NDK, cargo-ndk und für den lokalen signierten Build den vorhandenen Projektschlüssel. Details zum Swift-Host stehen in [ios/README.md](ios/README.md).

[AGENTS.md](AGENTS.md) beschreibt Zuständigkeiten und Prüfungen. SmartTV/ enthält den separaten Portierungsplan. references/ enthält Vergleichsmaterial. probe/ und .workbuddy-ai/ enthalten Prüfwerkzeuge und historische Aufnahmen. Frühere Baupläne und Prüfberichte stehen unter docs/history/.

## Portable Windows-Updates

Die drei Ausgabeformate liegen lokal in `Releases/`. Der Android-Build prüft die Signatur der APK, der iOS-Build erzeugt die IPA für die anschließende Gerätesignierung.

Die Windows-Version steht in `windows/VERSION`. Zum Paketieren `./windows/package.ps1` mit einer Python-Installation mit Pillow ausführen (optional `-Python <Pfad>`). Die portable EXE steht danach in `Releases/Dado-Player.exe`. Für ein neues Update zuerst die Version erhöhen, testen, den zugehörigen Quellstand committen und das Paket bauen. Anschließend `./windows/publish-portable.ps1` ausführen. Der Befehl veröffentlicht die EXE im privaten Repo als Git-Tag `portable-vX.Y.Z`.

Die portable App fragt auf Wunsch unter **Einstellungen → Auf GitHub-Updates prüfen** die Tags ab. Sie benötigt Git für Windows und ein Konto mit Lesezugriff auf das Repo. Nach Auswahl einer neueren Version lädt sie den bezeichneten Commit und prüft das Paket. Beim Beenden ersetzt der Launcher seine EXE am ursprünglichen Ort, behält die letzte Datei als `.previous` und startet die neue Version. Ein späteres Release benötigt einen neuen Tag.

### Mobile Updates aus GitHub Releases

Unter Einstellungen → App-Updates kann der Player ein Paket prüfen und laden. Für das private Repository einen kurzlebigen Fine-grained Token mit Contents: Read für dado-player eingeben oder die Release-Seite im angemeldeten Browser öffnen. Android öffnet den Systeminstaller. Auf iOS die IPA in Dateien sichern und in AltStore Classic über My Apps → + mit derselben Apple-ID installieren.

Bei jedem mobilen Release die native Buildnummer und die entsprechende Buildnummer in src/ui/mobile/update.rs gemeinsam erhöhen. Die APK als Dado-Player-arm64-debug.apk mit Asset-Label android-build-<versionCode>, die IPA als Dado-Player-unsigned.ipa mit ios-build-<CFBundleVersion> veröffentlichen. Der Updater prüft Buildnummer, Dateigröße und den SHA-256-Digest von GitHub.

## D7-Oberfläche und Wiedergabe

Die Zeichenform in assets/d7.svg ist die Vorlage für App-Icons und das gemeinsame Markenzeichen. Die Farbpalette liegt in src/theme.rs. Telefone verwenden die untere Navigation, breite mobile Fenster die Seitenleiste. Nach der Anmeldung öffnet die App beim nächsten Start das letzte Profil. Abmelden setzt diesen Wiedereinstieg aus.

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
