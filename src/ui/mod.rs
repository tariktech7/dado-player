pub mod continue_watching;
pub mod desktop;
pub mod favorites;
pub mod home;
pub mod library;
pub mod live;
pub mod login;
pub mod mobile;
pub mod palette;
pub mod parental;
pub mod player;
pub(crate) mod player_desktop;
pub mod search;
pub mod series;
pub mod settings;
pub mod sidebar;
pub mod vod;

use crate::app::App;
use crate::theme;
use egui::{Align2, Color32, FontId, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2};

/// Center an entire image without cropping or changing its aspect ratio.
pub fn contain_rect(bounds: egui::Rect, image_size: Vec2) -> egui::Rect {
    if image_size.x <= 0.0 || image_size.y <= 0.0 || !image_size.is_finite() {
        return egui::Rect::from_center_size(bounds.center(), Vec2::ZERO);
    }
    let scale = (bounds.width() / image_size.x).min(bounds.height() / image_size.y);
    egui::Rect::from_center_size(bounds.center(), image_size * scale)
}

/// Width of one poster card (image + padding).
pub const CARD_W: f32 = 148.0;
/// Poster image size, 2:3 like every streaming service.
pub const CARD_IMG: Vec2 = Vec2::new(132.0, 190.0);
/// Height of one channel row.
pub const ROW_H: f32 = 58.0;

/// Non-blocking status banner for a partial catalogue load.
///
/// A single failing Xtream endpoint used to abort the whole login. Now the
/// working areas are shown immediately; this banner names what is missing and
/// offers a one-tap refresh instead of the "log out and back in ten times"
/// workaround. It also shows the in-flight state while that refresh runs.
pub fn catalog_banner(app: &mut App, ui: &mut Ui) {
    let summary = if desktop::active(ui.ctx()) {
        app.library.catalog_issue_summary()
    } else if app.catalog_refreshing {
        "Katalog wird erneut geladen…".to_string()
    } else {
        let issues = app.library.catalog_issue_summary();
        if issues.is_empty() {
            app.catalog_refresh_notice.clone().unwrap_or_default()
        } else {
            issues
        }
    };
    if summary.is_empty() {
        return;
    }

    egui::Frame::none()
        .fill(ui.visuals().widgets.noninteractive.bg_fill)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme::RATING))
        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("!").size(13.0).strong().color(theme::RATING));
                ui.add(egui::Label::new(RichText::new(&summary).size(11.5)).wrap(true));
                if !app.catalog_refreshing
                    && !app.library.catalog_issues.is_empty()
                    && ui.button("Erneut laden").clicked()
                {
                    app.refresh_catalog();
                    ui.ctx().request_repaint();
                }
            });
        });
    ui.add_space(8.0);
}

/// A small star toggle bound to a favorite key.
pub fn fav_button(app: &mut App, ui: &mut Ui, key: &str) -> Response {
    let marked = app.is_favorite(key);
    let size = desktop::handheld_size((42.0 * desktop::scale(ui.ctx())).max(20.0), 40.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let response = response.on_hover_text(if marked {
        "Favorit entfernen"
    } else {
        "Als Favorit markieren"
    });
    desktop::remember_favorite_target(&response, key);
    let clicked = if app.tv {
        mobile::focus::clicked(ui, &response)
    } else {
        response.clicked()
    };
    if clicked {
        app.toggle_favorite(key);
    }
    favorite_star(ui, rect.center(), size * 0.34, app.is_favorite(key));
    response
}

pub(super) fn favorite_star(ui: &Ui, center: egui::Pos2, radius: f32, marked: bool) {
    let color = if marked {
        if ui.visuals().dark_mode {
            Color32::from_rgb(255, 211, 107)
        } else {
            Color32::from_rgb(143, 83, 0)
        }
    } else {
        theme::muted(ui)
    };
    draw_favorite_star(ui, center, radius, marked, color);
}

pub(super) fn draw_favorite_star(
    ui: &Ui,
    center: egui::Pos2,
    radius: f32,
    marked: bool,
    color: Color32,
) {
    let points: Vec<_> = (0..10)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / 10.0 - std::f32::consts::FRAC_PI_2;
            center + Vec2::angled(angle) * radius * if i % 2 == 0 { 1.0 } else { 0.46 }
        })
        .collect();
    if marked {
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(center, color);
        for point in &points {
            mesh.colored_vertex(*point, color);
        }
        for i in 0..10 {
            mesh.add_triangle(0, i + 1, (i + 1) % 10 + 1);
        }
        ui.painter().add(egui::Shape::mesh(mesh));
    } else {
        ui.painter().add(egui::Shape::closed_line(
            points,
            Stroke::new((radius * 0.13).max(1.0), color),
        ));
    }
}

