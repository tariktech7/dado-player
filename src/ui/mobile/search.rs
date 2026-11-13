//! Phone search: one big field, grouped results, no filters to configure.

use egui::Ui;

use super::components::{self, Icon, Kb, Row};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::types::{PlayItem, PlayKind};

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    let mut focus_rect = None;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(l.pad);
        ui.vertical(|ui| {
            ui.set_width(l.content_w());
            let r = components::field(
                ui,
                "",
                &mut app.search_q,
                "Sender, Film oder Serie",
                Kb::Text,
                &mut false, // search_q doesn't use password toggle
            );
            // Register the field with the remote focus engine; activating it
            // opens the IME just like a tap would.
            if focus::clicked(ui, &r) {
                r.request_focus();
            }
            if r.gained_focus() {
                focus_rect = Some(r.rect);
                crate::platform::seed_ime(&app.search_q);
            }
            // Autofocus once per visit so the keyboard opens right away.
            let id = ui.id().with("search-autofocus");
            let done = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
            if !done {
                r.request_focus();
                ui.data_mut(|d| d.insert_temp(id, true));
            }
        });
    });

    let q = app.search_q.trim().to_lowercase();
    if q.len() < 2 {
        components::empty_state(
            ui,
            Icon::Search,
            "Wonach suchst du?",
            "Mindestens zwei Zeichen eingeben.",
            None,
        );
        return;
    }

    let mut play: Option<PlayItem> = None;
    let mut open_series: Option<u32> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);

            let mut empty = true;
            let channels: Vec<_> = app
                .library
                .channels
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&q))
                .take(20)
                .cloned()
                .collect();
            if !channels.is_empty() {
                empty = false;
                header(ui, "Sender");
                for c in channels {
                    let key = format!("live:{}", c.id);
                    let img_key = format!("logo:{}", c.id);
                    let row = Row::new(&c.name).img(&img_key, &c.logo).sub("Live-TV");
                    let r = components::list_row(app, ui, row, l.pad);
                    if focus::clicked(ui, &r) {
                        play = Some(PlayItem {
                            title: c.name.clone(),
                            url: c.stream_url.clone(),
                            kind: PlayKind::Live,
                            progress_key: key,
                        });
                    }
                }
            }

            let movies: Vec<_> = app
                .library
                .movies
                .iter()
                .filter(|m| m.name.to_lowercase().contains(&q))
                .take(20)
                .cloned()
                .collect();
            if !movies.is_empty() {
                empty = false;
                header(ui, "Filme");
                for m in movies {
                    let img_key = format!("cover:{}", m.id);
                    let row = Row::new(&m.name).img(&img_key, &m.cover).sub("Film");
                    let r = components::list_row(app, ui, row, l.pad);
                    if focus::clicked(ui, &r) {
                        play = Some(PlayItem {
                            title: m.name.clone(),
                            url: m.stream_url.clone(),
                            kind: PlayKind::Vod,
                            progress_key: format!("vod:{}", m.id),
                        });
                    }
                }
            }

            let series: Vec<_> = app
                .library
                .series
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&q))
                .take(20)
                .cloned()
                .collect();
            if !series.is_empty() {
                empty = false;
                header(ui, "Serien");
                for s in series {
                    let img_key = format!("scov:{}", s.id);
                    let row = Row::new(&s.name).img(&img_key, &s.cover).sub("Serie");
                    let r = components::list_row(app, ui, row, l.pad);
                    if focus::clicked(ui, &r) {
                        open_series = Some(s.id);
                    }
                }
            }

            if empty {
                components::empty_state(
                    ui,
                    Icon::Search,
                    "Nichts gefunden",
                    "Andere Schreibweise versuchen.",
                    None,
                );
            }
            // Mobile: clear the bottom nav bar (NAV_H) plus a gap.
            ui.add_space(GAP);
        });

    if let Some(item) = play {
        app.play(item);
    }
    if let Some(id) = open_series {
        app.view = crate::app::View::Series;
        app.selected_series = Some(id);
        app.series_season = 0;
        app.request_series(id);
    }
    if let Some(r) = focus_rect {
        ui.scroll_to_rect(r, Some(egui::Align::Center));
    }
}

fn header(ui: &mut Ui, text: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(
            egui::RichText::new(text)
                .font(bold(FS_CAP + 0.5))
                .color(muted(ui)),
        );
    });
    ui.add_space(2.0);
}
