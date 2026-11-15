# Samsung USB-Verteilung: Stand 15.11.2026

Samsung beschreibt in der offiziellen [Installations-FAQ](https://developer.samsung.com/smarttv/develop/faq/application-installation.html)
eine Umwandlung über das USB Demo Packaging Tool im TV Seller Office. Eine normale WGT lässt sich
auf Consumer-TVs nicht direkt vom USB-Stick installieren. Die erzeugten Samsung-Dateien wären
eine TMG und eine zugehörige Lizenzdatei. Das lokale Entwicklerzertifikat ersetzt diese nicht.

Im angemeldeten Seller Office war am 15.11.2026 noch keine Seller-Gruppe eingerichtet und kein
Tools-/USB-Packaging-Menü sichtbar. Das Formular Membership → Group Information → Create Group
ist erreichbar. Es verlangt Kontotyp, Land/Region sowie Verkäufer- und Kontaktinformationen
einschließlich Homepage, Telefonnummer und Adresse. Die Seite weist auf die Anzeige von
Verkäuferinformationen auf dem TV hin. Diese Angaben müssen vom Betreiber kommen.

Eine Gruppenerstellung allein ist noch keine bestätigte Berechtigung für USB-Packaging.
Die Oberfläche unterscheidet Public Groups und Partner Groups; für letztere nennt sie einen
Samsung-Vertrag und die Zustimmung eines Content Managers. Der tatsächliche Zugang zum
USB-Werkzeug und die Kompatibilität mit dem Ziel-TV müssen danach erneut geprüft werden.

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
