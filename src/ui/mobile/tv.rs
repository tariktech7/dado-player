//! 10-foot Android TV shell (MOBILE-BAUPLAN §23.10).
//!
//! Deliberately a *presentation* module, not a second app: it reads the same
//! `App` state and the same provider data as the phone shell, and reuses the
//! hand-painted primitives in `components.rs`. What differs is the navigation
//! model, following Leanback's `BrowseSupportFragment` / RowsSupportFragment
//! and the streaming apps listed in §23.3:
//!
//! * a left **headers column** instead of a bottom touch bar (a TV has no
//!   thumb to hop between tabs);
//! * horizontal **card rails** with artwork / logo tiles instead of long
//!   vertical text lists;
//! * a **hero / backdrop band** that describes the focused title;
//! * **spatial** D-pad focus (see `focus.rs`) and a real forward/back section
//!   history driven by `App::tv_goto` / `App::mobile_back`;
//! * 5 % overscan on every edge and a separate, larger type scale. No global
//!   `pixels_per_point` multiplier (§23.5) — only values are scaled.

use egui::{
    pos2, vec2, Align, Align2, Color32, Context, Frame, Rect, Response, Rounding, Sense, Stroke,
    Ui, Vec2,
};

use super::components::{self, Icon};
use super::focus;
use super::tokens;
use super::Layout;
use crate::app::App;
use crate::types::{Channel, LiveFormat, Series, Theme, TvMode, VodMovie};

/// Left-hand navigation. The third field is the description shown in the hero
/// while the header itself holds focus.
pub const HEADERS: [(Icon, &str, &str); 7] = [
    (Icon::Tv, "Live TV", "Sender, Logos und Jetzt / Danach"),
    (Icon::Film, "Filme", "Die Filmbibliothek des Anbieters"),
    (Icon::Series, "Serien", "Serien, Staffeln und Folgen"),
    (
        Icon::Play,
        "Fortsetzen",
        "Zuletzt gesehen und Wiedergabeposition",
    ),
    (Icon::StarOn, "Favoriten", "Gemerkte Sender und Titel"),
    (
        Icon::Search,
        "Suche",
        "Titel per Buchstaben ohne Tastatur finden",
    ),
    (
        Icon::Sliders,
        "Einstellungen",
        "Darstellung, TV-Modus und Profil",
    ),
];
pub const HEADER_COUNT: usize = HEADERS.len();

/// Compact labels for the narrow headers column (portrait / split-screen QA).
/// A real TV is landscape and uses `HEADERS`, but forcing TV mode on a phone
/// must not turn "Fortsetzen" into "Fort…".
const SHORT_HEADERS: [&str; HEADER_COUNT] = [
    "Live", "Filme", "Serien", "Weiter", "Favs", "Suche", "Setup",
];

// ---- 10-foot scale (§23.4 P1) ----
/// 5 % safe area on each edge, with a floor for small windows.
const OVERSCAN_MIN: f32 = 20.0;
const BRAND_H: f32 = 52.0;
const HEADER_ROW_H: f32 = 62.0;
const COLUMN_GAP: f32 = 20.0;
const RAIL_GAP: f32 = 16.0;
const MAX_CARDS: usize = 24;
const FS_BRAND: f32 = 26.0;
const FS_HEADER: f32 = 24.0;
const FS_HERO: f32 = 38.0;
const FS_HERO_SUB: f32 = 24.0;
const FS_RAIL: f32 = 24.0;
const FS_CARD: f32 = 22.0;
const FS_CARD_SUB: f32 = 18.0;
const FS_META: f32 = 18.0;

struct Geom {
    brand: Rect,
    headers: Rect,
    content: Rect,
}

fn geom(ctx: &Context) -> Geom {
    let r = ctx.screen_rect();
    let ox = (r.width() * 0.05).max(OVERSCAN_MIN);
    let oy = (r.height() * 0.05).max(OVERSCAN_MIN);
    let rail_w = ((r.width() - 2.0 * ox) * 0.26).clamp(160.0, 280.0);
    let content_x = ox + rail_w + COLUMN_GAP;
    let content_right = (r.max.x - ox).max(content_x + 120.0);
    let brand = Rect::from_min_size(pos2(ox, oy), vec2(content_right - ox, BRAND_H));
    let top = (brand.bottom() + 14.0).min(r.max.y - oy - 60.0);
    let bottom = (r.max.y - oy).max(top + 60.0);
    Geom {
        brand,
        headers: Rect::from_min_max(pos2(ox, top), pos2(ox + rail_w, bottom)),
        content: Rect::from_min_max(pos2(content_x, top), pos2(content_right, bottom)),
    }
}

