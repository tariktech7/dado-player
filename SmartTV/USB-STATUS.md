# Samsung USB-Verteilung

Samsung muss das USB-Demopaket für das Zielgerät ausstellen. Der lokale Build erzeugt ein WGT zur Entwicklerinstallation. Die Anforderungen stehen in der [Samsung-Dokumentation](https://developer.samsung.com/smarttv/develop/faq/application-installation.html).

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
