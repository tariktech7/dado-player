//! Touch-first phone UI.
//!
//! The desktop build keeps its TV-style layout (sidebar + side panels). On
//! Android — and in any small desktop window — this module takes over: bottom
//! navigation, full-screen pages, 64 dp rows, 48 dp hit areas, keyboard-aware
//! forms. Same `App` state and the same backend, only a different presentation.

pub mod components;
pub mod focus;
pub mod tokens;
pub mod tv;

mod continue_watching;
mod favorites;
mod live;
mod login;
mod more;
mod movies;
mod player;
mod search;
mod series;
mod settings;
#[cfg(any(target_os = "ios", target_os = "android"))]
mod update;

use egui::{Align, Layout as EguiLayout, Rect};
use egui::{Context, Frame, TopBottomPanel};

use crate::app::{App, View};
use components::Icon;

/// Usable height below which the vertical budget forces list mode.
///
/// Measured, not guessed: a 411x914 dp portrait phone has ~860 dp of usable
/// height, the same phone in landscape has ~253 dp. Anything in between is a
/// tablet in landscape (≥ 600 dp) or a resized desktop window, and those keep
/// the full hero. 400 sits in the empty band between the two: it cannot fire on
/// any phone in portrait, and it fires on every phone in landscape.
const COMPACT_MAX_H: f32 = 400.0;

/// Width (usable, after insets) that separates the compact bucket from medium.
/// Below it the device is a phone; at/above it we are in tablet / open-foldable
/// territory (MOBILE-BAUPLAN §23.7).
const BUCKET_MEDIUM_W: f32 = 600.0;
/// Width (usable, after insets) that separates medium from expanded.
const BUCKET_EXPANDED_W: f32 = 840.0;
/// Raw window size below which the *desktop* build uses the touch UI. This only
/// decides which UI renders (phone vs desktop/TV) — internal, inset-aware
/// geometry lives in `Layout::new` via the buckets below.
const TOUCH_UI_MAX_W: f32 = 860.0;
const TOUCH_UI_MAX_H: f32 = 600.0;

/// Width class derived from the *usable* area after insets (never raw screen
/// size). Drives `pad`, column count and prose width (MOBILE-BAUPLAN §23.7).
/// Height is handled separately by `compact` (`COMPACT_MAX_H`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    Compact,
    Medium,
    Expanded,
}

/// Geometry of the current viewport.
///
/// `Clone` exists for screens that hand a *narrower* copy to a `CentralPanel`
/// (e.g. the login form beside its landscape action column) while keeping all
/// the inset-derived padding/buckets of the original.
#[derive(Clone)]
pub struct Layout {
    pub w: f32,
    pub h: f32,
    pub pad: f32,
    pub gap: f32,
    pub portrait: bool,
    /// Status-bar / cutout inset in points.
    pub top_inset: f32,
    /// Gesture-bar / navigation inset in points.
    pub bottom_inset: f32,
    /// Height of the soft keyboard in points (0 when closed).
    pub ime: f32,
    /// Cutout / system-bar inset on the long edges (landscape).
    pub side: f32,
    pub appbar_h: f32,
    /// Short landscape phone: vertical space is precious.
    ///
    /// Stored rather than recomputed. The threshold used to exist twice — once
    /// as a local in `new()` and once inside `compact_height()` — and a
    /// duplicated condition is how a value drifts between the two places that
    /// are supposed to agree.
    compact: bool,
    /// Width class from usable width after insets (MOBILE-BAUPLAN §23.7).
    bucket: Bucket,
}