/// One control's description for the hero band.
#[derive(Clone, Default)]
struct Meta {
    title: String,
    sub: String,
    detail: String,
    progress: Option<f32>,
}

#[derive(Clone)]
enum Act {
    /// A progress key resolved through `App::resolve_play`.
    PlayKey(String),
    /// Open the episodes rail for a series (a forward step, BACK clears it).
    Series(u32),
    Letter(char),
    SearchBackspace,
    SearchClear,
    Theme(Theme),
    TvMode(TvMode),
    #[cfg(target_os = "android")]
    Player(crate::types::PlayerBackend),
    LiveFmt(LiveFormat),
    Logout,
}

#[derive(Clone)]
struct Card {
    key: String,
    url: String,
    title: String,
    sub: String,
    meta: String,
    progress: Option<f32>,
    act: Act,
}

impl Card {
    fn meta(&self) -> Meta {
        Meta {
            title: self.title.clone(),
            sub: if self.sub.is_empty() {
                self.meta.clone()
            } else {
                self.sub.clone()
            },
            detail: self.meta.clone(),
            progress: self.progress,
        }
    }
}

struct Rail {
    title: String,
    wide: bool,
    cards: Vec<Card>,
}

/// Entry point from `ui::mobile::show()`.
pub fn show(app: &mut App, ctx: &Context, _l: &Layout) {
    let g = geom(ctx);
    let bg = ctx.style().visuals.panel_fill;

    egui::CentralPanel::default()
        .frame(Frame::none().fill(bg))
        .show(ctx, |ui| {
            ui.painter().rect_filled(ctx.screen_rect(), 0.0, bg);
            paint_brand(ui, g.brand, app);

            let mut metas: Vec<Meta> = Vec::new();
            let mut hui = ui.child_ui(g.headers, egui::Layout::top_down(Align::LEFT));
            draw_headers(app, &mut hui, &mut metas);

            let mut cui = ui.child_ui(g.content, egui::Layout::top_down(Align::LEFT));
            if app.view == crate::app::View::Live {
                crate::ui::live::show(app, &mut cui);
                return;
            }
            let hero_rect = draw_content(app, &mut cui, &g, &mut metas);

            // Painted last: only now are all `metas` (in registration order)
            // known, so `focus::index()` can pick the focused card's copy.
            paint_hero(ui, hero_rect, &metas, app);
        });
}

fn paint_brand(ui: &Ui, rect: Rect, app: &App) {
    let p = ui.painter();
    // Brand mark: accent square with a play triangle, painted rather than typed
    // so it cannot turn into a missing glyph.
    let mark = Rect::from_min_size(pos2(rect.left(), rect.center().y - 17.0), Vec2::splat(34.0));
    p.rect_filled(mark, Rounding::same(9.0), tokens::accent_fill());
    p.add(egui::Shape::convex_polygon(
        vec![
            pos2(mark.left() + 12.0, mark.top() + 8.0),
            pos2(mark.right() - 9.0, mark.center().y),
            pos2(mark.left() + 12.0, mark.bottom() - 8.0),
        ],
        Color32::WHITE,
        Stroke::NONE,
    ));
    components::draw_line(
        ui,
        pos2(mark.right() + 14.0, rect.center().y),
        "Dado Player",
        tokens::bold(FS_BRAND),
        tokens::text(ui),
        220.0,
    );
    let profile = app
        .active_profile
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_default();
    if !profile.is_empty() {
        let g = components::fit(
            ui,
            &profile,
            tokens::medium(FS_META),
            tokens::muted(ui),
            220.0,
        );
        p.galley(
            pos2(
                rect.right() - g.size().x,
                rect.center().y - g.size().y * 0.5,
            ),
            g,
            tokens::muted(ui),
        );
    }
}

