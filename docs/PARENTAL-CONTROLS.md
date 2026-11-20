# Elternsperre · ab 0.7.3

Windows, Android einschließlich TV-Modus, iOS und Samsung Tizen besitzen unter
Einstellungen einen Bereich **Elternsperre**. Vier Ziffern eingeben und bestätigen;
danach Sender, Filme oder Serien suchen und einzeln sperren. Die Auswahl gehört zum
jeweiligen Anbieterprofil auf diesem Gerät. Gesperrte Serien schließen sämtliche
Staffeln und Folgen ein. PIN ändern und Sperren entfernen sind ebenfalls geschützt.

Jeder neue Wiedergabestart eines gesperrten Inhalts benötigt die PIN, einschließlich
Vorschau, Favoriten, Suche, Fortsetzen, letztem Sender und Folgewiedergabe. Die
Freigabe gilt nur für den angeforderten Start; das Vergrößern derselben bereits
freigegebenen Vorschau unterbricht keine laufende Wiedergabe. Abbrechen startet
keinen Stream. Vor einer PIN-Abfrage wird vorhandene Wiedergabe gestoppt, damit
weder Bild noch Ton hinter dem Dialog weiterlaufen.

Die PIN wird mit einem zufälligen 16-Byte-Salt und PBKDF2-HMAC-SHA256 (100.000
Iterationen) gespeichert. Keine Klartext-PIN, kein Standardcode, kein Cloud-Aufruf.
Nach fünf falschen Eingaben folgen 30 Sekunden Wartezeit; Abbrechen setzt die
Fehlversuche nicht zurück. Neustart erhält die Sperrliste, keine Freigabesitzung.
Speicherfehler werden angezeigt und übernehmen keine ungespeicherten Änderungen.
Beschädigte vorhandene Schutzdaten schalten den Schutz nicht stillschweigend aus.

Native Ziele teilen `src/parental.rs` und `src/ui/parental.rs`; die zentralen
Einstiege `App::play` und `App::start_preview` prüfen vor jeder Übergabe an den
integrierten oder externen Player. Samsung verwendet `parental.ts` und
`parental-ui.tsx` vor `start()`. Die Serienkennung wird aus `ep:<serie>:<folge>`
beziehungsweise `seriesId` abgeleitet. Kind und Anbieterprofil trennen gleichnamige
IDs verschiedener Kataloge. Separate Speicherung (`parental.json` beziehungsweise
`dado.parental.v1`) verhindert, dass gewöhnliche Einstellungsänderungen oder
Profilimporte die PIN zurücksetzen. Dies ist eine lokale App-Sperre, kein Schutz
gegen Änderungen der App-Daten außerhalb der App. Keine plattformübergreifende
Synchronisierung von Eltern-PINs oder Sperrlisten.

Fernbedienung und Touch verwenden ein Ziffernfeld. Die Verwaltung lädt nur einen
begrenzten Ausschnitt der Treffer; der native Client hält den Suchindex zwischen
Frames fest. LG übernimmt diesen Vertrag bei einer künftigen Portierung und ist
weiterhin kein gebautes Ziel.
