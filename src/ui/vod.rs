use crate::app::App;
use crate::types::{PlayItem, PlayKind, VodMovie};
use crate::ui;
use egui::{RichText, Ui};

const PER_PAGE: usize = 48;

/// Providers send the rating as "7.8", "7.8 / 10", "0" or "" — normalise it to
/// one short badge and drop anything that is not a usable score.
pub(crate) fn rating_badge_for_home(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let num: String = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let value: f32 = num.replace(',', ".").parse().ok()?;
    if !(0.1..=10.0).contains(&value) {
        return None;
    }
    Some(format!("★ {}", t.split_whitespace().next().unwrap_or(t)))
}

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
    recently_added(app, ui);

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
        .filter(|(_, m)| cat.as_ref().is_none_or(|id| &m.category_id == id))
        .map(|(i, _)| i)
        .collect();

    let total = idx.len();
    let pages = total.div_ceil(PER_PAGE);
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
                            let resp = ui::poster_badge(
                                app,
                                ui,
                                &format!("cover:{}", m.id),
                                &m.cover,
                                false,
                                rating_badge_for_home(&m.rating).as_deref(),
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
                            });
                        });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(10.0);
            }
            if pages > 1 {
                ui.add_space(12.0);
                ui::pager(ui, &mut app.vod_page, pages, total, "Filme");
                ui.add_space(16.0);
            }
        });

    if let Some(item) = play {
        app.play(item);
    }
}

/// A compact horizontally scrolling rail. It appears only if the provider
/// supplied usable timestamps; no artificial ordering is inferred.
fn recently_added(app: &mut App, ui: &mut Ui) {
    let mut indices: Vec<usize> = app
        .library
        .movies
        .iter()
        .enumerate()
        .filter_map(|(i, m)| m.added.filter(|t| *t > 0).map(|t| (i, t)))
        .map(|(i, _)| i)
        .collect();
    indices.sort_by_key(|&i| std::cmp::Reverse(app.library.movies[i].added.unwrap_or(0)));
    indices.truncate(12);
    if indices.is_empty() {
        return;
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Zuletzt hinzugefügt").size(16.0).strong());
        ui.add_space(6.0);
        ui.label(RichText::new("Neu beim Anbieter").size(11.0).weak());
    });
    ui.add_space(4.0);

    let items: Vec<(u32, String, String, String, String)> = indices
        .iter()
        .map(|&i| {
            let m = &app.library.movies[i];
            (
                m.id,
                m.name.clone(),
                m.cover.clone(),
                m.stream_url.clone(),
                m.rating.clone(),
            )
        })
        .collect();
    let mut play: Option<PlayItem> = None;
    egui::ScrollArea::horizontal()
        .id_source("recently-added-movies")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (id, name, cover, url, rating) in &items {
                    ui.vertical(|ui| {
                        ui.set_width(ui::CARD_W);
                        let resp = ui::poster_badge(
                            app,
                            ui,
                            &format!("cover:{}", id),
                            cover,
                            false,
                            rating_badge_for_home(rating).as_deref(),
                        );
                        if resp.clicked() {
                            play = Some(PlayItem {
                                title: name.clone(),
                                url: url.clone(),
                                kind: PlayKind::Vod,
                                progress_key: format!("vod:{}", id),
                            });
                        }
                        ui.add_space(3.0);
                        ui::one_line(ui, name, 12.0, false);
                    });
                    ui.add_space(8.0);
                }
            });
        });
    if let Some(item) = play {
        app.play(item);
    }
}
