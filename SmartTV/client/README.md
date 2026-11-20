# Dado Player für Samsung Tizen

Eigenständige TV-Web-App, gemeinsamer Beta-Stand **0.7.5**. Standardziel: Tizen **6.5+**
(Chromium 85). Für 2019-Geräte mit Tizen 5.0/Chromium 63 gibt es ein separates Buildprofil.
Der Standard-Test-TV GQ77S94CATXZG meldet Tizen 9.0; ein GQ55Q60RGTXZG mit Tizen 5.0
wurde am 18. November 2026 mit dem älteren Profil erfolgreich installiert und gestartet.

## Tizen 5.0 / Modelljahr 2019

In PowerShell im Clientordner:

```powershell
$env:DADO_TIZEN_PROFILE='2019'
npm run check
npm run test:ui
$env:DADO_PACKAGE_DIR='../../Releases/Tests/Samsung-Tizen5'
npm run package -- --profile DEIN_SAMSUNG_TV_PROFIL
```

`TIZEN_CLI` muss wie beim Standardbuild auf die SDK-CLI zeigen. Das Samsung-Distributor-
Zertifikat muss diesen TV enthalten; bestehende Autorenzertifikate und andere Geräteprofile
behalten. Paket anschließend mit `tizen install -s TV_IP:26101 -n Dado-Player-Tizen-0.7.9.wgt -- PAKETORDNER`
installieren und `tizen run -s TV_IP:26101 -p DadoPlayTV.Player` starten.

Das Profil setzt die deklarierte Mindestversion auf 5.0 und übersetzt JavaScript für Chrome 63.
Es ergänzt fehlende replaceAll/flatMap/UUID-/ResizeObserver-APIs und abbrechbare Netzwerkanfragen
über XMLHttpRequest. Fokus ohne automatische Scrollsprünge bleibt erhalten. Browsertests
entfernen die betreffenden modernen APIs gezielt; tatsächliche TV-Wiedergabe separat prüfen.

Danach die Variablen entfernen und den Standard-Build wiederherstellen:

```powershell
Remove-Item Env:DADO_TIZEN_PROFILE,Env:DADO_PACKAGE_DIR -ErrorAction SilentlyContinue
npm run build
```

Das Eltern-TV-Paket bleibt bewusst getrennt vom veröffentlichten Standard-WGT; der normale
Release wird durch diesen Testbuild nicht ersetzt. Noch keine Zusage für Tizen vor 5.0.
Ältere Geräte und LG webOS sind noch keine freigegebenen Ziele.

USB-Verteilung und aktueller Seller-Office-Stand: [USB-STATUS.md](../USB-STATUS.md).
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
`Releases/Dado-Player-Tizen-0.6.0-unsigned.wgt`. **Unsigniert bedeutet: noch nicht auf einem
echten Samsung-TV installierbar.** Das Paket ist kein mit einem Emulatorzertifikat
fälschlich als gerätegeeignet ausgewiesener Build.

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

`deploy.ps1` verbindet den konfigurierten TV, baut erneut, signiert, installiert und startet
`DadoPlayTV.Player`. App-ID und Package-ID (`DadoPlayTV`) bleiben für Updates stabil.
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
