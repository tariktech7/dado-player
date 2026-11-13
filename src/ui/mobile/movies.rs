//! Phone movie browser: chips + 3-column poster grid with "load more".

use egui::Ui;

use super::components::{self, Icon, PosterItem};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::types::{PlayItem, PlayKind};

const CHUNK: usize = 60;

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    if !app.library.loaded {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.spinner();
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Bibliothek wird geladen…")
                    .font(reg(FS_LABEL))
                    .color(muted(ui)),
            );
        });
        return;
    }

    // ---- chips ----
    let mut labels: Vec<(String, bool)> = Vec::with_capacity(app.library.vod_categories.len() + 1);
    labels.push(("Alle".to_string(), app.vod_cat == "all"));
    for c in &app.library.vod_categories {
        labels.push((c.name.clone(), app.vod_cat == c.id));
    }
    if let Some(i) = components::chip_row(ui, &labels, l.pad) {
        app.vod_cat = if i == 0 {
            "all".to_string()
        } else {
            app.library.vod_categories[i - 1].id.clone()
        };
        app.vod_page = 0;
    }
    ui.add_space(8.0);

    // ---- filter ----
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

    if idx.is_empty() {
        components::empty_state(
            ui,
            Icon::Film,
            "Keine Filme",
            "In dieser Kategorie ist nichts hinterlegt.",
            None,
        );
        return;
    }

    // ---- progressive loading state (per category) ----
    let state_id = ui.id().with("vod-chunk");
    let (saved_cat, mut shown) = ui
        .data(|d| d.get_temp::<(String, usize)>(state_id))
        .unwrap_or((app.vod_cat.clone(), CHUNK));
    if saved_cat != app.vod_cat {
        shown = CHUNK;
    }
    let n = idx.len().min(shown);

    let items: Vec<PosterItem> = idx[..n]
        .iter()
        .map(|&i| {
            let m = &app.library.movies[i];
            PosterItem {
                key: format!("cover:{}", m.id),
                url: m.cover.clone(),
                title: m.name.clone(),
                meta: if m.rating.is_empty() {
                    String::new()
                } else {
                    // No U+2605 here: a star glyph is not in the UI font family
                    // and would render as tofu on some devices.
                    format!("IMDb {}", m.rating)
                },
            }
        })
        .collect();

    let mut play: Option<PlayItem> = None;
    let mut load_more = false;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(2.0);
            if let Some(i) = components::poster_grid(app, ui, l, &items) {
                let m = &app.library.movies[idx[i]];
                play = Some(PlayItem {
                    title: m.name.clone(),
                    url: m.stream_url.clone(),
                    kind: PlayKind::Vod,
                    progress_key: format!("vod:{}", m.id),
                });
            }
            if n < idx.len() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(l.pad);
                    let r = components::button(ui, "Mehr laden", l.content_w(), false, true);
                    if focus::clicked(ui, &r) {
                        load_more = true;
                    }
                });
            }
            // Mobile: clear the fixed bottom nav bar (NAV_H) plus a gap.
            ui.add_space(GAP);
        });

    if load_more {
        shown = (shown + CHUNK).min(idx.len());
    }
    ui.data_mut(|d| d.insert_temp(state_id, (app.vod_cat.clone(), shown)));

    if let Some(item) = play {
        app.play(item);
    }
}
