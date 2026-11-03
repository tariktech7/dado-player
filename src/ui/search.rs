use crate::app::{App, View};
use crate::types::{Channel, PlayItem, PlayKind, Series, VodMovie};
use crate::ui;
use egui::{RichText, Ui};

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new("Suche").size(22.0).strong()));
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(&mut app.search_q)
            .hint_text("Kanal, Film oder Serie suchen…")
            .desired_width(360.0),
    );
    ui.add_space(4.0);

    let q = app.search_q.trim().to_lowercase();
    if q.is_empty() {
        ui.add(egui::Label::new(
            RichText::new("Suchbegriff eingeben.").size(13.0).weak(),
        ));
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
                .take(100)
                .cloned()
                .collect();
            let movies: Vec<VodMovie> = app
                .library
                .movies
                .iter()
                .filter(|m| m.name.to_lowercase().contains(&q))
                .take(100)
                .cloned()
                .collect();
            let series: Vec<Series> = app
                .library
                .series
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&q))
                .take(100)
                .cloned()
                .collect();

            if !chans.is_empty() {
                section(ui, &format!("Kanäle ({})", chans.len()));
                for c in &chans {
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::Label::new(RichText::new(&c.name).size(14.0))
                                .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            play = Some(PlayItem {
                                title: c.name.clone(),
                                url: c.stream_url.clone(),
                                kind: PlayKind::Live,
                                progress_key: format!("live:{}", c.id),
                            });
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("▶").clicked() {
                                play = Some(PlayItem {
                                    title: c.name.clone(),
                                    url: c.stream_url.clone(),
                                    kind: PlayKind::Live,
                                    progress_key: format!("live:{}", c.id),
                                });
                            }
                        });
                    });
                }
                ui.add_space(6.0);
            }

            if !movies.is_empty() {
                section(ui, &format!("Filme ({})", movies.len()));
                for m in &movies {
                    ui.horizontal(|ui| {
                        ui::image_or_placeholder(
                            app,
                            ui,
                            &format!("cover:{}", m.id),
                            &m.cover,
                            egui::vec2(30.0, 42.0),
                        );
                        let resp = ui.add(
                            egui::Label::new(RichText::new(&m.name).size(14.0))
                                .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            play = Some(PlayItem {
                                title: m.name.clone(),
                                url: m.stream_url.clone(),
                                kind: PlayKind::Vod,
                                progress_key: format!("vod:{}", m.id),
                            });
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("▶").clicked() {
                                play = Some(PlayItem {
                                    title: m.name.clone(),
                                    url: m.stream_url.clone(),
                                    kind: PlayKind::Vod,
                                    progress_key: format!("vod:{}", m.id),
                                });
                            }
                        });
                    });
                }
                ui.add_space(6.0);
            }

            if !series.is_empty() {
                section(ui, &format!("Serien ({})", series.len()));
                for s in &series {
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::Label::new(RichText::new(&s.name).size(14.0))
                                .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            open_series = Some(s.id);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Öffnen").clicked() {
                                open_series = Some(s.id);
                            }
                        });
                    });
                }
            }
        });

    if let Some(item) = play {
        app.play(item);
    }
    if let Some(id) = open_series {
        app.selected_series = Some(id);
        app.series_season = 0;
        app.request_series(id);
        app.view = View::Series;
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.add(egui::Label::new(RichText::new(title).size(12.0).weak()));
    ui.add_space(2.0);
}
