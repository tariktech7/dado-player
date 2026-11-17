# Samsung USB-Verteilung: Stand 16.11.2026

## Aktualisierung 17.11.2026: Partnerantrag abgelehnt

Laut vom Nutzer bereitgestellter Antwort von Christian Eickhoff wurde die
Partneranfrage für den generischen IPTV-Player abgelehnt. Die Antwort nennt eine
Vorgabe der EU-Zentrale gegen weitere generische Player ohne eigenes Inhaltsangebot.
Damit sind der deutsche Store-Weg und das partnergebundene USB-Werkzeug aktuell
nicht freigegeben. Keine neue Anfrage oder Umgehung wird automatisch gestartet.
Entwicklerinstallation über Netzwerk bleibt der vorhandene Testweg. Die folgenden
Einträge dokumentieren den vorangegangenen Ablauf, keine aktuelle Partnerfreigabe.

Samsung beschreibt in der offiziellen [Installations-FAQ](https://developer.samsung.com/smarttv/develop/faq/application-installation.html)
eine Umwandlung über das USB Demo Packaging Tool im TV Seller Office. Eine normale WGT lässt sich
auf Consumer-TVs nicht direkt vom USB-Stick installieren. Die erzeugten Samsung-Dateien wären
eine TMG und eine zugehörige Lizenzdatei. Das lokale Entwicklerzertifikat ersetzt diese nicht.

Die Gruppe **DadoPlayer** wurde am 15.11.2026 erfolgreich als **Public Group** angelegt.
Nach erneuter Anmeldung sind Group Information und Group Members verfügbar. Ein erster
Versuch hatte beim Abruf der Gruppendaten einen 401-Fehler geliefert; dieser Fehler ist kein
Nachweis für eine ungültige Homepage oder Support-Adresse. Persönliche Verkäuferdaten werden
hier nicht dokumentiert.

Auch nach erfolgreicher Gruppenerstellung enthält das tatsächlich angezeigte Hauptmenü nur
Applications, Membership, 1:1 Q&A, Notification, Guide und Reports. Ein Tools-/USB-Packaging-Menü
ist für dieses Konto weiterhin nicht sichtbar.

Eine Gruppenerstellung allein ist noch keine bestätigte Berechtigung für USB-Packaging.
Die Oberfläche unterscheidet Public Groups und Partner Groups; für letztere nennt sie einen
Samsung-Vertrag und die Zustimmung eines Content Managers. Die aufgerufene Partnerschaftsseite
verlangt ausdrücklich die E-Mail-Adresse des eigenen Samsung Content Managers und einen Kommentar.
Ein formaler Partnerschaftsantrag wurde nicht abgesendet. Ohne einen entsprechenden
Samsung-Kontakt kann dieses Formular nicht wahrheitsgemäß abgeschlossen werden.
Der tatsächliche Zugang zum USB-Werkzeug und die Kompatibilität mit dem Ziel-TV müssten auch
nach einer solchen Freigabe erneut geprüft werden.

Mit ausdrücklicher Nutzerfreigabe wurde am 15.11.2026 eine gemeinsame Supportanfrage eingereicht:
**Ticket 2646069523**, Kategorie App Management → Group management, bestätigter Status **Submitted**.
Sie nennt ausdrücklich die geplante Store-Veröffentlichung in Deutschland/Europa und fragt nach
dem zuständigen Content Manager, den Partneranforderungen und dem USB-Testzugang. Sie beschränkt
das Projekt nicht auf private Nutzung. Das Supportformular verlangte keine App-Verknüpfung und
keinen Dateianhang; es wurde keine WGT als Store-Zertifizierung eingereicht.
Ticket: https://seller.samsungapps.com/tv/qna/detail/2646069523
Wortlaut: SAMSUNG-REQUEST.txt. Store-Vorbereitung: STORE-READINESS.md.

## Supportantwort und nächster Schritt (16.11.2026)

Ticket 2646069523 steht im tatsächlich geöffneten Seller Office auf **Answered**.
Samsung verweist für Deutschland an **Christian Eickhoff, c.eickhoff@samsung.com**
und bestätigt ausdrücklich: USB Demo Packaging ist nur für Samsung-Partner verfügbar.
Dies ist noch keine Freigabe, kein Vertrag und kein Zugang zum USB-Werkzeug.

Die geöffnete Membership-Seite bestätigt außerdem: Public Groups können Apps nur
in den USA verteilen. Für das eigentliche Store-Ziel Deutschland/EU ist daher die
Partneraufnahme erforderlich, unabhängig davon, ob USB später nutzbar ist.

Das Partnerformular ist erreichbar und verlangt Content-Manager-E-Mail,
„Check Validity“ sowie einen Kommentar mit höchstens 500 Bytes. Der mit ausdrücklicher
Nutzerfreigabe **abgesendete 449-Byte-Antrag** liegt in PARTNER-REQUEST-2026-11-16.txt.
Er nennt den Entwickler als Studenten, priorisiert die Store-Veröffentlichung und
nennt USB nur als optionalen Testweg. Angaben zu Trial, Preis und Freischaltung wurden
auf ausdrücklichen Nutzerwunsch vor dem Absenden entfernt. Der Antrag erklärt,
dass noch kein Partnervertrag besteht.

Beobachtet: „The email entered is valid“, anschließend Done → Bestätigungsdialog →
Confirm und automatische Rückleitung zur Gruppenübersicht ohne angezeigten Fehler.
Eine erneute Ansicht der Partnership-Seite zeigt ein leeres Formular; eine dauerhafte
Antragsnummer oder Pending-Anzeige wurde nicht angezeigt. Es wurde kein zweiter Antrag
gesendet. Die Gruppe bleibt Public Group; Partnerfreigabe und USB-Zugang stehen noch aus.

## Vorbereiteter Buildweg

Im Client-Verzeichnis nach Erstellen der signierten WGT:

```text
npm run usb
```

Erzeugt ausdrücklich nur ein Vorbereitungsarchiv mit Quell-WGT, Prüfsumme und
`installable: false`. Dieses Archiv ist kein TV-USB-Installer.

Wenn Samsung die beiden benötigten Dateien ausgestellt hat:

```text
npm run usb -- --tmg <Samsung-Datei.tmg> --license <widget.license>
```

Der Befehl bündelt sie unter `userwidget/`. Er prüft Dateitypen, leere Dateien und eine bloß
umbenannte WGT, ersetzt aber keine Prüfung der Samsung-Signatur, Lizenzgültigkeit oder TV-Installation.
Die Lizenz kann eigene Ablauf- und Gerätebedingungen haben, unabhängig vom Dado-Kaufstatus.

Der vorhandene Netzwerk-Installationsweg über PC/Tizen Studio bleibt verfügbar. Er wird
nicht als direkte USB-Installation bezeichnet. Zugangsdaten und private Zertifikate gehören nicht in das Archiv.
