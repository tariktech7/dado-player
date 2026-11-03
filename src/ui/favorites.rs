use crate::app::App;
use crate::types::{Channel, PlayItem, PlayKind, VodMovie};
use crate::ui;
use egui::Ui;

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.heading("Favoriten");
    let favs = app.favorites.clone();

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

    if chans.is_empty() && movies.is_empty() {
        ui.label("Noch keine Favoriten. Tippe auf ☆ in einer Liste.");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        if !chans.is_empty() {
            ui.heading("Kanäle");
            for c in &chans {
                ui.horizontal(|ui| {
                    ui.label(&c.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui::fav_button(app, ui, &format!("live:{}", c.id));
                        if ui.button("▶").clicked() {
                            app.play(PlayItem {
                                title: c.name.clone(),
                                url: c.stream_url.clone(),
                                kind: PlayKind::Live,
                                progress_key: format!("live:{}", c.id),
                            });
                        }
                    });
                });
            }
        }
        if !movies.is_empty() {
            ui.heading("Filme");
            for m in &movies {
                ui.horizontal(|ui| {
                    ui::image_or_placeholder(app, ui, &format!("cover:{}", m.id), &m.cover, egui::vec2(32.0, 44.0));
                    ui.label(&m.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui::fav_button(app, ui, &format!("vod:{}", m.id));
                        if ui.button("▶").clicked() {
                            app.play(PlayItem {
                                title: m.name.clone(),
                                url: m.stream_url.clone(),
                                kind: PlayKind::Vod,
                                progress_key: format!("vod:{}", m.id),
                            });
                        }
                    });
                });
            }
        }
    });
}