impl Layout {
    pub fn new(ctx: &Context) -> Self {
        let r = ctx.screen_rect();
        let ppp = ctx.pixels_per_point().max(0.1);
        let ins = crate::platform::insets();
        let top = (ins.top / ppp).clamp(0.0, r.height() * 0.2);
        let ime = (ins.ime / ppp).clamp(0.0, r.height());
        let bottom = (ins.bottom / ppp).clamp(0.0, r.height());
        let side = ins.left.max(ins.right) / ppp;
        // Breakpoint on *usable* height: a 412 dp landscape phone loses ~48 dp
        // to the navigation bar and must switch to list mode.
        let compact = r.height() - top - bottom < COMPACT_MAX_H;
        // Usable width AFTER the side insets — never the raw screen size. A
        // landscape phone and an open foldable report very different raw widths;
        // buckets must come from the area content can actually use
        // (MOBILE-BAUPLAN §23.7).
        let usable_w = (r.width() - 2.0 * side).max(0.0);
        let bucket = if usable_w < BUCKET_MEDIUM_W {
            Bucket::Compact
        } else if usable_w < BUCKET_EXPANDED_W {
            Bucket::Medium
        } else {
            Bucket::Expanded
        };
        // Compact keeps the small-phone tweak (< 360 dp → 12 dp) so the tiniest
        // handsets keep their margin; medium/expanded use the spec `PAD`. `side`
        // is always added so a landscape cutout never eats the page margin.
        let base_pad = match bucket {
            Bucket::Compact => {
                if r.width() < 360.0 {
                    12.0
                } else {
                    tokens::PAD
                }
            }
            Bucket::Medium => 24.0,
            Bucket::Expanded => 32.0,
        };
        Layout {
            w: r.width(),
            h: r.height(),
            pad: base_pad + side,
            gap: tokens::GAP,
            portrait: r.height() >= r.width(),
            top_inset: top,
            bottom_inset: bottom,
            ime,
            side,
            appbar_h: if compact {
                tokens::APPBAR_H_LAND
            } else {
                tokens::APPBAR_H
            },
            compact,
            bucket,
        }
    }

    /// Width available for content.
    pub fn content_w(&self) -> f32 {
        (self.w - 2.0 * self.pad).max(1.0)
    }

    /// Number of columns for cards at least `min_w` wide.
    ///
    /// Clamped per width bucket (MOBILE-BAUPLAN §23.7): compact caps at 3 so a
    /// small phone never shows posters too small to read, while medium/expanded
    /// may use up to 5. The lower bound is 1 rather than 2 so a genuinely narrow
    /// window can never overflow its own content width.
    pub fn cols(&self, min_w: f32) -> usize {
        let max = if self.bucket == Bucket::Compact { 3 } else { 5 };
        (((self.content_w() + self.gap) / (min_w + self.gap)).floor() as usize).clamp(1, max)
    }

    /// Maximum prose measure, bucket-aware. Per MOBILE-BAUPLAN §23.7 the limit is
    /// `PROSE_MAX_W` in every bucket (a wide screen does not get wider lines),
    /// but routing it through `Layout` keeps the decision in one place.
    pub fn prose_w(&self) -> f32 {
        tokens::PROSE_MAX_W
    }

    /// Short landscape phone: vertical space is precious.
    pub fn compact_height(&self) -> bool {
        self.compact
    }
}

/// Should the touch UI be used?
pub fn active(ctx: &Context) -> bool {
    if std::env::var("IPTV_DESKTOP_UI").is_ok() {
        return false;
    }
    if std::env::var("IPTV_MOBILE").is_ok() {
        return true;
    }
    if cfg!(any(target_os = "android", target_os = "ios")) {
        return true;
    }
    // On desktop, use the touch UI for any window that reads as a phone or small
    // tablet. Thresholds are raw (no insets): this only picks *which* UI renders,
    // not its internal layout — buckets (Layout::new) own the inset-aware geometry.
    let s = ctx.screen_rect().size();
    s.x < TOUCH_UI_MAX_W || s.y < TOUCH_UI_MAX_H
}

/// Entry point, called from `App::draw`.
pub fn show(app: &mut App, ctx: &Context) {
    let mut l = Layout::new(ctx);
    if app.tv {
        // 5 % overscan on every edge (MOBILE-BAUPLAN §23.4 P1). The TV shell
        // computes its own geometry, but login and the player still use `l`,
        // and on a TV they must not put a control in the overscan band either.
        let r = ctx.screen_rect();
        let ox = (r.width() * 0.05).max(20.0);
        let oy = (r.height() * 0.05).max(20.0);
        l.pad = l.pad.max(ox);
        l.top_inset = l.top_inset.max(oy);
        l.bottom_inset = l.bottom_inset.max(oy);
        tv_style(ctx);
    } else {
        touch_style(ctx);
    }

    // The player must be checked *before* the login guard. "Ohne Anbieter
    // testen" starts playback without ever creating a profile, so with the old
    // order `play()` ran (libmpv initialised) but the screen stayed on login —
    // and, because the player is where Android's paint callback lives, mpv's
    // render context was never created and `loadfile` never fired. The desktop
    // path in App::draw() checks View::Player first for the same reason.
    if app.view == View::Player && app.current_play.is_some() {
        player::show(app, ctx, &l);
        return;
    }

    if app.active_profile.is_none() {
        login::show(app, ctx, &l);
        return;
    }
    // The 10-foot shell replaces the bottom bar + "Mehr" sheet with a left
    // headers column and horizontal rails. Login and playback stay shared.
    if app.tv {
        tv::show(app, ctx, &l);
        return;
    }
    shell(app, ctx, &l);
}

