use crate::app::{App, View};
use crate::theme;
use crate::types::{PlayItem, PlayKind};
use crate::ui;
use crate::ui::mobile::{active as mobile_active, tokens::GAP};
use egui::{Align, Color32, Layout, RichText, Sense, Stroke, Ui, Vec2};

const POSTER_W: f32 = 112.0;
const POSTER_H: f32 = 166.0;

/// Full-width content dashboard inspired by the supplied reference capture.
pub fn show(app: &mut App, ui: &mut Ui) {
    let desktop = cfg!(target_os = "windows") && !mobile_active(ui.ctx());
    if desktop && ui.available_width() >= 940.0 {
        let gap = 20.0;
        let right = (ui.available_width() * 0.28).clamp(280.0, 340.0);
        let left = ui.available_width() - right - gap;
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(
                Vec2::new(left, height),
                Layout::top_down(Align::Min),
                |ui| show_content(app, ui, false),
            );
            ui.allocate_ui_with_layout(
                Vec2::new(right, height),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_source("favorite-epg-column")
                        .show(ui, |ui| favorite_epg(app, ui));
                },
            );
        });
    } else {
        show_content(app, ui, desktop);
    }
}

fn show_content(app: &mut App, ui: &mut Ui, inline_favorites: bool) {
    egui::ScrollArea::vertical()
        .id_source("home-page-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            theme::ambient(ui,ui.max_rect());
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            // Home is the landing view on desktop AND mobile (mobile/mod.rs
            // routes View::Home here), so one banner covers both platforms.
            ui::catalog_banner(app, ui);
            let featured = featured_item(app);
            if let Some(item) = featured {
                hero(app, ui, &item);
            } else if app.library.channels.is_empty() {
                empty_hero(app, ui);
            }

            let recent = continue_items(app);
            if !recent.is_empty() {
                // "Weiter ansehen" gets split by category so each type has its
                // own rail: series, movies and live TV.
                let series_items = filter_by_kind(&recent, PlayKind::Episode);
                let vod_items = filter_by_kind(&recent, PlayKind::Vod);
                let live_items = filter_by_kind(&recent, PlayKind::Live);
                if !series_items.is_empty() {
                    poster_rail(app, ui, "Weiter ansehen · Serien", &series_items, true, Some(View::Continue));
                }
                if !vod_items.is_empty() {
                    poster_rail(app, ui, "Weiter ansehen · Filme", &vod_items, true, Some(View::Continue));
                }
                if !live_items.is_empty() {
                    if cfg!(target_os="windows") {
                        let channels:Vec<_>=live_items.iter().map(|(key,name,logo,url,_)|(key.clone(),name.clone(),logo.clone(),url.clone())).collect();
                        live_rail(app,ui,"Zuletzt gesehene Sender",&channels,Some(View::Continue));
                    }else{
                        poster_rail(app, ui, "Weiter ansehen · Live-TV", &live_items, true, Some(View::Continue));
                    }
                }
            }

            let live = favorite_live_items(app);
            if inline_favorites {
                favorite_epg(app,ui);
            } else if !cfg!(target_os="windows") && !live.is_empty() {
                live_rail(app, ui, "Live jetzt auf deinen Favoriten", &live, Some(View::Favorites));
            }

            let favorites = favorite_vod_items(app);
            if !favorites.is_empty() {
                poster_rail(app, ui, "Favoriten: Filme & Serien", &favorites, false, Some(View::Favorites));
            }

            let cache_id=ui.id().with("discovery-rails");
            let signature=(app.catalog_generation(),app.library.movies.len(),app.library.series.len());
            type Rail=Vec<(String,String,String,String,PlayKind)>;
            type Cached=((u64,usize,usize),std::sync::Arc<(Rail,Rail)>);
            let rails=ui.data(|d|d.get_temp::<Cached>(cache_id)).filter(|c|c.0==signature).map(|c|c.1).unwrap_or_else(||{
                let result=std::sync::Arc::new((recently_added_items(app),discover_series_items(app)));
                ui.data_mut(|d|d.insert_temp(cache_id,(signature,result.clone())));
                result
            });
            let (movies,series)=rails.as_ref();
            if !movies.is_empty() {
                poster_rail(app, ui, "Zuletzt hinzugefügt", movies, false, Some(View::Movies));
            }


            if !series.is_empty() {
                poster_rail(app, ui, "Serien entdecken", series, false, Some(View::Series));
            }

            if recent.is_empty() && live.is_empty() && favorites.is_empty() && movies.is_empty() && series.is_empty() {
                ui.add_space(18.0);
                ui.label(RichText::new("Deine Playlist ist bereit. Markiere Sender und Titel als Favoriten, damit sie hier erscheinen.")
                    .size(12.0).color(theme::muted(ui)));
            }
            // Mobile: the bottom navigation bar is a fixed overlay above the
            // central panel, so leave a full nav-height + gap of clearance so
            // the last rail can scroll clear instead of straddling the tabs.
            if mobile_active(ui.ctx()) {
                ui.add_space(GAP);
            } else {
                ui.add_space(20.0);
            }
        });
}

