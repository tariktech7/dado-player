//! Shared category → title → details browser for desktop, phone and TV.
use crate::{
    app::App,
    types::{PlayItem, PlayKind},
    ui::mobile::{components, focus, tokens},
};
use egui::{pos2, vec2, Align, Layout, Rect, RichText, Sense, Ui};

#[derive(Clone)]
struct Entry {
    id: u32,
    name: String,
    cover: String,
    category: String,
    plot: String,
    meta: String,
    url: String,
    added: i64,
}
fn hit(ui: &Ui, r: &egui::Response, tv: bool) -> bool {
    if tv {
        focus::clicked(ui, r)
    } else {
        r.clicked()
    }
}
fn button(ui: &mut Ui, label: impl Into<egui::WidgetText>, tv: bool) -> bool {
    let r = ui.button(label);
    hit(ui, &r, tv)
}

pub fn show(app: &mut App, ui: &mut Ui, series: bool) {
    let id = if series { 1 } else { 0 };
    ui.push_id(("library", id, app.catalog_generation()), |ui| {
        let available = ui.available_rect_before_wrap();
        let narrow = available.width() < 620.0;
        let gap = 10.0;
        let category_w = if narrow {
            (available.width() * 0.28).clamp(84.0, 130.0)
        } else {
            (available.width() * 0.18).clamp(116.0, 200.0)
        };
        let details_w = if narrow {
            0.0
        } else {
            (available.width() * 0.36).clamp(230.0, 380.0)
        };
        let top_h = if narrow {
            (available.height() * 0.53).max(150.0)
        } else {
            available.height()
        };
        let categories = Rect::from_min_size(available.min, vec2(category_w, top_h));
        let list = Rect::from_min_max(
            pos2(categories.right() + gap, available.top()),
            pos2(
                available.right() - details_w - if narrow { 0.0 } else { gap },
                available.top() + top_h,
            ),
        );
        let details = if narrow {
            Rect::from_min_max(pos2(available.left(), list.bottom() + gap), available.max)
        } else {
            Rect::from_min_max(pos2(list.right() + gap, available.top()), available.max)
        };
        let mut category_ui = ui.child_ui(categories, Layout::top_down(Align::Min));
        category_ui.set_clip_rect(categories.intersect(ui.clip_rect()));
        focus::set_zone(1);
        category_list(app, &mut category_ui, series);
        let mut list_ui = ui.child_ui(list, Layout::top_down(Align::Min));
        list_ui.set_clip_rect(list.intersect(ui.clip_rect()));
        focus::set_zone(2);
        title_list(app, &mut list_ui, series);
        let mut details_ui = ui.child_ui(details, Layout::top_down(Align::Min));
        details_ui.set_clip_rect(details.intersect(ui.clip_rect()));
        focus::set_zone(3);
        details_panel(app, &mut details_ui, series);
    });
}
fn entries(app: &App, series: bool) -> Vec<Entry> {
    if series {
        app.library
            .series
            .iter()
            .map(|v| Entry {
                id: v.id,
                name: v.name.clone(),
                cover: v.cover.clone(),
                category: v.category_id.clone(),
                plot: v.plot.clone(),
                meta: "Serie".into(),
                url: String::new(),
                added: v.added.unwrap_or(0),
            })
            .collect()
    } else {
        app.library
            .movies
            .iter()
            .map(|v| Entry {
                id: v.id,
                name: v.name.clone(),
                cover: v.cover.clone(),
                category: v.category_id.clone(),
                plot: v.plot.clone(),
                meta: if v.rating.is_empty() {
                    "Film".into()
                } else {
                    format!("★ {}", v.rating)
                },
                url: v.stream_url.clone(),
                added: v.added.unwrap_or(0),
            })
            .collect()
    }
}
fn category_list(app: &mut App, ui: &mut Ui, series: bool) {
    let tv = app.tv;
    ui.label(
        RichText::new("Kategorien")
            .strong()
            .size(if tv { 16.0 } else { 15.0 }),
    );
    ui.add_space(8.0);
    let mut categories = vec![
        ("favorites".to_string(), "Favoriten".to_string()),
        ("all".into(), "Alle".into()),
        ("recent".into(), "Neu hinzugefügt".into()),
    ];
    categories.extend(
        if series {
            &app.library.series_categories
        } else {
            &app.library.vod_categories
        }
        .iter()
        .map(|v| (v.id.clone(), v.name.clone())),
    );
    let selected = if series {
        app.series_cat.clone()
    } else {
        app.vod_cat.clone()
    };
    let mut next = None;
    egui::ScrollArea::vertical()
        .id_source("categories")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (id, name) in categories {
                let r = ui.add_sized(
                    [ui.available_width(), if tv { 34.0 } else { 42.0 }],
                    egui::SelectableLabel::new(
                        selected == id,
                        RichText::new(name).size(if tv { 14.0 } else { 13.0 }),
                    ),
                );
                if hit(ui, &r, tv) {
                    next = Some(id);
                }
            }
        });
    if let Some(next) = next {
        if next != selected {
            if series {
                app.series_cat = next;
                app.selected_series = None;
            } else {
                app.vod_cat = next;
                app.selected_movie = None;
            }
        }
    }
}
fn title_list(app: &mut App, ui: &mut Ui, series: bool) {
    let tv = app.tv;
    ui.label(
        RichText::new(if series { "Serien" } else { "Filme" })
            .strong()
            .size(if tv { 18.0 } else { 20.0 }),
    );
    let q = if series { 1 } else { 0 };
    let mut show_password = false;
    components::field(
        ui,
        "",
        &mut app.catalog_queries[q],
        "Titel suchen",
        components::Kb::Text,
        &mut show_password,
    );
    ui.add_space(6.0);
    let category = if series {
        &app.series_cat
    } else {
        &app.vod_cat
    };
    let search = app.catalog_queries[q].to_lowercase();
    let prefix = if series { "series" } else { "vod" };
    let mut rows: Vec<_> = entries(app, series)
        .into_iter()
        .filter(|e| match category.as_str() {
            "all" | "recent" => true,
            "favorites" => app.is_favorite(&format!("{prefix}:{}", e.id)),
            cat => e.category == cat,
        })
        .filter(|e| search.is_empty() || e.name.to_lowercase().contains(&search))
        .collect();
    if category == "recent" {
        rows.sort_by_key(|e| std::cmp::Reverse(e.added));
    }
    let selected = if series {
        app.selected_series
    } else {
        app.selected_movie
    };
    let height = if tv { 48.0 } else { 62.0 };
    let mut choose = None;
    let mut play = None;
    egui::ScrollArea::vertical()
        .id_source(("titles", category, &search))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if rows.is_empty() {
                ui.label("Keine passenden Titel");
            }
            for entry in rows {
                let (rect, r) =
                    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
                if ui.is_rect_visible(rect) {
                    ui.painter().rect_filled(
                        rect.shrink(2.0),
                        10.0,
                        if selected == Some(entry.id) {
                            crate::theme::selection_surface(ui)
                        } else {
                            ui.visuals().faint_bg_color
                        },
                    );
                    let cover = Rect::from_center_size(
                        pos2(rect.left() + 22.0, rect.center().y),
                        vec2(28.0, height - 10.0),
                    );
                    components::thumb(
                        app,
                        ui,
                        &format!("library:{prefix}:{}", entry.id),
                        &entry.cover,
                        cover,
                        &entry.name,
                    );
                    let x = cover.right() + 8.0;
                    let w = (rect.right() - x - 8.0).max(1.0);
                    components::draw_line(
                        ui,
                        pos2(x, rect.center().y - 8.0),
                        &entry.name,
                        tokens::medium(if tv { 14.0 } else { 15.0 }),
                        ui.visuals().text_color(),
                        w,
                    );
                    components::draw_line(
                        ui,
                        pos2(x, rect.center().y + 11.0),
                        &entry.meta,
                        tokens::reg(11.0),
                        crate::theme::muted(ui),
                        w,
                    );
                }
                if hit(ui, &r, tv) {
                    if !series && selected == Some(entry.id) {
                        play = Some(entry.clone());
                    } else {
                        choose = Some(entry.id);
                    }
                }
            }
        });
    if let Some(id) = choose {
        if series {
            app.selected_series = Some(id);
            app.series_season = 0;
            app.request_series(id);
        } else {
            app.selected_movie = Some(id);
        }
    }
    if let Some(entry) = play {
        app.play(PlayItem {
            title: entry.name,
            url: entry.url,
            kind: PlayKind::Vod,
            progress_key: format!("vod:{}", entry.id),
        });
    }
}
fn details_panel(app: &mut App, ui: &mut Ui, series: bool) {
    let tv = app.tv;
    let selected = if series {
        app.selected_series
    } else {
        app.selected_movie
    };
    let item = selected.and_then(|id| entries(app, series).into_iter().find(|v| v.id == id));
    ui.label(
        RichText::new(if series {
            "Serie & Folgen"
        } else {
            "Filmdetails"
        })
        .strong()
        .size(if tv { 16.0 } else { 18.0 }),
    );
    ui.add_space(8.0);
    let Some(entry) = item else {
        ui.label(if series {
            "Serie auswählen"
        } else {
            "Film auswählen"
        });
        ui.label(
            RichText::new("Kategorien und Favoriten bleiben links sichtbar.")
                .small()
                .weak(),
        );
        return;
    };
    let key = format!("{}:{}", if series { "series" } else { "vod" }, entry.id);
    let mut play = false;
    egui::ScrollArea::vertical()
        .id_source(("details", series, entry.id))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let (cover, _) = ui.allocate_exact_size(vec2(68.0, 98.0), Sense::hover());
                components::thumb(
                    app,
                    ui,
                    &format!("library-detail:{key}"),
                    &entry.cover,
                    cover,
                    &entry.name,
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new(&entry.name).strong().size(if tv {
                        16.0
                    } else {
                        17.0
                    }));
                    ui.label(&entry.meta);
                    let r = ui.button(if app.is_favorite(&key) {
                        "★ Favorit"
                    } else {
                        "☆ Merken"
                    });
                    if hit(ui, &r, tv) {
                        app.toggle_favorite(&key);
                    }
                });
            });
            if !series {
                play = button(
                    ui,
                    if app.get_progress(&key) > 1 {
                        "▶ Fortsetzen"
                    } else {
                        "▶ Abspielen"
                    },
                    tv,
                );
            }
            if !entry.plot.is_empty() {
                ui.label(RichText::new(&entry.plot).small());
            }
            if series {
                series_details(app, ui, entry.id);
            }
        });
    if play {
        app.play(PlayItem {
            title: entry.name,
            url: entry.url,
            kind: PlayKind::Vod,
            progress_key: key,
        });
    }
}
fn series_details(app: &mut App, ui: &mut Ui, sid: u32) {
    let tv = app.tv;
    app.request_series(sid);
    let Some(mut episodes) = app.series_episodes_for(sid).cloned() else {
        if app.series_failed(sid) {
            ui.label("Folgen konnten nicht geladen werden.");
            if button(ui, "Erneut versuchen", tv) {
                app.retry_series(sid);
            }
        } else {
            ui.spinner();
            ui.label("Folgen werden geladen …");
        }
        return;
    };
    episodes.sort_by_key(|e| (e.season, e.episode, e.id));
    if let Some((next, pos)) = app.series_resume_target(sid) {
        let caption = format!(
            "▶ {} · Staffel {} · Folge {}",
            if pos > 1 { "Fortsetzen" } else { "Abspielen" },
            next.season,
            next.episode
        );
        if button(ui, caption, tv) {
            app.play_series_resume(sid);
            return;
        }
    }
    let mut seasons: Vec<_> = episodes.iter().map(|e| e.season).collect();
    seasons.sort_unstable();
    seasons.dedup();
    if !seasons.contains(&app.series_season) {
        app.series_season = seasons.first().copied().unwrap_or(1);
    }
    ui.horizontal_wrapped(|ui| {
        for season in &seasons {
            let r = ui.selectable_label(app.series_season == *season, format!("Staffel {season}"));
            if hit(ui, &r, tv) {
                app.series_season = *season;
            }
        }
    });
    ui.separator();
    let season = app.series_season;
    let mut play = None;
    ui.push_id(("season", sid, season), |ui| {
        for e in episodes.iter().filter(|e| e.season == season) {
            let label = format!("Folge {}", e.episode);
            if button(
                ui,
                RichText::new(label).size(if tv { 14.0 } else { 15.0 }),
                tv,
            ) {
                play = Some(e.clone());
            }
        }
    });
    if let Some(e) = play {
        app.play(PlayItem {
            title: format!("Staffel {} · Folge {}", e.season, e.episode),
            url: e.stream_url,
            kind: PlayKind::Episode,
            progress_key: format!("ep:{sid}:{}", e.id),
        });
    }
}
