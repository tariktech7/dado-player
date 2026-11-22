//! Shared virtual poster catalogue and fixed-height series details.
use crate::{
    app::App,
    types::{Episode, PlayItem, PlayKind},
    ui::mobile::{components, focus, tokens},
};
use egui::{pos2, vec2, Align, Layout, Rect, RichText, Sense, Ui};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
#[derive(Clone)]
struct Entry {
    id: u32,
    name: String,
    cover: String,
    plot: String,
    meta: String,
    url: String,
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
fn play_button(app: &mut App, ui: &mut Ui, label: impl Into<egui::WidgetText>, tv: bool) -> bool {
    let response = ui.button(label);
    if app.catalog_focus_play {
        response.request_focus();
        if tv {
            focus::request_id(response.id);
        }
        app.catalog_focus_play = false;
        ui.scroll_to_rect(response.rect, None);
        ui.ctx().request_repaint();
    }
    hit(ui, &response, tv)
}
fn entry_at(app: &App, series: bool, index: usize) -> Option<Entry> {
    if series {
        let v = app.library.series.get(index)?;
        Some(Entry {
            id: v.id,
            name: v.name.clone(),
            cover: v.cover.clone(),
            plot: v.plot.clone(),
            meta: "Serie".into(),
            url: String::new(),
        })
    } else {
        let v = app.library.movies.get(index)?;
        Some(Entry {
            id: v.id,
            name: v.name.clone(),
            cover: v.cover.clone(),
            plot: v.plot.clone(),
            meta: crate::ui::vod::rating_badge_for_home(&v.rating).unwrap_or_else(|| "Film".into()),
            url: v.stream_url.clone(),
        })
    }
}
pub fn show(app: &mut App, ui: &mut Ui, series: bool) {
    if !super::classic::enabled(app) {
        let category = if series {
            &mut app.series_cat
        } else {
            &mut app.vod_cat
        };
        if category == "continue" {
            *category = "all".into();
        }
    }
    ui.push_id(("library", series, app.catalog_generation()), |ui| {
        let a = ui.available_rect_before_wrap();
        let desktop = super::desktop::active(ui.ctx());
        let classic = super::classic::enabled(app);
        let scale = super::desktop::scale(ui.ctx());
        let landscape = super::mobile::handheld_landscape(ui.ctx());
        let narrow = a.width() < 620.0 && !landscape;
        let selected = if series {
            app.selected_series
        } else {
            app.selected_movie
        }
        .is_some();
        let animation = ui.ctx().animate_bool_with_time(
            ui.id().with("catalog-preview-slide"),
            selected,
            if super::desktop::reduced_motion(ui.ctx()) {
                0.0
            } else {
                0.24
            },
        );
        let gap = if classic {
            (super::desktop::appearance().classic_gap * scale).max(6.0)
        } else if desktop {
            24.0 * scale
        } else {
            10.0
        };
        let cw = if classic {
            (a.width() * super::desktop::appearance().classic_category_share)
                .max(102.0)
                .min(a.width() * 0.31)
        } else if desktop {
            super::desktop::category_width(a.width(), scale)
        } else if landscape {
            (a.width() * 0.25).min(212.0)
        } else if narrow {
            (a.width() * 0.31).clamp(96.0, 150.0)
        } else {
            (a.width() * 0.25).clamp(174.0, 244.0)
        };
        let dw = if !selected {
            0.0
        } else if desktop && super::desktop::handheld() {
            (a.width() * 0.42).min(360.0)
        } else if desktop {
            (super::desktop::appearance().catalog_preview_width * scale).min(a.width() * 0.45)
        } else if landscape {
            (a.width() * 0.39).min(340.0)
        } else if narrow {
            0.0
        } else {
            (a.width() * 0.32).clamp(220.0, 340.0)
        };
        let h = if narrow && selected {
            (a.height() * 0.48).max(120.0)
        } else {
            a.height()
        };
        let cr = Rect::from_min_size(a.min, vec2(cw, h));
        let lr = Rect::from_min_max(
            pos2(cr.right() + gap, a.top()),
            pos2(
                a.right() - dw - if narrow || !selected { 0.0 } else { gap },
                a.top() + h,
            ),
        );
        let dr = if narrow {
            Rect::from_min_max(pos2(a.left(), lr.bottom() + gap), a.max)
        } else {
            Rect::from_min_max(pos2(lr.right() + gap, a.top()), a.max)
        };
        if desktop {
            let offset = super::desktop::offset(ui.ctx());
            ui.painter().rect_filled(
                lr.expand2(vec2(6.0, 16.0)),
                0.0,
                egui::Color32::from_black_alpha(24),
            );
            focus::set_zone(1);
            super::desktop::column(ui, cr, "catalog-categories", offset, |ui| {
                if classic {
                    let language = app.settings.language;
                    super::desktop::search_field(
                        ui,
                        &mut app.catalog_queries[usize::from(series)],
                        language.text("Titel suchen", "Search titles"),
                        app.tv,
                    );
                    let id = egui::Id::new(("classic-catalog-sort", series));
                    let alphabetic = ui
                        .ctx()
                        .data(|data| data.get_temp::<bool>(id).unwrap_or(false));
                    let caption = if alphabetic {
                        language.text("Sortierung: A–Z", "Sort: A–Z")
                    } else {
                        language.text("Sortierung: Anbieter", "Sort: Provider")
                    };
                    if button(ui, caption, app.tv) {
                        ui.ctx().data_mut(|data| data.insert_temp(id, !alphabetic));
                    }
                }
                categories(app, ui, series)
            });
            focus::set_zone(2);
            super::desktop::column(ui, lr, "catalog-posters", offset, |ui| {
                posters(app, ui, series, false)
            });
            if selected {
                let clip = ui.clip_rect();
                ui.set_clip_rect(clip.intersect(dr));
                let animated = dr.translate(vec2((1.0 - animation) * dr.width(), 0.0));
                focus::set_zone(3);
                super::desktop::column(ui, animated, "catalog-details", 0.0, |ui| {
                    super::desktop::catalog_heading(
                        ui,
                        if series {
                            app.settings
                                .language
                                .text("Serie & Folgen", "Series & episodes")
                        } else {
                            app.settings.language.text("Details", "Details")
                        },
                    );
                    details(app, ui, series, false)
                });
                ui.set_clip_rect(clip);
            }
            return;
        }
        let mut c = ui.child_ui(cr, Layout::top_down(Align::Min));
        c.set_clip_rect(cr.intersect(ui.clip_rect()));
        focus::set_zone(1);
        categories(app, &mut c, series);
        let mut l = ui.child_ui(lr, Layout::top_down(Align::Min));
        l.set_clip_rect(lr.intersect(ui.clip_rect()));
        focus::set_zone(2);
        posters(app, &mut l, series, false);
        if selected {
            let animated = dr.translate(vec2((1.0 - animation) * dr.width(), 0.0));
            let mut d = ui.child_ui(animated, Layout::top_down(Align::Min));
            d.set_clip_rect(dr.intersect(ui.clip_rect()));
            focus::set_zone(3);
            details(app, &mut d, series, false);
        }
    });
}
/// A phone catalogue is a full-page grid followed by a separate detail page.
/// It shares data/filtering with desktop, never the desktop column geometry.
pub fn show_touch(app: &mut App, ui: &mut Ui, series: bool, layout: &super::mobile::Layout) {
    // Frames do not clip their children by default. Keep virtual poster rows
    // inside the page so an overscan row can never paint over the bottom bar.
    ui.set_clip_rect(ui.clip_rect().intersect(ui.available_rect_before_wrap()));
    ui.push_id(("touch-library", series, app.catalog_generation()), |ui| {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(layout.pad, 12.0))
            .show(ui, |ui| {
                let selected = if series {
                    app.selected_series
                } else {
                    app.selected_movie
                };
                if let Some(id) = selected {
                    egui::ScrollArea::vertical()
                        .id_source(("detail-page", id))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            details(app, ui, series, true);
                            ui.add_space(layout.bottom_inset + 24.0);
                        });
                } else {
                    let mut cats = vec![
                        ("favorites".to_owned(), "Favoriten".to_owned()),
                        (
                            "all".into(),
                            if series { "Alle Serien" } else { "Alle Filme" }.into(),
                        ),
                        ("recent".into(), "Neu hinzugefügt".into()),
                    ];
                    cats.extend(
                        if series {
                            &app.library.series_categories
                        } else {
                            &app.library.vod_categories
                        }
                        .iter()
                        .map(|c| (c.id.clone(), c.name.clone())),
                    );
                    let cat = if series {
                        &mut app.series_cat
                    } else {
                        &mut app.vod_cat
                    };
                    let caption = cats
                        .iter()
                        .find(|(id, _)| id == cat)
                        .map(|(_, name)| name.as_str())
                        .unwrap_or("Kategorie");
                    egui::ComboBox::from_id_source("category-dropdown")
                        .selected_text(caption)
                        .width(ui.available_width().min(360.0))
                        .show_ui(ui, |ui| {
                            for (id, name) in cats {
                                ui.selectable_value(cat, id, name);
                            }
                        });
                    if touch_search_open(ui.ctx(), series) {
                        let mut reveal = false;
                        components::field(
                            ui,
                            "",
                            &mut app.catalog_queries[usize::from(series)],
                            "Titel suchen",
                            components::Kb::Text,
                            &mut reveal,
                        );
                    }
                    ui.add_space(10.0);
                    posters(app, ui, series, true);
                }
            });
    });
}
fn touch_search_open(ctx: &egui::Context, series: bool) -> bool {
    ctx.data(|d| {
        d.get_temp(egui::Id::new(("catalog-search", series)))
            .unwrap_or(false)
    })
}
pub fn toggle_touch_search(app: &mut App, ctx: &egui::Context, series: bool) {
    let open = !touch_search_open(ctx, series);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(("catalog-search", series)), open));
    if !open {
        app.catalog_queries[usize::from(series)].clear();
    }
}
fn categories(app: &mut App, ui: &mut Ui, series: bool) {
    let tv = app.tv;
    let desktop = super::desktop::active(ui.ctx());
    let scale = super::desktop::scale(ui.ctx());
    let classic_counts = super::classic::enabled(app).then(|| {
        let generation = app.catalog_generation();
        if series {
            super::classic::category_counts(
                ui,
                generation,
                app.library
                    .series
                    .iter()
                    .map(|item| item.category_id.as_str()),
            )
        } else {
            super::classic::category_counts(
                ui,
                generation,
                app.library
                    .movies
                    .iter()
                    .map(|item| item.category_id.as_str()),
            )
        }
    });
    if desktop {
        super::desktop::catalog_heading(ui, app.settings.language.text("Kategorien", "Categories"));
    } else {
        ui.label(RichText::new("Kategorien").strong().size(15.0));
    }
    ui.add_space(6.0);
    let cats = if series {
        &app.library.series_categories
    } else {
        &app.library.vod_categories
    };
    let language = app.settings.language;
    let mut special = vec![
        ("favorites", language.text("Favoriten", "Favorites")),
        ("all", language.text("Alle", "All titles")),
        ("recent", language.text("Neu hinzugefügt", "Recently added")),
    ];
    if super::classic::enabled(app) {
        special.push(("continue", language.text("Fortsetzen", "Continue watching")));
    }
    let count = cats.len() + special.len();
    let selected = if series {
        app.series_cat.clone()
    } else {
        app.vod_cat.clone()
    };
    let mut next = None;
    egui::ScrollArea::vertical()
        .id_source("categories")
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let row_height = if desktop {
                super::desktop::handheld_size(58.0 * scale, 46.0)
            } else if tv {
                39.0
            } else {
                46.0
            };
            let stride = row_height + ui.spacing().item_spacing.y;
            ui.set_height((stride * count as f32 - ui.spacing().item_spacing.y).max(0.0));
            // Keep neighbours mounted on BOTH sides for remote navigation.
            // Offscreen provider categories must not create text/paint/focus
            // work on every animation frame of the independent poster pane.
            let start = (viewport.min.y / stride).floor().max(0.0) as usize;
            let start = start.saturating_sub(2).min(count);
            let end = ((viewport.max.y / stride).ceil() as usize + 2).min(count);
            let rect = Rect::from_min_max(
                pos2(
                    ui.max_rect().left(),
                    ui.max_rect().top() + start as f32 * stride,
                ),
                pos2(
                    ui.max_rect().right(),
                    ui.max_rect().top() + end as f32 * stride,
                ),
            );
            ui.allocate_ui_at_rect(rect, |ui| {
                for index in start..end {
                    let (id, name) = special.get(index).copied().unwrap_or_else(|| {
                        let category = &cats[index - special.len()];
                        (category.id.as_str(), category.name.as_str())
                    });
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), row_height),
                        Sense::hover(),
                    );
                    let r = ui.interact(rect, ui.id().with((&id, "category")), Sense::click());
                    if classic_counts.is_some() {
                        super::classic::list_surface(
                            ui,
                            rect,
                            selected == id,
                            r.has_focus() || r.hovered(),
                        );
                    } else if selected == id {
                        ui.painter().rect_filled(
                            rect.shrink(1.0),
                            9.0,
                            crate::theme::selection_surface(ui),
                        );
                    }
                    let total_count = if series {
                        app.library.series.len()
                    } else {
                        app.library.movies.len()
                    };
                    let count = classic_counts.as_ref().and_then(|counts| match id {
                        "all" => Some(total_count),
                        "favorites" => Some(
                            app.favorites
                                .iter()
                                .filter(|key| {
                                    key.starts_with(if series { "series:" } else { "vod:" })
                                })
                                .count(),
                        ),
                        "recent" | "continue" => None,
                        _ => Some(counts.get(id).copied().unwrap_or(0)),
                    });
                    let text_space = if count.is_some() {
                        (52.0 * scale).max(34.0)
                    } else {
                        0.0
                    };
                    let mut job = egui::text::LayoutJob::simple(
                        name.to_owned(),
                        tokens::reg(if desktop {
                            let font = if classic_counts.is_some() {
                                super::desktop::appearance().classic_category_font
                            } else {
                                21.0
                            };
                            super::desktop::handheld_type((font * scale).max(11.0), 17.0)
                        } else if tv {
                            12.0
                        } else {
                            13.0
                        }),
                        ui.visuals().text_color(),
                        (rect.width() - 16.0 - text_space).max(1.0),
                    );
                    job.wrap.max_rows = 2;
                    let galley = ui.fonts(|f| f.layout_job(job));
                    ui.painter().galley(
                        pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
                        galley,
                        ui.visuals().text_color(),
                    );
                    if let Some(count) = count {
                        ui.painter().text(
                            rect.right_center() - vec2(7.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            count.to_string(),
                            tokens::reg((18.0 * scale).max(12.0)),
                            crate::theme::muted(ui),
                        );
                    }
                    if hit(ui, &r, tv) {
                        next = Some(id.to_owned());
                    }
                    r.on_hover_text(name);
                }
            });
        });
    if let Some(id) = next {
        if selected != id {
            if series {
                app.series_cat = id;
                app.selected_series = None;
            } else {
                app.vod_cat = id;
                app.selected_movie = None;
            }
            app.nav_collapsed = true;
        }
    }
}
type FilterSignature = (u64, usize, String, String, u64, bool);
type FilterCache = (FilterSignature, Arc<Vec<usize>>);
fn indices(app: &App, ui: &Ui, series: bool) -> Arc<Vec<usize>> {
    let cat = if series {
        &app.series_cat
    } else {
        &app.vod_cat
    };
    let query = app.catalog_queries[usize::from(series)].to_lowercase();
    let continued: std::collections::HashSet<u32> = if cat == "continue" {
        crate::types::resume_history(&app.history)
            .filter_map(|record| {
                let prefix = if series { "ep:" } else { "vod:" };
                record
                    .key
                    .strip_prefix(prefix)?
                    .split(':')
                    .next()?
                    .parse()
                    .ok()
            })
            .collect()
    } else {
        Default::default()
    };
    let source_hash = app.favorites.iter().fold(0, |sum, k| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        k.hash(&mut h);
        sum ^ h.finish()
    }) ^ continued.iter().fold(0, |sum, id| {
        sum ^ u64::from(*id)
            .wrapping_mul(0x9E3779B97F4A7C15)
            .rotate_left(*id % 64)
    });
    let alphabetic = super::classic::enabled(app)
        && ui.ctx().data(|data| {
            data.get_temp::<bool>(egui::Id::new(("classic-catalog-sort", series)))
                .unwrap_or(false)
        });
    let sig = (
        app.catalog_generation(),
        if series {
            app.library.series.len()
        } else {
            app.library.movies.len()
        },
        cat.clone(),
        query.clone(),
        source_hash,
        alphabetic,
    );
    let key = ui.id().with(("filtered", series));
    if let Some((old, rows)) = ui.data(|d| d.get_temp::<FilterCache>(key)) {
        if old == sig {
            return rows;
        }
    }
    let keep = |id: u32, category: &str, name: &str| {
        (match cat.as_str() {
            "all" | "recent" => true,
            "continue" => continued.contains(&id),
            "favorites" => {
                app.is_favorite(&format!("{}:{id}", if series { "series" } else { "vod" }))
            }
            v => v == category,
        }) && (query.is_empty() || name.to_lowercase().contains(&query))
    };
    let mut rows: Vec<_> = if series {
        app.library
            .series
            .iter()
            .enumerate()
            .filter(|(_, v)| keep(v.id, &v.category_id, &v.name))
            .map(|(i, _)| i)
            .collect()
    } else {
        app.library
            .movies
            .iter()
            .enumerate()
            .filter(|(_, v)| keep(v.id, &v.category_id, &v.name))
            .map(|(i, _)| i)
            .collect()
    };
    if alphabetic {
        rows.sort_by_cached_key(|index| {
            if series {
                app.library.series[*index].name.to_lowercase()
            } else {
                app.library.movies[*index].name.to_lowercase()
            }
        });
    } else if cat == "recent" {
        rows.sort_by_key(|i| {
            std::cmp::Reverse(if series {
                app.library.series[*i].added
            } else {
                app.library.movies[*i].added
            })
        });
    }
    let rows = Arc::new(rows);
    ui.data_mut(|d| d.insert_temp(key, (sig, rows.clone())));
    rows
}
fn posters(app: &mut App, ui: &mut Ui, series: bool, touch: bool) {
    let desktop = super::desktop::active(ui.ctx());
    let scale = super::desktop::scale(ui.ctx());
    let tv = app.tv;
    if !touch {
        if super::desktop::active(ui.ctx()) {
            super::desktop::catalog_heading(
                ui,
                if series {
                    app.settings.language.text("Serien", "Series")
                } else {
                    app.settings.language.text("Filme", "Movies")
                },
            );
        } else {
            ui.label(
                RichText::new(if series {
                    app.settings.language.text("Serien", "Series")
                } else {
                    app.settings.language.text("Filme", "Movies")
                })
                .strong()
                .size(18.0),
            );
        }
        if desktop && !super::classic::enabled(app) {
            super::desktop::search_field(
                ui,
                &mut app.catalog_queries[usize::from(series)],
                app.settings.language.text("Titel suchen", "Search titles"),
                tv,
            );
        } else if !desktop {
            let mut reveal = false;
            components::field(
                ui,
                "",
                &mut app.catalog_queries[usize::from(series)],
                "Titel suchen",
                components::Kb::Text,
                &mut reveal,
            );
        }
        ui.add_space(6.0);
    }
    let rows = indices(app, ui, series);
    let cat = if series {
        app.series_cat.clone()
    } else {
        app.vod_cat.clone()
    };
    let gap = 8.0;
    let classic = super::classic::enabled(app);
    let a = super::desktop::appearance();
    let min_width = if classic {
        (a.classic_poster_width * scale).max(136.0)
    } else if desktop {
        (super::desktop::appearance().catalog_poster_width * scale).max(128.0)
    } else {
        128.0
    };
    let min_cols = if touch || super::desktop::handheld() {
        1
    } else {
        2
    };
    let cols = (((ui.available_width() + gap) / (min_width + gap)).floor() as usize)
        .clamp(min_cols, if touch { 5 } else { usize::MAX });
    let width = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).max(24.0);
    let art_h = (width * 1.4).min(if classic {
        (a.classic_poster_row_height * scale - 38.0).max(144.0)
    } else if tv {
        154.0
    } else {
        190.0
    });
    let row_h = art_h + 38.0;
    let selected = if series {
        app.selected_series
    } else {
        app.selected_movie
    };
    let poster_scope = ui.id().with("poster-control");
    let returning_movie = if series {
        None
    } else {
        app.desktop_return_item
            .as_deref()
            .and_then(|key| key.strip_prefix("vod:"))
            .and_then(|id| id.parse().ok())
    };
    let mut choose = None;
    let mut scroll = egui::ScrollArea::vertical()
        .id_source(("posters", &cat, &app.catalog_queries[usize::from(series)]))
        .auto_shrink([false, false]);
    if let Some(index) = returning_movie.and_then(|id| {
        rows.iter()
            .position(|&index| app.library.movies[index].id == id)
    }) {
        let stride = row_h + ui.spacing().item_spacing.y;
        let y = (index / cols) as f32 * stride;
        scroll = scroll.vertical_scroll_offset((y - ui.available_height() * 0.5).max(0.0));
    }
    scroll.show_rows(ui, row_h, rows.len().div_ceil(cols), |ui, range| {
        ui.spacing_mut().item_spacing.x = gap;
        for row in range {
            ui.horizontal_top(|ui| {
                for column in 0..cols {
                    let Some(index) = rows.get(row * cols + column) else {
                        break;
                    };
                    let Some(entry) = entry_at(app, series, *index) else {
                        continue;
                    };
                    ui.push_id(("poster", entry.id), |ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(width, row_h), Sense::hover());
                        let response =
                            ui.interact(rect, poster_scope.with(entry.id), Sense::click());
                        if returning_movie == Some(entry.id) {
                            response.request_focus();
                            if tv {
                                focus::request_id(response.id);
                            }
                            app.desktop_return_item = None;
                        }
                        let kind = if series { "series" } else { "vod" };
                        let key = format!("{kind}:{}", entry.id);
                        super::desktop::remember_favorite_target(&response, &key);
                        if ui.is_rect_visible(rect) {
                            if classic {
                                super::classic::list_surface(
                                    ui,
                                    rect,
                                    selected == Some(entry.id),
                                    response.has_focus() || response.hovered(),
                                );
                            } else {
                                ui.painter().rect_filled(
                                    rect.shrink(1.0),
                                    10.0,
                                    if selected == Some(entry.id) {
                                        crate::theme::selection_surface(ui)
                                    } else {
                                        ui.visuals().faint_bg_color
                                    },
                                );
                                if response.has_focus() {
                                    ui.painter().rect_stroke(
                                        rect.shrink(1.0),
                                        10.0,
                                        egui::Stroke::new(2.0_f32, crate::theme::accent(ui)),
                                    );
                                }
                            }
                            let art = Rect::from_min_size(
                                rect.min + vec2(4.0, 4.0),
                                vec2((width - 8.0).max(1.0), art_h - 4.0),
                            );
                            components::thumb(
                                app,
                                ui,
                                &format!("poster:{}:{}", series, entry.id),
                                &entry.cover,
                                art,
                                &entry.name,
                            );
                            if desktop {
                                let radius = (12.0 * scale).max(6.0);
                                let center = art.right_top() + vec2(-radius - 3.0, radius + 3.0);
                                ui.painter().circle_filled(
                                    center,
                                    radius + 3.0,
                                    ui.visuals().panel_fill,
                                );
                                super::favorite_star(ui, center, radius, app.is_favorite(&key));
                            }
                            let mut job = egui::text::LayoutJob::simple(
                                entry.name.clone(),
                                tokens::medium(super::desktop::handheld_type(
                                    if classic {
                                        (22.0 * scale).max(14.0)
                                    } else if tv {
                                        11.0
                                    } else {
                                        13.0
                                    },
                                    14.0,
                                )),
                                ui.visuals().text_color(),
                                (width - 8.0).max(1.0),
                            );
                            job.wrap.max_rows = 2;
                            let galley = ui.fonts(|f| f.layout_job(job));
                            ui.painter().galley(
                                pos2(rect.left() + 4.0, rect.top() + art_h + 4.0),
                                galley,
                                ui.visuals().text_color(),
                            );
                        }
                        if hit(ui, &response, tv) {
                            choose = Some(entry.id);
                        }
                    });
                }
            });
        }
    });
    if rows.is_empty() {
        ui.label(
            app.settings
                .language
                .text("Keine passenden Titel.", "No matching titles."),
        );
    }
    if let Some(id) = choose {
        app.catalog_focus_play = !touch;
        app.nav_collapsed = true;
        if series {
            app.open_series(id);
        } else {
            app.selected_movie = Some(id);
        }
    }
}
fn details(app: &mut App, ui: &mut Ui, series: bool, touch: bool) {
    let tv = app.tv;
    let selected = if series {
        app.selected_series
    } else {
        app.selected_movie
    };
    let index = selected.and_then(|id| {
        if series {
            app.library.series.iter().position(|v| v.id == id)
        } else {
            app.library.movies.iter().position(|v| v.id == id)
        }
    });
    let entry = index.and_then(|i| entry_at(app, series, i));
    let Some(entry) = entry else {
        ui.heading(if series {
            "Serie auswählen"
        } else {
            "Film auswählen"
        });
        ui.label("Poster auswählen für Details und Wiedergabe.");
        return;
    };
    let key = format!("{}:{}", if series { "series" } else { "vod" }, entry.id);
    ui.horizontal_top(|ui| {
        let (art, _) = ui.allocate_exact_size(
            if touch {
                vec2(96.0, 144.0)
            } else {
                vec2(36.0, 50.0)
            },
            Sense::hover(),
        );
        components::thumb(
            app,
            ui,
            &format!("detail:{key}"),
            &entry.cover,
            art,
            &entry.name,
        );
        ui.vertical(|ui| {
            let mut job = egui::text::LayoutJob::simple(
                entry.name.clone(),
                tokens::medium(if tv { 15.0 } else { 17.0 }),
                ui.visuals().text_color(),
                ui.available_width(),
            );
            job.wrap.max_rows = 2;
            ui.label(job);
            let response = ui
                .horizontal(|ui| {
                    let response = crate::ui::fav_button(app, ui, &key);
                    ui.label(if app.is_favorite(&key) {
                        app.settings.language.text("Favorit", "Favorite")
                    } else {
                        app.settings.language.text("Merken", "Save")
                    });
                    response
                })
                .inner;
            if series && app.catalog_focus_play && app.series_episodes_for(entry.id).is_none() {
                let key = ui
                    .id()
                    .with(("loading-series-focus", app.series_open_revision));
                if !ui.data(|data| data.get_temp::<bool>(key).unwrap_or(false)) {
                    ui.data_mut(|data| data.insert_temp(key, true));
                    response.request_focus();
                    if tv {
                        focus::request_id(response.id);
                    }
                }
            }
        });
    });
    if !entry.plot.is_empty() {
        let mut job = egui::text::LayoutJob::simple(
            entry.plot.clone(),
            tokens::reg(if tv { 11.0 } else { 12.0 }),
            crate::theme::muted(ui),
            ui.available_width(),
        );
        let expanded_key = ui.id().with(("plot-expanded", entry.id));
        let expanded = touch && ui.data(|d| d.get_temp::<bool>(expanded_key).unwrap_or(false));
        if !expanded {
            job.wrap.max_rows = if touch { 3 } else { 2 };
        }
        ui.label(job);
        if button(
            ui,
            if expanded {
                "Weniger anzeigen"
            } else {
                "Mehr lesen"
            },
            tv,
        ) {
            if touch {
                ui.data_mut(|d| d.insert_temp(expanded_key, !expanded));
            } else {
                app.description_open = Some((entry.name.clone(), entry.plot.clone()));
            }
        }
    }
    if series {
        series_details(app, ui, entry.id, touch);
    } else {
        ui.label(&entry.meta);
        let caption = if app.get_progress(&key) > 1 {
            "▶ Fortsetzen"
        } else {
            "▶ Abspielen"
        };
        if play_button(app, ui, caption, tv) {
            app.play(PlayItem {
                title: entry.name,
                url: entry.url,
                kind: PlayKind::Vod,
                progress_key: key,
            });
        }
    }
}
fn series_details(app: &mut App, ui: &mut Ui, sid: u32, touch: bool) {
    let tv = app.tv;
    app.request_series(sid);
    let Some(source) = app.series_episodes_for(sid) else {
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
    type Cached = ((usize, usize), Arc<Vec<Episode>>);
    let signature = (source.as_ptr() as usize, source.len());
    let cache = ui.id().with(("episode-cache", sid));
    let episodes = ui
        .data(|d| d.get_temp::<Cached>(cache))
        .filter(|v| v.0 == signature)
        .map(|v| v.1)
        .unwrap_or_else(|| {
            let mut rows = source.clone();
            rows.sort_by_key(|e| (e.season, e.episode, e.id));
            let rows = Arc::new(rows);
            ui.data_mut(|d| d.insert_temp(cache, (signature, rows.clone())));
            rows
        });
    let target = app.series_resume_target(sid);
    if let Some((next, pos)) = &target {
        let caption = format!(
            "▶ {} · S{} F{}",
            if *pos > 1 { "Fortsetzen" } else { "Abspielen" },
            next.season,
            next.episode
        );
        if play_button(app, ui, caption, tv) {
            app.play_series_resume(sid);
            return;
        }
    }
    let mut seasons: Vec<_> = episodes.iter().map(|e| e.season).collect();
    seasons.dedup();
    if !seasons.contains(&app.series_season) {
        app.series_season = target
            .as_ref()
            .map(|v| v.0.season)
            .or_else(|| seasons.first().copied())
            .unwrap_or(1);
    }
    let position_key = ui.id().with(("episode-position", sid));
    let old = ui.data(|d| d.get_temp::<(u64, u32, Option<u32>)>(position_key));
    let position = old
        != Some((
            app.series_open_revision,
            app.series_season,
            target.as_ref().map(|v| v.0.id),
        ));
    let season_before = app.series_season;
    egui::ScrollArea::horizontal()
        .id_source(("season-tabs", sid))
        .max_height(if touch { 56.0 } else { 34.0 })
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for s in seasons {
                    let r = ui.selectable_label(app.series_season == s, format!("Staffel {s}"));
                    if position && app.series_season == s {
                        ui.scroll_to_rect(r.rect, Some(Align::Min));
                    }
                    if hit(ui, &r, tv) {
                        app.series_season = s;
                    }
                }
            });
        });
    let returning_episode = app.desktop_return_item.as_deref().and_then(|key| {
        let (series, episode) = key.strip_prefix("ep:")?.split_once(':')?;
        (series.parse::<u32>().ok()? == sid)
            .then(|| episode.parse::<u32>().ok())
            .flatten()
    });
    if let Some(episode) = episodes.iter().find(|e| Some(e.id) == returning_episode) {
        app.series_season = episode.season;
    }
    let season = app.series_season;
    let rows: Vec<_> = episodes.iter().filter(|e| e.season == season).collect();
    let mut play = None;
    let target_id = target
        .as_ref()
        .filter(|t| t.0.season == season)
        .map(|v| v.0.id);
    let has_history = app
        .history
        .iter()
        .any(|h| h.key.starts_with(&format!("ep:{sid}:")));
    let scroll_to = if touch && (!has_history || season != season_before) {
        None
    } else if position || season != season_before {
        target_id.or_else(|| rows.first().map(|e| e.id))
    } else {
        None
    };
    let episode_list = |ui: &mut Ui| {
        if touch {
            ui.add_space(12.0);
            ui.label(RichText::new("Folgen").strong());
        }
        for e in rows {
            let r = ui.add_sized(
                [
                    ui.available_width(),
                    if touch {
                        56.0
                    } else if tv {
                        29.0
                    } else {
                        38.0
                    },
                ],
                egui::SelectableLabel::new(target_id == Some(e.id), format!("Folge {}", e.episode)),
            );
            if returning_episode == Some(e.id) {
                r.request_focus();
                app.desktop_return_item = None;
                ui.scroll_to_rect(r.rect, Some(Align::Center));
            } else if scroll_to == Some(e.id) {
                ui.scroll_to_rect(r.rect, Some(Align::Min));
            }
            #[cfg(windows)]
            if r.has_focus() {
                super::desktop::remember_favorite_target(&r, &format!("ep:{sid}:{}", e.id));
            }
            if hit(ui, &r, tv) {
                play = Some(e.clone());
            }
        }
    };
    if touch {
        episode_list(ui);
    } else {
        egui::ScrollArea::vertical()
            .id_source(("episodes", sid, season))
            .auto_shrink([false, false])
            .show(ui, episode_list);
    }
    ui.data_mut(|d| {
        d.insert_temp(
            position_key,
            (
                app.series_open_revision,
                season,
                target.as_ref().map(|v| v.0.id),
            ),
        )
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
pub fn description_dialog(app: &mut App, ctx: &egui::Context) {
    let Some((title, text)) = app.description_open.clone() else {
        return;
    };
    let mut open = true;
    let mut close = false;
    egui::Window::new(title)
        .id(egui::Id::new("description-dialog"))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .default_width((ctx.screen_rect().width() * 0.65).clamp(260.0, 720.0))
        .show(ctx, |ui| {
            if app.tv {
                focus::begin_modal(0);
            }
            egui::ScrollArea::vertical()
                .max_height(ctx.screen_rect().height() * 0.6)
                .show(ui, |ui| {
                    ui.label(text);
                });
            close = button(ui, "Schließen", app.tv);
        });
    if !open || close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.description_open = None;
    }
}