fn filter_by_kind(
    items: &[(String, String, String, String, PlayKind)],
    kind: PlayKind,
) -> Vec<(String, String, String, String, PlayKind)> {
    items
        .iter()
        .filter(|(_, _, _, _, k)| *k == kind)
        .cloned()
        .collect()
}

fn featured_item(app: &App) -> Option<PlayItem> {
    app.history
        .iter()
        .filter_map(|h| app.resolve_play(&h.key))
        .find(|item| item.kind != PlayKind::Live)
        .or_else(|| {
            app.library.movies.first().map(|m| PlayItem {
                title: m.name.clone(),
                url: m.stream_url.clone(),
                kind: PlayKind::Vod,
                progress_key: format!("vod:{}", m.id),
            })
        })
}

fn hero(app: &mut App, ui: &mut Ui, item: &PlayItem) {
    let width = ui.available_width();
    let compact = width < 560.0;
    let h = if compact {
        (width * 0.56).clamp(164.0, 232.0)
    } else {
        (width * 0.27).clamp(228.0, 320.0)
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
    theme::glass(ui, rect, 24.0);
    let cover = cover_for(app, &item.progress_key);
    let image_width = if compact { width * 0.28 } else { width * 0.40 };
    if !cover.is_empty() {
        let key = format!("hero:{}", item.progress_key);
        app.request_image(key.clone(), cover);
        if let Some(tex) = app.image(&key) {
            let art = egui::Rect::from_min_max(
                egui::pos2(rect.right() - image_width, rect.top()),
                rect.max,
            );
            let ratio = art.aspect_ratio() / (tex.size()[0] as f32 / tex.size()[1] as f32);
            let uv = egui::Rect::from_center_size(
                egui::pos2(0.5, 0.5),
                Vec2::new(ratio.min(1.0), (1.0 / ratio).min(1.0)),
            );
            ui.painter().image(tex.id(), art, uv, Color32::WHITE);
        }
    }
    let pad = if compact { 16.0 } else { 28.0 };
    let area = egui::Rect::from_min_max(
        rect.min + Vec2::splat(pad),
        egui::pos2(rect.right() - image_width - pad, rect.bottom() - pad),
    );
    let mut info = ui.child_ui(area, Layout::top_down(Align::Min));
    info.label(
        RichText::new(match item.kind {
            PlayKind::Live => "Live-TV",
            PlayKind::Vod => "Film",
            PlayKind::Episode => "Serie",
        })
        .color(theme::ACCENT),
    );
    info.add_space(6.0);
    let font = egui::FontId::proportional(if compact { 22.0 } else { 32.0 });
    let mut job = egui::text::LayoutJob::simple(
        item.title.clone(),
        font,
        ui.visuals().text_color(),
        area.width(),
    );
    job.wrap.max_rows = 2;
    info.label(job);
    let button = egui::Rect::from_min_size(
        egui::pos2(area.left(), area.bottom() - 40.0),
        Vec2::new(area.width().min(152.0), 40.0),
    );
    let label = if app.get_progress(&item.progress_key) > 1 {
        "Fortsetzen"
    } else {
        "Abspielen"
    };
    if info
        .put(
            button,
            egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
                .fill(theme::ACCENT_FILL)
                .rounding(20.0),
        )
        .clicked()
    {
        app.play(item.clone());
    }
}

fn empty_hero(app: &mut App, ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 250.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::CARD_DARK);
    let mut child = ui.child_ui(rect.shrink(24.0), Layout::top_down(Align::Min));
    child.label(
        RichText::new("DADO PLAYER  ·  STARTSEITE")
            .size(10.0)
            .strong()
            .color(theme::accent(&child)),
    );
    child.add_space(28.0);
    child.label(
        RichText::new("Dein IPTV. Dein Überblick.")
            .size(25.0)
            .strong(),
    );
    child.add_space(8.0);
    child.label(
        RichText::new("Entdecke Live-TV, Filme und Serien aus deiner Playlist.")
            .size(13.0)
            .color(theme::muted(&child)),
    );
    child.add_space(14.0);
    if child.button("Live-TV öffnen").clicked() {
        app.view = View::Live;
    }
}

