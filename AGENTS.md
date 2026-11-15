# Arbeitsrichtlinien für Dado Player

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
