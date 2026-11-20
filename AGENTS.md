# Arbeitsrichtlinien für Dado Player

Produktversionen ausschließlich in release.json erhöhen, danach python scripts/release_version.py ausführen.
python scripts/release_version.py --check gehört zu jedem Release. Gemeinsame Ablaufwerte stehen in
shared/product-policy.json; Sprachkürzel in shared/language-aliases.json. Plattformgrenzen und
Abnahmeumfang stehen in docs/PLATFORM-PARITY.md. Android/iOS behalten bis zur IAP-Einführung
die ausdrücklich vereinbarte Ausnahme von der Windows-/TV-Kaufsperre.

Diese Regeln gelten vom Repository-Stamm aus für Windows, Android und iOS.

Samsung Tizen ist ein separater Web-TV-Client unter SmartTV/client. Gemeinsame Produktabläufe
gemäß SmartTV/UI-SPEC.md beibehalten; native Samsung-APIs in platform.ts/player.ts kapseln.
Bei Web-TV-Änderungen npm run check, npm run test:ui und npm run package ausführen.
Browser-API-Simulation, gültige Gerätesignierung und echte TV-Wiedergabe getrennt dokumentieren.
Build-/Installationswege stehen in SmartTV/client/README.md. LG noch nicht als funktionierendes Ziel ausgeben.

- Gemeinsame Funktionen in src/ entwickeln. Betriebssystem-Aufrufe in src/platform/windows, android oder ios kapseln. Native Hosts, Signierung und Paketierung gehören zum jeweiligen Ziel. Die gemeinsamen Benutzerabläufe über die Plattform-Fassade anbinden.
- Touch-Oberflächen für Filme/Serien verwenden eigene volle Seiten (library::show_touch), keine verkleinerten Desktop-/TV-Spalten. Kategorien als Dropdown, Suche im Tab, Details mit Zurück und Rand-Wischgeste. Folgen mit dem gesamten Detailinhalt vertikal scrollen; nur Staffelbuttons horizontal. TV-Modus behält seine Spalten und Fokuszonen.
- Ein Build-Einstieg pro Ziel gemäß README.md. Bestehende Aufrufe unter probe/ sind Weiterleitungen für ältere Werkzeuge. Bei Änderungen an Pfaden, SDKs, Artefakten oder Voraussetzungen im selben Änderungssatz README, CI und aufrufende Skripte abgleichen.
- Gemeinsame Rust-Änderungen mit cargo fmt --all -- --check und cargo test --locked --all-targets auf Windows prüfen. Für Android den Gradle-/NDK-Build, für iOS den Geräte-Build mit Apple-SDK prüfen. cargo check --features ios-check --lib deckt nur einen Teil des iOS-Codes ab.
- Bei iOS-ABI-Änderungen Header, Rust-Bridge und Swift-Host zusammen ändern. Einen erfolgreichen Paketbuild, eine gültige Gerätesignatur und einen Wiedergabetest auf Hardware getrennt protokollieren.
- Vor Umstrukturierungen den Git-Stand prüfen. Bestehende APKs, portable Pakete, lokale Signierschlüssel und Referenzdaten erhalten. Zugangsdaten, Schlüssel und generierte Pakete nicht committen. Historische Prüfberichte mit ihrem ursprünglichen Stand aufbewahren.
- Die aktuelle Windows-Portable im Release-Ordner heißt immer Releases/Dado-Player.exe, ohne Versionsnummer im Dateinamen. Vor Ersetzen alte Pakete unter Releases/Archiv erhalten. windows/package.ps1 und packaging/organize_releases.py müssen diese Regel gemeinsam einhalten; bei gesperrter EXE klar abbrechen, keine zusätzliche versionierte aktuelle EXE anlegen.

- Scroll-Änderungen mit großem Katalog und wiederholten Richtungstasten prüfen, nicht
  nur durch eine einzelne langsame Fokusbewegung. Arbeit pro Frame muss vom sichtbaren
  Bereich abhängen; native Bilddekodierung/RGBA-Konvertierung außerhalb des UI-Threads.
  Builds und Browser-Tests allein bestätigen keine physische Hardware-Framerate.

- Dado Connect: Vertrag in connect/README.md auf Rust und Web-TV synchron halten. Dauerhaft speichern vor Empfangsbestätigung, stabile Nachrichten-IDs, ausschließlich manuell abrufen. Keine Zugangsdaten protokollieren. Senderkontogenehmigung nicht mit Kauflizenz vermischen. Private Geräteadresse auf Mobile klar benennen.

- Ab 0.7.2: MAC-Zuordnung ohne Kopplungscode, nur für genehmigte Sender und registrierte Geräte; fremde Zuordnungen nicht überschreiben. Endnutzer-UI nur Geräteangaben und Profile aktualisieren. Einmalige Registrierung ist kein Profil-Polling. Dauerhafte Vorlagen nur ausdrücklich speichern, verschlüsseln und kontogebunden prüfen; deren stabile importierte IDs dürfen nur den zugehörigen Cloud-Eintrag aktualisieren.

- Windows breite Standardoberfläche: src/ui/desktop.rs (native Wiedergabe beibehalten).
  Geometrie und Nav-Bewegung aus shared/shell-appearance.json mit Samsung synchron halten.
  Blaupalette in shared/normal-palette.json; Dunkel ebenfalls blau, Hell weiß. Logo-Raster
  aus assets/d7.svg bzw. d7-dark.svg erzeugen, keine unabhängigen Raster-Neuzeichnungen.
- Einstellungen: Karten pro Reihe gleich hoch und Beschriftungen/Controls ausrichten.
  Live-Preview muss lange ungetrennte Metadaten umbrechen, Video behält 16:9 und nutzbare Höhe.
- Isolierte native Sichtprüfung: IPTV_DEMO=1 plus absoluter DADO_QA_DATA_DIR. Windows-APPDATA
  überschreibt KnownFolder-Auflösung nicht. Demo registriert kein Dado-Connect-Gerät.

- Nach abgeschlossenen Builds/Testläufen temporäre Debug-Ausgaben, entpackte
  Testkopien und Testmedien entfernen. Release-Dateien vor dem Aufräumen
  verifizieren. SDKs, Reproduktionsarchive, Quellcode und Signiermaterial erhalten.
  Die Debugprofile verwenden Zeilentabellen statt vollständiger Debugsymbole
  und erzeugen keine inkrementellen Zwischenstände.