fn touch_style(ctx: &Context) {
    if ctx.style().spacing.interact_size == egui::vec2(tokens::TOUCH, tokens::TOUCH)
        && ctx.style().spacing.scroll.bar_width == 3.0
    {
        return;
    }
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 10.0);
    style.spacing.interact_size = egui::vec2(tokens::TOUCH, tokens::TOUCH);
    style.spacing.scroll.bar_width = 3.0;
    style.spacing.scroll.floating = true;
    style
        .text_styles
        .insert(egui::TextStyle::Body, tokens::reg(tokens::FS_BODY));
    style
        .text_styles
        .insert(egui::TextStyle::Button, tokens::bold(tokens::FS_BODY));
    style
        .text_styles
        .insert(egui::TextStyle::Small, tokens::reg(tokens::FS_CAP));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, tokens::bold(tokens::FS_TITLE));
    ctx.set_style(style);
}

/// 10-foot type / interaction scale (MOBILE-BAUPLAN §23.4 P1).
///
/// Only egui's *default* styles grow here; `tv.rs` passes explicit TV-sized
/// `FontId`s for every hand-painted label. This is a separate mode with scaled
/// values — deliberately NOT a global `pixels_per_point` multiplier (§23.5),
/// which would blur the measured fonts and break the phone geometry.
fn tv_style(ctx: &Context) {
    if ctx.style().spacing.interact_size.y == 36.0 && ctx.style().spacing.scroll.bar_width == 5.0 {
        return;
    }
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(36.0, 36.0);
    style.spacing.scroll.bar_width = 5.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, tokens::reg(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, tokens::bold(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, tokens::reg(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, tokens::bold(18.0));
    ctx.set_style(style);
}

/// App bar + page + bottom navigation.
fn shell(app: &mut App, ctx: &Context, l: &Layout) {
    let mut detail_page = catalog_detail(app);
    if detail_page && edge_back(ctx, l.side) {
        app.mobile_back();
        detail_page = false;
    }
    let bar_color = ctx.style().visuals.window_fill;
    let page_color = ctx.style().visuals.panel_fill;

    // ---- bottom navigation (with the gesture-bar inset) ----
    let mut open_more = false;
    let mut page_layout = l.clone();
    if detail_page {
        // Detail pages own the full content height; the top bar supplies Back.
    } else if l.w - 2.0 * l.side >= 720.0 && !l.compact_height() {
        let width = 104.0 + l.side;
        egui::SidePanel::left("library-rail")
            .exact_width(width)
            .resizable(false)
            .frame(Frame::none().fill(bar_color))
            .show(ctx, |ui| {
                ui.add_space(l.top_inset + 20.0);
                ui.vertical_centered(|ui| crate::ui::brand_mark(ui, 42.0));
                ui.add_space(24.0);
                for (label, view) in [
                    ("Start", View::Home),
                    ("Live-TV", View::Live),
                    ("Filme", View::Movies),
                    ("Serien", View::Series),
                    ("Suche", View::Search),
                    ("Favoriten", View::Favorites),
                    ("Fortsetzen", View::Continue),
                    ("Einstellungen", View::Settings),
                ] {
                    if ui
                        .add_sized(
                            [width, 44.0],
                            egui::SelectableLabel::new(app.view == view, label),
                        )
                        .clicked()
                    {
                        app.view = view;
                    }
                }
            });
        page_layout.w -= width;
    } else {
        TopBottomPanel::bottom("m-nav")
            .exact_height(tokens::NAV_H + l.bottom_inset)
            .frame(Frame::none().fill(page_color))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, page_color);
                // The inset is subtracted by hand: `Frame::inner_margin` does not
                // shrink `max_rect`, so the tab labels would sit under the system
                // navigation bar.
                let content =
                    Rect::from_min_max(r.min, egui::pos2(r.max.x, r.max.y - l.bottom_inset));
                let mut child = ui.child_ui(content, EguiLayout::left_to_right(Align::Center));
                open_more = components::nav_bar(app, &mut child);
            });
    }
    let l = &page_layout;
    if open_more {
        app.mobile_more = true;
    }

    // ---- top app bar (with the status-bar inset) ----
    let (title, can_back, action) = bar_content(app);
    TopBottomPanel::top("m-appbar")
        .exact_height(l.appbar_h + l.top_inset)
        .frame(Frame::none().fill(bar_color))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            crate::theme::glass(ui, r, 0.0);
            // Full-bleed: the bar's *background* must reach the screen edges,
            // and the content inset now comes from `l.pad` inside `app_bar`
            // (`pad` already includes `side`, so a landscape cutout still
            // keeps the back button out of it).
            let bar_rect = Rect::from_min_size(
                egui::pos2(r.left(), r.top() + l.top_inset),
                egui::vec2(r.width(), l.appbar_h),
            );
            let mut child = ui.child_ui(bar_rect, EguiLayout::left_to_right(Align::Center));
            let out = components::app_bar(&mut child, &title, can_back, action, l.pad);
            if out.back {
                app.mobile_back();
            }
            if out.action {
                match app.view {
                    View::Movies => crate::ui::library::toggle_touch_search(app, ctx, false),
                    View::Series => crate::ui::library::toggle_touch_search(app, ctx, true),
                    _ => app.view = View::Search,
                }
            }
        });

    // ---- page ----
    egui::CentralPanel::default()
        .frame(Frame::none().fill(page_color))
        .show(ctx, |ui| {
            crate::theme::ambient(ui, ui.max_rect());
            ui.push_id(("section", app.view as u64), |ui| match app.view {
                View::Home => crate::ui::home::show(app, ui),
                View::Live => live::show(app, ui, l),
                View::Movies => movies::show(app, ui, l),
                View::Series => series::show(app, ui, l),
                View::Search => search::show(app, ui, l),
                View::Favorites => favorites::show(app, ui, l),
                View::Continue => continue_watching::show(app, ui, l),
                View::Settings => settings::show(app, ui, l),
                View::Player => {}
                View::Login => {}
            });
        });

    if app.mobile_more {
        more::sheet(app, ctx, l);
    }
    if app.mobile_epg_channel.is_some() {
        live::epg_sheet(app, ctx, l);
    }
}

