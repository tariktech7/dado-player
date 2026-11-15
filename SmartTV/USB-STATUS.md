# Samsung USB-Verteilung: Stand 15.11.2026

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
