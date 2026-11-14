//! Category, channel and preview columns shared by desktop and TV mode.
use crate::app::App;
use crate::theme;
use crate::types::{Channel, PlayItem, PlayKind};
use crate::ui::mobile::{components, focus, tokens};
use egui::{pos2, vec2, Align, Color32, Layout, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    number_jump(app, ui);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(30));
    let tv = app.tv;
    let available = ui.available_rect_before_wrap();
    let gap = 12.0;
    let width = available.width().max(1.0);
    let has_preview = app.preview.is_some();
    let category_w = (width * if tv { 0.19 } else { 0.22 }).clamp(
        if tv { 118.0 } else { 130.0 },
        if tv { 250.0 } else { 220.0 },
    );
    let preview_w = if has_preview {
        (width * 0.40)
            .clamp(180.0, 600.0)
            .min((width - category_w - gap * 2.0) * 0.55)
    } else {
        0.0
    };
    let channel_w =
        (width - category_w - preview_w - gap * if has_preview { 2.0 } else { 1.0 }).max(1.0);
    egui::ScrollArea::horizontal()
        .id_source("live-columns-horizontal")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(
                    vec2(category_w, available.height()),
                    Layout::top_down(Align::Min),
                    |ui| categories(app, ui, true),
                );
                ui.allocate_ui_with_layout(
                    vec2(channel_w, available.height()),
                    Layout::top_down(Align::Min),
                    |ui| channels(app, ui, true),
                );
                if has_preview {
                    ui.allocate_ui_with_layout(
                        vec2(preview_w, available.height()),
                        Layout::top_down(Align::Min),
                        |ui| preview_panel(app, ui),
                    );
                }
            });
        });
}

/// TV keeps the category title in the brand row and search in the column
/// header, leaving the full remaining height for ten or more channel rows.
pub fn paint_tv_columns(ui: &Ui, content: Rect, nav: Rect, preview: bool) {
    let (category_w, channel_w, gap) = tv_columns(content, preview);
    let screen = ui.ctx().screen_rect();
    let channel_x = content.left() + category_w + gap;
    let shade = Color32::from_black_alpha(28);
    for (left, right) in [
        (screen.left(), nav.right() + 6.0),
        (channel_x - gap * 0.5, channel_x + channel_w + gap * 0.5),
    ] {
        ui.painter().rect_filled(
            Rect::from_min_max(pos2(left, screen.top()), pos2(right, screen.bottom())),
            0.0,
            shade,
        );
    }
}

fn tv_columns(content: Rect, preview: bool) -> (f32, f32, f32) {
    let gap = 10.0;
    let category_w = (content.width() * 0.18).clamp(116.0, 168.0);
    let preview_w = if preview {
        (content.width() * 0.37).clamp(220.0, 420.0)
    } else {
        0.0
    };
    (
        category_w,
        (content.width() - category_w - preview_w - gap * if preview { 2.0 } else { 1.0 }).max(1.0),
        gap,
    )
}