/// Show an image from the cache, requesting it lazily. Falls back to a placeholder.
pub fn image_or_placeholder(app: &mut App, ui: &mut Ui, key: &str, url: &str, size: Vec2) {
    if !url.is_empty() {
        app.request_image(key.to_string(), url.to_string());
        if let Some(tex) = app.image(key) {
            let (bounds, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter().image(
                tex.id(),
                contain_rect(bounds, tex.size_vec2()),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            return;
        }
    }
    let (rect, _resp) = ui.allocate_exact_size(size, Sense::hover());
    placeholder_rect(ui, rect);
}

fn placeholder_rect(ui: &Ui, rect: egui::Rect) {
    ui.painter()
        .rect_filled(rect, Rounding::same(8.0), ui.visuals().faint_bg_color);
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
    poster_badge(app, ui, key, url, selected, None)
}

/// Poster with an optional corner badge (used for the rating on movies).
pub fn poster_badge(
    app: &mut App,
    ui: &mut Ui,
    key: &str,
    url: &str,
    selected: bool,
    badge: Option<&str>,
) -> Response {
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

    // Corner pill, e.g. "★ 7.8". Painted over the artwork, dark enough to stay
    // readable on bright covers.
    if let Some(text) = badge.filter(|b| !b.trim().is_empty()) {
        let galley = ui.painter().layout_no_wrap(
            text.to_string(),
            FontId::proportional(10.5),
            theme::RATING,
        );
        let pad = Vec2::new(6.0, 3.0);
        let size = galley.size() + pad * 2.0;
        let pill = egui::Rect::from_min_size(
            egui::pos2(rect.right() - size.x - 6.0, rect.top() + 6.0),
            size,
        );
        ui.painter().rect_filled(
            pill,
            Rounding::same(size.y * 0.5),
            Color32::from_black_alpha(175),
        );
        ui.painter().galley(pill.min + pad, galley, theme::RATING);
    }

    if selected {
        ui.painter()
            .rect_stroke(rect, rounding, Stroke::new(2.0_f32, theme::accent(ui)));
    } else if resp.hovered() {
        ui.painter().rect_stroke(
            rect,
            rounding,
            Stroke::new(2.0_f32, theme::accent(ui).gamma_multiply(0.8)),
        );
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
        ui.label(
            RichText::new(format!("{} {}", count, noun))
                .weak()
                .size(13.0),
        );
    });
}

/// The little square app mark with "D7".
pub fn brand_mark(ui: &mut Ui, size: f32) {
    let dark = ui.visuals().dark_mode && !crate::theme::is_normal(ui);
    let key = egui::Id::new(("d7-icon-texture", dark));
    let texture = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureHandle>(key))
        .unwrap_or_else(|| {
            let img = image::load_from_memory(if dark {
                include_bytes!("../../assets/icon-dark.png").as_slice()
            } else {
                include_bytes!("../../assets/icon.png").as_slice()
            })
            .expect("bundled D7 icon")
            .thumbnail(128, 128)
            .to_rgba8();
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [img.width() as usize, img.height() as usize],
                img.as_raw(),
            );
            let texture = ui
                .ctx()
                .load_texture("D7", image, egui::TextureOptions::LINEAR);
            ui.ctx().data_mut(|d| d.insert_temp(key, texture.clone()));
            texture
        });
    ui.image((texture.id(), Vec2::splat(size)));
}

/// Segmented switch (tabs drawn as pills). Returns the newly picked index.
pub fn segmented(ui: &mut Ui, id: &str, labels: &[&str], current: usize) -> Option<usize> {
    let h = 30.0;
    let pad = 3.0;
    let cell_w = 104.0;
    let total = cell_w * labels.len() as f32 + pad * 2.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, h), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(h * 0.5), ui.visuals().faint_bg_color);
    let mut picked = None;
    let mut x = rect.left() + pad;
    for (i, label) in labels.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + pad),
            Vec2::new(cell_w, h - pad * 2.0),
        );
        let resp = ui.interact(cell, ui.id().with((id, i)), Sense::click());
        if i == current {
            ui.painter().rect_filled(
                cell,
                Rounding::same(cell.height() * 0.5),
                theme::accent_fill(ui),
            );
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

#[cfg(test)]
mod image_tests {
    #[test]
    fn logos_keep_their_aspect_ratio_and_stay_inside_the_slot() {
        let slot = egui::Rect::from_min_size(egui::pos2(17.0, 23.0), egui::vec2(34.0, 34.0));
        for size in [
            egui::vec2(320.0, 80.0),
            egui::vec2(80.0, 160.0),
            egui::vec2(40.0, 40.0),
        ] {
            let fitted = super::contain_rect(slot, size);
            assert!(slot.contains_rect(fitted));
            assert_eq!(fitted.center(), slot.center());
            assert!((fitted.width() / fitted.height() - size.x / size.y).abs() < 0.001);
        }
        assert_eq!(
            super::contain_rect(slot, egui::Vec2::ZERO).size(),
            egui::Vec2::ZERO
        );
    }
}
