# Dado-Player — Bauplan Mobile (Android / Android TV)

> **Was das ist:** Übergabedokument vom fertigen Windows-Client an die Mobile-Entwicklung.
> Es beschreibt *was* die App tut, *wie* sie es tut, *was* übernommen werden darf und *was nicht*.
> **Stand:** 3. November 2026, Desktop-Runde 4 abgeschlossen (`dist/Dado-Player.exe`, 6,2 MB, lauffähig).
> **Quellcode Desktop:** `C:\Users\Tarik\WorkBuddy AI\IPTV App`
> **Sprache der App:** Deutsch (UI-Texte). Code/Kommentare Englisch.

---

## 0. So benutzt du dieses Dokument

1. Lies **§1–§4**, um das Produkt und den Ist-Zustand zu verstehen. Nicht raten, nicht neu erfinden.
2. Entscheide mit dem Nutzer **§5** (Stack). Das ist die einzige wirklich offene Grundsatzfrage.
3. Arbeite **§11** (Meilensteine) von oben nach unten ab. Jeder Meilenstein hat ein „fertig wenn".
4. **§10** ist Pflichtlektüre vor der ersten Zeile Android-Code — dort stehen die Fallen, in die wir auf Windows schon getreten sind.
5. **§6 + §7** sind Verträge. Wenn du davon abweichst, bricht die Kompatibilität mit bestehenden Nutzerdaten und Provider-Panels.

---

## 1. Produkt

Ein schlanker, werbefreier IPTV-Player. Der Nutzer bringt seinen eigenen Provider mit (Xtream-Codes-API oder M3U-Playlist). Die App liefert **keine** Inhalte, **keine** Provider, **keine** Accounts, **keine** Telemetrie.

Drei Inhaltsarten, ein Player:

| Bereich | Inhalt | Besonderheit |
|---|---|---|
| **Live-TV** | Kanäle mit Logo, Kategorie, EPG | Zappen per ↑/↓, Vorschau inline, „● LIVE"-Badge statt Fortschrittsbalken |
| **Filme** | VOD-Poster-Grid | Fortschritt wird gespeichert, „Weiter bei mm:ss" |
| **Serien** | Serien-Grid → Staffeln/Folgen | Folgen als flache, sortierte Liste |

Querschnitt: **Suche**, **Favoriten**, **Fortsetzen** (Verlauf mit drei Tabs), **Einstellungen** (Einfach/Experte), **Hell/Dunkel/System**.

**Leitplanken, die nicht verhandelbar sind:**
- Keine Werbung, keine Abos, keine „Premium"-Sperren, kein Tracking, kein Crash-Reporting nach außen.
- Kein RAM-Fresser: Das ist der Grund, warum der Desktop-Client in Rust gebaut wurde. Auf Mobile heißt das: Listen virtualisieren, Bilder begrenzt cachen, keine riesigen JSON-Objekte im Speicher halten.
- Bedienung muss auf 5-Zoll-Handy **und** auf 55-Zoll-TV mit Fernbedienung funktionieren.

---

## 2. Was auf Windows fertig ist (das ist die Referenz)

Feature-Parität mit diesem Stand ist das Ziel für v1.0 Mobile.

| Feature | Verhalten im Detail |
|---|---|
| Login | Karte mittig im Fenster, oben das D7-Logo. Zwei Modi: **Xtream** (Host/User/Passwort) und **M3U** (URL oder lokale Datei). Fehler werden inline angezeigt, Passwörter in Fehlermeldungen maskiert. |
| Profil-Speicherung | Mehrere Provider möglich, `default_profile` wird beim Start automatisch geladen. |
| Bibliothek | Ein Ladevorgang holt 6 Requests (3× Kategorien, 3× Inhalte) **sequenziell**, danach ist die App offline-navigierbar (Streams brauchen natürlich Netz). *Auf Mobile darfst du die Requests parallelisieren — auf Windows war Sequenz einfacher und für die Panel-Verträglichkeit unkritisch.* |
| Live-TV | Liste virtualisiert, Kategorie-Dropdown, Suchfeld, Favoriten-Stern pro Zeile, Kanalnummer, Logo. |
| **Vorschau** | Klick auf eine Zeile startet den Kanal **in einer Seitenleiste, die Liste bleibt sichtbar**. Zweiter Klick auf denselben Kanal (oder Doppelklick) → großer Player. Der ▶-Button in der Zeile springt direkt in den Player. |
| **Zappen** | ↑/↓ (im Hauptmenü) wechselt den Kanal **und lädt die Vorschau neu**, mit Wrap-Around (letzter → erster). ←/→ tut dasselbe. |
| Player | Vollflächig. ZURÜCK oben links, Titel + Untertitel oben rechts, Fortschritt + Puffer, Lautstärke, Transport **exakt zentriert**, Vollbild-Icon rechts. Auto-Hide nach 3 s. Details in §8.3. |
| Filme | Poster-Grid (2:3), Kategorie-Filter, Suche, Detailansicht mit Handlung, „Abspielen"/„Fortsetzen". |
| Serien | Gleiches Grid wie Filme (bewusst identisch), Klick → Staffel-Auswahl + Folgenliste mit Fortschritt. |
| Fortsetzen | Drei Tabs: **Live-TV / Filme / Serien**. Filme & Serien als Poster-Grid wie die Hauptansichten, mit „Weiter bei mm:ss". Live als Zeilenliste. |
| Favoriten | Gemischte Ansicht: Live-Zeilen + Film-Poster, gleiche Stern-Logik. |
| Suche | Globale Suche über Live + Filme + Serien, Treffer gruppiert. |
| Einstellungen | Umschalter **Einfach / Experten**. Einfach = Farbschema, 3 Presets, Puffer, Autoplay. Experten = alles (Hardware-Decoder, Video-Output, Sync, Interpolation, Debanding, Audio-Exklusiv/Passthrough/Normalisierung, Untertitelgröße, Netzwerk-Timeout, User-Agent). Speichert automatisch, kein „Speichern"-Button. |
| Theme | Standard **System** (folgt dem OS, reagiert live), zusätzlich Dunkel/Hell erzwingbar. Palette in §8.1. |
| Tastatur | Space = Pause, M = Mute, F = Vollbild, Esc = zurück, +/- = Lautstärke, Enter = Vorschau in den Player. |

**Bewusste Entscheidungen, die du übernehmen solltest:**
- Serien sehen aus wie Filme. Kein zweites visuelles System.
- Kein „Speichern"-Button in den Einstellungen — alles speichert sofort.
- Keine Beleuchtung, keine Glows, keine Gradients als Deko. Flach, hoher Kontrast, ein Akzentblau.
- Alle zentrierten Elemente werden **relativ zur verfügbaren Fläche** zentriert, nie mit Fixwerten. Der Nutzer verwendet unterschiedliche Gerätegrößen und hat das explizit eingefordert.

---

## 3. Architektur Desktop (Ist-Zustand)

```
src/
├── main.rs         Fenster, Startgröße aus Arbeitsbereich + DPI, Icon
├── app.rs          Zentraler State, View-Router, Tasten, Verlauf, Theme-Poll
├── types.rs        Datenmodell (Profile, Library, Settings, PlayItem …)
├── client.rs       Xtream-API + M3U-Parser  ← PORTABEL
├── store.rs        JSON-Persistenz + Migration ← PORTABEL (nur Pfade anders)
├── theme.rs        Palette + egui-Style
├── sys.rs          Windows: Registry-Theme, Cursor, Fensterrect ← NICHT portabel
├── video.rs        Natives Kind-Fenster für Video ← NICHT portabel
├── mpv.rs          libmpv per FFI (libloading) ← NICHT portabel
├── demo.rs         Demo-Daten für Screenshots (nur Dev)
└── ui/
    ├── mod.rs            Poster, Grid-Metriken, Segmented-Control, Brandmark, Helfer
    ├── sidebar.rs        Navigation links
    ├── login.rs          Login-Karte
    ├── live.rs           Kanalliste + Inline-Vorschau
    ├── vod.rs            Film-Grid + Detail
    ├── series.rs         Serien-Grid + Staffeln/Folgen
    ├── favorites.rs      Favoriten
    ├── search.rs         Suche
    ├── continue_watching.rs  Fortsetzen mit Tabs
    ├── settings.rs       Einfach/Experten
    └── player.rs         Vollflächiger Player (reine Geometrie-Berechnung)
```

**Datenfluss:** UI-Event → `App`-Methode → `tokio`-Task (Netz) → Ergebnis über `mpsc`-Channel zurück in den UI-Thread → State-Update → Repaint. Kein UI-Thread blockiert je auf Netzwerk. **Das musst du auf Mobile genauso machen** (Dart `Isolate`/`Future` bzw. Coroutines), sonst friert die Liste beim Kategorie-Wechsel ein.

**Größenordnung:** Kanallisten mit 10.000+ Einträgen sind normal. Der Desktop rendert nur den sichtbaren Ausschnitt (`ScrollArea::show_rows`). Auf Mobile: `ListView.builder` / `LazyColumn`, niemals `Column` mit allen Kindern.

---

## 4. Was portierbar ist — und was nicht

| Modul | Portierbar? | Anmerkung |
|---|---|---|
| `types.rs` | **Ja, 1:1 als Datenmodell** | Feldnamen sind der Vertrag (§6). |
| `client.rs` | **Ja, Logik 1:1** | Nur HTTP-Client und JSON-Decoder tauschen. Die Toleranzregeln in §7.2 sind Pflicht. |
| `store.rs` | **Ja, Logik 1:1** | Dateiformate identisch, Pfade plattformspezifisch (§6.1). |
| `theme.rs` | **Ja, als Werte** | Hex-Codes in §8.1 übernehmen, nicht das egui-API. |
| `ui/player.rs` | **Ja, als Spezifikation** | Die Geometrie ist reine Mathematik (§8.3) und funktioniert auf jeder Größe. |
| `ui/*` übrige | **Ja, als Layout-Vorgabe** | Nicht Code kopieren, sondern Verhalten nachbauen. |
| `mpv.rs` | **Nein** | Android nutzt `media_kit` (libmpv) oder ExoPlayer/Media3. |
| `video.rs` | **Nein** | Kind-HWND ist ein Windows-Konstrukt. Auf Mobile ist Video eine Texture/SurfaceView. |
| `sys.rs` | **Nein** | Theme: Android `Configuration.uiMode`. |

**Wichtig:** Der Desktop-Client hat eine **echte Einschränkung**, die auf Mobile wegfällt: weil das Video in einem nativen Kind-Fenster über der UI liegt, kann kein UI-Element darüber gezeichnet werden und echtes Blur über dem Video ist unmöglich. Auf Mobile liegt das Video in der Widget-Hierarchie — **dort kannst du die Glas-Optik wirklich umsetzen** (BackdropFilter / `blur`), statt sie nur zu simulieren. Nutze das, aber übertreibe es nicht: Der Nutzer hat „leicht durchsichtig, aber alles erkennbar" gesagt, nicht „milchig".

---

## 5. Stack-Entscheidung (mit dem Nutzer klären)

Zielplattformen: **Android-Handy + Tablet zuerst**, danach **Android TV / Fire TV**. iOS ist „vielleicht später". Samsung Tizen / LG webOS sind **nicht** realistisch mit einem gemeinsamen Codebase — falls der Nutzer „Smart TV" sagt, meint er praktisch Android TV.

| Option | Wiedergabe | Aufwand | TV | iOS | Bewertung |
|---|---|---|---|---|---|
| **Flutter + `media_kit`** | libmpv (gleiche Engine wie Desktop) | niedrig, eine Codebase | gut (Focus-System vorhanden) | ja | **Empfehlung.** Verhalten identisch zum Desktop, HLS/TS wie gewohnt, ein UI für alle Größen. APK wächst um ~20 MB pro ABI (libmpv). |
| Flutter + `video_player`/ExoPlayer | ExoPlayer | niedrig | gut | ja | Leichter, aber HLS/TS-Verhalten weicht ab, weniger Codec-Abdeckung. Fallback, wenn APK-Größe das Problem ist. |
| Kotlin + Compose + Media3 | ExoPlayer (Referenz-Implementierung) | mittel | **sehr gut** (Leanback) | nein | Beste Wiedergabe auf Android, aber iOS braucht ein zweites Projekt. Richtig, wenn „nur Android" feststeht. |
| React Native + `react-native-video` | ExoPlayer | mittel | ok | ja | Kein Vorteil gegenüber Flutter hier. |

**Empfehlung:** Flutter + `media_kit`. Begründung: Der Desktop-Client nutzt libmpv — mit `media_kit` bekommt Mobile **dieselbe Engine mit denselben Eigenheiten**, was bedeutet, dass Provider-Streams, die auf Windows laufen, auch auf dem Handy laufen. Das UI lässt sich 1:1 nachbauen (das Layout ist bewusst flach und ohne egui-spezifische Tricks). Und ein Codebase für Handy + TV + eventuell iOS.

**Player-Layer isolieren:** Baue eine schmale Abstraktion (`PlayerController`: `open(url)`, `play/pause`, `seek`, `volume`, `position`, `duration`, `buffered`, `isLive`), damit ein Wechsel von `media_kit` auf ExoPlayer später ein Austausch einer Klasse ist und nicht ein Umbau der App. Der Desktop-Client macht das implizit schon so (`mpv.rs` kapselt alles).

---

## 6. Datenmodell & Dateiformate

### 6.1 Speicherorte

| Datei | Windows (Ist) | Android (Soll) |
|---|---|---|
| `profiles.json` | `%APPDATA%\Tarik\Dado-Player\config\` | `getApplicationSupportDirectory()` bzw. `filesDir` |
| `settings.json` | dito | dito |
| `favorites.json` | dito | dito |
| `progress.json` | dito | dito |
| `history.json` | dito | dito |

Alle Dateien sind **UTF-8 JSON, pretty-printed**. Der Ordner heißt `com.Tarik.Dado-Player` (Windows) — auf Android nimm `applicationId = com.tarik.dadoplayer` und denselben Dateinamen-Satz, damit ein späterer Import/Export trivial bleibt.

### 6.2 `profiles.json` — Array

```json
[
  {
    "id": "8f3a1c0d9b2e4f61",
    "name": "Mein Anbieter",
    "kind": "Xtream",
    "host": "anbieter.tld:8080",
    "username": "user",
    "password": "pass",
    "m3u_url": "",
    "m3u_file": "",
    "last_channel": null
  }
]
```

- `kind`: `"Xtream"` oder `"M3u"` (exakt diese Schreibweise).
- `host` **ohne** Schema erlaubt — `normalize_host()` ergänzt `http://` und entfernt trailing `/`.
- `id`: 16 Hex-Zeichen. Auf Mobile `Uuid().v4()` oder gleichwertig; nur eindeutig muss es sein.
- **Sicherheitsstatus:** Passwörter werden in `profiles.json` im Klartext gespeichert (auch auf Desktop). Android legt die Datei im app-privaten `filesDir` ab. Android-Backup/Transfer ist per `android:allowBackup="false"` deaktiviert, damit die Datei nicht über den System-Backup-Pfad exportiert wird. Das ersetzt keine Keystore-Verschlüsselung; dafür ist eine kompatible Migration erforderlich.Änderung an bestehenden Profilen wurde vermieden.

### 6.3 `settings.json` — Objekt (alle Felder, Defaults in Klammern)

```json
{
  "cache_secs": 45,
  "network_timeout_s": 20,
  "theme": "System",
  "settings_mode": "Easy",
  "auto_play_last": true,
  "default_profile": null,
  "hwdec": "AutoSafe",
  "vo": "Gpu",
  "video_sync": "Audio",
  "interpolation": false,
  "deband": false,
  "audio_exclusive": false,
  "audio_passthrough": false,
  "audio_normalize": false,
  "sub_size": 55.0,
  "user_agent": "",
  "settings_version": 1
}
```

- `theme`: `"System" | "Dark" | "Light"` — **System ist Default und muss es bleiben**.
- `settings_mode`: `"Easy" | "Expert"`.
- `settings_version`: bei Änderungen an Defaults hochzählen und beim Laden migrieren (Muster: `if version < 1 { … }`).
- `hwdec`/`vo`/`video_sync` sind mpv-Begriffe. Auf Android mappst du sie auf `media_kit`-Properties (`hwdec`, `vo` entfällt, `audio-exclusive` etc.). **Feldnamen beibehalten**, Werte intern übersetzen — sonst ist die Settings-Datei nicht mehr zwischen Plattformen austauschbar.
- Unbekannte Felder beim Laden **ignorieren**, nicht verwerfen (Vorwärtskompatibilität).

### 6.4 `favorites.json` — Array von Keys

```json
["live:12", "vod:44", "ep:3:99"]
```

### 6.5 `progress.json` — Objekt Key → Sekunden

```json
{ "vod:1000": 401, "ep:3:99": 1320 }
```

Live-Kanäle bekommen **keinen** Fortschritt (kein Sinn), dürfen aber als Key vorkommen.

### 6.6 `history.json` — Array, neueste zuerst, **max. 60 Einträge**

```json
[
  { "key": "live:1", "ts": 1793734275, "title": "Tagesschau 24", "sub": "Live-TV" },
  { "key": "vod:1000", "ts": 1793733827, "title": "Der letzte Zug", "sub": "Film" },
  { "key": "ep:3:99", "ts": 1793730227, "title": "Dark Woods", "sub": "Staffel 2 · Folge 4" }
]
```

- `ts` = Unix-Sekunden.
- `sub` wird beim Anzeigen aus dem Key abgeleitet, ist aber gespeichert, damit Einträge auch ohne Bibliothek lesbar bleiben (z. B. wenn der Provider gerade nicht erreichbar ist).

### 6.7 Key-Schema (kritisch — nicht ändern)

| Präfix | Bedeutung | Beispiel |
|---|---|---|
| `live:{stream_id}` | Live-Kanal | `live:12` |
| `vod:{stream_id}` | Film | `vod:1000` |
| `ep:{series_id}:{episode_id}` | Folge | `ep:3:99` |

Dasselbe Schema gilt für Favoriten, Fortschritt und Verlauf. Wenn du es änderst, sind alle bestehenden Nutzerdaten wertlos.

---

## 7. Provider-API-Vertrag

### 7.1 Xtream-Codes

Basis: `{host}/player_api.php?username={u}&password={p}`

| Zweck | Request |
|---|---|
| Login / Auth | `GET {base}` ohne `action` → `user_info.auth` muss `1` sein |
| Live-Kategorien | `&action=get_live_categories` |
| VOD-Kategorien | `&action=get_vod_categories` |
| Serien-Kategorien | `&action=get_series_categories` |
| Live-Streams | `&action=get_live_streams` |
| VOD-Streams | `&action=get_vod_streams` |
| Serien | `&action=get_series` |
| Folgen einer Serie | `&action=get_series_info&series_id={id}` |
| Kurz-EPG | `&action=get_short_epg&stream_id={id}` |

**Stream-URLs** werden aus dem Stream-Base gebaut (siehe unten):

```
{stream_base}/live/{user}/{pass}/{stream_id}.m3u8
{stream_base}/movie/{user}/{pass}/{stream_id}.{container_extension}
{stream_base}/series/{user}/{pass}/{episode_id}.m3u8
```

**Stream-Base ≠ Login-Host.** Beim Login liefert `server_info.url` + `server_info.https_port` oft einen anderen, HTTPS-fähigen Endpunkt. Regel aus dem Desktop-Client, die echten Ärger gelöst hat:

> Wenn `server_info.url` nicht leer **und** `https_port` nicht leer und nicht `"0"` ist, dann ist der Stream-Base `https://{url}:{https_port}`. Sonst der Login-Host. Viele Panels verweigern Live-Streams über HTTP oder über den Login-Port.