pub fn show_tv(app: &mut App, ui: &mut Ui, content: Rect, title_row: Rect) {
    number_jump(app, ui);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(30));
    let (category_w, channel_w, gap) = tv_columns(content, app.preview.is_some());
    let channels_x = content.left() + category_w + gap;
    let title = match app.live_cat.as_str() {
        "all" => "Alle Sender",
        "favorites" => "Favoriten",
        id => app
            .library
            .live_categories
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.as_str())
            .unwrap_or("Sender"),
    };
    components::draw_line(
        ui,
        pos2(channels_x, title_row.center().y),
        title,
        tokens::bold(18.0),
        tokens::text(ui),
        channel_w,
    );
    let header_h = 28.0;
    components::draw_line(
        ui,
        pos2(content.left(), title_row.center().y),
        "Kategorien",
        tokens::medium(16.0),
        tokens::text(ui),
        category_w,
    );
    let search_rect =
        Rect::from_min_size(pos2(channels_x, content.top()), vec2(channel_w, header_h));
    let mut search_ui = ui.child_ui(search_rect, Layout::top_down(Align::Min));
    search_ui.spacing_mut().interact_size.y = header_h;
    let text_h = search_ui.fonts(|fonts| fonts.row_height(&tokens::reg(16.0)));
    let search_margin_y = ((header_h - text_h) * 0.5).max(0.0);
    let query = search_ui.put(
        search_rect,
        egui::TextEdit::singleline(&mut app.live_q)
            .font(tokens::reg(16.0))
            .margin(vec2(6.0, search_margin_y))
            .hint_text("Sender suchen"),
    );
    if focus::clicked(&search_ui, &query) {
        query.request_focus();
    }
    let body_y = content.top() + header_h + 4.0;
    let mut category_ui = ui.child_ui(
        Rect::from_min_max(
            pos2(content.left(), content.top()),
            pos2(content.left() + category_w, content.bottom()),
        ),
        Layout::top_down(Align::Min),
    );
    category_ui.spacing_mut().item_spacing.y = 4.0;
    categories(app, &mut category_ui, false);
    let mut channel_ui = ui.child_ui(
        Rect::from_min_max(
            pos2(channels_x, body_y),
            pos2(channels_x + channel_w, content.bottom()),
        ),
        Layout::top_down(Align::Min),
    );
    channel_ui.spacing_mut().item_spacing.y = 3.0;
    channels(app, &mut channel_ui, false);
    if app.preview.is_some() {
        let preview_rect = Rect::from_min_max(
            pos2(channels_x + channel_w + gap, content.top()),
            content.max,
        );
        let mut preview_ui = ui.child_ui(preview_rect, Layout::top_down(Align::Min));
        preview_panel(app, &mut preview_ui);
    }
}

fn activated(ui: &Ui, response: &Response, tv: bool) -> bool {
    if tv {
        focus::clicked(ui, response)
    } else {
        response.clicked()
    }
}

fn row(ui: &mut Ui, id: impl std::hash::Hash, height: f32, selected: bool) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if selected {
            theme::selection_surface(ui)
        } else if response.hovered() {
            theme::card_hover(ui)
        } else {
            ui.visuals().faint_bg_color
        };
        ui.painter().rect_filled(rect.shrink(2.0), 12.0, color);
    }
    response
}

fn categories(app: &mut App, ui: &mut Ui, heading: bool) {
    let tv = app.tv;
    if heading {
        ui.label(
            RichText::new("Kategorien")
                .size(if tv { 20.0 } else { 22.0 })
                .strong(),
        );
        ui.add_space(10.0);
    }
    let mut list = vec![
        ("all".to_owned(), "Alle Sender".to_owned()),
        ("favorites".to_owned(), "Favoriten".to_owned()),
    ];
    list.extend(
        app.library
            .live_categories
            .iter()
            .map(|c| (c.id.clone(), c.name.clone())),
    );
    let mut selected = None;
    egui::ScrollArea::vertical()
        .id_source("live-categories")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (id, name) in list {
                let r = row(
                    ui,
                    ("category", &id),
                    if tv { 34.0 } else { 44.0 },
                    app.live_cat == id,
                );
                if ui.is_rect_visible(r.rect) {
                    components::draw_line(
                        ui,
                        pos2(r.rect.left() + 12.0, r.rect.center().y),
                        &name,
                        tokens::medium(if tv { 16.0 } else { 14.0 }),
                        ui.visuals().text_color(),
                        r.rect.width() - 24.0,
                    );
                }
                if activated(ui, &r, tv) {
                    selected = Some(id);
                }
            }
        });
    if let Some(id) = selected {
        if app.live_cat != id {
            app.live_cat = id;
            app.live_page = 0;
            app.scroll_to = None;
            app.close_preview();
        }
    }
}

