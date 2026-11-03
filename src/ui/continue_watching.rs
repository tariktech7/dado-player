use crate::app::{App, ContinueTab};
use crate::types::{PlayItem, PlayKind};
use crate::ui;
use egui::{RichText, Sense, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new("Fortsetzen").size(22.0).strong()));
        ui.add_space(14.0);
        let cur = match app.continue_tab {
            ContinueTab::Live => 0,
            ContinueTab::Movies => 1,
            ContinueTab::Series => 2,
        };
        if let Some(i) = ui::segmented(ui, "continue_tab", &["Live-TV", "Filme", "Serien"], cur)
        {
            app.continue_tab = match i {
                0 => ContinueTab::Live,
                1 => ContinueTab::Movies,
                _ => ContinueTab::Series,
            };
        }
    });
    ui.add_space(6.0);

    match app.continue_tab {
        ContinueTab::Live => live_tab(app, ui),
        ContinueTab::Movies => movies_tab(app, ui),
        ContinueTab::Series => series_tab(app, ui),
    }
}

fn empty(ui: &mut Ui, text: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Label::new(RichText::new(text).size(13.0).weak()));
    });
}

/// Recently watched channels, in the same row style as the Live list.
fn live_tab(app: &mut App, ui: &mut Ui) {
    let keys: Vec<(String, String)> = app
        .history
        .iter()
        .filter(|h| h.key.starts_with("live:"))
        .map(|h| (h.key.clone(), h.title.clone()))
        .collect();
    if keys.is_empty() {
        empty(ui, "Noch kein Sender gesehen.");
        return;
    }

    let mut play: Option<PlayItem> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (key, title) in keys {
                let id: Option<u32> = key
                    .strip_prefix("live:")
                    .and_then(|s| s.parse().ok());
                let ch = id.and_then(|id| app.library.channels.iter().find(|c| c.id == id).cloned());

                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui::ROW_H),
                    Sense::click(),
                );
                if resp.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        10.0,
                        ui.visuals().widgets.hovered.weak_bg_fill,
                    );
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let mut row = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center));
                row.add_space(10.0);
                let logo = ch.as_ref().map(|c| c.logo.clone()).unwrap_or_default();
                ui::image_or_placeholder(
                    app,
                    &mut row,
                    &format!("logo:{}", id.unwrap_or(0)),
                    &logo,
                    Vec2::splat(38.0),
                );
                row.add_space(10.0);
                row.vertical(|ui| {
                    ui::one_line(ui, &title, 15.0, true);
                    ui.add(
                        egui::Label::new(RichText::new("Live-TV · zuletzt gesehen").size(11.5).weak()),
                    );
                });
                row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui.button("▶").on_hover_text("Abspielen").clicked() {
                        if let Some(c) = &ch {
                            play = Some(PlayItem {
                                title: c.name.clone(),
                                url: c.stream_url.clone(),
                                kind: PlayKind::Live,
                                progress_key: format!("live:{}", c.id),
                            });
                        }
                    }
                });
                if resp.clicked() {
                    if let Some(c) = &ch {
                        play = Some(PlayItem {
                            title: c.name.clone(),
                            url: c.stream_url.clone(),
                            kind: PlayKind::Live,
                            progress_key: format!("live:{}", c.id),
                        });
                    }
                }
                ui.add_space(2.0);
            }
        });

    if let Some(item) = play {
        app.play(item);
    }
}

