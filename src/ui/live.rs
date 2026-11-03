use crate::app::App;
use crate::theme;
use crate::types::{Channel, PlayItem, PlayKind};
use crate::ui;
use egui::{Align, Color32, Layout, RichText, Sense, Stroke, Ui, Vec2};

const PER_PAGE: usize = 100;

/// What a click on a channel should do.
enum Click {
    /// Show it in the side preview, list stays visible.
    Preview(PlayItem),
    /// Open the big player window (no fullscreen).
    Big(PlayItem),
    /// Open the big player in fullscreen.
    Full(PlayItem),
}

pub fn show(app: &mut App, ui: &mut Ui) {
    header(app, ui);

    // ---- filtering (indices only, so we never clone the whole 4k list) ----
    let cat = if app.live_cat == "all" {
        None
    } else {
        Some(app.live_cat.clone())
    };
    let q = app.live_q.trim().to_lowercase();
    let idx: Vec<usize> = app
        .library
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| cat.as_ref().map_or(true, |id| &c.category_id == id))
        .filter(|(_, c)| q.is_empty() || c.name.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();

    // Remember the visible order so ←/→ can walk through exactly this list.
    app.live_order = idx.iter().map(|&i| app.library.channels[i].id).collect();

    // If a channel was picked from outside (arrow keys), show its page.
    if let Some(target) = app.scroll_to {
        if let Some(pos) = idx.iter().position(|&i| app.library.channels[i].id == target) {
            app.live_page = pos / PER_PAGE;
        }
    }

    let total = idx.len();
    let pages = (total + PER_PAGE - 1) / PER_PAGE;
    let page = app.live_page.min(pages.saturating_sub(1));
    app.live_page = page;
    let start = page * PER_PAGE;
    let end = (start + PER_PAGE).min(total);

    ui::pager(ui, &mut app.live_page, pages, total, "Kanäle");
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(
                RichText::new("Hoch/Runter oder Links/Rechts: Kanal wechseln  ·  +/-: Lautstärke  ·  Klick = Vorschau, nochmal = Vollbild")
                    .size(11.0)
                    .weak(),
            ),
        );
    });
    ui.add_space(2.0);

    // Only the visible page is cloned.
    let slice: Vec<Channel> = idx[start..end]
        .iter()
        .map(|&i| app.library.channels[i].clone())
        .collect();

    let mut action: Option<Click> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let now = chrono::Utc::now().timestamp();

            for ch in &slice {
                let key = format!("live:{}", ch.id);
                let playing = app.is_previewing(&key);
                let (now_txt, next_txt) = epg_lines(app, ch.id, now);

                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui::ROW_H),
                    Sense::click(),
                );

                let bg = if playing {
                    theme::ACCENT_DIM
                } else if resp.hovered() {
                    ui.visuals().widgets.hovered.weak_bg_fill
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, 10.0, bg);
                if playing {
                    ui.painter()
                        .rect_stroke(rect, 10.0, Stroke::new(1.0_f32, theme::ACCENT));
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                let mut row = ui.child_ui(rect, Layout::left_to_right(Align::Center));
                row.add_space(10.0);
                ui::image_or_placeholder(
                    app,
                    &mut row,
                    &format!("logo:{}", ch.id),
                    &ch.logo,
                    Vec2::splat(38.0),
                );
                row.add_space(10.0);

                let text_w = (rect.width() - 190.0).max(80.0);
                // keep the text block vertically centred for 1–3 lines
                let pad = match (&now_txt, &next_txt) {
                    (None, None) => 18.0,
                    (Some(_), None) | (None, Some(_)) => 9.0,
                    _ => 2.0,
                };
                row.allocate_ui_with_layout(
                    Vec2::new(text_w, ui::ROW_H),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.add_space(pad);
                        ui::one_line(ui, &ch.name, 15.0, true);
                        if let Some(t) = &now_txt {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("Jetzt: {}", t)).size(11.5).weak(),
                                )
                                .truncate(true),
                            );
                        }
                        if let Some(t) = &next_txt {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("Danach: {}", t)).size(11.5).weak(),
                                )
                                .truncate(true),
                            );
                        }
                    },
                );

                row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui
                        .button("▶")
                        .on_hover_text("Im großen Player öffnen")
                        .clicked()
                    {
                        action = Some(Click::Big(play_item(ch)));
                    }
                    ui::fav_button(app, ui, &key);
                });

                // Click anywhere on the row (name included) starts the preview.
                // Clicking the same channel again goes fullscreen (a quick
                // double click does the same). Never override the ▶ button.
                if action.is_none() && (resp.clicked() || resp.double_clicked()) {
                    action = Some(if playing {
                        Click::Full(play_item(ch))
                    } else {
                        Click::Preview(play_item(ch))
                    });
                }

                // Keep the keyboard-selected channel visible.
                if app.scroll_to == Some(ch.id) {
                    resp.scroll_to_me(Some(Align::Center));
                    app.scroll_to = None;
                }

                app.request_epg(ch.id);
                ui.add_space(2.0);
            }
        });

    match action {
        Some(Click::Preview(item)) => app.start_preview(item),
        Some(Click::Big(item)) => {
            if app.is_previewing(&item.progress_key) {
                app.promote_preview(false);
            } else {
                app.play(item);
            }
        }
        Some(Click::Full(item)) => {
            if app.is_previewing(&item.progress_key) {
                app.promote_preview(true);
            } else {
                app.start_preview(item);
            }
        }
        None => {}
    }
}

