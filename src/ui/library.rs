//! Shared virtual poster catalogue and fixed-height series details.
use crate::{
    app::App,
    types::{Episode, PlayItem, PlayKind},
    ui::mobile::{components, focus, tokens},
};
use egui::{pos2, vec2, Align, Color32, Layout, Rect, RichText, Sense, Ui};
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
pub(super) fn show_detail_page(app: &mut App, ui: &mut Ui, series: bool, id: u32) {
    let index = if series {
        app.library.series.iter().position(|entry| entry.id == id)
    } else {
        app.library.movies.iter().position(|entry| entry.id == id)
    };
    if let Some(entry) = index.and_then(|index| entry_at(app, series, index)) {
        show_title_page(app, ui, entry, series);
    } else if button(
        ui,
        app.settings.language.text("‹ Zurück", "‹ Back"),
        app.shared_remote_navigation(),
    ) {
        app.close_catalog_detail();
    }
}

fn show_title_page(app: &mut App, ui: &mut Ui, entry: Entry, series: bool) {
    let bounds = ui.available_rect_before_wrap();
    if !bounds.is_positive() {
        return;
    }
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let key = format!("{}:{}", if series { "series" } else { "vod" }, entry.id);
    let image_key = format!("detail:{key}");
    let base = Color32::from_rgb(12, 27, 44);
    ui.painter().rect_filled(bounds, 12.0, base);
    if !entry.cover.is_empty() {
        app.request_image(image_key.clone(), entry.cover.clone());
        if let Some(texture) = app.image(&image_key) {
            let art = bounds;
            let size = texture.size_vec2();
            let ratio = (art.width() / art.height()) / (size.x / size.y);
            let visible = vec2(ratio.min(1.0), ratio.recip().min(1.0));
            let uv = Rect::from_center_size(
                pos2(0.5, 0.38_f32.clamp(visible.y * 0.5, 1.0 - visible.y * 0.5)),
                visible,
            );
            ui.painter().image(texture.id(), art, uv, Color32::WHITE);
        }
    }
    let shade = |rect: Rect, colours: [Color32; 4]| {
        let mut mesh = egui::Mesh::default();
        for (point, colour) in [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ]
        .into_iter()
        .zip(colours)
        {
            mesh.colored_vertex(point, colour);
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(mesh));
    };
    let touch = super::desktop::handheld();
    let shade_split = bounds.left() + bounds.width() * if touch { 0.69 } else { 0.58 };
    let fade_end = bounds.left() + bounds.width() * 0.92;
    let shade_start = Color32::from_black_alpha(228);
    let shade_mid = Color32::from_black_alpha(158);
    shade(
        Rect::from_min_max(bounds.min, pos2(shade_split, bounds.bottom())),
        [shade_start, shade_mid, shade_mid, shade_start],
    );
    shade(
        Rect::from_min_max(
            pos2(shade_split, bounds.top()),
            pos2(fade_end, bounds.bottom()),
        ),
        [
            shade_mid,
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
            shade_mid,
        ],
    );
    shade(
        Rect::from_min_max(
            pos2(bounds.left(), bounds.top() + bounds.height() * 0.35),
            bounds.max,
        ),
        [
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
            Color32::BLACK,
            Color32::BLACK,
        ],
    );
    let scale = super::desktop::scale(ui.ctx());
    let inset = super::desktop::handheld_size(24.0 * scale, 14.0);
    let content = bounds.shrink(inset);
    let mut top = ui.child_ui_with_id_source(content, Layout::top_down(Align::Min), "title-back");
    focus::set_zone(1);
    let back = super::desktop::back_button(&mut top, t("‹ Zurück", "‹ Back"));
    if hit(&top, &back, app.shared_remote_navigation()) {
        app.close_catalog_detail();
        ui.ctx().request_repaint();
        return;
    }
    let rail_height = (bounds.height() * 0.065).clamp(24.0, 54.0);
    let gap = 8.0;
    let season_label_height = 20.0;
    let episode_height = (bounds.height() * 0.21).clamp(48.0, 190.0).min(
        (content.bottom()
            - back.rect.bottom()
            - rail_height
            - season_label_height
            - 5.0 * gap
            - 85.0)
            .max(28.0),
    );
    let controls_bottom = if series {
        content.bottom() - rail_height - episode_height - season_label_height - 3.0 * gap
    } else {
        content.bottom()
    };
    let font_size = (bounds.height() * 0.069).clamp(if touch { 18.0 } else { 24.0 }, 54.0);
    let mut play_size = vec2(
        super::desktop::handheld_size((280.0 * scale).max(144.0), 174.0),
        super::desktop::handheld_size((96.0 * scale).max(52.0), 62.0),
    );
    if series {
        play_size.x = play_size.x.min(content.width() * 0.31);
        play_size.y = super::desktop::handheld_size((64.0 * scale).max(42.0), 44.0);
    }
    let controls = Rect::from_min_max(
        pos2(
            content.left(),
            (controls_bottom - play_size.y).max(content.top()),
        ),
        pos2(content.right(), controls_bottom),
    );
    let copy_bounds = Rect::from_min_max(
        pos2(content.left(), back.rect.bottom() + gap),
        pos2(
            content.left() + content.width() * if touch { 0.72 } else { 0.62 },
            (controls.top() - gap).max(back.rect.bottom() + gap),
        ),
    );
    if copy_bounds.is_positive() {
        let mut copy =
            ui.child_ui_with_id_source(copy_bounds, Layout::top_down(Align::Min), "title-copy");
        copy.set_clip_rect(copy_bounds.intersect(ui.clip_rect()));
        let width = copy.available_width();
        let title = components::wrap(
            &copy,
            &entry.name,
            tokens::bold(font_size),
            Color32::WHITE,
            width,
            2,
        );
        copy.label(title);
        if !entry.plot.is_empty() {
            let text_size = (bounds.height() * 0.033).clamp(13.0, 27.0);
            let lines = (copy.available_height() / (text_size * 1.28))
                .floor()
                .max(0.0) as usize;
            if lines > 0 {
                let plot = components::wrap(
                    &copy,
                    &entry.plot,
                    tokens::reg(text_size),
                    Color32::WHITE,
                    width,
                    lines.min(if series { 4 } else { 10 }),
                );
                copy.label(plot);
            }
        }
    }
    let mut bottom = ui.child_ui_with_id_source(
        controls,
        Layout::left_to_right(Align::Center),
        "title-actions",
    );
    focus::set_zone(1);
    let target = if series {
        app.request_series(entry.id);
        app.series_resume_target(entry.id)
    } else {
        None
    };
    let caption = t("▶ Abspielen", "▶ Play");
    let tv = app.shared_remote_navigation();
    let mut random = false;
    let play = bottom
        .horizontal(|ui| {
            let play_text = RichText::new(caption)
                .size(if series {
                    super::desktop::handheld_type((27.0 * scale).max(16.0), 16.0)
                } else {
                    super::desktop::handheld_type((38.0 * scale).max(20.0), 23.0)
                })
                .strong()
                .color(Color32::WHITE);
            let primary = egui::Button::new(play_text)
                .min_size(play_size)
                .rounding(egui::Rounding::same(12.0))
                .fill(crate::theme::accent_fill(ui))
                .stroke(egui::Stroke::new(1.5_f32, Color32::from_rgb(154, 201, 243)));
            let play = if !series || target.is_some() {
                play_button(app, ui, primary, tv)
            } else {
                ui.add_enabled(false, primary);
                false
            };
            if series && target.is_some() {
                let shuffle = ui.add(
                    egui::Button::new(t("Zufällige Folge", "Random episode"))
                        .min_size(vec2(0.0, play_size.y)),
                );
                random = hit(ui, &shuffle, tv);
            }
            let marked = app.is_favorite(&key);
            let heart = ui
                .add(
                    egui::Button::new(
                        RichText::new(if marked { "♥" } else { "♡" })
                            .size(if touch { 20.0 } else { 25.0 })
                            .color(if marked {
                                Color32::from_rgb(255, 135, 157)
                            } else {
                                Color32::WHITE
                            }),
                    )
                    .min_size(vec2(play_size.y * 0.85, play_size.y)),
                )
                .on_hover_text(t("Favorit", "Favorite"));
            super::desktop::remember_favorite_target(&heart, &key);
            if hit(ui, &heart, tv) {
                app.toggle_favorite(&key);
            }
            if series && !entry.plot.is_empty() {
                let description = ui.add(
                    egui::Button::new(t("Mehr lesen", "Read more"))
                        .min_size(vec2(0.0, play_size.y)),
                );
                if hit(ui, &description, tv) {
                    app.description_open = Some((entry.name.clone(), entry.plot.clone()));
                }
            }
            play
        })
        .inner;
    if series {
        let rails = Rect::from_min_max(pos2(content.left(), controls_bottom + gap), content.max);
        let mut rows = ui.child_ui_with_id_source(
            rails,
            Layout::top_down(Align::Min),
            ("series-rows", entry.id),
        );
        rows.set_clip_rect(rails.intersect(ui.clip_rect()));
        episode_rows(
            app,
            &mut rows,
            entry.id,
            target.as_ref().map(|(e, _)| e),
            vec2(rail_height, episode_height),
            gap,
            season_label_height,
            &entry.cover,
        );
        if play {
            app.play_series_resume(entry.id);
        } else if random {
            if let Some(episodes) = app
                .series_episodes_for(entry.id)
                .filter(|episodes| !episodes.is_empty())
            {
                let mut entropy = [0; 8];
                if getrandom::getrandom(&mut entropy).is_ok() {
                    let episode =
                        &episodes[(u64::from_ne_bytes(entropy) % episodes.len() as u64) as usize];
                    let item = episode_item(entry.id, episode);
                    app.play_untracked(item);
                }
            }
        }
    } else if play {
        app.play_selected(PlayItem {
            title: entry.name,
            url: entry.url,
            kind: PlayKind::Vod,
            progress_key: key,
        });
    }
}
fn play_button(app: &mut App, ui: &mut Ui, button: egui::Button<'_>, tv: bool) -> bool {
    let response = ui.add(button);
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
            url: String::new(),
        })
    } else {
        let v = app.library.movies.get(index)?;
        Some(Entry {
            id: v.id,
            name: v.name.clone(),
            cover: v.cover.clone(),
            plot: v.plot.clone(),
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
        if let Some(id) = if series {
            app.selected_series
        } else {
            app.selected_movie
        } {
            show_detail_page(app, ui, series, id);
            return;
        }
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
        let cr = Rect::from_min_size(a.min, vec2(cw, a.height()));
        let lr = Rect::from_min_max(pos2(cr.right() + gap, a.top()), a.max);
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
                    let remote = app.shared_remote_navigation();
                    super::desktop::search_field(
                        ui,
                        &mut app.catalog_queries[usize::from(series)],
                        language.text("Titel suchen", "Search titles"),
                        remote,
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
                    if button(ui, caption, app.shared_remote_navigation()) {
                        ui.ctx().data_mut(|data| data.insert_temp(id, !alphabetic));
                    }
                }
                categories(app, ui, series)
            });
            focus::set_zone(2);
            super::desktop::column(ui, lr, "catalog-posters", offset, |ui| {
                posters(app, ui, series, false)
            });
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
                    show_detail_page(app, ui, series, id);
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
    let tv = app.shared_remote_navigation();
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
    let alphabetic = ui.ctx().data(|data| {
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
    let tv = app.shared_remote_navigation();
    if !touch {
        if super::desktop::active(ui.ctx()) {
            let header = super::desktop::catalog_heading(
                ui,
                if series {
                    app.settings.language.text("Serien", "Series")
                } else {
                    app.settings.language.text("Filme", "Movies")
                },
            );
            if cfg!(windows) || app.tv {
                let legend = Rect::from_min_max(
                    pos2(header.left() + 115.0 * scale, header.top()),
                    header.max,
                );
                if legend.width() >= 230.0 {
                    super::desktop::browse_shortcuts(ui, legend, app.view, app.settings.language);
                }
            }
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
    let handheld = super::desktop::handheld();
    let a = super::desktop::appearance();
    let min_width = if classic {
        (a.classic_poster_width * scale).max(136.0)
    } else if desktop {
        (super::desktop::appearance().catalog_poster_width * scale).max(128.0)
    } else {
        128.0
    };
    let min_cols = if touch || handheld { 1 } else { 2 };
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
    let returning_to_poster = app
        .catalog_return_poster
        .filter(|&(is_series, _)| is_series == series)
        .map(|(_, id)| id);
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
                        if returning_movie == Some(entry.id)
                            || returning_to_poster == Some(entry.id)
                        {
                            response.request_focus();
                            if tv {
                                focus::request_id(response.id);
                            }
                            app.desktop_return_item = None;
                            app.catalog_return_poster = None;
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
                            let size = if handheld {
                                15.0
                            } else if classic {
                                (22.0 * scale).max(14.0)
                            } else if tv {
                                11.0
                            } else {
                                13.0
                            };
                            let caption = Rect::from_min_max(
                                pos2(art.left(), art.bottom()),
                                pos2(art.right(), rect.bottom() - 2.0),
                            );
                            components::draw_line(
                                ui,
                                pos2(caption.left() + 2.0, caption.center().y),
                                &entry.name,
                                tokens::medium(size),
                                ui.visuals().text_color(),
                                caption.width() - 4.0,
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
        app.catalog_return_poster = Some((series, id));
        if series {
            app.open_series(id);
        } else {
            app.selected_movie = Some(id);
            app.ask_resume(&format!("vod:{id}"));
        }
    }
}
fn episode_item(sid: u32, e: &Episode) -> PlayItem {
    PlayItem {
        title: format!("Staffel {} · Folge {}", e.season, e.episode),
        url: e.stream_url.clone(),
        kind: PlayKind::Episode,
        progress_key: format!("ep:{sid}:{}", e.id),
    }
}

fn episode_rows(
    app: &mut App,
    ui: &mut Ui,
    sid: u32,
    target: Option<&Episode>,
    heights: egui::Vec2,
    gap: f32,
    heading_height: f32,
    cover: &str,
) {
    let height = heights.x;
    let episode_height = heights.y;
    let tv = app.shared_remote_navigation();
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let Some(source) = app.series_episodes_for(sid) else {
        if app.series_failed(sid) {
            if button(ui, t("Folgen erneut laden", "Reload episodes"), tv) {
                app.retry_series(sid);
            }
        } else {
            ui.label(t("Folgen werden geladen …", "Loading episodes…"));
        }
        return;
    };
    if source.is_empty() {
        ui.label(t("Keine Folgen verfügbar.", "No episodes available."));
        return;
    }
    type Cached = ((usize, usize), Arc<(Vec<Episode>, Vec<u32>)>);
    let signature = (source.as_ptr() as usize, source.len());
    let cache = ui.id().with("episodes");
    let sorted = ui
        .data(|data| data.get_temp::<Cached>(cache))
        .filter(|value| value.0 == signature)
        .map(|value| value.1)
        .unwrap_or_else(|| {
            let mut episodes = source.clone();
            episodes.sort_by_key(|e| (e.season, e.episode, e.id));
            let mut seasons: Vec<_> = episodes.iter().map(|e| e.season).collect();
            seasons.dedup();
            let sorted = Arc::new((episodes, seasons));
            ui.data_mut(|data| data.insert_temp(cache, (signature, Arc::clone(&sorted))));
            sorted
        });
    let (episodes, seasons) = sorted.as_ref();
    let profile = app
        .active_profile
        .as_ref()
        .map(|p| p.id.as_str())
        .unwrap_or("");
    let locked = app
        .parental
        .policy
        .locked(profile, &format!("series:{sid}"));
    if locked {
        app.cancel_episode_frame();
    }
    let return_id = app.desktop_return_item.as_deref().and_then(|key| {
        let (series, episode) = key.strip_prefix("ep:")?.split_once(':')?;
        (series.parse::<u32>().ok()? == sid)
            .then(|| episode.parse::<u32>().ok())
            .flatten()
    });
    let returning = return_id.and_then(|id| episodes.iter().find(|episode| episode.id == id));
    let position_key = ui.id().with("episode-position");
    let previous = ui.data(|data| data.get_temp::<(u64, u32)>(position_key));
    let new_entry = previous.map(|(revision, _)| revision) != Some(app.series_open_revision);
    if let Some(e) = returning {
        app.series_season = e.season;
    } else if new_entry || seasons.binary_search(&app.series_season).is_err() {
        app.series_season = target.map_or(seasons[0], |e| e.season);
    }
    let tab = seasons.binary_search(&app.series_season).unwrap_or(0);
    let card_width = super::desktop::handheld_size((height * 2.9).max(92.0), 112.0);
    let step = card_width + gap;
    let bounds = ui.available_rect_before_wrap();
    let heading = Rect::from_min_size(bounds.min, vec2(bounds.width(), heading_height));
    ui.painter().text(
        heading.left_center(),
        egui::Align2::LEFT_CENTER,
        t("Staffeln", "Seasons"),
        tokens::bold(16.0),
        Color32::WHITE,
    );
    let tab_rect = Rect::from_min_size(
        pos2(bounds.left(), heading.bottom() + gap),
        vec2(bounds.width(), height),
    );
    let mut tabs =
        ui.child_ui_with_id_source(tab_rect, Layout::top_down(Align::Min), "season-tabs");
    tabs.set_clip_rect(tab_rect.intersect(ui.clip_rect()));
    let tabs_offset = (new_entry || returning.is_some()).then_some(tab as f32 * step);
    if let Some(selected) = episode_rail(
        &mut tabs,
        "seasons",
        seasons.len(),
        height,
        card_width,
        gap,
        tabs_offset,
        |ui, index, rect| {
            let season = seasons[index];
            let r = ui.put(
                rect,
                egui::SelectableLabel::new(
                    index == tab,
                    format!("{} {season}", t("Staffel", "Season")),
                ),
            );
            hit(ui, &r, tv)
        },
    ) {
        if app.series_season != seasons[selected] {
            app.cancel_episode_frame();
            app.series_season = seasons[selected];
        }
    }
    let season = app.series_season;
    let rows = &episodes[episodes.partition_point(|e| e.season < season)
        ..episodes.partition_point(|e| e.season <= season)];
    let target_id = target.filter(|e| e.season == season).map(|e| e.id);
    let focus_index = returning.and_then(|e| rows.iter().position(|row| row.id == e.id));
    let episode_width = (episode_height * 1.58).max(108.0);
    let episode_step = episode_width + gap;
    let offset = if let Some(index) = focus_index {
        Some(index as f32 * episode_step)
    } else if new_entry || previous.map(|(_, old)| old) != Some(season) {
        Some(
            target_id
                .and_then(|id| rows.iter().position(|e| e.id == id))
                .unwrap_or(0) as f32
                * episode_step,
        )
    } else {
        None
    };
    let episode_rect = Rect::from_min_max(
        pos2(
            bounds.left(),
            (bounds.bottom() - episode_height).max(tab_rect.bottom()),
        ),
        bounds.max,
    );
    let mut cards =
        ui.child_ui_with_id_source(episode_rect, Layout::top_down(Align::Min), "episode-cards");
    cards.set_clip_rect(episode_rect.intersect(ui.clip_rect()));
    let selected = episode_rail(
        &mut cards,
        ("episodes", season),
        rows.len(),
        episode_rect.height(),
        episode_width,
        gap,
        offset,
        |ui, index, rect| {
            let e = &rows[index];
            let key = format!("ep:{sid}:{}", e.id);
            let r = ui.interact(rect, ui.id().with(e.id), Sense::click());
            if ui.is_rect_visible(rect) {
                let fill = if target_id == Some(e.id) || r.hovered() {
                    crate::theme::accent_fill(ui)
                } else {
                    Color32::from_rgb(27, 50, 75)
                };
                ui.painter().rect_filled(rect, 8.0, fill);
                let art = Rect::from_min_max(rect.min + vec2(3.0, 3.0), rect.max - vec2(3.0, 21.0));
                let frame_key = format!("episode-frame:{sid}:{}", e.id);
                let fallback = if !locked && e.preview.trim() != cover.trim() {
                    e.preview.as_str()
                } else {
                    ""
                };
                if !locked {
                    app.request_episode_frame(&frame_key, e);
                }
                if let Some(image) = (!locked).then(|| app.image(&frame_key)).flatten() {
                    ui.painter().image(
                        image.id(),
                        art,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    components::thumb(app, ui, &key, fallback, art, "▶");
                }
                if let Some(duration) = app.durations.get(&key).filter(|duration| **duration > 0) {
                    let progress =
                        (app.get_progress(&key) as f32 / *duration as f32).clamp(0.0, 1.0);
                    if progress > 0.0 {
                        let bar = Rect::from_min_size(
                            art.left_bottom() + vec2(5.0, -7.0),
                            vec2((art.width() - 10.0) * progress, 4.0),
                        );
                        ui.painter()
                            .rect_filled(bar, 2.0, Color32::from_rgb(136, 197, 247));
                    }
                }
                ui.painter().text(
                    rect.left_bottom() + vec2(7.0, -10.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} {}", t("Folge", "Episode"), e.episode),
                    tokens::medium(13.0),
                    Color32::WHITE,
                );
            }
            if focus_index == Some(index) {
                r.request_focus();
                if tv {
                    focus::request_id(r.id);
                }
                app.desktop_return_item = None;
            }
            #[cfg(windows)]
            if r.has_focus() {
                super::desktop::remember_favorite_target(&r, &format!("ep:{sid}:{}", e.id));
            }
            hit(ui, &r, tv)
        },
    );
    ui.data_mut(|data| data.insert_temp(position_key, (app.series_open_revision, season)));
    if let Some(index) = selected {
        app.play_selected(episode_item(sid, &rows[index]));
    }
}

#[allow(clippy::too_many_arguments)]
fn episode_rail(
    ui: &mut Ui,
    id: impl Hash,
    count: usize,
    height: f32,
    width: f32,
    gap: f32,
    offset: Option<f32>,
    mut render: impl FnMut(&mut Ui, usize, Rect) -> bool,
) -> Option<usize> {
    let step = width + gap;
    let mut scroll = egui::ScrollArea::horizontal()
        .id_source(id)
        .max_height(height)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .auto_shrink([false, true]);
    if let Some(offset) = offset {
        let max = (count as f32 * step - gap - ui.available_width()).max(0.0);
        scroll = scroll.horizontal_scroll_offset(offset.min(max));
    }
    let mut selected = None;
    scroll.show_viewport(ui, |ui, viewport| {
        let origin = ui.cursor().min;
        ui.set_min_size(vec2((count as f32 * step - gap).max(0.0), height));
        let visible = (viewport.min.x / step) as usize;
        let end = ((viewport.max.x / step).ceil() as usize + 3).min(count);
        for index in visible.saturating_sub(3)..end {
            let rect =
                Rect::from_min_size(origin + vec2(index as f32 * step, 0.0), vec2(width, height));
            let active = ui.push_id(index, |ui| render(ui, index, rect)).inner;
            if active {
                selected = Some(index);
            }
        }
    });
    selected
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
            if app.shared_remote_navigation() {
                focus::begin_modal(0);
            }
            egui::ScrollArea::vertical()
                .max_height(ctx.screen_rect().height() * 0.6)
                .show(ui, |ui| {
                    ui.label(text);
                });
            close = button(ui, "Schließen", app.shared_remote_navigation());
        });
    if !open || close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.description_open = None;
    }
}