fn draw_headers(app: &mut App, ui: &mut Ui, metas: &mut Vec<Meta>) {
    let w = ui.available_width();
    let narrow = w < 210.0;
    let fs = if narrow { 20.0 } else { FS_HEADER };
    let mut activate: Option<usize> = None;
    for (i, (ic, label, desc)) in HEADERS.iter().enumerate() {
        let (rect, resp) = ui.allocate_exact_size(vec2(w, HEADER_ROW_H), Sense::click());
        let selected = i == app.tv_nav;
        let focused = focus::next_is_focused();
        let inner = rect.shrink(2.0);
        if selected {
            ui.painter()
                .rect_filled(inner, Rounding::same(12.0), tokens::accent_soft(ui));
            ui.painter().rect_filled(
                Rect::from_min_size(
                    pos2(inner.left(), inner.top() + 12.0),
                    vec2(4.0, inner.height() - 24.0),
                ),
                Rounding::same(2.0),
                tokens::accent(),
            );
        } else if resp.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(inner, Rounding::same(12.0), tokens::pressed(ui));
        }
        let color = if selected || focused {
            tokens::text(ui)
        } else {
            tokens::muted(ui)
        };
        components::icon(
            ui,
            Rect::from_center_size(pos2(rect.left() + 30.0, rect.center().y), Vec2::splat(26.0)),
            *ic,
            if selected {
                tokens::accent_text(ui)
            } else {
                color
            },
        );
        components::draw_line(
            ui,
            pos2(rect.left() + 56.0, rect.center().y),
            if narrow { SHORT_HEADERS[i] } else { label },
            if selected {
                tokens::bold(fs)
            } else {
                tokens::medium(fs)
            },
            color,
            (w - 62.0).max(40.0),
        );
        metas.push(Meta {
            title: label.to_string(),
            sub: desc.to_string(),
            detail: String::new(),
            progress: None,
        });
        if focus::clicked(ui, &resp) {
            activate = Some(i);
        }
    }
    if let Some(i) = activate {
        app.tv_goto(i);
    }
}

/// Reserve the hero band, then draw the rails for the selected section.
/// Returns the hero rect so it can be painted after all cards registered.
fn draw_content(app: &mut App, ui: &mut Ui, g: &Geom, metas: &mut Vec<Meta>) -> Rect {
    // Tall enough for title + subtitle + the bottom hint/progress zone. A
    // shorter band used to paint the hint on top of the subtitle (QA finding).
    let hero_h = (g.content.height() * 0.36).clamp(150.0, 200.0);
    let (hero_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), hero_h), Sense::hover());
    ui.add_space(10.0);
    let rails_h = ui.available_height().max(70.0);

    let mut actions: Vec<Act> = Vec::new();
    egui::ScrollArea::vertical()
        .id_source("tv-rails")
        .max_height(rails_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let rails = build_rails(app);
            for (i, rail) in rails.iter().enumerate() {
                draw_rail(app, ui, rail, i, metas, &mut actions);
            }
            ui.add_space(24.0);
        });
    for a in actions {
        apply_action(app, a);
    }
    hero_rect
}

fn draw_rail(
    app: &mut App,
    ui: &mut Ui,
    rail: &Rail,
    rail_no: usize,
    metas: &mut Vec<Meta>,
    actions: &mut Vec<Act>,
) {
    if rail.cards.is_empty() {
        return;
    }
    let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
    components::draw_line(
        ui,
        pos2(hr.left() + 2.0, hr.center().y),
        &rail.title,
        tokens::bold(FS_RAIL),
        tokens::text(ui),
        hr.width(),
    );
    let (cw, art_h, text_h) = if rail.wide {
        (176.0, 100.0, 46.0)
    } else {
        (122.0, 164.0, 46.0)
    };
    egui::ScrollArea::horizontal()
        .id_source(("tv-rail", app.tv_nav, rail_no))
        .max_height(art_h + text_h)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ui.add_space(2.0);
                for c in &rail.cards {
                    let focused = focus::next_is_focused();
                    let resp = tv_card(app, ui, c, cw, art_h, text_h, focused);
                    metas.push(c.meta());
                    if focus::clicked(ui, &resp) {
                        actions.push(c.act.clone());
                    }
                }
                ui.add_space(2.0);
            });
        });
    ui.add_space(RAIL_GAP);
}