fn play_item(ch: &Channel) -> PlayItem {
    PlayItem {
        title: ch.name.clone(),
        url: ch.stream_url.clone(),
        kind: PlayKind::Live,
        progress_key: format!("live:{}", ch.id),
    }
}

fn epg_lines(app: &App, id: u32, now: i64) -> (Option<String>, Option<String>) {
    match app.epg_for(id) {
        Some(epg) => {
            let cur = epg
                .iter()
                .find(|e| e.start <= now && now < e.end)
                .map(|e| e.title.clone());
            let nxt = epg.iter().find(|e| e.start > now).map(|e| e.title.clone());
            (cur, nxt)
        }
        None => (None, None),
    }
}

fn header(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new("Live TV").size(22.0).strong()));
        ui.add_space(8.0);

        let selected = if app.live_cat == "all" {
            "Alle Kategorien".to_string()
        } else {
            app.library
                .live_categories
                .iter()
                .find(|c| c.id == app.live_cat)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Alle Kategorien".to_string())
        };

        let mut cat_changed = false;
        egui::ComboBox::from_id_source("live_cat")
            .selected_text(selected)
            .width(210.0)
            .show_ui(ui, |cb| {
                cat_changed |= cb
                    .selectable_value(&mut app.live_cat, "all".to_string(), "Alle Kategorien")
                    .clicked();
                for c in &app.library.live_categories {
                    cat_changed |= cb
                        .selectable_value(&mut app.live_cat, c.id.clone(), c.name.clone())
                        .clicked();
                }
            });
        if cat_changed {
            app.live_page = 0;
        }

        ui.add_space(6.0);
        let resp = ui.add(
            egui::TextEdit::singleline(&mut app.live_q)
                .hint_text("Kanal suchen…")
                .desired_width(210.0),
        );
        if resp.changed() {
            app.live_page = 0;
        }
        if !app.live_q.is_empty() && ui.button("×").clicked() {
            app.live_q.clear();
            app.live_page = 0;
        }
    });
    ui.add_space(2.0);
}

