//! Full-window player.
//!
//! Layout (all positions are computed from the window rect, so it stays correct
//! on every screen size):
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │ ◀ Zurück                          Titel      │  top bar
//! ├──────────────────────────────────────────────┤
//! │                  video                       │
//! ├──────────────────────────────────────────────┤
//! │ 0:12 ──────●────────────────────── 45:10     │  progress
//! │  🔊 ────────        «10  ▶  »10        ⛶     │  transport (play = exact centre)
//! └──────────────────────────────────────────────┘
//! ```

use crate::app::App;
use crate::theme;
use crate::types::PlayKind;
use crate::ui;
use egui::{
    pos2, vec2, Align2, Color32, FontId, Rect, Rounding, Sense, Stroke, Ui,
};

const TOP_H: f32 = 56.0;
const BOT_H: f32 = 120.0;
/// Seconds of inactivity before the chrome fades out.
const HIDE_AFTER: f64 = 3.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let ppp = ctx.pixels_per_point();
    let full = ui.available_rect_before_wrap();
    ui.painter().rect_filled(full, 0.0, Color32::BLACK);

    let now = ctx.input(|i| i.time);
    if app.ui_activity < 0.0 {
        app.ui_activity = now;
    }
    // The native video window covers the middle, so egui never sees the mouse
    // there – poll the real cursor instead.
    if app.note_pointer_activity() {
        app.ui_activity = now;
    }
    let show_ui = now - app.ui_activity < HIDE_AFTER;

    let top_h = if show_ui { TOP_H } else { 0.0 };
    let bot_h = if show_ui { BOT_H } else { 0.0 };
    let video = Rect::from_min_max(
        pos2(full.left(), full.top() + top_h),
        pos2(full.right(), full.bottom() - bot_h),
    );
    app.position_video(video, ppp);

    if !show_ui {
        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
    }

    let (pos, dur, paused, cache) = app
        .player
        .as_ref()
        .map(|p| (p.position, p.duration, p.paused, p.mpv.cache_time()))
        .unwrap_or((0.0, 0.0, false, 0.0));
    let is_live = app
        .current_play
        .as_ref()
        .map(|p| p.kind == PlayKind::Live)
        .unwrap_or(false);
    let title = app
        .current_play
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_default();
    let sub = app
        .current_play
        .as_ref()
        .map(|p| match p.kind {
            PlayKind::Live => "Live-TV".to_string(),
            PlayKind::Vod => "Film".to_string(),
            PlayKind::Episode => "Serie".to_string(),
        })
        .unwrap_or_default();

    if let Some(e) = &app.player_error {
        ui.painter().text(
            video.center(),
            Align2::CENTER_CENTER,
            format!("Player-Fehler: {}", e),
            FontId::proportional(15.0),
            theme::DANGER,
        );
    }

    if !show_ui {
        return;
    }

    // ---------------- top bar ----------------
    let top = Rect::from_min_max(
        pos2(full.left(), full.top()),
        pos2(full.right(), full.top() + TOP_H),
    );
    glass(ui, top, 0.0, 0.0);

    let mut do_back = false;
    let back_rect = Rect::from_min_size(pos2(top.left() + 16.0, top.top() + 12.0), vec2(104.0, 32.0));
    if pill_button(ui, back_rect, "◀  Zurück").clicked() {
        do_back = true;
    }

    let right = pos2(top.right() - 20.0, top.center().y);
    ui.painter().text(
        pos2(right.x, right.y - 7.0),
        Align2::RIGHT_CENTER,
        &title,
        FontId::proportional(15.0),
        text_color(ui),
    );
    let sub_txt = sub;
    ui.painter().text(
        pos2(right.x, right.y + 11.0),
        Align2::RIGHT_CENTER,
        sub_txt,
        FontId::proportional(11.0),
        muted(ui),
    );

    // ---------------- bottom bar ----------------
    let bot = Rect::from_min_max(
        pos2(full.left(), full.bottom() - BOT_H),
        pos2(full.right(), full.bottom()),
    );
    glass(ui, bot, 0.0, 0.0);

    let mut seek_to: Option<f64> = None;
    let mut seek_rel: f64 = 0.0;
    let mut toggle = false;
    let mut new_vol: Option<i64> = None;
    let mut do_full = false;
    let mut step: i32 = 0;

    // ---- progress row ----
    let row_y = bot.top() + 30.0;
    if is_live {
        let badge = Rect::from_center_size(pos2(bot.left() + 58.0, row_y), vec2(74.0, 22.0));
        ui.painter()
            .rect_filled(badge, Rounding::same(11.0), theme::DANGER);
        ui.painter().text(
            badge.center(),
            Align2::CENTER_CENTER,
            "● LIVE",
            FontId::proportional(11.0),
            Color32::WHITE,
        );
        ui.painter().text(
            pos2(bot.right() - 22.0, row_y),
            Align2::RIGHT_CENTER,
            format!("Puffer {:.0}s", cache),
            FontId::proportional(11.0),
            muted(ui),
        );
    } else {
        let t_left = pos2(bot.left() + 24.0, row_y);
        let t_right = pos2(bot.right() - 24.0, row_y);
        ui.painter().text(
            t_left,
            Align2::LEFT_CENTER,
            ui::fmt_time(pos),
            FontId::proportional(12.0),
            text_color(ui),
        );
        ui.painter().text(
            t_right,
            Align2::RIGHT_CENTER,
            ui::fmt_time(dur),
            FontId::proportional(12.0),
            muted(ui),
        );

        let bar = Rect::from_min_max(
            pos2(bot.left() + 86.0, row_y - 9.0),
            pos2(bot.right() - 86.0, row_y + 9.0),
        );
        let frac = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) } else { 0.0 };
        let buffered = if dur > 0.0 {
            ((pos + cache) / dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if let Some(t) = drag_bar(ui, bar, frac, buffered, dur) {
            seek_to = Some(t);
        }
    }

    // ---- transport row: play/pause is the exact centre of the window ----
    let cy = bot.top() + 78.0;
    let cx = bot.center().x;

    let play_rect = Rect::from_center_size(pos2(cx, cy), vec2(58.0, 58.0));
    if circle_button(
        ui,
        play_rect,
        theme::ACCENT,
        theme::ACCENT.gamma_multiply(1.15),
        if paused { "▶" } else { "⏸" },
        24.0,
        Color32::WHITE,
    ) {
        toggle = true;
    }

    let side = vec2(58.0, 40.0);
    let back_rect = Rect::from_center_size(pos2(cx - 76.0, cy), side);
    let fwd_rect = Rect::from_center_size(pos2(cx + 76.0, cy), side);
    let (back_label, back_tip, fwd_label, fwd_tip) = if is_live {
        ("◀", "Vorheriger Kanal (←)", "▶", "Nächster Kanal (→)")
    } else {
        ("« 10", "10 Sekunden zurück (←)", "10 »", "10 Sekunden vor (→)")
    };
    if pill_button(ui, back_rect, back_label).on_hover_text(back_tip).clicked() {
        if is_live {
            step = -1;
        } else {
            seek_rel = -10.0;
        }
    }
    if pill_button(ui, fwd_rect, fwd_label).on_hover_text(fwd_tip).clicked() {
        if is_live {
            step = 1;
        } else {
            seek_rel = 10.0;
        }
    }

    // volume, left
    let vol_icon = Rect::from_center_size(pos2(bot.left() + 30.0, cy), vec2(24.0, 24.0));
    speaker_icon(ui, vol_icon, if app.volume == 0 { muted(ui) } else { text_color(ui) });
    let vbar = Rect::from_min_max(
        pos2(bot.left() + 50.0, cy - 9.0),
        pos2(bot.left() + 186.0, cy + 9.0),
    );
    let vfrac = (app.volume as f64 / 100.0).clamp(0.0, 1.0);
    if let Some(f) = drag_bar(ui, vbar, vfrac, 0.0, 1.0) {
        new_vol = Some((f * 100.0).round() as i64);
    }
    ui.painter().text(
        pos2(bot.left() + 196.0, cy),
        Align2::LEFT_CENTER,
        format!("{}", app.volume),
        FontId::proportional(11.0),
        muted(ui),
    );

    // fullscreen, right
    let fs_rect = Rect::from_center_size(pos2(bot.right() - 36.0, cy), vec2(38.0, 38.0));
    let fs_resp = ui.interact(
        fs_rect,
        ui.id().with(("fs", fs_rect.min.x as i32)),
        Sense::click(),
    );
    if fs_resp.hovered() {
        ui.painter()
            .rect_filled(fs_rect, Rounding::same(9.0), Color32::from_white_alpha(18));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    fullscreen_icon(ui, fs_rect.shrink(10.0), text_color(ui));
    if fs_resp.clicked() {
        do_full = true;
    }

    // ---- apply ----
    if do_back {
        app.stop_play();
        return;
    }
    if toggle {
        if let Some(pl) = &app.player {
            pl.mpv.toggle_pause();
        }
        app.ui_activity = now;
    }
    if seek_rel != 0.0 {
        if let Some(pl) = &app.player {
            pl.mpv.seek_relative(seek_rel);
        }
    }
    if let Some(t) = seek_to {
        if let Some(pl) = &app.player {
            pl.mpv.seek_absolute(t);
        }
    }
    if let Some(v) = new_vol {
        app.volume = v;
        if let Some(pl) = &app.player {
            pl.mpv.set_volume(v);
        }
        app.ui_activity = now;
    }
    if step != 0 {
        app.step_channel(step);
    }
    if do_full {
        app.fullscreen = !app.fullscreen;
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.fullscreen));
    }
    // Keep the timer alive while the pointer rests on the chrome.
    if ui.rect_contains_pointer(bot) || ui.rect_contains_pointer(top) {
        app.ui_activity = now;
    }
}

