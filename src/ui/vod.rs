use crate::app::App;
use crate::types::{PlayItem, PlayKind, VodMovie};
use crate::ui;
use egui::{RichText, Ui};

const PER_PAGE: usize = 48;

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new("Filme").size(22.0).strong()));
        ui.add_space(8.0);

        let selected = if app.vod_cat == "all" {
            "Alle Kategorien".to_string()
        } else {
            app.library
                .vod_categories
                .iter()
                .find(|c| c.id == app.vod_cat)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Alle Kategorien".to_string())
        };

        let mut cat_changed = false;
        egui::ComboBox::from_id_source("vod_cat")
            .selected_text(selected)
            .width(210.0)
            .show_ui(ui, |cb| {
                cat_changed |= cb
                    .selectable_value(&mut app.vod_cat, "all".to_string(), "Alle Kategorien")
                    .clicked();
                for c in &app.library.vod_categories {
                    cat_changed |= cb
                        .selectable_value(&mut app.vod_cat, c.id.clone(), c.name.clone())
                        .clicked();
                }
            });
        if cat_changed {
            app.vod_page = 0;
        }
    });
    ui.add_space(2.0);

    let cat = if app.vod_cat == "all" {
        None
    } else {
        Some(app.vod_cat.clone())
    };
    let idx: Vec<usize> = app
        .library
        .movies
        .iter()
        .enumerate()
        .filter(|(_, m)| cat.as_ref().map_or(true, |id| &m.category_id == id))
        .map(|(i, _)| i)
        .collect();

    let total = idx.len();
    let pages = (total + PER_PAGE - 1) / PER_PAGE;
    let page = app.vod_page.min(pages.saturating_sub(1));
    app.vod_page = page;
    let start = page * PER_PAGE;
    let end = (start + PER_PAGE).min(total);

    ui::pager(ui, &mut app.vod_page, pages, total, "Filme");
    ui.add_space(4.0);

    let slice: Vec<VodMovie> = idx[start..end]
        .iter()
        .map(|&i| app.library.movies[i].clone())
        .collect();

    let mut play: Option<PlayItem> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (per_row, gap) = ui::grid_metrics(ui);
            for chunk in slice.chunks(per_row) {
                ui.horizontal_top(|ui| {
                    for m in chunk {
                        ui.vertical(|ui| {
                            ui.set_width(ui::CARD_W);
                            let resp = ui::poster(
                                app,
                                ui,
                                &format!("cover:{}", m.id),
                                &m.cover,
                                false,
                            );
                            if resp.clicked() {
                                play = Some(PlayItem {
                                    title: m.name.clone(),
                                    url: m.stream_url.clone(),
                                    kind: PlayKind::Vod,
                                    progress_key: format!("vod:{}", m.id),
                                });
                            }
                            ui.add_space(2.0);
                            ui::one_line(ui, &m.name, 13.0, false);

                            ui.horizontal(|ui| {
                                ui::fav_button(app, ui, &format!("vod:{}", m.id));
                                if !m.rating.is_empty() {
                                    ui.add(egui::Label::new(
                                        RichText::new(format!("{} / 10", m.rating)).size(11.0).weak(),
                                    ));
                                }
                            });
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