/// Movies as a poster grid – identical look to the "Filme" view.
fn movies_tab(app: &mut App, ui: &mut Ui) {
    let keys: Vec<(String, String)> = app
        .history
        .iter()
        .filter(|h| h.key.starts_with("vod:"))
        .map(|h| (h.key.clone(), h.title.clone()))
        .collect();
    if keys.is_empty() {
        empty(ui, "Noch keinen Film gesehen.");
        return;
    }

    let mut play: Option<PlayItem> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (per_row, gap) = ui::grid_metrics(ui);
            for chunk in keys.chunks(per_row) {
                ui.horizontal_top(|ui| {
                    for (key, title) in chunk {
                        let id: Option<u32> =
                            key.strip_prefix("vod:").and_then(|s| s.parse().ok());
                        let movie = id.and_then(|id| {
                            app.library.movies.iter().find(|m| m.id == id).cloned()
                        });
                        let cover = movie.as_ref().map(|m| m.cover.clone()).unwrap_or_default();
                        let at = app.progress.get(key).copied().unwrap_or(0);

                        ui.vertical(|ui| {
                            ui.set_width(ui::CARD_W);
                            let resp = ui::poster(app, ui, &format!("cover:{}", id.unwrap_or(0)), &cover, false);
                            if resp.clicked() {
                                if let Some(m) = &movie {
                                    play = Some(PlayItem {
                                        title: m.name.clone(),
                                        url: m.stream_url.clone(),
                                        kind: PlayKind::Vod,
                                        progress_key: key.clone(),
                                    });
                                }
                            }
                            ui.add_space(4.0);
                            ui::one_line(ui, &title, 13.0, false);
                            if at > 0 {
                                ui.add(egui::Label::new(
                                    RichText::new(format!("Weiter bei {}", ui::fmt_time(at as f64)))
                                        .size(11.0)
                                        .color(crate::theme::ACCENT),
                                ));
                            }
                        });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(10.0);
            }
        });

    if let Some(item) = play {
        app.play(item);
    }
}

/// Series as a poster grid; a click continues the last watched episode.
fn series_tab(app: &mut App, ui: &mut Ui) {
    let entries: Vec<(String, String, String)> = app
        .history
        .iter()
        .filter(|h| h.key.starts_with("ep:"))
        .map(|h| (h.key.clone(), h.title.clone(), h.sub.clone()))
        .collect();
    if entries.is_empty() {
        empty(ui, "Noch keine Serie gesehen.");
        return;
    }

    let mut play: Option<PlayItem> = None;
    let mut open_detail: Option<u32> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (per_row, gap) = ui::grid_metrics(ui);
            for chunk in entries.chunks(per_row) {
                ui.horizontal_top(|ui| {
                    for (key, title, sub) in chunk {
                        let parts: Vec<&str> = key.strip_prefix("ep:").unwrap_or("").split(':').collect();
                        let sid = parts.first().and_then(|s| s.parse::<u32>().ok());
                        let eid = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                        let series = sid.and_then(|sid| {
                            app.library.series.iter().find(|s| s.id == sid).cloned()
                        });
                        let cover = series.as_ref().map(|s| s.cover.clone()).unwrap_or_default();
                        let episode = sid
                            .zip(eid)
                            .and_then(|(sid, eid)| {
                                app.series_episodes
                                    .get(&sid)
                                    .and_then(|v| v.iter().find(|e| e.id == eid).cloned())
                            });

                        ui.vertical(|ui| {
                            ui.set_width(ui::CARD_W);
                            let resp = ui::poster(
                                app,
                                ui,
                                &format!("cover:s{}", sid.unwrap_or(0)),
                                &cover,
                                false,
                            );
                            if resp.clicked() {
                                match &episode {
                                    Some(e) => {
                                        play = Some(PlayItem {
                                            title: title.clone(),
                                            url: e.stream_url.clone(),
                                            kind: PlayKind::Episode,
                                            progress_key: key.clone(),
                                        });
                                    }
                                    None => open_detail = sid,
                                }
                            }
                            ui.add_space(4.0);
                            ui::one_line(ui, title, 13.0, false);
                            ui.add(egui::Label::new(
                                RichText::new(sub)
                                    .size(11.0)
                                    .color(crate::theme::ACCENT),
                            ));
                        });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(10.0);
            }
        });

    if let Some(item) = play {
        app.play(item);
    } else if let Some(sid) = open_detail {
        app.selected_series = Some(sid);
        app.series_season = 0;
        app.request_series(sid);
        app.view = crate::app::View::Series;
    }
}
