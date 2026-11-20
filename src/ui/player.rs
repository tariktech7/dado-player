//! Full-window player.
//!
//! Layout (all positions are computed from the window rect, so it stays correct
//! on every screen size):
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │ ◀ Zurück                          Titel      │  top bar  ┐ egui chrome,
//! │                  video                       │           │ sits IN FRONT
//! │ 0:12 ──────●────────────────────── 45:10     │  progress │ of the video
//! │  🔊 ────────        «10  ▶  »10        ⛶     │  transport┘ (region cutouts)
//! └──────────────────────────────────────────────┘
//! ```
//!
//! The native mpv child HWND keeps ONE fixed rect (the full player area) for
//! the whole playback. mpv only re-fits/scales the picture when that HWND is
//! resized, so player chrome must never change the video rect: top/bottom bars
//! and track popovers are expressed as holes in the child window's visible
//! region instead. That keeps the scaling rock-steady while the controls are
//! free to cover ("im Weg stehen") parts of the picture.

use crate::app::App;
use crate::theme;
use crate::types::{PlayKind, StreamInfo, Track};
use crate::ui;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Rounding, Sense, Stroke, Ui};

const TOP_H: f32 = 56.0;
const BOT_H: f32 = 120.0;
/// Seconds of inactivity before the chrome fades out.
const HIDE_AFTER: f64 = crate::release_info::CONTROLS_HIDE_SECONDS;

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
    // Touch/click counts as activity too. On Android there is no cursor at all,
    // so the desktop polling above never fires and the chrome would stay hidden
    // forever after the first auto-hide (verified in the emulator).
    if ctx.input(|i| i.pointer.any_pressed() || i.pointer.any_down()) {
        app.ui_activity = now;
    }
    // Touch devices have no cursor to poll, so `App::update()` records taps
    // and TV-remote keys in `ui_activity` before this screen is drawn. That is
    // what brings the chrome back; it must NOT be pinned on Android, which was
    // the "menu never disappears" bug.
    // QA hook DADO_KEEP_UI=1 pins the chrome (screenshots can't poll the
    // native video area for cursor activity, so the menu would auto-hide and
    // the cluster would never be captured).
    let keep_ui = std::env::var("DADO_KEEP_UI").is_ok();
    let idle = now - app.ui_activity;
    // The click/key that brings hidden chrome back must not press the control
    // that appears under it (that would hit "Zurück" and stop playback).
    // `App::update()` flags exactly that frame; it is spent on the picture.
    let reveal_only = std::mem::take(&mut app.player_reveal);
    let show_ui = (keep_ui || idle < HIDE_AFTER) && !reveal_only;
    app.player_chrome = show_ui;
    if !show_ui {
        app.player_menu_navigation = false;
    }
    if show_ui && !keep_ui {
        // Wake up exactly when the hide deadline expires, even if no further
        // input arrives (egui is otherwise completely idle while playing).
        let ms = ((HIDE_AFTER - idle) * 1000.0).ceil() as u64 + 16;
        ctx.request_repaint_after(std::time::Duration::from_millis(ms));
    }

    let is_live = app
        .current_play
        .as_ref()
        .map(|p| p.kind == PlayKind::Live)
        .unwrap_or(false);

    // A track popover needs more room than the normal 120px bottom bar.
    // `top_h` / `bot_h` now describe ONLY the egui chrome — they no longer
    // shrink the video window. mpv re-fits and re-scales the picture on every
    // HWND size change, which is exactly the "scaling jumps back and forth"
    // when the controls auto-hide, reappear or a menu opens. Keeping one
    // fixed video rect for the entire playback removes that entirely; the
    // chrome is revealed by punching holes into the child window's visible
    // region instead (see `set_video_cutouts` below).
    let menu_open = show_ui && !is_live && (app.audio_menu_open || app.sub_menu_open);
    // The bottom cutout grows by EXACTLY the height the open track popover
    // needs (computed the same way as `draw_track_popover`), not a flat 100px
    // extension. A fixed +100 left a dead strip of video behind a short
    // single-track popover, which read as a mysterious black bar. Sizing the
    // strip to the panel removes that dead space; when closed it is 0, so the
    // normal controls never move.
    let popover_h = if menu_open {
        track_popover_height(app)
    } else {
        0.0
    };
    let top_h = if show_ui { TOP_H } else { 0.0 };
    let bot_h = if show_ui { BOT_H + popover_h } else { 0.0 };
    // FIXED geometry: identical in every player state, and it only changes
    // when the window itself is resized. `position_video()` is still called
    // exactly once per frame (a second call would reintroduce the resize
    // storm that made the whole system "durchspacken").
    let video = full;
    let radio = app
        .current_play
        .as_ref()
        .map(|p| app.is_radio(&p.progress_key))
        .unwrap_or(false);
    // The desktop mpv child HWND sits above egui, so park it for audio-only
    // streams. Android uses the separate GL player module and keeps its render
    // callback alive so queued playback can initialize normally.
    #[cfg(not(target_os = "android"))]
    {
        app.position_video(
            if radio {
                Rect::from_min_size(pos2(-50.0, -50.0), vec2(1.0, 1.0))
            } else {
                video
            },
            ppp,
        );
        // egui draws the whole chrome on the parent surface; the child HWND
        // would cover it. Cut exactly those bands out of the HWND's visible
        // region so the controls stand in front of the (unrescaled) picture.
        // With the chrome hidden the slice is empty -> full-bleed video.
        // `set_video_cutouts` caches, so SetWindowRgn runs only on a real
        // state/window change, never every frame.
        let mut holes: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(2);
        if !radio && show_ui {
            let bands = [
                Rect::from_min_max(
                    pos2(full.left(), full.top()),
                    pos2(full.right(), full.top() + top_h),
                ),
                Rect::from_min_max(
                    pos2(full.left(), full.bottom() - bot_h),
                    pos2(full.right(), full.bottom()),
                ),
            ];
            for r in bands {
                holes.push(rect_to_hole(r, full.min, ppp));
            }
        }
        app.set_video_cutouts(&holes);
    }
    if radio {
        draw_radio_card(app, ui, video);
    }

    if !show_ui {
        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
    }

    let (pos, dur, paused, cache) = app
        .player
        .as_ref()
        .map(|p| (p.position, p.duration, p.paused, p.mpv.cache_time()))
        .unwrap_or((0.0, 0.0, false, 0.0));
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
        // Drop the track popovers together with the rest of the chrome.
        app.audio_menu_open = false;
        app.sub_menu_open = false;
        return;
    }

    // `bot_h` (and therefore the bottom cutout) was already grown above when
    // a track popover is open. Nothing to do here: `position_video()` was
    // called once, before this point, with the fixed rect, and the cutouts
    // already match this frame's chrome. A second call would resize the HWND
    // and re-introduce the per-frame repaint storm.

    // ---------------- top bar ----------------
    let top = Rect::from_min_max(
        pos2(full.left(), full.top()),
        pos2(full.right(), full.top() + TOP_H),
    );
    glass(ui, top, 0.0, 0.0);

    let mut do_back = false;
    let back_rect =
        Rect::from_min_size(pos2(top.left() + 16.0, top.top() + 12.0), vec2(104.0, 32.0));
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
        pos2(full.left(), full.bottom() - bot_h),
        pos2(full.right(), full.bottom()),
    );
    // Frost only the normal 120px bar. When a track popover is open the bar is
    // taller, but drawing glass over the extra strip turns half the picture
    // into a big opaque box. Leaving that strip unframed keeps the extra area
    // as plain black and the video jump barely noticeable.
    let glass_bar = Rect::from_min_max(pos2(full.left(), full.bottom() - BOT_H), bot.max);
    glass(ui, glass_bar, 0.0, 0.0);

    // Reserve the right edge for the stream-info / audio cluster so the
    // progress bar and transport never slide under it.
    let cluster_w = 276.0;
    let reserve = cluster_w + 20.0;
    let right_edge = bot.right() - reserve;

    let mut seek_to: Option<f64> = None;
    let mut seek_rel: f64 = 0.0;
    let mut toggle = false;
    let mut new_vol: Option<i64> = None;
    let mut do_full = false;
    let mut step: i32 = 0;

    // ---- progress row ----
    // Controls stay strictly anchored at the bottom (relative to BOT_H), so
    // opening the audio/subtitle popover never makes the transport jump.
    let ctrl_top = full.bottom() - BOT_H;
    let row_y = ctrl_top + 30.0;
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
            pos2(right_edge - 22.0, row_y),
            Align2::RIGHT_CENTER,
            format!("Puffer {:.0}s", cache),
            FontId::proportional(11.0),
            muted(ui),
        );
    } else {
        let t_left = pos2(bot.left() + 24.0, row_y);
        let t_right = pos2(right_edge - 24.0, row_y);
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
            pos2(right_edge - 86.0, row_y + 9.0),
        );
        let frac = if dur > 0.0 {
            (pos / dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let buffered = if dur > 0.0 {
            ((pos + cache) / dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if let Some(t) = drag_bar(ui, bar, frac, buffered, dur) {
            // `drag_bar` returns a 0..1 fraction — turn it into seconds.
            if dur > 0.0 {
                seek_to = Some(t * dur);
            }
        }
    }

    // ---- transport row: play/pause is the centre of the area left of the
    // cluster (so it never sits under the info/audio panel on the right). ----
    let cy = ctrl_top + 78.0;
    let cx = (bot.left() + right_edge) / 2.0;

    let play_rect = Rect::from_center_size(pos2(cx, cy), vec2(58.0, 58.0));
    if circle_button(
        ui,
        play_rect,
        theme::accent_fill(ui),
        theme::accent_fill(ui).gamma_multiply(1.15),
        if paused { "▶" } else { "⏸" },
        24.0,
        Color32::WHITE,
        // "⏸" (pause) renders visually above the circle's geometric center
        // under Align2::CENTER_CENTER; nudge it down so the bars sit centered.
        if paused { 0.0 } else { 3.0 },
    ) {
        toggle = true;
    }

    let side = vec2(58.0, 40.0);
    let back_rect = Rect::from_center_size(pos2(cx - 76.0, cy), side);
    let fwd_rect = Rect::from_center_size(pos2(cx + 76.0, cy), side);
    let (back_label, back_tip, fwd_label, fwd_tip) = if is_live {
        ("◀", "Vorheriger Kanal (←)", "▶", "Nächster Kanal (→)")
    } else {
        (
            "« 10",
            "10 Sekunden zurück (←)",
            "10 »",
            "10 Sekunden vor (→)",
        )
    };
    if pill_button(ui, back_rect, back_label)
        .on_hover_text(back_tip)
        .clicked()
    {
        if is_live {
            step = -1;
        } else {
            seek_rel = -10.0;
        }
    }
    if pill_button(ui, fwd_rect, fwd_label)
        .on_hover_text(fwd_tip)
        .clicked()
    {
        if is_live {
            step = 1;
        } else {
            seek_rel = 10.0;
        }
    }

    // volume, left
    let vol_icon = Rect::from_center_size(pos2(bot.left() + 30.0, cy), vec2(24.0, 24.0));
    speaker_icon(
        ui,
        vol_icon,
        if app.volume == 0 {
            muted(ui)
        } else {
            text_color(ui)
        },
    );
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

    // fullscreen, right (kept clear of the cluster)
    let fs_rect = Rect::from_center_size(pos2(right_edge - 36.0, cy), vec2(38.0, 38.0));
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

    // ---------------- stream-info / audio cluster (inside the bottom bar) ----
    // Resolution / fps / codecs for every kind; audio + subtitle selectors for
    // movies & series. Drawn *inside* the egui chrome (the bottom bar), never
    // over the picture — the native video window would otherwise paint over it.
    // Only shown while the auto-hiding menu is visible (we returned above when
    // `!show_ui`), so it never stays on screen.
    //
    // The glass box holds ONLY the stream info. The two track buttons sit
    // free-standing below it, and the popover is anchored above the whole box.
    // Previously the buttons were buried inside the glass panel and the popover
    // opened on top of the button row, covering the resolution/codec text.
    {
        let pl_ref = app.player.as_ref();
        let stream: StreamInfo = pl_ref.map(|pl| pl.stream.clone()).unwrap_or_default();
        let atracks: Vec<Track> = pl_ref.map(|pl| pl.audio_tracks.clone()).unwrap_or_default();
        let stracks: Vec<Track> = pl_ref.map(|pl| pl.sub_tracks.clone()).unwrap_or_default();
        let cur_a = pl_ref.map(|pl| pl.current_audio).unwrap_or(0);
        let cur_s = pl_ref.map(|pl| pl.current_sub).unwrap_or(0);

        let pad = 16.0;
        let line_h = 16.0;
        let info_h = line_h * 2.0 + 12.0; // two text lines + vertical padding
        let is_vod = !is_live;
        let has_tracks = atracks.len() > 1 || !stracks.is_empty();
        let btns_h = if has_tracks { 32.0 } else { 0.0 };
        let gap = if has_tracks { 8.0 } else { 0.0 };
        let panel_h = info_h;
        let panel_w = 276.0;
        let panel = Rect::from_min_size(
            pos2(
                bot.right() - pad - panel_w,
                bot.bottom() - pad - btns_h - gap - panel_h,
            ),
            vec2(panel_w, panel_h),
        );
        glass(ui, panel, 10.0, 10.0);

        // Inner content rect: 12 px horizontal padding + ~10 px right margin,
        // so long codec / container strings get clipped inside the panel
        // instead of spilling over the glass frame or off-screen.
        let inner = Rect::from_min_max(
            pos2(panel.left() + 12.0, panel.top() + 8.0),
            pos2(panel.right() - 10.0, panel.bottom() - 8.0),
        );
        // The two info lines are vertically centered as a pair inside the
        // (info-only) glass panel, so it reads as one balanced block.
        let info_area_h = panel_h;
        let block_h = line_h * 2.0;
        let info_top = panel.top() + (info_area_h - block_h) * 0.5;
        let text_clip = Rect::from_min_max(
            pos2(inner.left(), info_top),
            pos2(inner.right(), info_top + block_h),
        );
        let clip = ui.painter().with_clip_rect(text_clip);

        // Line 1: resolution + fps
        let mut res = String::new();
        if stream.width > 0 && stream.height > 0 {
            res = format!("{}×{}", stream.width, stream.height);
            if stream.fps > 0.0 {
                res = format!("{}  ·  {:.0} fps", res, stream.fps);
            }
        }
        clip.text(
            pos2(inner.left(), info_top + line_h * 0.5),
            Align2::LEFT_CENTER,
            if res.is_empty() { "…" } else { &res },
            FontId::proportional(13.0),
            text_color(ui),
        );
        // Line 2: codecs (+ container for movies / series)
        let mut codec = String::new();
        if !stream.video_codec.is_empty() {
            codec.push_str(&clean_codec(&stream.video_codec));
        }
        if !stream.audio_codec.is_empty() {
            if !codec.is_empty() {
                codec.push_str("  ·  ");
            }
            codec.push_str(&clean_codec(&stream.audio_codec));
        }
        if is_vod && !stream.container.is_empty() {
            if !codec.is_empty() {
                codec.push_str("  ·  ");
            }
            codec.push_str(&clean_codec(&stream.container));
        }
        clip.text(
            pos2(inner.left(), info_top + line_h * 1.5),
            Align2::LEFT_CENTER,
            if codec.is_empty() { "…" } else { &codec },
            FontId::proportional(11.0),
            muted(ui),
        );

        // Audio / subtitle buttons (movies & series only). They sit BELOW the
        // glass info panel, as free-standing pills on the bottom bar.
        if has_tracks {
            let by = panel.bottom() + gap;
            let inner_w = panel_w - 24.0;
            let btn_w = (inner_w - 8.0) / 2.0;
            let a_rect = Rect::from_min_size(pos2(panel.left() + 12.0, by), vec2(btn_w, btns_h));
            let s_rect = Rect::from_min_size(
                pos2(panel.left() + 12.0 + btn_w + 8.0, by),
                vec2(btn_w, btns_h),
            );
            if pill_button(ui, a_rect, "🔊 Audio").clicked() {
                app.audio_menu_open = !app.audio_menu_open;
                app.sub_menu_open = false;
            }
            if pill_button(ui, s_rect, "CC Untertitel").clicked() {
                app.sub_menu_open = !app.sub_menu_open;
                app.audio_menu_open = false;
            }

            // Anchor above the WHOLE info panel, never above a button inside
            // it: otherwise the popover covers the resolution/codec text.
            //
            // Only draw popovers on frames whose `bot_h` already includes the
            // popover extension. On the exact frame a button is clicked,
            // `menu_open` was still false at the top of `show()`, so the
            // extended bar does not exist yet and the popover would be clamped
            // to a ~10px sliver for one frame. Skipping that single frame
            // avoids the flash; the next frame has the full geometry.
            if menu_open {
                let ph = track_popover_height(app);
                if let Some(p) = draw_track_popover(
                    ui,
                    bot,
                    panel,
                    &atracks,
                    cur_a,
                    false,
                    ph,
                    &mut app.audio_menu_open,
                ) {
                    match p {
                        TrackPick::Select(id) => {
                            if let Some(pl) = app.player.as_ref() {
                                pl.mpv.set_audio_track(id);
                            }
                        }
                        TrackPick::Off => {}
                    }
                    app.audio_menu_open = false;
                }
                if let Some(p) = draw_track_popover(
                    ui,
                    bot,
                    panel,
                    &stracks,
                    cur_s,
                    true,
                    ph,
                    &mut app.sub_menu_open,
                ) {
                    match p {
                        TrackPick::Select(id) => {
                            if let Some(pl) = app.player.as_ref() {
                                pl.mpv.set_sub_track(id);
                            }
                        }
                        TrackPick::Off => {
                            if let Some(pl) = app.player.as_ref() {
                                pl.mpv.disable_sub();
                            }
                        }
                    }
                    app.sub_menu_open = false;
                }
            }
        }
    }

    // Keep the timer alive while the pointer rests on the chrome or a menu is open.
    if ui.rect_contains_pointer(bot) || ui.rect_contains_pointer(top) || menu_open {
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

/// Convert an egui chrome rect (logical points) into a physical-pixel hole
/// relative to the native video HWND's client origin. `SetWindowRgn` works in
/// window-relative device pixels, while every rect above is in egui points
/// relative to the parent client origin.
fn rect_to_hole(r: Rect, origin: egui::Pos2, ppp: f32) -> (i32, i32, i32, i32) {
    (
        ((r.min.x - origin.x) * ppp).round() as i32,
        ((r.min.y - origin.y) * ppp).round() as i32,
        (r.width() * ppp).round() as i32,
        (r.height() * ppp).round() as i32,
    )
}

/// Height of the open track popover, mirroring `draw_track_popover` exactly so
/// the bottom cutout reveals precisely the panel and nothing else. The two
/// selectors open one at a time, so the open one's track count governs this.
fn track_popover_height(app: &App) -> f32 {
    const ROW_H: f32 = 28.0;
    const HEADER_H: f32 = 30.0;
    const MAX_H: f32 = 88.0;
    const MIN_H: f32 = HEADER_H + ROW_H + 14.0;
    let pl = app.player.as_ref();
    let (a_rows, s_rows) = match pl {
        Some(pl) => {
            let a = pl.audio_tracks.len();
            let s = pl.sub_tracks.len() + 1; // subtitle list always has the "Aus" row
            (a, s)
        }
        None => (0, 1),
    };
    let rows = if app.audio_menu_open { a_rows } else { s_rows };
    let natural = HEADER_H + (rows as f32) * ROW_H + 12.0;
    natural.clamp(MIN_H, MAX_H) + 8.0
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

// Drawing helpers take explicit geometry/colour arguments rather than a
// context struct: they are called once per frame and must stay allocation-free.
#[allow(clippy::too_many_arguments)]
fn circle_button(
    ui: &mut Ui,
    rect: Rect,
    fill: Color32,
    hover: Color32,
    glyph: &str,
    glyph_size: f32,
    color: Color32,
    // Manual vertical offset (in points, positive = down) applied to the glyph
    // so that glyphs like "⏸" (pause) sit visually centered in the circle
    // rather than shifted up by egui's glyph-bounding-box centering.
    glyph_offset_y: f32,
) -> bool {
    let resp = ui.interact(
        rect,
        ui.id()
            .with(("circ", rect.center().x as i32, rect.center().y as i32)),
        Sense::click(),
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            Rounding::same(16.0),
            Stroke::new(2.0_f32, theme::accent_fill(ui)),
        );
    }
    let f = if resp.hovered() { hover } else { fill };
    ui.painter()
        .circle_filled(rect.center(), rect.width().min(rect.height()) * 0.5, f);
    ui.painter().text(
        pos2(rect.center().x, rect.center().y + glyph_offset_y),
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
        ui.id()
            .with(("pill", rect.center().x as i32, rect.center().y as i32)),
        Sense::click(),
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            Rounding::same(16.0),
            Stroke::new(2.0_f32, theme::accent_fill(ui)),
        );
    }
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
        ui.id()
            .with(("bar", rect.center().x as i32, rect.center().y as i32)),
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
        theme::accent(ui),
    );
    let knob_x = rect.left() + w as f32;
    let radius = if resp.hovered() || resp.dragged() {
        7.0
    } else {
        5.0
    };
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
        Rect::from_min_max(
            pos2(cx - s, cy - s * 0.35),
            pos2(cx - s * 0.35, cy + s * 0.35),
        ),
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

// ---------------------------------------------------------- track popovers

/// Result of a click inside the track selector popover.
enum TrackPick {
    /// Select the audio/subtitle track with this mpv id.
    Select(i64),
    /// Turn subtitles off (subtitle popover only).
    Off,
}

/// Draw the audio/subtitle track selector anchored above `anchor`. Returns the
/// chosen action (or `None` when nothing was picked / the menu is closed).
// All geometry is passed in from `position_video`, which must own the single
// authoritative video rectangle; bundling it would hide that invariant.
#[allow(clippy::too_many_arguments)]
fn draw_track_popover(
    ui: &mut Ui,
    bounds: Rect,
    anchor: Rect,
    tracks: &[Track],
    current: i64,
    for_sub: bool,
    content_h: f32,
    open: &mut bool,
) -> Option<TrackPick> {
    if !*open {
        return None;
    }
    let row_h = 28.0;
    let header_h = 30.0;
    let pop_w = 260.0;
    // `content_h` is passed in from `track_popover_height` (the SAME value the
    // bottom cutout reserves), so the panel and the revealed strip always
    // match - there is never a dead strip of video above the popover.
    let mut pop = Rect::from_min_size(
        pos2(anchor.right() - pop_w, anchor.top() - 8.0 - content_h),
        vec2(pop_w, content_h),
    );
    // Keep the popover inside the bounds
    let m = 6.0;
    if pop.left() < bounds.left() + m {
        pop = pop.translate(vec2((bounds.left() + m) - pop.left(), 0.0));
    }
    if pop.top() < bounds.top() + m {
        pop = pop.translate(vec2(0.0, (bounds.top() + m) - pop.top()));
    }
    if pop.right() > bounds.right() - m {
        pop = pop.translate(vec2((bounds.right() - m) - pop.right(), 0.0));
    }
    if pop.bottom() > anchor.top() - 4.0 {
        pop = pop.translate(vec2(0.0, (anchor.top() - 4.0) - pop.bottom()));
    }

    glass(ui, pop, 10.0, 10.0);

    // Header
    ui.painter().text(
        pos2(pop.left() + 14.0, pop.top() + header_h / 2.0),
        Align2::LEFT_CENTER,
        if for_sub {
            "Untertitel-Spuren"
        } else {
            "Audiospuren"
        },
        FontId::proportional(12.5),
        text_color(ui),
    );

    // Close button "×" top-right
    let close_rect = Rect::from_center_size(
        pos2(pop.right() - 16.0, pop.top() + header_h / 2.0),
        vec2(22.0, 22.0),
    );
    let close_resp = ui.interact(
        close_rect,
        ui.id().with(("pop_close", for_sub)),
        Sense::click(),
    );
    if close_resp.hovered() {
        ui.painter()
            .circle_filled(close_rect.center(), 10.0, Color32::from_white_alpha(30));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().text(
        close_rect.center(),
        Align2::CENTER_CENTER,
        "×",
        FontId::proportional(14.0),
        muted(ui),
    );
    if close_resp.clicked() {
        *open = false;
        return None;
    }

    // Divider
    ui.painter().line_segment(
        [
            pos2(pop.left() + 6.0, pop.top() + header_h),
            pos2(pop.right() - 6.0, pop.top() + header_h),
        ],
        Stroke::new(1.0_f32, Color32::from_white_alpha(20)),
    );

    let list_rect = Rect::from_min_max(
        pos2(pop.left(), pop.top() + header_h + 2.0),
        pos2(pop.right(), pop.bottom() - 4.0),
    );

    let mut pick: Option<TrackPick> = None;
    // Stable ids: the old id was derived from `rect.min.y`, which shifts with
    // the scroll offset, so egui saw "new" widgets every frame while scrolling
    // and the hover/click state flickered. Use a fixed index per row instead.
    let row_id_base = ui.id().with(("track_row", for_sub));
    let mut child_ui = ui.child_ui(list_rect, egui::Layout::top_down(egui::Align::Min));
    egui::ScrollArea::vertical()
        .id_source(("track_scroll", for_sub))
        .max_height(list_rect.height())
        .auto_shrink([false, false])
        .show(&mut child_ui, |ui| {
            let row = |ui: &mut Ui, idx: usize, label: &str, selected: bool| -> bool {
                let r = Rect::from_min_size(
                    pos2(ui.cursor().left() + 6.0, ui.cursor().top()),
                    vec2(pop_w - 12.0, row_h),
                );
                let hit = draw_track_row(ui, r, label, selected, row_id_base.with(idx));
                ui.add_space(row_h);
                hit
            };
            let mut idx = 0usize;
            if for_sub {
                if row(ui, idx, "Aus (Keine Untertitel)", current == 0) {
                    pick = Some(TrackPick::Off);
                }
                idx += 1;
            }
            if tracks.is_empty() {
                let r = Rect::from_min_size(
                    pos2(ui.cursor().left() + 6.0, ui.cursor().top()),
                    vec2(pop_w - 12.0, row_h),
                );
                ui.painter().text(
                    r.min + vec2(10.0, r.height() * 0.5),
                    Align2::LEFT_CENTER,
                    if for_sub {
                        "Keine Untertitel verfügbar"
                    } else {
                        "Keine Spuren verfügbar"
                    },
                    FontId::proportional(12.0),
                    muted(ui),
                );
                ui.add_space(row_h);
            } else {
                for t in tracks {
                    let id = t.id;
                    if row(ui, idx, &t.label(), id == current) {
                        pick = Some(TrackPick::Select(id));
                    }
                    idx += 1;
                }
            }
        });

    if pick.is_some() {
        *open = false;
    }
    pick
}

/// A single selectable row inside a track popover. Returns `true` on click.
fn draw_track_row(ui: &mut Ui, rect: Rect, label: &str, selected: bool, id: egui::Id) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let dark = ui.visuals().dark_mode;
    let bg = if selected {
        theme::selection_surface(ui)
    } else if resp.hovered() {
        if dark {
            Color32::from_white_alpha(26)
        } else {
            Color32::from_black_alpha(18)
        }
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(6.0), bg);
    let fg = if selected {
        theme::accent_text(ui)
    } else {
        text_color(ui)
    };
    ui.painter().text(
        rect.min + vec2(10.0, rect.height() * 0.5),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        fg,
    );
    if selected {
        ui.painter().text(
            pos2(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "✓",
            FontId::proportional(12.0),
            theme::accent(ui),
        );
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// mpv reports lower-case fourcc-ish codec strings; show them upper-case.
fn clean_codec(c: &str) -> String {
    c.to_uppercase()
}

/// Audio-only (M3U `radio="true"`) playback surface. The native video window
/// is parked off-screen, so we paint a dedicated "now playing" card here.
fn draw_radio_card(app: &App, ui: &Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, theme::BG_DARK);
    let cx = rect.center().x;
    let cy = rect.center().y - 28.0;
    let r = 52.0;
    ui.painter()
        .circle_filled(egui::pos2(cx, cy), r, theme::accent_fill(ui));
    ui.painter().circle_stroke(
        egui::pos2(cx, cy),
        r,
        Stroke::new(2.0_f32, theme::accent(ui)),
    );
    // Speaker glyph keeps the card readable without an image dependency.
    ui.painter().text(
        egui::pos2(cx, cy),
        Align2::CENTER_CENTER,
        "📻",
        FontId::proportional(38.0),
        Color32::WHITE,
    );
    let title = app
        .current_play
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_default();
    ui.painter().text(
        egui::pos2(cx, cy + r + 24.0),
        Align2::CENTER_CENTER,
        &title,
        FontId::proportional(20.0),
        theme::TEXT_DARK,
    );
    ui.painter().text(
        egui::pos2(cx, cy + r + 52.0),
        Align2::CENTER_CENTER,
        "Radio  ·  nur Audio",
        FontId::proportional(12.0),
        theme::MUTED_DARK,
    );
}
