# Erste Samsung-Tizen-Implementierung, Stand 0.1.3

Eigenständiger Preact-/TypeScript-TV-Client unter SmartTV/client, mit AVPlay und
eigener Dado-Steuerung. Windows-/Android-/iOS-Projekte bleiben erhalten.
Build, Installation, unterstützte Einstellungen und optionale API-/EPG-Companion-
Konfiguration stehen in SmartTV/client/README.md.

## Nachweise

- TypeScript-Prüfung und neun Unit-/API-/Speicher-/XMLTV-/Companion-Tests bestanden.
- Zehn UI-Tests in einem eigenen headless Edge: Anmeldung, Profilwiederaufnahme,
  Full-HD-/4K-Geometrie, zehn Senderzeilen, proportionale Logos, Vorschau/Vollbild,
  Spuren, Favoriten, Serien, Einstellungen, virtualisierte Navigation, Auswahl erst
  mit OK, stabile Tastaturhöhe, Eingabe erst mit OK und Wiederherstellung der oberen Abstände.
- Browser-Videotest: mitgeliefertes H.264-/AAC-MP4 decodiert in Edge, Steuerung blendet sich aus.
  Playwrights mitgeliefertes Windows-Chromium hat keinen H.264-/AAC-Decoder; dort
  wird ausschließlich dieser Test begründet übersprungen, nicht als erfolgreich ausgegeben.
- npm audit nach Aktualisierung der gepinnten Preact-/fflate-Abhängigkeiten ohne bekannte Schwachstellen.
- Offizielle Tizen-CLI und SDB lokal aus SDK-Paketen bereitgestellt, SHA256 gegen
  den offiziellen Paketindex geprüft. Samsung Certificate Extension 2.0.75 eingerichtet.
- Besitzer erstellte Samsung-TV-Profil DadoPlayer selbst einschließlich persönlicher Anmeldung.
  Private Schlüssel liegen unter dem Benutzerprofil, nicht im Repository oder Widget.
- GQ77S94CATXZG meldet über SDB Tizen 9.0. Signaturprüfung und Installation von
  0.1.0, 0.1.1, 0.1.2 und 0.1.3 erfolgreich; 0.1.3 unter DadoPlayTV.Player gestartet.
- Besitzer bestätigt sichtbare TV-Oberfläche und Wiedergabe eines echten Live-Senders.

## Während des Gerätetests korrigiert

- Native HTML-select-Felder wurden durch explizit mit OK geöffnete Dado-Menüs ersetzt.
- Bildschirmtastatur verkleinert nur sichtbare Formularhöhe; das Raster skaliert nach Breite.
- Textfelder sind beim Navigieren readonly und öffnen die IME erst nach OK/Klick.
- Scrollen bleibt in den Inhaltsbereichen; verborgenes Canvas/Document darf nicht mitwandern.
- AVPlay benötigt für das Paket-Testvideo den echten wgt-package-Dateipfad statt
  der virtuellen WebKit-Adresse file:///test-video.mp4. Über filesystem.resolve angebunden.

Noch offen: Besitzerabnahme des lokalen Testvideos nach 0.1.3, weitere reale
VOD-/Serien-/Spuren-/Suspend-Tests und die anschließend zusätzlich angeforderten
gemeinsamen Bibliotheksspalten/Rubrikzustände sowie TV-Zapping/Overlay-Anpassungen.
Dieser Bericht beschreibt den historischen Zwischenstand, keine vollständige Hardwarefreigabe.
