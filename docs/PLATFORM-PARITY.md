# Gemeinsamer Produktstand 0.6.2

`release.json` ist die Versionsquelle für Windows, Android, iOS und Samsung. Android-Buildnummer
16 und iOS-Buildnummer 11 bleiben eigenständig monoton steigend. Nach Änderungen
`python scripts/release_version.py` ausführen; alle Build-Einstiege prüfen auf Abweichungen.

## Verbindliche Abläufe

| Funktion | Gemeinsamer Vertrag |
| --- | --- |
| Anmeldung | Gespeichertes Profil beim Neustart wieder öffnen, explizites Abmelden beachten. |
| Daten je Anbieter | Favoriten, Verlauf, Positionen und Laufzeiten je Profil isolieren. Gleiche Sender-/Film-IDs anderer Anbieter dürfen nicht kollidieren. Alte native Dateien werden einmal dem bisherigen Standardprofil zugeordnet und als Sicherung erhalten. |
| Startseite | Fortsetzen mit Artwork, kompakte Senderlogos, unabhängiges aktuelles Favoritenprogramm. |
| Favoriten / Fortsetzen | Live-TV, Filme und Serien getrennt anzeigen. |
| Kataloge | Filme/Serien: Favoriten zuerst; Poster und eigene Detailansicht. Live-TV: alle Sender und Favoriten. |
| Serien | Staffelzeile horizontal, Folgen als Folge 1 usw.; letzte Staffel/Folge vorpositionieren. |
| Fortsetzen | Direkt fortsetzen. Bei höchstens 60 Sekunden Rest die nächste Folge; fertige Filme neu beginnen. Noch fehlende Episodendaten asynchron laden und einen überholten Startauftrag verwerfen. |
| Player | Eigene Dado-Steuerung; nach zwei Sekunden ausblenden. Live-TV: Hoch/Runter schaltet, 0 ruft letzten Sender auf, soweit das Eingabegerät diese Tasten liefert. |
| Ton / Untertitel | Bevorzugte Sprachen mit gemeinsamen ISO-Aliasen, Untertitel automatisch ein/aus. Manuelle Auswahl bleibt während des Streams erhalten. Verfügbare Spuren kommen vom Stream/Player. |
| Verlauf | Maximal 150 Einträge. Automatischer Start des letzten Streams bei neuen Installationen aus; vorhandene bewusste Einstellung erhalten. |

Die Werte für Zeitgrenzen, Verlaufslimit und den Standard für automatischen Start kommen aus
`shared/product-policy.json`. Native Konstanten werden erzeugt, der Web-TV-Client importiert die Datei.

## Bewusste Plattformunterschiede

| Bereich | Windows / Android | iOS | Samsung |
| --- | --- | --- | --- |
| Player | libmpv | AVPlayerLayer mit Dado-Overlay; Apple-Steuerung optional | AVPlay |
| Eingabe | Maus/Tastatur bzw. Touch und Android-TV-Fernbedienung | Touch, native Inline-Tastatur | Fernbedienung, Tastatur erst nach Aktivierung |
| Touch-UI | Android: Kategorien-Dropdown, Suche, volle Detailseite mit Rand-Zurück | Gleiches Touch-Konzept | Spalten, Fokusgedächtnis, feste Vorschau |
| TV-/Desktop-UI | Kategorien, Poster/Sender, Detailbereich; unabhängige Fokuszonen | Nur bei breiter Darstellung/TV-Modus sinnvoll | Spalten und virtuelle Listen |
| Lizenz | Windows: 7 Tage Trial, manuelle Dauerfreigabe; Android: noch keine Kaufsperre | Keine Kaufsperre bis IAP | Gleiche Trial-/Freigabelogik wie Windows |
| Updates | Portable ersetzt eigene EXE; Android öffnet Systeminstaller | IPA herunterladen, mit eigener Apple-ID neu signieren/installieren | Signatur, TV-Berechtigung und Samsung-Verteilweg erforderlich |
| Puffer/Timeout | libmpv-Netzwerkoptionen | Automatisch AVPlayer für HLS / VLC für Dateien, begrenzter Fallback; Puffer und Lade-Watchdog | AVPlay-Optionen nach Geräteunterstützung |

Identische Bedienabläufe bedeuten keine identischen Codec-, DRM-, Installations- oder Store-Fähigkeiten.
Ein User-Agent erweitert keine Codec-Unterstützung. Samsung-USB-Demo-Lizenzen sind unabhängig
von der Dado-Dauerlizenz und können selbst zeitlich oder auf Geräte beschränkt sein.

## Release-Abnahme

- Rust: Formatierung, Tests aller Targets, Clippy; Android zusätzlich echter NDK-/Gradle-Build.
- iOS: echter Apple-SDK-Gerätebuild und ABI-Prüfung. Eine unsigned IPA ist noch keine Gerätesignatur.
- Web-TV: Typecheck, Unit-Tests, Browser-Fernbedienungstests, Paket-/Signaturprüfung.
- Android-Emulator mit Testanbieter für Tabs, Details, Scrollen, Eingabe und Rotation; echte Hardware separat ausweisen.
- Echte iPhone-/TV-Wiedergabe, Overscan und Fernbedienung erst nach Gerätetest als bestätigt bezeichnen.
- Alte Pakete archivieren. Windows aktuell ausschließlich Releases/Dado-Player.exe.
- Eine WGT niemals als USB-installierbar ausgeben. Dafür müssen von Samsung erzeugte TMG- und Lizenzdateien vorliegen.

## Automatik und Experten (0.6.1)

Standard ist integrierte Wiedergabe mit Dado-Steuerung. Android benötigt keine
externe Player-App. iOS verwendet AVPlayer für HLS und den eingebetteten
VLC-Decoder für Dateicontainer; ein AVPlayer-Fehler kann einmal auf VLC wechseln.
Der optionale Apple-Player erzwingt AVPlayer und dessen Formatgrenzen.
Samsung verwendet im Automatikmodus AVPlay mit einmaliger HTML-Alternative bei
Fehlern. Diese Alternative erweitert nicht die Hardware-Codecs des Fernsehers.
Decoderwahl, externe Android-Apps und Netzwerkexperimente stehen im Expertenmodus.
Filme/Folgen behalten die tatsächliche Anbieter-Dateiendung auf allen Plattformen.

Samsung: Einstellungen haben eigene Fokuszonen pro Bereich. Links/Rechts bewegt
sich durch die benachbarten Spalten, Hoch/Runter erreicht weitere Bereiche der
aktuellen Spalte. Zurück schließt zunächst offene Dialoge/Player-Stufen und führt
anschließend zur Hauptnavigation desselben Menüs. Dort öffnet Zurück die
Beenden-Abfrage. Die schmale Leiste ändert ihr Raster nur einmal pro Wechsel;
Animationen verwenden transform/opacity und respektieren reduzierte Bewegung.
Ein Browser-Test der Mechanik ersetzt keine Messung der Framerate auf dem TV.