/// The inline preview: video + controls, drawn next to the still visible list.
pub fn preview_panel(app: &mut App, ui: &mut Ui) {
    let item = match app.preview.clone() {
        Some(i) => i,
        None => return,
    };
    let ch_id: Option<u32> = item
        .progress_key
        .strip_prefix("live:")
        .and_then(|s| s.parse().ok());

    let mut close = false;
    let mut promote = false;
    let mut fullscreen = false;
    let mut toggle = false;
    let mut step: i32 = 0;

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new("Vorschau").size(18.0).strong()));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button("×")
                .on_hover_text("Vorschau schließen (Kanal stoppen)")
                .clicked()
            {
                close = true;
            }
        });
    });
    ui.add_space(4.0);

    // 16:9 video area
    let w = ui.available_width();
    let h = (w * 9.0 / 16.0).max(150.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    ui.painter().rect_filled(rect, 10.0, Color32::BLACK);
    app.position_preview(rect, ui.ctx().pixels_per_point());

    if let Some(e) = &app.player_error {
        ui.add_space(4.0);
        ui.colored_label(theme::DANGER, format!("Player: {}", e));
    }

    ui.add_space(8.0);
    ui::one_line(ui, &item.title, 16.0, true);

    let paused = app.player.as_ref().map(|p| p.paused).unwrap_or(false);
    ui.horizontal(|ui| {
        if ui.button("◀").on_hover_text("Vorheriger Kanal").clicked() {
            step = -1;
        }
        if ui
            .button(if paused { "▶" } else { "⏸" })
            .on_hover_text(if paused { "Abspielen" } else { "Pause" })
            .clicked()
        {
            toggle = true;
        }
        if ui.button("▶").on_hover_text("Nächster Kanal").clicked() {
            step = 1;
        }
        ui.add_space(4.0);
        if ui
            .button("Großer Player")
            .on_hover_text("In den Player wechseln")
            .clicked()
        {
            promote = true;
        }
        if ui.button("Vollbild").clicked() {
            fullscreen = true;
        }
        ui::fav_button(app, ui, &item.progress_key);
    });

    // EPG for the previewed channel
    if let Some(id) = ch_id {
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(2.0);
        let now = chrono::Utc::now().timestamp();
        let epg_opt = app.epg_for(id).cloned();
        let pending = app.epg_pending(id);
        match epg_opt {
            Some(epg) if !epg.is_empty() => {
                let cur = epg.iter().find(|e| e.start <= now && now < e.end);
                if let Some(cur) = cur {
                    ui.add(egui::Label::new(RichText::new("Jetzt").size(11.5).weak()));
                    ui.add(egui::Label::new(
                        RichText::new(format!(
                            "{} – {}",
                            fmt_clock(cur.start),
                            fmt_clock(cur.end)
                        ))
                        .size(11.5)
                        .weak(),
                    ));
                    ui.add(egui::Label::new(RichText::new(&cur.title).size(14.0)).truncate(true));
                }
                ui.add_space(6.0);
                ui.add(egui::Label::new(RichText::new("Danach").size(11.5).weak()));
                let mut shown = 0;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for e in epg.iter().filter(|e| e.start > now).take(10) {
                            ui.add(egui::Label::new(
                                RichText::new(format!("{}  {}", fmt_clock(e.start), e.title))
                                    .size(12.5)
                                    .weak(),
                            ));
                            shown += 1;
                        }
                    });
                if shown == 0 {
                    ui.add(egui::Label::new(
                        RichText::new("Keine weiteren EPG-Daten.").size(12.0),
                    ));
                }
            }
            _ => {
                if pending {
                    ui.add(egui::Label::new(
                        RichText::new("EPG wird geladen…").size(12.0).weak(),
                    ));
                }
            }
        }
    }

    if toggle {
        if let Some(pl) = &app.player {
            pl.mpv.toggle_pause();
        }
    }
    if step != 0 {
        app.step_channel(step);
    }
    if close {
        app.close_preview();
    }
    if promote {
        app.promote_preview(false);
    }
    if fullscreen {
        app.promote_preview(true);
    }
}

fn fmt_clock(ts: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string())
}
