# Dado Connect · 0.7.0

Portal: https://dado-connect.dado-licensing.workers.dev

Verwaltung: https://dado-admin.dado-licensing.workers.dev/admin/connect

## Benutzung

1. Mit Name, E-Mail und Passwort (mindestens fünf Zeichen, eine Zahl und ein Sonderzeichen) registrieren. Das Konto wartet auf Tariks manuelle Genehmigung. Es gibt keinen automatischen Mailversand; Tarik prüft die Person selbst.
2. Tarik genehmigt das Konto in der durch Cloudflare Access geschützten Verwaltung. Das ist unabhängig von einer gekauften App-Lizenz.
3. Am Empfangsgerät in Einstellungen → Dado Connect einen Kopplungscode erzeugen. Ohne Anbieterprofil ist Connect auch auf der Anmeldeseite erreichbar.
4. Im Portal Geräteadresse (oder unterstützte Hardware-MAC), den zehn Minuten gültigen Code und einen Gerätenamen eingeben. Die Adresse allein berechtigt nicht zum Senden. Ein Gerät gehört jeweils einem Senderkonto.
5. M3U-URL oder Xtream-Codes-Server, Benutzername und Passwort senden. M3U-Dateiuploads sind nicht enthalten.
6. Am Gerät ausdrücklich Aktualisieren drücken. Die Profile werden lokal gespeichert und können anschließend in der Profilwahl ausgewählt werden. Bestehende Profile bleiben erhalten; keine automatische Anmeldung oder Wiedergabe.

Windows, Android einschließlich TV-Modus, iOS und Samsung unterstützen denselben Ablauf. LG ist über das Protokoll vorbereitet, noch kein ausgelieferter Client. Auf iOS/Android erscheint eine dauerhafte private Geräteadresse im MAC-Format, keine Hardware-MAC.

## Betrieb und Grenzen

Separater Worker dado-connect mit separater EU-D1-Datenbank. Der Lizenzbestand bleibt unverändert. Der bestehende dado-admin bindet zusätzlich CONNECT_DB ein. Admin-Routen bleiben hinter derselben verifizierten Cloudflare-Access-Identität.

Konten: pending / approved / suspended. Passwörter: PBKDF2-SHA256 mit individuellem Salt und geheimem Pepper. Sitzungen: zufällige Tokens, serverseitig gehasht, Secure/HttpOnly/SameSite-Strict-Cookie. Portaländerungen prüfen Origin. Geräte verwenden eigene zufällige Bearer-Tokens; MAC ist kein Passwort. Raten-, Körpergrößen- und Warteschlangenlimits gelten serverseitig.

Zugangsdaten werden in der Warteschlange mit AES-GCM verschlüsselt. Der Dienst entschlüsselt beim Abruf; keine Ende-zu-Ende-Verschlüsselung. Sender können gespeicherte Zugangsdaten nicht wieder auslesen. Die App speichert dauerhaft vor der Empfangsbestätigung. Stabile Nachrichten-IDs verhindern Duplikate bei Wiederholungsabruf. Die Bestätigung löscht den verschlüsselten Inhalt. Unabgeholte Sendungen verfallen nach sieben Tagen, stündliche Bereinigung entfernt den Inhalt. Metadaten bleiben 30 Tage, Auditdaten 90 Tage.

Keine automatischen Profilabrufe beim Start oder während Wiedergabe. Verbindung trennen entfernt offene Sendungen und Zuordnung, nicht bereits lokal gespeicherte Profile. Verlorene Geräte-Tokens sind nicht aus der Adresse rekonstruierbar.

Passwort vergessen: Tarik erzeugt einen einmaligen Reset-Link (eine Stunde gültig) und übermittelt ihn selbst. Kein E-Mail-Dienst. Reset genehmigt kein wartendes Konto.

Cloudflare-Free-Kontingente gelten; kostenlos bedeutet keine unbegrenzte Kapazität. Keine kostenpflichtige Erweiterung wurde für Connect aktiviert. Datenschutzinformationen stehen unter /privacy und müssen mit den tatsächlichen Betreiberabläufen aktuell gehalten werden.

## Entwicklung und Deployment

`node --test connect/tests/service.test.mjs`

`python connect/scripts/deploy.py` verwendet den bestehenden Wrangler-Login, prüft Remote-Schlüssel, migriert D1 und deployt Dienst und Verwaltung. Vorhandene AUTH_PEPPER und CRYPTO_KEY werden bei normalen Deployments niemals ersetzt. Fehlender/teilweiser Schlüsselbestand führt zum Abbruch; initiale Schlüssel nur ausdrücklich mit --initialize-empty erzeugen. Die ignorierte lokale Sicherung connect/.local/connect-secrets.dpapi ist mit Windows-CurrentUser-DPAPI verschlüsselt. Nicht entfernen oder Schlüssel ungeplant rotieren: Verlust kann Passwörter und offene Sendungen unbrauchbar machen.

`node connect/scripts/smoke.mjs` prüft Produktion mit temporären Testdaten und entfernt sie anschließend. Es genehmigt keine echten Konten.

Portal: /api/auth/register, /api/auth/login, /api/session, /api/pair, /api/devices, /api/messages, /api/unpair. Geräte: /v1/device/register, pairing, pull, ack. Clients: src/connect.rs und SmartTV/client/src/connect.tsx. Kompatibilität, manuelle Abrufe und Speichern vor Bestätigung beibehalten. Keine Zugangsdaten protokollieren.
