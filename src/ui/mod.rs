pub mod continue_watching;
pub mod favorites;
pub mod live;
pub mod login;
pub mod player;
pub mod search;
pub mod series;
pub mod settings;
pub mod sidebar;
pub mod vod;

use crate::app::App;
use crate::theme;
use egui::{Align2, Color32, FontId, Image, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2};

/// Width of one poster card (image + padding).
pub const CARD_W: f32 = 148.0;
/// Poster image size, 2:3 like every streaming service.
pub const CARD_IMG: Vec2 = Vec2::new(132.0, 190.0);
/// Height of one channel row.
pub const ROW_H: f32 = 58.0;

/// A small star toggle bound to a favorite key.
pub fn fav_button(app: &mut App, ui: &mut Ui, key: &str) {
    let is = app.is_favorite(key);
    let text = if is { "★" } else { "☆" };
    let mut b = egui::Button::new(RichText::new(text).size(16.0)).frame(false);
    if is {
        b = b.fill(Color32::TRANSPARENT);
    }
    if ui.add(b).on_hover_text("Favorit").clicked() {
        app.toggle_favorite(key);
    }
}

/// Show an image from the cache, requesting it lazily. Falls back to a placeholder.
pub fn image_or_placeholder(app: &mut App, ui: &mut Ui, key: &str, url: &str, size: Vec2) {
    if !url.is_empty() {
        app.request_image(key.to_string(), url.to_string());
        if let Some(tex) = app.image(key) {
            ui.add(Image::new(tex).fit_to_exact_size(size));
            return;
        }
    }
    let (rect, _resp) = ui.allocate_exact_size(size, Sense::hover());
    placeholder_rect(ui, rect);
}

fn placeholder_rect(ui: &Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, Rounding::same(8.0), ui.visuals().faint_bg_color);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "IMG",
        FontId::proportional(12.0),
        ui.visuals().weak_text_color(),
    );
}

/// Clickable 2:3 poster used by the movie and series grids.
pub fn poster(app: &mut App, ui: &mut Ui, key: &str, url: &str, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(CARD_IMG, Sense::click());
    let rounding = Rounding::same(10.0);

    ui.painter()
        .rect_filled(rect, rounding, ui.visuals().faint_bg_color);

    if !url.is_empty() {
        app.request_image(key.to_string(), url.to_string());
        if let Some(tex) = app.image(key) {
            ui.painter().image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "…",
                FontId::proportional(16.0),
                ui.visuals().weak_text_color(),
            );
        }
    } else {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "IMG",
            FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
    }

    if selected {
        ui.painter()
            .rect_stroke(rect, rounding, Stroke::new(2.0_f32, theme::ACCENT));
    } else if resp.hovered() {
        ui.painter()
            .rect_stroke(rect, rounding, Stroke::new(2.0_f32, theme::ACCENT.gamma_multiply(0.8)));
    }

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Columns per row and the gap needed to fill the row evenly.
/// Subtracts a safety margin so the last column never collides with the
/// scrollbar.
pub fn grid_metrics(ui: &Ui) -> (usize, f32) {
    let avail = (ui.available_width() - 26.0).max(120.0);
    let per_row = ((avail / (CARD_W + 14.0)).floor() as usize).max(1);
    let cell = avail / per_row as f32;
    let gap = (cell - CARD_W - 8.0).max(0.0);
    (per_row, gap)
}

/// Prev/next pager used by every list view.
pub fn pager(ui: &mut Ui, page: &mut usize, pages: usize, count: usize, noun: &str) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(*page > 0, egui::Button::new("◀ Zurück"))
            .clicked()
        {
            *page = page.saturating_sub(1);
        }
        ui.add(egui::Label::new(
            RichText::new(format!("Seite {} / {}", *page + 1, pages.max(1))).size(13.0),
        ));
        if ui
            .add_enabled(*page + 1 < pages, egui::Button::new("Weiter ▶"))
            .clicked()
        {
            *page += 1;
        }
        ui.add_space(10.0);
        ui.label(RichText::new(format!("{} {}", count, noun)).weak().size(13.0));
    });
}

/// The little square app mark with "D7".
pub fn brand_mark(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let r = Rounding::same(size * 0.30);
    ui.painter().rect_filled(rect, r, theme::ACCENT);
    ui.painter()
        .rect_stroke(rect, r, Stroke::new(1.0_f32, Color32::from_white_alpha(34)));
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "D7",
        FontId::proportional(size * 0.46),
        Color32::WHITE,
    );
}

/// Segmented switch (tabs drawn as pills). Returns the newly picked index.
pub fn segmented(ui: &mut Ui, id: &str, labels: &[&str], current: usize) -> Option<usize> {
    let h = 30.0;
    let pad = 3.0;
    let cell_w = 104.0;
    let total = cell_w * labels.len() as f32 + pad * 2.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, h), Sense::hover());
    ui.painter().rect_filled(
        rect,
        Rounding::same(h * 0.5),
        ui.visuals().faint_bg_color,
    );
    let mut picked = None;
    let mut x = rect.left() + pad;
    for (i, label) in labels.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + pad),
            Vec2::new(cell_w, h - pad * 2.0),
        );
        let resp = ui.interact(cell, ui.id().with((id, i)), Sense::click());
        if i == current {
            ui.painter()
                .rect_filled(cell, Rounding::same(cell.height() * 0.5), theme::ACCENT);
        }
        let color = if i == current {
            Color32::WHITE
        } else {
            ui.visuals().text_color()
        };
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.0),
            color,
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            picked = Some(i);
        }
        x += cell_w;
    }
    picked
}

/// A muted, single-line label that truncates instead of wrapping.
pub fn one_line(ui: &mut Ui, text: &str, size: f32, strong: bool) {
    let mut rt = RichText::new(text).size(size);
    if strong {
        rt = rt.strong();
    }
    ui.add(egui::Label::new(rt).truncate(true));
}

/// Format seconds as h:mm:ss / m:ss.
pub fn fmt_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, sec)
    } else {
        format!("{:02}:{:02}", m, sec)
    }
}
