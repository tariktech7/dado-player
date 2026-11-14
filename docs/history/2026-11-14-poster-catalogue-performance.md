# Poster-Raster und Serienbedienung, 14.11.2026

Zielversionen: Windows 0.5.5, Android 0.2.8/12, Tizen 0.1.6, iOS-Quellcode 0.2.5/7.

## Ursache und Änderung

Der Tizen-Katalog renderte bisher jeden Film/jede Serie als DOM-Zeile. Das konnte beim ersten
Öffnen großer Bibliotheken den UI-Thread blockieren. Der gemeinsame Rust-Katalog kopierte
zusätzlich zweimal pro Frame sämtliche Titel einschließlich Beschreibung/URL/Cover und baute
für jeden Titel ein Widget auf. Beide Wege sind durch virtualisierte Poster-Raster ersetzt.

Tizen filtert per Memoisierung und rendert nur sichtbare Reihen plus Puffer. Der Scrollanker
bleibt bei Änderungen der Rasterbreite erhalten. Details werden erst bei Auswahl angefordert,
profilgebunden bis 64 Titel gecacht und vor verspäteten Antworten alter Auswahl geschützt.
Rust cached passende Indizes statt aller Metadaten; nur sichtbare Karten und die ausgewählten
Details werden kopiert. Sortierte Episoden werden einmal zwischengespeichert. Die Resume-
Ermittlung benötigt keine Kopie/Sortierung sämtlicher Episodentexte mehr.

Kategorien sind breiter, die Hauptnavigation klappt auf Icons ein und bei Rückkehr des Fokus
wieder auf. Filme und Serien zeigen wieder Poster und Titel darunter. Beschreibung maximal
zwei Zeilen; Volltext über „Mehr lesen“. Staffelwahl bleibt eine horizontale Reihe, Folgen sind
direkt darunter in einer eigenen Scrollfläche. Gespeicherter Fortschritt positioniert Staffel
und Folge beim Öffnen; Beispiel Staffel 5/Folge 6, einschließlich bestehender 60-Sekunden-Regel.

## Prüfung

- Rust: 46 Tests einschließlich 20.000-Titel-Raster, begrenzter gezeichneter Titelzahl,
  automatischer Staffelwahl und unveränderter 60-Sekunden-/Staffelgrenzen-Regel.
- TypeScript und zehn Unit-/Integrationstests; 17 UI-Tests in eigenem Headless Edge.
- UI-Test mit 20.000 Filmen: weniger als 80 Poster-Elemente gleichzeitig im DOM, auch nach
  tiefem Scrollen; kein erneuter Seriendetail-Request beim Wiederöffnen derselben Serie.
- Staffel 5/Folge 6 vorpositioniert, Staffelbuttons einzeilig, Ziel-Folge vollständig sichtbar;
  Beschreibung öffnet/schließt ohne Verlust der Serienansicht.
- Navigation, Vorschau, Tastatur und Live-Fernbedienung bleiben durch Regressionstests abgedeckt.

Anbieter-Netzwerkzeiten können durch die lokale Virtualisierung nicht beseitigt werden.
Gemessene Browser-/Emulatorprüfungen sind kein Nachweis für Reaktionszeiten auf jedem TV.
iOS benötigt weiterhin einen Apple-SDK-Build; der Host-Typcheck allein erzeugt keine IPA.
Die bisherige GitHub-Billing-Sperre wird vor Abschluss erneut geprüft. Keine alte IPA als neuen
Build veröffentlichen. Signierschlüssel und vorherige funktionierende Pakete bleiben erhalten.


Tizen 0.1.6: signiertes Paket vom Samsung-TV angenommen, Update ohne Deinstallation und
App-Start erfolgreich (PID 19350). SHA-256 des signierten Pakets:
80ee971f9aa1fcfb162122b0af9e2dca5ef54879eeb1e521c3c2cef66a2c2711.
Die visuelle Layout-/Fokusabnahme auf dem physischen TV steht noch aus; Installation und
Start allein belegen noch keine Reaktionszeit oder vollständige Bedienbarkeit auf Hardware.
