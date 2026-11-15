# Gemeinsamer TV-UI-Vertrag

Gilt für Android-TV-Modus, Samsung Tizen und zukünftiges LG webOS. Basis: Android 0.2.8 / Tizen 0.1.6.

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
Kategorien im nativen TV-Modus: 25 % der restlichen Inhaltsbreite, begrenzt auf 170–230 Punkte. Abstand
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


## Gemeinsamer Ablauf ab 14.11.2026

- Filme und Serien: Navigation → Kategorien → Poster-Raster → Details. Favoriten steht zuerst in der
  Kategorienliste. Der rechte Bereich bleibt auch ohne Auswahl reserviert.
- Live-Vorschau ist immer sichtbar, mit einer leeren Hinweiskarte vor der ersten Auswahl.
- Bereiche behalten eigene Kategorie, Suchtext, Auswahl und Scrollposition. Ein Wechsel setzt
  keinen anderen Bereich auf dessen aktuelle Scrollposition. Neue Profile starten getrennt.
- Serien: Staffelbuttons, nur Folgen der ausgewählten Staffel, ausschließlich „Folge N“ als
  Folgenlabel. Ein Play-Button startet die zuletzt gesehene Folge; ab höchstens 60 Sekunden
  Rest beginnt die nächste Folge. Ohne bekannte Dauer wird nicht vorschnell weitergeschaltet.
- Favoriten und Fortsetzen: Live-TV, Serien und Filme als unabhängig wählbare Rubriken.
- Startseite: persönliche Begrüßung, Fortsetzen mit Artwork, eigene Favoriten-EPG-Spalte rechts
  auf breiten Displays; auf Telefonen untereinander. Verlauf und Favoriten sind getrennte Daten.
- Horizontaler Fernbedienungsfokus bleibt innerhalb einer Reihe und scrollt den zugehörigen
  Container in beide Richtungen. Der erste Eintrag stellt die ursprüngliche Randposition wieder her.
- Live-Vollbild: Hoch/Runter wechseln Sender; 0 schaltet zwischen den letzten beiden Sendern um.
  Navigation in Listen bleibt davon unberührt. Eigene Player-Overlays verschwinden nach zwei
  Sekunden ohne Interaktion; offene Einstellungsmenüs und pausierte Wiedergabe bleiben bedienbar.
- Externe Android-Player verwalten ihre eigenen Overlays/Fernbedienung. MX Player/VLC bekommen
  die Startposition und liefern bei unterstützter Activity-Result-API Position und Dauer zurück.
  Systemauswahl/andere Player garantieren diesen Rückkanal nicht.

Diese Abläufe gelten auch als Implementierungsvertrag für spätere LG-webOS- und ältere
Samsung-Versionen; deren Plattform-Player und Installationswege bleiben separat.


Fokusvertrag: Jede Spalte merkt sich den letzten fokussierten Eintrag über dessen stabile
Identität. Links aus einer Senderliste kehrt zur zuletzt fokussierten Kategorie zurück; Rechts
kehrt zum Sender zurück. Die Höhe des aktuellen Elements bestimmt diese Rückkehr nicht.
Das gilt für Navigation, Kategorien, Titel und Details sowie die zwei Startseitenspalten.
Auf horizontalen Reihen bewegt Links/Rechts zunächst innerhalb der Reihe; am Ende ist die
benachbarte Spalte erreichbar. Hoch/Runter bleibt in der aktuellen Spalte.


## Poster und kompakte Seriendetails, Version 0.1.6 / 0.2.8 / 0.5.5

Hauptnavigation: beim Fokus in Inhaltsspalten zu einer reinen Icon-Leiste einklappen;
Rückkehr des Fokus in die Hauptnavigation klappt sie wieder auf. Tizen: 276 → 80
Referenzpixel, Kategorien 320 Referenzpixel. Native TV-Leiste: 128–176 → 48 Punkte.
Desktop: 224 → 60 Punkte, Aufklappen über Maus oder Pfeil links. Telefon nutzt weiterhin
seine untere Navigation. Fokusgedächtnis bleibt bei der Größenänderung erhalten.

