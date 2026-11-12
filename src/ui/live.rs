use crate::app::App;
use crate::theme;
use crate::types::{Channel, PlayItem, PlayKind};
use crate::ui;
use egui::{
    pos2, Align, Align2, Color32, FontId, Layout, RichText, Rounding, Sense, Stroke, Ui, Vec2,
};

const PER_PAGE: usize = 100;

/// What a click on a channel should do.
enum Click {
    /// Open the big player window (immediate playback, no preview step).
    Play(PlayItem),
}

pub fn show(app: &mut App, ui: &mut Ui) {
    number_jump(app, ui);
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
        .filter(|(_, c)| cat.as_ref().is_none_or(|id| &c.category_id == id))
        .filter(|(_, c)| q.is_empty() || c.name.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();

    // Remember the visible order so ←/→ can walk through exactly this list.
    app.live_order = idx.iter().map(|&i| app.library.channels[i].id).collect();

    // If a channel was picked from outside (arrow keys), show its page.
    if let Some(target) = app.scroll_to {
        if let Some(pos) = idx
            .iter()
            .position(|&i| app.library.channels[i].id == target)
        {
            app.live_page = pos / PER_PAGE;
        }
    }

    let total = idx.len();
    let pages = total.div_ceil(PER_PAGE);
    let page = app.live_page.min(pages.saturating_sub(1));
    app.live_page = page;
    let start = page * PER_PAGE;
    let end = (start + PER_PAGE).min(total);

    ui::pager(ui, &mut app.live_page, pages, total, "Kanäle");
    epg_timeline_controls(app, ui);
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(
                RichText::new("Hoch/Runter oder Links/Rechts: Kanal wechseln  ·  +/-: Lautstärke  ·  Klick = sofort starten")
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
                let entries = app.epg_for(ch.id).cloned().unwrap_or_default();
                let (now_txt, next_txt) = epg_lines(&entries, now);

                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui::ROW_H + 42.0),
                    Sense::click(),
                );

                let bg = if playing {
                    theme::selection_surface(ui)
                } else if resp.hovered() {
                    ui.visuals().widgets.hovered.weak_bg_fill
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, 10.0, bg);
                if playing {
                    ui.painter()
                        .rect_stroke(rect, 10.0, Stroke::new(1.0_f32, theme::accent(ui)));
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                let row_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right(), rect.top() + ui::ROW_H),
                );
                let mut row = ui.child_ui(row_rect, Layout::left_to_right(Align::Center));
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
                    ui::fav_button(app, ui, &key);
                });

                // Click anywhere on the row (name included) starts playback immediately.
                // No more preview step - user clicked, so play it.
                if action.is_none() && (resp.clicked() || resp.double_clicked()) {
                    action = Some(Click::Play(play_item(ch)));
                }

                // Keep the keyboard-selected channel visible.
                if app.scroll_to == Some(ch.id) {
                    resp.scroll_to_me(Some(Align::Center));
                    app.scroll_to = None;
                }

                draw_channel_timeline(app, ui, rect, &entries, now);

                // Thin progress bar for the programme that is on air now.
                if let Some(frac) = epg_progress(app, ch.id, now) {
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 58.0, rect.top() + ui::ROW_H - 4.0),
                        Vec2::new((rect.width() - 270.0).max(60.0), 2.0),
                    );
                    ui.painter().rect_filled(bar, 1.0, theme::hairline(ui));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(bar.min, Vec2::new(bar.width() * frac, 2.0)),
                        1.0,
                        theme::accent(ui),
                    );
                }

                app.request_epg(ch.id);
                ui.add_space(2.0);
            }
        });

    match action {
        Some(Click::Play(item)) => {
            // Immediate playback - no preview confirmation. `App::play` owns
            // the view transition; forcing View::Player here would break the
            // Android external-player hand-off, which deliberately stays on
            // the list so BACK returns to where the user left off.
            app.play(item);
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

/// Direct channel entry, like a remote control: type digits while Live TV is in
/// front and the app zaps to that channel number after a short pause (or Enter).
/// Digits are ignored while a text field has focus, so search keeps working.
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

fn epg_timeline_controls(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Programmübersicht").size(12.0).strong());
        ui.add_space(8.0);
        // Segmented control: Zurück | Zeit-Offset | Vor | Heute
        let seg_h = 28.0;
        let seg_pad = 2.0;
        let seg_w = 56.0;
        let total = seg_w * 4.0 + seg_pad * 3.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(total, seg_h), Sense::hover());
        let dark = ui.visuals().dark_mode;
        ui.painter().rect_filled(
            rect,
            Rounding::same(seg_h * 0.5),
            if dark {
                Color32::from_white_alpha(12)
            } else {
                Color32::from_black_alpha(8)
            },
        );
        let mut x = rect.left() + seg_pad;
        // Zurück
        let back_rect = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + seg_pad),
            Vec2::new(seg_w, seg_h - seg_pad * 2.0),
        );
        let back_resp = ui.interact(back_rect, ui.id().with("epg_back"), Sense::click());
        let back_fill = if back_resp.hovered() {
            if dark {
                Color32::from_white_alpha(30)
            } else {
                Color32::from_black_alpha(20)
            }
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(
            back_rect,
            Rounding::same(back_rect.height() * 0.5),
            back_fill,
        );
        ui.painter().text(
            back_rect.center(),
            Align2::CENTER_CENTER,
            "◀",
            FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
        if back_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if back_resp.clicked() {
            app.epg_offset_hours = (app.epg_offset_hours - 2).max(-12);
        }
        x += seg_w + seg_pad;
        // Offset Label
        let label_rect = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + seg_pad),
            Vec2::new(84.0, seg_h - seg_pad * 2.0),
        );
        let label = if app.epg_offset_hours == 0 {
            "Jetzt".to_string()
        } else {
            format!("{:+} Std.", app.epg_offset_hours)
        };
        ui.painter().rect_filled(
            label_rect,
            Rounding::same(label_rect.height() * 0.5),
            if dark {
                Color32::from_white_alpha(8)
            } else {
                Color32::from_black_alpha(6)
            },
        );
        ui.painter().text(
            label_rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(10.5),
            theme::muted(ui),
        );
        x += 84.0 + seg_pad;
        // Vor
        let fwd_rect = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + seg_pad),
            Vec2::new(seg_w, seg_h - seg_pad * 2.0),
        );
        let fwd_resp = ui.interact(fwd_rect, ui.id().with("epg_fwd"), Sense::click());
        let fwd_fill = if fwd_resp.hovered() {
            if dark {
                Color32::from_white_alpha(30)
            } else {
                Color32::from_black_alpha(20)
            }
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(fwd_rect, Rounding::same(fwd_rect.height() * 0.5), fwd_fill);
        ui.painter().text(
            fwd_rect.center(),
            Align2::CENTER_CENTER,
            "▶",
            FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
        if fwd_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if fwd_resp.clicked() {
            app.epg_offset_hours = (app.epg_offset_hours + 2).min(12);
        }
        x += seg_w + seg_pad;
        // Heute
        let today_rect = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + seg_pad),
            Vec2::new(seg_w, seg_h - seg_pad * 2.0),
        );
        let today_resp = ui.interact(today_rect, ui.id().with("epg_today"), Sense::click());
        let today_fill = if today_resp.hovered() {
            if dark {
                Color32::from_white_alpha(30)
            } else {
                Color32::from_black_alpha(20)
            }
        } else if app.epg_offset_hours == 0 {
            theme::accent_fill(ui).gamma_multiply(0.3)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(
            today_rect,
            Rounding::same(today_rect.height() * 0.5),
            today_fill,
        );
        ui.painter().text(
            today_rect.center(),
            Align2::CENTER_CENTER,
            "Heute",
            FontId::proportional(10.5),
            if app.epg_offset_hours == 0 {
                Color32::WHITE
            } else {
                ui.visuals().text_color()
            },
        );
        if today_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if today_resp.clicked() && app.epg_offset_hours != 0 {
            app.epg_offset_hours = 0;
        }
    });
}