/// One card. Only the *painted* artwork grows while focused; the allocated hit
/// rect stays put, so neither the focus geometry nor the rail spacing moves.
fn tv_card(
    app: &mut App,
    ui: &mut Ui,
    c: &Card,
    w: f32,
    art_h: f32,
    text_h: f32,
    focused: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, art_h + text_h), Sense::click());
    let base = Rect::from_min_size(rect.min, vec2(w, art_h));
    let art = if focused {
        Rect::from_center_size(base.center(), base.size() * 1.05)
    } else {
        base
    };
    let p = ui.painter();
    p.rect_filled(art, Rounding::same(10.0), tokens::surface(ui));
    p.rect_stroke(
        art,
        Rounding::same(10.0),
        Stroke::new(1.0_f32, tokens::hairline(ui)),
    );

    if !c.url.is_empty() {
        app.request_image(c.key.clone(), c.url.clone());
        if let Some(tex) = app.image(&c.key) {
            p.image(
                tex.id(),
                art,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            letter(ui, art, &c.title, art_h * 0.38);
        }
    } else {
        // Logo / cover missing is the normal case for many IPTV panels. A
        // letter tile reads as a channel logo from across the room; an empty
        // rectangle reads as a broken image.
        letter(ui, art, &c.title, art_h * 0.38);
    }

    if let Some(f) = c.progress {
        let bar = Rect::from_min_size(
            pos2(art.left() + 8.0, art.bottom() - 9.0),
            vec2((art.width() - 16.0) * f.clamp(0.0, 1.0), 4.0),
        );
        p.rect_filled(
            Rect::from_min_size(
                pos2(art.left() + 8.0, art.bottom() - 9.0),
                vec2(art.width() - 16.0, 4.0),
            ),
            Rounding::same(2.0),
            Color32::from_white_alpha(30),
        );
        p.rect_filled(bar, Rounding::same(2.0), tokens::accent());
    }

    let ty = rect.min.y + art_h + 14.0;
    components::draw_line(
        ui,
        pos2(rect.left() + 2.0, ty),
        &c.title,
        tokens::bold(FS_CARD),
        tokens::text(ui),
        w - 4.0,
    );
    if !c.sub.is_empty() {
        components::draw_line(
            ui,
            pos2(rect.left() + 2.0, ty + 22.0),
            &c.sub,
            tokens::reg(FS_CARD_SUB),
            tokens::muted(ui),
            w - 4.0,
        );
    }
    resp
}

fn letter(ui: &Ui, rect: Rect, title: &str, size: f32) {
    let ch = title
        .chars()
        .next()
        .unwrap_or('•')
        .to_uppercase()
        .to_string();
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ch,
        tokens::bold(size.clamp(24.0, 62.0)),
        tokens::muted(ui),
    );
}

