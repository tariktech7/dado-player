# Manuelle Dado-Freischaltungen

Stand: 14.11.2026. Windows und Samsung haben eine siebentägige Testversion. Android/iOS erhalten in diesem Schritt keine Kaufsperre. LG und Android TV sind noch keine implementierten Lizenz-Clients.

## Bedienung

1. Kunde öffnet Einstellungen und startet die Testversion einmal online. Der Server speichert den Beginn; eine Neuinstallation desselben erkannten Geräts beginnt keine neue Frist.
2. Kunde nennt die angezeigte Geräte-ID oder eine registrierte MAC-Adresse. Rohkennungen bleiben am Gerät; der Dienst speichert Fingerabdrücke und MAC-Hashes. Windows kann mehrere Netzwerkadressen anzeigen.
3. Zahlung selbst prüfen. Preis und Zahlungsabwicklung sind außerhalb dieses Systems.
4. [Private Verwaltung öffnen](https://dado-admin.dado-licensing.workers.dev/admin), mit dem für „Nur Tarik“ zugelassenen Konto anmelden, Gerätekennung suchen, Ziel prüfen und dauerhaft freischalten. Eine kurze Referenz ist optional.
5. Kunde wählt „Freischaltung abrufen“. Erst nach erfolgreicher Signaturprüfung und Speicherung zeigt die App „Unlimited“. Danach keine automatischen Lizenz-Serveraufrufe. Bei gelöschter Lizenz ist ein ausdrücklicher Wiederherstellungsabruf nötig.

## Verhalten der Apps

- Ohne gespeicherte Lizenz erscheint vor Anmeldung und Katalog ein eigener Startbildschirm. Windows zeigt getrennte Knöpfe für „7 Tage kostenlos testen“ und den Abruf einer vorhandenen Freigabe; Samsung kombiniert beide im ersten Abrufknopf. Kein unbeabsichtigter Teststart allein durch Öffnen der App. Ab Windows 0.5.7: kompakte, bei kleinen Fenstern scrollbar bleibende Karte mit D7-Logo. Angezeigt und als MAC-Suchalias registriert wird derselbe bevorzugte Netzwerkadapter; virtuelle Wi-Fi-Direct-/VPN-Duplikate werden ausgefiltert. Die Gerätebindung bleibt unverändert.
- Nach genau 7 × 24 Stunden erscheint „Ihre Testversion ist abgelaufen“. Katalog und Wiedergabe bleiben gesperrt, auch im Windows-TV-Modus. Laufende Wiedergabe wird beendet. MAC-Adresse (sofern verfügbar), Geräte-ID, Fehlermeldungen und „Freischaltung abrufen“ bleiben erreichbar. Das ist unabhängig vom IPTV-Anbieterlogin. Es werden keine Profile oder Favoriten gelöscht.
- Hintergrundabrufe: ausschließlich während einer gespeicherten Testversion beim App-Start, wenn der letzte erfolgreiche Abruf mindestens sechs Stunden zurückliegt. Windows versucht zusätzlich einmal bei erkanntem Ablauf, eine eventuell schon genehmigte Freigabe abzuholen. Samsung sperrt beim Ablauf lokal und bietet den manuellen Abruf. Kein dauerhaftes Polling und keine Abfrage pro Kanalwechsel. Für Hintergrundabrufe erscheint kein Bestätigungsdialog.
- Netzfehler entziehen keine noch gültige Test- oder Dauerlizenz. Eine abgelaufene Testversion bleibt auch bei Netzfehler gesperrt. Eine fehlende/beschädigte lokale Lizenz kann über denselben Knopf wiederhergestellt werden; ein bereits abgelaufener Server-Trial beginnt dadurch nicht erneut.
- Eine gespeicherte Dauerlizenz wird beim Start lokal kryptografisch und auf Gerätebindung geprüft. Sie hat kein Ablaufdatum. Weder Neustart noch Standby lösen automatische Serverabfragen aus. Der ausdrücklich gewählte Wiederherstellungsabruf benötigt weiterhin Internet.

Der öffentliche Dienst ist https://dado-license.dado-licensing.workers.dev. Er stellt keine Verwaltungsoberfläche bereit. `dado-licensing.workers.dev` allein ist keine Website.

## Cloudflare Access

Die Anwendung schützt den Worker `dado-admin`. Ihr öffentlicher Hostname lautet **dado-admin.dado-licensing.workers.dev**, Pfad `admin*`. In der Dashboard-Eingabe: Subdomain `dado-admin`, Domain `dado-licensing.workers.dev`. Ein leerer Subdomain-Eintrag leitete nach erfolgreichem Login auf die nicht existierende Basisadresse zurück (NXDOMAIN); am 14.11. korrigiert und angemeldete Suche im Browser geprüft.

`Nur Tarik` bleibt eine Allow-Regel für die Eigentümer-E-Mail. Kein Everyone, kein Bypass. Der Worker prüft zusätzlich die authentisierte Identität. `dado-license` bleibt für Geräte ohne Admin-Login erreichbar. Den öffentlichen API-Worker nicht zur privaten Access-Anwendung hinzufügen.

## Lokaler Verwaltungsweg

`Freischalten.cmd` in diesem Ordner öffnen. Das Werkzeug verwendet die vorhandene Wrangler-Anmeldung und fragt Gerätekennung, ausdrückliche Bestätigung und Referenz ab. Node.js und die Abhängigkeiten dieses Ordners sind nötig (`npm ci`). Bei abgelaufener Anmeldung selbst `npx wrangler login` ausführen.

Für technische Nutzung: `node admin-cli.mjs lookup <Geräte-ID>`; zum Genehmigen `node admin-cli.mjs approve <Geräte-ID> --confirm <dieselbe-Geräte-ID> --reference <Referenz>`.

## Betrieb und Grenzen

- Öffentliches Vertrauen: `public-config.json` enthält nur URL und öffentlichen ES256-Schlüssel. Private Schlüssel niemals ins Repository, in Pakete oder Kundenbrowser schreiben.
- `SIGNING_JWK` liegt ausschließlich als Cloudflare-Secret im API-Worker. Die lokale Sicherung `.local/signing-key.dpapi` ist an den Windows-Benutzer und dessen Entschlüsselungsmöglichkeit gebunden. Das ist noch keine portable Notfallsicherung für einen anderen PC. Vor produktivem Verkauf zusätzlich sichere, getestete Schlüsselsicherung und Datenbankexport einrichten.
- Dauerlizenzen sind absichtlich offline und unbefristet. Bereits ausgegebene Dokumente lassen sich ohne erneute Prüfung nicht aus der Ferne sperren. Geräte-/Plattform-Reset kann Wiederherstellung oder manuelle Zuordnung erfordern.
- Der Trial verwendet Serverbeginn, signierte Zeit, gespeicherten Höchststand und monotone Laufzeit. Ein manipulationssicherer Offline-Client wird nicht versprochen.
- Free-Kontingente sind begrenzt. Kein kostenpflichtiger Tarif wird durch die Deploy-Skripte ausgewählt. Nutzung im Cloudflare-Dashboard überwachen.
- Web-Verwaltung bietet derzeit Suche und Einzelgenehmigung mit Audit-Protokoll; CSV-Import, Exportoberfläche und vollständige Protokollansicht sind noch offen.

## Prüfen und bereitstellen

`npm test` prüft mit SQLite und separaten Testschlüsseln Trial, Gerätebindung, manuelle Genehmigung, Rollen, Zeitgrenzen, Origin-Prüfung und Limits. `npx wrangler deploy --config wrangler.jsonc` und danach `npx wrangler deploy --config wrangler.admin.jsonc` stellen beide Dienste bereit. Vor einer Neuinstallation Datenbank und bestehende Schlüssel erhalten; keine neue Signieridentität über eine vorhandene schreiben.

Samsung: `npm run check`, `npm run test:ui`, Paket signieren und `verify:package` mit dem tatsächlichen Paketpfad aufrufen. UI-Tests erzeugen eigene Testschlüssel; Testbundles dürfen nicht verteilt werden. Paketprüfung ersetzt keinen Installationstest auf dem TV.