**EPG:** `get_short_epg` liefert `title` und `description` **Base64-kodiert**. Dekodiere nur, wenn der String plausibel Base64 ist (Länge durch 4 teilbar, nur `A–Z a–z 0–9 + / =`), sonst roh verwenden. Sonst zerschießt du Panels, die bereits Klartext liefern.

### 7.2 Tolerantes Parsen (das war unser größter Bug)

Der erste Login-Versuch auf Windows scheiterte mit `error decoding response body`. Ursache: Xtream-Panels sind **nicht** sauber typisiert. Zahlen kommen als Strings, Strings als `null`, `auth` mal `1`, mal `"1"`.

Pflicht für jeden JSON-Decoder:

1. **Jedes** Feld tolerant lesen: String-Felder akzeptieren String/Number/Bool und machen daraus einen String; `null` und fehlend → `""`.
2. Zahlenfelder (`stream_id`, `num`, `id`) akzeptieren Number **und** String, sonst `0`.
3. Objekte, die fehlen können (`user_info`, `server_info`, `episodes`), sind optional.
4. `get_series_info` liefert `episodes` als **Objekt mit Staffel-Keys**, nicht als Array. Über alle Keys iterieren, flachklopfen, nach `(season, episode)` sortieren.
5. Nie auf `Content-Type: application/json` vertrauen — manche Panels schicken `text/html`. Body als Text lesen und selbst parsen.

### 7.3 M3U

- Zeilen mit `#EXTINF` starten einen Eintrag, Attribute `tvg-logo` und `group-title` (mit `"` oder `'` gequotet) auslesen, Name = alles nach dem **letzten** Komma.
- Die nächste Nicht-Kommentar-Zeile ist die Stream-URL.
- Andere `#`-Zeilen brechen die Erwartungshaltung ab (nicht als URL missbrauchen).
- `group-title` wird zur Kategorie; fehlt sie → `"Channels"`.
- IDs werden fortlaufend vergeben (1, 2, 3 …), `epg_channel_id` bleibt leer.
- Datei **oder** URL als Quelle.

### 7.4 Netzwerk-Defaults

| Parameter | Wert | Warum |
|---|---|---|
| Timeout | 20 s (Minimum 5 s erzwingen) | Manche Panels brauchen lange |
| HTTP-Pool | max. 8 idle pro Host | 6 Requests beim Laden, Pool verhindert Verbindungsaufbau pro Request |
| gzip | an | Kategorien + Streams sind groß |
| Puffer | 45 s (Presets: 20 s / 45 s / 90 s) | Sprungfestigkeit vs. Umschaltzeit |

---

## 8. UI-Spezifikation

### 8.1 Palette (exakt übernehmen)

| Rolle | Dunkel | Hell |
|---|---|---|
| Hintergrund | `#0D1015` | `#F3F5F9` |
| Panel / Sidebar | `#13171E` | `#FFFFFF` |
| Karte | `#1A202A` | `#E9EDF4` |
| Rahmen | `#262E3A` | `#D5DCE7` |
| Text | `#E8ECF3` | `#181D26` |
| Sekundärtext | `#939EB0` | `#5F6B7D` |
| **Akzent** | `#4C86FF` | `#4C86FF` |
| Akzent gedimmt | `#243A66` | `#243A66` |
| Fehler | `#E56B6B` | `#E56B6B` |
| OK / LIVE | `#4EC08A` | `#4EC08A` |

Ein Akzent, sonst nichts. Keine Schatten als Deko, keine Glows, keine Farbverläufe außer dem Logo.

**Typografie Desktop:** Heading 22, Body 14,5, Button 14, Small 11,5 (px).
**Mobile:** Body 14–16 dp, Skalierung über `textScaleFactor` respektieren.
**TV (10-Foot):** alles ×1,6, Mindest-Touchziel 48 dp (TV: 56 dp).

### 8.2 Layout-Regeln (der Nutzer hat darauf bestanden)

- **Alles, was mittig sein soll, wird zentriert berechnet** — Login-Karte, Player-Transport, Dialoge. Nie mit festen Offsets. Formel: `center = (verfügbareBreite - eigeneBreite) / 2`.
- Poster-Grid responsiv: `spalten = floor((breite - 26) / (148 + 14))`, Zelle = `breite / spalten`. Poster 2:3 (Desktop 132×190 px).
- Feste Breiten aus dem Desktop-Client (als Referenz, nicht als Pflicht): Navigation 224 px (min. 186), Vorschau-Panel 440 px (min. 300), Serien-Detail 430 px, Einstellungs-Spalte 720 px, Login-Karte 440 px.
- Sidebar links auf Desktop/Tablet/TV. Auf dem Handy: Bottom-Navigation oder Drawer — die Vorschau wird dort ein Bottom-Sheet, das nur ~45 % der Höhe einnimmt, damit die Liste sichtbar bleibt.
- Einstellungen: eine zentrierte Spalte, max. 720 px breit, auf Handy volle Breite mit 16 dp Rand.
- Keine Inhalte unter Statusleiste/NavigationBar — `SafeArea` verwenden.

### 8.3 Player (die exakte Vorgabe)

```
┌──────────────────────────────────────────────┐
│ ◀ Zurück                          Titel      │  oben, 56 px
│                                   Untertitel │
├──────────────────────────────────────────────┤
│                  video                       │
├──────────────────────────────────────────────┤
│ 0:12 ──────●────────────────────── 45:10     │  Fortschritt + Puffer
│  🔊 ────────        «10  ▶  »10        ⛶     │  Transport, unten 120 px
└──────────────────────────────────────────────┘
```

| Element | Regel |
|---|---|
| **Zurück** | Pille **oben links in der Ecke**, getrennt von der Transportleiste. Nicht unten. |
| **Titel** | **oben rechts**, rechtsbündig. Zeile 1 = Kanal/Film/Serienname, Zeile 2 = Kontext („Live-TV", „Film", „Staffel 2 · Folge 4"). |
| **Play/Pause** | Kreis 58 px, **exakt in der horizontalen UND vertikalen Mitte der unteren Leiste**. Berechnung: `Rect::from_center_size(mittelpunkt, 58×58)`. |
| **±10 s** | links/rechts vom Play-Kreis, Mittelpunkt ±76 px. Bei Live: stattdessen Kanal ◀/▶. |
| **Fortschrittsbalken** | über die volle Breite, links aktuelle Zeit, rechts Restdauer. **Zwei Ebenen**: geladen/gepuffert (heller) + Position (Akzent). Bei Live: kein Balken, stattdessen rotes „● LIVE"-Badge + Pufferanzeige. |
| **Lautstärke** | eigene Zeile links, Icon + Balken, 0–100. |
| **Vollbild** | Icon rechts außen (4 Ecken, selbst gezeichnet). |
| **Glas-Optik** | Leisten halbtransparent: `rgba(12,15,21,0.85)` über dem Video, 1 px Kante. Auf Mobile mit echtem Blur möglich. Text muss immer lesbar bleiben. |
| **Auto-Hide** | Nach **3 s** ohne Interaktion verschwinden **beide** Leisten und der Cursor; das Video nimmt die volle Fläche ein. Jede Bewegung/Taste holt sie zurück. |
| **Touch** | Tippen auf das Video toggelt die Leisten. Doppeltipp links/rechts = ±10 s. |

### 8.4 Weitere Screens

- **Login:** Karte mittig, D7-Logo oben, Segmented-Switch „Xtream / M3U", Felder darunter, Primärbutton volle Kartenbreite, Fehler als roter Text in der Karte.
- **Live:** Kategorie-Dropdown + Suchfeld in einer Zeile, darunter virtualisierte Liste. Zeile: Nummer, Logo 40 px, Name, aktuelle EPG-Sendung (klein, gedimmt), Stern, ▶.
- **Filme/Serien:** Grid mit 2:3-Postern, darunter Name (1 Zeile, Ellipsis) + Meta (Rating bzw. Staffelzahl). Hover/Fokus: 2 px Akzentrahmen.
- **Fortsetzen:** Segmented-Tabs **Live-TV / Filme / Serien** oben. Filme/Serien als Grid mit Fortschrittsbalken am Posterboden und Text „Weiter bei 12:34". Live als Zeilen mit „zuletzt: HH:MM".
- **Einstellungen:** Segmented „Einfach / Experten" ganz oben, darunter immer sichtbar „Darstellung" (System/Dunkel/Hell). Einfach zeigt nur: Farbschema, 3 Presets (Standard / Schwacher PC / Beste Qualität), Puffergröße, Autoplay. Experten zeigt zusätzlich alle Felder aus §6.3.

---

## 9. Interaktionsmodell (Touch + D-Pad)

Der Desktop-Client ist tastaturzentriert; Mobile braucht beides.

| Aktion | Touch | D-Pad / Fernbedienung |
|---|---|---|
| Kanal wechseln | Zeile antippen (startet Vorschau) | ↑/↓ in der Liste = **Fokus** bewegen **und Vorschau laden** (wie Desktop), OK = in den Player |
| In den Player | zweiter Tap auf dieselbe Zeile / ▶ | OK auf fokussierter Zeile |
| Vollbild | Player öffnen, Vollbild-Icon | OK auf dem Vollbild-Icon |
| Play/Pause | Tipp auf Video → Kreis | OK im Player |
| ±10 s | Doppeltipp links/rechts | ←/→ im Player |
| Lautstärke | Slider | ↑/↓ im Player, bzw. Hardwaretasten |
| Zurück | System-Back | Back-Taste / Esc |
| Menü einblenden | Tap | jede Richtungstaste |

**TV-spezifisch:** Fokus muss **immer** sichtbar sein (2 px Akzentrahmen, nicht nur Farbe — auch Form/Outline ändern). Kein Hover-Konzept. Der Fokus darf nie „verschwinden" (nach Listenende nicht ins Nichts springen — Wrap-Around wie auf Desktop). Beim Zappen im TV darf die Vorschau nicht bei jedem Tastendruck neu puffern, wenn der Nutzer schnell durchschaltet: **Debounce ~250 ms**, erst dann `open()`.

---

## 10. Android-spezifische Pflichten (die Fallen)

1. **Cleartext-HTTP.** Sehr viele IPTV-Panels laufen auf `http://…:8080`. Ab Android 9 ist Cleartext standardmäßig blockiert. Du brauchst `android:usesCleartextTraffic="true"` **oder** eine `network_security_config.xml` mit den Provider-Hosts. Ohne das scheitert jeder Login mit einer nichtssagenden Exception.
2. **INTERNET-Permission** in `AndroidManifest.xml` — klingt trivial, wird vergessen.
3. **HLS/TS.** Viele Provider liefern MPEG-TS in einem `.m3u8`-Container. `media_kit` (libmpv) kann das; `video_player`/ExoPlayer braucht je nach Variante Extra-Konfiguration. Vor der Stack-Entscheidung mit **einem echten Provider-Stream** testen, nicht mit einem Demo-HLS.
4. **Kein UI-Thread-Netzwerk.** Bibliothek laden = 6 Requests, teils 10 MB JSON. In einen Isolate/Coroutine auslagern, Fortschritt anzeigen, abbrechbar machen.
5. **Bilder.** 10.000 Kanal-Logos dürfen nicht alle geladen werden. Nur sichtbare laden, LRU-Cache (~100 MB), Fehler still ignorieren und Platzhalter zeigen.
6. **Listen.** `ListView.builder` / `LazyColumn` mit `itemExtent`, sonst ruckelt es ab ~2.000 Einträgen.
7. **Lifecycle.** Beim Wechsel in den Hintergrund: Wiedergabe pausieren (oder bewusst als Audio weiterlaufen lassen — dann `foregroundService` + Notification, sonst killt Android den Prozess). Beim Zurückkommen: Position wiederherstellen.
8. **Audio-Fokus.** Bei eingehendem Anruf/Benachrichtigung leise schalten, danach zurück. `AudioManager.requestAudioFocus`.
9. **Orientation.** Live-TV im Vollbild: Landscape erlauben, aber nicht erzwingen. Filme/Serien: Landscape sinnvoll. Grid-Ansichten: beide Orientierungen sauber.
10. **Screen-on.** Während Wiedergabe `keepScreenOn`/`FLAG_KEEP_SCREEN_ON`, sonst geht das Display im Film aus.
11. **TV-Manifest.** Für Android TV: `LEANBACK_LAUNCHER`-Intent, Banner 320×180, `android.software.leanback` nicht als `required` (sonst kein Install auf Handys), Touchscreen nicht als `required`.
12. **APK-Größe.** libmpv bringt ~20 MB pro ABI. `abiFilters` auf `arm64-v8a` (+ `armeabi-v7a` nur falls nötig) und App Bundle nutzen.
13. **Passwörter.** Siehe §6.2 — nicht im Klartext auf Disk.
14. **Back-Verhalten.** System-Back im Player = zurück zur Liste, **nicht** App beenden. In der Vorschau = Vorschau schließen, Liste bleibt.
15. **Kein `localhost`-Roundtrip für Bilder.** Cover/Logos direkt per URL laden (mit Cache), nicht durch die App proxyen.

---

## 11. Meilensteine

Jeder Meilenstein ist eigenständig lauffähig und vorzeigbar.

### M0 — Gerüst (kein Feature)
Projekt anlegen, Stack aus §5, Manifest-Permissions (§10.1–2), Theme-Palette aus §8.1 als Konstanten, Navigation-Shell mit den 6 Bereichen (leer), Dark/Light/System aus `MediaQuery.platformBrightness`.
**Fertig wenn:** App startet auf Handy + Emulator, Theme folgt dem System, keine Crashes.

### M1 — Provider + Bibliothek
`client`-Port (Xtream + M3U, §7 inkl. Toleranzregeln), Profil-Speicherung (§6.2), Ladebildschirm, Fehleranzeige. Noch keine Wiedergabe.
**Fertig wenn:** Ein echter Xtream-Account lädt Kanäle, Filme und Serien; Kategorien stimmen; ein zweiter Start lädt aus dem Cache/Speicher ohne erneuten Login.

### M2 — Live-Liste + Vorschau
Virtualisierte Kanalliste mit Logo, Kategorie, Suche, Favoriten-Stern. Direktstart beim Klick (ohne Preview/Bestätigung), Zappen mit Vorschau-Nachladen, Debounce (§9). **Fertig wenn:** 10.000 Kanäle flüssig scrollen, Zappen per ↑/↓ startet Live sofort, kein Speicherleck.

### M3 — Player
Vollflächiger Player nach §8.3, Auto-Hide 3 s, Fortschritt + Puffer, Lautstärke, ±10 s, Vollbild, Live-Badge, Zurück oben links, Titel oben rechts. `PlayerController`-Abstraktion.
**Fertig wenn:** Ein Film lässt sich pausieren, spulen, die Position überlebt App-Neustart; Live zeigt kein Fortschrittselement; nach 3 s ist die Leiste weg und kommt bei Berührung zurück.

### M4 — Filme & Serien
Poster-Grid (2:3, responsiv), Detailansicht, Serien → Staffeln → Folgen, Fortschritt pro Folge, „Fortsetzen"-Button sobald eine gespeicherte Position existiert.
**Fertig wenn:** Ein Film anspielen, beenden, wieder öffnen → „Weiter bei mm:ss" stimmt; Serien-Folgen sind korrekt sortiert. *(Regel im Desktop-Client: Die Position wird beim Verlassen des Players immer gespeichert, ohne Schwellwert — es gibt keinen „ab 5 Sekunden"-Filter. „Fortsetzen"/„Weiter bei" erscheint, sobald ein Eintrag in `progress.json` existiert.)*

### M5 — Fortsetzen, Suche, Einstellungen
`history.json` (§6.6) füllen, Fortsetzen mit drei Tabs und Grid/Liste wie Desktop, globale Suche, Einstellungen Einfach/Experten mit Autosave.
**Fertig wenn:** Nach 3 angespielten Inhalten stehen alle drei Tabs korrekt; Theme-Wechsel wirkt sofort ohne Neustart.

### M6 — TV & Feinschliff
D-Pad-Fokus-System, Leanback-Manifest, Banner, Fokus-Ringe, Debounce beim Zappen, Lifecycle (§10.7–10), Audio-Fokus, Keep-Screen-On.
**Fertig wenn:** Auf einem Fire-TV-Stick oder Android-TV-Emulator ist die komplette App **ohne Touch** bedienbar und der Fokus ist immer sichtbar.

### M7 — Release
Icon (D7), Splash, APK/AAB, `arm64-v8a`, Versionierung, „Über"-Seite mit Version + Lizenzhinweis (libmpv = GPL/LGPL — prüfen!), keine Tracker.
**Fertig wenn:** Eine signierte APK auf einem frischen Gerät installiert, einloggt und abspielt.

---

## 12. Nicht-Ziele (bewusst weggelassen)

- Keine eigenen Inhalte, keine Provider-Liste, keine Empfehlungen, kein „Store".
- Keine Accounts, keine Cloud-Sync, keine Telemetrie, kein Crash-Reporting.
- Kein Chromecast/AirPlay in v1 (später denkbar, aber nicht Teil der Parität).
- Keine Aufnahmefunktion, kein Timeshift.
- Kein EPG-Vollraster in v1 — nur „jetzt läuft" pro Kanal (der Desktop-Client hat auch nur Kurz-EPG).
- Keine Untertitel-/Tonspur-Auswahl in v1 (offener Punkt auch auf Desktop).

---

## 13. Risiken aus der Desktop-Entwicklung (schon bezahltes Lehrgeld)

| Problem | Was passiert ist | Regel für Mobile |
|---|---|---|
| Xtream-JSON | `error decoding response body` beim ersten Login, weil `auth` als String kam | §7.2 strikt umsetzen |
| Live über HTTP | Streams liefen nicht, weil der Login-Port kein Streaming kann | `https_port` aus `server_info` bevorzugen (§7.1) |
| EPG-Titel | Kamen als Base64-Müll an | Plausibilitätsprüfung vor dem Dekodieren |
| Weißer Blitz | Beim Öffnen der Videofläche blitzte Weiß auf | Hintergrund **vor** dem ersten Frame schwarz setzen |
| Maus über Video | Klicks kamen nicht an, Auto-Hide ließ sich nicht aufwecken | Auf Mobile irrelevant, aber: Gesten über dem Video brauchen ein `GestureDetector` **über** der Video-Surface, nicht darunter |
| Fenster zu groß | Player-Leiste lag hinter der Taskleiste | Immer `SafeArea` + verfügbare Fläche abfragen, nie volle Bildschirmhöhe annehmen |
| Passwort im Fehlertext | Provider-Fehler zeigte das Passwort im Klartext | Fehlermeldungen maskieren (`password=***`) |
| Hell-Modus | Überschriften weiß auf weiß | Palette konsequent aus §8.1 ziehen, nie „Standard-Textfarbe" annehmen |

---

## 14. Übergabe-Checkliste

**Mitnehmen (kopieren/portieren):**
- [ ] `src/types.rs` → Datenmodell (§6)
- [ ] `src/client.rs` → Netzwerk + Parser (§7), inkl. aller Toleranzregeln
- [ ] `src/store.rs` → Persistenz (§6.1), nur Pfade anpassen
- [ ] `src/theme.rs` → Hex-Werte (§8.1)
- [ ] `src/ui/player.rs` → nur als Geometrie-Vorlage lesen (§8.3)
- [ ] `assets/icon.png` → D7-Icon als Basis für App-Icon/Banner

