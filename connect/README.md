# Dado Connect · 0.7.2

Portal: https://dado-connect.dado-licensing.workers.dev

Verwaltung: https://dado-admin.dado-licensing.workers.dev/admin/connect

## Ablauf

1. Sender registriert ein Konto (mindestens fünf Zeichen, eine Ziffer und ein Sonderzeichen im Passwort). Tarik prüft die Person und genehmigt das Konto. Keine automatische E-Mail-Verifikation oder Mailzustellung. Kontofreigabe ist unabhängig von einer Kauflizenz.
2. Dado Player einmal online öffnen. Die App registriert einmalig ihre Adresse und einen eigenen zufälligen Gerätetoken. Kein automatischer Profilabruf und kein regelmäßiger Heartbeat. Bei fehlgeschlagener Registrierung versucht ein manueller Abruf es erneut.
3. Links unter „1 · Gerät auswählen“ ein gespeichertes Gerät wählen. Name und blassere Adresse stehen nebeneinander. Unter „Neues Gerät verbinden“ nur MAC-/Geräteadresse und Gerätenamen eingeben. Kein Kopplungscode. Noch nicht registrierte Geräte müssen zuerst Dado Player online öffnen. Bereits anders zugeordnete Geräte werden nicht übernommen.
4. Rechts M3U-URL oder Xtream-Server, Benutzername und Anbieterpasswort eingeben. Webseitenlogin und Anbieterzugang sind getrennt. Die Anbieterfelder haben eigene Namen/Autofill-Sektionen und Passwortmanager-Hinweise; das Auge zeigt nur das Anbieterpasswort. Browsererweiterungen können Autofill-Hinweise übergehen.
5. „Profil im Konto speichern“ legt ausdrücklich eine wiederverwendbare verschlüsselte Vorlage an. Später auswählen, bearbeiten, erneut speichern, an ein anderes gespeichertes Gerät senden oder löschen. Bis zu 50 Vorlagen und 50 Geräte pro Konto. Einfaches Senden allein speichert keine dauerhafte Vorlage.
6. Am Gerät „Profile aktualisieren“ drücken. Die App zeigt keinen Portal-Link, Kontonamen, Kopplungscode oder technischen Connect-Infoblock. Windows/Samsung behalten ihre bisherigen Geräteangaben; Android/iOS zeigen die private Geräteadresse und Geräte-ID. Keine automatische Anbieteranmeldung.

## Sicherheit und Speicherung

MAC-Adressen sind keine Geheimnisse. Die bewusst gewählte MAC-Zuordnung setzt Vertrauen in die manuell genehmigten Senderkonten voraus. Nur genehmigte Konten dürfen Geräte zuordnen und Profile senden. Mehrere freigegebene Senderkonten können dieselbe Geräteadresse unabhängig hinterlegen und Profile senden. Jedes Konto sieht seinen eigenen Gerätenamen und Versandverlauf. Trennt ein Sender sein Gerät im Portal, werden nur seine Verknüpfung und seine ausstehenden Sendungen entfernt. Der Abruf von Profilen erfordert weiterhin den zufälligen Gerätetoken, niemals nur die MAC. Portaländerungen und vertrauliches Laden gespeicherter Vorlagen prüfen zusätzlich Origin.

Separater öffentlicher Worker dado-connect mit eigener EU-D1-Datenbank, getrennt von Lizenzdaten. Verwaltung in dado-admin unter bestehendem Cloudflare-Access-Schutz. Passwörter mit PBKDF2-SHA256, individuellem Salt und geheimem Pepper; gehashte Sitzungen und Secure/HttpOnly/SameSite-Strict-Cookie. Raten-, Körpergrößen- und Warteschlangenlimits bleiben aktiv.

Vorlagen sind mit AES-GCM und zusätzlicher Bindung an Profil-ID/Konto verschlüsselt. Nur das angemeldete Besitzerkonto kann sie laden, ändern, senden oder löschen. Die Listenantwort enthält nur Metadaten. Vorlagen bleiben bis zur Löschung oder Kontolöschung erhalten. Es gibt keine Klartext-Ablage der Anbieterzugänge im Browser-LocalStorage. Der Dienst kann sie beim autorisierten Laden/Senden entschlüsseln: keine Ende-zu-Ende-Verschlüsselung.

Die Versandwarteschlange ist davon getrennt und ebenfalls verschlüsselt. Nach dauerhaftem Speichern bestätigt das Gerät den Empfang; erst dann wird der Versandinhalt entfernt. Nicht abgeholte Sendungen verfallen nach sieben Tagen und werden stündlich bereinigt. Metadaten 30 Tage, Auditdaten 90 Tage. Das Bestätigen einer Sendung löscht keine gespeicherte Vorlage.

Ab 0.7.2 verwenden gespeicherte Vorlagen stabile Geräteprofil-IDs (cloud-profile-UUID). Erneutes Senden aktualisiert ausschließlich den eigenen importierten Vorlageneintrag, ohne weitere Duplikate; manuelle Profile bleiben erhalten. Native letzte Senderposition bleibt dabei erhalten. Alte Einmalsendungen bleiben nach Nachrichten-ID dedupliziert. Ältere Apps ignorieren die zusätzliche sourceId; für Vorlagen-Aktualisierungen auf 0.7.2 aktualisieren.

Die Gerätezuordnung wird im Portal getrennt; keine Kontoverwaltung im Endnutzer-UI. Bereits lokal gespeicherte Profile werden dadurch nicht gelöscht. iOS/Android geben keine verlässliche Hardware-MAC heraus; dort gilt die dauerhaft private Geräteadresse im MAC-Format. LG ist vorbereitet, noch kein gebauter Client.

## Betrieb

`node --test connect/tests/service.test.mjs`

`python connect/scripts/deploy.py` prüft bestehende Remote-Schlüssel, migriert D1 und deployt Dienst und Verwaltung. Bestehende AUTH_PEPPER/CRYPTO_KEY niemals ungeplant ersetzen. Die ignorierte lokale Sicherung connect/.local/connect-secrets.dpapi ist Windows-CurrentUser-DPAPI-geschützt. Schlüsselverlust kann Passwörter und verschlüsselte Vorlagen unbrauchbar machen. Free-Kontingente gelten; keine kostenpflichtige Erweiterung aktiviert.

`node connect/scripts/smoke.mjs` prüft Produktion mit temporären Daten einschließlich Vorlagenverschlüsselung, erneutem Versand und Löschung und entfernt das Testkonto anschließend. Keine echten Konten werden automatisch genehmigt.

Protokoll: /api/profiles (Metadaten), /api/profiles/save, load, delete; /api/messages nimmt alternativ profileId an. Geräte registrieren über /v1/device/register und rufen ausdrücklich mit pull/ack ab. Das alte pairing-Endpunktformat bleibt für ältere Apps kompatibel, sein Code ist keine Voraussetzung der MAC-Zuordnung mehr. Zugangsdaten nicht protokollieren.