// ---------------------------------------------------------------- helpers

fn text_color(ui: &Ui) -> Color32 {
    ui.visuals().text_color()
}

fn muted(ui: &Ui) -> Color32 {
    ui.visuals().weak_text_color()
}

/// Frosted, semi-transparent bar.
fn glass(ui: &Ui, rect: Rect, top_round: f32, bot_round: f32) {
    let dark = ui.visuals().dark_mode;
    let fill = if dark {
        Color32::from_rgba_unmultiplied(12, 15, 21, 216)
    } else {
        Color32::from_rgba_unmultiplied(248, 250, 253, 224)
    };
    let edge = if dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 22)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 20)
    };
    let rounding = Rounding {
        nw: top_round,
        ne: top_round,
        sw: bot_round,
        se: bot_round,
    };
    ui.painter().rect_filled(rect, rounding, fill);
    ui.painter()
        .rect_stroke(rect, rounding, Stroke::new(1.0_f32, edge));
}

fn circle_button(
    ui: &mut Ui,
    rect: Rect,
    fill: Color32,
    hover: Color32,
    glyph: &str,
    glyph_size: f32,
    color: Color32,
) -> bool {
    let resp = ui.interact(
        rect,
        ui.id().with(("circ", rect.center().x as i32, rect.center().y as i32)),
        Sense::click(),
    );
    let f = if resp.hovered() { hover } else { fill };
    ui.painter()
        .circle_filled(rect.center(), rect.width().min(rect.height()) * 0.5, f);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(glyph_size),
        color,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn pill_button(ui: &mut Ui, rect: Rect, label: &str) -> egui::Response {
    let resp = ui.interact(
        rect,
        ui.id().with(("pill", rect.center().x as i32, rect.center().y as i32)),
        Sense::click(),
    );
    let dark = ui.visuals().dark_mode;
    let base = if dark {
        Color32::from_white_alpha(16)
    } else {
        Color32::from_black_alpha(12)
    };
    let hover = if dark {
        Color32::from_white_alpha(34)
    } else {
        Color32::from_black_alpha(26)
    };
    ui.painter().rect_filled(
        rect,
        Rounding::same(rect.height() * 0.5),
        if resp.hovered() { hover } else { base },
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        ui.visuals().text_color(),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Seek/volume bar with drag support. Returns the new fraction (0..1).
fn drag_bar(ui: &mut Ui, rect: Rect, frac: f64, buffered: f64, _span: f64) -> Option<f64> {
    let resp = ui.interact(
        rect.expand2(vec2(0.0, 6.0)),
        ui.id().with(("bar", rect.center().x as i32, rect.center().y as i32)),
        Sense::click_and_drag(),
    );
    let p = ui.painter();
    let dark = ui.visuals().dark_mode;
    let track = if dark {
        Color32::from_white_alpha(30)
    } else {
        Color32::from_black_alpha(26)
    };
    let buf = if dark {
        Color32::from_white_alpha(60)
    } else {
        Color32::from_black_alpha(48)
    };
    let r = rect.height() * 0.5;
    p.rect_filled(rect, Rounding::same(r), track);
    if buffered > frac {
        let w = rect.width() as f64 * buffered;
        p.rect_filled(
            Rect::from_min_size(rect.min, vec2(w as f32, rect.height())),
            Rounding::same(r),
            buf,
        );
    }
    let w = rect.width() as f64 * frac.clamp(0.0, 1.0);
    p.rect_filled(
        Rect::from_min_size(rect.min, vec2(w as f32, rect.height())),
        Rounding::same(r),
        theme::ACCENT,
    );
    let knob_x = rect.left() + w as f32;
    let radius = if resp.hovered() || resp.dragged() { 7.0 } else { 5.0 };
    p.circle_filled(
        pos2(knob_x.clamp(rect.left(), rect.right()), rect.center().y),
        radius,
        Color32::WHITE,
    );

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
            return Some(f);
        }
    }
    None
}

fn speaker_icon(ui: &Ui, rect: Rect, color: Color32) {
    let p = ui.painter();
    let cx = rect.center().x;
    let cy = rect.center().y;
    let s = rect.width() * 0.5;
    // body
    p.rect_filled(
        Rect::from_min_max(pos2(cx - s, cy - s * 0.35), pos2(cx - s * 0.35, cy + s * 0.35)),
        0.0,
        color,
    );
    p.add(egui::Shape::convex_polygon(
        vec![
            pos2(cx - s * 0.38, cy - s * 0.35),
            pos2(cx + s * 0.12, cy - s * 0.85),
            pos2(cx + s * 0.12, cy + s * 0.85),
            pos2(cx - s * 0.38, cy + s * 0.35),
        ],
        color,
        Stroke::NONE,
    ));
    // waves
    for (i, r) in [0.42_f32, 0.72].iter().enumerate() {
        let rad = s * r;
        let pts: Vec<egui::Pos2> = (0..10)
            .map(|k| {
                let a = -0.55 + 1.1 * (k as f32 / 9.0);
                pos2(cx + s * 0.16 + rad * a.cos(), cy + rad * a.sin())
            })
            .collect();
        p.add(egui::Shape::line(
            pts,
            Stroke::new(1.6 - i as f32 * 0.4, color),
        ));
    }
}

fn fullscreen_icon(ui: &Ui, rect: Rect, color: Color32) {
    let p = ui.painter();
    let s = rect.width().min(rect.height()) * 0.5;
    let c = rect.center();
    let l = 7.0;
    let st = Stroke::new(1.8_f32, color);
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let x = c.x + dx * s;
        let y = c.y + dy * s;
        p.line_segment([pos2(x, y), pos2(x - dx * l, y)], st);
        p.line_segment([pos2(x, y), pos2(x, y - dy * l)], st);
    }
}
