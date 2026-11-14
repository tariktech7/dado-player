# Dado Player für Smart TVs

Stand: 14. November 2026. Android TV ist Teil der bestehenden Android-App.
LG webOS und Samsung Tizen sind **vorbereitete Portierungsziele**, noch keine
baubaren Dado-Apps. Eine APK oder Windows-EXE lässt sich nicht als IPK/WGT installieren.

- [UI-SPEC.md](UI-SPEC.md): Darstellung und Fernbedienungsabläufe.
- [TECHNICAL-PLAN.md](TECHNICAL-PLAN.md): Architektur, Schnittstellen, Referenz und Paketwege.
- [shared/tv-ui.json](shared/tv-ui.json): maschinenlesbare Maße für den künftigen Web-Client.
- [PORTING-PLAN.md](PORTING-PLAN.md): historischer Samsung-Plan, ausdrücklich abgelöst.

Aktuelle Implementierung: src/ui/mobile/tv.rs (Shell), src/ui/live.rs (Live-TV-Spalten),
src/ui/mobile/focus.rs (Fernbedienung), src/ui/mod.rs::contain_rect (proportionale Bilder).
Die Tests in src/app.rs prüfen Kopfzeilen, zehn vollständige Senderzeilen mit Vorschau
und getrennte Jetzt-/Danach-Texte bei 960×540 und 1280×720 Punkten.

Bei Änderungen an der TV-Dichte diese Spezifikationsdateien im selben Änderungssatz
mitführen. Die JSON-Werte sind eine Portierungsvorgabe, noch kein Runtime-Input für Rust.
Native Auflösung und CSS-/egui-Punkte müssen auf jedem Zielgerät abgeglichen werden.
