# Dado Player für Smart TVs

Stand: 14. November 2026. Android TV ist Teil der bestehenden Android-App.
Samsung Tizen hat jetzt einen eigenen baubaren [Web-TV-Client](client/README.md)
(0.1.6, Tizen 6.5+). Die Samsung-Zertifikatssignierung und Hardwareabnahme sind getrennte Schritte.
LG webOS bleibt ein vorbereitetes Portierungsziel. APK/EXE sind keine IPK/WGT.

- [UI-SPEC.md](UI-SPEC.md): Darstellung und Fernbedienungsabläufe.
- [TECHNICAL-PLAN.md](TECHNICAL-PLAN.md): Architektur, Schnittstellen, Referenz und Paketwege.
- [shared/tv-ui.json](shared/tv-ui.json): maschinenlesbare Maße für den künftigen Web-Client.
- [PORTING-PLAN.md](PORTING-PLAN.md): historischer Samsung-Plan, ausdrücklich abgelöst.
- [client/README.md](client/README.md): Tizen-Build, Einstellungen, Companion und Installation.

Aktuelle Implementierung: src/ui/mobile/tv.rs (Shell), src/ui/live.rs (Live-TV-Spalten),
src/ui/mobile/focus.rs (Fernbedienung), src/ui/mod.rs::contain_rect (proportionale Bilder).
Die Tests in src/app.rs prüfen Kopfzeilen, zehn vollständige Senderzeilen mit Vorschau
und getrennte Jetzt-/Danach-Texte bei 960×540 und 1280×720 Punkten.

Bei Änderungen an der TV-Dichte diese Spezifikationsdateien im selben Änderungssatz
mitführen. Die JSON-Werte sind eine Portierungsvorgabe, noch kein Runtime-Input für Rust.
Native Auflösung und CSS-/egui-Punkte müssen auf jedem Zielgerät abgeglichen werden.
