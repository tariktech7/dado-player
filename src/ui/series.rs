use crate::app::App;
use crate::theme;
use crate::types::{PlayItem, PlayKind, Series};
use crate::ui;
use egui::{Align, Layout, RichText, Sense, Ui, Vec2};

const PER_PAGE: usize = 48;

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new("Serien").size(22.0).strong(),
        ));
        ui.add_space(8.0);

        let selected = if app.series_cat == "all" {
            "Alle Kategorien".to_string()
        } else {
            app.library
                .series_categories
                .iter()
                .find(|c| c.id == app.series_cat)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Alle Kategorien".to_string())
        };

        let mut cat_changed = false;
        egui::ComboBox::from_id_source("series_cat")
            .selected_text(selected)
            .width(210.0)
            .show_ui(ui, |cb| {
                cat_changed |= cb
                    .selectable_value(&mut app.series_cat, "all".to_string(), "Alle Kategorien")
                    .clicked();
                for c in &app.library.series_categories {
                    cat_changed |= cb
                        .selectable_value(&mut app.series_cat, c.id.clone(), c.name.clone())
                        .clicked();
                }
            });
        if cat_changed {
            app.series_page = 0;
        }
    });
    ui.add_space(2.0);

    let cat = if app.series_cat == "all" {
        None
    } else {
        Some(app.series_cat.clone())
    };
    let idx: Vec<usize> = app
        .library
        .series
        .iter()
        .enumerate()
        .filter(|(_, s)| cat.as_ref().is_none_or(|id| &s.category_id == id))
        .map(|(i, _)| i)
        .collect();

    let total = idx.len();
    let pages = total.div_ceil(PER_PAGE);
    let page = app.series_page.min(pages.saturating_sub(1));
    app.series_page = page;
    let start = page * PER_PAGE;
    let end = (start + PER_PAGE).min(total);

    ui::pager(ui, &mut app.series_page, pages, total, "Serien");
    ui.add_space(4.0);

    let slice: Vec<Series> = idx[start..end]
        .iter()
        .map(|&i| app.library.series[i].clone())
        .collect();

    let mut select: Option<u32> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (per_row, gap) = ui::grid_metrics(ui);
            for chunk in slice.chunks(per_row) {
                ui.horizontal_top(|ui| {
                    for s in chunk {
                        ui.vertical(|ui| {
                            ui.set_width(ui::CARD_W);
                            let selected = app.selected_series == Some(s.id);
                            let resp =
                                ui::poster(app, ui, &format!("cover:{}", s.id), &s.cover, selected);
                            if resp.clicked() {
                                select = Some(s.id);
                            }
                            ui.add_space(2.0);
                            ui::one_line(ui, &s.name, 13.0, selected);
                        });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(10.0);
            }
            if pages > 1 {
                ui.add_space(12.0);
                ui::pager(ui, &mut app.series_page, pages, total, "Serien");
                ui.add_space(16.0);
            }
        });

    if let Some(id) = select {
        if app.selected_series == Some(id) {
            app.selected_series = None;
        } else {
            app.selected_series = Some(id);
            app.series_season = 0;
            app.request_series(id);
        }
    }
}