fn continue_items(app: &App) -> Vec<(String, String, String, String, PlayKind)> {
    app.history
        .iter()
        .filter_map(|h| {
            let item = app.resolve_play(&h.key)?;
            let key = item.progress_key.clone();
            let cover = cover_for(app, &key);
            Some((key, item.title, cover, item.url, item.kind))
        })
        .take(10)
        .collect()
}

fn recently_added_items(app: &App) -> Vec<(String, String, String, String, PlayKind)> {
    // Rank by (timestamp, kind, index) first and clone only the 12 winners.
    // The previous version cloned name + cover + URL for EVERY movie and
    // series on every single frame, even on a 20,000-title provider, only to
    // throw all but 12 away. That was the largest avoidable allocation on Home.
    let mut ranked: Vec<(i64, u8, usize)> = Vec::new();
    for (i, m) in app.library.movies.iter().enumerate() {
        if let Some(added) = m.added.filter(|t| *t > 0) {
            ranked.push((added, 0, i));
        }
    }
    // IPTVnator parity: recently added series share the rail with movies.
    for (i, s) in app.library.series.iter().enumerate() {
        if let Some(added) = s.added.filter(|t| *t > 0) {
            ranked.push((added, 1, i));
        }
    }
    ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    ranked
        .into_iter()
        .take(12)
        .map(|(_, kind, i)| {
            if kind == 0 {
                let m = &app.library.movies[i];
                (
                    format!("vod:{}", m.id),
                    m.name.clone(),
                    m.cover.clone(),
                    m.stream_url.clone(),
                    PlayKind::Vod,
                )
            } else {
                let s = &app.library.series[i];
                (
                    format!("series:{}", s.id),
                    s.name.clone(),
                    s.cover.clone(),
                    String::new(),
                    PlayKind::Episode,
                )
            }
        })
        .collect()
}

fn discover_series_items(app: &App) -> Vec<(String, String, String, String, PlayKind)> {
    // Keep this rail distinct from "Zuletzt hinzugefügt": only series without
    // provider timestamps appear here, avoiding duplicate series posters while
    // ensuring providers that omit `added` still expose their catalogue.
    // Lowercased sort keys are built once per series, not once per comparison
    // inside the sort (which would be O(n log n) String allocations).
    let mut items: Vec<(usize, String)> = app
        .library
        .series
        .iter()
        .enumerate()
        .filter(|(_, s)| s.added.filter(|timestamp| *timestamp > 0).is_none())
        .map(|(i, s)| (i, s.name.to_lowercase()))
        .collect();
    items.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then(app.library.series[a.0].id.cmp(&app.library.series[b.0].id))
    });
    items
        .into_iter()
        .take(12)
        .map(|(i, _)| {
            let s = &app.library.series[i];
            (
                format!("series:{}", s.id),
                s.name.clone(),
                s.cover.clone(),
                String::new(),
                PlayKind::Episode,
            )
        })
        .collect()
}

