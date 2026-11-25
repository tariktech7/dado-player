use crate::app::App;
use crate::theme;
use crate::types::{Channel, PlayItem, PlayKind, Series, VodMovie};
use crate::ui;
use egui::{Align, Color32, Layout, RichText, Rounding, Sense, Stroke, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new("Favoriten").size(22.0).strong(),
        ));
        ui.add_space(8.0);
        ui.add(egui::Label::new(
            RichText::new(format!("{} gespeichert", app.favorites.len()))
                .size(11.0)
                .weak(),
        ));
    });
    ui.add_space(6.0);

    category_tabs(app, ui);
    let favs = app.favorites.clone();

    // Collect favorites by type
    let chans: Vec<Channel> = app
        .library
        .channels
        .iter()
        .filter(|c| favs.contains(&format!("live:{}", c.id)))
        .cloned()
        .collect();
    let movies: Vec<VodMovie> = app
        .library
        .movies
        .iter()
        .filter(|m| favs.contains(&format!("vod:{}", m.id)))
        .cloned()
        .collect();
    let series: Vec<Series> = app
        .library
        .series
        .iter()
        .filter(|s| favs.contains(&format!("series:{}", s.id)))
        .cloned()
        .collect();

    if chans.is_empty() && movies.is_empty() && series.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Label::new(RichText::new("⭐").size(48.0).color(theme::muted(ui))));
            ui.add_space(12.0);
            ui.add(egui::Label::new(RichText::new("Noch keine Favoriten").size(17.0).strong()));
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new("Tippe in einer Liste auf den Stern, um Sender, Filme oder Serien hier zu sammeln.").size(12.0).color(theme::muted(ui))).wrap(true));
            ui.add_space(18.0);
            if ui.button("Zu Live-TV").clicked() {
                app.view = crate::app::View::Live;
            }
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if app.favorite_section == 0 && !chans.is_empty() {
                section_title(ui, "Live-TV Kanäle", chans.len());
                render_channels(app, ui, &chans);
                ui.add_space(10.0);
            }
            if app.favorite_section == 2 && !movies.is_empty() {
                section_title(ui, "Filme", movies.len());
                render_movies(app, ui, &movies);
                ui.add_space(10.0);
            }
            if app.favorite_section == 1 && !series.is_empty() {
                section_title(ui, "Serien", series.len());
                render_series(app, ui, &series);
            }
        });
}

fn section_title(ui: &mut Ui, title: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(15.0).strong());
        ui.label(
            RichText::new(format!("({})", count))
                .size(11.0)
                .color(theme::muted(ui)),
        );
    });
    ui.add_space(4.0);
}

fn render_channels(app: &mut App, ui: &mut Ui, chans: &[Channel]) {
    let mut play: Option<PlayItem> = None;
    for c in chans {
        let key = format!("live:{}", c.id);
        let playing = app.is_previewing(&key);

        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.0), Sense::click());
        let dark = ui.visuals().dark_mode;
        let bg = if playing {
            theme::selection_surface(ui)
        } else if resp.hovered() {
            if dark {
                Color32::from_white_alpha(18)
            } else {
                Color32::from_black_alpha(10)
            }
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, Rounding::same(10.0), bg);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if playing {
            ui.painter().rect_stroke(
                rect,
                Rounding::same(10.0),
                Stroke::new(1.0_f32, theme::accent(ui)),
            );
        }

        let mut row = ui.child_ui(rect, Layout::left_to_right(Align::Center));
        row.add_space(12.0);
        // Logo
        ui::image_or_placeholder(
            app,
            &mut row,
            &format!("logo:{}", c.id),
            &c.logo,
            Vec2::splat(44.0),
        );
        row.add_space(14.0);
        // Name + channel number
        row.vertical(|ui| {
            let name_row = format!(
                "{}  {}",
                if c.num > 0 {
                    format!("{:>3}", c.num)
                } else {
                    "".to_string()
                },
                c.name
            );
            ui::one_line(ui, &name_row, 14.0, true);
            let cat = app
                .library
                .live_categories
                .iter()
                .find(|cat| cat.id == c.category_id)
                .map(|cat| cat.name.clone())
                .unwrap_or_default();
            if !cat.is_empty() {
                ui.add(egui::Label::new(
                    RichText::new(cat).size(10.5).color(theme::muted(ui)),
                ));
            }
        });
        row.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(8.0);
            ui::fav_button(app, ui, &key);
            ui.add_space(8.0);
            if ui.button("▶").on_hover_text("Abspielen").clicked() {
                play = Some(PlayItem {
                    title: c.name.clone(),
                    url: c.stream_url.clone(),
                    kind: PlayKind::Live,
                    progress_key: key.clone(),
                });
            }
            ui.add_space(8.0);
        });

        if resp.clicked() && play.is_none() {
            play = Some(PlayItem {
                title: c.name.clone(),
                url: c.stream_url.clone(),
                kind: PlayKind::Live,
                progress_key: key.clone(),
            });
        }
        ui.add_space(2.0);
    }
    if let Some(item) = play {
        app.play_selected(item);
    }
}

fn render_movies(app: &mut App, ui: &mut Ui, movies: &[VodMovie]) {
    let mut play: Option<PlayItem> = None;
    egui::ScrollArea::horizontal()
        .id_source("fav-movies")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for m in movies {
                    ui.vertical(|ui| {
                        ui.set_width(ui::CARD_W);
                        let resp = ui::poster_badge(
                            app,
                            ui,
                            &format!("cover:{}", m.id),
                            &m.cover,
                            false,
                            crate::ui::vod::rating_badge_for_home(&m.rating).as_deref(),
                        );
                        if resp.clicked() {
                            play = Some(PlayItem {
                                title: m.name.clone(),
                                url: m.stream_url.clone(),
                                kind: PlayKind::Vod,
                                progress_key: format!("vod:{}", m.id),
                            });
                        }
                        ui.add_space(3.0);
                        ui::one_line(ui, &m.name, 11.0, true);
                        ui.horizontal(|ui| {
                            ui::fav_button(app, ui, &format!("vod:{}", m.id));
                        });
                    });
                }
            });
        });
    if let Some(item) = play {
        app.play_selected(item);
    }
}

fn render_series(app: &mut App, ui: &mut Ui, series: &[Series]) {
    let mut open: Option<u32> = None;
    egui::ScrollArea::horizontal()
        .id_source("fav-series")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for s in series {
                    ui.vertical(|ui| {
                        ui.set_width(ui::CARD_W);
                        let selected = app.selected_series == Some(s.id);
                        let resp =
                            ui::poster(app, ui, &format!("cover:{}", s.id), &s.cover, selected);
                        if resp.clicked() {
                            open = Some(s.id);
                        }
                        ui.add_space(3.0);
                        ui::one_line(ui, &s.name, 11.0, selected);
                        ui.horizontal(|ui| {
                            ui::fav_button(app, ui, &format!("series:{}", s.id));
                        });
                    });
                }
            });
        });
    if let Some(id) = open {
        app.open_series(id);
    }
}

pub fn category_tabs(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for (i, label) in ["Live-TV", "Serien", "Filme"].iter().enumerate() {
            let r = ui.selectable_label(app.favorite_section == i, *label);
            let clicked = if app.shared_remote_navigation() {
                crate::ui::mobile::focus::clicked(ui, &r)
            } else {
                r.clicked()
            };
            if clicked {
                app.favorite_section = i;
            }
        }
    });
    ui.add_space(10.0);
}
