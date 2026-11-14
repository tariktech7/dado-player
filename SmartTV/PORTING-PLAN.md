# Dado Player — Samsung Smart TV (Tizen) Porting Plan

**Status: HISTORISCH.** Ersetzt durch [README.md](README.md), [UI-SPEC.md](UI-SPEC.md)
und [TECHNICAL-PLAN.md](TECHNICAL-PLAN.md) für LG webOS **und** Samsung Tizen.
Der ursprüngliche Stand ist unter docs/history/2026-11-07-samsung-porting-plan.md erhalten.
**Basis:** the TV architecture and packaging workflow of IPTVnator
(`references/iptvnator/docs/architecture/tv-application.md`,
`references/iptvnator/apps/tv/tizen/README.md`,
`references/iptvnator/docs/development/tv-development.md`), adapted to a Rust/egui product.
**Date:** 7 November 2026

---

## 1. Decision

Ship Samsung support as **one separate Tizen web client**, not as a port of the Windows/Android binary.

- The Windows app is Rust + egui + libmpv. None of that runs in a Tizen widget: there is no
  Rust runtime, no native window, no libmpv, and the Tizen web engine is Chromium.
- What *is* portable is the **product contract**: Xtream/M3U data model, categories, favourites,
  continue-watching, settings semantics, visual language, and the 10-foot interaction rules.
- Therefore: reuse contracts and UX, rebuild rendering, playback and persistence with web APIs.
  This is exactly the split IPTVnator uses (separate Preact TV client, same provider contracts).

Do **not** promise the same codec/transport support as the Windows build until it is tested on
real hardware. TV codecs depend on the panel's SoC, not on our code.

## 2. Targets

| Target | Status | Notes |
| --- | --- | --- |
| **Samsung Tizen 5.5+ (2020, Chromium 69)** | Primary target | Build floor is Chromium 68, same as IPTVnator's shared TV build. |
| Tizen 5.0 (2019, Chromium 63) | Out of scope | Below the Chromium 68 floor of the shared TV build. |
| Orsay 2011–2014 (pre-Tizen Samsung) | Out of scope | Legacy runtime, no modern web engine; a separate investigation at most. |
| LG webOS | Optional later | Same client can be packaged as `.ipk`; not part of the first milestone. |

Distribution is **Developer Mode only**: the owner builds, signs with their own free Samsung
certificate and installs the `.wgt` on their own TV. No store listing, no pre-signed package.

## 3. Architecture

```
smarttv/
  tizen/
    config.xml          # widget manifest: id, version, privileges, access origin
    index.html          # single page, classic deferred bundle (no ES modules)
    app/                # our TV client (vanilla JS, no framework)
      api/xtream.js     # provider adapter (same fields as src/client.rs)
      api/m3u.js        # M3U/M3U8 parser + VOD detection
      data/store.js     # localStorage: sources, favourites, progress, settings
      ui/               # rails, hero, grids, EPG, settings, player overlay
      player.js         # AVPlay (live TS/HLS) + <video> (VOD MP4/HLS)
      keys.js           # remote key map + Back handling
    icon.png            # 512×512, TV-safe
```

### 3.1 Data layer

- **Xtream:** same endpoints and field mapping as `src/client.rs`
  (`player_api.php` actions, `get_short_epg`, VOD/series info). Base64-encoded EPG text must be
  decoded, and the panel's `server_info.timezone` applied to programme times.
- **M3U:** parse `#EXTINF` (name, logo, group-title), keep `url|User-Agent` variants, detect
  standalone VOD by extension/`#EXTGRP`. No invented series hierarchy.
- **TLS:** the TV web engine validates certificates. Xtream panels frequently use self-signed
  certificates on a separate `https_port`; a browser cannot disable verification. Practical
  options, in order: (1) provider offers a valid certificate, (2) HTTP port, (3) route through
  the optional companion (section 6). Document the limitation in the app's error text.
- **Secrets:** never log provider URLs, credentials or resolved stream URLs.

### 3.2 Playback

- **Live TV (TS/HLS):** Tizen **AVPlay** (`webapis.avplay`) — hardware decoding, the only
  reliable path for MPEG-TS on Samsung panels.
- **VOD / series (MP4, HLS):** native `<video>` with HLS where the panel supports it; fall back
  to AVPlay for containers the element rejects.
- One player instance at a time. On `pause`/`end`/`error`/source change/app hide, call
  `avplay.stop()`/`close()` and release the display.
- Screensaver: keep the screen awake while playback is active, restore protection on stop.

### 3.3 Persistence

`localStorage` only: sources, favourites, watch progress, settings. No cloud, no telemetry, no accounts.

## 4. Package, sign, install

1. Install **Tizen Studio** with *Samsung TV Extension* and *Samsung Certificate Extension*.
   Put `tizen` (`tizen.bat` on Windows) and `sdb` on `PATH`.
2. On the TV: **Apps → 1 2 3 4 5 → Developer Mode On**, enter the PC's IP, restart the TV.
3. In Certificate Manager: create a **Samsung** certificate profile for TV, sign in with a
   Samsung account, add the TV's DUID (shown after `sdb connect <tv-ip>:26101`).
   Keep the profile out of the repository and back it up.
4. Build the widget input, then package and sign:

