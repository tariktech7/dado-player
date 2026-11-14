# Gemeinsamer TV-UI-Vertrag

Gilt für Android-TV-Modus und zukünftige LG-/Samsung-Clients. Basis: Android 0.2.6.

## Fläche und Spalten

Logische Referenzfläche: 960×540 Punkte. Full HD bei 320 dpi und 4K bei 640 dpi
behalten dieselbe Dichte. 55 und 77 Zoll verwenden dieselbe Aufteilung;
die Diagonale allein darf weder Schrift noch Bedienelemente vergrößern.
Safe Area: 5 % pro Rand, mindestens 20 Punkte. Für Web-Clients zunächst die tatsächliche
CSS-Fläche messen und das Raster darauf skalieren. Browser-Zoom, Geräteskalierung
und Overscan auf Hardware prüfen; 4K bedeutet keine Halbierung der Schriftgröße.

Vier Bereiche: Hauptnavigation → Kategorien → Sender → optionale Vorschau.
Oben liegen „Dado Player“, „Kategorien“ und der aktive Sendertitel („Alle Sender“)
auf einer horizontalen Mittellinie. Die Suche liegt direkt darunter in der Senderspalte;
Kategorien beginnen schon auf Höhe der Suche. Kein Kategorien-Dropdown und keine
mehrstündige EPG-Zeitachse in der Senderliste.

Navigation und Senderspalte: dezenter dunkler Hintergrund (Schwarz mit Alpha 28/255
über der Grundfläche). Kategorien und Vorschau: Grundfläche. Die Spaltenflächen
laufen über die volle Bildschirmhöhe; Bedienung bleibt innerhalb der Safe Area.

Navigation: 16 % der sicheren Breite, begrenzt auf 128–176 Punkte, danach 12 Punkte Abstand.
Kategorien: 18 % der restlichen Inhaltsbreite, begrenzt auf 116–168 Punkte. Abstand
zwischen Inhaltsspalten: 10 Punkte. Vorschau: 37 % der Inhaltsbreite, begrenzt auf
220–420 Punkte. Der Rest gehört den Sendern. Lange Titel werden gekürzt;
die Vorschau zeigt die Details.

## Kompakte Senderzeile

Mindestens zehn vollständige Zeilen bei 960×540 mit geöffneter Vorschau.
Zeilenhöhe `(verfügbare Listenhöhe − 9×3) / 10`, begrenzt auf 34–40 Punkte.
Abstand 3 Punkte. Name 13 Punkte, EPG 10 Punkte. Gemessene Schrifthöhen bestimmen die
vertikale Zentrierung. Senderlogo in einem 24×24-Punkte-Slot, vollständig proportional
eingepasst (`contain`), ohne Strecken oder Abschneiden. Das gilt auch für Windows und Handy.

Zweite Textzeile: „Jetzt: …“ links, „Danach: …“ rechts. Beide Bereiche gleich breit,
dazwischen 24 Punkte freier Abstand. Vollständige Titel, Zeiten und Beschreibungen
stehen in der Vorschau. Fehlendes EPG erzeugt keine dritte Zeile.
Suche: 28 Punkte Höhe, danach 4 Punkte Abstand.

## Fernbedienung und Abnahme

Tizen-Umsetzung: Auswahlfelder bleiben beim Fokussieren geschlossen; erst OK öffnet
eine Dado-Liste, Zurück verwirft sie. Native HTML-select-Elemente dürfen die räumliche
Navigation nicht übernehmen. Die Bildschirmtastatur verkleinert die sichtbare Höhe,
nicht das TV-Raster: Skalierung aus der Breite ableiten, aktive Formularbereiche auf
die verbleibende Höhe begrenzen und das Eingabefeld dorthin scrollen. Dies auch beim
späteren LG-Client prüfen.

- Pfeile bewegen den sichtbaren Fokus räumlich; Listen scrollen den Fokus mit.
- OK auf einem Sender öffnet die Vorschau. Erneutes OK auf demselben Sender oder
  OK auf der Vorschau öffnet den vollständigen Player.
- Kategorienwechsel beendet die alte Vorschau. Favoriten sind eigene Daten, kein Verlauf.
- Zurück schließt Dialoge, Player, Vorschau und Unterseiten in dieser Reihenfolge.
  Erst am Hauptmenü wird die Plattformaktion zum Verlassen/Hintergrund verwendet.
- Fokusrahmen ergänzt Auswahlfarbe; Auswahl nie ausschließlich durch Farbe vermitteln.

Vor Release: Full HD/4K, lange Kategorien/Titel, breite/hohe Logos, mehrere tausend
Sender, fehlerhaftes EPG, Fokus nach Zurück und Scrollen bis zum letzten Element prüfen.
55-/77-Zoll-Ansicht mit realistischem Sitzabstand und Wiedergabe, Suspend, Screensaver,
Audio-/Untertitelwahl und Wiederaufnahme separat auf Hardware abnehmen.
Android-Emulator-Aufnahmen ersetzen keine LG-/Samsung-Geräteprüfung.