fn draw_channel_timeline(
    app: &App,
    ui: &Ui,
    row: egui::Rect,
    epg: &[crate::types::EpgEntry],
    now: i64,
) {
    let left = row.left() + 190.0;
    let right = row.right() - 126.0;
    // timeline sits cleanly below the channel row with proper spacing
    let top = row.top() + ui::ROW_H + 6.0;
    let bottom = row.bottom() - 4.0;
    let width = (right - left).max(60.0);
    let area = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    let painter = ui.painter().with_clip_rect(area);
    painter.rect_filled(area, 4.0, theme::WIDGET_DARK);
    let start = now - now.rem_euclid(1800) + app.epg_offset_hours as i64 * 3600;
    let span = 6 * 3600_i64;
    // hour grid + labels
    let grid_bottom = top + 18.0;
    for hour in 0..=6 {
        let t = start + hour * 3600;
        let x = left + width * (hour as f32 / 6.0);
        painter.line_segment(
            [egui::pos2(x, top), egui::pos2(x, grid_bottom)],
            Stroke::new(1.0_f32, theme::hairline(ui)),
        );
        if hour < 6 {
            let text = chrono::DateTime::<chrono::Utc>::from_timestamp(t, 0)
                .map(|d| d.format("%H:%M").to_string())
                .unwrap_or_default();
            painter.text(
                egui::pos2(x + 3.0, top + 1.0),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional(8.5),
                theme::muted(ui),
            );
        }
    }
    // programme blocks
    let prog_top = grid_bottom + 2.0;
    for entry in epg
        .iter()
        .filter(|e| e.end > e.start && e.end > start && e.start < start + span)
    {
        let clipped_start = entry.start.max(start);
        let clipped_end = entry.end.min(start + span);
        let x1 = left + width * ((clipped_start - start) as f32 / span as f32);
        let x2 = left + width * ((clipped_end - start) as f32 / span as f32);
        let cell = egui::Rect::from_min_max(
            egui::pos2(x1 + 1.0, prog_top + 1.0),
            egui::pos2((x2 - 1.0).max(x1 + 1.0), bottom - 1.0),
        );
        let active = entry.start <= now && now < entry.end;
        painter.rect_filled(
            cell,
            4.0,
            if active {
                theme::selection_surface(ui)
            } else {
                theme::CARD_DARK
            },
        );
        painter.rect_stroke(
            cell,
            4.0,
            Stroke::new(
                if active { 1.0_f32 } else { 0.5_f32 },
                if active {
                    theme::accent(ui)
                } else {
                    theme::hairline(ui)
                },
            ),
        );
        painter.text(
            cell.left_center() + egui::vec2(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            entry.title.clone(),
            egui::FontId::proportional(9.5),
            if active {
                theme::accent_text(ui)
            } else {
                theme::muted(ui)
            },
        );
    }
    // now indicator line
    if now >= start && now <= start + span {
        let x = left + width * ((now - start) as f32 / span as f32);
        painter.line_segment(
            [egui::pos2(x, top), egui::pos2(x, bottom)],
            Stroke::new(1.5_f32, theme::LIVE),
        );
    }
}

fn epg_lines(epg: &[crate::types::EpgEntry], now: i64) -> (Option<String>, Option<String>) {
    let current = epg
        .iter()
        .find(|e| e.start <= now && now < e.end)
        .map(|e| e.title.clone());
    let next = epg
        .iter()
        .filter(|e| e.start > now)
        .min_by_key(|e| e.start)
        .map(|e| e.title.clone());
    (current, next)
}

/// Fraction (0..1) of the programme currently on air, for the row progress bar.
fn epg_progress(app: &App, id: u32, now: i64) -> Option<f32> {
    let epg = app.epg_for(id)?;
    let cur = epg.iter().find(|e| e.start <= now && now < e.end)?;
    let span = (cur.end - cur.start) as f32;
    if span <= 0.0 {
        return None;
    }
    Some(((now - cur.start) as f32 / span).clamp(0.0, 1.0))
}

fn header(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new("Live TV").size(22.0).strong(),
        ));
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
        ui.add(egui::Label::new(
            RichText::new("Vorschau").size(18.0).strong(),
        ));
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
                        RichText::new(format!("{} – {}", fmt_clock(cur.start), fmt_clock(cur.end)))
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
                } else if app.epg_failed(id) {
                    ui.add(egui::Label::new(
                        RichText::new("EPG konnte nicht geladen werden.")
                            .size(12.0)
                            .weak(),
                    ));
                    if let Some(detail) = app.epg_error.clone() {
                        ui.add(
                            egui::Label::new(RichText::new(detail).size(11.0).weak()).wrap(true),
                        );
                    }
                    if ui.button("Erneut versuchen").clicked() {
                        app.retry_epg(id);
                        ui.ctx().request_repaint();
                    }
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
