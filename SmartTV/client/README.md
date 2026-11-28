# Dado Player für Samsung Tizen

Beide Samsung-Pakete verwenden dieselbe TV-Web-App, dieselben Ressourcen und
dasselbe für Chromium 63 übersetzte JavaScript. Die Paketprofile deklarieren Tizen
**5.0+** (Modelljahr 2019) bzw. **6.5+**. Die vorhandenen App- und Update-Identitäten
bleiben pro Paketprofil erhalten.

## Beide Samsung-Pakete bauen

Vom Repository-Stamm in PowerShell:

```powershell
./SmartTV/client/build.ps1
./SmartTV/client/build.ps1 -SigningProfile DadoTV -LegacySigningProfile DadoTV2019
```

Der Einstieg baut beide Widgets derselben Produktversion. Die öffentlichen Downloads
tragen den Zusatz `-unsigned.wgt`. Mit einem Signierungsprofil entstehen zusätzlich
`Dado-Player-Tizen-<VERSION>.wgt` und `Dado-Player-Tizen5-<VERSION>.wgt` für die lokale
Installation. Das Samsung-Distributorzertifikat enthält die Geräte-ID des jeweiligen TVs.
Mit `DADO_PACKAGE_DIR` kann ein separater Ausgabeordner gewählt werden.

Für lokale Browser-Tests beider Profile im Clientordner:

```powershell
npm run test:ui
$env:DADO_TIZEN_PROFILE='2019'
npm run test:ui
Remove-Item Env:DADO_TIZEN_PROFILE
```

Die Browser-Tests simulieren Samsung-APIs und fehlende Chromium-Funktionen.
Installation, Wiedergabe und Fernbedienung auf dem Gerät separat prüfen.

USB-Verteilung: [USB-STATUS.md](../USB-STATUS.md).
Das lokale Skript `npm run usb` erstellt ohne Samsung-TMG/Lizenzdateien nur ein klar
als nicht installierbar gekennzeichnetes Vorbereitungsarchiv.

## Implementierter Umfang

- Gemeinsames D7-Logo und Source Sans 3; skalierbares 1920×1080-Referenzraster mit 5 % Safe Area.
- Navigation, Kategorien, virtualisierte Senderliste und Vorschau; zehn sichtbare Senderzeilen,
  Jetzt/Danach nebeneinander, dezente abwechselnde Spaltenflächen und proportionale Logos.
- Xtream Codes und HTTP(S)-M3U; XMLTV-EPG optional für M3U. Filme, Serien mit Staffeln/Folgen,
  Suche, Favoriten und Wiedergabefortschritt. Anbieter-Teilfehler löschen nicht die übrige Bibliothek.
- Inline-Eingaben über native HTML-Felder mit TV-Tastatur. Gespeichertes Profil wird beim
  nächsten Start automatisch verbunden; explizites Abmelden verhindert dies.
- Auswahlfelder öffnen erst mit OK ein Dado-Menü. Pfeiltasten auf einem geschlossenen
  Auswahlfeld bewegen weiter den Fokus. Die TV-Tastatur ändert nur die sichtbare
  Formularhöhe; Schriftgröße und TV-Raster bleiben stabil und das aktive Feld scrollt ins Bild.
- AVPlay mit eigener Dado-Steuerung, Live-Vorschau und Vollbild ohne erneutes Öffnen des Streams,
  Pause, VOD-Seek, Ton-/Untertitelspuren, Untertitelgröße und Bildformat.
- Alternative HTML-Video-Engine; kein MPV/VLC/MX-Player auf Tizen.
- Einstellbare Sprache, Puffer, Timeout, Live-HLS/TS, Player-User-Agent, automatische Wiedergabe,
  Theme und Senderdichte. Tizen-/TV-eigene Tonausgabe statt wirkungsloser Windows-Schalter.
- Suspend/Restore über Samsung-APIs; Screensaver wird bei Pause, Ende und Fehler wieder aktiviert.
- Mitgeliefertes H.264-/AAC-Testvideo für Wiedergabeprüfung ohne Anbieterzugang;
  es wird aus ios/Resources/Playback.mp4 übernommen und im Widget mitgeliefert.
- GitHub-Updateprüfung mit Hinweis zur erneuten Signierung/Installation über den PC.
  Ein Widget kann sich nicht selbst durch eine beliebige unsignierte WGT ersetzen.

