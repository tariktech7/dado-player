# Gemeinsamer Produktstand 0.7.8

`release.json` ist die Versionsquelle für Windows, Android, iOS und Samsung. Android-Buildnummer
27 und iOS-Buildnummer 22 bleiben eigenständig monoton steigend. Nach Änderungen
`python scripts/release_version.py` ausführen; alle Build-Einstiege prüfen auf Abweichungen.

## Verbindliche Abläufe

| Funktion | Gemeinsamer Vertrag |
| --- | --- |
| Anmeldung | Gespeichertes Profil beim Neustart wieder öffnen, explizites Abmelden beachten. |
| Daten je Anbieter | Favoriten, Verlauf, Positionen und Laufzeiten je Profil isolieren. Gleiche Sender-/Film-IDs anderer Anbieter dürfen nicht kollidieren. Alte native Dateien werden einmal dem bisherigen Standardprofil zugeordnet und als Sicherung erhalten. |
| Startseite | Fortsetzen mit Artwork, kompakte Senderlogos, unabhängiges aktuelles Favoritenprogramm. |
| Favoriten / Fortsetzen | Live-TV, Filme und Serien getrennt anzeigen. |
| Kataloge | Filme/Serien: Favoriten zuerst; Poster und eigene Detailansicht. Live-TV: alle Sender und Favoriten. |
| Seitenleiste (Windows und Samsung) | Öffnet sich beim Überfahren mit der Maus. Ohne Auswahl klappt sie 700 ms nach dem Verlassen ein. Ein Klick auf einen Menüpunkt lässt sie geöffnet. |
| Player-Overlay (Windows und Samsung) | Abgesetzte, abgerundete Playerleisten mit Bild hinter der Oberfläche, gemeinsamen Farben und vier Elementen in der Hauptleiste. Ton, Untertitel, Bildformat und Beenden stehen unter Optionen. Windows behält die native Vollbild- und Lautstärkesteuerung. |
| Live-Vorschau (Windows und Samsung) | Gemeinsame Breite von 640 Einheiten bei 1920 × 1080, 16:9-Fläche und vollständiges Streambild mit erhaltenem Seitenverhältnis. Die Programmdetails bleiben separat scrollbar. |
| Liste aktualisieren | Sender, Filme und Serien des aktiven Profils neu laden. Bei einem Teilfehler bleiben die bisherigen Einträge des betroffenen Bereichs verfügbar. |
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

## Wiedergabe und Bildtakt (0.6.3)

Samsung serialisiert asynchrone Spulvorgänge. Ein abgelehnter Spulpunkt ist eine
Hinweismeldung; ein fataler Fehler beendet den Decoder vollständig. Zurück aus
einem Film/einer Folge stoppt die Wiedergabe, Live-TV darf zur Vorschau wechseln.
Alte Callbacks dürfen nach Stop oder Quellenwechsel keine Wiedergabe reaktivieren.

iOS legt Video in ein eigenes nicht interaktives Hintergrundfenster unter dem
winit/Metal-Eingabefenster. Apple Player bleibt optional. Die neue Fensteranordnung
ist kompiliert, ihre Sichtprüfung auf einem iPhone übernimmt der Nutzer.
Native Bild-Uploads werden pro Frame begrenzt. iOS/Android melden dem Betriebssystem
den bevorzugten Displaytakt; iOS pausiert den Timing-Hinweis bei ruhender UI.
Diese Hinweise garantieren keine bestimmte reale Framerate oder Decoderfähigkeit.

## Scroll-Arbeit pro Frame (0.7.2)

Samsung navigiert Posterreihen anhand stabiler Rasterindizes; laufende Scrollbewegungen
werden mit einem einzigen requestAnimationFrame-Takt weitergeführt und neu ausgerichtet.
Keine globale Vermessung aller Kategorien pro Richtungstaste. Virtuelle Listen verwalten
ihre Spacer selbst (overflow-anchor: none); der erste montierte Eintrag ist nicht zwingend
der Listenanfang. Verminderte Bewegung bleibt sofort, direkte Benutzerbewegung unterbricht
die automatische Bewegung. Der Takt verwendet verstrichene Zeit statt einer festen Framezahl.