fn channels(app: &mut App, ui: &mut Ui, controls: bool) {
    let tv = app.tv;
    if controls {
        ui.heading(if app.live_cat == "all" {
            "Alle Sender"
        } else if app.live_cat == "favorites" {
            "Favoriten"
        } else {
            app.library
                .live_categories
                .iter()
                .find(|c| c.id == app.live_cat)
                .map(|c| c.name.as_str())
                .unwrap_or("Sender")
        });
        ui.add_space(6.0);
        let query = ui.add_sized(
            [ui.available_width(), if tv { 42.0 } else { 30.0 }],
            egui::TextEdit::singleline(&mut app.live_q).hint_text("Sender suchen"),
        );
        if tv && focus::clicked(ui, &query) {
            query.request_focus();
        }
    }
    let search = app.live_q.trim().to_lowercase();
    let indices: Vec<_> = app
        .library
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| match app.live_cat.as_str() {
            "all" => true,
            "favorites" => app.is_favorite(&format!("live:{}", c.id)),
            cat => c.category_id == cat,
        })
        .filter(|(_, c)| search.is_empty() || c.name.to_lowercase().contains(&search))
        .map(|(i, _)| i)
        .collect();
    app.live_order = indices
        .iter()
        .map(|&i| app.library.channels[i].id)
        .collect();
    if controls {
        ui.label(
            RichText::new(format!("{} Sender", indices.len()))
                .small()
                .color(theme::muted(ui)),
        );
    }
    let tv_row_h = ((ui.available_height() - 9.0 * 3.0) / 10.0).clamp(34.0, 40.0);
    let (name_size, epg_size) = (13.0, 10.0);
    let (name_y, current_y, next_y) = ui.fonts(|fonts| {
        let name_h = fonts.row_height(&tokens::reg(name_size));
        let epg_h = fonts.row_height(&tokens::reg(epg_size));
        let top = ((tv_row_h - name_h - epg_h) * 0.5).max(0.0);
        (
            top + name_h * 0.5,
            top + name_h + epg_h * 0.5,
            top + name_h + epg_h * 0.5,
        )
    });
    let now = chrono::Utc::now().timestamp();
    let mut selected = None;
    egui::ScrollArea::vertical()
        .id_source(("live-channels", &app.live_cat))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if indices.is_empty() {
                ui.label("Keine passenden Sender");
            }
            for index in indices {
                let id = app.library.channels[index].id;
                let key = format!("live:{id}");
                let r = row(
                    ui,
                    ("channel", id),
                    if tv { tv_row_h } else { 90.0 },
                    app.is_previewing(&key),
                );
                if app.scroll_to == Some(id) {
                    ui.scroll_to_rect(r.rect, Some(Align::Center));
                    app.scroll_to = None;
                }
                if ui.is_rect_visible(r.rect) {
                    let channel = app.library.channels[index].clone();
                    app.request_epg(id);
                    if app.epg_pending(id) {
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(250));
                    }
                    let logo = Rect::from_min_size(
                        r.rect.min + vec2(8.0, if tv { (tv_row_h - 24.0) * 0.5 } else { 15.0 }),
                        Vec2::splat(if tv { 24.0 } else { 34.0 }),
                    );
                    components::thumb(
                        app,
                        ui,
                        &format!("logo:{id}"),
                        &channel.logo,
                        logo,
                        &channel.name,
                    );
                    let x = logo.right() + 10.0;
                    let width = (r.rect.right() - x - 10.0).max(1.0);
                    let (current, next) =
                        epg_lines(app.epg_for(id).map(Vec::as_slice).unwrap_or(&[]), now);
                    for (line, (offset, text, size, color)) in [
                        (
                            if tv { name_y } else { 20.0 },
                            channel.name,
                            if tv { name_size } else { 15.0 },
                            ui.visuals().text_color(),
                        ),
                        (
                            if tv { current_y } else { 45.0 },
                            current.map(|s| format!("Jetzt: {s}")).unwrap_or_else(|| {
                                if app.epg_pending(id) {
                                    "EPG wird geladen …".into()
                                } else {
                                    "Kein aktuelles EPG".into()
                                }
                            }),
                            if tv { epg_size } else { 12.0 },
                            theme::muted(ui),
                        ),
                        (
                            if tv { next_y } else { 68.0 },
                            next.map(|s| format!("Danach: {s}")).unwrap_or_default(),
                            if tv { epg_size } else { 12.0 },
                            theme::muted(ui),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let epg_width = ((width - 24.0) * 0.5).max(1.0);
                        let line_x = if tv && line == 2 {
                            x + epg_width + 24.0
                        } else {
                            x
                        };
                        components::draw_line(
                            ui,
                            pos2(line_x, r.rect.top() + offset),
                            &text,
                            tokens::reg(size),
                            color,
                            if tv && line > 0 { epg_width } else { width },
                        );
                    }
                }
                if activated(ui, &r, tv) {
                    selected = Some(index);
                }
            }
        });
    if let Some(index) = selected {
        let channel = app.library.channels[index].clone();
        app.select_live_channel(play_item(&channel));
        ui.ctx().request_repaint();
    }
}

