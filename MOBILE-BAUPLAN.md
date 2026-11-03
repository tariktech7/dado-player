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
- **Sicherheits-To-do (offen, auch auf Desktop):** Passwörter liegen im Klartext. Auf Android: `EncryptedSharedPreferences` oder Keystore-verschlüsselt; die JSON-Struktur dabei unverändert lassen (Feld `password` bleibt, nur der Inhalt wird entschlüsselt/verschlüsselt).

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
Virtualisierte Kanalliste mit Logo, Kategorie, Suche, Favoriten-Stern. Inline-Vorschau (Bottom-Sheet auf Handy, Seitenleiste auf Tablet/TV), Zappen mit Vorschau-Nachladen, Debounce (§9).
**Fertig wenn:** 10.000 Kanäle flüssig scrollen, Zappen per ↑/↓ die Vorschau in <1 s aktualisiert, kein Speicherleck.

### M3 — Player
Vollflächiger Player nach §8.3, Auto-Hide 3 s, Fortschritt + Puffer, Lautstärke, ±10 s, Vollbild, Live-Badge, Zurück oben links, Titel oben rechts. `PlayerController`-Abstraktion.
**Fertig wenn:** Ein Film lässt sich pausieren, spulen, die Position überlebt App-Neustart; Live zeigt kein Fortschrittselement; nach 3 s ist die Leiste weg und kommt bei Berührung zurück.

### M4 — Filme & Serien
Poster-Grid (2:3, responsiv), Detailansicht, Serien → Staffeln → Folgen, Fortschritt pro Folge, „Fortsetzen"-Button ab 5 s Wiedergabe.
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
- [ ] `Cargo.toml`, `.cargo/config.toml`, `target/`, `dist/` — Desktop-Build

**Vor dem ersten Commit klären:**
- [ ] Stack-Entscheidung (§5) mit dem Nutzer
- [ ] Zielgeräte: nur Handy? + Tablet? + Android TV? (beeinflusst M6)
- [ ] Ein **echter** Test-Account für die Entwicklung (Xtream oder M3U) — ohne den ist M1 nicht abnehmbar
- [ ] Ist iOS wirklich geplant? (ändert die Stack-Wahl, nicht das Design)

**Definition of Done für v1.0 Mobile:** M0–M7 abgehakt, Paritätstabelle aus §2 vollständig, auf einem echten Gerät mit einem echten Provider getestet, kein Crash in 30 Minuten Nutzung, APK < 60 MB.