/// Detail sheet for the selected series: seasons + episodes, list stays visible.
pub fn detail_panel(app: &mut App, ui: &mut Ui) {
    let id = match app.selected_series {
        Some(i) => i,
        None => return,
    };
    let s = match app.library.series.iter().find(|s| s.id == id) {
        Some(s) => s.clone(),
        None => {
            app.selected_series = None;
            return;
        }
    };

    let mut close = false;
    let mut play: Option<PlayItem> = None;
    let muted = theme::muted(ui);

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new("Serie").size(18.0).strong()));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("×").on_hover_text("Schließen").clicked() {
                close = true;
            }
        });
    });
    ui.add_space(4.0);

    ui.horizontal_top(|ui| {
        ui::image_or_placeholder(
            app,
            ui,
            &format!("cover:{}", s.id),
            &s.cover,
            Vec2::new(104.0, 150.0),
        );
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(RichText::new(&s.name).size(17.0).strong()).wrap(true));
            let cat = app
                .library
                .series_categories
                .iter()
                .find(|c| c.id == s.category_id)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            if !cat.is_empty() {
                ui.add(egui::Label::new(RichText::new(cat).size(11.5).color(muted)));
            }
        });
    });

    if !s.plot.is_empty() {
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_source("plot")
            .max_height(110.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(&s.plot).size(12.5).color(muted)).wrap(true));
            });
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(2.0);

    let eps_opt = app.series_episodes_for(id).cloned();
    match eps_opt {
        None if app.series_failed(id) => {
            // A failed request is not "no episodes". Show the real cause and a
            // retry, otherwise the series looks permanently empty.
            ui.add(egui::Label::new(
                RichText::new("Episoden konnten nicht geladen werden.")
                    .size(12.5)
                    .color(muted),
            ));
            if let Some(detail) = app.series_error.clone() {
                ui.add(egui::Label::new(RichText::new(detail).size(11.0).weak()).wrap(true));
            }
            ui.add_space(6.0);
            if ui.button("Erneut versuchen").clicked() {
                app.retry_series(id);
                ui.ctx().request_repaint();
            }
        }
        None => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.add(egui::Label::new(
                    RichText::new("Lade Episoden…").size(12.5).color(muted),
                ));
            });
        }
        Some(eps) if eps.is_empty() => {
            ui.add(egui::Label::new(
                RichText::new("Für diese Serie wurden keine Episoden gefunden.").size(12.5),
            ));
        }
        Some(eps) => {
            let mut seasons: Vec<u32> = eps.iter().map(|e| e.season).collect();
            seasons.sort_unstable();
            seasons.dedup();
            if !seasons.contains(&app.series_season) {
                app.series_season = *seasons.first().unwrap_or(&1);
            }

            ui.horizontal_wrapped(|ui| {
                for sn in &seasons {
                    let sel = app.series_season == *sn;
                    if ui
                        .selectable_label(sel, format!("Staffel {}", sn))
                        .clicked()
                    {
                        app.series_season = *sn;
                    }
                }
            });
            ui.add_space(4.0);

            let season = app.series_season;
            egui::ScrollArea::vertical()
                .id_source("eps")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for e in eps.iter().filter(|e| e.season == season) {
                        let key = format!("ep:{}:{}", e.series_id, e.id);
                        let (rect, resp) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 42.0),
                            Sense::click(),
                        );
                        let is_playing = app.is_previewing(&key);
                        if resp.hovered() || is_playing {
                            ui.painter().rect_filled(
                                rect,
                                8.0,
                                ui.visuals().widgets.hovered.weak_bg_fill,
                            );
                        }
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        let mut row = ui.child_ui(rect, Layout::left_to_right(egui::Align::Center));
                        row.add_space(10.0);
                        row.add(egui::Label::new(
                            RichText::new(format!("E{:02}", e.episode))
                                .size(12.0)
                                .color(muted),
                        ));
                        row.add_space(10.0);
                        let title_w = (rect.width() - 120.0).max(60.0);
                        row.allocate_ui_with_layout(
                            Vec2::new(title_w, 42.0),
                            Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui::one_line(ui, &e.title, 13.0, false);
                            },
                        );
                        row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            if ui.button("▶").on_hover_text("Episode abspielen").clicked() {
                                play = Some(PlayItem {
                                    title: e.title.clone(),
                                    url: e.stream_url.clone(),
                                    kind: PlayKind::Episode,
                                    progress_key: key.clone(),
                                });
                            }
                            ui.add_space(10.0);
                        });

                        if resp.clicked() {
                            play = Some(PlayItem {
                                title: e.title.clone(),
                                url: e.stream_url.clone(),
                                kind: PlayKind::Episode,
                                progress_key: key.clone(),
                            });
                        }
                    }
                });
        }
    }

    if close {
        app.selected_series = None;
    }
    if let Some(item) = play {
        app.play(item);
    }
}