Filme und Serien verwenden Poster mit zweizeiligem Titel darunter. Die Raster erzeugen nur
sichtbare Reihen plus einen kleinen Puffer; keine vollständige UI-Liste eines großen Katalogs.
Kategorien/Suchfilter werden beim Ändern ihrer Eingaben berechnet, nicht pro Player-Statusupdate.
Einzelne Anbieter-Details werden erst nach Auswahl geladen und zwischengespeichert.

Seriendetails: kompakter Kopf, Beschreibung höchstens zwei Zeilen, „Mehr lesen“ für den Volltext.
Staffelbuttons stehen in einer horizontal scrollbaren Zeile. Die Folgenliste darunter scrollt
unabhängig vom Kopf und bleibt sofort sichtbar. Ohne neue Auswahl springt das Öffnen anhand
des gespeicherten Fortschritts zur passenden Staffel und Folge; die bestehende 60-Sekunden-
Regel für den Play-Button gilt weiterhin. Staffelwechsel setzt nur die Folgenansicht um.

## Tizen 0.6.1: Fokus und Bewegung

Die Einstellungen besitzen unabhängige Zonen für Lizenz, Darstellung,
Wiedergabe, Ton/Untertitel, Netzwerk und Profile/Updates. Links aus der rechten
Spalte führt zuerst zur benachbarten linken Einstellungsgruppe. Erst von dort
geht es zur Hauptnavigation. Hoch/Runter erreicht weitere Gruppen derselben
Spalte; jede Gruppe merkt ihren letzten Eintrag. Checkboxen sind keine aktiven
Texteingaben und dürfen die Richtungstasten nicht verschlucken.

Zurück schließt zunächst aktive Eingaben, Auswahllisten, Dialoge und die
bestehenden Vollbild-/Vorschaustufen. Im Inhaltsmenü führt Zurück zur
Hauptnavigation, ohne das Menü auf Startseite umzuschalten. Ein weiteres Zurück
in der Hauptnavigation zeigt die bestätigungspflichtige Beenden-Abfrage.

Keine Animation von grid-template-columns oder Breiten über mehrere Frames:
Nach einmaligem Layoutwechsel werden Spalten/Icon-Positionen mittels FLIP und
Web Animations mit translate3d über 220 ms bewegt. Titel verschwinden per
opacity/transform. Fokuswechsel verursachen kein Resize/Repaint des gesamten
App-Baums. Schnelle Richtungswechsel übernehmen die gerade sichtbare Position.
Reduzierte Bewegung deaktiviert diese Animationen. Native AVPlay-Geometrie wird
nach Abschluss synchronisiert; dessen feste rechte Vorschau bleibt verankert.

## 0.6.2: Auswahl, Scrollen und Film-Tasten

OK auf einem Film-/Serienposter fokussiert direkt Abspielen/Fortsetzen in der
rechten Detailspalte. Bei asynchron geladenen Folgen wird genau dieser eine
Fokuswechsel nachgeholt; spätere Metadaten dürfen den Fokus nicht zurücksetzen.
Die vorherige Auswahl bleibt beim Zurückwechseln in die Titelliste erhalten.

Links/Rechts spult Filme und Folgen zunächst um jeweils 10 Sekunden. Hoch/Runter
aktiviert die Navigation der sichtbaren Player-Bedienelemente. Verschwindet das
Overlay, gilt wieder die Spulfunktion. Live-TV behält die vorhandene Kanalwahl.
Eingaben vor fertig vorbereitetem AVPlay werden als relative Spulschritte
zwischengespeichert. Ein unbekannter Dauerwert begrenzt Spulen nicht auf Null.

Fokus-Scrolling nutzt die native sanfte Browser-Animation und bündelt identische
Scrollziele; reduzierte Bewegung bleibt sofort. Virtuelle Listen rendern erst
beim Wechsel einer Reihe statt für jedes Pixel. Die Senderliste behält bei
schnellen Richtungstasten den logischen nächsten Sender im DOM, ohne zur
Vorschau abzuspringen. Android TV/Windows nutzen animiertes egui-Scrolling mit
minimal nötigem Nachscrollen statt ständigem Zentrieren.