Die Version 0.1.6 ergänzt dieselbe Kategorien-/Titel-/Detailstruktur für Filme und Serien,
einen immer sichtbaren Live-Vorschaubereich und getrennte Speicher für Auswahl/Scrollposition
jedes Bereichs. Staffelbuttons begrenzen die Folgenliste auf eine Staffel. Der Serien-Play-Button
setzt fort und wechselt ab 60 Sekunden Rest zur nächsten Folge (auch über Staffelgrenzen).
Favoriten und Fortsetzen besitzen Live-TV-/Serien-/Filme-Rubriken. Die Startseite hat einen
Fortsetzen-Hero mit Artwork und eine separate rechte Favoriten-EPG-Spalte. Alle horizontalen
Reihen scrollen bei Fernbedienungsfokus mit; Dokument und TV-Safe-Area bleiben stehen.
Im Live-Vollbild: Hoch/Runter zum Zappen, 0 zum vorherigen Sender, zwei Sekunden Auto-Hide.

Profile, Zugangsdaten, Favoriten und Verlauf liegen im privaten lokalen App-Speicher (localStorage),
nicht in einer Dado-Cloud. Zugangsdaten sind dort nicht zusätzlich verschlüsselt.
Löschen der App kann diese Daten entfernen. Keine echten Providerdaten in Tests oder Repository ablegen.

## Reproduzierbarer Build

Node 24 und npm, vom Repository-Stamm:

```powershell
./SmartTV/client/build.ps1
```

Der Einstieg führt `npm ci`, TypeScript-Prüfung, Unit-Tests und esbuild aus und erzeugt
beide Pakete. **Unsignierte Pakete sind auf Samsung-TVs nicht installierbar.**

Für Entwicklung im Unterordner:

```powershell
npm ci
npm run check
npx playwright install chromium
npm run test:ui
npm run serve
```

Vorschau: http://127.0.0.1:18497. Die Browser-Tests simulieren Samsungs API, ersetzen
keinen Decoder-/Wiedergabetest auf Hardware. Tests enthalten ausschließlich Demodaten.
Der Build lädt keine Anwendungsskripte von einem CDN. Samsung webapis.js wird nur auf Tizen geladen.

## Web-Updates über GitHub

Unter **Einstellungen → App-Updates** sucht der Fernseher auf Knopfdruck im öffentlichen
GitHub-Update-Kanal. Die App prüft die Signatur eines verfügbaren Updates und startet bei
Leerlauf neu. Seit 0.8.3 erfolgt die Abfrage ohne GitHub-Token. Ältere Versionen benötigen
für den ersten Wechsel weiterhin ihren bisher konfigurierten Token oder das signierte WGT.

**Vorherige Version wiederherstellen** lädt das letzte gesicherte kompatible Web-Update.
Fehlt eine solche Sicherung oder schlägt ein neuer Start fehl, dient das installierte WGT
als Rückfallebene. Profile, Favoriten und Lizenz bleiben im App-Speicher. Wenn sich Ressourcen,
Berechtigungen oder das Startprogramm ändern, ist ein neues WGT erforderlich.

Für einen neuen Web-Stand zuerst `tizenBuild` in `release.json` erhöhen und
`python scripts/release_version.py` im Repository-Stamm ausführen. Danach im TV-Client bauen und
signieren. Der getrennte P-256-Updateschlüssel liegt außerhalb des Repositories unter
`~/.dado-player/tizen-update-private.pem`. `DADO_TV_KEY_FILE` kann auf den geschützten
Speicherort zeigen. Der öffentliche Schlüssel in `tizen/update-key.json` gehört zum WGT.

```powershell
npm run build
npm run sign:update -- ../../Releases/Tizen-Updates
$env:DADO_TIZEN_PROFILE = '2019'
npm run build
npm run sign:update -- ../../Releases/Tizen-Updates
Remove-Item Env:DADO_TIZEN_PROFILE
npm run publish:update -- ../../Releases/Tizen-Updates
```