fn paint_hero(ui: &Ui, rect: Rect, metas: &[Meta], app: &App) {
    let p = ui.painter();
    let r = rect.shrink(2.0);
    p.rect_filled(r, Rounding::same(14.0), tokens::surface(ui));
    p.rect_filled(
        Rect::from_min_size(r.min, vec2(6.0, r.height())),
        Rounding::same(3.0),
        tokens::accent(),
    );
    let x = r.left() + 26.0;
    let max_w = (r.width() - 52.0).max(80.0);

    let fallback = Meta {
        title: HEADERS[app.tv_nav].1.to_string(),
        sub: HEADERS[app.tv_nav].2.to_string(),
        detail: String::new(),
        progress: None,
    };
    let m = focus::index()
        .and_then(|i| metas.get(i))
        .unwrap_or(&fallback);

    // Reserve the bottom zone FIRST. The hint and the optional progress bar
    // are pinned there, and every description line stops before it. With a
    // short band (small TV, big overscan) the old absolute offsets printed the
    // hint directly on top of the subtitle.
    let hint = "Pfeiltasten: navigieren   ·   OK: auswählen   ·   Zurück: vorheriges Menü";
    let hint_g = components::fit(ui, hint, tokens::reg(FS_META), tokens::muted(ui), max_w);
    let hint_h = hint_g.size().y;
    let hint_y = (r.bottom() - 12.0 - hint_h).max(r.top() + 40.0);
    let progress_y = hint_y - 12.0;
    let desc_bottom = if m.progress.is_some() {
        progress_y - 8.0
    } else {
        hint_y - 8.0
    };

    let mut y = r.top() + 14.0;
    let title_g = components::fit(ui, &m.title, tokens::bold(FS_HERO), tokens::text(ui), max_w);
    p.galley(pos2(x, y), title_g.clone(), tokens::text(ui));
    y += title_g.size().y + 6.0;

    let sub_g = components::fit(
        ui,
        &m.sub,
        tokens::medium(FS_HERO_SUB),
        tokens::muted(ui),
        max_w,
    );
    if y + sub_g.size().y <= desc_bottom {
        p.galley(pos2(x, y), sub_g.clone(), tokens::muted(ui));
        y += sub_g.size().y + 6.0;

        if !m.detail.is_empty() && m.detail != m.sub {
            let d = components::fit(
                ui,
                &m.detail,
                tokens::reg(FS_META),
                tokens::muted(ui),
                max_w,
            );
            if y + d.size().y <= desc_bottom {
                p.galley(pos2(x, y), d, tokens::muted(ui));
            }
        }
    }

    p.galley(pos2(x, hint_y), hint_g, tokens::muted(ui));
    if let Some(f) = m.progress {
        let w = max_w.min(360.0);
        p.rect_filled(
            Rect::from_min_size(pos2(x, progress_y), vec2(w, 4.0)),
            Rounding::same(2.0),
            Color32::from_white_alpha(30),
        );
        p.rect_filled(
            Rect::from_min_size(pos2(x, progress_y), vec2(w * f.clamp(0.0, 1.0), 4.0)),
            Rounding::same(2.0),
            tokens::accent(),
        );
    }
}

// ------------------------------------------------------------- rail data ----

fn build_rails(app: &mut App) -> Vec<Rail> {
    if !app.library.loaded {
        return vec![Rail {
            title: "Bibliothek wird geladen…".to_string(),
            wide: true,
            cards: Vec::new(),
        }];
    }
    match app.tv_nav {
        0 => live_rails(app),
        1 => movie_rails(app),
        2 => series_rails(app),
        3 => continue_rails(app),
        4 => favorite_rails(app),
        5 => search_rails(app),
        _ => settings_rail(app),
    }
}

fn channel_card(app: &App, c: &Channel) -> Card {
    let now = chrono::Utc::now().timestamp();
    let cur = app
        .epg_for(c.id)
        .and_then(|v| v.iter().find(|e| e.start <= now && e.end > now));
    let (sub, progress) = match cur {
        Some(e) => {
            let span = (e.end - e.start) as f32;
            let f = if span > 0.0 {
                Some(((now - e.start) as f32 / span).clamp(0.0, 1.0))
            } else {
                None
            };
            (format!("Jetzt: {}", e.title), f)
        }
        None => (format!("Kanal {}", c.num), None),
    };
    Card {
        key: format!("logo:{}", c.id),
        url: c.logo.clone(),
        title: c.name.clone(),
        sub,
        meta: format!("Live · Kanal {}", c.num),
        progress,
        act: Act::PlayKey(format!("live:{}", c.id)),
    }
}

fn live_rails(app: &mut App) -> Vec<Rail> {
    let ids: Vec<u32> = app
        .library
        .channels
        .iter()
        .take(MAX_CARDS)
        .map(|c| c.id)
        .collect();
    for id in ids {
        app.request_epg(id);
    }
    let mut rails = vec![Rail {
        title: "Alle Sender".to_string(),
        wide: true,
        cards: app
            .library
            .channels
            .iter()
            .take(MAX_CARDS)
            .map(|c| channel_card(app, c))
            .collect(),
    }];
    let cats: Vec<(String, String)> = app
        .library
        .live_categories
        .iter()
        .take(3)
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();
    for (id, name) in cats {
        let cards: Vec<Card> = app
            .library
            .channels
            .iter()
            .filter(|c| c.category_id == id)
            .take(12)
            .map(|c| channel_card(app, c))
            .collect();
        if !cards.is_empty() {
            rails.push(Rail {
                title: name,
                wide: true,
                cards,
            });
        }
    }
    rails
}

