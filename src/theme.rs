//! Visual identity for Dado-Player.
//!
//! Deliberately flat and high-contrast so the same layout survives a later port
//! to Android / Smart-TV (10-foot UI): big hit areas, strong accent, no gradients.
//!
//! Palette reference: the "graphite" dark theme of IPTVnator (see
//! `references/iptvnator/apps/web/src/_theme-palettes.scss`): distinct surface
//! layers (shell → rail → content → widget), a soft azure selection accent and a
//! flat, near-white light theme. We keep our own brand and layout but adopt the
//! same layer/selection language.

use egui::{Color32, Context, FontFamily, FontId, Rounding, Stroke, TextStyle, Visuals};

/// Primary accent used as *ink* (rings, strokes, progress, selected text) on a
/// dark surface. On light surfaces use [`accent`] instead.
pub const ACCENT: Color32 = Color32::from_rgb(0x71, 0xDF, 0xD1);
/// Accent used as a *fill* behind white text (buttons, segments, brand mark).
/// A step darker than [`ACCENT`] so white labels keep ≥4.3:1 contrast.
pub const ACCENT_FILL: Color32 = Color32::from_rgb(0x09, 0x74, 0x70);
/// Soft accent surface (selected row / playing row) in the dark theme.
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x20, 0x39, 0x39);
/// Keyboard focus ring (brighter than the accent for visibility).
pub const FOCUS: Color32 = Color32::from_rgb(0xA1, 0xEE, 0xE0);

pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x8D, 0x9B);
pub const OK: Color32 = Color32::from_rgb(0x4A, 0xDE, 0x80);
/// On-air / live indicator.
pub const LIVE: Color32 = Color32::from_rgb(0xFF, 0x8F, 0x8F);
/// Rating stars / chips.
pub const RATING: Color32 = Color32::from_rgb(0xFF, 0xCF, 0x5C);

/// Provider accents (brighter variants for dark surfaces).
pub const PROVIDER_XTREAM: Color32 = Color32::from_rgb(0x71, 0xDF, 0xD1);
pub const PROVIDER_M3U: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);

// ---- dark palette (graphite steps) ----
pub const BG_DARK: Color32 = Color32::from_rgb(0x0D, 0x13, 0x16);
pub const PANEL_DARK: Color32 = Color32::from_rgb(0x15, 0x1D, 0x20);
pub const CARD_DARK: Color32 = Color32::from_rgb(0x1B, 0x25, 0x28);
pub const WIDGET_DARK: Color32 = Color32::from_rgb(0x21, 0x2D, 0x30);
pub const CARD_HOVER_DARK: Color32 = Color32::from_rgb(0x25, 0x2B, 0x3A);
pub const BORDER_DARK: Color32 = Color32::from_rgb(0x26, 0x2E, 0x3A);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0xEF, 0xF5, 0xF3);
pub const MUTED_DARK: Color32 = Color32::from_rgb(0xA1, 0xAA, 0xBD);

// ---- light palette (flat, near-white cards) ----
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xF0, 0xF4, 0xF2);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const CARD_LIGHT: Color32 = Color32::from_rgb(0xF2, 0xF4, 0xF7);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0xDF, 0xE3, 0xEA);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x18, 0x1D, 0x26);
pub const MUTED_LIGHT: Color32 = Color32::from_rgb(0x5F, 0x6B, 0x7D);

pub fn apply(ctx: &Context, dark: bool) {
    // Real system UI font (Segoe UI / Roboto) instead of egui's Ubuntu-Light.
    crate::fonts::install(ctx);
    let mut style = (*ctx.style()).clone();
    style.visuals = if dark {
        dark_visuals()
    } else {
        light_visuals()
    };

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

/// Accent as ink: bright azure on dark, the deeper fill blue on light.
pub fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT
    } else {
        ACCENT_FILL
    }
}

/// Accent as a fill behind white text. Same value in both themes.
pub fn accent_fill(_ui: &egui::Ui) -> Color32 {
    ACCENT_FILL
}

/// Accent as text. Lighter than [`ACCENT`] on dark for 4.5:1 on the rail.
pub fn accent_text(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(0xAE, 0xEC, 0xDF)
    } else {
        ACCENT_FILL
    }
}

