//! Visual identity for Dado-Player.
//!
//! Shared blue identity with normal, midnight and white appearances.
//!
//! Palette reference: the "graphite" dark theme of IPTVnator (see
//! `references/iptvnator/apps/web/src/_theme-palettes.scss`): distinct surface
//! layers (shell → rail → content → widget), a soft azure selection accent and a
//! flat, near-white light theme. We keep our own brand and layout but adopt the
//! same layer/selection language.

use egui::{Color32, Context, FontFamily, FontId, Rounding, Stroke, TextStyle, Visuals};

/// Primary accent used as *ink* (rings, strokes, progress, selected text) on a
/// dark surface. On light surfaces use [`accent`] instead.
pub const ACCENT: Color32 = Color32::from_rgb(0x91, 0xB9, 0xE8);
/// Accent used as a *fill* behind white text (buttons, segments, brand mark).
/// A step darker than [`ACCENT`] so white labels keep ≥4.3:1 contrast.
pub const ACCENT_FILL: Color32 = Color32::from_rgb(0x29, 0x58, 0x85);
/// Soft accent surface (selected row / playing row) in the dark theme.
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x29, 0x3D, 0x55);
/// Keyboard focus ring (brighter than the accent for visibility).
pub const FOCUS: Color32 = Color32::from_rgb(0xC9, 0xE1, 0xFF);

pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x8D, 0x9B);
pub const OK: Color32 = Color32::from_rgb(0x4A, 0xDE, 0x80);
/// On-air / live indicator.
pub const LIVE: Color32 = Color32::from_rgb(0xFF, 0x8F, 0x8F);
/// Rating stars / chips.
pub const RATING: Color32 = Color32::from_rgb(0xFF, 0xCF, 0x5C);

/// Provider accents (brighter variants for dark surfaces).
pub const PROVIDER_XTREAM: Color32 = Color32::from_rgb(0x91, 0xB9, 0xE8);
pub const PROVIDER_M3U: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);

// ---- dark palette (graphite steps) ----
pub const BG_DARK: Color32 = Color32::from_rgb(0x10, 0x19, 0x27);
pub const PANEL_DARK: Color32 = Color32::from_rgb(0x18, 0x25, 0x38);
pub const CARD_DARK: Color32 = Color32::from_rgb(0x20, 0x2F, 0x44);
pub const WIDGET_DARK: Color32 = Color32::from_rgb(0x27, 0x38, 0x50);
pub const CARD_HOVER_DARK: Color32 = Color32::from_rgb(0x25, 0x2B, 0x3A);
pub const BORDER_DARK: Color32 = Color32::from_rgb(0x26, 0x2E, 0x3A);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0xEF, 0xF5, 0xFF);
pub const MUTED_DARK: Color32 = Color32::from_rgb(0xA1, 0xAA, 0xBD);

