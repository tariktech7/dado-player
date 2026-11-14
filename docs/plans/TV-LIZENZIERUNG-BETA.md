# TV-Lizenzierung für die Beta — Plan und Umsetzungsstand

Stand: 14.11.2026. Der nachfolgende Entwurf wurde freigegeben. Aktueller Betrieb und offene Punkte stehen in [licensing/README.md](../../licensing/README.md). Implementiert sind API, private Verwaltung, Windows- und Samsung-Client. Der Browser-Login und eine Suche nach einem synthetischen Testgerät wurden nach Korrektur des Access-Hostnamens erfolgreich geprüft. Windows 0.5.6 und Samsung 0.1.7 wurden lokal gebaut; Hardware-/Endabnahme und Veröffentlichung dieser Beta stehen noch aus. Bestehende veröffentlichte Pakete wurden nicht ersetzt.

Abweichungen vom ursprünglichen Vorschlag: Windows ist auf ausdrücklichen Wunsch eingeschlossen. Der Gerätecode verwendet sechs Gruppen aus jeweils vier Hex-Zeichen nach `D7-`, ohne zusätzliche Prüfziffer. CSV-Import/Export und eine portable Schlüsselsicherung sind noch offen. Die folgenden Abschnitte bewahren den ursprünglichen Plan und sind keine pauschale Fertigmeldung.

