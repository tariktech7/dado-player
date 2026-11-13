//! Phone channel list: category chips + virtualised 64 dp rows.

use egui::{pos2, vec2, Align2, Color32, Context, Rect, Rounding, Sense, Ui};

use super::components::{self, Icon, Row};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::theme;
use crate::types::{PlayItem, PlayKind};

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    if !app.library.loaded {
        skeleton(ui, l);
        return;
    }

    // ---- category chips ----
    let mut labels: Vec<(String, bool)> = Vec::with_capacity(app.library.live_categories.len() + 1);
    labels.push(("Alle".to_string(), app.live_cat == "all"));
    for c in &app.library.live_categories {
        labels.push((c.name.clone(), app.live_cat == c.id));
    }
    if let Some(i) = components::chip_row(ui, &labels, l.pad) {
        app.live_cat = if i == 0 {
            "all".to_string()
        } else {
            app.library.live_categories[i - 1].id.clone()
        };
        app.live_page = 0;
    }
    ui.add_space(6.0);

    // ---- filter ----
    let cat = if app.live_cat == "all" {
        None
    } else {
        Some(app.live_cat.clone())
    };
    let q = app.live_q.trim().to_lowercase();
    let key = ui.id().with((
        "channels",
        app.catalog_generation(),
        &cat,
        &q,
        app.library.channels.len(),
    ));
    let idx = ui
        .data(|d| d.get_temp::<std::sync::Arc<Vec<usize>>>(key))
        .unwrap_or_else(|| {
            let filtered: Vec<_> = app
                .library
                .channels
                .iter()
                .enumerate()
                .filter(|(_, c)| cat.as_ref().is_none_or(|id| &c.category_id == id))
                .filter(|(_, c)| q.is_empty() || c.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect();
            let indices = std::sync::Arc::new(filtered);
            ui.data_mut(|d| d.insert_temp(key, indices.clone()));
            indices
        });
    let last = ui.id().with("active-channel-order");
    if ui.data(|d| d.get_temp::<egui::Id>(last)) != Some(key) {
        app.live_order = idx.iter().map(|&i| app.library.channels[i].id).collect();
        ui.data_mut(|d| d.insert_temp(last, key));
    }

    if idx.is_empty() {
        components::empty_state(
            ui,
            Icon::Tv,
            "Keine Sender",
            "Für diese Kategorie gibt es keine Kanäle.",
            None,
        );
        return;
    }

    ui.horizontal(|ui| {
        ui.add_space(l.pad);
        ui.label(
            egui::RichText::new(format!("{} Sender", idx.len()))
                .font(reg(FS_CAP))
                .color(muted(ui)),
        );
    });
    ui.add_space(2.0);

    let total = idx.len();
    let now = chrono::Utc::now().timestamp();

    // `show_rows` sets the content height to `rows × ROW_H` itself and derives
    // the drawn range from it, so there is no content to append padding to.
    // Reserving it from the *viewport* is equivalent: the scroll extent ends
    // `pad` above the navigation bar either way, so the last row keeps the page
    // margin instead of touching the bar.
    let list_h = (ui.available_height() - l.pad).max(0.0);
    let mut play: Option<PlayItem> = None;
    let mut open_epg: Option<u32> = None;
    egui::ScrollArea::vertical()
        .max_height(list_h)
        .auto_shrink([false, false])
        .show_rows(ui, ROW_H, total, |ui, range| {
            for i in range {
                let ch = app.library.channels[idx[i]].clone();
                app.request_epg(ch.id);
                let cur = app.epg_for(ch.id).and_then(|v| {
                    v.iter()
                        .find(|e| e.start <= now && e.end > now)
                        .map(|e| (e.title.clone(), e.start, e.end))
                });
                let sub = match &cur {
                    Some((title, _, _)) => format!("Jetzt: {}", title),
                    None => format!("Kanal {}", ch.num),
                };
                // MOBILE-BAUPLAN §23.6 C: how far into the running programme we
                // are. EPG times are whole seconds; a zero/negative span would
                // divide by zero, so malformed entries simply get no bar.
                let prog = cur.as_ref().and_then(|(_, start, end)| {
                    let span = (end - start) as f32;
                    if span > 0.0 {
                        Some(((now - start) as f32 / span).clamp(0.0, 1.0))
                    } else {
                        None
                    }
                });
                let key = format!("live:{}", ch.id);
                let playing = app
                    .current_play
                    .as_ref()
                    .map(|p| p.progress_key == key)
                    .unwrap_or(false);
                let img_key = format!("logo:{}", ch.id);
                let row = Row::new(&ch.name)
                    .img(&img_key, &ch.logo)
                    .sub(&sub)
                    .playing(playing);
                let row = match prog {
                    Some(f) => row.progress(f),
                    None => row,
                };
                let r = components::list_row(app, ui, row, l.pad);
                if focus::clicked(ui, &r) {
                    open_epg = Some(ch.id);
                }
                let play_rect = Rect::from_center_size(
                    pos2(r.rect.right() - 32.0, r.rect.center().y),
                    vec2(44.0, 44.0),
                );
                let play_resp = ui.interact(
                    play_rect,
                    ui.id().with(("live-play", ch.id)),
                    Sense::click(),
                );
                let dark = ui.visuals().dark_mode;
                ui.painter().rect_filled(
                    play_rect,
                    Rounding::same(22.0),
                    if dark {
                        Color32::from_white_alpha(12)
                    } else {
                        Color32::from_black_alpha(12)
                    },
                );
                ui.painter().text(
                    play_rect.center(),
                    Align2::CENTER_CENTER,
                    "▶",
                    reg(FS_CAP),
                    theme::accent(ui),
                );
                if focus::clicked(ui, &play_resp) {
                    play = Some(PlayItem {
                        title: ch.name.clone(),
                        url: ch.stream_url.clone(),
                        kind: PlayKind::Live,
                        progress_key: key.clone(),
                    });
                }
            }
        });

    if let Some(ch_id) = open_epg {
        app.request_epg(ch_id);
        app.mobile_epg_channel = Some(ch_id);
    }
    if let Some(item) = play {
        app.play(item);
    }
}