fn favorite_vod_items(app: &App) -> Vec<(String, String, String, String, PlayKind)> {
    let mut out: Vec<_> = app
        .library
        .movies
        .iter()
        .filter(|m| app.favorites.contains(&format!("vod:{}", m.id)))
        .map(|m| {
            (
                format!("vod:{}", m.id),
                m.name.clone(),
                m.cover.clone(),
                m.stream_url.clone(),
                PlayKind::Vod,
            )
        })
        .collect();
    out.extend(
        app.library
            .series
            .iter()
            .filter(|s| app.favorites.contains(&format!("series:{}", s.id)))
            .map(|s| {
                (
                    format!("series:{}", s.id),
                    s.name.clone(),
                    s.cover.clone(),
                    String::new(),
                    PlayKind::Episode,
                )
            }),
    );
    out
}

fn cover_for(app: &App, key: &str) -> String {
    if let Some(id) = key
        .strip_prefix("live:")
        .and_then(|s| s.parse::<u32>().ok())
    {
        return app
            .library
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.logo.clone())
            .unwrap_or_default();
    }
    if let Some(id) = key.strip_prefix("vod:").and_then(|s| s.parse::<u32>().ok()) {
        return app
            .library
            .movies
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.cover.clone())
            .unwrap_or_default();
    }
    if let Some(rest) = key.strip_prefix("ep:") {
        if let Some(id) = rest.split(':').next().and_then(|s| s.parse::<u32>().ok()) {
            return app
                .library
                .series
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.cover.clone())
                .unwrap_or_default();
        }
    }
    String::new()
}

fn poster_rail(
    app: &mut App,
    ui: &mut Ui,
    title: &str,
    items: &[(String, String, String, String, PlayKind)],
    show_progress: bool,
    target: Option<View>,
) {
    section_title(app, ui, title, items.len(), target);
    let mut play = None;
    egui::ScrollArea::horizontal()
        .id_source(("home-poster-rail", title))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (key, name, cover, url, kind) in items {
                    ui.vertical(|ui| {
                        ui.set_width(POSTER_W);
                        let resp = home_poster(app, ui, key, cover, name);
                        if resp.clicked() {
                            if let Some(id) = key
                                .strip_prefix("series:")
                                .and_then(|s| s.parse::<u32>().ok())
                            {
                                app.open_series(id);
                                ui.ctx().request_repaint();
                            } else if let Some(item) = app.resolve_play(key) {
                                play = Some(item);
                            } else if !url.is_empty() {
                                play = Some(PlayItem {
                                    title: name.clone(),
                                    url: url.clone(),
                                    kind: *kind,
                                    progress_key: key.clone(),
                                });
                            }
                        }
                        ui.add_space(3.0);
                        ui::one_line(ui, name, 12.0, true);
                        if show_progress {
                            if let Some(pos) = app.progress.get(key) {
                                ui.label(
                                    RichText::new(format!("Bei {}", ui::fmt_time(*pos as f64)))
                                        .size(10.0)
                                        .color(theme::muted(ui)),
                                );
                            }
                        }
                    });
                }
                ui.add_space(8.0);
            });
        });
    if let Some(item) = play {
        app.play(item);
        ui.ctx().request_repaint();
    }
}

fn home_poster(app: &mut App, ui: &mut Ui, key: &str, url: &str, title: &str) -> egui::Response {
    let size = Vec2::new(POSTER_W, POSTER_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    ui.painter().rect_filled(rect, 7.0, theme::WIDGET_DARK);
    if !url.is_empty() {
        let image_key = format!("home-poster:{}", key);
        app.request_image(image_key.clone(), url.to_string());
        if let Some(tex) = app.image(&image_key) {
            // "Contain" fit: keep the artwork's aspect ratio, centred in the
            // card, so wide channel logos are never squashed into the portrait
            // poster format.
            let ts = tex.size();
            let tr = ts[0] as f32 / ts[1] as f32;
            let cr = rect.width() / rect.height();
            let draw = if tr > cr {
                let w = rect.width();
                let h = w / tr;
                egui::Rect::from_center_size(rect.center(), Vec2::new(w, h))
            } else {
                let h = rect.height();
                let w = h * tr;
                egui::Rect::from_center_size(rect.center(), Vec2::new(w, h))
            };
            ui.painter().image(
                tex.id(),
                draw,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            poster_art(ui, rect, title, key);
        }
    } else {
        poster_art(ui, rect, title, key);
    }
    if resp.hovered() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            7.0,
            Stroke::new(1.0_f32, theme::accent(ui)),
        );
    }
    resp
}