fn movie_card(m: &VodMovie) -> Card {
    Card {
        key: format!("cover:{}", m.id),
        url: m.cover.clone(),
        title: m.name.clone(),
        sub: if m.rating.is_empty() {
            String::new()
        } else {
            format!("IMDb {}", m.rating)
        },
        meta: "Film · mit OK starten".to_string(),
        progress: None,
        act: Act::PlayKey(format!("vod:{}", m.id)),
    }
}

fn movie_rails(app: &mut App) -> Vec<Rail> {
    let mut rails = vec![Rail {
        title: "Alle Filme".to_string(),
        wide: false,
        cards: app
            .library
            .movies
            .iter()
            .take(MAX_CARDS)
            .map(movie_card)
            .collect(),
    }];
    let cats: Vec<(String, String)> = app
        .library
        .vod_categories
        .iter()
        .take(3)
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();
    for (id, name) in cats {
        let cards: Vec<Card> = app
            .library
            .movies
            .iter()
            .filter(|m| m.category_id == id)
            .take(12)
            .map(movie_card)
            .collect();
        if !cards.is_empty() {
            rails.push(Rail {
                title: name,
                wide: false,
                cards,
            });
        }
    }
    rails
}

fn series_card(s: &Series) -> Card {
    Card {
        key: format!("scov:{}", s.id),
        url: s.cover.clone(),
        title: s.name.clone(),
        sub: String::new(),
        meta: "Serie · OK öffnet die Folgen".to_string(),
        progress: None,
        act: Act::Series(s.id),
    }
}

fn series_rails(app: &mut App) -> Vec<Rail> {
    let mut rails = Vec::new();
    if let Some(sid) = app.tv_series {
        app.request_series(sid);
        let eps = app.series_episodes.get(&sid).cloned().unwrap_or_default();
        let cards: Vec<Card> = eps
            .iter()
            .take(MAX_CARDS)
            .map(|e| Card {
                key: format!("scov:{}", sid),
                url: String::new(),
                title: format!("S{} F{} · {}", e.season, e.episode, e.title),
                sub: String::new(),
                meta: "Folge · mit OK starten".to_string(),
                progress: None,
                act: Act::PlayKey(format!("ep:{}:{}", sid, e.id)),
            })
            .collect();
        if !cards.is_empty() {
            rails.push(Rail {
                title: "Folgen · Zurück schließt diese Reihe".to_string(),
                wide: true,
                cards,
            });
        }
    }
    rails.push(Rail {
        title: "Alle Serien".to_string(),
        wide: false,
        cards: app
            .library
            .series
            .iter()
            .take(MAX_CARDS)
            .map(series_card)
            .collect(),
    });
    let cats: Vec<(String, String)> = app
        .library
        .series_categories
        .iter()
        .take(2)
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();
    for (id, name) in cats {
        let cards: Vec<Card> = app
            .library
            .series
            .iter()
            .filter(|s| s.category_id == id)
            .take(12)
            .map(series_card)
            .collect();
        if !cards.is_empty() {
            rails.push(Rail {
                title: name,
                wide: false,
                cards,
            });
        }
    }
    rails
}

fn history_card(app: &App, key: &str, title: &str, sub: &str) -> Card {
    let pos = app.get_progress(key);
    let detail = if pos > 1 {
        format!("Weiter bei {}", crate::ui::fmt_time(pos as f64))
    } else {
        sub.to_string()
    };
    let (img_key, img_url) = image_for_key(app, key);
    Card {
        key: img_key,
        url: img_url,
        title: title.to_string(),
        sub: detail,
        meta: "Zuletzt gesehen · mit OK fortsetzen".to_string(),
        progress: if pos > 1 {
            Some((pos as f32 / 5400.0).clamp(0.02, 1.0))
        } else {
            None
        },
        act: Act::PlayKey(key.to_string()),
    }
}

