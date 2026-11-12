//! Design tokens for the touch UI.
//!
//! Everything is expressed in points (= dp on Android, logical px on Windows),
//! so one set of numbers works on a 360 dp phone and on a resized desktop
//! window. Values follow the mobile spec (see MOBILE-BAUPLAN.md §18): large
//! type, 64 dp rows, 48 dp minimum hit area, flat surfaces, no shadows.

use egui::{Color32, FontFamily, FontId, Ui};

/// Outer page margin.
pub const PAD: f32 = 16.0;
/// Gap between cards / list blocks.
pub const GAP: f32 = 12.0;
/// One channel / setting row.
pub const ROW_H: f32 = 64.0;
/// Episode row (two text lines).
pub const EP_ROW_H: f32 = 72.0;
/// Bottom navigation bar (without the system gesture inset).
pub const NAV_H: f32 = 56.0;
/// Top app bar (without the status-bar inset).
pub const APPBAR_H: f32 = 56.0;
/// App bar in landscape (short screens).
pub const APPBAR_H_LAND: f32 = 48.0;
/// Minimum touch target (Android / WCAG).
pub const TOUCH: f32 = 48.0;
/// Rows of prose on a detail page. Capped by rows, never by characters.
pub const PLOT_ROWS: usize = 3;
/// Rows of prose when the viewport is a short landscape phone.
///
/// Three rows do not fit under a 96 dp cover in compact: the text column alone
/// needs 26 + 4 + 3x20 + 8 + 17 = 115 dp, which pushes the hero back over the
/// 96 dp the layout budgeted for it and leaves no room for an episode row.
pub const PLOT_ROWS_COMPACT: usize = 2;
/// Cover width on a series detail hero. Compact uses 64x96, not 96x144: the
/// 144 dp cover is what left zero episode rows on a 411x253 dp landscape.
pub const COVER_W: f32 = 96.0;
pub const COVER_W_COMPACT: f32 = 64.0;
/// Maximum measure for prose, at every width — ~65 characters at `FS_SUB`.
///
/// Applied at the call site, not inside `wrap`: `wrap` stays a dumb line
/// capper, and the decision about how wide a line of prose may get is a layout
/// decision. Without this a landscape phone gives the plot a 655 dp column and
/// ~100 characters per line, which the eye cannot track back from.
pub const PROSE_MAX_W: f32 = 460.0;

pub const RADIUS: f32 = 16.0;
pub const RADIUS_SM: f32 = 12.0;

/// Subtle selection scale for cards (MOBILE-BAUPLAN §23.6 B): pops the artwork
/// over the gutter without ever growing the click area past the allocated rect.
pub const SEL_SCALE: f32 = 1.03;
/// Stroke width for the accent selection ring (B) on rows and cards.
pub const SEL_RING: f32 = 2.0;

// ---- type scale ----
pub const FS_DISPLAY: f32 = 24.0;
pub const FS_TITLE: f32 = 20.0;
pub const FS_HEAD: f32 = 17.0;
pub const FS_ROW: f32 = 16.0;
pub const FS_BODY: f32 = 16.0;
/// Supporting line under a heading (login subtitle, empty-state body).
pub const FS_SUB: f32 = 15.0;
pub const FS_LABEL: f32 = 13.0;
pub const FS_CAP: f32 = 11.5;

/// Family name of the bold face registered by `crate::fonts`.
pub const BOLD: &str = "ui-bold";
/// Family name of the medium face (Roboto-Medium / Segoe UI Semibold).
pub const MEDIUM: &str = "ui-medium";

/// Medium weight — section labels, list titles.
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

/// Bold text. egui has no font weights, so real bold needs its own family.
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// Regular text.
pub fn reg(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// Filled-button background. `#2F7BFF` keeps white labels at ≥4.3:1.
pub fn accent_fill() -> Color32 {
    crate::theme::ACCENT_FILL
}

/// Accent used as *text* on a light background.
pub fn accent_text(ui: &Ui) -> Color32 {
    crate::theme::accent_text(ui)
}

/// Soft accent background (playing row, selected chip in some contexts).
pub fn accent_soft(ui: &Ui) -> Color32 {
    crate::theme::selection_surface(ui)
}

/// Card / raised surface (level 2).
pub fn surface(ui: &Ui) -> Color32 {
    ui.visuals().faint_bg_color
}

/// Bar surface (level 1).
pub fn bar(ui: &Ui) -> Color32 {
    ui.visuals().window_fill
}

/// Page background (level 0).
pub fn bg(ui: &Ui) -> Color32 {
    ui.visuals().panel_fill
}

/// Pressed surface (level 3).
pub fn pressed(ui: &Ui) -> Color32 {
    ui.visuals().widgets.hovered.weak_bg_fill
}

/// 1 px divider colour.
pub fn hairline(ui: &Ui) -> Color32 {
    ui.visuals().widgets.noninteractive.bg_stroke.color
}

/// Secondary text.
pub fn muted(ui: &Ui) -> Color32 {
    crate::theme::muted(ui)
}

/// Primary text.
pub fn text(ui: &Ui) -> Color32 {
    ui.visuals().text_color()
}

/// Accent used for selection / primary actions.
pub fn accent() -> Color32 {
    crate::theme::ACCENT
}

/// Stroke colour of the D-pad focus ring.
///
/// It must contrast with BOTH a dark surface and an accent-filled primary
/// button. The ring used to be `accent()` — on the blue "Anmelden" button that
/// is blue-on-blue and completely invisible, so a TV user pressing DOWN saw a
/// dead key. Verified on-device with `input keyevent 20` on dado_test.
pub fn focus_ring(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(0xFF, 0xFF, 0xFF)
    } else {
        Color32::from_rgb(0x10, 0x14, 0x1C)
    }
}

/// Destructive actions. Grey reads as "hide this", red as "remove it".
pub fn danger() -> Color32 {
    crate::theme::DANGER
}
