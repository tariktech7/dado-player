# IPTVnator — reference analysis for Dado Player

Source: `references/iptvnator` (https://github.com/4gray/iptvnator, shallow clone).
Purpose: extract feature and design ideas, not code. IPTVnator is a separate product with its
own licence and brand; we take **concepts, interaction patterns and design tokens**, never
source files, assets or trademarks.

Date: 7 November 2026.

---

## 1. What the reference is

- Angular/Electron/PWA desktop app + a separate Preact TV client (`apps/tv`) packaged for LG
  webOS and Samsung Tizen, plus an Android/Fire-TV shell (`tv-shell`).
- Providers: M3U/M3U8, Xtream Codes, Stalker/Ministra portals.
- Design system: Angular Material as behaviour layer, IPTVnator "design kit" as look, driven by
  `--app-*` tokens in `apps/web/src/_theme-palettes.scss`.

## 2. Design tokens we adopted

Dark ("graphite") palette, copied as values into `src/theme.rs`:

| Role | IPTVnator | Dado constant |
| --- | --- | --- |
| Shell | `#0d0f12` | `BG_DARK` |
| Rail | `#111318` | `PANEL_DARK` |
| Content | `#161a22` | `CARD_DARK` |
| Widget | `#1e2330` | `WIDGET_DARK` |
| Card hover | `#252b3a` | `CARD_HOVER_DARK` |
| Text | `#e6e1e5` | `TEXT_DARK` |
| Muted | `#a1aabd` | `MUTED_DARK` |
| Selection ink | `#78adff` | `ACCENT` |
| Selection text | `#a9caff` | `accent_text()` |
| Focus ring | `#8cbaff` | `FOCUS` |
| Live | `#ff8f8f` | `LIVE` |
| Success | `#4ade80` | `OK` |
| Rating | `#ffcf5c` | `RATING` |
| Xtream / M3U | `#78adff` / `#fbbf24` | `PROVIDER_XTREAM` / `PROVIDER_M3U` |

Light theme follows their rule "light is flatter and cleaner": `#eef0f3` shell, white panels,
`#f2f4f7` cards.

Their typography is **DM Sans**. We kept Segoe UI / Source Sans: DM Sans ships only as a variable
font, and `ab_glyph` (egui's text stack) cannot select a weight axis — bundling it would silently
render bold as regular. Revisit if static instances are added to the bundle.

### Selection language

Their UI guideline: *"Selection uses surface tint and a full outline, without vertical accent
strips."* We adopted exactly that for the navigation rail (`src/ui/sidebar.rs`) and for the
playing row in Live TV: tinted surface (`ACCENT_DIM`) + 1 px accent outline, accent-tinted text.
Filled buttons use a darker accent (`ACCENT_FILL #2f7bff`) so white labels keep contrast.

## 3. Feature matrix

| Feature | IPTVnator | Dado before | Decision |
| --- | --- | --- | --- |
| M3U / M3U8 (file + URL) | yes | yes | parity |
| Xtream Codes | yes | yes | parity |
| Stalker / Ministra portal | yes | no | **skip** (large new provider, separate milestone) |
| Custom User-Agent per playlist | yes | no | planned |
| Built-in HTML5 player | yes | n/a (libmpv) | n/a |
| External player (MPV/VLC) | yes | n/a | n/a |
| Embedded MPV | yes | yes (libmpv) | parity |
| Radio streams (`radio="true"`) | yes | no | planned |
| EPG now/next per channel | yes | yes | parity |
| Multi-channel EPG grid | yes | **yes** (desktop; Android remains compact now/next) | **implemented** with six-hour timeline and time navigation |
| Catch-up / timeshift | yes | no | planned (provider-dependent) |
| Channel groups + search | yes | yes | parity |
| Channel-number selection | yes | **yes** | **implemented** |
| Movies/series posters, seasons | yes | yes | parity |
| Resume positions | yes | yes (+ prompt) | parity, we prompt |
| Download manager | yes | no | skip (desktop-only, out of scope) |
| "Recently added" rails | yes | **yes** | **implemented** for Xtream VOD with provider timestamps; omitted when unavailable |
| Global search (live+VOD+series) | yes | yes | parity |
| TMDB enrichment | yes | no | planned (opt-in, own API key) |
| Favourites (per source + global) | yes | yes | parity |
| Watch history | yes | yes | parity |
| Continue watching | yes | yes | parity |
| Command palette (Ctrl+K) | yes | **no** | **implemented** |
| Settings search | yes | no | **implemented** |
| Rating badge on posters | yes | text only | **implemented** |
| Light/dark/system theme | yes | yes | parity |
| 21 languages | yes | de/en strings | skip for now |
| Keyboard shortcuts | yes | partial | improved (Ctrl+K) |
| Desktop auto-update | yes | no | skip |
| Phone remote | yes | no | skip |
| Self-hosted PWA | yes | no | skip |
| Docker backend | yes | no | skip |
| TV client (webOS/Tizen) | yes | plan only | **plan rewritten** (`SmartTV/PORTING-PLAN.md`) |
| Android TV shell | yes | TV mode in APK | parity-ish |

## 4. What was implemented in this pass

1. **Graphite design refresh** — `src/theme.rs`: new palette, surface layers, focus/live/success/
   rating/provider tokens, theme-aware accent helpers.
2. **Selection language** — `src/ui/sidebar.rs`, `src/ui/live.rs`, `src/ui/player.rs`,
   `src/ui/mod.rs`: tinted surface + outline instead of solid accent fill; contrast-safe fills.
3. **Command palette** — new `src/ui/palette.rs`, wired to `Ctrl/Cmd+K` in `src/app.rs`.
   Searches views, theme toggle, logout, and channels/movies/series; ↑/↓/Enter/Esc; click support.
4. **Provider chip** in the rail (Xtream azure / M3U amber) so the active source is visible.
5. **Live programme progress bar** under the "Jetzt:" line, driven by the EPG we already fetch.
6. **Mobile token alignment** — `src/ui/mobile/tokens.rs` now uses the shared theme helpers, so
   Android matches the desktop palette.
7. **SmartTV plan rewritten** from IPTVnator's Tizen workflow; old `.tmg` reference archived to
   `references/smartiptv-legacy/`.
8. **Channel-number quick jump** — `src/ui/live.rs`: digits typed in Live TV accumulate in a
   floating badge and zap to that channel after a 1.1 s idle (or Enter). Ignored while a text
   field has focus; clears category/search filters so the target is always reachable.
9. **Settings search** — `src/ui/settings.rs`: one filter box over a single `SECTIONS` table
   (title + keywords) that drives both the match test and the hit counter, so headers and
   keywords can never drift apart. Empty result offers "Filter löschen".
10. **Poster rating pill** — `src/ui/mod.rs` (`poster_badge`) + `src/ui/vod.rs`: the rating moves
    from a weak text label onto the artwork as a `★ 7.8` pill; provider strings like
    `"7.8 / 10"`, `"0"` or `""` are normalised and dropped when unusable.
11. **Content-led Home** — `src/ui/home.rs`: featured provider title, continue watching,
    favourite live channels and timestamped new movies. Integrated into desktop navigation,
    Android page dispatch and the "Mehr" menu; profile login opens Home. Unknown timestamps are
    left out rather than guessed, and the feature CTA plays the selected item or uses the existing
    resume confirmation.
12. **Desktop Live EPG timeline** — `src/ui/live.rs`: six-hour time axis, programme blocks,
    current-time marker, active programme highlight and two-hour backward/forward controls.
    Xtream first tries `get_simple_data_table` for a fuller schedule and falls back to
    `get_short_epg`; malformed intervals are discarded. Android keeps its virtualized compact list.

## 5. Deliberately not copied

- Angular/Material/CDK code, SCSS partials, component markup, icons and screenshots.
- Their brand, name, colours as brand identity (we only take neutral surface/selection values).
- Their backend, Docker images, telemetry-free but cloud-sync features.
- Their Stalker implementation (large, provider-specific, needs MAG headers).

## 6. Next candidates (ordered by value/effort)

1. **EPG timeline grid on Android** — desktop timeline is implemented; consider a touch-appropriate
   schedule presentation without compromising the virtualized channel list.
2. **Series recently-added rail** — movie timestamps are integrated; verify provider timestamp
   conventions and extend the rail to series without inventing dates.
3. **Radio stream handling** (`radio="true"` → audio-only player surface, no video area).
4. **TMDB enrichment** as an opt-in with a user-supplied key.