/// Tinted selection surface (the rail's selected row, playing row).
pub fn selection_surface(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT_DIM
    } else {
        Color32::from_rgb(0xDB, 0xEF, 0xE9)
    }
}

/// Raised / hovered row surface.
pub fn card_hover(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        CARD_HOVER_DARK
    } else {
        Color32::from_rgb(0xE7, 0xEC, 0xF4)
    }
}

/// Hairline colour that works on both themes.
pub fn hairline(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_white_alpha(20)
    } else {
        BORDER_LIGHT
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
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);
    v.window_stroke = Stroke::new(1.0_f32, BORDER_DARK);
    v.selection.bg_fill = ACCENT_FILL;
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

    w.inactive.bg_fill = WIDGET_DARK;
    w.inactive.weak_bg_fill = WIDGET_DARK;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_DARK);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_DARK);
    w.inactive.rounding = Rounding::same(9.0);

    w.hovered.bg_fill = CARD_HOVER_DARK;
    w.hovered.weak_bg_fill = CARD_HOVER_DARK;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    w.hovered.rounding = Rounding::same(9.0);

    w.active.bg_fill = ACCENT_FILL;
    w.active.weak_bg_fill = ACCENT_FILL;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_FILL);
    w.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    w.active.rounding = Rounding::same(9.0);

    w.open.bg_fill = WIDGET_DARK;
    w.open.weak_bg_fill = WIDGET_DARK;
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
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);
    v.window_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    v.selection.bg_fill = ACCENT_FILL;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.hyperlink_color = ACCENT_FILL;
    v.striped = false;
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL_LIGHT;
    w.noninteractive.weak_bg_fill = PANEL_LIGHT;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.noninteractive.rounding = Rounding::same(10.0);

    w.inactive.bg_fill = Color32::from_rgb(0xF2, 0xF4, 0xF7);
    w.inactive.weak_bg_fill = Color32::from_rgb(0xF2, 0xF4, 0xF7);
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_LIGHT);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.inactive.rounding = Rounding::same(9.0);

    w.hovered.bg_fill = Color32::from_rgb(0xE7, 0xEC, 0xF4);
    w.hovered.weak_bg_fill = Color32::from_rgb(0xE7, 0xEC, 0xF4);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_FILL);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);
    w.hovered.rounding = Rounding::same(9.0);

    w.active.bg_fill = ACCENT_FILL;
    w.active.weak_bg_fill = ACCENT_FILL;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_FILL);
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

/// Lightweight translucent chrome, with an edge highlight in both appearances.
pub fn glass(ui: &egui::Ui, rect: egui::Rect, radius: f32) {
    let dark = ui.visuals().dark_mode;
    let base = if dark {
        Color32::from_rgba_unmultiplied(29, 43, 48, 224)
    } else {
        Color32::from_rgba_unmultiplied(247, 252, 253, 225)
    };
    ui.painter().rect_filled(
        rect.translate(egui::vec2(0.0, 4.0)),
        radius,
        Color32::from_black_alpha(if dark { 40 } else { 12 }),
    );
    ui.painter().rect_filled(rect, radius, base);
    let inner = rect.shrink(1.0);
    ui.painter().rect_stroke(
        inner,
        radius,
        Stroke::new(
            1.0_f32,
            Color32::from_white_alpha(if dark { 48 } else { 240 }),
        ),
    );
    let top = rect.shrink2(egui::vec2(radius * 0.7, 2.0));
    ui.painter().hline(
        top.left()..=top.right(),
        top.top(),
        Stroke::new(
            1.0_f32,
            Color32::from_white_alpha(if dark { 70 } else { 255 }),
        ),
    );
}
/// Ambient light beneath translucent navigation and library surfaces.
pub fn ambient(ui: &egui::Ui, rect: egui::Rect) {
    let dark = ui.visuals().dark_mode;
    let base = ui.visuals().panel_fill;
    ui.painter().rect_filled(rect, 0.0, base);
    let mut mesh = egui::Mesh::default();
    let colors = if dark {
        [
            Color32::from_rgb(19, 42, 52),
            Color32::from_rgb(29, 28, 47),
            base,
            base,
        ]
    } else {
        [
            Color32::from_rgb(221, 238, 243),
            Color32::from_rgb(234, 227, 244),
            base,
            base,
        ]
    };
    for (pos, color) in [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ]
    .into_iter()
    .zip(colors)
    {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}