/// Placeholder rows while the provider library loads.
fn skeleton(ui: &mut Ui, l: &Layout) {
    ui.add_space(10.0);
    for _ in 0..3 {
        ui.horizontal(|ui| {
            ui.add_space(l.pad);
            for _ in 0..2 {
                let (rect, _) = ui.allocate_exact_size(vec2(84.0, 36.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, Rounding::same(18.0), ui.visuals().faint_bg_color);
                ui.add_space(8.0);
            }
        });
    }
    ui.add_space(12.0);
    for _ in 0..8 {
        let (rect, _) = ui.allocate_exact_size(
            vec2(ui.available_width() - 2.0 * l.pad, ROW_H),
            egui::Sense::hover(),
        );
        let rect = rect.translate(vec2(l.pad, 0.0));
        let img =
            Rect::from_center_size(pos2(rect.left() + 30.0, rect.center().y), vec2(44.0, 44.0));
        ui.painter()
            .rect_filled(img, Rounding::same(10.0), ui.visuals().faint_bg_color);
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + 62.0, rect.center().y - 12.0),
                vec2((rect.width() * 0.45).min(180.0), 12.0),
            ),
            Rounding::same(6.0),
            ui.visuals().faint_bg_color,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + 62.0, rect.center().y + 6.0),
                vec2((rect.width() * 0.30).min(120.0), 9.0),
            ),
            Rounding::same(5.0),
            Color32::from_white_alpha(6),
        );
    }
}