Nachtrag 15.11.2026: Eigene Start-/Ablaufsperrbildschirme für Windows und Samsung ergänzt und geprüft. Lizenz-Beta als getrenntes [GitHub-Prerelease](https://github.com/tariktech7/dado-player/releases/tag/dado-license-beta-2026-11-15) veröffentlicht; Quellstand `aa75212`. Hardwareabnahme bleibt offen. Details und Hashes im [Prüfbericht](../LICENSING-STATUS-2026-11-14.md).

## Entscheidungsvorschlag

Sieben Tage gerätegebundene Testzeit ab dem ersten erfolgreich bestätigten Online-Start. Danach ist die Wiedergabe gesperrt, die Aktivierungs-/Supportoberfläche und vorhandene Daten bleiben erreichbar. Nach manuell bestätigter Zahlung wird eine unbefristete, signierte Lizenz abgerufen und lokal gespeichert. Ab dann keine Serverabfragen zur Lizenz, keine Lease, kein Ablaufdatum und kein Hintergrund-Polling. Eine lokale Signatur- und Geräteprüfung beim Start schützt vor einem einfachen veränderten Freischalt-Flag und kopierten Lizenzdateien; ihr Ergebnis bleibt im Speicher und liegt außerhalb des Render-/Wiedergabepfads.

Empfohlene Infrastruktur: Cloudflare Workers Free + D1 Free, geschützter Admin-Worker mit Cloudflare Access und eine öffentlich erreichbare Lizenz-API. Kostenlos bereitgestellte workers.dev-Adressen reichen; eigene Domain ist optional. Kein Zahlungsanbieter-API-Zwang, keine automatische Zahlungsfreischaltung, kein laufender VPS.

Geltungsbereich: zuerst Samsung Tizen, Verträge und Prüfmodule für spätere LG-/Android-TV-Ausgaben vorbereiten. Mobilversionen erhalten jetzt keine Kauf-/Testsperre; später eigene IAP-Anbindung über eine gemeinsame Berechtigungs-Schnittstelle. Windows bleibt in diesem Vorhaben unverändert. Android-TV-Lizenzierung muss an ein TV-Releaseprofil gebunden werden, nicht an die veränderbare UI-Einstellung TV-Modus. Ein Umschalten der Darstellung darf keine Lizenzsperre umgehen.

## Gerätekennung und MAC-Ablauf

Eine echte Netzwerk-MAC ist keine plattformübergreifend verfügbare Grundlage. Samsung bietet DUID und eine MAC-Abfrage; LG stellt LGUDID bereit; Android verbietet gewöhnlichen Drittanbieter-Apps den Zugriff auf die Hardware-MAC. WLAN/LAN-Wechsel dürfen auch auf Samsung keine zweite Zahlung auslösen.

Deshalb besitzt Dado eine lesbare Geräte-ID, zum Beispiel D7-7K2M-9P4X-8R6T-3W5N plus Prüfziffern. Der interne vollständige Fingerabdruck wird aus Produktnamespace, Plattform und unterstützter Gerätekennung abgeleitet. Der kurze Code ist eine Such-/Supportkennung, kein Geheimnis. Kollisionen und Schreibfehler werden beim Registrieren behandelt; die Lizenz bindet an den vollständigen Fingerabdruck.

| Plattform | Kandidat | Noch zu prüfen |
| --- | --- | --- |
| Samsung Tizen | ProductInfo getDuid | Verfügbarkeit, Berechtigungen, Stabilität nach Update/Neuinstallation und WLAN/LAN-Wechsel am echten TV |
| LG webOS | Device Unique ID / LGUDID | Erst bei LG-Portierung testen; LG fordert Zustimmung vor Verwendung der Geräte-ID |
| Android TV | ANDROID_ID im TV-Releaseprofil | Bindung an Signierschlüssel/Nutzer/Gerät, Verhalten bei Reset; kein Ersatz durch eine Hardware-MAC vortäuschen |

Auf Samsung kann die angezeigte echte MAC zusätzlich als Admin-Suchalias registriert werden, sofern die API eine brauchbare Adresse liefert. Dann funktioniert auch „Kunde nennt MAC, Tarik schaltet frei“. Die Lizenz selbst bleibt an die stabile Dado-Gerätekennung gebunden. Wenn MAC fehlt, mehrere Geräte zugeordnet sind oder das Gerät noch nie registriert war, zeigt die Verwaltung einen eindeutigen Fehler und fordert die Dado-ID. Eine erfundene Kennung wird nicht als echte MAC beschriftet.

Rohkennungen möglichst auf dem Gerät lassen; abgeleitete Gerätefingerabdrücke und optional normalisierte MAC-Hashes verwenden. Auch Hashes sind hier pseudonyme Daten, nicht automatisch anonym. Keine öffentlichen MAC-/Kundenlisten, keine Anbieterzugangsdaten, Playlists oder Sehgewohnheiten im Lizenzdienst.

## Kundenablauf

1. App installieren und erstmals mit Internet starten. Kurzer Hinweis auf Testzeit/Gerätekennung; erforderliche Plattform-Zustimmung einholen. Ein idempotenter Serveraufruf legt den Testbeginn fest und gibt ein signiertes Trial-Dokument mit Serverzeit zurück.
2. Sieben mal 24 Stunden nutzen. Der TV speichert das Dokument und kann die verbleibende Zeit lokal prüfen. Ohne bestehendes Trial-Dokument ist zum Teststart eine Verbindung notwendig; kein immer wieder neuer Offline-Fallback-Trial.
3. Während der Testphase höchstens ein asynchroner Statusabruf beim App-Start mit Abkühlzeit, beispielsweise sechs Stunden. Manuell „Freischaltung abrufen“ umgeht die normale Abkühlzeit innerhalb sinnvoller Missbrauchsgrenzen. Keine Abfrage pro Sender, Film, Tastendruck oder Videoframe.
4. Bei Ablauf erfolgt eine letzte Abrufmöglichkeit für eventuell bereits genehmigte Freischaltungen. Ohne Lizenz stoppt die Wiedergabe und die Aktivierungsseite erscheint. Einstellungen, Daten und Hilfe bleiben zugänglich. Ein einzelner Ablauf-Timer und lokale Prüfung nach Standby/Vordergrundwechsel verhindern, dass eine wochenlang geöffnete App das Ende ignoriert.
5. Kunde zahlt außerhalb dieses technischen Systems und nennt Tarik die Dado-ID oder auf Samsung die registrierte MAC. Die Bezahlart und der Preis bleiben offen.
6. Tarik genehmigt die Freischaltung manuell. Der Kunde drückt „Freischaltung abrufen“. Die App lädt die unbefristete Lizenz, prüft sie und speichert sie dauerhaft. Ein Eintrag im Backend kann einen TV ohne Abruf/Verbindung nicht von selbst erreichen.
7. Nach erfolgreichem Abruf entfallen sämtliche automatischen Lizenz-Serverkontakte. Updates behalten Lizenz, Signaturvertrauen und App-Daten bei. Bei gelöscht gespeicherter Lizenz ist ein explizites Wiederherstellen aus dem Backend möglich; das ist ein neuer Abruf, keine regelmäßige Prüfung.

Trial-Beginn und Ende entstehen ausschließlich auf dem Server. Ein unveränderter Gerätefingerabdruck erhält bei Neuinstallation denselben Beginn. Ein signierter Zeitstempel, gespeicherter höchster Zeitstand und monotone Zeit innerhalb eines laufenden Prozesses erschweren Zurückstellen der Uhr. Ohne regelmäßige vertrauenswürdige Online-Zeit und auf veränderbaren Clients ist ein vollständig manipulationssicherer Offline-Trial nicht erreichbar. Factory Reset oder Plattform-ID-Wechsel können eine Wiederherstellung bzw. manuelle Zuordnung erfordern; keine universelle Reset-Stabilität versprechen.

## Bedienung für Tarik

Eine private kleine Verwaltungsseite, die am PC und Telefon funktioniert:

- Geräte-ID/MAC eingeben und vorhandenen Datensatz suchen.
- Plattform, Testbeginn/-ende, Status und bisherige Freischaltungen sehen.
- Optional eine kurze Zahlungs-/Bestellreferenz eintragen; keine Zahlungsdaten verarbeiten.
- Zielkennung kontrollieren und „Dauerhaft freischalten“ bestätigen.
- Liste/Export, Suchfunktion und Änderungsprotokoll; CSV-Import für mehrere Geräte mit Vorschau und Duplikatprüfung.

Die Freischaltung ist ein ausdrücklicher Admin-Vorgang. Keine Payment-Webhooks oder Kontoeinsicht. Ein Preisfeld im Frontend oder ein vom Client gesendetes paid=true kann niemals eine Lizenz ausstellen. Ein CSV ist ein Importformat für die geschützte Verwaltung, keine Datei im öffentlichen Repository.

Für die Beta reicht zusätzlich ein dokumentierter lokaler Admin-Befehl als Wiederherstellungs-/Supportweg. Auch ich könnte auf deine konkrete Anweisung eine Kennung eintragen, solange der autorisierte Zugriff auf das Konto besteht; deine Verwaltung darf nicht von diesem Chat abhängig sein.

## Technische Aufteilung

Vorgesehene neue Bereiche, keine Implementierung in diesem Planungsschritt:

- licensing/service: Worker-API, D1-Migrationen, Schema, Limits und Lizenzsignierung.
- licensing/admin: private Verwaltungsseite, Admin-Authentisierung, CSV-Import/Export.
- SmartTV/client/src/licensing: Geräteadapter, Signaturprüfung, Lizenzspeicher, Zustandsmodell und Freischaltseite für Samsung; spätere LG-Adapter ohne Samsung-Sonderlogik im UI.
- src/licensing: native Berechtigungs-Schnittstelle für ein späteres Android-TV-Profil und spätere IAP-Provider. Mobil-IAP wird jetzt nicht eingebaut.
- docs/licensing: Einrichtung, Notfallwiederherstellung, Geräteaustausch und Betrieb.

Backend-Zustand pro Gerät: Produkt/Edition, Fingerabdruck, lesbare ID, optional MAC-Alias-Hash, erster Trial-Beginn, Trial-Ende, Freischaltstatus/-datum, Lizenz-ID, Signierschlüssel-ID und optionale Bestellreferenz. Eindeutige Datenbankschlüssel und transaktionale/idempotente Vorgänge verhindern parallele neue Trials und doppelte Freischaltungen.

Client-Zustände: nicht eingerichtet, Trial gültig, Trial abgelaufen, dauerhaft aktiviert, Wiederherstellung erforderlich. Netzfehler sind ein eigener Abrufstatus und dürfen eine vorhandene gültige Lizenz nicht entziehen. Kein Bild-/Ton-/EPG-Verkehr über diesen Dienst.

Lizenzformat: versioniertes signiertes Dokument, vorzugsweise JWS ES256/P-256. Felder unter anderem product, edition, subject, license_id, issued_at, kind und key_id; Trial mit expiry, Dauerlizenz ohne zeitlichen Ablauf. Fester zugelassener Algorithmus; keine vom Dokument beliebig ausgewählte unsichere Prüfung. Signaturformat zwischen WebCrypto und Rust vereinheitlichen. P-256/WebCrypto-Verfügbarkeit zuerst am Samsung-TV beweisen; bei Bedarf überprüfte lokale Verifikation einsetzen. Keine Implementierung eigener Kryptografie.

Privater Signierschlüssel nur im geschützten Dienst und in einer verschlüsselten Sicherung. Apps enthalten ausschließlich die benötigten öffentlichen Prüfschlüssel. Schlüsselrotation bewahrt alte öffentliche Schlüssel für bereits ausgestellte Dauerlizenzen. Admin-Login über Cloudflare Access auf eigenem geschütztem Worker, nur autorisiertes Konto und MFA beim Identitätsanbieter; Admin-API selbst authentisieren. Öffentliche Geräte-API erhält Eingabegrenzen, Rate-Limits und keine Listen-/Suchfunktion über alle Geräte.

Test- und Produktionsschlüssel, Datenbanken und Releaseprofile bleiben getrennt. Kein schaltbarer produktiver Entwickler-Bypass in einer Release-App. Die Testphase beginnt für bestehende Beta-Installationen bei Einführung des Systems, nicht rückwirkend anhand früherer Installationsdaten.

## Hosting und Kosten

Stand der offiziellen Tarife am 14.11.2026:

| Option | Einschätzung |
| --- | --- |
| Cloudflare Workers Free + D1 Free | Empfehlung. Workers 100.000 Requests/Tag, 10 ms CPU je Aufruf; D1 5 Mio. gelesene und 100.000 geschriebene Zeilen/Tag sowie insgesamt 5 GB. Für einen kleinen effizienten Aktivierungsdienst voraussichtlich ausreichend; Signierung/Abfragen unter realen Free-Limits messen. |
| Cloudflare Access Free | Private Admin-Oberfläche für ein kleines Team; Free-Angebot für weniger als 50 Nutzer. Kunden brauchen keine Access-Konten. |
| AWS Lambda + API Gateway + DynamoDB | Technisch geeignet, aber mehr Einrichtung/Billing-Komponenten. Das neue kostenlose AWS-Kontomodell ist bis zu sechs Monate/Verbrauch der Credits begrenzt; es existieren daneben dienstspezifische Always-Free-Kontingente. Nicht pauschal als für immer kostenlose Gesamtlösung versprechen. |
| Rein offline signierte Aktivierungsdatei/-code | Kein Hosting für die Dauerlizenz nötig, aber unpraktischer TV-Transfer und kein serverseitiger Schutz des ursprünglichen Trial-Beginns. Deshalb nicht die empfohlene Beta-Lösung. |

Zielkosten 0 Euro/Monat im Free-Kontingent, ohne kostenpflichtige Domain, automatische Plan-Upgrades oder bezahlte Zusatzdienste. Kein Versprechen unveränderlicher zukünftiger Anbieterpreise. Zahlungs-/Bankgebühren und Store-Kosten sind davon getrennt. Bei Free-Limits oder Ausfall können neue Teststarts/Freischaltungen vorübergehend scheitern; gültige gespeicherte Trials und Dauerlizenzen bleiben nutzbar. Bezahlte Nutzer erzeugen nach Aktivierung keine regelmäßigen Lizenzanfragen.

## Bewusste Grenzen

- Eine vollständig offline nutzbare Dauerlizenz lässt sich nach Rückzahlung, Chargeback oder Geräteweitergabe nicht zuverlässig aus der Ferne sperren. Auch ein Transfer kann die bereits gespeicherte Lizenz des alten TVs nicht zurückholen. Das ist eine direkte Folge des gewünschten Modells, kein mit einem Admin-Schalter lösbares Detail.
- App-Datenlöschung kann die lokale Lizenz entfernen; Wiederherstellung braucht dann wieder Dienst und erkennbare Geräteidentität. Kein erneuter Kauf bei bloßem Update.
- Modifizierte Apps/gerootete Geräte lassen sich ohne zusätzliche Plattformmechanismen nicht absolut gegen Umgehung schützen. Eine MAC-Adresse allein ist kein Kopierschutz.
- Alte bereits verteilte Builds ohne Lizenzprüfung werden durch eine neue Beta nicht nachträglich kostenpflichtig. Vorhandene funktionierende Pakete bleiben im Archiv erhalten.
- Externe manuelle Bezahlung muss zum Distributionskanal passen. Für Samsung Store vor öffentlicher Veröffentlichung Geschäfts-/Billing-Einstufung im Seller Office klären. Bei Google Play gelten auch auf Android TV die Payments-Regeln einschließlich etwaiger regionaler Programme/Ausnahmen; direkte APK-Beta und Play-Veröffentlichung getrennt planen. Mobile-IAP bleibt ein späteres eigenes Vorhaben.

## Reihenfolge nach Zustimmung und Abnahme

1. Auf dem echten Samsung-TV Kennung, Netzwerkwechsel, verfügbare Kryptografie und persistenten Speicher prüfen. Distributionsweg der Beta festlegen; Preis bleibt konfigurierbar/offen.
2. Protokoll/Schema und Threat Model fixieren; getrennte Entwicklungs-/Produktionsschlüssel und Sicherung einrichten.
3. Cloudflare-Free-Konto anbinden, API und D1 mit reproduzierbaren Migrationen aufbauen. Abfragen und Signierung unter Free-Limits messen.
4. Private Verwaltung mit Suche, manueller Freigabe, Vorschau und Protokoll bauen. Erst isolierte Testgeräte freischalten.
5. Samsung-Testzugang, Sperrseite, Abrufen/Wiederherstellen und dauerhaftes Offline-Recht integrieren. Profile und Wiedergabe unverändert erhalten.
6. Ablauf und Fehler automatisiert mit Testzeit prüfen: einmaliger Trial, genaues Ende auch bei offener App/Standby, Uhr-Rückstellung, parallele Registrierung, ungültige/kopierte Lizenzen, API-Ausfall, Neustart, Update, Datenverlust, falsche MAC, nicht autorisierter Admin und Rate-Limits.
7. Am TV eine manuelle Aktivierung nachstellen und anschließend den Lizenzdienst blockieren: App bleibt nach Neustart dauerhaft freigeschaltet; Netzwerkprotokoll enthält keine Lizenzanfragen. Nicht freigeschaltete Geräte bleiben nach Trial-Ende gesperrt.
8. Datenschutz-/Aktivierungstexte und Betriebshandbuch vervollständigen, Sicherungs-/Wiederherstellungsweg testen, Beta signieren und eindeutig versionieren. Mobile-Builds per Regressionstest ohne neue Kauf-/Testsperre halten. LG-/Android-TV-Anbindung erst für die jeweils tatsächlich gebauten und geprüften Editionen als fertig ausgeben.

Ergebnis nach Umsetzung: ein installierbares Samsung-Beta-Paket, dauerhaft erreichbare Freischaltverwaltung, reproduzierbares Backend, Schlüssel-/Datenbackup und eine kurze Bedienanleitung für manuelle Freischaltungen.

## Primärquellen

- Samsung ProductInfo / DUID: https://developer.samsung.com/smarttv/develop/api-references/samsung-product-api-references/productinfo-api.html
- Samsung Network / MAC: https://developer.samsung.com/smarttv/develop/api-references/samsung-product-api-references/network-api.html
- LG Gerätekennung: https://webostv.developer.lge.com/develop/references/device-unique-id
- Android Kennungen / MAC-Beschränkung: https://developer.android.com/identity/user-data-ids
- Android ANDROID_ID: https://developer.android.com/reference/android/provider/Settings.Secure
- Workers Preise: https://developers.cloudflare.com/workers/platform/pricing/
- Workers Limits: https://developers.cloudflare.com/workers/platform/limits/
- D1 Preise: https://developers.cloudflare.com/d1/platform/pricing/
- Access Preise: https://www.cloudflare.com/plans/zero-trust-services/
- Access für workers.dev: https://developers.cloudflare.com/workers/configuration/cloudflare-access/
- AWS Free Tier: https://docs.aws.amazon.com/us_en/awsaccountbilling/latest/aboutv2/free-tier.html
- Google Play Payments: https://support.google.com/googleplay/android-developer/answer/10281818?hl=en
- Samsung Geschäftskonditionen / Content Manager: https://developer.samsung.com/tv-seller-office/faq/tv-seller-office-use.html
