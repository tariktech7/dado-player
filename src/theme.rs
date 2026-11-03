//! Visual identity for Dado-Player.
//!
//! Deliberately flat and high-contrast so the same layout survives a later port
//! to Android / Smart-TV (10-foot UI): big hit areas, strong accent, no gradients.

use egui::{Color32, Context, FontFamily, FontId, Rounding, Stroke, TextStyle, Visuals};

/// Primary accent (buttons, selection, focus).
pub const ACCENT: Color32 = Color32::from_rgb(0x4C, 0x86, 0xFF);
/// Accent used for soft backgrounds (playing row, badges).
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x24, 0x3A, 0x66);
pub const DANGER: Color32 = Color32::from_rgb(0xE5, 0x6B, 0x6B);
pub const OK: Color32 = Color32::from_rgb(0x4E, 0xC0, 0x8A);

// dark palette
pub const BG_DARK: Color32 = Color32::from_rgb(0x0D, 0x10, 0x15);
pub const PANEL_DARK: Color32 = Color32::from_rgb(0x13, 0x17, 0x1E);
pub const CARD_DARK: Color32 = Color32::from_rgb(0x1A, 0x20, 0x2A);
pub const BORDER_DARK: Color32 = Color32::from_rgb(0x26, 0x2E, 0x3A);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0xE8, 0xEC, 0xF3);
pub const MUTED_DARK: Color32 = Color32::from_rgb(0x93, 0x9E, 0xB0);

// light palette
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xF3, 0xF5, 0xF9);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const CARD_LIGHT: Color32 = Color32::from_rgb(0xE9, 0xED, 0xF4);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0xD5, 0xDC, 0xE7);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x18, 0x1D, 0x26);
pub const MUTED_LIGHT: Color32 = Color32::from_rgb(0x5F, 0x6B, 0x7D);

pub fn apply(ctx: &Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    style.visuals = if dark { dark_visuals() } else { light_visuals() };

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(11.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.indent = 16.0;
    style.spacing.menu_margin = egui::Margin::same(6.0);

    style.text_styles = text_styles();
    ctx.set_style(style);
}

pub fn muted(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        MUTED_DARK
    } else {
        MUTED_LIGHT
    }
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    use TextStyle::*;
    [
        (Heading, FontId::new(22.0, FontFamily::Proportional)),
        (Body, FontId::new(14.5, FontFamily::Proportional)),
        (Button, FontId::new(14.0, FontFamily::Proportional)),
        (Small, FontId::new(11.5, FontFamily::Proportional)),
        (Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into()
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = BG_DARK;
    v.window_fill = PANEL_DARK;
    v.extreme_bg_color = Color32::from_rgb(0x08, 0x0A, 0x0E);
    v.faint_bg_color = CARD_DARK;
    v.code_bg_color = CARD_DARK;
    v.window_rounding = Rounding::same(14.0);
    v.menu_rounding = Rounding::same(10.0);
    v.window_stroke = Stroke::new(1.0_f32, BORDER_DARK);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.hyperlink_color = ACCENT;
    v.striped = false;
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL_DARK;
    w.noninteractive.weak_bg_fill = PANEL_DARK;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_DARK);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DARK);
    w.noninteractive.rounding = Rounding::same(10.0);

    w.inactive.bg_fill = CARD_DARK;
    w.inactive.weak_bg_fill = CARD_DARK;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_DARK);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_DARK);
    w.inactive.rounding = Rounding::same(9.0);

    w.hovered.bg_fill = Color32::from_rgb(0x22, 0x2A, 0x36);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x22, 0x2A, 0x36);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    w.hovered.rounding = Rounding::same(9.0);

    w.active.bg_fill = ACCENT;
    w.active.weak_bg_fill = ACCENT;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    w.active.rounding = Rounding::same(9.0);

    w.open.bg_fill = CARD_DARK;
    w.open.weak_bg_fill = CARD_DARK;
    w.open.bg_stroke = Stroke::new(1.0_f32, BORDER_DARK);
    w.open.fg_stroke = Stroke::new(1.0_f32, TEXT_DARK);
    w.open.rounding = Rounding::same(9.0);
    v
}

fn light_visuals() -> Visuals {
    let mut v = Visuals::light();
    v.dark_mode = false;
    v.panel_fill = BG_LIGHT;
    v.window_fill = PANEL_LIGHT;
    v.extreme_bg_color = PANEL_LIGHT;
    v.faint_bg_color = CARD_LIGHT;
    v.code_bg_color = CARD_LIGHT;
    v.window_rounding = Rounding::same(14.0);
    v.menu_rounding = Rounding::same(10.0);
    v.window_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.hyperlink_color = ACCENT;
    v.striped = false;
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL_LIGHT;
    w.noninteractive.weak_bg_fill = PANEL_LIGHT;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.noninteractive.rounding = Rounding::same(10.0);

    w.inactive.bg_fill = Color32::from_rgb(0xEC, 0xF0, 0xF6);
    w.inactive.weak_bg_fill = Color32::from_rgb(0xEC, 0xF0, 0xF6);
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.inactive.rounding = Rounding::same(9.0);

    w.hovered.bg_fill = Color32::from_rgb(0xE1, 0xE8, 0xF3);
    w.hovered.weak_bg_fill = Color32::from_rgb(0xE1, 0xE8, 0xF3);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.hovered.rounding = Rounding::same(9.0);

    w.active.bg_fill = ACCENT;
    w.active.weak_bg_fill = ACCENT;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    // egui uses `widgets.active.fg_stroke` for `RichText::strong()` too, so in
    // the light theme this must be a dark colour or headings turn white-on-white.
    w.active.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.active.rounding = Rounding::same(9.0);

    w.open.bg_fill = PANEL_LIGHT;
    w.open.weak_bg_fill = PANEL_LIGHT;
    w.open.bg_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    w.open.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.open.rounding = Rounding::same(9.0);
    v
}
