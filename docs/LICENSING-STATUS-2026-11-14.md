# Lizenzierung: Zwischenstand 14.11.2026

Abschluss in der Nacht zum 15.11.2026: Quellstand `aa75212b5e23b9658ef4e129422f68b37b3694f1` ist nach `origin/main` gepusht. [Lizenz-Beta auf GitHub Releases](https://github.com/tariktech7/dado-player/releases/tag/dado-license-beta-2026-11-15) als getrenntes Prerelease veröffentlicht. Alle drei hochgeladenen Paketgrößen und SHA-256-Digests stimmen mit den lokalen Paketen überein. Kein automatisches Update des bisherigen stabilen Releases. Lokaler Release-Index aktualisiert, ältere Windows-/Samsung-Pakete ohne Löschen ins Archiv verschoben. Der Benutzer hat das vollständige Herunterfahren nach Arbeitsabschluss ausdrücklich beauftragt.

## Geprüft

- Backend: 5 Tests bestanden; SQL-Adapter verwendet echte SQLite-Datenbank und eigene kurzlebige Testschlüssel.
- Rust: 50 Tests bestanden; Clippy und Formatprüfung bestanden (bestehende Vendor-Warnungen separat). Exakte Ablaufgrenze und dauerhafte Berechtigung werden ausdrücklich geprüft.
- TV: 14 Unit-Tests und 20 Browser-UI-Tests bestanden. Produktionsbundle nach UI-Testlauf wiederhergestellt; Testpakete werden vom Paketprüfer zurückgewiesen. Zusätzliche Tests prüfen den gesperrten Katalog, sichtbare MAC/Geräte-ID, Offline-Fehler beim Teststart und erfolgreiche Wiederfreigabe mit anschließend netzwerkfreiem Neustart. Ablaufbildschirm als Screenshot visuell geprüft.
- Öffentlicher Cloudflare-Dienst: signiertes Trial für synthetisches Samsung-Testgerät erhalten und gegen öffentlichen Produktionsschlüssel geprüft. Keine automatische Kundengenehmigung.
- Private Verwaltung: Cloudflare-Login im Benutzerbrowser erfolgreich, Suche nach dem synthetischen Gerät liefert genau einen Trial-Datensatz. Keine Freischaltung in diesem Browsertest vorgenommen.
- Access-Fehler behoben: Benutzer ergänzte die fehlende Subdomain `dado-admin` in der bestehenden Access-Anwendung. Die vorherige Rückleitung auf `dado-licensing.workers.dev` scheiterte mit NXDOMAIN.
- Beide Worker nach letzten Backend-Änderungen bereitgestellt.
- Portable-Paket: Laufzeitrichtlinie geprüft, zwei Packaging-Tests bestanden.
- Synthetisches Deployment-Testgerät anschließend über lokalen Eigentümer-CLI ausdrücklich genehmigt (`INTERNAL-DEPLOYMENT-TEST`). Der öffentliche Produktionsdienst stellte darauf eine gültig signierte Dauerlizenz ohne Ablauf aus; Signatur, Plattform und Gerätebindung geprüft. Kein Kundengerät aktiviert.
- Android: tatsächlicher NDK-/Gradle-Build und Signatur-/Alignment-Prüfung erfolgreich. Die bestehende veröffentlichte APK wurde lokal aus der Sicherung wiederhergestellt; keine mobile Kaufsperre und kein neues mobiles Release.
- iOS: tatsächliche arm64-Geräte-Rustbibliothek mit Apple-SDK gebaut, ABI geprüft (3 definierte und 3 erwartete importierte Bridge-Symbole). Swift-Host unverändert; keine neue IPA veröffentlicht.

## Neue Lizenz-Beta-Pakete

| Paket | SHA-256 |
| --- | --- |
| Windows 0.5.6 portable | `01fa4a61b40dc830841067587a62df32495ff5de645b184454b16692c1cca114` |
| Samsung 0.1.7 signiert | `a29b98cc18fceef167d5b9979633087e41c025d89ae996430934f84e4b833231` |
| Samsung 0.1.7 unsigniert | `3898283ad3db702c41a013c1609122852666963fb32256bd80f9e064ec7be3ee` |

Pakete liegen unter Releases. Windows ist ohne öffentliche Authenticode-Identität; dadurch keine Zusage zu SmartScreen. Samsung-Paket enthält Signaturen; Zertifikatskette und DUID müssen bei tatsächlicher Geräteinstallation bestätigt werden.

## Noch offen vor Freigabe dieser Beta

Neues Windows-Paket und Samsung-Installation mit echter Gerätekennung, MAC und vollständigem Trial-/Freigabeablauf praktisch abnehmen. Keine Hardwareabnahme durch die Browser-Simulation behauptet. Datenbank-/Schlüsselsicherung mit Wiederherstellungsprobe vor produktivem Verkauf abschließen. CSV-Verwaltung ist ein zusätzlicher offener Planpunkt. Frühere stabile Releases bleiben erhalten. Die Lizenz-Beta wird getrennt als GitHub-Prerelease verteilt und ersetzt nicht das bisherige stabile mobile Release.