**Nicht anfassen / nicht portieren:**
- [ ] `src/mpv.rs`, `src/video.rs`, `src/sys.rs` — Windows-spezifisch
- [ ] `src/demo.rs` — nur Dev-Daten
- [ ] `Cargo.toml`, `.cargo/config.toml` — **angepasst** für den Android-cdylib-Build (siehe §15); `target/`, `dist/` bleiben reine Desktop-Build-Artefakte

**Vor dem ersten Commit klären:**
- [ ] Stack-Entscheidung (§5) mit dem Nutzer
- [ ] Zielgeräte: nur Handy? + Tablet? + Android TV? (beeinflusst M6)
- [ ] Ein **echter** Test-Account für die Entwicklung (Xtream oder M3U) — ohne den ist M1 nicht abnehmbar
- [ ] Ist iOS wirklich geplant? (ändert die Stack-Wahl, nicht das Design)

**Definition of Done für v1.0 Mobile:** M0–M7 abgehakt, Paritätstabelle aus §2 vollständig, auf einem echten Gerät mit einem echten Provider getestet, kein Crash in 30 Minuten Nutzung, APK < 60 MB.

---

## 15. Mobile-Stand: Rust/egui-Port (Abweichung von §5)

> **Entscheidung (mit Nutzer geklärt):** Statt der in §5 empfohlenen Flutter-Variante
> wurde der **bestehende Rust/egui-Desktop-Code portiert** (kein Rewrite). Die
> Wiedergabe-Engine bleibt **libmpv** — auf Android jedoch via Rust-FFI
> (`libloading` + `android-surface`), nicht über Flutters `media_kit`. Provider-
> Streams verhalten sich damit identisch zum Desktop (§4 nennt libmpv für Android
> ohnehin als Option; hier nur als Rust-FFI statt Flutter-Binding).

**Was das für §3–§9 bedeutet:**
- Die Feature-Parität (§2) wird über die *bestehende* `src/`-Struktur erreicht.
  §3–§9 bleiben als Verhaltens-Vorgabe voll gültig.
- `src/mpv.rs`, `src/video.rs`, `src/sys.rs` sind **doch portabel** gemacht worden
  (plattform-neutral via `cfg(...)` + `VideoSurface`-Enum in `src/platform/`),
  statt — wie in §4 behauptet — weggeworfen. `src/platform/{windows,android,ios}/`
  kapselt die OS-spezifika; `app.rs` greift plattformneutral zu.

**APK-Bau (CI-only, siehe `.github/workflows/android-apk.yml` + `android/README.md`):**
- `cargo ndk` → `libdadoplayer.so` (arm64-v8a) + Gradle → APK.
- `libmpv.so` **samt ffmpeg/c++-Deps** wird im CI automatisch aus dem gepinnten
  mpv-android Release-APK (arm64-v8a) extrahiert (dlopen braucht die DT_NEEDED-
  Partner, nicht nur libmpv.so).

**Bereits erledigt (Scaffold):**
- ✅ Echtes Gradle-8.9-Wrapper, `GameActivity.java`-Glue, `androidx.games:games-activity:2.0.2`.
- ✅ `android:usesCleartextTraffic="true"` im Manifest (§10.1 — sonst scheitern HTTP-Logins still).
- ✅ `INTERNET`-Permission (§10.2).
- ⬜ `store.rs`: Android-Speicherpfade (`filesDir`, §6.1) noch nicht angebunden
  (nutzt aktuell noch die `directories`-Crate → Windows-Pfade). Muss vor M1 angepasst werden.
- ⬜ `assets/icon.png` → echtes Launcher-Icon/Banner (aktuell Platzhalter-Vektor).
- ⬜ Laufzeit-Verhalten aus §10 (Lifecycle, Audio-Fokus, Keep-Screen-On, Bilder-LRU)
  und M1–M7 sind noch nicht umgesetzt — derzeit steht nur das build-fähige Gerüst.

**Lokaler Build 2026-11-03 (ERFOLGREICH):**
- Toolchain (temporär in `%TEMP%`): Temurin JDK 17, Android cmdline-tools, SDK
  `platforms;android-34`, `build-tools;34.0.0`, `ndk;27.0.12077973`, `cargo-ndk` 4.1.2,
  Rust-Target `aarch64-linux-android`.
- `cargo ndk -t arm64-v8a --platform 24 -o android/app/src/main/jniLibs build --release --lib`
  → `libdadoplayer.so` (6,6 MB).
- Gradle 8.9 (`gradle assembleDebug`) → `android/app/build/outputs/apk/debug/app-debug.apk`
  (41,8 MB, alle 11 `.so` enthalten; `aapt` bestätigt `native-code: arm64-v8a`).
- `apksigner verify` grün (Debug-Zertifikat), `usesCleartextTraffic=true` im kompilierten Manifest.

**Beim Build behobene Blocker:**
- `entry.rs`: winit 0.29 braucht `EventLoopBuilderExtAndroid::with_android_app` — über
  `NativeOptions.event_loop_builder` gesetzt (sonst Panic in `EventLoop::build()`).
  `AppCreator` liefert `Box<dyn App>` (kein `Result`); `NativeWindow::ptr()` ist `NonNull` → `.as_ptr()`.
- `winit = "0.29"` (Feature `android-game-activity`) als Android-Dep ergänzt.
- `GameActivity.java` lag falsch: echte Basisklasse ist **`com.google.androidgamesdk.GameActivity`**
  (nicht `androidx.games.activity`). Jetzt `com.tarik.dadoplayer.GameActivity`, Superklasse per FQN
  (ein Import würde mit dem eigenen Klassennamen kollidieren, JLS 7.5.1).
- `GameActivity` erbt `AppCompatActivity` → **AppCompat-Theme** nötig
  (`Theme.AppCompat.NoActionBar` + `windowFullscreen`) statt Framework-Material-Theme.
- `androidx.appcompat:appcompat:1.7.0` ergänzt (POM von games-activity deklariert keine Deps);
  Kotlin-BOM 1.8.22 gegen `kotlin-stdlib-jdk7/jdk8:1.6.21`-Duplikate.
- `gradle-wrapper.jar` war unvollständig (`IDownload` fehlte) → Main+Shared-Jar gemerged;
  der Build lief final über die volle Gradle-8.9-Distribution.
- `gradle.properties` mit `android.useAndroidX=true` ergänzt.

**Offen (Laufzeit, nicht Build):** egui (glow) und libmpv rendern beide auf dasselbe
`ANativeWindow` — auf Windows nutzt die App ein Kindfenster (`wid`), auf Android gibt es das
nicht. Für flüssige Video-Wiedergabe braucht es eine getrennte `SurfaceView`/`SurfaceTexture`
für mpv (oder mpv → Textur → egui). UI/Login starten; Video-Surface-Integration ist der nächste Schritt.

**Korrektur zu §14 "Nicht anfassen":** `Cargo.toml` und `.cargo/config.toml` wurden
**bewusst** für den Android-cdylib-Build erweitert (zusätzliches `[lib]` cdylib,
`android-activity`/`ndk`-Deps, `reqwest` auf `rustls-tls` statt openssl). Sie sind
Teil des Ports, nicht nur Desktop-Build. `target/` und `dist/` bleiben reine
Desktop-Artefakte.

## 16. iOS-Build: xtool (Stand 2026-11-12)

> **Nutzer-Hinweis (2026-11-03, unverändert gültig):** Für iOS soll
> **https://github.com/xtool-org/xtool** verwendet werden — ein Open-Source-„Xcode-Ersatz",
> der SwiftPM-Pakete zu iOS-Apps baut, signiert und auf Geräten installiert.

### 16.1 Host-Anforderungen (aus der offiziellen Doku)

Quellen: `https://xtool.sh/documentation/xtool/installation-linux` und das First-App-Tutorial.

- Unterstützte Hosts: **Linux, macOS und Windows über WSL**. Natives Windows ist **nicht**
  unterstützt.
- Swift **6.4**, `usbmuxd`, Xcode **27.x** (`.xip` von `developer.apple.com`, Download im
  Browser mit Apple ID — nicht per `curl`).
- `xtool setup` fragt den Login-Modus ab: `0` = API Key (bezahltes Apple Developer Program),
  `1` = Passwort (jede Apple ID, nutzt private APIs).
- Danach wird das Xcode-`.xip` in ein Darwin-Swift-SDK entpackt; `swift sdk list` zeigt
  `darwin`.
- `xtool new <Name>` legt ein SwiftPM-Paket an (`Package.swift`, `xtool.yml`,
  `.bsp/xtool.json`, `Sources/<Name>/*.swift`). Bei uns nicht nötig, das Paket existiert.
- `xtool dev` baut, signiert, registriert das Gerät, provisioniert, installiert und kann
  starten.
- **xtool hat keine dokumentierte Rust-, Static-Library-, `.a`- oder
  xcframework-Integration.** Rust wird über gewöhnliche SwiftPM-Linker-Einstellungen
  eingebunden — genau das tut `ios/Package.swift`.
- USB-Gerätezugriff aus WSL braucht USBIPD. Für den bekannten `AFCClient.Error.muxError`
  gibt es einen dokumentierten Workaround, der über iTunes auf der Windows-Seite relayt.

### 16.2 Natives Windows ist nicht möglich, WSL ist hier nicht installiert

Beides ist auf dieser Maschine gemessen, nicht vermutet:

- `cargo check --target aarch64-apple-ios` scheitert in `objc_exception v0.1.2`:
  `error occurred in cc-rs: failed to find tool "xcrun": program not found`. `cc-rs` fragt das
  SDK über `xcrun --sdk iphoneos --show-sdk-path` ab; `xcrun` gibt es nur mit Apple-Toolchain.
  `ring v0.17.14` trifft derselbe Fehler.
- `wsl.exe --status` endet mit Exit 50:
  `Der Windows-Subsystem für Linux ist nicht installiert.`

Zwei Dinge, die leicht verwechselt werden: der Eintrag `wsl.exe` in der
**Program Blacklist** (Security Center) ist aufgehoben — der Prozess startet wieder und
gibt `--help` vollständig aus. Die **Windows-Funktion** war davon unabhängig und ist
nie aktiviert worden. `wsl.exe --install` aus einer nicht erhöhten Sitzung scheitert mit
Exit 1; die Elevation muss der Benutzer selbst auslösen:

```powershell
wsl.exe --install --no-distribution   # erhöhte PowerShell, danach Neustart
```

Damit ist auf diesem Rechner **keiner** der beiden Bauwege begehbar. Der iOS-Stand ist
Quelltext, der auf einer anderen Maschine zum ersten Mal übersetzt wird. Weg B
(GitHub-Actions, `macos-14`) braucht kein WSL, sondern nur ein Repository — der
Workflow `.github/workflows/rust-gates.yml` liegt fertig vor.

### 16.3 Was implementiert wurde

- `Cargo.toml`: `staticlib` in `[lib] crate-type`, Feature `ios-check`.
- `src/lib.rs`: `crate::mpv` wird bei `target_os = "ios"` auf `src/mpv_ios.rs` aliast;
  `dado_app_start` wird re-exportiert.
- `src/platform/ios/mod.rs`, `entry.rs`, `sys.rs`, `video_surface.rs`, `bridge.rs`:
  Plattformmodul, Entry mit `Renderer::Wgpu` und `transparent = true`, System-Fassade,
  Video-Oberfläche, C-ABI-Implementierung.
- `src/mpv_ios.rs`: AVPlayer-Shim mit derselben öffentlichen Oberfläche wie `src/mpv.rs`.
- `ios/Sources/DadoHost/include/DadoBridge.h`: ABI-Header, 3× `dado_app_*`, 16× `dado_host_*`.
- `ios/Sources/DadoHost/module.modulemap`, `ios/Package.swift`, `ios/xtool.yml`.
- `ios/Sources/DadoPlayer/{main,AppDelegate,RootViewController,VideoView,HostBridge}.swift`.
- `probe/build_ios.sh`: Preflight, `cargo build --release --target … --lib`, Kopie nach
  `ios/Vendor/libdadoplayer.a`, `nm`-ABI-Check, optional `xtool dev` (`--deploy`) bzw.
  Simulator-Slice (`--sim`).

### 16.4 Was verifiziert ist — und was nicht

Verifiziert auf dieser Maschine:

- `cargo check --features ios-check` → PASS (type-checked `bridge.rs`, `sys.rs`,
  `mpv_ios.rs` gegen den Windows-Host; `entry.rs` und `video_surface.rs` sind dabei per
  `cfg` ausgeschlossen).
- `cargo check --lib --bin Dado-Player` → PASS, der Windows-Build ist unverändert.
- `bash probe/build_ios.sh` → bricht mit `!! No Apple iOS SDK found.` ab. Das ist das
  erwartete Verhalten.

Nicht verifiziert (kein SDK, kein Swift-Compiler für iOS, keine `.a`):

- Ob `ios/Package.swift` von SwiftPM aufgelöst wird und `import DadoBridge` funktioniert.
- Ob der Swift-Host kompiliert.
- Ob `libdadoplayer.a` linkt und die 16 `dado_host_*`-Symbole aufgelöst werden.
- Ob winit 0.29 + eframe 0.27 auf iOS startet.
- Ob die Videoebene hinter der egui-Oberfläche sichtbar ist (Layering/Transparenz).
- Ob HLS abspielt, ob Safe-Area und IME-Insets ankommen.

### 16.5 Betriebsanleitung

Die konkreten Schritte, Kommandos und Abnahmekriterien stehen in **`README.iOS.md`**
(Status, beide Bauwege, Projektlayout, Architekturentscheidungen, Verifikations-Checkliste,
offene Risiken). Dieses Kapitel hält nur den Planungsstand fest.

### 16.6 Zwei Blocker, die kein Build-Fehler sind

Beide sind beim statischen Abgleich aufgefallen und würden auch mit perfektem SDK zuschlagen:

1. **Kein Text-Input.** winit 0.29 hat im iOS-Backend keinen Texteingabe-Pfad — kein
   `set_ime_allowed`, kein `becomeFirstResponder`, kein `UITextField`. Das Login-Formular
   (Host/Benutzer/Passwort/M3U-URL) ist damit nicht ausfüllbar; die App ist ohne
   Host-seitige Tastatur-Brücke nicht benutzbar. Der Fix gehört in den Swift-Host (unsichtbares
   `UITextField` + ABI-Callbacks an egui), nicht in die UI: die egui-Textfelder funktionieren,
   ihnen fehlt nur die Tastatur. Aufwand: ein neuer ABI-Block, ~4 Funktionen.
2. **Layering ist unbewiesen.** Die Video-`UIView` liegt an Index 0 der winit-Root-View,
   `transparent = true` auf der egui-Seite. Ob winits Root-View selbst die Metal-Surface ist
   (dann läge das Video *über* der UI), lässt sich nur am Gerät entscheiden. Fallback steht im
   Code dokumentiert: Video-View stattdessen direkt an das `UIWindow` hängen.

## 17. APK-Test ohne Android-Gerät (Emulator) — Stand 2026-11-03

**Ziel:** Prüfen, ob die APK startet, wie sie aussieht und was schon funktioniert —
ohne physisches Android-Gerät.

### 17.1 Setup (einmalig, ist auf diesem Rechner erledigt)
```bash
SDK="$TEMP/android-build/sdkroot"
"$SDK/cmdline-tools/latest/bin/sdkmanager.bat" --install \
    "emulator" "system-images;android-34;google_apis;x86_64"
```
- Hardware-Beschleunigung: `emulator -accel-check` → **WHPX aktiv** (Hyper-V läuft bereits).
- AVD: `dado_test` (Pixel 6, Android 14 / API 34, 3072 MB RAM), angelegt von
  `android/emulator-test.sh`.
- **Wichtig:** Das APK ist **nur arm64-v8a**. Auf dem x86_64-Image läuft es trotzdem, weil
  Googles API-30+-`google_apis`-Images die ARM→x86-Übersetzung (`libndk_translation`)
  mitbringen. Installation war erfolgreich, `INSTALL_FAILED_NO_MATCHING_ABIS` trat nicht auf.

### 17.2 Bedienung
| Zweck | Befehl |
|---|---|
| Alles automatisch (booten, installieren, Screenshots, beenden) | `bash android/emulator-test.sh` |
| Emulator-Fenster zum Selbst-Klicken | `Start-Emulator.bat` (Doppelklick) |
| Nur Screenshot | `adb -s emulator-5554 exec-out screencap -p > shot.png` |
| Log mitlesen | `adb -s emulator-5554 logcat -s RustStdoutStderr` |
| Emulator beenden | `adb -s emulator-5554 emu kill` |

### 17.3 Ergebnis: was auf Android nachweislich läuft
- ✅ Installation + Start (`Displayed com.tarik.dadoplayer/.GameActivity: +5s594ms`).
- ✅ egui/glow rendert (GLES über `libEGL_emulation`, `s_glBindAttribLocation`-Logs).
- ✅ Touch-Eingabe: Tabs umschalten, Scrollen, Buttons — funktioniert.
- ✅ Layout im Querformat korrekt (Login-Karte zentriert, ~440 dp breit).
- ✅ `libdadoplayer.so` wird gefunden und geladen, kein Crash, kein `UnsatisfiedLinkError`.
- ❌ **Video-Wiedergabe** (siehe 17.4).

### 17.4 Beim Test gefundene und behobene Fehler
1. **Login-Formular war abgeschnitten** (Passwort + „Anmelden" + „Teststream" außerhalb des
   Screens, kein Scrollen) → `src/ui/login.rs`: `ScrollArea::vertical()` um die Karte,
   Kartenbreite `clamp(280, 440)`.
2. **Portrait unbrauchbar** (Karte breiter als das Display) → Manifest
   `screenOrientation="sensorLandscape"` statt `fullSensor`.