fn poster_art(ui: &Ui, rect: egui::Rect, title: &str, key: &str) {
    let colors = [
        Color32::from_rgb(0x9C, 0x77, 0x43),
        Color32::from_rgb(0x3C, 0x55, 0x62),
        Color32::from_rgb(0x8B, 0x8A, 0x67),
        Color32::from_rgb(0x72, 0x75, 0x78),
        Color32::from_rgb(0xB1, 0x3D, 0x36),
        Color32::from_rgb(0x3F, 0x50, 0x68),
    ];
    let seed = key.bytes().fold(0usize, |a, b| a.wrapping_add(b as usize));
    let color = colors[seed % colors.len()];
    ui.painter().rect_filled(rect, 7.0, color);
    ui.painter()
        .rect_filled(rect, 7.0, Color32::from_black_alpha(28));
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, rect.top() + 12.0),
        egui::Align2::LEFT_TOP,
        title.to_uppercase(),
        egui::FontId::proportional(13.0),
        Color32::from_white_alpha(225),
    );
    let horizon = rect.top() + rect.height() * 0.68;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), horizon),
            egui::pos2(rect.right(), horizon),
        ],
        Stroke::new(1.0_f32, Color32::from_white_alpha(80)),
    );
    ui.painter().circle_filled(
        egui::pos2(rect.center().x, horizon - 8.0),
        12.0,
        Color32::from_black_alpha(35),
    );
}

fn favorite_live_items(app: &App) -> Vec<(String, String, String, String)> {
    app.library
        .channels
        .iter()
        .filter(|c| app.favorites.contains(&format!("live:{}", c.id)))
        .take(10)
        .map(|c| {
            (
                format!("live:{}", c.id),
                c.name.clone(),
                c.logo.clone(),
                c.stream_url.clone(),
            )
        })
        .collect()
}

fn live_rail(
    app: &mut App,
    ui: &mut Ui,
    title: &str,
    items: &[(String, String, String, String)],
    target: Option<View>,
) {
    section_title(app, ui, title, items.len(), target);
    let mut play = None;
    egui::ScrollArea::horizontal()
        .id_source("home-live-rail")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (key, name, logo, url) in items {
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(148.0, 58.0), Sense::click());
                    theme::glass(ui, rect, 12.0);
                    channel_logo(
                        app,
                        ui,
                        key,
                        logo,
                        egui::Rect::from_min_size(
                            rect.min + Vec2::new(8.0, 9.0),
                            Vec2::splat(40.0),
                        ),
                        name,
                    );
                    let text_rect = egui::Rect::from_min_max(
                        rect.min + Vec2::new(56.0, 12.0),
                        rect.max - Vec2::new(8.0, 8.0),
                    );
                    let mut child = ui.child_ui(text_rect, Layout::top_down(Align::Min));
                    ui::one_line(&mut child, name, 12.0, true);
                    child.label(RichText::new("Live-TV").size(10.0).color(theme::muted(ui)));
                    if resp.clicked() {
                        play = Some(PlayItem {
                            title: name.clone(),
                            url: url.clone(),
                            kind: PlayKind::Live,
                            progress_key: key.clone(),
                        });
                    }
                }
                ui.add_space(8.0);
            });
        });
    if let Some(item) = play {
        app.start_preview(item);
        app.view = View::Live;
        ui.ctx().request_repaint();
    }
}

fn channel_logo(app: &mut App, ui: &Ui, key: &str, url: &str, rect: egui::Rect, name: &str) {
    ui.painter()
        .rect_filled(rect, 8.0, ui.visuals().faint_bg_color);
    if !url.is_empty() {
        let cache = format!("home-channel:{key}");
        app.request_image(cache.clone(), url.to_owned());
        if let Some(tex) = app.image(&cache) {
            let size = tex.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y);
            let image_rect = egui::Rect::from_center_size(rect.center(), size * scale);
            ui.painter().image(
                tex.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            return;
        }
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        name.chars().take(2).collect::<String>(),
        egui::FontId::proportional(13.0),
        theme::muted(ui),
    );
}