// ---- light palette (flat, near-white cards) ----
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xF7, 0xF6, 0xFA);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const CARD_LIGHT: Color32 = Color32::from_rgb(0xF2, 0xF4, 0xF7);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0xDF, 0xE3, 0xEA);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x18, 0x1D, 0x26);
pub const MUTED_LIGHT: Color32 = Color32::from_rgb(0x5F, 0x6B, 0x7D);
/// The saved `Warm` key remains compatible; normal mode is now steel blue.
pub fn normal_color(name: &str) -> Color32 {
    static COLORS: std::sync::OnceLock<std::collections::HashMap<String, Color32>> =
        std::sync::OnceLock::new();
    COLORS.get_or_init(|| {
        let values: std::collections::HashMap<String, String> =
            serde_json::from_str(include_str!("../shared/normal-palette.json"))
                .expect("bundled normal palette");
        values
            .into_iter()
            .map(|(name, value)| {
                let rgb =
                    u32::from_str_radix(value.trim_start_matches('#'), 16).expect("palette RGB");
                (
                    name,
                    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8),
                )
            })
            .collect()
    })[name]
}
pub fn apply_warm(ctx: &Context) {
    ctx.style_mut(|s| {
        s.visuals = dark_visuals();
        let v = &mut s.visuals;
        v.panel_fill = normal_color("background");
        v.window_fill = normal_color("panel");
        v.extreme_bg_color = normal_color("input");
        v.faint_bg_color = normal_color("card");
        v.code_bg_color = v.faint_bg_color;
        v.hyperlink_color = normal_color("accent");
        v.selection.bg_fill = normal_color("accentFill");
        v.window_stroke.color = normal_color("line");
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.fg_stroke.color = normal_color("text");
            w.bg_stroke.color = normal_color("line");
        }
        v.widgets.noninteractive.bg_fill = v.window_fill;
        v.widgets.noninteractive.weak_bg_fill = v.window_fill;
        v.widgets.inactive.bg_fill = normal_color("card");
        v.widgets.inactive.weak_bg_fill = v.widgets.inactive.bg_fill;
        v.widgets.hovered.bg_fill = normal_color("hover");
        v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
        v.widgets.hovered.bg_stroke.color = normal_color("accent");
        v.widgets.active.bg_fill = normal_color("selection");
        v.widgets.active.weak_bg_fill = v.widgets.active.bg_fill;
        v.widgets.open.bg_fill = v.window_fill;
        v.widgets.open.weak_bg_fill = v.window_fill;
    });
}
pub fn is_normal(ui: &egui::Ui) -> bool {
    ui.visuals().panel_fill == normal_color("background")
}
fn warm(ui: &egui::Ui) -> bool {
    is_normal(ui)
}

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
    if warm(ui) {
        return normal_color("muted");
    }
    if ui.visuals().dark_mode {
        MUTED_DARK
    } else {
        MUTED_LIGHT
    }
}

/// Accent as ink: bright azure on dark, the deeper fill blue on light.
pub fn accent(ui: &egui::Ui) -> Color32 {
    if warm(ui) {
        return normal_color("accent");
    }
    if ui.visuals().dark_mode {
        ACCENT
    } else {
        normal_color("accentFill")
    }
}

/// Accent as a fill behind white text. Same value in both themes.
pub fn accent_fill(ui: &egui::Ui) -> Color32 {
    if warm(ui) || !ui.visuals().dark_mode {
        normal_color("accentFill")
    } else {
        ACCENT_FILL
    }
}

/// Accent as text. Lighter than [`ACCENT`] on dark for 4.5:1 on the rail.
pub fn accent_text(ui: &egui::Ui) -> Color32 {
    if warm(ui) {
        return normal_color("accent");
    }
    if ui.visuals().dark_mode {
        Color32::from_rgb(0xC9, 0xDF, 0xFA)
    } else {
        normal_color("accentFill")
    }
}

/// Tinted selection surface (the rail's selected row, playing row).
pub fn selection_surface(ui: &egui::Ui) -> Color32 {
    if warm(ui) {
        return normal_color("selection");
    }
    if ui.visuals().dark_mode {
        ACCENT_DIM
    } else {
        Color32::from_rgb(0xDB, 0xEA, 0xF6)
    }
}