/// Title, back button and right-hand action per screen.
fn bar_content(app: &App) -> (String, bool, Option<(Icon, &'static str)>) {
    if catalog_detail(app) {
        return (
            if app.view == View::Series {
                "Serie"
            } else {
                "Film"
            }
            .into(),
            true,
            None,
        );
    }
    let search_action = Some((Icon::Search, "Suche"));
    match app.view {
        View::Live => ("Live TV".to_string(), false, search_action),
        View::Movies => ("Filme".into(), false, search_action),
        View::Series => ("Serien".to_string(), false, search_action),
        View::Home => ("Startseite".to_string(), false, search_action),
        View::Search => ("Suche".to_string(), true, None),
        View::Favorites => ("Favoriten".to_string(), false, None),
        View::Continue => ("Fortsetzen".to_string(), false, None),
        View::Settings => ("Einstellungen".to_string(), false, None),
        View::Player | View::Login => (String::new(), false, None),
    }
}

fn catalog_detail(app: &App) -> bool {
    (app.view == View::Series && app.selected_series.is_some())
        || (app.view == View::Movies && app.selected_movie.is_some())
}

/// Only a horizontal gesture starting at the left edge is a page-back gesture.
/// Do not compete with vertical episode scrolling or horizontal season swipes.
fn edge_back(ctx: &Context, side: f32) -> bool {
    let id = egui::Id::new("catalog-edge-back");
    let mut start = ctx.data(|d| d.get_temp::<egui::Pos2>(id));
    let mut back = false;
    let edge = ctx.screen_rect().left() + side + 24.0;
    if ctx.memory(|m| m.any_popup_open()) {
        return false;
    }
    ctx.input(|i| {
        for event in &i.events {
            if let egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                ..
            } = event
            {
                if *pressed {
                    start = (pos.x <= edge).then_some(*pos);
                } else if let Some(origin) = start.take() {
                    let delta = *pos - origin;
                    back = delta.x >= 72.0 && delta.x > delta.y.abs() * 1.5;
                }
            }
        }
    });
    ctx.data_mut(|d| {
        if let Some(start) = start {
            d.insert_temp(id, start);
        } else {
            d.remove::<egui::Pos2>(id);
        }
    });
    back
}