```sh
tizen package -t wgt -s <profile> -- smarttv/tizen
# output: smarttv/tizen/*.wgt (single file)
```

5. Install:

```sh
sdb connect <tv-ip>:26101
tizen install -s <tv-ip>:26101 -n "DadoPlayer.wgt" -- smarttv/tizen
```

6. Repeat steps 4–5 to update; sources and settings stay on the TV.

Never commit certificates, profiles, DUIDs or signed `.wgt` files.

## 5. Manifest requirements (`config.xml`)

- Widget ID neutral and owned by this project (e.g. `https://github.com/<owner>/dado-player/tv`).
- `required_version` ≥ `5.5`.
- Privileges, kept minimal:
  - `http://tizen.org/privilege/internet` — provider APIs, EPG, artwork, streams.
  - `http://tizen.org/privilege/tv.inputdevice` — register media keys.
  - `http://tizen.org/privilege/application.launch` only if a companion app must be opened.
- `<access origin="*" subdomains="true"/>` for user-selected provider hosts.
- No filesystem, no location, no telemetry privilege.

## 6. Optional companion service (only if needed)

If provider CORS/TLS/headers make direct access impossible, the existing Windows app can expose a
small LAN HTTP API (reusing `src/client.rs`): the TV then talks to one origin, and the companion
handles TLS, `User-Agent`/MAG headers and redirects. This mirrors IPTVnator's backend, but it is
**optional** and must be explicit — never silently fall back between direct and companion access.
Bind to LAN only on request; no credentials in logs.

## 7. 10-foot UI specification

- Design canvas **1920×1080**, safe area **5 %** on all sides (overscan).
- **Focus is the primary state**: 2 px accent ring + 3 % scale, never colour-only. Focus must be
  visible on every surface and on accent-filled buttons.
- Navigation: left rail (Live, Movies, Series, Search, Favourites, Continue, Settings) as
  icon + label, selected = tinted surface + outline (same language as desktop).
- Home: hero (backdrop, title, meta, Play/Resume) + horizontal poster rails.
- Lists: 5–6 visible rows, no nested scrolling; the list owns the scroll, the header stays.
- EPG: horizontal timeline, one row per channel, "now" indicator, 2-hour window, D-pad scroll.
- Text: minimum 24 px body at 1080p; metadata 20 px; truncate, never wrap into the next row.
- Player overlay: title, position/duration, play/pause, audio/subtitle, stream info; auto-hide
  after 4 s, any key brings it back.

## 8. Remote keys

| Key | Code | Action |
| --- | --- | --- |
| Back | `10009` | close overlay → pop view → exit app |
| Enter / OK | `13` | activate focused control |
| Arrows | `37/38/39/40` | move focus |
| Play/Pause | `10252` | toggle playback |
| Play | `415` | play |
| Pause | `19` | pause |
| Stop | `413` | stop |
| Rewind / FF | `412/417` | seek ∓10 s / ±10 s |

Register only keys the API exposes; a missing key must warn and never block the rest.

Back priority: open dialog → fullscreen/overlay → view history → app exit. Exit follows Samsung's
policy (confirmation on the root view if the platform requires it).

## 9. Milestones

1. **Skeleton**: `config.xml`, `index.html`, package + sign + install on a real TV; static screen.
2. **Sources**: add Xtream and M3U, persist, list categories.
3. **Live**: channel list + AVPlay + zap + favourites.
4. **VOD/Series**: posters, detail, seasons, resume.
5. **EPG**: short EPG on rows, then timeline grid.
6. **Polish**: hero, rails, focus pass, settings, error states, overscan check.

Each milestone must be verified on hardware before the next one is claimed done.

## 10. Validation checklist (per release)

- [ ] Installs from a clean TV in Developer Mode with a fresh certificate.
- [ ] Boots to the source list in < 5 s on the target panel.
- [ ] Xtream live channel plays with hardware decoding; audio/video in sync.
- [ ] HLS and MPEG-TS both tested; unsupported codec shows a readable error, not a black screen.
- [ ] M3U playlist with 1 000+ entries scrolls without dropped frames.
- [ ] Back never exits from a non-root view.
- [ ] Focus ring visible on every screen, including accent-filled buttons.
- [ ] No overscan clipping at 5 % safe area.
- [ ] Favourites/progress survive an app restart.
- [ ] No credentials or stream URLs in logs.

## 11. Out of scope / risks

- **TLS with self-signed provider certificates** cannot be bypassed in a browser engine — this is
  the single biggest functional risk versus the Windows build.
- Codec/container support differs per panel; only hardware tests count.
- Orsay and Tizen < 5.5 are not supported.
- No store distribution, no auto-update; updates are a manual re-install.
- Stalker/Ministra portals are **not** in the first TV milestone.

## 12. References

- `references/iptvnator/apps/tv/README.md` — TV feature set, compatibility table.
- `references/iptvnator/apps/tv/tizen/README.md` — Samsung signing and install steps.
- `references/iptvnator/docs/architecture/tv-application.md` — runtime/provider/persistence contract.
- `references/iptvnator/docs/development/tv-development.md` — build and device validation.
- `references/iptvnator/docs/development/tv-tizen-validation.md` — device acceptance results.
- `references/smartiptv-legacy/` — the old `.tmg` reference and its licence (archived, not a build input).
