use crate::app::{App, View};
use crate::theme;
use crate::types::ProfileKind;
use crate::ui;
use egui::{Align2, Color32, FontId, Layout, RichText, Rounding, Sense, Stroke, Ui, Vec2};

/// One navigation entry. Selection is a tinted surface plus a full outline —
/// the language IPTVnator uses for its workspace rail (no solid accent fill,
/// no side strips), so nav, channel rows and EPG cards read as one system.
fn nav(ui: &mut Ui, app: &mut App, label: &str, v: View) {
    let selected = app.view == v;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), Sense::click());

    let bg = if selected {
        theme::selection_surface(ui)
    } else if resp.hovered() {
        theme::card_hover(ui)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(9.0), bg);
    if selected {
        ui.painter().rect_stroke(
            rect,
            Rounding::same(9.0),
            Stroke::new(1.0_f32, theme::accent(ui)),
        );
    }

    let color = if selected {
        theme::accent_text(ui)
    } else {
        ui.visuals().text_color()
    };
    let y = rect.center().y;
    use crate::ui::mobile::components::{icon as draw_icon, Icon};
    let symbol = match v {
        View::Home => Icon::User,
        View::Live => Icon::Tv,
        View::Movies => Icon::Film,
        View::Series => Icon::Series,
        View::Search => Icon::Search,
        View::Favorites => Icon::Star,
        View::Continue => Icon::Play,
        _ => Icon::Sliders,
    };
    draw_icon(
        ui,
        egui::Rect::from_center_size(egui::pos2(rect.left() + 21.0, y), Vec2::splat(22.0)),
        symbol,
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
    let resp = resp.on_hover_text(label);
    if resp.clicked() {
        app.view = v;
    }
    ui.add_space(2.0);
}

/// Small provider pill: coloured dot + kind, so the active source is readable
/// at a glance (Xtream = azure, M3U = amber), matching the provider accents.
fn provider_chip(ui: &mut Ui, kind: ProfileKind) {
    let (name, col) = match kind {
        ProfileKind::Xtream => ("Xtream", theme::PROVIDER_XTREAM),
        ProfileKind::M3u => ("M3U", theme::PROVIDER_M3U),
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(74.0, 20.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(6.0), col.gamma_multiply(0.16));
    let dot = egui::pos2(rect.left() + 9.0, rect.center().y);
    ui.painter().circle_filled(dot, 3.0, col);
    ui.painter().text(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(11.0),
        col,
    );
}

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui::brand_mark(ui, 34.0);
        ui.add_space(9.0);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(
                RichText::new("Dado-Player").size(17.0).strong(),
            ));
            ui.add(egui::Label::new(
                RichText::new(include_str!("../../windows/VERSION").trim())
                    .size(10.5)
                    .weak(),
            ));
        });
    });

    if let Some(p) = &app.active_profile {
        let kind = p.kind;
        ui.add_space(8.0);
        provider_chip(ui, kind);
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    let mut logout = false;
    let mut open_palette = false;

    ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(12.0);
        let btn_resp = ui.add_sized(
            [ui.available_width(), 32.0],
            egui::Button::new(RichText::new("⏻  Abmelden").size(12.5)),
        );
        if btn_resp.clicked() {
            logout = true;
        }

        ui.add_space(8.0);
        if let Some(p) = &app.active_profile {
            ui.horizontal(|ui| {
                ui.label(RichText::new("👤").size(12.0).color(theme::muted(ui)));
                ui.add(
                    egui::Label::new(
                        RichText::new(&p.name)
                            .size(11.5)
                            .strong()
                            .color(theme::muted(ui)),
                    )
                    .truncate(true),
                );
            });
        }

        ui.add_space(6.0);
        let pal_w = ui.available_width();
        let (pal_rect, pal_resp) = ui.allocate_exact_size(Vec2::new(pal_w, 24.0), Sense::click());
        let pal_bg = if pal_resp.hovered() {
            theme::card_hover(ui)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(pal_rect, Rounding::same(6.0), pal_bg);
        ui.painter().text(
            egui::pos2(pal_rect.left() + 6.0, pal_rect.center().y),
            Align2::LEFT_CENTER,
            "⌘K / Ctrl+K",
            FontId::proportional(10.5),
            theme::accent_text(ui),
        );
        ui.painter().text(
            egui::pos2(pal_rect.right() - 6.0, pal_rect.center().y),
            Align2::RIGHT_CENTER,
            "Palette",
            FontId::proportional(10.5),
            theme::muted(ui),
        );
        if pal_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if pal_resp.clicked() {
            open_palette = true;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        // Navigation links in remaining space
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                nav(ui, app, "Startseite", View::Home);
                nav(ui, app, "Live TV", View::Live);
                nav(ui, app, "Filme", View::Movies);
                nav(ui, app, "Serien", View::Series);
                nav(ui, app, "Suche", View::Search);
                nav(ui, app, "Favoriten", View::Favorites);
                nav(ui, app, "Fortsetzen", View::Continue);
                nav(ui, app, "Einstellungen", View::Settings);
            });
    });

    if logout {
        app.logout();
    }
    if open_palette {
        app.palette_open = true;
    }
}