/// Expandable EPG sheet for one channel (IPTVnator parity). Tapping a channel
/// row opens this overlay instead of jumping straight into playback, so the
/// virtualised Live list above is never disturbed. The sheet lists the channel's
/// programme schedule (now highlighted) and a "Jetzt schauen" button.
pub fn epg_sheet(app: &mut App, ctx: &Context, l: &Layout) {
    let ch_id = match app.mobile_epg_channel {
        Some(id) => id,
        None => return,
    };
    let ch = match app.library.channels.iter().find(|c| c.id == ch_id) {
        Some(c) => c.clone(),
        None => {
            app.mobile_epg_channel = None;
            return;
        }
    };
    app.request_epg(ch.id);
    let now = chrono::Utc::now().timestamp();

    focus::begin_modal(0);
    let screen = ctx.screen_rect();
    let row_h = 56.0;
    let header_h = 96.0;
    let mut epg = app.epg_for(ch.id).cloned().unwrap_or_default();
    epg.sort_by_key(|entry| entry.start);
    let max_sheet_h = (screen.height() * 0.85).max(header_h + row_h + l.bottom_inset + 24.0);
    let list_h = (max_sheet_h - header_h - l.bottom_inset - 24.0).max(row_h);
    let h = (header_h + (epg.len() as f32 * row_h).min(list_h) + l.bottom_inset + 24.0)
        .min(screen.height() - l.top_inset);

    egui::Area::new(egui::Id::new("epg-sheet"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            let (bg, bg_resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(bg, 0.0, Color32::from_black_alpha(150));

            let sheet = Rect::from_min_size(
                pos2(screen.left(), screen.bottom() - h),
                vec2(screen.width(), h),
            );
            ui.painter().rect_filled(
                sheet,
                Rounding {
                    nw: 22.0,
                    ne: 22.0,
                    sw: 0.0,
                    se: 0.0,
                },
                bar(ui),
            );
            // drag handle
            let handle =
                Rect::from_center_size(pos2(sheet.center().x, sheet.top() + 12.0), vec2(36.0, 4.0));
            ui.painter()
                .rect_filled(handle, Rounding::same(2.0), muted(ui));

            // ---- header: logo + name + play ----
            let logo_key = format!("logo:{}", ch.id);
            let logo_r = Rect::from_center_size(
                pos2(sheet.left() + 36.0, sheet.top() + 42.0),
                vec2(48.0, 48.0),
            );
            if !ch.logo.is_empty() {
                app.request_image(logo_key.clone(), ch.logo.clone());
                if let Some(tex) = app.image(&logo_key) {
                    ui.painter().image(
                        tex.id(),
                        logo_r,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().circle_filled(
                        logo_r.center(),
                        24.0,
                        Color32::from_white_alpha(20),
                    );
                }
            }
            ui.painter().text(
                pos2(sheet.left() + 72.0, sheet.top() + 32.0),
                Align2::LEFT_CENTER,
                &ch.name,
                reg(FS_TITLE),
                text(ui),
            );
            ui.painter().text(
                pos2(sheet.left() + 72.0, sheet.top() + 56.0),
                Align2::LEFT_CENTER,
                "Programmführer",
                reg(FS_CAP),
                muted(ui),
            );
            let play_rect = Rect::from_min_size(
                pos2(sheet.right() - 158.0, sheet.top() + 24.0),
                vec2(142.0, 40.0),
            );
            let play_resp = ui.interact(play_rect, ui.id().with("epg-play"), Sense::click());
            if play_resp.is_pointer_button_down_on() {
                ui.painter()
                    .rect_filled(play_rect, Rounding::same(10.0), pressed(ui));
            }
            ui.painter()
                .rect_filled(play_rect, Rounding::same(10.0), theme::ACCENT_FILL);
            ui.painter().text(
                play_rect.center(),
                Align2::CENTER_CENTER,
                "▶  Jetzt schauen",
                reg(FS_BODY),
                Color32::WHITE,
            );
            if focus::clicked(ui, &play_resp) {
                app.mobile_epg_channel = None;
                app.play(PlayItem {
                    title: ch.name.clone(),
                    url: ch.stream_url.clone(),
                    kind: PlayKind::Live,
                    progress_key: format!("live:{}", ch.id),
                });
            }

            // ---- programme list ----
            let list_top = sheet.top() + header_h;
            let list_rect = Rect::from_min_max(
                pos2(sheet.left() + 8.0, list_top),
                pos2(sheet.right() - 8.0, sheet.bottom() - l.bottom_inset - 12.0),
            );
            let mut selected_program = false;
            let epg_scroll = egui::ScrollArea::vertical()
                .id_source(("mobile-epg-list", ch.id))
                .max_height(list_rect.height().max(0.0))
                .auto_shrink([false, false]);
            if epg.is_empty() {
                if app.epg_failed(ch.id) {
                    let retry = components::empty_state(
                        ui,
                        Icon::Tv,
                        "Programm nicht verfügbar",
                        app.epg_error
                            .as_deref()
                            .unwrap_or("Der Anbieter hat nicht geantwortet."),
                        Some("Erneut versuchen"),
                    );
                    if retry {
                        app.retry_epg(ch.id);
                        ui.ctx().request_repaint();
                    }
                } else {
                    let message = if app.epg_pending(ch.id) {
                        "Programm wird geladen…"
                    } else {
                        "Keine Programmdaten verfügbar"
                    };
                    ui.painter().text(
                        list_rect.center(),
                        Align2::CENTER_CENTER,
                        message,
                        reg(FS_BODY),
                        muted(ui),
                    );
                }
            } else {
                epg_scroll.show(ui, |ui| {
                    ui.set_min_width(list_rect.width());
                    ui.set_clip_rect(list_rect);
                    ui.add_space(2.0);
                    for e in &epg {
                        let on_now = e.start <= now && e.end > now;
                        let row = ui.allocate_ui_with_layout(
                            vec2(list_rect.width(), row_h),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                let r = ui.max_rect();
                                if on_now {
                                    ui.painter().rect_filled(
                                        r.shrink(1.0),
                                        RADIUS_SM,
                                        theme::ACCENT_DIM,
                                    );
                                }
                                let t = format!("{}–{}", fmt_hhmm(e.start), fmt_hhmm(e.end));
                                ui.add_sized(
                                    vec2(84.0, row_h),
                                    egui::Label::new(
                                        egui::RichText::new(t).font(reg(FS_CAP)).color(muted(ui)),
                                    ),
                                );
                                ui.add_sized(
                                    vec2((r.width() - 120.0).max(60.0), row_h),
                                    egui::Label::new(
                                        egui::RichText::new(&e.title).font(reg(FS_BODY)).color(
                                            if on_now { theme::TEXT_DARK } else { text(ui) },
                                        ),
                                    )
                                    .truncate(true),
                                );
                                if on_now {
                                    ui.label(
                                        egui::RichText::new("JETZT")
                                            .font(reg(FS_CAP))
                                            .color(theme::LIVE),
                                    );
                                }
                                let resp = ui.interact(
                                    r,
                                    ui.id().with(("epg-program", ch.id, e.start)),
                                    Sense::click(),
                                );
                                if focus::clicked(ui, &resp) {
                                    selected_program = true;
                                }
                            },
                        );
                        let _ = row;
                    }
                });
            }
            if selected_program {
                app.mobile_epg_channel = None;
                app.play(PlayItem {
                    title: ch.name.clone(),
                    url: ch.stream_url.clone(),
                    kind: PlayKind::Live,
                    progress_key: format!("live:{}", ch.id),
                });
            }

            // close on scrim tap above the sheet
            if bg_resp.clicked() {
                if let Some(p) = bg_resp.interact_pointer_pos() {
                    if p.y < sheet.top() {
                        app.mobile_epg_channel = None;
                    }
                }
            }
        });
}

/// Format an EPG timestamp as `HH:MM` in UTC (EPG times are UTC).
fn fmt_hhmm(ts: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default()
}