Der gemeinsame native Katalog verarbeitet nur sichtbare Kategorien plus Nachbarn. Bilder
werden begrenzt dekodiert, auf höchstens 640 Pixel Kantenlänge für Poster/Detail-Miniaturen
und 1280 für übrige Artwork-Flächen verkleinert und im Hintergrund in ColorImage umgewandelt.
Der UI-Thread erhält fertige Pixel; vier laufende Bildanforderungen und zusammen acht
Anforderungen/fertige Bilder begrenzen die Warteschlange. Der volle Cache darf neue Bilder
nicht blockieren, und fertige Bilder dürfen vor dem GPU-Upload nicht erneut geladen werden.
Die begrenzten Uploads pro Frame und vorhandene VSync-/Displaytakt-Anbindung bleiben erhalten.

## Dado Connect

Alle aktuellen Clients: genehmigte Senderkonten, MAC-/Geräteadresse ohne Kopplungscode, M3U-URL/Xtream und manueller Aktualisieren-Abruf. Dauerhaft speichern vor Bestätigung; stabile Nachrichten-IDs verhindern Duplikate. Kein automatisches Login oder Überschreiben bestehender Profile. Mobile zeigt private Geräteadresse statt Hardware-MAC. LG muss diesen Vertrag künftig übernehmen, ist noch nicht gebaut. Siehe connect/README.md.

## Elternsperre ab 0.7.3

Windows, Android/Android TV, iOS und Samsung: vierstellige Eltern-PIN unter Einstellungen,
einzelne Sender/Filme/Serien pro Anbieterprofil sperren. Alle Folgen einer gesperrten
Serie geschützt; Wiedergabe, Vorschau, Favoriten und Fortsetzen prüfen zentral. PIN
für Änderungen und jeden neuen gesperrten Wiedergabestart. Lokale Speicherung, keine
Cloud-Abhängigkeit. Vertrag und Grenzen: [PARENTAL-CONTROLS.md](PARENTAL-CONTROLS.md).

## Gemeinsame Darstellung 0.7.4
Windows nutzt `src/ui/desktop.rs` als normale breite Oberfläche; Samsung bleibt Web-TV.
Beide lesen Spaltenmaße und Bewegung aus `shared/shell-appearance.json`.
Normale steel-blue Palette in `shared/normal-palette.json`; Dunkel ist weiches Mitternachtsblau,
Hell weiß mit blauen Akzenten. Kein Lavendel-Modus. Alte gespeicherte `Warm`/`warm`-Werte
bleiben lesbar und wählen nun Blau. Mobile Tabs und volle Detailseiten bleiben eigenständig.


## Einstellungen und Player ab 0.7.5

Die Reihenfolge der Bereiche stammt aus `shared/settings-sections.json`. Die Liste
der Ton- und Untertitelsprachen kommt aus `shared/playback-languages.json`; vorhandene
eigene Sprachcodes bleiben verfügbar. UI-Sprache und 12/24-Stunden-Anzeige werden
je Gerät gespeichert. Bestehende Einstellungen starten mit Deutsch und 24 Stunden.
EPG-Uhrzeiten verwenden lokale Gerätezeit, Laufzeiten bleiben Minuten und Sekunden.

Windows und Samsung teilen Maße der Playerleisten und die Sprungweite von zehn
Sekunden. Windows verwendet libmpv und Windows-spezifische Audio- und Vollbildregler;
Samsung verwendet AVPlay und die Tizen-Steuerung für Bildformat und Fernbedienung.


Beim Wechsel der Live-Kategorie bleibt der Stream in der Vorschau aktiv,
auch wenn der Sender zu einer anderen Kategorie gehört. Die 0-Taste wechselt
zwischen den letzten beiden Live-Sendern im aktuellen Profil, in der Vorschau
und im Vollbild. Beim Auswählen eines Films oder einer Serie springt der
Fernbedienungsfokus in die Details und auf die Wiedergabe, sobald sie bereit ist.