fn favorite_epg(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);
    ui.label(
        RichText::new("Das läuft auf deinen Favoriten")
            .size(18.0)
            .strong(),
    );
    ui.add_space(12.0);
    let channels: Vec<_> = app
        .library
        .channels
        .iter()
        .filter(|c| app.favorites.contains(&format!("live:{}", c.id)))
        .cloned()
        .collect();
    if channels.is_empty() {
        ui.label(
            RichText::new("Markiere Live-Sender als Favoriten, um hier ihr Programm zu sehen.")
                .color(theme::muted(ui)),
        );
        return;
    }
    let now = chrono::Utc::now().timestamp();
    let mut play = None;
    for channel in channels {
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 96.0), Sense::click());
        if !ui.is_rect_visible(rect) {
            continue;
        }
        app.request_epg(channel.id);
        if app.epg_pending(channel.id) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        let current = app
            .epg_for(channel.id)
            .and_then(|entries| entries.iter().find(|e| e.start <= now && now < e.end))
            .cloned();
        if current.is_none() && app.epg_for(channel.id).is_some() && !app.epg_pending(channel.id) {
            let key = ui
                .id()
                .with(("favorite-epg-refresh", app.catalog_generation(), channel.id));
            let last = ui.data(|d| d.get_temp::<i64>(key)).unwrap_or(0);
            if now - last >= 60 {
                ui.data_mut(|d| d.insert_temp(key, now));
                app.retry_epg(channel.id);
            }
        }
        theme::glass(ui, rect, 14.0);
        channel_logo(
            app,
            ui,
            &format!("live:{}", channel.id),
            &channel.logo,
            egui::Rect::from_min_size(rect.min + Vec2::new(12.0, 16.0), Vec2::splat(40.0)),
            &channel.name,
        );
        let text = egui::Rect::from_min_max(
            rect.min + Vec2::new(64.0, 12.0),
            rect.max - Vec2::new(12.0, 12.0),
        );
        let mut child = ui.child_ui(text, Layout::top_down(Align::Min));
        ui::one_line(&mut child, &channel.name, 13.0, true);
        if let Some(entry) = current {
            ui::one_line(&mut child, &entry.title, 13.0, false);
            let local = |ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
                    .unwrap_or_default()
            };
            child.label(
                RichText::new(format!("{} – {}", local(entry.start), local(entry.end)))
                    .size(11.0)
                    .color(theme::muted(ui)),
            );
            let fraction = ((now - entry.start) as f32 / (entry.end - entry.start).max(1) as f32)
                .clamp(0.0, 1.0);
            let track = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 12.0, rect.bottom() - 7.0),
                Vec2::new(rect.width() - 24.0, 2.0),
            );
            ui.painter().rect_filled(track, 1.0, theme::hairline(ui));
            ui.painter().rect_filled(
                egui::Rect::from_min_size(track.min, Vec2::new(track.width() * fraction, 2.0)),
                1.0,
                theme::accent(ui),
            );
        } else {
            let status = if app.epg_pending(channel.id) {
                "Programm wird geladen …"
            } else if app.epg_failed(channel.id) {
                "EPG nicht erreichbar"
            } else {
                "Keine aktuelle EPG-Sendung"
            };
            child.label(RichText::new(status).size(12.0).color(theme::muted(ui)));
        }
        if resp.clicked() {
            play = Some(PlayItem {
                title: channel.name,
                url: channel.stream_url,
                kind: PlayKind::Live,
                progress_key: format!("live:{}", channel.id),
            });
        }
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(30));
    if let Some(item) = play {
        app.play(item);
    }
}

fn section_title(app: &mut App, ui: &mut Ui, title: &str, count: usize, target: Option<View>) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(15.0).strong());
        ui.label(
            RichText::new(format!("({})", count))
                .size(11.0)
                .color(theme::muted(ui)),
        );
        if let Some(view) = target {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn = ui.add(
                    egui::Button::new(
                        RichText::new("Alle anzeigen  ›")
                            .size(11.0)
                            .color(theme::accent(ui)),
                    )
                    .frame(false),
                );
                if btn.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if btn.clicked() {
                    app.view = view;
                }
            });
        }
    });
    ui.add_space(3.0);
}