3. **Player-Chrome verschwand dauerhaft**: `ui_activity` wurde nur über den *Desktop-Cursor*
   (`note_pointer_activity`) erneuert; auf Android gibt es keinen Cursor → nach 3 s war die
   Leiste weg und kam nie zurück (App wirkte „eingefroren"). → `src/ui/player.rs`:
   Chrome auf Android dauerhaft sichtbar.
4. **`android-surface`-Option existiert in libmpv nicht** → `Mpv::new` scheiterte immer mit
   `option not found`.

### 17.5 Offener Blocker: Videoausgabe
Empirisch belegt (Logcat, `terminal=yes`):
- Stream öffnet, HLS parst, Video/Audio werden erkannt
  (`● Video --vid=5 (h264 [High] 1920x1080 60 fps)`, `AO: [aaudio] 44100Hz stereo`).
- `[vo/gpu/android] No Java virtual machine has been registered` →
  `Failed initializing any suitable GPU context` → `Video: no video`.
  mpv braucht für `gpu-context=android` eine registrierte `JavaVM`, die es nur über sein
  eigenes `JNI_OnLoad` bekommt — dieses Symbol ist im mpv-Versionsskript **nicht exportiert**
  (per `llvm-nm -D` geprüft).
- Setzt man stattdessen `wid` auf das `ANativeWindow` der GameActivity, **übernimmt mpv die
  EGL-Surface von egui** → kompletter Bildschirm schwarz, UI reagiert nicht mehr.
- Zusätzlich: ohne CA-Bundle schlägt jede `https://`-Verbindung fehl
  (`TLS certificate verification failed`) — Android hat keinen OpenSSL-CA-Store.

**Aktueller Kompromiss:** `Mpv::new` bricht auf Android **vor** `mpv_create` mit einer
klaren Meldung ab („Videoausgabe auf Android noch nicht angebunden"). UI bleibt bedienbar.

**Nächster Schritt (der eigentliche Fix):** libmpv-**Render-API** statt Fenster-Overlay —
`mpv_render_context_create` mit egui's GL-Kontext, Video in eine FBO/Textur rendern und als
`egui::Image` zeichnen. Damit gibt es keinen Streit ums `ANativeWindow`, kein JNI-Problem und
die Controls liegen sauber über dem Video. Zusätzlich nötig: `cacert.pem` mitliefern
(oder `tls-verify=no` bewusst als Option).

---

## 18. Touch-UI: Layout-Regeln aus der Geräte-QA

Gilt für die Rust/egui-Oberfläche (§15), ergänzt §8.2. Alle Werte in dp. Grundlage sind
Messungen an acht Geräteaufnahmen (411,4 × 914,2 dp, 2,6254 px/dp), nicht Schätzungen.

### 18.1 Inhaltsbreite und Ränder

- `pad = 16 + side`, mit `side = max(systemBars.left, systemBars.right, displayCutout.left, displayCutout.right)`.
- **`systemGestures` gehört nicht in die Ränder.** Der Zurück-Gestenstreifen ist eine
  Berührungszone, kein reservierter Platz. Er hat auf dem Gerät einen Phantomrand von
  29,7 dp je Seite erzeugt: Inhaltsbreite 320 statt 379 dp, Poster-Grid 2 statt 3 Spalten,
  Plot-Spalte 210 statt 269 dp.
- Jede Komponente, die einen Rand braucht, nimmt ihn aus `Layout::pad` — **nie als Literal**.
  Betroffen waren `chip_row` (16), `list_row` (8) und `app_bar` (8). Ein Literal bricht,
  sobald ein echtes Seiten-Inset dazukommt (Querformat mit Cutout), und es bricht schon
  vorher die Ausrichtung: auf einem Bildschirm gab es vier linke Kanten (8 / 16 / 45,7 / 48).
- **Genau eine linke Kante pro Bildschirm.** Sind zwei Elemente unterschiedlich eingerückt,
  ist einer von beiden falsch.

### 18.2 Text: Zeilen, nie Zeichen

- Prosa (Plot, Beschreibung) wird über **Zeilen** begrenzt: `PLOT_ROWS = 3`,
  `PLOT_ROWS_COMPACT = 2`. Niemals über `chars().take(n)` — eine Zeichenbegrenzung ist
  blind gegenüber Breite und Schriftgröße und begrenzt deshalb nicht, was sie begrenzen soll.
- Zusätzlich eine **Lesebreite**: Prosa max. **460 dp** (~65 Zeichen) in jeder Ausrichtung,
  linksbündig in der Spalte. Im Querformat maß die volle Spalte 655 dp ≈ 100 Zeichen pro
  Zeile — das Doppelte des lesbaren Bereichs.
- Die Zeilenbegrenzung **ersetzt** die Zeichenbegrenzung, sie ergänzt sie nicht: `take()`
  wird gelöscht, nicht verkürzt.
- Belegt am Gerät: 258 Zeichen → Hochformat 3 Zeilen + `…`, Querformat 3 Zeilen ohne `…`
  und vollständig. Die Begrenzung greift also nach Zeilen und nicht nach Zeichen.

### 18.3 Detail-Hero im Querformat

Nutzbare Höhe < 400 dp (Querformat-Telefon): Hero **96 dp** hoch.

| Element | Hochformat | Querformat (< 400 dp nutzbar) |
|---|---|---|
| Cover | 96 × 144 dp | 64 × 96 dp |
| Titel | `FS_HEAD`, 1 Zeile | `FS_HEAD`, 1 Zeile |
| Meta-Zeile | `FS_LABEL` 13 pt, 1 Zeile | **entfällt** |
| Plot | `PLOT_ROWS` = 3 | `PLOT_ROWS_COMPACT` = 2 |
| Staffel-Chips | darunter, 36 dp | darunter, 36 dp |
| Erste Folgenzeile | — | **muss vollständig über der Navigationsleiste liegen** |

**Meta-Zeile (Hochformat):** 8 dp unter dem Plot, `FS_LABEL` 13 pt, `muted()`, eine Zeile.
Serie: „{n} Staffeln · {m} Folgen", aus `series_episodes_for(sid)` gezählt (eindeutige
`season`-Werte, Gesamtzahl `Episode`). Film: die Bewertung, dasselbe Format wie im Grid.
Solange die Folgen noch nicht geladen sind (`None`), entfällt die Zeile — kein Platzhalter,
kein „…".

**Warum die Zeile existiert:** der Hero hatte zwischen dem letzten Plot-Strich und den
Staffel-Chips gemessene **130 dp** Leerraum. Davon waren 22 dp doppelt gezählter Abstand
(§18.8) und **108 dp ungenutzte Coverhöhe** — das Cover ist 144 dp hoch, die Textspalte trug
nur Titel und drei Zeilen, also ~82 dp. Eine Detail-Kopfzeile, die nur einen Titel und einen
kurzen Anriss trägt, hinterlässt immer ein lochförmiges Poster hoch Leerraum. Die Ursache ist
nicht, dass die Chips zu weit weg stehen, sondern dass die Spalte zu wenig enthält.

Mit der Meta-Zeile: 22 (Titel) + 12 + 60 (3 Zeilen) + 8 + 20 (Meta) = **122 von 144 dp**.
Toter Raum 22 dp, plus die 22 dp Abstandsfehler → **~44 dp statt 130**.

**Verworfen — und warum**, damit es nicht wieder aufkommt:

- **Chips in die Textspalte** (unter den Plot): nein. Eine Chip-Reihe muss bei mehr als drei
  Staffeln waagerecht scrollen; in einer 269 dp breiten Spalte wird daraus ein enger
  Schieber. Und die Chips sind die Kopfzeile der *Folgenliste* — sie gehören an deren linke
  Kante, nicht in die Spalte des Covers.
- **Cover 96 × 96 im Hochformat**: nein. Das Quellbild ist 2:3. Ein quadratischer Kasten
  bedeutet entweder Verzerrung oder Beschnitt, und der Beschnitt frisst bei einem Poster
  genau die Titelzeile oben und die Credits unten.
- **Lassen wie es ist**: nein, 130 dp sind 14 % der Bildschirmhöhe an der wertvollsten
  Stelle.

Rechnung bei 914 × 411 dp: nutzbare Höhe **253 dp** (411 − 22 Status − 48 App-Bar − 56 Nav −
24 Geste). Mit Hero 143 dp (Ist-Zustand) bleiben 31 dp — keine Folgenzeile passt, der Nutzer
sieht im Querformat keinen einzigen Inhalt. Mit Hero 96 dp:
12 + 96 + 12 + 36 + 8 + 72 = **236 dp**, also eine ganze Folgenzeile plus 17 dp der nächsten.

**Ist-Zustand, gemessen (APK `fae3b44a`, `B2_series_detail_landscape.png`):** `series.rs:203`
setzt `let cover_w = 96.0;` ohne Querformat-Zweig, die Höhe ist `cover_w * 1.5` = **144 dp**.
Der Hero ist im Querformat also 144 dp hoch, nicht 96 — und die Rechnung oben beschreibt
genau das, was die Aufnahme zeigt: Cover, Titel, eine Plot-Zeile, Meta-Zeile, Chips,
**keine einzige Folgenzeile**. Zusätzlich rendert der Code die Meta-Zeile im Querformat,
obwohl die Tabelle sie dort ausblendet. Drei Abweichungen, eine Ursache: es gibt keinen
Querformat-Zweig im Hero.

**Drei Vorhersagen für den Fix** (nach §22.8 vorher aufgeschrieben, nicht hinterher):

| Beobachtung | Cover 96→64 **und** Plot 3→2 **und** Meta entfällt | nur Cover 96→64 |
|---|---|---|
| Cover | 64 × 96 dp | 64 × 96 dp |
| Hero | 96 dp | ≥ 115 dp |
| Meta-Zeile | **weg** | noch da |
| Sichtbare Folgenzeilen | **genau 1** | 0 |

Die Zeile, die die beiden Kandidaten trennt, ist die **Meta-Zeile**, nicht die Folgenzahl.
Wer nur das Cover verkleinert, bekommt weiterhin null Zeilen: die Textspalte braucht bei drei
Plot-Zeilen 26 + 4 + 3×20 + 8 + 17 ≈ **115 dp** und zieht den Hero wieder über 96 dp. Die drei
Änderungen gehören zusammen. `PLOT_ROWS_COMPACT` existiert in `tokens.rs` noch nicht — der
Plot ist überall `PLOT_ROWS` = 3.

Warum die Meta-Zeile im Querformat entfällt, obwohl sie im Hochformat den Leerraum füllt: sie
existiert, um ein Loch zu schließen, das nur entsteht, weil das Cover 144 dp hoch ist. Bei
96 dp Hero und 70 dp Textspalte (Titel + zwei Zeilen) gibt es kein Loch. Eine Zeile, die nur
dazu da ist, Leerraum zu füllen, verschwindet mit dem Leerraum.

**Film: keine Detailseite.** Die Bewertung braucht keinen neuen Bildschirm — sie steht schon
auf der Kachel: `movies.rs:88-94` setzt `meta: format!("IMDb {}", m.rating)`, und
`poster_card` zeichnet `meta` bei `components.rs:713-717` in `FS_CAP` / `muted()` 26 dp unter
dem Poster. Damit ist „die Bewertung hat keinen Ort" nicht zutreffend; sie hat einen, seit es
das Grid gibt.

Die Regel dahinter, weil sie den nächsten Fall entscheidet: **eine Detailseite wird durch eine
Liste begründet, nicht durch Metadaten.** Die Serienseite existiert, weil die Folgenliste
irgendwo liegen muss. Ein Film ist ein Tipp bis zum Abspielen; eine Seite dazwischen kostet
einen Tipp auf dem häufigsten Pfad und fügt nichts hinzu, was die Kachel nicht schon sagt
(Titel, Cover, Bewertung). Wenn der Plot eines Films je gebraucht wird, gehört er in den
Leerlaufzustand des Players (§19, Z1) — dort gibt es bereits eine Fläche über dem schwarzen
Band — und nicht auf eine neue Seite.

### 18.4 Navigation ab 600 dp nutzbarer Breite

- **< 600 dp:** untere Leiste, `NAV_H` 56 dp + `bottom_inset`, 5 Ziele gleichmäßig verteilt.
- **≥ 600 dp:** **Rail links**, 72 dp breit, volle Höhe zwischen den Insets. 5 Ziele
  untereinander, je 72 × 64 dp: Icon 24 dp mittig, Label `FS_CAP` 11,5 pt darunter.
  Auswahl = Akzent-Pille 56 × 32 hinter dem Icon, Icon und Label in `accent_text()`.
  Keine untere Leiste.
- Begründung: bei 914 dp verteilt die untere Leiste fünf Labels auf je 183 dp und kostet
  80 dp der knappsten Dimension. Der Rail tauscht das in Breite um, die im Querformat
  reichlich vorhanden ist. Schwelle auf **nutzbarer** Breite, wie bei der Höhe.

### 18.5 Fehlendes Cover

Ein Poster ohne Bild ist kein leerer Kasten.

- Fläche `surface()`, Radius `RADIUS_SM` 12 dp.
- Initiale des Titels mittig, `bold(FS_DISPLAY)` 24 pt, `muted()`.
  Bei Karten < 120 dp Breite: `bold(FS_TITLE)` 20 pt.
- Ohne Titel: 24 dp Icon (`Icon::Film` bzw. `Icon::Tv`) in `muted()`.
- Gleiche Behandlung wie der Detail-Hero (`thumb()`), nur skaliert. **Zwei Behandlungen für
  denselben Zustand sind ein Fehler** — aktuell zeichnet der Hero eine Buchstabenkachel,
  das Grid eine leere Fläche.

### 18.6 Zählerzeile („17 Sender")

- **Regel: angeheftet wird nur, was sich durch die aktuelle Eingabe ändert.** Eine Zahl, die
  eine feste Eigenschaft der Liste ist, scrollt mit dem Inhalt.
- Die Live-Zählerzeile scrollt daher mit. Angeheftet kostet sie dauerhaft ~44 dp, davon 8 dp
  Zeile und 36 dp Luft — für eine Zahl, die beim Scrollen nie einen anderen Wert annimmt.
- Abstände: 8 dp über, 12 dp unter der Zeile.
- Ausnahme: eine Trefferzahl, die sich beim Tippen ändert (Suche), **darf** angeheftet
  werden — dort ist sie Rückmeldung auf eine Eingabe, nicht Etikett einer Liste.
- Zähler nur auf einspaltigen Listen, die länger als ein Bildschirm sind.

### 18.7 Login: sekundäre Aktion bei offener Tastatur

- „Ohne Anbieter testen" wird nur bei `l.ime <= 0` gezeichnet. Bewusst: der Platz über der
  Tastatur ist die knappste Ressource des Bildschirms, und der Demo-Pfad ist gegenüber einem
  echten Login nachrangig. Der Primärbutton bleibt in beiden Fällen sichtbar, in einer
  angehefteten Fußleiste mit 1 px `hairline()` darüber.
- Belegt am Gerät: bei offener Tastatur liegt „Anmelden" vollständig über dem IME, das
  fokussierte Feld trägt einen 2 px Akzentrahmen.

**Speicher-Banner: es fügt ein, es reserviert nicht.** (Entschieden, nicht offen.) Das Banner
steht am **obersten Rand des scrollbaren Inhalts** — `login.rs:194` ruft
`components::storage_banner` innerhalb von `body()`, und `body()` liegt in der `ScrollArea`
ab `login.rs:88`. Nie in der Aktionsleiste, nie als fester Streifen darüber.

Warum nicht reservieren: ein reservierter Streifen ist auf einem gesunden Gerät dauerhaft
leer, also 40 dp dauerhaft weg für einen Zustand, der fast nie eintritt. Ein Layout, das sich
nur verschiebt, wenn die Meldung wahr ist, ist der ehrliche Preis.

Warum das den IME-Fall nicht kaputt macht: die Aktionsleiste liegt **außerhalb** der
`ScrollArea` (`action_height`, `login.rs:118`) und wird von einer Einfügung im Inhalt nicht
bewegt. Der Primärbutton kann also nicht unter die Tastatur rutschen — genau der Fehler, der
bei einem festen Streifen über der Leiste entstanden wäre.

Abnahmeprüfungen für das Banner:

| Prüfung | Erwartung |
|---|---|
| Position | oberste Zeile des scrollbaren Inhalts, über dem Markenblock |
| Nicht vorhanden | kein leerer Platz, keine Lücke, kein Verschieben des Inhalts |
| Bei offener Tastatur eingeblendet | „Anmelden" bleibt vollständig über dem IME |
| In der Aktionsleiste | nie — weder Text noch Fläche |
| Schließen | kein Schließen, kein Wiederholen, verschwindet mit dem nächsten erfolgreichen Schreibvorgang |

**Der eine benannte Preis.** Weil das Banner in den Inhalt einfügt und die Scrollposition in
Inhaltskoordinaten bleibt, verschiebt sich der sichtbare Inhalt beim Erscheinen um die
Bannerhöhe — auf einer langen, gescrollten Liste (Live) um rund 40 dp, ohne Geste. Bei einem
seltenen Ereignis ist das hinnehmbar; wer es weg haben will, gleicht die Scrollposition in dem
Frame aus, in dem das Banner von „aus" auf „an" geht (`vertical_scroll_offset(offset + h)`).
Das ist eine Zeile und keine Änderung der Entscheidung — deshalb steht sie hier als P2 und
nicht als Einwand.

**Das Banner scrollt mit dem Inhalt weg.** Wer im Formular nach unten scrollt, sieht es nicht
mehr. Auch das ist die Folge der Entscheidung und keine Nachlässigkeit: dauerhaft sichtbar
ginge nur mit einem festen Streifen, und der ist abgelehnt. Nicht später „reparieren", indem
man das Banner anheftet — dann ist der Preis zurück.

### 18.8 Abstände sind netto, nicht `add_space`

`touch_style` setzt `item_spacing = (8, 8)` (`mod.rs:127`). egui fügt diesen Abstand **nach
jedem Widget** in einem `horizontal` selbst ein. `add_space(GAP)` ergibt deshalb `GAP + 8`,
nicht `GAP`.

Belegt an drei Stellen, alle mit derselben Ursache:

| Symptom | Erwartet | Gemessen |
|---|---|---|
| Grid-Spaltenabstand | 12 dp (`GAP`) | **19,8 dp** |
| Hero-Abstand Plot → Chips | 14 dp | **22 dp** |
| Grid-Gesamtbreite | 379 dp | **410 dp** von 411 — rechte Spalte ohne Rand |

**Regel: jeder Abstand in dieser Spezifikation ist ein *netto* gemessener Abstand.** Die
Umsetzung setzt `item_spacing` in dem Container, der die Abstände trägt, auf **0** und
benutzt dann `add_space` mit dem Tokenwert. Das ist eine Zeile pro Container statt einer
Subtraktion an zehn Stellen — und es macht den Token wieder zu dem, was er behauptet.

Wer stattdessen `add_space(GAP - 8)` schreibt, hat den Fehler an jeder Aufrufstelle einzeln
und keine davon sagt, warum.

### 18.9 Eine linke Kante, auch für Boxen

Der Avatar in `list_row` saß bei `pad + 24` Mittelpunkt mit 44 dp Kasten, also **2 dp
innerhalb** der Inhaltskante. Das war kein Entwurf, sondern Rest einer Rechnung aus der Zeit
vor `pad`. Kastenkante gehört auf `pad` — bei einem Kreis im Kasten ist die linke Kante des
Kreises dann die Inhaltskante.

Gilt für jedes Element mit einer Box: die **Box** liegt auf der Kante, nicht ihr Mittelpunkt
und nicht ihr Innenabstand.

---

## 19. Player-Zustände (was der Nutzer sieht, wenn kein Video läuft)

Grundgeometrie unverändert nach §8.3: obere Leiste 56 dp, untere Transportleiste 120 dp.

**`video_band` ist feste Geometrie.** 16:9-Band, waagerecht über die volle Breite, senkrecht
mittig in der freien Fläche. Das Videobild wird **in** das Band gelegt und innen beschnitten
(Letterbox im eigenen Painter). Das Widget-Rechteck wird niemals kleiner. Sonst fällt der
zweite Tipp im schwarzen Balken auf ein Nichts, und die Leisten lassen sich dort weder ein-
noch ausblenden — der Fehler wäre unsichtbar und nur durch Tippen an der richtigen Stelle
zu finden.

**Grundsatz: die Transportleiste ist nie sichtbar, wenn sie nichts bewirken kann.**

### 19.1 Zustandsübersicht

| Zustand | Zurück | Play/Pause | Scrubber | ±10 s | Vollbild | Auto-Hide |
|---|---|---|---|---|---|---|
| Z0 Verbinden | ✓ | – | – | – | – | aus |
| Z1 Erster Frame | ✓ | ✓ | ✓ | ✓ | ✓ | startet mit Frame 1 |
| Z2 Puffern | ✓ | ✓ | ✓ | ✓ | ✓ | ausgesetzt |
| Z3 Nur Ton (live) | ✓ | ✓ | – | – | ✓ | 3 s |
| Z3 Nur Ton (Datei) | ✓ | ✓ | ✓ | ✓ | ✓ | 3 s |
| Z4 Fehlgeschlagen | ✓ | – | – | – | – | aus |
| Z5 Hintergrund | – | ✓ (pausiert) | ✓ | ✓ | ✓ | aus |
| Z6 Spulen | ✓ | ✓ | ✓ (zieht) | ✓ | ✓ | ausgesetzt |

Zwei Bedingungen, die über der Tabelle stehen und in **keinem** Zustand gebrochen werden
dürfen — sonst zeigt die Leiste einen Scrubber, der sich nicht bewegen lässt:

1. **Die Spalte „Scrubber" gilt nur für spulbare Inhalte.** Bei Live-Streams ist der Scrubber
   in **jedem** Zustand abwesend; an seiner Stelle steht das `● LIVE`-Badge nach §8.3.
   Z6 (Spulen) entfällt bei Live vollständig — es gibt kein Ziel.
2. **Der Scrubber erscheint erst, wenn die Dauer bekannt ist** (`duration > 0`). Bei einer
   VOD-Datei, deren Länge mpv noch nicht gemeldet hat, ist die Fortschrittszeile reserviert,
   aber leer — 28 dp Höhe bleiben stehen, damit die untere Leiste ihre 120 dp behält und das
   `video_band` nicht springt. Ein Scrubber ohne bekannte Dauer ist ein toter Regler.

### 19.2 Z0 — Verbinden, noch kein Frame

Ein Live-Stream braucht Sekunden. Der Nutzer darf nicht auf eine schwarze Fläche ohne
Rückmeldung sehen.

- `video_band`: `bg()`. Mittig 24 dp Spinner in `accent_text()`.
- 12 dp darunter eine Zeile `FS_LABEL` 13 pt in `muted()`, `fit` auf 240 dp: „Verbinde …".
- Chrome: **nur die obere Leiste** (Zurück + Titel). **Keine Transportleiste** — es gibt
  nichts zu steuern, und ein toter Scrubber ist schlechter als ein Spinner. Abbrechen geht
  über Zurück, das immer sichtbar ist.
- Auto-Hide aus.

### 19.3 Z1 — Der erste Frame, die erste Sekunde

Das Video ist schwarz, die Leisten sind schon gezeichnet. Frage: was tun sie?

- Chrome erscheint **sofort voll deckend, ohne Einblendung**. Ein 300-ms-Fade über einer
  schwarzen Fläche liest sich als Fehler, nicht als Eleganz.
- Der Auto-Hide-Timer startet **erst mit dem ersten präsentierten Frame**. Damit bekommt der
  Nutzer volle 3 s Leisten ab dem Moment, in dem es etwas zu steuern gibt — nicht 3 s, von
  denen 2 auf einen schwarzen Bildschirm entfallen.
- Ausblenden: 200 ms Opazität, danach nimmt `video_band` die volle Fläche ein.

### 19.4 Z2 — Puffern während der Wiedergabe

- Das **letzte Frame bleibt stehen** (nicht schwarz — es zeigt, wo man war).
- Darüber ein Scrim `from_black_alpha(102)` (40 %) **nur über `video_band`**, nicht über die
  Leisten.
- Mittig 32 dp Spinner.
- „Puffert …" `FS_LABEL` 13 pt, `muted()`, erst ab **800 ms** Stillstand — ein 300-ms-Hänger
  darf kein Label aufblitzen lassen.
- Chrome bleibt sichtbar und **voll bedienbar**. Suchen ist die eigentliche Abhilfe gegen
  einen Hänger, deshalb bleibt der Scrubber aktiv und zeigt den gepufferten Bereich.
- Auto-Hide ausgesetzt, solange gepuffert wird.

### 19.5 Z3 — Nur Ton (Radio)

`video_band` behält seine 16:9-Geometrie, damit die Tippzonen stabil bleiben — es wird nicht
auf die freie Fläche ausgedehnt.

- Mittig ein Kreis 64 dp in `accent_soft()`, darin eine 28 dp Lautsprecher-Glyphe in
  `accent_text()`.
- 12 dp darunter der Sender- bzw. Titelname in `FS_HEAD` 17 pt, `text()`, `fit` auf 280 dp.
- 4 dp darunter „Nur Ton" in `FS_LABEL` 13 pt, `muted()`.
- Transport: bei **nicht spulbaren** Streams (Live-Radio) entfällt die Fortschrittszeile,
  die untere Leiste behält aber ihre 120 dp Höhe, damit nichts springt. Bei einer Audiodatei
  mit bekannter Dauer bleibt der Scrubber.
- **Regel: die Fortschrittszeile richtet sich nach der Spulbarkeit, nicht danach, ob eine
  Videospur existiert.**

### 19.6 Z4 — Wiedergabe fehlgeschlagen

- `video_band`: `bg()`. Inhalt mittig, von oben: 48 dp Warndreieck in `danger()`;
  16 dp Abstand; „Wiedergabe fehlgeschlagen" in `FS_HEAD` 17 pt, `text()`;
  8 dp; die Ursache in `FS_SUB` 15 pt, `muted()`, `wrap` mit **2 Zeilen**, max. 300 dp,
  zentriert.
- **Ursachen werden übersetzt, nicht durchgereicht.** Vier Fälle mit eigener Formulierung:

  | Fall | Text |
  |---|---|
  | Netzwerk | „Keine Verbindung zum Anbieter." |
  | Anbieter lehnt ab | „Der Anbieter hat die Wiedergabe abgelehnt." |
  | Format | „Dieses Format kann nicht wiedergegeben werden." |
  | Zeitüberschreitung | „Der Anbieter antwortet nicht." |

  Der rohe mpv-Text erscheint ausschließlich hinter „Details".
- Aktionen: Primärbutton „Erneut versuchen", Breite = `video_band` − 2 × `PAD`, Höhe 48 dp;
  8 dp darunter ein Textbutton „Details" in `FS_ROW` 16 pt, `accent_text()`.
- Chrome: **obere Leiste bleibt** (Zurück + Titel). **Transportleiste vollständig entfernt** —
  es gibt nichts zu steuern, und eine tote Leiste über einer Fehlermeldung nimmt der Meldung
  den Platz weg.
- Auto-Hide aus.

### 19.7 Z5 — App im Hintergrund, Bildschirm aus

- `onPause`: Wiedergabe **pausieren**. Kein Weiterspielen im Hintergrund — ohne
  Foreground-Service beendet Android den Prozess ohnehin, und ein laufender IPTV-Stream
  kostet Datenvolumen und einen Provider-Slot.
- `onResume`: pausierter Player, Chrome sichtbar, letztes Frame steht, Scrubber auf Position.
  **Kein automatisches Fortsetzen** — der Nutzer entscheidet.
- Kein Spinner beim Zurückkehren: die Textur überlebt den Wechsel.
- Bewusstes Nicht-Ziel: Audio im Hintergrund mit Media-Notification (braucht einen
  Foreground-Service), siehe §12.

### 19.8 Z6 — Spulen

- Während des Ziehens: Knopf folgt dem Finger, beide Zeitlabels aktualisieren live, das Bild
  bleibt stehen.
- Nach dem Loslassen, Ziel nicht gepuffert: direkt **Z2**, nicht zurück auf Z0. Ein
  aufblitzendes „Verbinde …" nach jedem Sprung ist ein Rückschritt.

### 19.9 Was in jedem Zustand gilt

- Die obere Leiste ist in jedem Zustand sichtbar außer Z5. Zurück ist immer erreichbar.
- Jeder Zustandswechsel zwischen Z0, Z2 und Z4 dauert 150 ms (Opazität des Inhalts in
  `video_band`), damit ein schneller Wechsel nicht flackert.
- Kein Zustand zeigt zwei Spinner gleichzeitig.
- Der Fehlerzustand ist der einzige, in dem ein Button im `video_band` liegt; in allen
  anderen bleibt das Band eine reine Fläche.

---

## 20. Player: Delta gegen §8.3

§8.3 bleibt als Vorgabe des Nutzers unverändert stehen. **Wo §20 und §8.3 sich
widersprechen, gilt §20.** Drei Punkte, mehr nicht.

### 20.1 Einzel- und Doppeltipp

§8.3 verlangt: „Tippen auf das Video toggelt die Leisten. Doppeltipp links/rechts = ±10 s."

Frühere Fassungen dieser Spezifikation haben das für nicht umsetzbar erklärt, weil `clicked()`
bereits beim ersten Tipp feuert. **Das war falsch.** Richtig ist: die Einzelaktion muss
*verzögert* werden, das Gestell selbst ist nicht unmöglich.

- Bei `button_pressed`: Position merken, `request_repaint_after(250 ms)` anfordern, **nichts
  ausführen**.
- Kommt `button_double_clicked(PointerButton::Primary)` vor Ablauf: Einzelaktion verwerfen,
  um ∓10 s springen (Zone, siehe unten).
- Läuft die Zeit ab: Einzelaktion ausführen (Leisten umschalten).

**Zonen für den Doppeltipp:** links 40 % ∓10 s, rechts 40 % ±10 s, Mitte 20 % ohne
Spulaktion. Ein Doppeltipp in der Mitte fällt auf die Einzelaktion zurück und schaltet die
Leisten einmal um. Die Mitte darf so schmal sein, weil das Umschalten über den Einzelttipp
**überall** verfügbar ist — sie muss keine eigene Fläche mehr tragen.

**Eine Asymmetrie, weil sie gratis ist:** Ist die Leiste **ausgeblendet**, wird sie beim Tipp
sofort gezeigt, ohne die 250 ms. Das ist die dringende Reaktion, und ein Doppeltipp auf einen
nackten Bildschirm ist selten. Ist sie **eingeblendet**, wartet das Ausblenden die 250 ms ab.
Das ist die einzige Stelle, an der die Verzögerung spürbar wäre.

**Repaint-Budget:** `request_repaint_after(250 ms)` nur, solange eine Einzelaktion aussteht.
Danach keine geplanten Repaints.

### 20.2 Einheiten: die Zahlen aus §8.3 sind Punkte, nicht Pixel

Die Werte in §8.3 (58, 56, 120, 76) wurden in egui-Punkten für den Desktop-Build geschrieben.
Bei 100 % Skalierung ist ein Punkt ein Pixel, also ist „58 px" dort **58 dp gemeint**. Die
Zahl bleibt, die Einheit wird korrigiert: **58 dp Spielkreis, 56 dp obere Leiste, 120 dp
untere Leiste, 76 dp Abstand der Spultasten.**

Damit niemand es erneut umrechnet: 58 dp sind auf diesem Gerät 152 physische Pixel. Wer 58 px
durch 2,6254 teilt, bekommt 22 dp und einen Spielkreis von einem Drittel der Größe.

Transportzeile bei 411 dp (Inhalt 379 dp):

| Element | Größe | Lage |
|---|---|---|
| Spielkreis | 58 dp | Mitte bei x = 205,5 → 176,5…234,5 |
| Spultaste links (∓10 s) | 76 × 48 dp | Mitte 129,5 → 91,5…167,5 |
| Spultaste rechts (±10 s) | 76 × 48 dp | Mitte 281,5 → 243,5…319,5 |
| Vollbild | 48 × 48 dp | rechts bei 395 → 347…395 |
| linke Restfläche | — | 16…91,5 bleibt **leer**, nicht füllen |

Unterhalb von 360 dp Inhaltsbreite dürfen die **Spultasten** auf 64 dp schrumpfen. Der
**Spielkreis nie** — er ist der Anker der Komposition.

**Fortschrittszeile:** sichtbarer Balken 4 dp hoch, Trefferfläche **48 dp** hoch, mittig auf
dem Balken und in die Abstände darüber und darunter hineinragend. Zeiten links und rechts in
`FS_LABEL` 13 pt, 12 dp Abstand zum Balken.

### 20.3 Lautstärkezeile: auf dem Handy entfernt

§8.3 nennt eine eigene Lautstärkezeile mit Icon und Balken. Das ist eine Desktop-Idiomatik.

- **Handy:** keine Lautstärkezeile. Hardwaretasten erledigen das, und die untere Leiste hat
  keinen Platz für eine dritte Zeile.
- **TV:** bleibt wie in §8.3.

Bewusste Plattform-Abweichung, kein fehlendes Feature.

---

## 21. Navigation und destruktive Aktionen

### 21.1 Ziele und Reihenfolge

`Live | Filme | Serien | Fortsetzen | Mehr`

| Ziel | Icon | Label | Begründung |
|---|---|---|---|
| Live | `Icon::Tv` | „Live" | Inhaltsäule |
| Filme | `Icon::Film` | „Filme" | Inhaltsäule |
| Serien | `Icon::Series` | „Serien" | Inhaltsäule |
| **Fortsetzen** | `Icon::Resume` (neu) | „Fortsetzen" | häufigste Rückkehraktion |
| Mehr | `Icon::More` | „Mehr" | Overlay |

- **`Icon::Resume`:** gefülltes Spiel-Dreieck 10 dp, darunter 3 dp Abstand eine gerundete
  Leiste 16 × 2 dp, gleiche Farbe. Liest sich als „weiterspielen", nicht als „neu starten",
  und ist von `Icon::Play` (nacktes Dreieck) und `Icon::Film` unterscheidbar.
- Längstes Label wächst von „Favoriten" (9) auf „Fortsetzen" (10) Zeichen — bei `FS_CAP`
  11,5 pt und 82,2 dp je Reiter unkritisch.
- **Favoriten** wandert in das Mehr-Sheet. Der Stern bleibt als Icon erhalten.

Begründung der Reihenfolge: die drei Inhaltsäulen bleiben, der vierte Platz gehört der
Aktion, für die die App geöffnet wird. Favoriten ist eine kuratierte Liste, die man bewusst
aufsucht und die einen zusätzlichen Tipp verträgt; Fortsetzen ist der Grund, die App
überhaupt zu öffnen.

### 21.2 Mehr-Sheet

| Position | Eintrag | Icon |
|---|---|---|
| 1 | Favoriten | `Icon::Star` |
| 2 | Suche | `Icon::Search` |
| 3 | Einstellungen | `Icon::Sliders` |
| 4 | Abmelden | keins, `danger()` |

- Reihenfolge-Prinzip: **Ziele → Werkzeuge → Konfiguration → Destruktives.** Neue Einträge
  finden ihre Position nach dieser Regel, nicht nach Alter.
- Favoriten steht oben, weil es der Eintrag ist, der vorher ein Reiter war — wer ihn sucht,
  schaut zuerst nach oben.
- 1 px `hairline()` über „Abmelden".
- **Die Versionszeile entfällt im Sheet.** Sie steht nur auf dem Einstellungs-Bildschirm.

### 21.3 Wann bestätigt wird

**Regel: bestätigt wird nur, was sich nicht mit einem Tipp rückgängig machen lässt.**

Das ist am Code geprüft, nicht angenommen:

| Aktion | Wirkung | Rückgängig? | Dialog? |
|---|---|---|---|
| `App::logout()` (`app.rs:581`) | `active_profile = None`, `library` geleert, Ansicht → Login | **ja** — `profiles` und `profiles.json` bleiben unverändert, der Login-Bildschirm bietet das Profil als 64-dp-Zeile unter „Gespeicherte Profile" an; Favoriten, Fortschritt, Verlauf, Einstellungen bleiben | **nein** |
| `App::delete_profile(id)` (`app.rs:545`) | Eintrag wird aus `profiles.json` entfernt | nein | **ja** |

**„Abmelden" bekommt deshalb keinen Bestätigungsdialog.** Es geht nichts verloren; die
Sitzung endet, und ein Tipp auf dem Login-Bildschirm holt sie zurück. Ein Dialog auf einer
verlustfreien Aktion erzieht den Nutzer dazu, Dialoge wegzutippen — und entwertet damit den
einen Dialog, der zählt.

**Diese Begründung hat eine Vorbedingung: dass `profiles.json` geschrieben wird.** Solange
die Persistenz defekt ist (§22), gibt es nach einem Neustart keine gespeicherten Profile, und
der Login-Bildschirm kann nichts zurückgeben — dann *ist* „Abmelden" verlustbehaftet, ohne
dass sich der Code geändert hätte.

Die Antwort darauf ist **nicht** ein Dialog. Ein Dialog wäre die Institutionalisierung des
Fehlers: er würde dauerhaft eine Warnung vor etwas zeigen, das nur wegen eines Schreibfehlers
passiert. Die Antwort ist, §22 zu beheben. Bis dahin gilt die Regel oben als Absicht, nicht
als Beschreibung des Ist-Zustands.

Soll „Abmelden" künftig wirklich die Zugangsdaten löschen, muss es `delete_profile` aufrufen.
Dann ist der Dialog berechtigt und der Text unten gilt wörtlich.

### 21.4 Bestätigungsdialog (für `delete_profile`)

**Der Dialog gehört an `login.rs:247`, nicht an `Abmelden`.** Dort steht die einzige
unwiderrufliche Aktion der App: ein 48 dp großer, unsichtbarer Löschtreffer auf der rechten
Seite jeder Profilzeile, gekennzeichnet durch ein 20 dp `Icon::Close` in `muted()`, ohne
Rückfrage. Ein Tipp löscht Zugangsdaten aus `profiles.json`.

- Fläche: `min(content_w, 320)` breit, zentriert; bei 411 dp also 320 dp.
- Karte `card()`, Radius `RADIUS` 16 dp, 1 px `hairline()`, Innenabstand **24 dp**.
- Scrim `from_black_alpha(150)` über dem ganzen Bildschirm.
- **Titel: „Profil löschen?"** — `FS_HEAD` 17 pt `bold`, `text()`. 8 dp Abstand.
- Fließtext `FS_SUB` 15 pt `muted()`, `wrap` mit max. 3 Zeilen, volle Kartenbreite. 20 dp.
- Zwei Buttons, beide volle Kartenbreite, je 48 dp hoch:

| Position | Text | Stil |
|---|---|---|
| oben | **„Löschen"** | Textbutton, `danger()`, `FS_ROW` 16 pt |
| unten | **„Abbrechen"** | gefüllt, `accent_fill()`, weiße Schrift |

- Der Button heißt „Löschen", nicht „Profil löschen": der Titel benennt den Gegenstand
  bereits, und ein Button, der den Titel wiederholt, wird als zweite Überschrift gelesen
  statt als Aktion.
- **Abbrechen ist die größere und leichtere Fläche** und liegt unten, wo der Daumen sitzt.
  Die destruktive Aktion ist nie die Standardaktion.
- Abweichung von Material (dort zwei Textbuttons): bei einer destruktiven Aktion auf dem
  Handy senkt ein explizit gefüllter Standard die Fehlbedienung stärker, als Symmetrie
  gewinnt.
- Der Dialog öffnet mit Fokus auf **Abbrechen**, nicht auf der destruktiven Aktion.
- Schließen: Tipp auf den Scrim, Zurück-Geste, beide = Abbrechen.

**Fließtext, wörtlich:**

> Zugangsdaten für „{name}“ werden von diesem Gerät entfernt.
> Favoriten und Fortschritt bleiben erhalten.

**Anführungszeichen:** deutsches Paar `„…“` (U+201E / U+201C), kein ASCII. Das ist die
Konvention der übrigen Strings im Projekt (`settings.rs:64`, `:93`, `:244`) und nicht
Geschmack — ein ASCII-Zeichen nach einem `„` liest sich als Tippfehler, weil die öffnende
Hälfte typografisch ist und die schließende nicht. Die frühere Fassung dieses Abschnitts
hatte hier ASCII stehen; das war ein Fehler im Dokument, nicht in der Umsetzung.

**Der Name ist unbegrenzt — der Satz ist es nicht.** `body_1` wird mit `wrap(…, 3)` gesetzt
(`login.rs:441`). Die feste Umgebung des Namens sind 51 Zeichen; bei `inner_w` 272 dp und
`FS_SUB` 15 pt passen rund 36 Zeichen je Zeile, also **~108 Zeichen in drei Zeilen**. Ab
einem Namen von **etwa 57 Zeichen** schneidet die Zeilenbegrenzung deshalb nicht den Namen,
sondern die Anweisung: „… werden von diesem Gerät entfernt." fällt weg und wird durch „…"
ersetzt, im schlechtesten Fall zusammen mit dem schließenden `“`. Der Satz endet dann offen
und ohne Prädikat — und sieht aus wie ein zu langer Name.

Profilnamen sind nirgends längenbegrenzt (kein `len`/`truncate` in `login.rs`), und
Anbieternamen wie `benutzer@example.invalid` sind realistisch länger als 57 Zeichen.

**Regel: den variablen Teil begrenzen, nicht den Satz.** Der Name wird vor dem Formatieren
auf 20 Zeichen gekürzt, mit `…`:

```rust
let short: String = if name.chars().count() > 20 {
    let mut s: String = name.chars().take(19).collect();
    s.push('…');
    s
} else {
    name.to_string()
};
```

Damit ist der längste Fall 17 + 20 + 34 = 71 Zeichen, also zwei Zeilen. Die Grenze von 3
Zeilen bleibt als Fangnetz stehen, greift aber nie — was richtig ist: eine Begrenzung, die
im Normalfall nicht auslöst, ist eine Versicherung, keine Formatierung. `chars().count()`
statt `len()`, weil ein Name mit Umlauten sonst mitten im Zeichen gekürzt würde.

Der Satz nennt **was bleibt**, nicht nur was geht. Das ist der Unterschied zum abgelehnten
Abmelde-Dialog und der Grund, warum dieser Text funktioniert: die häufigste Angst bei
„löschen" ist der Verlust der eigenen Mediathek, nicht der Zugangsdaten. Wer weiß, dass
Favoriten und Fortschritt bleiben, trifft die Entscheidung anhand des tatsächlichen
Verlusts — zwei Zeilen Anmeldedaten — und nicht anhand einer Befürchtung.

Sachlich geprüft: `delete_profile` ruft `profiles.retain` + `save_profiles` auf
(`app.rs:545-548`) und berührt `favorites.json`, `progress.json`, `history.json` und
`settings.json` nicht. Der Satz ist korrekt, nicht nur beruhigend.

### 21.5 Profilzeile im Login: zwei Aktionen in einer Zeile

Ist-Zustand (`login.rs:234-249`): die Zeile ist „mit diesem Profil anmelden", und rechts
darin liegt ein 48 × 48 dp großer Löschtreffer, der Vorrang vor dem Zeilentipp hat.

Das ist strukturell falsch, unabhängig vom Dialog: **eine Zeile trägt genau eine Aktion.**
Der Löschtreffer liegt in der rechten 52 dp der Zeile — genau dort, wo der Daumen beim
Auswählen und Scrollen landet.

Minimalfassung (wenn kein Umbau gewünscht ist):

- **Neues `Icon::Trash`** statt `Icon::Close`, in `danger()`. Ein × heißt „ausblenden", ein
  Papierkorb heißt „endgültig entfernen" — und nur das zweite ist hier wahr.
- Der Dialog nach §21.4 vor jedem `delete_profile`-Aufruf.
- Treffer bleibt 48 dp, rückt aber auf die **äußere** Kante der Zeile (Mitte bei
  `rect.right() - 24`), damit er nicht mitten in der Auswahlfläche sitzt.

### 21.6 `Icon::Trash` (neu)

Gezeichnet im selben Vokabular wie der übrige Satz: Einheitsrechteck `r`, normierte
Koordinaten, `line(poly(&[...]))` mit dem gemeinsamen `Stroke s`.

| Teil | Punkte |
|---|---|
| Deckel | (0.16, 0.26) → (0.84, 0.26) |
| Griff | (0.38, 0.26) → (0.41, 0.15) → (0.59, 0.15) → (0.62, 0.26) |
| Korpus | (0.26, 0.26) → (0.31, 0.85) → (0.69, 0.85) → (0.74, 0.26) |
| Rippe (nur ab 24 dp) | (0.50, 0.40) → (0.50, 0.72) |

- Bei **20 dp ohne Rippe**: drei Striche, eindeutig lesbar. Mit Rippe wird der Korpus bei
  20 dp zu dicht — dieselbe Dichte, an der die Schieberegler-Glyphe im Mehr-Sheet kippt.
- Ab **24 dp mit Rippe**, dann liest der Korpus als Behälter und nicht als Becher.
- Farbe am Aufrufort: `danger()`. Größe 20 dp in einem 48 dp Treffer (§21.5).

### 21.7 Der Löschtreffer ist der einzige seiner Art

**In der gesamten App liegt genau eine destruktive Aktion innerhalb der Trefferfläche eines
anderen Steuerelements: dieser Papierkorb in der Profilzeile.** Das ist ein Altlast-Befund,
kein Muster.

Wer eine zweite destruktive Aktion hinzufügt, gibt ihr eine eigene Fläche — nie eine, die
innerhalb einer Zeile, Karte oder Schaltfläche liegt, die etwas anderes tut. Zwei Aktionen in
einem Rechteck sind ein Fehler, auch wenn beide einen Dialog haben.

Besser, und erstrebenswert sobald Platz ist: der Löschtreffer verschwindet aus der Zeile und
die Profilverwaltung bekommt eine eigene Zeile „Profile verwalten" unter der Liste. Dann hat
jede Zeile wieder genau eine Aktion, und Löschen ist zwei Tipps entfernt statt einen.

**Abnahmekriterien der Profilzeile** (vor der ersten Geräteaufnahme festgelegt, damit sie
nicht nachträglich an das Bild angepasst werden):

| Prüfung | Erwartet |
|---|---|
| Zeilenhöhe | 64 dp |
| Avatar | 44 dp, **Kastenkante** auf `pad` (linkester Kreis-Pixel bei 16 dp) |
| Avatar-Inhalt | erster Buchstabe des Namens, groß, `bold(18)`, `accent_text()` auf `accent_soft()` |
| Titel | `FS_ROW` 16 pt, 1 Zeile, `fit` — endet **vor** dem Papierkorb, läuft nicht darunter |
| Zweite Zeile | `FS_LABEL` 13 pt, `muted()`, „Xtream" bzw. „M3U" |
| Papierkorb | 20 dp, `danger()`, Treffer 48 dp, Mitte bei `rect.right() - 24` |
| Tipp auf Zeilenkörper | startet den Login |
| Tipp auf Papierkorb | öffnet den Dialog — **löscht nicht** |
| 3 Profile | 3 Zeilen, kein Scrollen, nichts am unteren Rand abgeschnitten |
| 0 Profile | Überschrift „Gespeicherte Profile" fehlt **ganz**, kein leerer Abschnitt |

Der Titel-Check ist der, der in der Aufnahme am ehesten kippt: `fit` bekommt seine Breite aus
`right - 48 - pad`, und wenn der Papierkorb nicht in diese Rechnung eingeht, läuft ein langer
Profilname unter das Symbol.

### 21.8 Zurück-Geste: eine Reihenfolge, nicht ein Sonderfall

Die Zurück-Geste ist derzeit auf jedem Bildschirm ohne Wirkung, einschließlich des Dialogs,
wo sie abbrechen muss statt die App zu schließen. Das ist kein Layout-Problem, sondern eine
fehlende Reihenfolge — und ohne eine solche wird jeder einzelne Fall einzeln geflickt.

**Die Geste verbraucht genau eine Ebene, von oben nach unten. Erster Treffer gewinnt:**

| # | Zustand | Wirkung |
|---|---|---|
| 1 | Tastatur offen (`l.ime > 0`) | Tastatur schließen — das System erledigt das, nicht abfangen |
| 2 | Modale Ebene (Dialog, Sheet) | nur die modale Ebene schließen, sonst nichts |
| 3 | Player offen | Player verlassen, zurück auf die Seite, von der er gestartet wurde |
| 4 | Unterseite (Seriendetail, Suche, Einstellungen, Mehr) | eine Ebene nach oben |
| 5 | Nicht der Standard-Tab | zum Standard-Tab wechseln |
| 6 | Wurzel erreicht | **nicht abfangen** — das System beendet die App |

Zu 6: ein „Zum Beenden erneut drücken" ist kein Schutz, sondern eine Verzögerung, und es
bricht die Predictive-Back-Animation ab API 33. Auf der Wurzel gehört die Geste dem System.

Zu 2: der Dialog ist der Fall, der jetzt auffällt, aber die Regel ist allgemein — eine modale
Ebene zu schließen darf nie gleichzeitig eine Seite poppen. Sonst schließt ein Tastendruck
zwei Dinge, und der Nutzer landet eine Ebene tiefer, als er wollte.

Zu 3: „zurück, wo man herkam" heißt mit erhaltenem Tab *und* Scrollposition. Ein Player, der
immer auf `Live` zurückfällt, verliert bei jedem Abbrechen den Ort in einer 2000-Zeiler-Liste.

---

## 22. Persistenz: stille Schreibfehler sind Datenverlust

Befund: `app_dir()` fällt auf `.` zurück, das Arbeitsverzeichnis der App ist `/`,
`write("/profiles.json")` scheitert mit EACCES, und der Fehler wird verschluckt
(`let _ = store::save_*`). Auf dem Gerät wird **nichts** gespeichert: Profile, Favoriten,
Fortschritt, Verlauf und Einstellungen sind nach jedem Neustart weg.

Das ist kein Layout-Fehler, hat aber drei Entwurfsfolgen.

### 22.1 Ein verschluckter Schreibfehler ist schlimmer als eine Fehlermeldung

Der Nutzer legt ein Profil an, die App bestätigt, und beim nächsten Start ist es weg. Aus
seiner Sicht hat die App gelogen. Eine sichtbare Meldung ist unangenehmer und ehrlicher.

**Regel: kein `let _ =` bei einem Schreibvorgang, dessen Ergebnis der Nutzer erwartet.**

### 22.2 Die Meldung hängt an einem fehlgeschlagenen Schreibvorgang, nicht an einer Annahme

Nicht „dieses Gerät kann nicht speichern" als Dauerzustand, sondern ein Ereignis.

- **Mechanik:** ein `AtomicBool` in `store.rs`, gesetzt von **jedem** fehlgeschlagenen
  Schreibvorgang, gelöscht von jedem erfolgreichen. Der Balken liest das Flag.
- **Ort des Flags:** in `write_json` (`store.rs:54`), **nicht** an den fünf Aufrufstellen
  `save_profiles`, `save_settings`, `save_favorites`, `save_progress`, `save_history`. Alle
  fünf gehen bereits durch `write_json`, das Flag erbt sich also von selbst — und eine neue
  Schreibfunktion erbt es mit. Dieselbe Begründung wie in §18.8: eine Stelle statt fünf.
- **Der Balken erscheint genau dann, wenn das Flag gesetzt ist**, und verschwindet von
  selbst, sobald ein Schreibvorgang gelingt. Kein Schließen-Knopf: eine wegtippbare Warnung
  vor Datenverlust wird weggetippt, und danach ist der Verlust wieder stumm. Selbstheilend
  statt bestätigungspflichtig.
- **Der Balken liest das Flag beim Zeichnen und sonst nichts.** Kein Timer, kein
  `request_repaint_after`, keine „erneute Prüfung". Polling ändert den Zustand nicht, es fragt
  ihn nur öfter ab — und der einzige Weg, das Flag zu löschen, ist ein erfolgreicher
  Schreibvorgang, der ohnehin einen Repaint auslöst. Ein Poll wäre reine Rechenzeit.
- **Folge, die bewusst in Kauf genommen wird:** tut der Nutzer nichts, bleibt ein einmal
  gesetztes Flag stehen und der Balken mit ihm. Das ist richtig — die Bedingung, die er
  beschreibt, besteht weiter. Ein Banner, das von allein verschwindet, ohne dass etwas
  gespeichert wurde, würde lügen.

**Text — ereignisbezogen, nicht gerätebezogen:**

> Die letzte Änderung konnte nicht gespeichert werden. Sie gilt nur bis zum Beenden der App.

Der Unterschied ist nicht kosmetisch. „Können auf diesem Gerät nicht gespeichert werden" ist
eine Behauptung über das Gerät, die ein einzelner fehlgeschlagener Schreibvorgang nicht
belegt — und sie bleibt stehen, wenn der nächste Schreibvorgang gelingt. Der ereignisbezogene
Satz ist wahr, wenn er erscheint, und verschwindet mit dem Zustand, den er beschreibt.

Im aktuellen Defekt (jeder Schreibvorgang scheitert) verhält sich der ereignisbezogene Balken
wie ein dauerhafter, weil ihn nie ein Erfolg löscht. Genau das ist gewollt: er verhält sich
richtig, ohne den Defekt zu kennen.

- Ein Hinweisbalken, kein Modal-Dialog. Ein Dialog bei jedem Start für einen Zustand, den der
  Nutzer nicht beheben kann, wird weggetippt und lehrt nichts.
- Ort: Einstellungs-Bildschirm und Login-Bildschirm, oberhalb des Inhalts.
- `FS_SUB` 15 pt, `text()` auf `accent_soft()`, 12 dp Innenabstand, Radius `RADIUS_SM`.
- Kein Wiederholen-Knopf: ein Rechtefehler wird durch Wiederholen nicht besser. Der nächste
  reguläre Schreibvorgang ist der Wiederholungsversuch.

### 22.3 Was ohne Persistenz nicht funktioniert

| Funktion | Zustand ohne Persistenz |
|---|---|
| Profile | nach jedem Start neu eingeben |
| Favoriten | leer |
| Fortschritt / „Fortsetzen" | leer |
| Verlauf | leer |
| Einstellungen | auf Standard zurückgesetzt |

**„Fortsetzen" ist der Reiter, den §21.1 an die vierte Stelle der Navigation geholt hat.** Er
ist ohne Persistenz dauerhaft leer. Die Informationsarchitektur ist damit richtig und
wirkungslos zugleich — die Priorität liegt bei §22, nicht bei der Navigation.

### 22.4 Lehre aus dem Fix-Versuch: `XDG_CONFIG_HOME` allein tut nichts

Der erste Fix-Vorschlag war, `XDG_CONFIG_HOME` aus dem Java-`filesDir` zu setzen. Das hätte
**nichts** geändert, ohne Fehler und ohne Logzeile. Nachgeprüft in der Crate selbst
(`directories-5.0.1/src/lin.rs:11-13`):

```rust
pub fn base_dirs() -> Option<BaseDirs> {
    if let Some(home_dir) = dirs_sys::home_dir() {
        let cache_dir  = env::var_os("XDG_CACHE_HOME")...
        let config_dir = env::var_os("XDG_CONFIG_HOME")...   // nur INNERHALB
```

`XDG_CONFIG_HOME` wird ausschließlich innerhalb des `if let Some(home_dir)` gelesen. Android
setzt `HOME` nicht, `home_dir()` liefert `None`, der Block läuft nie, `ProjectDirs::from`
liefert `None` — unabhängig von `XDG_CONFIG_HOME`.

**Richtig: `HOME` *und* `XDG_CONFIG_HOME` beide auf `filesDir` setzen, am Anfang von
`android_main`, bevor irgendein Pfad gelesen wird.**

Die ganze Kette, weil jede Verknüpfung für sich harmlos aussieht:

| Schritt | Ergebnis | Laut? |
|---|---|---|
| `HOME` nicht gesetzt | `home_dir()` → `None` | still |
| `base_dirs()` | `None` | still |
| `ProjectDirs::from` | `None` | still |
| `app_dir()` | Rückfall auf `.` | **still — und sieht aus wie ein Pfad** |
| CWD der App ist `/` | `write("/profiles.json")` → EACCES | still, `let _ =` |

Fünf Verknüpfungen, vier davon stumm. Der Rückfall auf `.` ist die schlimmste: `None` wäre
eine ehrliche Antwort, `"."` ist ein Pfad, der gültig aussieht.

**Daraus zwei Regeln:**

1. **Ein fehlendes Verzeichnis ist ein Fehler, kein Standardwert.** `unwrap_or(".")` macht aus
   „unbekannt" ein „hier" und verschiebt den Fehler an eine Stelle, an der niemand mehr nach
   dem Verzeichnis sucht.
2. **„Leer" und „nicht lesbar" dürfen nie gleich aussehen.** Weil `app_dir()` auf `.` zeigt,
   rendert ein nicht lesbarer Speicher wie ein leerer: leeres Formular statt Meldung. Deshalb
   ist das Flag aus §22.2 nicht optional — ohne es ist der Unterschied zwischen „keine Profile
   vorhanden" und „Profile nicht ladbar" im UI nicht vorhanden.

Das ist der Grund, warum „wir haben die Variable gesetzt, die in der Doku stand, und es ging
immer noch nicht" hier als Beispiel steht und nicht als Prinzip: der Fehler war nicht das
Setzen, sondern das Fehlen einer Rückmeldung, dass es nicht gewirkt hat.

### 22.5 Der tatsächliche Pfad — und warum man ihn nicht raten kann

`app_dir()` ruft `ProjectDirs::from("com", "Tarik", "Dado-Player")`. Wer daraus den Pfad
ableitet, liegt falsch, und zwar an zwei Stellen:

1. **`qualifier` und `organization` werden auf Linux ignoriert.** Die Signatur in
   `lin.rs:92` lautet wörtlich `fn project_dirs_from(_qualifier: &str, _organization: &str,
   application: &str)` — die ersten beiden Parameter sind mit Unterstrich markiert, weil sie
   nicht gelesen werden. Auf macOS ergäbe `"com"`/`"Tarik"` einen Pfadbestandteil, auf Android
   nicht.
2. **Nur der Anwendungsname zählt, kleingeschrieben, ohne Leerzeichen**
   (`trim_and_lowercase_then_replace_spaces`). `"Dado-Player"` → `dado-player`.

Mit `XDG_CONFIG_HOME = filesDir` ist der reale Ordner also
**`/data/data/com.tarik.dadoplayer/files/dado-player`** — `applicationId` geprüft in
`android/app/build.gradle:11`.

### 22.6 Test des Selbstheilens (der einzige, der die Löschung prüft)

Auf einem gesunden Gerät lässt sich nur zeigen, dass das Flag **nie gesetzt** wird. Dass es
**gelöscht** wird, ist die andere Hälfte und die, die ein latch-artiger Fehler still bricht.
Dafür braucht es einen erzwungenen Fehler und eine Erholung:

```
adb shell run-as com.tarik.dadoplayer chmod 000 files/dado-player   # nächster Schreibvorgang scheitert
adb shell run-as com.tarik.dadoplayer chmod 755 files/dado-player   # nächster Schreibvorgang gelingt
```

Drei Bedingungen, ohne die der Test nichts beweist:

- **`run-as` ist nötig**, `/data/data/<pkg>/files` ist aus einer gewöhnlichen `adb shell`
  nicht beschreibbar.
- **Der Unterordner ist das Ziel, nicht `files`.** Die Rechte, die den Schreibvorgang regeln,
  sind die von `files/dado-player`; ein `chmod` auf `files` ändert daran nichts.
- **Zwischen den beiden Schritten muss ein Schreibvorgang stattfinden.** Der billigste ist ein
  Umschalten in den Einstellungen, weil dort beim Ändern gespeichert wird. Rechte ändern und
  einen Screenshot machen beweist nichts: das Flag bewegt sich nur, wenn geschrieben wird.

### 22.7 Das Muster: stumm akzeptiert, plausibel ausgegeben

Vier Fehler in diesem Projekt hatten dieselbe Form. Nicht derselbe Ort, nicht dieselbe
Ursache — dieselbe Form:

| Fall | Was akzeptiert wurde | Was tatsächlich passierte |
|---|---|---|
| Zeichenbegrenzung | `chars().take(180)` | begrenzte Zeichen, nicht Zeilen — sah aus wie eine Zeilenbegrenzung |
| Seiten-Inset | `systemGestures.left/right` | ist eine Berührungszone, kein reservierter Platz — 29,7 dp Phantomrand |
| Speicherort | `XDG_CONFIG_HOME` | wird ohne `HOME` nie gelesen (`lin.rs:11`) — Pfad blieb `None` |
| Pfadbestandteil | `qualifier`, `organization` | auf Linux unbenutzt (`lin.rs:92`) — nur der Anwendungsname zählt |

In allen vier Fällen: **die Eingabe wird ohne Widerspruch angenommen, es gibt keinen Fehler,
und die Ausgabe sieht plausibel aus.** Nichts davon ist ein Absturz. Alles davon hat mehrere
Runden gekostet.

**Der Test, der sie alle vier gefunden hätte:** den Wert ändern und sehen, ob sich die Ausgabe
ändert. Eine Grenze, die sich nicht verschiebt, wenn man sie verschiebt, ist keine Grenze.
Ein Parameter, dessen Änderung nichts bewirkt, ist Dekoration. Ein Inset, das man verdoppeln
kann, ohne dass sich das Layout bewegt, wird nicht benutzt.

Das ist derselbe Grundsatz wie „prüfe, ob die Eingabe den Pfad erreicht, nicht ob der Pfad
existiert" — nur auf Werte statt auf Aufrufe angewandt.

**Für die Design-Prüfung heißt das:** ein Wert, der *aussieht*, als täte er etwas — eine
Begrenzung, ein Inset, eine Konfiguration —, wird geprüft, indem man ihn ändert. Nicht indem
man ihn liest. Lesen bestätigt die Absicht, Ändern bestätigt die Wirkung.

### 22.8 Die Ausgabe muss sich *so* ändern, wie vorhergesagt — nicht nur ändern

Der Variantentest aus §22.7 hat eine Schwachstelle, und dieses Projekt hat sie bezahlt:
**„etwas hat sich geändert" ist eine andere Behauptung als „das Behauptete hat sich
geändert".**

Beispiel Zeilenbegrenzung. Nach dem Umbau änderte sich die Zeilenzahl des Plots — aus fünf
Zeilen wurden drei mit Auslassungszeichen. Alle Beteiligten lasen das als Beweis. Die
Behauptung war aber nicht „die Zahl der Zeilen hat sich geändert", sondern **„die Grenze liegt
an Zeilen und nicht an Zeichen"**. Und im Hochformat können beide Mechanismen dieselbe
Beobachtung erzeugen: 258 Zeichen in einer 269 dp breiten Spalte ergeben so oder so einen
gekürzten Block.

Unterschieden hat erst ein zweiter Eingang: dasselbe 258-Zeichen-Fixture im Querformat. Dort
lief der Text **vollständig durch, ohne Auslassungszeichen** — ein Zeichenbudget hätte auch
dort bei 180 abgeschnitten, eine Zeilengrenze schneidet nicht, wenn drei Zeilen reichen. Erst
dieser Fall trennt die beiden Kandidaten.

**Nachtrag zur Messbedingung.** Dieser Lauf fand bei einem Measure von **655 dp** statt. Seit
`PROSE_MAX_W = 460` (`tokens.rs:34`) bindet nicht mehr die Spalte, sondern das Measure: 258
Zeichen ergeben bei 460 dp rund 4,2 Zeilen und damit auch im Querformat drei Zeilen **mit**
Auslassungszeichen — dieselbe Beobachtung wie im Hochformat. Der Test ist in dieser Form
nicht wiederholbar, und wer ihn wiederholt, sieht ein Ergebnis, das wie eine Regression
aussieht und keine ist.

Was ihn ersetzt, ist stärker als jede Beobachtung: `grep -rn "\.take(" src/ui/mobile/`
liefert nur noch `search.rs:66/94/121` — Ergebnisgrenzen, kein Text. Die Zeichenbegrenzung
ist im Quelltext **nicht mehr vorhanden**, und `wrap` nimmt `rows`. Ein Grep ist erschöpfend,
ein Screenshot ist es nie: er zeigt einen Eingang. Eine Beobachtung wäre nur noch nötig, wenn
man der Zeilenbegrenzung selbst misstraut — dann muss der unterschiedende Eingang die
**Breite** sein, oberhalb von 460 dp, und nicht die Textlänge.

**Regel: vor dem Test aufschreiben, was jeder Kandidat vorhersagt.** Wenn alle Kandidaten
dieselbe Beobachtung vorhersagen, kann der Test sie nicht trennen — und die Beobachtung ist
Dekoration, egal wie deutlich sie ausfällt.

Konkret heißt das für jeden Nachweis in diesem Dokument:

| Frage | Reicht „es hat sich geändert"? |
|---|---|
| Ist die Zeichenbegrenzung weg? | ja — jeder Unterschied genügt |
| Ist die Begrenzung zeilenbasiert? | nein — braucht einen Eingang, bei dem Zeichen- und Zeilenbudget **verschiedene** Ergebnisse liefern |
| Ist `side` jetzt 0? | nein — braucht eine Messung gegen einen erwarteten Wert, nicht gegen den vorherigen |
| Wird gespeichert? | nein — braucht den Neustart, sonst beweist der Test nur, dass der Schreibvorgang *lief* |

Der letzte Eintrag ist der, den die Persistenz-Abnahme prüft: `ls -l` zeigt, dass eine Datei
entstanden ist; erst der Neustart zeigt, dass sie gelesen wird.

### 22.9 Zwei Aufnahmen mit gleichem Hash sind eine Aufnahme

Die Umkehrung von §22.8 auf der Beweismittelseite. Ein Screenshot, der byte-gleich mit einem
früheren ist, ist **kein zweiter Datenpunkt** — er ist derselbe. Und er ist mit **drei**
verschiedenen Geschichten vereinbar:

| # | Erklärung | Gegenmittel |
|---|---|---|
| 1 | Es hat sich nichts geändert — das ist die Behauptung, die geprüft werden sollte | keine: das ist das Ergebnis |
| 2 | Die Aufnahme wurde nicht neu gemacht; der Puffer war noch derselbe | Positivkontrolle in derselben Sitzung |
| 3 | Die Datei ist eine Kopie einer **anderen** Aufnahme; der Name ist falsch | Hash über die ganze Serie, dann das Bild ansehen |

Ursprünglich standen hier zwei Erklärungen. Die dritte fehlte, und sie ist die, die im
aktuellen Fall zutrifft — der Fehler war, „byte-gleich" nur als Frage der Aufnahme zu lesen
und nicht als Frage der **Benennung**.

Aktueller Fall: `A1_confirm_dialog.png`, `A2_dialog_after_back.png` und
`B4_login_back.png` haben denselben Hash `4fb5b26b…`; `B1_series_detail_portrait.png` und
`B3_after_back_series.png` denselben Hash `028002a2…`. Drei Folgen davon:

- **`B4_login_back` ist keine Login-Aufnahme, sondern eine Kopie der Dialog-Aufnahme** — Fall
  3, nicht Fall 2. Das ist ein **falscher Dateiname**, kein Fehler des Tests. Der Test hat
  funktioniert: der Hash hat den falschen Namen aufgedeckt. Ein Dateiname ist eine Behauptung,
  kein Beleg, und dieser hier würde eine Aussage über den Login stützen, die niemand geprüft
  hat.
- **„Zurück ist überall wirkungslos" ist durch diese Bilder nicht belegt.** Es ist durch die
  Diagnose belegt und wird durch die Bilder nicht widerlegt. Das ist weniger, als es aussieht.
- **Der Hash ist die billigste Namensprüfung, die es gibt.** `md5sum probe/*.png | sort`
  findet in einer Sekunde jede doppelt benutzte Datei. Zweimal in diesem Projekt war ein Name
  falsch (`32_series_short_portrait` zuerst, `B4_login_back` jetzt), und beide Male wurde es
  zufällig bemerkt, nie durch Hinsehen.

**Regel: der Name einer Aufnahme ist eine Behauptung und wird gegen das Bild geprüft, bevor
sie zitiert wird.** Praktisch heißt das: den Hash über die Serie laufen lassen, bevor
ausgewertet wird, und jede Aufnahme, die in einem Bericht auftaucht, einmal ansehen. Ein
falscher Name ist derselbe Fehlertyp wie die vier aus §22.7 — er wird stumm akzeptiert und
sieht plausibel aus; nur ist die Ausgabe hier ein Dateiname statt eines Layouts.

**Und daraus folgt für das Zitieren: die Beobachtung zitieren, nicht die Datei.** „Der Dialog
schließt nicht auf Zurück" ist eine Aussage, die stimmen kann oder nicht; „siehe `A2`" ist
eine Aussage, die von einem Dateinamen abhängt, den niemand geprüft hat.

**Was Fall 2 trennt: eine Positivkontrolle in derselben Sitzung.** Eine Aktion, deren Wirkung
feststeht — der Zurück-Pfeil in der App-Bar — und danach eine Aufnahme. Ändert sich der Frame
nach dem Pfeil und nicht nach der Geste, dann läuft die Aufnahme und die Geste ist tot. Ohne
die Kontrolle bleibt „byte-gleich" mehrdeutig, egal wie oft man es wiederholt.

Dieselbe Prüfung gilt für jede Aufnahmeserie: eine Aufnahme, die eine Änderung *belegen* soll,
braucht eine zweite, die eine Änderung *zeigt*.

**Strukturell: den Namen ableiten, nicht behaupten.** Solange der Dateiname von Hand getippt
wird, ist er eine Absichtserklärung, und Absichtserklärungen sind in diesem Projekt bisher
zweimal falsch gewesen. Die Aufnahme soll ihren Namen aus dem Zustand bilden, den sie
tatsächlich vorgefunden hat — neben jedem PNG eine Textdatei mit dem, was das Gerät selbst
meldet (`dumpsys window | grep mCurrentFocus` und der vom Programm gemeldete Bildschirm), und
der Dateiname wird daraus erzeugt. Dann ist ein falscher Name nicht mehr möglich, sondern nur
noch eine fehlende Aufnahme — und die fällt sofort auf.

---

## 23. Android-TV-Tauglichkeit, 10-Foot-UI und adaptive Layouts

Anlass: nach Video-Fix und finalem App-Check prüfen, ob die App auf Android TV mit
Fernbedienung bedienbar ist (D-Pad, größere UI, Logos statt Text) — und was davon auf die
normale Phone-App übergeht. §9, §10.11 und M6 (§11) haben TV-Unterstützung **vorgesehen**;
der Rust/egui-Port (§15) hat sie nicht umgesetzt. Dieser Abschnitt ist Planung. `src/` bleibt
unangetastet, die Umsetzung erfolgt getrennt.

### 23.1 Befund: nicht TV-tauglich, 4 von 5 Kriterien scheitern

| # | Kriterium | Status | Beleg |
|---|---|---|---|
| 1 | Als TV-App auffindbar: `LEANBACK_LAUNCHER`, `<uses-feature android.software.leanback android:required="false">`, `android:banner` 320×180 dp | scheitert | `AndroidManifest.xml:54-57` enthält nur `MAIN`/`LAUNCHER`; kein Banner, keine Feature-Deklaration |
| 2 | Ohne Touch bedienbar: `KEYCODE_DPAD_*`/`ENTER`/`BACK` → egui-Events | scheitert | §23.2 |
| 3 | Fokus sichtbar, Ziel- und Fokusflächen ≥ 64 dp | scheitert | `mod.rs:153` → `TOUCH = 48.0` (`tokens.rs:25`); kein Fokusring |
| 4 | Overscan-sicher: ~5 % Rand (≈27 dp waagerecht / ≈48 dp senkrecht bei 1920×1080) | scheitert | nur `l.pad` + Insets (`mod.rs:75`), auf Telefonmaße gerechnet |
| 5 | Läuft im Querformat, Geometrie nicht hartkodiert | **hält** | `configChanges` deckt `uiMode`/`navigation` ab (`AndroidManifest.xml:49`); `Layout::new` liest `ctx.screen_rect()` (`mod.rs:64`) |

Kriterium 5 ist die einzige Grundlage, auf der §23.7 aufbauen kann. Zwei Befunde, die in P1
(§23.4) mitlaufen, aber keine eigenen Kriterien sind: die Typografie ist telefon-groß
(`FS_BODY = 16`, `FS_HEAD = 17`, `FS_CAP = 11.5`; `tokens.rs:50-58`) — TV verlangt ≥ 24 sp
Body, größere Köpfe, high-contrast dunklen Grund — und §9 beschreibt D-Pad-Verhalten als
**Vorgabe**, nicht als Code (§15).

**Regel: eine in §9/§10.11/M6 zugesagte Fähigkeit gilt erst als vorhanden, wenn eine geänderte
Eingabe sie verändert (§23.8). Lesen bestätigt die Absicht, nicht die Wirkung (§22.7).**

### 23.2 Die Input-Kette: warum DPAD/ENTER keine egui-Events werden

- `GameActivity` erbt `com.google.androidgamesdk.GameActivity` (`GameActivity.java:41`).
  Dessen `InputEnabledSurfaceView` besitzt einen eigenen Eingabekanal über
  `onTouchEvent`/AInputQueue (`GameActivity.java:132`, `:259`).
- `dispatchKeyEvent` behandelt **nur** `KEYCODE_BACK` und reicht alles andere an `super` durch
  (`GameActivity.java:270-279`). `KEYCODE_DPAD_*` und `KEYCODE_ENTER` werden nirgends in native
  Events übersetzt.
- `jni_bridge.rs` pusht IME-Text (`:9`) und Insets (`:11`), keine Tastendrücke. Es gibt keinen
  Pfad, auf dem eine Fernbedienung `egui::InputState.events` erreicht.
- egui 0.27 hat **keinen** eingebauten Fokus-Navigationsgraphen und zeichnet **keinen**
  Fokusring. Selbst übersetzte Keys hätten nichts, das „fokussiert" heißt, und nichts, das es
  anzeigt.

**Regel: TV-Bedienbarkeit beginnt bei der Tastenübersetzung, nicht beim Layout.** Ein
Fokusring ohne Key-Events ist Dekoration; Key-Events ohne Fokuszustand sind wirkungslos. Beides
gehört zusammen in P0 (§23.4).

### 23.3 Vorbilder: was TV- und Streaming-Apps visuell anders machen

Quellenbelegt, ohne Bewertung; die Muster sind Vorlage, nicht Spezifikation.

- **AndroidX Leanback `BrowseSupportFragment` / `RowsSupportFragment`:** linke
  „Headers"-Navigationsspalte + waagerechte Kartenreihen; Fokus bewegt sich **räumlich**,
  fokussierte Karte skaliert hoch.
- **Netflix / Disney+ / Prime Video:** Poster-/Logo-Hero statt Textliste, waagerechte Rails,
  Senderlogos statt Textzeilen, gedämpfte Metadaten erst bei Fokus, vollflächiger Backdrop des
  fokussierten Titels.
- **Plex:** linke Nav-Rail + Posterreihen, Fokus = Skalierung + Rand. Nur allgemeine Referenz,
  keine spezifischen Behauptungen.
- **IPTV-spezifisch — Tivimate / Perfect Player / IPTV Smarters:** EPG-zuerst mit Kanalogos im
  Raster, großem Jetzt/Danach-Text, farbigem Fortschrittsbalken, Zifferntasten für direkten
  Kanalaufruf, kompaktem „TV Guide"-Raster statt langer vertikaler Textlisten. **Sideloadete
  Apps, keine Play-Store-TV-Apps — für sie wird keine Leanback-Konformität behauptet.**
- **Adaptive Layouts:** Android Window Size Classes (compact / medium / expanded),
  Foldable-Posture und Hinge-Bewusstsein.

**Quellen (kompakt):**

- https://developer.android.com/design/ui/tv/guides
- https://developer.android.com/training/tv/start/start-tv — `LEANBACK_LAUNCHER`, Banner 320×180 dp, Overscan 5 %, 10-Foot-Typografie
- https://m3.material.io/foundations/layout/applying-layout/tv — Fokusring, 64 dp Ziele
- https://developer.android.com/training/tv/playback/browse — Browse / Headers / Rows
- https://support.plex.tv/articles/ — allgemeine Referenz
- https://developer.android.com/guide/topics/large-screens/support-different-screen-sizes
- https://developer.android.com/guide/topics/large-screens/learn-about-foldables

### 23.4 Priorisierter Plan

**P0 — TV-Einstieg (bedienbar).** `AndroidManifest.xml`: `LEANBACK_LAUNCHER`-Alias/Intent-Filter,
`<uses-feature android.software.leanback android:required="false">`, `android:banner`
320×180 dp; Phone-Launcher-Icon bleibt. `GameActivity.java`: DPAD/ENTER/BACK in native Events
übersetzen, `jni_bridge.rs` pusht sie in `egui::InputState.events`. Fokus als Zustand in `App`
(`focus: Option<egui::Id>`, `app.rs:58`), sichtbarer Fokusring in `components.rs`.
*Abnahme:* Kaltstart auf einem TV-Emulator erreicht Wiedergabe **nur** mit DPAD/ENTER/BACK,
Fokus in jedem Schritt sichtbar.

**P1 — 10-Foot-Darstellung.** Eigenes TV-Layoutprofil: Overscan-Rand 5 %, Ziel-/Fokusflächen
≥ 64 dp, Body ≥ 24 sp, Poster-/Logo-Rails statt Textlisten, Hero-Backdrop bei Fokus.
*Abnahme:* aus 3 m lesbar, kein Bedienelement im Overscan-Band.

**P2 — IPTV-spezifisch.** Kanalogitter mit Logos, EPG Jetzt/Danach, Zifferntasten-Direktwahl,
größerer Fortschrittsbalken.

### 23.5 Regel: TV-Profil ist ein eigener Modus, kein globaler Faktor

**Regel: das TV-Profil ist ein separater Layout-/Skalenmodus, abgeleitet aus Breiten-/Höhen-
Buckets und Eingabemodalität — kein globaler `pixels_per_point`-Multiplikator auf die Phone-UI.**

Ein globaler Faktor verwischt Schriften (die `FS_*` sind gemessene Token, `tokens.rs:50-58`),
bricht die 48 dp-Touch-Ziele (`TOUCH`, `tokens.rs:25`; §18) und zerlegt Querformat- und
Foldable-Layouts — `mod.rs:65` nutzt `pixels_per_point` bereits ausschließlich zum Umrechnen
der Insets. Skaliert werden **Werte** (`pad`, `cols`, `FS_*`, Zielgrößen), nicht der
Koordinatenrahmen. §18.4 (Rail ab 600 dp nutzbarer Breite) und `COMPACT_MAX_H = 400`
(`mod.rs:35`) sind schon bucket-artige Regeln und bleiben die Vorlage.

### 23.6 Was auf die normale Phone-App übergeht

Bounded Shortlist für die getrennte Umsetzung. Keine zweite UI-Codebasis.

| | Maßnahme | Ort / Grenze |
|---|---|---|
| A | Kanalogos / Poster statt reiner Textzeilen in Live TV, **wenn** Artwork vorhanden; Buchstabenkachel-Fallback bleibt | `components.rs:632` (`thumb`), `:661` (`poster_card`), `:694`; `live.rs:108` (`list_row`) |
| B | Stärkerer „fokussiert/gewählt/aktuell"-Zustand auf Zeilen und Karten: subtile Skalierung + Akzentrahmen, aus TV-Fokusringen entliehen, aber **durch Touch/Selektion getrieben, nicht durch D-Pad-Fokus** | `components.rs` (`list_row`, `poster_card`) |
| C | Jetzt/Danach-Fortschrittsbalken auf Live-Zeilen, bessere Scanbarkeit bei jeder Größe | `live.rs` |
| D | Nutzbare Breiten-/Höhen-Buckets aus `ctx.screen_rect()` in `mod.rs` (compact / medium / expanded) steuern `pad`, Spaltenzahl und Prosa-Maximalbreite statt fester Phone-Annahmen; `COMPACT_MAX_H` bleibt | `mod.rs:64`, `:101` (`cols`), `tokens.rs:44` (`PROSE_MAX_W = 460`) |
| E | Größere Tippziele **nur dort, wo sie nichts kosten** — kein Rückschritt bei den gemessenen 48 dp / 16 dp aus §18 | `tokens.rs:11` (`PAD = 16`), `:25` (`TOUCH = 48`) |

**Nicht-Ziele (explizit, Stand der Planung):** kein D-Pad-Fokus-Engine in der Phone-App,
kein Overscan-Rand auf Phones, kein globaler Font-Upscale, keine zweite UI-Codebasis.

> **Aufgehoben durch nachfolgende Anforderung:** Der Nutzer hat danach ausdrücklich
> „mit Fernbedienung bedienbar" für die *normale* Phone-App verlangt. Der erste
> Nicht-Ziel-Punkt (kein D-Pad-Fokus-Engine) ist damit **nicht mehr gültig**; die
> Umsetzung steht in §23.9. Overscan, globaler Font-Upscale und zweite UI-Codebasis
> bleiben bewusst ausgeschlossen.

### 23.7 Adaptive Buckets: die App leitet ihre Klassen selbst ab

- Android Window Size Classes: **compact / medium / expanded**; Foldables zusätzlich Posture
  und Hinge (Quellen §23.3). Kleine Phones, große Phones, Foldables, Hoch-/Querformat und
  tablet-artige Layouts bedienen dieselben Regeln.
- **Jetpack WindowManager ist für eine Rust/egui-App nicht verfügbar.** Die App muss ihre
  Buckets selbst aus `ctx.screen_rect()` und den Insets ableiten, nicht aus einer Plattform-API.
  Vorhanden: `Layout::new` (`mod.rs:64`), `active()` mit Breiten-/Höhen-Schwelle
  (`mod.rs:112-123`), `cols()` auf 5 geklemmt (`mod.rs:101`), `COMPACT_MAX_H = 400`
  (`mod.rs:35`).

| Bucket (nutzbare Breite) | `pad` | Spalten | Prosa |
|---|---|---|---|
| compact (< 600 dp) | `PAD` + `side` | 1–3; `COMPACT_MAX_H` gilt | `PROSE_MAX_W` |
| medium (600–839 dp) | `PAD` + `side` | Rail statt unterer Leiste (§18.4) | `PROSE_MAX_W` |
| expanded (≥ 840 dp) | `PAD` + `side` | bis 5 (`cols`-Klammer) | `PROSE_MAX_W` |

**Regel: Buckets werden aus der nutzbaren Fläche nach Insets gebildet, nicht aus der rohen
Bildschirmgröße.** `COMPACT_MAX_H` (kompaktes Querformat) bleibt als eigene Höhenregel
unangetastet — Breiten-Buckets ersetzen sie nicht.

### 23.8 Prüfregel: Eingabe ändern, Ausgabe muss sich ändern

In der Nachfolge von §22.7/§22.8. Vor einer Aussage „TV-tauglich" oder „foldable-tauglich"
wird ein **Eingabewert verändert** und geprüft, ob die **Ausgabe** sich ändert:

| Eingabe ändern | Erwartete Ausgabe |
|---|---|
| Fenstergröße / 1920×1080-TV-AVD | anderes `pad`, andere Spaltenzahl, TV-Profil statt Phone-Profil |
| Hochformat → Querformat | `COMPACT_MAX_H` greift, Hero 96 dp (§18.3) |
| Tastatur ein/aus | `l.ime` verschiebt den Inhalt, Banner bleibt im Scrollbereich (§18.7) |
| DPAD/ENTER auf TV-AVD | Fokusring wandert, Aktion wird ausgelöst |

Vier Regeln, damit der Test etwas beweist:

1. **Vorher aufschreiben, was jeder Kandidat vorhersagt** (§18.3, §22.8). Sagen alle
   Kandidaten dieselbe Beobachtung voraus, trennt der Test sie nicht.
2. **Unveränderte Screenshots sind kein Beleg.** Zwei byte-gleiche Aufnahmen sind eine (§22.9);
   eine Aufnahme, die eine Änderung belegen soll, braucht eine zweite, die eine zeigt.
3. **Die Ausgabe muss sich *so* ändern, wie behauptet**, nicht nur „irgendwie": ein globaler
   Skalierungsfaktor und ein eigenes TV-Profil erzeugen beide „größer"; unterscheiden tut erst
   die 48 dp-Zielmessung und der Overscan-Rand.
4. **Positivkontrolle in derselben Sitzung** (§22.9): eine Aktion mit bekannter Wirkung, danach
   die Aufnahme. Ohne sie bleibt „unverändert" mehrdeutig.

**Regel: TV- und Foldable-Unterstützung werden nicht durch einen bestehenden Schnitt behauptet,
sondern durch eine geänderte Eingabe mit vorhergesagter, sichtbar anderer Ausgabe belegt.**

### 23.9 Umsetzungsstand (Ship-Kandidat, 2026-11-05)

§23.6 A–E sind umgesetzt: Logos/Poster mit Buchstabenkachel-Fallback (`components.rs`),
`SEL_SCALE`/`SEL_RING` für gewählte Zeilen, Jetzt/Danach-Balken aus EPG-Daten (`live.rs`),
Breiten-/Höhen-Buckets in `mod.rs::Layout::new` aus **nutzbarer** Breite nach Insets.
Die gemessenen 48 dp / 16 dp Token blieben unangetastet. Kein zweites UI, kein globaler
Font-Upscale, kein Overscan auf Phones.

Fernbedienung (P0 aus §23.4) steckt in der **normalen** Phone-App, nicht nur im TV-Profil:

- `platform::RemoteKey` + `drain_remote_keys()` (Desktop liefert eine leere Liste).
- `GameActivity.dispatchKeyEvent` → `nativeOnKey` (JNI) → begrenzte Queue (32 Einträge, nur
  `ACTION_DOWN`), `wake_ui()` nach jedem Key. Java und Rust teilen **dieselbe** Keycode-Tabelle
  (DPAD_*, ENTER, NUMPAD_ENTER, MEDIA_PLAY/PAUSE/PLAY_PAUSE).
- BACK bleibt auf der vorhandenen Kette `onBackPressed` → `nativeRequestBack` →
  `App::mobile_back()`. Die Fokus-Engine markiert Back nur als „Fokus aktiv" und poppt **nicht**,
  sonst würde doppelt zurück navigiert.
- Manifest: `LEANBACK_LAUNCHER`, `leanback required=false`, `touchscreen required=false`,
  `android:banner` 320×180 dp (Platzhalter).
- `ui/mobile/focus.rs`: thread-lokaler linearer Fokus in Registrierreihenfolge,
  Screen-Fingerprint-Reset, Touch blendet den Ring aus, `pending_ok` aktiviert genau **ein**
  Control, `scroll_to_rect` bei Richtungswechsel, `begin_modal()` beschränkt den Fokus auf
  Scrim-Overlays. Ring = Akzent-Halo + `tokens::focus_ring()` in Hochkontrast, weil
  Akzent-auf-Akzent auf dem blauen „Anmelden"-Button unsichtbar war.
- `PlayPause` wird im Player aus den Fokus-Keys gefiltert und schaltet nur `mpv`-Pause — sonst
  aktivierte die Medientaste das erste Control (Zurück) und beendete die Wiedergabe.

**Geräte-Verifikation** (emulator-5554, 1080×2400, Hochformat; Screenshots in `probe/qa_modal/`):
4× Rechts → Ring auf „Mehr"; OK → Sheet mit Ring **nur** auf „Suche" (kein Ring hinter dem
Scrim); ↓ → „Favoriten"; OK → Ansicht wechselt und Sheet schließt; BACK → Sheet schließt, App
bleibt offen. Lösch-Dialog: 3× ↓ → Ring auf Mülltonnen-Icon; OK → Dialog öffnet mit Fokus auf
**Abbrechen** statt „Löschen"; OK bricht ab und das Profil bleibt erhalten; ↑ → „Löschen",
OK löscht (`profiles.json` → `[]`). Hoch-/Querformat, Medientaste und animiertes Live-Video
wurden in derselben Sitzung separat belegt.

**Ship-Blocker behoben:** „Ohne Anbieter testen" lud bisher ein nur per `run-as` aufgespieltes
`test.avi` — auf einer frischen Installation blieb der Player schwarz. `src/test_pattern.rs`
erzeugt das unkomprimierte RGB24-AVI (96×72, 10 fps, **300 Frames / 30 s**, 6.228.232 Byte)
nun zur Laufzeit in `filesDir` bzw. `%TEMP%`, byte-identisch zum auf dem Gerät bewährten
`probe/video_test/gen_avi.py`; `play_test_stream()` baut die `file://`-URL daraus. Bei einer
Änderung von `FRAMES` greift der Größen-Fast-Path nicht und die Datei wird neu geschrieben.
Die QA-Datei-Hooks (`qa_demo`, `autostart_test`) und der `[layout]`-Diagnose-Log sind aus dem
Release-Code entfernt; `IPTV_DEMO` / `IPTV_TEST_STREAM` bleiben als Desktop-Env-Variablen.

**Repaint-Korrektur (betrifft alle Wiedergaben, nicht nur das Testbild):** `App::update()`
bat im Player bisher nur alle 120 ms um einen Frame. `vo=libmpv` zeigt einen Frame aber erst,
wenn der Client `mpv_render_context_render()` aus dem egui-Paint-Callback aufruft — 120 ms
waren eine harte ~8-fps-Obergrenze (unter den 10 fps des Testclips, weit unter 25/50 fps
Live-TV). Im `View::Player` wird jetzt pro Frame `ctx.request_repaint()` angefordert,
außerhalb bleibt der 400-ms-Poll für Fortschritt/EPG.

**Diagnose „schwarzer Player bei Clean-Install" (QA-Artefakt, kein App-Bug):** Die ersten
Clean-Install-Screenshots lagen 1 s bzw. 7 s nach OK. `mpv.log` zeigt auf dem
SwiftShader-Emulator: `loadfile` ~11 s, `Starting playback` ~14 s, `Video display: 96x72 ->
1080x810` und `first video frame` erst ~17,5 s nach mpv-Start; der Kaltstart braucht zusätzlich
~45 s, bis die Login-UI überhaupt gezeichnet ist. Ein 10-s-Clip war deshalb oft beendet, bevor
ein Frame die Oberfläche erreichte. Ein weiterer Fehlversuch war ein Android-`ANR in system`
des überlasteten Emulators, dessen Dialog den OK-Key schluckte — nicht die App.

**Finale Geräte-Verifikation** (APK `a96412dd…`, `pm clear` + Kaltstart, emulator-5554,
1080×2400): ein OK auf „Ohne Anbieter testen" erzeugt `test.avi` mit exakt 6.228.232 Byte im
`filesDir`; zwei Captures innerhalb der Wiedergabe unterscheiden sich um
**879.919 / 2.592.000 px (33,95 %)**, bbox genau im Video-Rect `(0,795)–(1080,2066)` —
animiertes Live-Video, kein Standbild. Screenshot: „Testbild (lokal)", `00:22 / 00:30`,
Farb-Gradient und weißer Fokus-Ring auf der Zurück-Taste. BACK kehrt auf den Login zurück,
DPAD_DOWN verschiebt den Ring auf „Anmelden".

**Bekannte Restpunkte (keine Ship-Blocker):** Auf Software-GL (SwiftShader) bleibt
`mpv_render_context_render() not being called or stuck.` sporadisch in `mpv.log`, weil der
Client langsamer konsumiert als mpv liefert (Rate ~2,1/s → ~1,4/s nach der Repaint-Korrektur);
auf echter Hardware nicht geprüft. Nach EOF (`keep-open=no`) wird der Player schwarz und
repaintet weiter — Feinschliff für später, ebenso ein loopendes Testbild. Vor dem
Store-Release bleiben: Release-Signierung (`android/SIGNING.md`), echtes Banner-/
Launcher-Artwork statt Vektor-Platzhalter. P1 „10-Foot-Layout" war **nicht** Teil dieses
Ship-Kandidaten und ist inzwischen als eigenes TV-Profil nachgezogen — siehe §23.10.

### 23.10 Eigener TV-Modus: Headers-Spalte statt Bottom-Bar (2026-11-06)

**Anlass:** Nach dem Login zeigte die TV-Variante dasselbe UI wie das Handy — inklusive
Bottom-Bar (`Live / Filme / Serien / Fortsetzen / Mehr`) und „Mehr"-Sheet. Ohne Touchscreen
lässt sich das nicht sinnvoll bedienen: Es fehlt ein echtes Vor-und-Zurück-Menü. §23.3 hatte
dafür längst die Vorbilder geliefert (Leanback `BrowseSupportFragment`, Netflix/Disney+,
Plex, Tivimate). P1 aus §23.4 wird damit nachgezogen, nicht nur geplant.

**Modus-Erkennung.** `GameActivity.onCreate` liest
`UiModeManager.getCurrentModeType() == UI_MODE_TYPE_TELEVISION` und meldet es über
`nativeOnTvDevice(boolean)` an `jni_bridge::TV_DEVICE` → `platform::tv_device()`.
`Settings::TvMode` (`Auto | Immer | Aus`, `#[serde(default)]`) erlaubt einen echten
Nutzer-Override — nötig für TV-Boxen ohne Leanback-Feature, Handys am Fernseher und QA.
Chips in `settings.rs`, ein eigener Settings-Rail im TV-Modus. `App::update` löst den Modus
**jeden Frame** auf, damit der Wechsel sofort sichtbar wird statt erst nach einem Neustart.

**`src/ui/mobile/tv.rs` (neu).** Linkes Headers-Feld (7 Einträge), horizontale Karten-Rails,
Hero-Band mit Titel/Untertitel des fokussierten Elements, Jetzt/Danach-Balken, Logos bzw.
Buchstabenkacheln, 5 % Overscan, ≥64 dp Trefferflächen, 24 sp Body / 32 sp Heading, deutlicher
Fokus-Ring. Keine Bottom-Bar, kein „Mehr"-Sheet. Buchstaben-Suche auf dem Bildschirm, weil
IME-Eingabe mit einer Fernbedienung unzumutbar ist. „Oberfläche ohne Anbieter ansehen" ruft
`App::load_demo()` auf und macht das Menü ohne Zugangsdaten erreichbar (zugleich
deterministischer QA-Pfad).

**Räumlicher Fokus.** `focus::begin(..., spatial)` ergänzt den linearen Phone-Fokus um eine
geometrische Nachbarschaftssuche über die `Rect`s des Vorframes (Richtung dominant,
Querabstand sekundär, Beam-Overlap bevorzugt — angelehnt an Androids `FocusFinder`).
Linear bleibt Login, Player und Phone-App. `request_index()` / `index()` erlauben es dem
Navigationsstack, den Fokus gezielt auf einen Header zurückzusetzen.

**Echter Vor-/Zurück-Stack.** `App::tv_goto(nav)` push den vorherigen Abschnitt,
`tv_series` merkt geöffnete Serien. `mobile_back()` schließt zuerst die Serie, dann den
letzten Abschnitt, dann springt es aus einer Kartenreihe zurück in die Headers-Spalte;
erst auf Root-Ebene liefert `nativeCanGoBack()` `false` und Java beendet die Activity.
BACK bleibt damit **ein** Pfad und poppt nicht doppelt (Regel aus §23.9).

**Kaltstart-Black-Screen — Ursache gefunden und behoben.** `Cargo.toml` aktiviert eframes
`wgpu`-Feature global (für den Windows-Backend-Dispatcher) und `glow` zusätzlich für Android.
Feature-Unification ⇒ beide Features an ⇒ `eframe 0.27::Renderer::default()` liefert **Wgpu**.
Folgen: (1) wgpu fordert `max_uniform_buffer_binding_size = 65536`, der Emulator-/viele
Android-Treiber liefern nur `16384` ⇒ `run_native` scheitert mit `LimitsExceeded`;
`android_main` kehrt zurück, die Activity schließt sich, der Bildschirm bleibt schwarz und
es folgt ein ANR-Dialog. (2) Das mpv-Bild läuft über `egui_glow::CallbackFn`, den **nur** der
Glow-Painter ausführt — unter wgpu würde gar kein Video gezeichnet.
Fix: `native_options.renderer = eframe::Renderer::Glow;` in `platform/android/entry.rs`.
Zusätzlich wird das `run_native`-Ergebnis nicht mehr verworfen, sondern geloggt (`dado:
eframe::run_native failed: …`), und `App::update` hält in den ersten 12 s einen 500-ms-Poll,
falls der erste Frame auf langsamen GL-Treibern verworfen wird. Ohne das Logging war der
Fehler unsichtbar — genau deshalb war die vorige QA-Sitzung in schwarzen Screenshots und
einem Emulator-ANR steckengeblieben.

**Geräte-Verifikation** (APK md5 `5ecdb8a4…`, 51.875.060 Byte, emulator-5554,
`tv_mode=Always` via `settings.json`): Kaltstart erreicht `dado: data dir = …`, kein
`run_native failed`, kein ANR. Zwei DPAD_DOWN + OK öffnen das Demo-Menü; Fokus-Ring lag
messbar auf „Oberfläche ohne Anbieter ansehen". Im Querformat (2400×1080) erscheinen die
**vollen** Header „Live TV / Filme / Serien / Fortsetzen", das Hero-Band zeigt
„Sender, Logos und Jetzt / Danach" **und** darunter getrennt
„Pfeiltasten: navigieren · OK: auswählen · Zurück: vorheriges Menü" — der zuvor
beobachtete Text-Overlap ist behoben (Höhe `0,36·content`, clamp 150–200, Hint-/Progress-
Zone wird vor den Beschreibungslinien reserviert). Im engen Hochformat greifen
`SHORT_HEADERS` („Weiter / Favs / Setup") und mind. 160 dp Rail-Breite, statt
„Forts… / Favo… / Einst…" abzuschneiden.

Eingabe→Ausgabe-Belege derselben Sitzung: DPAD_RIGHT Headers→erste Karte (Hero
„Tagesschau 24 / Kanal 1"), nochmal RIGHT → „Welt Nachrichten / Kanal 2" (9,32 % der
2.592.000 px geändert, bbox `(726,291)–(1708,1017)` = Hero + Kartenzeile); BACK → Ring
zurück auf „Live TV", Hero zurück auf die Abschnittsbeschreibung, App bleibt offen
(`DadoBack: … canGoBack=true`). DOWN → „Filme", OK ⇒ Abschnitt „Filme" mit Rail
„Alle Filme"; BACK ⇒ zurück zu „Live TV" (11,71 % geänderte px). Zum Vergleich auf
demselben APK mit `tv_mode=Never`: Phone-UI mit Top-Bar, vertikaler Senderliste,
Bottom-Nav und „Mehr"-Sheet (Suche / Favoriten / Einstellungen / Abmelden), Fokus-Ring
auf „Live" bzw. „Suche". Screenshots: `probe/qa_tv/90…102_*.png`.

**Offen (P2, keine Ship-Blocker):** Echte EPG-Grid-Ansicht, Ziffern-Direktwahl, reale
Senderlogos aus dem Anbieter statt Buchstabenkacheln, Release-Signierung und
Banner-Artwork. Auf echter TV-Hardware (nicht SwiftShader) ist der Modus noch nicht gemessen.