fn image_for_key(app: &App, key: &str) -> (String, String) {
    if let Some(rest) = key.strip_prefix("live:") {
        if let Ok(id) = rest.parse::<u32>() {
            if let Some(c) = app.library.channels.iter().find(|c| c.id == id) {
                return (format!("logo:{}", id), c.logo.clone());
            }
        }
    }
    if let Some(rest) = key.strip_prefix("vod:") {
        if let Ok(id) = rest.parse::<u32>() {
            if let Some(m) = app.library.movies.iter().find(|m| m.id == id) {
                return (format!("cover:{}", id), m.cover.clone());
            }
        }
    }
    if let Some(rest) = key.strip_prefix("ep:") {
        if let Some(sid) = rest.split(':').next().and_then(|s| s.parse::<u32>().ok()) {
            if let Some(s) = app.library.series.iter().find(|s| s.id == sid) {
                return (format!("scov:{}", sid), s.cover.clone());
            }
        }
    }
    (String::new(), String::new())
}

fn continue_rails(app: &mut App) -> Vec<Rail> {
    let entries: Vec<(String, String, String)> = app
        .history
        .iter()
        .take(MAX_CARDS)
        .map(|h| (h.key.clone(), h.title.clone(), h.sub.clone()))
        .collect();
    let cards: Vec<Card> = entries
        .iter()
        .map(|(k, t, s)| history_card(app, k, t, s))
        .collect();
    if cards.is_empty() {
        return vec![Rail {
            title: "Noch nichts gesehen – was du startest, erscheint hier".to_string(),
            wide: true,
            cards: Vec::new(),
        }];
    }
    vec![Rail {
        title: "Zuletzt gesehen".to_string(),
        wide: true,
        cards,
    }]
}

fn favorite_rails(app: &mut App) -> Vec<Rail> {
    let mut keys: Vec<String> = app.favorites.iter().cloned().collect();
    keys.sort();
    keys.truncate(MAX_CARDS);
    let cards: Vec<Card> = keys
        .iter()
        .filter_map(|k| {
            app.describe_key(k)
                .map(|(title, sub)| history_card(app, k, &title, &sub))
        })
        .collect();
    if cards.is_empty() {
        return vec![Rail {
            title: "Noch keine Favoriten".to_string(),
            wide: true,
            cards: Vec::new(),
        }];
    }
    vec![Rail {
        title: "Gemerkte Titel".to_string(),
        wide: true,
        cards,
    }]
}

fn search_rails(app: &mut App) -> Vec<Rail> {
    let mut keys: Vec<Card> = ('A'..='Z')
        .map(|ch| Card {
            key: String::new(),
            url: String::new(),
            title: ch.to_string(),
            sub: String::new(),
            meta: "Buchstabe hinzufügen".to_string(),
            progress: None,
            act: Act::Letter(ch),
        })
        .collect();
    keys.push(Card {
        key: String::new(),
        url: String::new(),
        title: "Löschen".to_string(),
        sub: String::new(),
        meta: "Letzten Buchstaben entfernen".to_string(),
        progress: None,
        act: Act::SearchBackspace,
    });
    keys.push(Card {
        key: String::new(),
        url: String::new(),
        title: "Leeren".to_string(),
        sub: String::new(),
        meta: "Suchtext löschen".to_string(),
        progress: None,
        act: Act::SearchClear,
    });

    let q = app.tv_search.trim().to_lowercase();
    let mut results: Vec<Card> = Vec::new();
    if q.is_empty() {
        for c in app.library.channels.iter().take(12) {
            results.push(channel_card(app, c));
        }
    } else {
        for c in app.library.channels.iter() {
            if c.name.to_lowercase().contains(&q) {
                results.push(channel_card(app, c));
            }
            if results.len() >= MAX_CARDS {
                break;
            }
        }
        for m in app.library.movies.iter() {
            if m.name.to_lowercase().contains(&q) {
                results.push(movie_card(m));
            }
            if results.len() >= MAX_CARDS {
                break;
            }
        }
        for s in app.library.series.iter() {
            if s.name.to_lowercase().contains(&q) {
                results.push(series_card(s));
            }
            if results.len() >= MAX_CARDS {
                break;
            }
        }
    }

    vec![
        Rail {
            title: if q.is_empty() {
                "Buchstaben wählen".to_string()
            } else {
                format!("Suche nach „{}“", app.tv_search)
            },
            wide: true,
            cards: keys,
        },
        Rail {
            title: if q.is_empty() {
                "Alle Titel".to_string()
            } else {
                format!("{} Treffer", results.len())
            },
            wide: true,
            cards: results,
        },
    ]
}