`publish:update` prüft beide Signaturen, Profil und Buildnummer und veröffentlicht beide Varianten
gemeinsam auf dem Branch `tizen-updates`. Bei geändertem Kompatibilitätswert zeigt die App
den Hinweis auf das neue WGT an. Der Ablauf orientiert sich am Web-Updater von
[APL-TV](https://github.com/Knikt/APL-TV/blob/main/app/js/updater.js).

## Signierung und Installation

1. Apps auf dem TV öffnen, 12345 eingeben, Developer Mode aktivieren, PC-IP eintragen,
   TV neu starten. PC und TV müssen im selben Netz erreichbar sein.
2. Tizen SDK mit Samsung Certificate Extension **mindestens 2.0.73** bereitstellen.
   Eigenes Samsung-TV-Zertifikatsprofil anlegen und die DUID dieses TVs hinzufügen.
   Samsung-Kontoanmeldung und persönliche Zertifikatsdaten gehören dem Besitzer.
3. SDK-Verzeichnis über `TIZEN_SDK_HOME`, `-SdkRoot` oder eine ignorierte tizen.local.json angeben.

Beispiel für die lokale Datei (keine Passwörter/Zertifikate eintragen):

```json
{"sdkRoot":"C:/tizen-studio","tvIp":"192.168.178.69"}
```

```powershell
./SmartTV/client/build.ps1 -SigningProfile DadoTV
./SmartTV/client/deploy.ps1 -SigningProfile DadoTV
```

`deploy.ps1` verbindet den konfigurierten TV, baut erneut, signiert und installiert das Paket.
Der optionale Schalter `-Run` startet anschließend `DadoPlayTV.Player`.
App-ID und Package-ID (`DadoPlayTV`) bleiben für Updates stabil.
Den Autorenschlüssel sichern; Updates müssen denselben Autor behalten. Schlüssel, Passwörter,
Samsung-Tokens, DUIDs und lokale SDK-Konfigurationen niemals committen.

Die SDK-Standardprofile `default`/`develop` sind für Emulatoren gedacht und werden vom
Deployment-Einstieg abgelehnt. Beim Import fremder Profile vorher prüfen, dass es tatsächlich
Samsung-TV-Zertifikate mit der richtigen DUID sind.

## Optionaler API-/EPG-Companion

Nur erforderlich, wenn ein Anbieter Browser-API-Requests wegen CORS blockiert.
Streams spielt AVPlay weiterhin direkt ab. Der Companion ist kein Medienproxy und kein
TLS-Ausnahmemechanismus. Zertifikatsprüfung bleibt aktiv.

```powershell
node SmartTV/client/scripts/companion.mjs --init C:/private/dado-companion.json
```

Die erzeugte private Datei enthält einen zufälligen Schlüssel. `providerOrigins` auf die exakten
Anbieter-Origins setzen, `clientOrigins` auf die tatsächliche TV-Origin. Die Browser-Vorschau
verwendet http://127.0.0.1:18497; paketierte Apps können eine null-/file-Origin senden.
Nur die beobachtete Origin erlauben. `host` auf die eigene LAN-Adresse setzen, wenn der TV
zugreifen soll; Standard ist bewusst nur 127.0.0.1. Gegebenenfalls den Windows-Netzwerkdialog
selbst für das private Netz beantworten. Es wird keine Firewallregel automatisch verändert.

```powershell
node SmartTV/client/scripts/companion.mjs C:/private/dado-companion.json
```

Danach in Dado → Einstellungen → Netzwerk den Companion-Modus, dessen LAN-Adresse und
den Schlüssel eintragen und Bibliothek neu laden. Der Dienst prüft Token und Origin,
erlaubt nur konfigurierte Provider-Origins (auch bei Redirects), begrenzt Antworten auf 30 MB
und protokolliert keine Provider-URLs oder Zugangsdaten. Der PC muss währenddessen laufen.
Ein reiner HTTP-Companion gehört ausschließlich in ein vertrauenswürdiges Heimnetz.

## Abnahme auf dem Fernseher

- Anmeldung mit eigenem Anbieter, gespeicherter Wiedereinstieg und Abmelden.
- 4K-/Full-HD-Darstellung, Fernbedienung, Scrollen durch große Bibliotheken, Suche und TV-Tastatur.
- Live-HLS und TS, VOD/Seek, Serienfolge, Audio-/Untertitelwechsel, Aspect Ratio.
- Vorschau → Vollbild → Zurück, Kanalwechsel, Suspend/Resume und Screensaver nach Fehler/Pause.
- Update mit gleichem Autorenzertifikat und Erhalt lokaler Daten.

Offizielle Grundlagen:
[Samsung AVPlay](https://developer.samsung.com/smarttv/develop/api-references/samsung-product-api-references/avplay-api.html),
[Web-Engines](https://developer.samsung.com/smarttv/develop/specifications/web-engine-specifications.html),
[Zertifikate](https://developer.samsung.com/smarttv/develop/getting-started/setting-up-sdk/creating-certificates.html),
[TV-Geräteverbindung](https://developer.samsung.com/smarttv/develop/getting-started/using-sdk/tv-device.html).

### 0.6.1 playback and navigation

Default playback uses AVPlay and Dado controls, with one HTML-video attempt on
native playback failure using the unchanged original URL and resume position.
The expert switch reveals forced engines/live format/buffer/network overrides.
Settings use a section list and one active options panel. Select a section with OK,
then press Right to enter its controls. Left returns to the remembered section,
then to navigation. Back returns to navigation, then asks before exiting (open inputs,
dialogs and playback stages close first).

`navigation-motion.ts` applies one grid layout change and animates only visual
transform offsets. It avoids grid-width transitions and whole-app renders on
focus, supports mid-animation reversal and honors prefers-reduced-motion. Browser
checks validate stable layout widths while offsets change; device smoothness is
a separate hardware acceptance check.
