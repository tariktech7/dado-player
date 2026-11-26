# LG webOS und Samsung Tizen: technische Vorbereitung

Historischer Vorbereitungsstand vom 14. November 2026. Inzwischen ist Samsung unter
[client/README.md](client/README.md) implementiert und baubar; dort stehen die aktuellen
Dado-Build-/Installationsbefehle. Die folgenden Referenzbefehle bleiben als Grundlage erhalten.
LG ist noch nicht implementiert. Die bestehenden Rust-/egui-Apps bleiben erhalten.

## Referenz und Quellen

Lokaler Checkout: references/iptvnator, Repository
[4gray/iptvnator](https://github.com/4gray/iptvnator), Commit
`c809c8c735b6dcaecceb6efd37ecac56b203f1e8`. Lizenz: LICENSE.md (MIT, 4gray).
Übernommener Quellcode muss die Lizenzhinweise behalten.

| Datei im Referenzcheckout | Verwendung für Dado |
| --- | --- |
| docs/architecture/tv-application.md | Separater Preact-/TypeScript-Client, Plattformgrenzen |
| docs/development/tv-development.md | Entwicklung, LG-Paketierung, Tizen-Staging und Prüfungen |
| apps/tv/project.json | Nx-Targets für Build, Prüfung, IPK und WGT |
| apps/tv/vite.config.ts und verify-build.mjs | Relative Assets, klassisches IIFE-Bundle, ältere Engines |
| apps/tv/public/appinfo.json | LG-Metadaten und App-ID |
| apps/tv/tizen/config.xml und README.md | Samsung-Manifest, Zertifikat, Installation |
| libs/tv/util/src/platform.ts | Erkennung und Fernbedienungstasten |
| libs/tv/ui/src/platform.ts | Optionale Plattform-APIs und Lifecycle |

Offizielle Werkzeuge, geprüft am obigen Datum:
[LG CLI](https://webostv.developer.lge.com/develop/tools/cli-dev-guide),
[LG Web-App](https://webostv.developer.lge.com/develop/getting-started/build-your-first-web-app),
[Samsung CLI](https://developer.samsung.com/smarttv/develop/getting-started/using-sdk/command-line-interface.html),
[Samsung TV-SDK](https://developer.samsung.com/smarttv/develop/getting-started/setting-up-sdk/installing-tv-sdk.html),
[Samsung Zertifikate](https://developer.samsung.com/smarttv/develop/getting-started/setting-up-sdk/creating-certificates.html).

## Architektur

Ein gemeinsamer schlanker Web-TV-Client, vorzugsweise Preact/TypeScript wie in der
Referenz. Kein Electron, keine Windows-DLLs oder Android-JNI-Abhängigkeiten.
Rust/WASM für reine Fachlogik kann später untersucht werden; vorhandene native
Artefakte sind dadurch noch keine TV-Web-Pakete.

Rendering und Navigation folgen UI-SPEC.md und shared/tv-ui.json. Gemeinsame Verträge:

- `CatalogSource`: Kategorien, Sender, EPG, Filme, Serien; profil- und anbietergebundene
  IDs, damit Favoriten und Verlauf nicht zwischen Quellen kollidieren.
- `PlaybackAdapter`: open, play, pause, seek, stop, tracks, selectTrack, setAspect;
  Ereignisse ready, time, tracksChanged, ended, error. Fähigkeiten für Seek, Live,
  Audio-/Untertitelwahl und PiP explizit melden und UI danach steuern.
- `StorageAdapter`: versionierte Profile, Einstellungen, Favoriten und Fortschritt;
  Migrationen, Fehlerbehandlung und profilgebundene Schlüssel. Zugangsdaten schützen;
  niemals URLs mit Passwörtern in Logs.
- `RemoteAdapter`: Pfeile, OK, Back, Media-Tasten und Wiederholungen normalisieren.
- `LifecycleAdapter`: Suspend/Resume, Sichtbarkeit, Screensaver und Exit.

Die Referenz verwendet HTML-Video. Samsung AVPlay ist eine mögliche zusätzliche
Umsetzung für konkrete Codec-/Containeranforderungen, nicht bereits in Dado vorhanden.
Decoderfähigkeiten variieren je Gerätegeneration: HLS, TS, MP4 und Audio-/Untertitelwahl
mit echten zulässigen Teststreams prüfen.

Web-Providerzugriff benötigt CORS und kompatibles TLS. Direkter Zugriff und optionaler
Companion-Backend-Modus müssen explizite Einstellungen sein, kein stiller Wechsel.
Bei Companion-Betrieb tatsächliche Paket-Origin erlauben und eine vom TV erreichbare
LAN-Adresse verwenden: localhost zeigt auf den TV. Externe Logo-/EPG-Requests mitprüfen.

## Referenz-Builds, noch keine Dado-Buildbefehle

Diese Befehle gehören **nur zum unveränderten IPTVnator-Checkout**. Voraussetzungen
laut gepinntem Stand: Node 24.22.1, pnpm 10.33.0 (Integritäts-Pin in package.json)
und passende SDK-CLIs. Hier wird nichts automatisch installiert.

```powershell
Set-Location references/iptvnator
corepack pnpm install --frozen-lockfile
corepack pnpm nx run tv:typecheck
corepack pnpm nx test tv
corepack pnpm nx run tv:verify-build
```

verify-build baut vorher den Client und prüft LG-Ausgabe dist/apps/tv sowie
Samsung-Staging dist/tv-tizen-input. Referenz: klassisches IIFE, Chrome-68-Buildziel,
relative Assets, Prüfung neuer Built-ins. Samsung-Ziel der Referenz ist Tizen 5.5+;
das ist noch keine Kompatibilitätszusage für Dado.

### LG webOS

LG-CLI installieren, Developer-Mode-App auf eigenem TV aktivieren und Gerät koppeln.
Im Referenzcheckout paketieren und prüfen:

```powershell
corepack pnpm nx run tv:package
corepack pnpm exec ares-package -i dist/tv-package/com.iptvnator.tv.prototype_0.1.25_all.ipk
corepack pnpm exec ares-setup-device --list
# Nur auf einem bereits gekoppelten eigenen Testgerät:
corepack pnpm exec ares-install --device <device-name> dist/tv-package/com.iptvnator.tv.prototype_0.1.25_all.ipk
corepack pnpm exec ares-launch --device <device-name> com.iptvnator.tv.prototype
```

Version und ID stammen aus appinfo.json des genannten Commits. Dado braucht eigene
stabile App-ID, Anbieter, Icons und Version. Spätere Ausgabe:
Releases/Dado-Player-webOS-<version>.ipk. Build-Einstieg erst zusammen mit echtem Client
anlegen. Magic-Remote-Pointer zusätzlich zum D-Pad prüfen.

### Samsung Tizen

Tizen Studio mit TV Extension und Samsung Certificate Extension. Eigenes Samsung-
Zertifikatsprofil mit der DUID des Testgeräts erstellen; Schlüssel und Profile außerhalb
von Git sichern. Danach im Referenzcheckout:

```powershell
$env:TIZEN_SIGNING_PROFILE = '<own-local-profile>'
corepack pnpm nx run tv:package-tizen
sdb connect <tv-ip>:26101
sdb devices
tizen install -s <serial> -n <generated-file>.wgt -- dist/tv-tizen-package
tizen run -s <serial> -p IPTVnatorT.TV
```

Package-Target ruft externe Tizen-CLI auf und schreibt nach dist/tv-tizen-package.
Run-ID ist die Application-ID, nicht die Widget-URL. Dado benötigt eigene IDs,
Manifest, Zertifikat und Version; spätere Ausgabe Releases/Dado-Player-Tizen-<version>.wgt.
Samsung-Skript $WEBAPIS/webapis/webapis.js nur auf Samsung laden, APIs auf Verfügbarkeit
prüfen. Back-Key: LG 461, Samsung 10009; Play/Pause 10252. Zusätzliche Media-Tasten
registrieren. Root-Exit und Wiederaufnahme auf Hardware abnehmen.

## Reihenfolge der späteren Umsetzung

1. Web-TV-Client und gemeinsame Daten-/Adapterverträge anlegen; bestehende Apps erhalten.
2. UI-Raster, Fokus, Suche und proportionale Logos gegen Android-Referenz implementieren.
3. Profile, Favoriten, Fortschritt und EPG einschließlich Migrationen portieren.
4. Direkten/Companion-Netzwerkmodus mit echten CORS-Origins prüfen.
5. Plattform-Player und Lifecycle anbinden; Ressourcen bei Wechsel freigeben.
6. Eigene Manifeste und je einen reproduzierbaren Build-Einstieg anlegen.
7. Engine-/Bundle-, Contract- und UI-Prüfungen in CI aufnehmen.
8. Auf echtem LG und Samsung Installation, UI, Fernbedienung, Wiedergabe, Tracks,
   Suspend und Update testen; erst danach Releases veröffentlichen.

Offen: tatsächliche Dado-Web-App, Adapter, CORS-Abnahme, Zertifikate und Hardwaretests.
Es wurde weder ein TV gekoppelt noch ein fremdes Paket installiert.