fn play_item(channel: &Channel) -> PlayItem {
    PlayItem {
        title: channel.name.clone(),
        url: channel.stream_url.clone(),
        kind: PlayKind::Live,
        progress_key: format!("live:{}", channel.id),
    }
}

fn epg_lines(entries: &[crate::types::EpgEntry], now: i64) -> (Option<String>, Option<String>) {
    (
        entries
            .iter()
            .find(|e| e.start <= now && now < e.end)
            .map(|e| e.title.clone()),
        entries
            .iter()
            .filter(|e| e.start > now)
            .min_by_key(|e| e.start)
            .map(|e| e.title.clone()),
    )
}

pub fn preview_panel(app: &mut App, ui: &mut Ui) {
    let Some(item) = app.preview.clone() else {
        return;
    };
    let tv = app.tv;
    if tv {
        let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
        components::draw_line(
            ui,
            pos2(header.left(), header.center().y),
            "Vorschau",
            tokens::medium(16.0),
            tokens::text(ui),
            header.width() - 36.0,
        );
        let rect = Rect::from_min_max(pos2(header.right() - 32.0, header.top()), header.max);
        let r = ui.interact(rect, ui.id().with("preview-favorite"), Sense::click());
        let favorite = app.is_favorite(&item.progress_key);
        components::icon(
            ui,
            Rect::from_center_size(rect.center(), vec2(20.0, 20.0)),
            if favorite {
                components::Icon::StarOn
            } else {
                components::Icon::Star
            },
            theme::accent(ui),
        );
        if activated(ui, &r, true) {
            app.toggle_favorite(&item.progress_key);
        }
    } else {
        ui.heading("Vorschau");
    }
    let w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0), Sense::click());
    ui.painter().rect_filled(rect, 8.0, Color32::BLACK);
    let visible = rect.intersect(ui.clip_rect());
    if visible.is_positive() {
        app.position_preview(visible, ui.ctx().pixels_per_point());
        #[cfg(target_os = "android")]
        ui.painter()
            .add(crate::platform::android::render::shape(visible));
    }
    let promote = activated(ui, &response, tv);
    ui.add_space(8.0);
    if tv {
        ui.label(
            RichText::new("OK: Vollbild · Zurück: Schließen")
                .size(13.0)
                .color(theme::muted(ui)),
        );
    } else {
        ui.horizontal_wrapped(|ui| {
            let r = ui.button("Im Player öffnen");
            if activated(ui, &r, tv) {
                app.promote_preview(false);
            }
            let r = ui.button("Vorschau schließen");
            if activated(ui, &r, tv) {
                app.close_preview();
            }
        });
    }
    if promote {
        app.promote_preview(false);
    }
    if let Some(error) = app.player_error.as_ref() {
        ui.colored_label(theme::DANGER, error);
    }
    #[cfg(target_os = "android")]
    if let Some(error) = crate::platform::android::render::last_error() {
        ui.colored_label(theme::DANGER, error);
    }
    egui::ScrollArea::vertical()
        .id_source("live-preview-details")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.heading(&item.title);
            if let Some(channel) = item
                .progress_key
                .strip_prefix("live:")
                .and_then(|id| id.parse::<u32>().ok())
                .and_then(|id| app.library.channels.iter().find(|c| c.id == id))
                .cloned()
            {
                let category = app
                    .library
                    .live_categories
                    .iter()
                    .find(|c| c.id == channel.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                if !tv {
                    ui.label(format!("Kanal {} · {}", channel.num, category));
                    let r = ui.button(if app.is_favorite(&item.progress_key) {
                        "Aus Favoriten entfernen"
                    } else {
                        "Zu Favoriten"
                    });
                    if activated(ui, &r, tv) {
                        app.toggle_favorite(&item.progress_key);
                    }
                }
                app.request_epg(channel.id);
                let now = chrono::Utc::now().timestamp();
                let entries = app.epg_for(channel.id).cloned().unwrap_or_default();
                if let Some(current) = entries.iter().find(|e| e.start <= now && now < e.end) {
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "Jetzt · {} – {}",
                            fmt_clock(current.start),
                            fmt_clock(current.end)
                        ))
                        .size(if tv { 16.0 } else { 14.0 })
                        .strong(),
                    );
                    ui.label(RichText::new(&current.title).strong());
                    if !current.description.is_empty() {
                        ui.label(&current.description);
                    }
                }
                if let Some(next) = entries
                    .iter()
                    .filter(|e| e.start > now)
                    .min_by_key(|e| e.start)
                {
                    ui.separator();
                    ui.label(RichText::new("Danach").strong());
                    ui.label(format!(
                        "{} – {}",
                        fmt_clock(next.start),
                        fmt_clock(next.end)
                    ));
                    ui.label(&next.title);
                    if !next.description.is_empty() {
                        ui.label(&next.description);
                    }
                }
                if entries.is_empty() {
                    ui.label(if app.epg_pending(channel.id) {
                        "EPG wird geladen …"
                    } else {
                        "Keine EPG-Daten verfügbar"
                    });
                    if app.epg_failed(channel.id) {
                        let r = ui.button("EPG erneut laden");
                        if activated(ui, &r, tv) {
                            app.retry_epg(channel.id);
                        }
                    }
                }
            }
            if let Some(player) = app.player.as_ref() {
                let stream = &player.stream;
                ui.separator();
                ui.label(RichText::new("Stream").strong());
                if stream.width > 0 && stream.height > 0 {
                    ui.label(format!(
                        "{} × {} · {:.0} fps",
                        stream.width, stream.height, stream.fps
                    ));
                }
                if !stream.video_codec.is_empty() {
                    ui.label(format!("Video: {}", stream.video_codec));
                }
                if !stream.audio_codec.is_empty() {
                    ui.label(format!("Audio: {}", stream.audio_codec));
                }
                if !stream.container.is_empty() {
                    ui.label(format!("Format: {}", stream.container));
                }
            }
        });
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(100));
}

