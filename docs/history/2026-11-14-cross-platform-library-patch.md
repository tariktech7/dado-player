# Gemeinsamer Bibliotheks- und TV-Patch, 14.11.2026

Ziele: Windows 0.5.4, Android 0.2.7 (11), Tizen 0.1.5; iOS-Quellstand 0.2.4 (6).
Bestehende Schlüssel, Profile und frühere Builds bleiben erhalten. Die alten getrennten
Film-/Serienansichten sind durch src/ui/library.rs ersetzt; Git erhält ihre Historie.

## Verhalten

Kategorien → Titel → Details für Filme und Serien, Favoriten zuerst; Details bleiben reserviert.
Auf schmalen Telefonen sind Kategorien/Titel oben und Details darunter. Serie: Staffelbuttons,
nur aktuelle Staffel, Folge N statt Anbieterdateinamen, zentraler Play-Button. Fortsetzen wählt
die zuletzt gesehene Folge; ab 60 Sekunden Rest startet die nächste Folge, auch über Staffeln.
Am Ende der letzten Staffel beginnt erneutes Abspielen bei der ersten Folge. Unbekannte Dauer
löst keinen Sprung aus. Dauer wird zusätzlich zum bisherigen Fortschritt gespeichert.

Favoriten und Fortsetzen besitzen Live-TV-/Serien-/Filme-Rubriken. Favorisierte Folgen werden
im gemeinsamen Rust-Client als Serie gemerkt. Die Startseite zeigt persönliche Begrüßung,
Fortsetzen mit Artwork sowie eine getrennte Favoriten-EPG-Spalte auf breiten Displays; auf
Telefonen stehen dieselben Informationen untereinander. Android TV bietet die Startseite
zusätzlich in der linken Navigation.

Live-Vorschau bleibt vor Auswahl als Platzhalter sichtbar. Bereiche behalten eigene Kategorien,
Suche und Scrollidentitäten. Live-Vollbild: Hoch/Runter zum Zappen, 0 zum vorherigen Sender,
zwei Sekunden Overlay. Tizen scrollt horizontale Reihen in beide Richtungen mit dem Fokus,
ohne den App-Canvas zu verschieben. Die Tastatur öffnet weiterhin erst bei OK/Klick.

Android: MX Player/VLC erhalten eine Startposition und liefern Activity-Ergebnisse für Position
und Dauer zurück. MX benötigt bei HTTP die explizite Activity-Klasse. Fremde/System-Player
können diesen Rückkanal ignorieren; deren eigene Overlays/D-pad-Aktionen werden von ihnen
verwaltet. Die Änderung ist gegen die dokumentierte MX-Intent-API implementiert:
https://sites.google.com/site/mxvpen/api

## Nachweise und Grenzen

- 44 Rust-Tests, Formatierung und Clippy mit -D warnings bestanden (bekannte Vendor-Warnungen).
- iOS-Host-Typcheck bestanden; dies ist kein Geräte-Build und prüft keinen Swift-Host.
- Tizen: TypeScript, zehn Unit-/Integrationstests, 15 UI-Tests in eigenem Headless-Edge bestanden.
  Die Tests prüfen Full HD/4K, Inline-Tastatur, TV-Auswahlfelder, proportionale Logos,
  Staffelwahl, Menü-/Scrollgedächtnis, Zappen/0, Zwei-Sekunden-Overlay und horizontale Reihen.
  H.264/AAC-Testvideo tatsächlich in HTML abgespielt; AVPlay durch kontrolliertes API-Mock geprüft.
- Tizen 0.1.4 mit bestehendem Samsung-Profil signiert, vom TV angenommen, erfolgreich
  aktualisiert und gestartet (PID 12152). Der abschließende Fokuspatch 0.1.5 wird als Paket
  gebaut, aber auf Wunsch des Eigentümers bei ausgeschaltetem TV nicht installiert. Echte Kanalwiedergabe war vom Eigentümer vor diesem
  Patch bestätigt. Neue Bedienung und lokaler AVPlay-Testclip sind noch nicht auf dem TV bestätigt.
- Windows portable: aus einem Ordner mit ausschließlich der EXE gestartet, neun Laufzeitdateien
  extrahiert, Testvideo laut MPV-Log abgespielt, sauberer Exit 0. Kein neuer Windows-10-Hardwaretest.
- Android: Release-Rust + Gradle gebaut, bestehende Projektsignatur v2/v3 geprüft, zipalign 4 KiB.
  UI-Prüfung im eigenen Android-Emulator mit künstlichem Anbieter; kein externer MX-/VLC-Hardwaretest.
- iOS-Build weiterhin blockiert: GitHub-Run 43596907029 wurde wegen fehlgeschlagener
  Kontozahlungen/Ausgabenlimit nicht gestartet. Keine neue IPA in diesem Patch veröffentlichen.
  Die bisherige IPA 0.2.1/Build 4 bleibt im älteren Release und enthält diese Änderungen nicht.
- Windows bleibt ohne öffentlich vertrauenswürdige Codesignatur. Tizen-Signatur ist für den
  eingerichteten Entwickler-TV bestimmt; andere Geräte benötigen ihre eigene Freigabe/Signierung.


Nachtrag Fokus/Artwork: Alle TV-Spalten haben eigene stabile Fokusidentitäten. Links/Rechts
stellt die zuletzt fokussierte Kategorie bzw. den Sender wieder her, anstatt die nächste
geometrische Höhe zu wählen. Horizontale Reihen behalten intern Links/Rechts; am Ende geht
es in die nächste Spalte. Home-Play → Favoriten und zurück wird ausdrücklich geprüft.
Historische Serienfolgen ohne Artwork bekommen das aktuelle Serien-Cover aus dem Katalog.


Release-Ablage: Aktuelle EXE/APK/WGT direkt im Wurzelordner, Index README.md und maschinenlesbare
AKTUELL.json mit SHA-256. Vorherige Tizen-Versionen nach Plattform/Version archiviert. Die alte
IPA wurde nach Archiv/iOS/0.2.1-Build-4 verschoben, damit sie nicht als aktueller Patch erscheint.
Vorherige lokale EXE/APK vor der letzten Fokuskorrektur zusätzlich im Archiv gesichert.
packaging/organize_releases.py wiederholt diese Ordnung nach späteren Builds, ohne zu löschen.


## Abschließende Paketprüfung

Android-Emulator: Spaltenrückkehr Sport → Motorsport HD → Sport → Motorsport HD
mit D-pad geprüft. Home → rechte Favoriten-EPG-Spalte ebenfalls per D-pad erreicht.
Windows-One-File-Endprüfung mit dem endgültigen Paket erneut bestanden.
Aktueller iOS-Run 43601485417 (Quellstand 4e29e11) scheitert vor Jobstart am Billing.

| Paket | Bytes | SHA-256 |
| --- | ---: | --- |
| Dado-Player.exe | 54968037 | a9965f86636b10cb6ea65d5cf780275165aa4b2f1ffb36695c8fa0e35d3f424b |
| Dado-Player-arm64-debug.apk | 44970535 | 2dab459aa319de408f9a8fe76fe7918b1b4ba4564195c23ada52839e239a643b |
| Dado-Player-Tizen-0.1.5.wgt | 911676 | 9b6b15a7891f0f6d5ad3d302d11b1d2db830ff0b5480d925ca1a0cb9f47719b4 |
| Dado-Player-Tizen-0.1.5-unsigned.wgt | 924541 | a9e84240c23f55435931367a930e3bc3d4dc9f4ca89417d62cd3bd58e85d6f5c |