/// Raised / hovered row surface.
pub fn card_hover(ui: &egui::Ui) -> Color32 {
    if warm(ui) {
        return ui.visuals().widgets.hovered.bg_fill;
    }
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
    if crate::ui::desktop::active(ui.ctx()) {
        // Match the web-TV material: one tinted panel and a soft diagonal
        // highlight, rather than dozens of horizontal reflection bands.
        ui.painter().rect_filled(
            rect.translate(egui::vec2(0.0, 4.0)),
            radius,
            Color32::from_black_alpha(18),
        );
        ui.painter()
            .rect_filled(rect, radius, ui.visuals().window_fill);
        ui.painter()
            .rect_stroke(rect.shrink(0.5), radius, ui.visuals().window_stroke);
        return;
    }
    let dark = ui.visuals().dark_mode;
    let base = if warm(ui) {
        normal_color("panel").gamma_multiply(0.92)
    } else if dark {
        Color32::from_rgba_unmultiplied(31, 43, 51, 185)
    } else {
        Color32::from_rgba_unmultiplied(247, 252, 253, 225)
    };
    ui.painter().rect_filled(
        rect.translate(egui::vec2(0.0, 4.0)),
        radius,
        Color32::from_black_alpha(if dark { 40 } else { 12 }),
    );
    ui.painter().rect_filled(rect, radius, base);
    // Edge-to-edge bars must not look like framed boxes around the safe area.
    if radius < 1.0 {
        return;
    }
    // A broad reflection and shaded lower edge give the material depth without
    // the old white outlines. Keep every vertex inside the rounded silhouette.
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let mut reflection = egui::Mesh::default();
    for i in 0..=32 {
        let t = i as f32 / 32.0;
        let y = rect.top() + rect.height() * t;
        let edge = (y - rect.top()).min(rect.bottom() - y);
        let inset = if edge < r {
            r - (r * r - (r - edge).powi(2)).max(0.0).sqrt()
        } else {
            0.0
        };
        let light = (1.0 - t * 2.8).max(0.0);
        let color = if t < 0.5 {
            Color32::from_white_alpha((light * if dark { 5.0 } else { 32.0 }) as u8)
        } else {
            Color32::from_black_alpha(((t - 0.5) * 24.0) as u8)
        };
        reflection.colored_vertex(egui::pos2(rect.left() + inset, y), color);
        reflection.colored_vertex(egui::pos2(rect.right() - inset, y), color);
        if i > 0 {
            let n = (i * 2) as u32;
            reflection.add_triangle(n - 2, n - 1, n);
            reflection.add_triangle(n - 1, n + 1, n);
        }
    }
    ui.painter().add(egui::Shape::mesh(reflection));
    let inner = rect.shrink(0.5);
    ui.painter().rect_stroke(
        inner,
        radius,
        Stroke::new(
            0.5_f32,
            Color32::from_white_alpha(if dark { 6 } else { 60 }),
        ),
    );
}
/// Ambient light beneath translucent navigation and library surfaces.
pub fn ambient(ui: &egui::Ui, rect: egui::Rect) {
    let dark = ui.visuals().dark_mode;
    let base = ui.visuals().panel_fill;
    ui.painter().rect_filled(rect, 0.0, base);
    if crate::ui::desktop::active(ui.ctx()) && dark && !warm(ui) {
        return;
    }
    let mut mesh = egui::Mesh::default();
    let colors = if warm(ui) {
        [normal_color("glow"), normal_color("background"), base, base]
    } else if dark {
        [
            Color32::from_rgb(19, 42, 52),
            Color32::from_rgb(23, 32, 49),
            base,
            base,
        ]
    } else {
        [
            Color32::from_rgb(221, 238, 243),
            Color32::from_rgb(227, 236, 247),
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

#[cfg(test)]
mod tests {
    #[test]
    fn blue_material_keeps_readable_text_and_keeps_a_distinct_midnight_mode() {
        fn luminance(c: egui::Color32) -> f32 {
            let f = |v: u8| {
                let v = f32::from(v) / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
        }
        let ctx = egui::Context::default();
        super::apply(&ctx, false);
        super::apply_warm(&ctx);
        let v = ctx.style().visuals.clone();
        assert!(v.dark_mode);
        assert!((luminance(v.text_color()) + 0.05) / (luminance(v.panel_fill) + 0.05) > 7.0);
        assert!(
            (luminance(egui::Color32::WHITE) + 0.05)
                / (luminance(super::normal_color("accentFill")) + 0.05)
                > 4.5
        );
        super::apply(&ctx, true);
        assert!(ctx.style().visuals.dark_mode);
        assert_eq!(ctx.style().visuals.panel_fill, super::BG_DARK);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert_eq!(super::accent(ui), super::ACCENT);
                assert_eq!(super::accent_fill(ui), super::ACCENT_FILL);
            });
        });
    }
}