fn option_card(label: &str, selected: bool, act: Act) -> Card {
    Card {
        key: String::new(),
        url: String::new(),
        title: label.to_string(),
        sub: if selected {
            "Aktiv".to_string()
        } else {
            String::new()
        },
        meta: "Mit OK übernehmen".to_string(),
        progress: None,
        act,
    }
}

fn settings_rail(app: &mut App) -> Vec<Rail> {
    let mut tv_opts = Vec::new();
    for m in TvMode::all() {
        tv_opts.push(option_card(
            m.label(),
            app.settings.tv_mode == m,
            Act::TvMode(m),
        ));
    }
    let mut theme_opts = Vec::new();
    for t in Theme::all() {
        theme_opts.push(option_card(
            t.label(),
            app.settings.theme == t,
            Act::Theme(t),
        ));
    }
    // A TV often has no MX Player at all. Without a rail here the new default
    // would be a dead end on exactly the device class this shell targets; the
    // user must be able to pick VLC / the system chooser / the built-in player
    // with the remote they actually hold.
    #[cfg(target_os = "android")]
    let mut player_opts = Vec::new();
    #[cfg(target_os = "android")]
    for p in crate::types::PlayerBackend::all() {
        player_opts.push(option_card(
            p.label(),
            app.settings.player_backend == p,
            Act::Player(p),
        ));
    }
    let mut fmt_opts = Vec::new();
    for f in LiveFormat::all() {
        fmt_opts.push(option_card(
            f.label(),
            app.settings.live_format == f,
            Act::LiveFmt(f),
        ));
    }
    vec![
        #[cfg(target_os = "android")]
        Rail {
            title: "Player (Standard: MX Player)".to_string(),
            wide: true,
            cards: player_opts,
        },
        Rail {
            title: "Live-Format (nur Xtream)".to_string(),
            wide: true,
            cards: fmt_opts,
        },
        Rail {
            title: "TV-Modus (Fernseh-Oberfläche)".to_string(),
            wide: true,
            cards: tv_opts,
        },
        Rail {
            title: "Design".to_string(),
            wide: true,
            cards: theme_opts,
        },
        Rail {
            title: "Profil".to_string(),
            wide: true,
            cards: vec![Card {
                key: String::new(),
                url: String::new(),
                title: "Abmelden".to_string(),
                sub: String::new(),
                meta: "Zurück zur Anbieter-Anmeldung".to_string(),
                progress: None,
                act: Act::Logout,
            }],
        },
    ]
}

fn apply_action(app: &mut App, act: Act) {
    match act {
        Act::PlayKey(key) => {
            if let Some(item) = app.resolve_play(&key) {
                app.play(item);
            }
        }
        Act::Series(id) => {
            app.request_series(id);
            app.tv_series = Some(id);
            // The episodes rail is now the first rail after the headers.
            focus::request_index(HEADER_COUNT);
        }
        Act::Letter(ch) => app.tv_search.push(ch.to_ascii_lowercase()),
        Act::SearchBackspace => {
            app.tv_search.pop();
        }
        Act::SearchClear => app.tv_search.clear(),
        Act::Theme(t) => {
            app.settings.theme = t;
            app.save_settings_now();
        }
        Act::TvMode(m) => {
            app.settings.tv_mode = m;
            app.save_settings_now();
            // `App::update` resolves this again next frame; doing it here too
            // keeps BACK and canGoBack correct on the frame of the change.
            app.tv = m.resolve(crate::platform::tv_device());
        }
        #[cfg(target_os = "android")]
        Act::Player(p) => {
            app.settings.player_backend = p;
            app.save_settings_now();
        }
        Act::LiveFmt(f) => {
            if app.settings.live_format != f {
                app.settings.live_format = f;
                // Rebuild the already-loaded Xtream channel URLs in place;
                // M3U links are tokenised and must never be rewritten.
                app.apply_live_format();
                app.save_settings_now();
            }
        }
        Act::Logout => app.logout(),
    }
}