fn fmt_clock(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default()
}
fn number_jump(app: &mut App, ui: &mut Ui) {
    let typing_elsewhere = ui.ctx().memory(|m| m.focused().is_some());
    let (digits, enter, now) = ui.input(|i| {
        let mut d = String::new();
        for ev in &i.events {
            if let egui::Event::Text(t) = ev {
                d.extend(t.chars().filter(|c| c.is_ascii_digit()));
            }
        }
        (d, i.key_pressed(egui::Key::Enter), i.time)
    });

    if !typing_elsewhere && !digits.is_empty() {
        if app.chan_jump.len() < 5 {
            app.chan_jump.push_str(&digits);
        }
        app.chan_jump_at = now;
    }

    if app.chan_jump.is_empty() {
        return;
    }

    // Wait for the number to be complete: Enter commits immediately, otherwise
    // a short idle window covers "1" → "12" → "123".
    if !enter && now - app.chan_jump_at < 1.1 {
        jump_badge(app, ui);
        return;
    }

    let n: Option<u32> = app.chan_jump.parse().ok();
    let target = n.and_then(|n| {
        app.library
            .channels
            .iter()
            .find(|c| c.num == n)
            .or_else(|| app.library.channels.iter().find(|c| c.id == n))
            .map(|c| c.id)
    });
    app.chan_jump.clear();
    app.chan_jump_at = 0.0;

    if let Some(id) = target {
        // Clear filters so the channel is guaranteed to be on screen.
        app.live_cat = "all".to_string();
        app.live_q.clear();
        app.live_page = 0;
        app.scroll_to = Some(id);
        if let Some(c) = app.library.channels.iter().find(|c| c.id == id) {
            let item = play_item(c);
            app.play(item);
        }
    }
}

/// Small floating badge showing the digits typed so far.
fn jump_badge(app: &App, ui: &mut Ui) {
    let text = format!("Kanal {}", app.chan_jump);
    let screen = ui.ctx().screen_rect();
    let w = 180.0;
    let h = 44.0;
    let pos = pos2(screen.center().x - w * 0.5, screen.bottom() - h - 48.0);
    egui::Area::new(egui::Id::new("dado-chan-jump"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::none()
                .fill(ui.visuals().window_fill)
                .stroke(Stroke::new(1.0_f32, theme::accent(ui)))
                .rounding(egui::Rounding::same(10.0))
                .inner_margin(egui::Margin::symmetric(16.0, 9.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(text)
                            .size(19.0)
                            .strong()
                            .color(theme::accent_text(ui)),
                    );
                });
        });
}
