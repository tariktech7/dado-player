use crate::app::{App, View};
use crate::theme;
use crate::ui;
use egui::{Align2, Color32, FontId, Layout, RichText, Rounding, Sense, Ui, Vec2};

fn nav(ui: &mut Ui, app: &mut App, icon: &str, label: &str, v: View) {
    let selected = app.view == v;
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 34.0),
        Sense::click(),
    );

    let bg = if selected {
        theme::ACCENT
    } else if resp.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(9.0), bg);

    let color = if selected {
        Color32::WHITE
    } else {
        ui.visuals().text_color()
    };
    let y = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, y),
        Align2::LEFT_CENTER,
        icon,
        FontId::proportional(14.0),
        color,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 40.0, y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.5),
        color,
    );

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        app.view = v;
    }
    ui.add_space(2.0);
}

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui::brand_mark(ui, 34.0);
        ui.add_space(9.0);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(RichText::new("Dado-Player").size(17.0).strong()));
            ui.add(egui::Label::new(RichText::new("v0.3").size(10.5).weak()));
        });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    nav(ui, app, "📺", "Live TV", View::Live);
    nav(ui, app, "🎬", "Filme", View::Movies);
    nav(ui, app, "📚", "Serien", View::Series);
    nav(ui, app, "🔎", "Suche", View::Search);
    nav(ui, app, "⭐", "Favoriten", View::Favorites);
    nav(ui, app, "▶", "Fortsetzen", View::Continue);
    nav(ui, app, "⚙", "Einstellungen", View::Settings);

    let mut logout = false;
    ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
        // Bottom-up: the first item sits at the very bottom.
        ui.add_space(12.0);
        if ui
            .add_sized(
                [ui.available_width(), 30.0],
                egui::Button::new(RichText::new("Abmelden").size(13.0)),
            )
            .clicked()
        {
            logout = true;
        }
        ui.add_space(6.0);
        if let Some(p) = &app.active_profile {
            ui.add(egui::Label::new(
                RichText::new(format!("Profil: {}", p.name))
                    .size(11.0)
                    .weak(),
            ));
        }
    });

    if logout {
        app.logout();
    }
}
