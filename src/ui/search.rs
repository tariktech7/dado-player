use crate::app::App;
use crate::theme;
use crate::types::{Channel, PlayItem, PlayKind, Series, VodMovie};
use crate::ui;
use egui::{Align, Color32, Layout, RichText, Rounding, Sense, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new("Suche").size(22.0).strong()));
    ui.add_space(6.0);
    if super::desktop::active(ui.ctx()) {
        super::desktop::search_field(
            ui,
            &mut app.search_q,
            "Kanal, Film oder Serie suchen…",
            app.tv,
        );
    } else {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.search_q)
                    .hint_text("Kanal, Film oder Serie suchen…")
                    .desired_width(380.0),
            );
            if !app.search_q.is_empty() && ui.button("×").clicked() {
                app.search_q.clear();
            }
        });
    }
    ui.add_space(6.0);

    let q = app.search_q.trim().to_lowercase();
    if q.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Label::new(
                RichText::new("🔎").size(40.0).color(theme::muted(ui)),
            ));
            ui.add_space(8.0);
            ui.add(egui::Label::new(
                RichText::new("Suchbegriff eingeben, um deine Bibliothek zu durchsuchen.")
                    .size(13.0)
                    .color(theme::muted(ui)),
            ));
        });
        return;
    }

    let mut play: Option<PlayItem> = None;
    let mut open_series: Option<u32> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let chans: Vec<Channel> = app
                .library
                .channels
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&q))
                .take(60)
                .cloned()
                .collect();
            let movies: Vec<VodMovie> = app
                .library
                .movies
                .iter()
                .filter(|m| m.name.to_lowercase().contains(&q))
                .take(60)
                .cloned()
                .collect();
            let series: Vec<Series> = app
                .library
                .series
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&q))
                .take(60)
                .cloned()
                .collect();

            let total_hits = chans.len() + movies.len() + series.len();
            if total_hits == 0 {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Label::new(
                        RichText::new(format!(
                            "Keine Ergebnisse für „{}“ gefunden.",
                            app.search_q.trim()
                        ))
                        .size(13.5)
                        .color(theme::muted(ui)),
                    ));
                });
                return;
            }

            if !chans.is_empty() {
                section(ui, &format!("Kanäle ({})", chans.len()));
                for c in &chans {
                    let (rect, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 48.0), Sense::click());
                    let dark = ui.visuals().dark_mode;
                    let bg = if resp.hovered() {
                        if dark {
                            Color32::from_white_alpha(18)
                        } else {
                            Color32::from_black_alpha(10)
                        }
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    let mut row = ui.child_ui(rect, Layout::left_to_right(Align::Center));
                    row.add_space(10.0);
                    ui::image_or_placeholder(
                        app,
                        &mut row,
                        &format!("logo:{}", c.id),
                        &c.logo,
                        Vec2::splat(32.0),
                    );
                    row.add_space(12.0);
                    row.vertical(|ui| {
                        ui::one_line(ui, &c.name, 13.5, true);
                    });
                    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui.button("▶").on_hover_text("Abspielen").clicked() {
                            play = Some(PlayItem {
                                title: c.name.clone(),
                                url: c.stream_url.clone(),
                                kind: PlayKind::Live,
                                progress_key: format!("live:{}", c.id),
                            });
                        }
                    });

                    if resp.clicked() {
                        play = Some(PlayItem {
                            title: c.name.clone(),
                            url: c.stream_url.clone(),
                            kind: PlayKind::Live,
                            progress_key: format!("live:{}", c.id),
                        });
                    }
                    ui.add_space(2.0);
                }
                ui.add_space(10.0);
            }

            if !movies.is_empty() {
                section(ui, &format!("Filme ({})", movies.len()));
                for m in &movies {
                    let (rect, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 54.0), Sense::click());
                    let dark = ui.visuals().dark_mode;
                    let bg = if resp.hovered() {
                        if dark {
                            Color32::from_white_alpha(18)
                        } else {
                            Color32::from_black_alpha(10)
                        }
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    let mut row = ui.child_ui(rect, Layout::left_to_right(Align::Center));
                    row.add_space(10.0);
                    ui::image_or_placeholder(
                        app,
                        &mut row,
                        &format!("cover:{}", m.id),
                        &m.cover,
                        Vec2::new(28.0, 40.0),
                    );
                    row.add_space(12.0);
                    row.vertical(|ui| {
                        ui::one_line(ui, &m.name, 13.5, true);
                    });
                    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui.button("▶").on_hover_text("Film abspielen").clicked() {
                            play = Some(PlayItem {
                                title: m.name.clone(),
                                url: m.stream_url.clone(),
                                kind: PlayKind::Vod,
                                progress_key: format!("vod:{}", m.id),
                            });
                        }
                    });

                    if resp.clicked() {
                        play = Some(PlayItem {
                            title: m.name.clone(),
                            url: m.stream_url.clone(),
                            kind: PlayKind::Vod,
                            progress_key: format!("vod:{}", m.id),
                        });
                    }
                    ui.add_space(2.0);
                }
                ui.add_space(10.0);
            }

            if !series.is_empty() {
                section(ui, &format!("Serien ({})", series.len()));
                for s in &series {
                    let (rect, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 54.0), Sense::click());
                    let dark = ui.visuals().dark_mode;
                    let bg = if resp.hovered() {
                        if dark {
                            Color32::from_white_alpha(18)
                        } else {
                            Color32::from_black_alpha(10)
                        }
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    let mut row = ui.child_ui(rect, Layout::left_to_right(Align::Center));
                    row.add_space(10.0);
                    ui::image_or_placeholder(
                        app,
                        &mut row,
                        &format!("cover:{}", s.id),
                        &s.cover,
                        Vec2::new(28.0, 40.0),
                    );
                    row.add_space(12.0);
                    row.vertical(|ui| {
                        ui::one_line(ui, &s.name, 13.5, true);
                    });
                    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui
                            .button("Öffnen")
                            .on_hover_text("Staffeln & Folgen anzeigen")
                            .clicked()
                        {
                            open_series = Some(s.id);
                        }
                    });

                    if resp.clicked() {
                        open_series = Some(s.id);
                    }
                    ui.add_space(2.0);
                }
            }
        });

    if let Some(item) = play {
        app.play(item);
    }
    if let Some(id) = open_series {
        app.open_series(id);
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(4.0);
    ui.add(egui::Label::new(
        RichText::new(title)
            .size(13.0)
            .strong()
            .color(theme::accent(ui)),
    ));
    ui.add_space(4.0);
}
