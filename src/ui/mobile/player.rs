//! Phone player: edge-to-edge video, big transport targets and chrome that
//! fades out after a few seconds of inactivity so the picture is in focus.
//! A tap or any TV-remote key brings the controls back — there is no cursor
//! to poll on a touch device, but remote keys are recorded in `App::update()`.

use egui::{pos2, vec2, Align2, Color32, Rect, Response, Rounding, Sense, Stroke, Ui};

use super::components::{self, Icon};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::types::{PlayKind, StreamInfo, Track};

const HIDE_AFTER: f64 = 4.0;
/// How long the "tap to show controls" hint stays visible after the hide.
const HINT_AFTER: f64 = 2.5;

pub fn show(app: &mut App, ctx: &egui::Context, l: &Layout) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(Color32::BLACK))
        .show(ctx, |ui| body(app, ui, l));
}

#[cfg(target_os = "android")]
fn title_for_radio(app: &App) -> String {
    app.current_play
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_default()
}

#[cfg(target_os = "android")]
fn draw_radio_card(ui: &Ui, rect: Rect, title: &str) {
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_rgb(0x0D, 0x0F, 0x12));
    let center = pos2(rect.center().x, rect.center().y - 28.0);
    ui.painter().circle_filled(center, 52.0, accent_fill());
    ui.painter()
        .circle_stroke(center, 52.0, Stroke::new(2.0_f32, accent()));
    ui.painter().text(
        center,
        Align2::CENTER_CENTER,
        "RADIO",
        bold(FS_HEAD),
        Color32::WHITE,
    );
    ui.painter().text(
        pos2(center.x, center.y + 82.0),
        Align2::CENTER_CENTER,
        title,
        bold(FS_TITLE),
        Color32::WHITE,
    );
    ui.painter().text(
        pos2(center.x, center.y + 108.0),
        Align2::CENTER_CENTER,
        "Nur Audio",
        reg(FS_CAP),
        Color32::from_white_alpha(170),
    );
}

fn body(app: &mut App, ui: &mut Ui, l: &Layout) {
    let ctx = ui.ctx().clone();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, Color32::BLACK);

    let now = ctx.input(|i| i.time);
    if app.ui_activity < 0.0 {
        app.ui_activity = now;
    }
    // Touch and drag activity. Remote keys never reach this screen: the native
    // queue is drained in `App::update()` before `draw()`, which records them
    // in `ui_activity` for the player view.
    if ctx.input(|i| i.pointer.any_pressed() || i.pointer.any_down()) {
        app.ui_activity = now;
    }
    // An open track selector is an active interaction, not idle time.
    let menu_open = app.audio_menu_open || app.sub_menu_open;
    if menu_open {
        app.ui_activity = now;
    }
    // QA-only hook for screenshots. It is NOT a runtime default on any
    // platform: pinning the chrome on Android was the original "menu never
    // disappears" bug.
    let keep_ui = std::env::var("DADO_KEEP_UI").is_ok();
    let idle = now - app.ui_activity;
    // A key or tap that lands while the chrome is hidden only brings it back.
    // `update()` flags that one frame; spending it on the picture alone means
    // the reveal press can never hit the Back button that is about to appear.
    let reveal_only = std::mem::take(&mut app.player_reveal);
    let show_ui = (keep_ui || idle < HIDE_AFTER) && !reveal_only;
    app.player_chrome = show_ui;
    if show_ui && !keep_ui {
        // egui only redraws on input; without a scheduled wake-up the controls
        // would stay on screen until the next touch even after 4 seconds.
        let ms = ((HIDE_AFTER - idle) * 1000.0).ceil() as u64 + 16;
        ctx.request_repaint_after(std::time::Duration::from_millis(ms));
    }

    let top_h = if show_ui { l.appbar_h } else { 0.0 };
    let bot_h = if show_ui {
        if l.compact_height() {
            132.0
        } else {
            168.0
        }
    } else {
        0.0
    };
    let video = Rect::from_min_max(
        pos2(full.left(), full.top() + top_h),
        pos2(full.right(), full.bottom() - bot_h),
    );

    #[cfg(target_os = "android")]
    let is_radio = app
        .current_play
        .as_ref()
        .map(|p| app.is_radio(&p.progress_key))
        .unwrap_or(false);

    // Android: paint mpv first so the render context is created even for an
    // audio-only station (the queued load depends on that callback), then cover
    // its empty/no-video output with an opaque radio card. The normal desktop
    // player uses a native child window instead.
    #[cfg(target_os = "android")]
    {
        ui.painter()
            .add(crate::platform::android::render::shape(full));
        if is_radio {
            draw_radio_card(ui, video, &title_for_radio(app));
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        app.position_video(video, ctx.pixels_per_point());
        // The desktop player reveals its chrome via HWND region cutouts. The
        // mobile layout draws all of its own chrome in egui, so drop any
        // leftover cutout when the same process switches player views.
        app.set_video_cutouts(&[]);
    }

    // A render-API failure is not a decode failure: it surfaces here rather
    // than as a black rectangle with a live UI around it.
    let err = app.player_error.clone().or({
        #[cfg(target_os = "android")]
        {
            crate::platform::android::render::last_error()
        }
        #[cfg(target_os = "ios")]
        {
            extern "C" {
                fn dado_host_error_code() -> i32;
            }
            let code = unsafe { dado_host_error_code() };
            (code != 0)
                .then(|| format!("AVPlayer-Fehler {code}. Streamformat und Verbindung prüfen."))
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            None
        }
    });

    if let Some(e) = err {
        #[cfg(target_os = "ios")]
        crate::platform::ios::bridge::video_visible(false);
        let text = format!("Wiedergabe nicht möglich\n{}", e);
        ui.painter().text(
            video.center(),
            Align2::CENTER_CENTER,
            text,
            reg(FS_LABEL),
            Color32::from_rgb(0xE5, 0x9B, 0x9B),
        );
    }

    // "MX Player ist nicht installiert, integrierter Player läuft" - a status
    // message, not a failure. It fades out after a few seconds instead of
    // becoming permanent furniture over the picture.
    if let Some((msg, at)) = app.player_note.clone() {
        if at.elapsed().as_secs_f64() < 6.0 {
            let g = components::fit(ui, &msg, reg(FS_CAP), Color32::WHITE, full.width() - 80.0);
            let box_r = Rect::from_center_size(
                pos2(full.center().x, full.bottom() - bot_h - 36.0),
                g.size() + vec2(36.0, 20.0),
            );
            ui.painter()
                .rect_filled(box_r, Rounding::same(12.0), Color32::from_black_alpha(200));
            ui.painter().galley(
                pos2(
                    box_r.center().x - g.size().x * 0.5,
                    box_r.center().y - g.size().y * 0.5,
                ),
                g,
                Color32::WHITE,
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        } else {
            app.player_note = None;
        }
    }

    if !show_ui {
        app.audio_menu_open = false;
        app.sub_menu_open = false;
        // The key/tap that reveals the chrome must not also activate the
        // first control (Back), which would stop playback immediately.
        focus::clear_activation();
        let since = idle - HIDE_AFTER;
        if !reveal_only && since < HINT_AFTER {
            let alpha = (150.0 * (1.0 - (since / HINT_AFTER).clamp(0.0, 1.0))) as u8;
            let g = components::fit(
                ui,
                "Tippen oder Taste drücken für Bedienung",
                reg(FS_CAP),
                Color32::WHITE,
                full.width() - 48.0,
            );
            ui.painter().galley(
                pos2(
                    full.center().x - g.size().x * 0.5,
                    full.bottom() - 40.0 - g.size().y,
                ),
                g,
                Color32::from_white_alpha(alpha),
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        return;
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

    // ---- top bar ----
    let top = Rect::from_min_size(full.min, vec2(full.width(), l.appbar_h + l.top_inset));
    scrim(ui, top, true);
    let back = Rect::from_center_size(
        pos2(top.left() + 8.0 + 24.0, top.center().y + l.top_inset * 0.5),
        vec2(TOUCH, TOUCH),
    );
    let back_resp = ui.interact(back, ui.id().with("p-back"), Sense::click());
    components::icon(
        ui,
        Rect::from_center_size(back.center(), vec2(24.0, 24.0)),
        Icon::Back,
        Color32::WHITE,
    );
    // Register Back first (top-left) so it is focus index 0 on a fresh player.
    let back_hit = focus::clicked(ui, &back_resp);
    let tw = (full.width() - 140.0).max(80.0);
    let g = components::fit(ui, &title, bold(FS_HEAD), Color32::WHITE, tw);
    ui.painter().galley(
        pos2(
            back.right() + 6.0,
            top.center().y + l.top_inset * 0.5 - g.size().y * 0.5,
        ),
        g,
        Color32::WHITE,
    );
    if is_live {
        let b = Rect::from_min_size(
            pos2(
                full.right() - 66.0,
                top.center().y + l.top_inset * 0.5 - 10.0,
            ),
            vec2(50.0, 20.0),
        );
        components::live_badge(ui, b);
    }

    // ---- bottom bar ----
    let bot = Rect::from_min_size(
        pos2(full.left(), full.bottom() - bot_h),
        vec2(full.width(), bot_h),
    );
    let controls = bot.shrink2(vec2(8.0, 4.0));
    ui.painter().rect_filled(
        controls,
        22.0,
        Color32::from_rgba_unmultiplied(19, 30, 32, 212),
    );
    ui.painter().rect_stroke(
        controls,
        22.0,
        Stroke::new(1.0_f32, Color32::from_white_alpha(40)),
    );

    let mut seek_to: Option<f64> = None;
    let mut seek_rel: f64 = 0.0;
    let mut toggle = false;
    let mut new_vol: Option<i64> = None;
    let mut do_full = false;
    let mut step: i32 = 0;

    // progress
    let bar = Rect::from_min_max(
        pos2(bot.left() + l.pad, bot.top() + 18.0),
        pos2(bot.right() - l.pad, bot.top() + 24.0),
    );
    if is_live {
        let b = Rect::from_min_size(pos2(bot.left() + l.pad, bot.top() + 14.0), vec2(54.0, 20.0));
        components::live_badge(ui, b);
        let txt = format!("Puffer {:.0}s", cache);
        let g = components::fit(ui, &txt, reg(FS_CAP), Color32::from_white_alpha(170), 120.0);
        ui.painter().galley(
            pos2(bot.right() - l.pad - g.size().x, bot.top() + 14.0),
            g,
            Color32::from_white_alpha(170),
        );
    } else {
        let frac = if dur > 0.0 {
            (pos / dur).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let buffered = if dur > 0.0 {
            ((pos + cache) / dur).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        // buffered track
        ui.painter().rect_filled(
            Rect::from_min_size(bar.min, vec2(bar.width() * buffered, bar.height())),
            Rounding::same(3.0),
            Color32::from_white_alpha(60),
        );
        if let Some(f) = scrub(ui, bar, frac) {
            if dur > 0.0 {
                seek_to = Some((f as f64 * dur).max(0.0));
            }
        }
        let g = components::fit(
            ui,
            &crate::ui::fmt_time(pos),
            reg(FS_CAP),
            Color32::WHITE,
            60.0,
        );
        ui.painter()
            .galley(pos2(bar.left(), bar.bottom() + 6.0), g, Color32::WHITE);
        let g = components::fit(
            ui,
            &crate::ui::fmt_time(dur),
            reg(FS_CAP),
            Color32::from_white_alpha(170),
            60.0,
        );
        ui.painter().galley(
            pos2(bar.right() - g.size().x, bar.bottom() + 6.0),
            g,
            Color32::from_white_alpha(170),
        );
    }

    // transport
    let cy = if is_live {
        bot.top() + 70.0
    } else {
        bot.bottom() - 44.0
    };
    let cx = full.center().x;

    let play_size = 64.0;
    let play_rect = Rect::from_center_size(pos2(cx, cy), vec2(play_size, play_size));
    let pr = ui.interact(play_rect, ui.id().with("p-play"), Sense::click());
    ui.painter()
        .circle_filled(play_rect.center(), play_size * 0.5, accent_fill());
    components::icon(
        ui,
        Rect::from_center_size(play_rect.center(), vec2(30.0, 30.0)),
        if paused { Icon::Play } else { Icon::Pause },
        Color32::WHITE,
    );
    if focus::clicked(ui, &pr) {
        toggle = true;
    }

    let side_x = 78.0;
    let left = Rect::from_center_size(pos2(cx - side_x, cy), vec2(TOUCH, TOUCH));
    let right = Rect::from_center_size(pos2(cx + side_x, cy), vec2(TOUCH, TOUCH));
    let lr = ui.interact(left, ui.id().with("p-left"), Sense::click());
    let rr = ui.interact(right, ui.id().with("p-right"), Sense::click());
    components::icon(
        ui,
        Rect::from_center_size(left.center(), vec2(28.0, 28.0)),
        Icon::SkipBack,
        Color32::WHITE,
    );
    components::icon(
        ui,
        Rect::from_center_size(right.center(), vec2(28.0, 28.0)),
        Icon::SkipFwd,
        Color32::WHITE,
    );
    if focus::clicked(ui, &lr) {
        if is_live {
            step = -1;
        } else {
            seek_rel = -10.0;
        }
    }
    if focus::clicked(ui, &rr) {
        if is_live {
            step = 1;
        } else {
            seek_rel = 10.0;
        }
    }

    // volume + fullscreen
    let vol_rect = Rect::from_center_size(pos2(bot.left() + l.pad + 24.0, cy), vec2(TOUCH, TOUCH));
    let vr = ui.interact(vol_rect, ui.id().with("p-vol"), Sense::click());
    components::icon(
        ui,
        Rect::from_center_size(vol_rect.center(), vec2(24.0, 24.0)),
        Icon::Volume,
        if app.volume == 0 {
            Color32::from_white_alpha(120)
        } else {
            Color32::WHITE
        },
    );
    if focus::clicked(ui, &vr) {
        new_vol = Some(if app.volume > 0 { 0 } else { 100 });
    }

    let fs_rect = Rect::from_center_size(pos2(bot.right() - l.pad - 24.0, cy), vec2(TOUCH, TOUCH));
    let fr = ui.interact(fs_rect, ui.id().with("p-fs"), Sense::click());
    components::icon(
        ui,
        Rect::from_center_size(fs_rect.center(), vec2(24.0, 24.0)),
        Icon::Fullscreen,
        Color32::WHITE,
    );
    if focus::clicked(ui, &fr) {
        do_full = true;
    }

    // ---- apply ----
    if back_hit {
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
    if step != 0 {
        app.step_channel(step);
    }
    if let Some(v) = new_vol {
        app.volume = v;
        if let Some(pl) = &app.player {
            pl.mpv.set_volume(v);
        }
    }
    if do_full {
        app.fullscreen = !app.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.fullscreen));
    }

    // ---------------- bottom-right stream overlay ----------------
    // Resolution / fps / codecs for every kind; audio + subtitle selectors for
    // movies & series. Only drawn while the auto-hiding menu is visible (we
    // already returned above when `!show_ui`), so it never stays on screen.
    {
        let pl_ref = app.player.as_ref();
        let stream: StreamInfo = pl_ref.map(|pl| pl.stream.clone()).unwrap_or_default();
        let atracks: Vec<Track> = pl_ref.map(|pl| pl.audio_tracks.clone()).unwrap_or_default();
        let stracks: Vec<Track> = pl_ref.map(|pl| pl.sub_tracks.clone()).unwrap_or_default();
        let cur_a = pl_ref.map(|pl| pl.current_audio).unwrap_or(0);
        let cur_s = pl_ref.map(|pl| pl.current_sub).unwrap_or(0);

        let pad: f32 = 14.0;
        let line_h: f32 = 18.0;
        let info_h = line_h * 2.0 + 14.0;
        let is_vod = !is_live;
        let btns_h = if is_vod { TOUCH } else { 0.0 };
        let gap: f32 = if is_vod { 10.0 } else { 0.0 };
        let panel_h = info_h + gap + btns_h;
        let panel_w: f32 = 260.0;
        let panel = Rect::from_min_size(
            pos2(full.right() - pad - panel_w, video.bottom() - pad - panel_h),
            vec2(panel_w, panel_h),
        );
        ui.painter().rect_filled(
            panel,
            Rounding::same(RADIUS_SM),
            Color32::from_black_alpha(150),
        );

        // Line 1: resolution + fps
        let mut res = String::new();
        if stream.width > 0 && stream.height > 0 {
            res = format!("{}×{}", stream.width, stream.height);
            if stream.fps > 0.0 {
                res = format!("{}  ·  {:.0} fps", res, stream.fps);
            }
        }
        let g = components::fit(
            ui,
            if res.is_empty() { "…" } else { &res },
            reg(FS_LABEL),
            Color32::WHITE,
            panel_w - 24.0,
        );
        ui.painter().galley(
            pos2(panel.left() + 12.0, panel.top() + 10.0),
            g,
            Color32::WHITE,
        );

        // Line 2: codecs (+ container for movies / series)
        let mut codec = String::new();
        if !stream.video_codec.is_empty() {
            codec.push_str(&stream.video_codec.to_uppercase());
        }
        if !stream.audio_codec.is_empty() {
            if !codec.is_empty() {
                codec.push_str("  ·  ");
            }
            codec.push_str(&stream.audio_codec.to_uppercase());
        }
        if is_vod && !stream.container.is_empty() {
            if !codec.is_empty() {
                codec.push_str("  ·  ");
            }
            codec.push_str(&stream.container.to_uppercase());
        }
        let g = components::fit(
            ui,
            if codec.is_empty() { "…" } else { &codec },
            reg(FS_CAP),
            Color32::from_white_alpha(170),
            panel_w - 24.0,
        );
        ui.painter().galley(
            pos2(panel.left() + 12.0, panel.top() + 10.0 + line_h),
            g,
            Color32::from_white_alpha(170),
        );

        if is_vod {
            let by = panel.top() + info_h + gap;
            let inner_w = panel_w - 24.0;
            let btn_w = (inner_w - 10.0) / 2.0;
            let a_rect = Rect::from_min_size(pos2(panel.left() + 12.0, by), vec2(btn_w, btns_h));
            let s_rect = Rect::from_min_size(
                pos2(panel.left() + 12.0 + btn_w + 10.0, by),
                vec2(btn_w, btns_h),
            );
            let ar = ui.interact(a_rect, ui.id().with("p-aud"), Sense::click());
            let sr = ui.interact(s_rect, ui.id().with("p-sub"), Sense::click());
            draw_mini_btn(ui, a_rect, "🔊 Audio", ar.hovered());
            draw_mini_btn(ui, s_rect, "CC Untertitel", sr.hovered());
            if focus::clicked(ui, &ar) {
                app.audio_menu_open = !app.audio_menu_open;
                app.sub_menu_open = false;
            }
            if focus::clicked(ui, &sr) {
                app.sub_menu_open = !app.sub_menu_open;
                app.audio_menu_open = false;
            }

            if let Some(p) = mobile_track_popover(
                ui,
                full,
                "aud",
                a_rect,
                &atracks,
                cur_a,
                false,
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
            if let Some(p) = mobile_track_popover(
                ui,
                full,
                "sub",
                s_rect,
                &stracks,
                cur_s,
                true,
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

/// Result of a click inside the mobile track selector popover.
enum TrackPick {
    Select(i64),
    Off,
}

/// A small filled button used inside the stream overlay.
fn draw_mini_btn(ui: &mut Ui, rect: Rect, label: &str, hovered: bool) {
    let fill = if hovered {
        Color32::from_white_alpha(46)
    } else {
        Color32::from_white_alpha(24)
    };
    ui.painter().rect_filled(rect, Rounding::same(10.0), fill);
    let g = components::fit(ui, label, reg(FS_CAP), Color32::WHITE, rect.width() - 12.0);
    ui.painter().galley(
        pos2(
            rect.center().x - g.size().x * 0.5,
            rect.center().y - g.size().y * 0.5,
        ),
        g,
        Color32::WHITE,
    );
}

/// Draw the audio/subtitle track selector anchored above `anchor`. Returns the
/// chosen action (or `None` when nothing was picked / the menu is closed).
// Kept as explicit arguments to stay allocation-free on the per-frame path.
#[allow(clippy::too_many_arguments)]
fn mobile_track_popover(
    ui: &mut Ui,
    full: Rect,
    tag: &str,
    anchor: Rect,
    tracks: &[Track],
    current: i64,
    for_sub: bool,
    open: &mut bool,
) -> Option<TrackPick> {
    if !*open {
        return None;
    }
    let row_h = TOUCH;
    let header_h = 36.0;
    let off_rows = if for_sub { 1 } else { 0 };
    let rows = tracks.len() + off_rows;
    let pop_w = 264.0;
    let content_h = header_h + (rows as f32) * row_h + 8.0;
    let mut pop = Rect::from_min_size(
        pos2(anchor.right() - pop_w, anchor.top() - 8.0 - content_h),
        vec2(pop_w, content_h),
    );
    let m = 8.0;
    if pop.left() < full.left() + m {
        pop = pop.translate(vec2((full.left() + m) - pop.left(), 0.0));
    }
    if pop.top() < full.top() + m {
        pop = pop.translate(vec2(0.0, (full.top() + m) - pop.top()));
    }
    if pop.right() > full.right() - m {
        pop = pop.translate(vec2((full.right() - m) - pop.right(), 0.0));
    }
    if pop.bottom() > full.bottom() - m {
        pop = pop.translate(vec2(0.0, (full.bottom() - m) - pop.bottom()));
    }
    ui.painter().rect_filled(
        pop,
        Rounding::same(RADIUS_SM),
        Color32::from_black_alpha(195),
    );
    let g = components::fit(
        ui,
        if for_sub { "Untertitel" } else { "Audio" },
        bold(FS_ROW),
        Color32::WHITE,
        pop_w - 24.0,
    );
    ui.painter().galley(
        pos2(
            pop.left() + 12.0,
            pop.top() + header_h * 0.5 - g.size().y * 0.5,
        ),
        g,
        Color32::WHITE,
    );

    let mut pick: Option<TrackPick> = None;
    let mut y = pop.top() + header_h;
    if for_sub {
        let r = Rect::from_min_size(pos2(pop.left() + 6.0, y), vec2(pop_w - 12.0, row_h));
        let resp = ui.interact(
            r,
            ui.id().with((tag, "off", r.min.y as i32)),
            Sense::click(),
        );
        let sel = current == 0;
        draw_row_bg(ui, r, sel, resp.hovered());
        let g = components::fit(ui, "Aus", reg(FS_ROW), row_fg(sel), pop_w - 24.0);
        ui.painter().galley(
            pos2(r.left() + 14.0, r.center().y - g.size().y * 0.5),
            g,
            row_fg(sel),
        );
        if focus::clicked(ui, &resp) {
            pick = Some(TrackPick::Off);
        }
        y += row_h;
    }
    if tracks.is_empty() {
        let r = Rect::from_min_size(pos2(pop.left() + 6.0, y), vec2(pop_w - 12.0, row_h));
        let g = components::fit(
            ui,
            if for_sub {
                "Keine Untertitel"
            } else {
                "Keine Spuren"
            },
            reg(FS_ROW),
            muted(ui),
            pop_w - 24.0,
        );
        ui.painter().galley(
            pos2(r.left() + 14.0, r.center().y - g.size().y * 0.5),
            g,
            muted(ui),
        );
    } else {
        for (i, t) in tracks.iter().enumerate() {
            let r = Rect::from_min_size(pos2(pop.left() + 6.0, y), vec2(pop_w - 12.0, row_h));
            let resp = ui.interact(r, ui.id().with((tag, i, r.min.y as i32)), Sense::click());
            let sel = t.id == current;
            draw_row_bg(ui, r, sel, resp.hovered());
            let g = components::fit(ui, &t.label(), reg(FS_ROW), row_fg(sel), pop_w - 44.0);
            ui.painter().galley(
                pos2(r.left() + 14.0, r.center().y - g.size().y * 0.5),
                g,
                row_fg(sel),
            );
            if sel {
                let c = components::fit(ui, "✓", reg(FS_ROW), accent_text(ui), 30.0);
                ui.painter().galley(
                    pos2(
                        r.right() - 14.0 - c.size().x,
                        r.center().y - c.size().y * 0.5,
                    ),
                    c,
                    accent_text(ui),
                );
            }
            if focus::clicked(ui, &resp) {
                pick = Some(TrackPick::Select(t.id));
            }
            y += row_h;
        }
    }
    if pick.is_some() {
        *open = false;
    }
    pick
}

/// Row background for the track popover (selected / hovered / plain).
fn draw_row_bg(ui: &mut Ui, rect: Rect, selected: bool, hovered: bool) {
    let fill = if selected {
        accent_fill().gamma_multiply(0.38)
    } else if hovered {
        Color32::from_white_alpha(30)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(8.0), fill);
}

/// Foreground colour for a popover row (accent when selected).
fn row_fg(selected: bool) -> Color32 {
    if selected {
        crate::theme::ACCENT
    } else {
        Color32::WHITE
    }
}

/// Draggable progress bar. Returns the new fraction while dragging.
fn scrub(ui: &mut Ui, bar: Rect, frac: f32) -> Option<f32> {
    // The track is 6 dp; the touch target must still be 48 dp, so the hit rect
    // is grown vertically to reach TOUCH rather than by a fixed ±14 (which
    // gave 34 dp and was hard to grab).
    let grow = ((TOUCH - bar.height()) * 0.5).max(12.0);
    let hit = Rect::from_min_max(
        pos2(bar.left() - 8.0, bar.top() - grow),
        pos2(bar.right() + 8.0, bar.bottom() + grow),
    );
    let resp: Response = ui.interact(hit, ui.id().with("scrub"), Sense::click_and_drag());
    let mut f = frac;
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            f = ((p.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
        }
    }
    // track
    ui.painter()
        .rect_filled(bar, Rounding::same(3.0), Color32::from_white_alpha(48));
    let filled = Rect::from_min_size(bar.min, vec2(bar.width() * f, bar.height()));
    ui.painter()
        .rect_filled(filled, Rounding::same(3.0), accent_fill());
    ui.painter().circle_filled(
        pos2(bar.left() + bar.width() * f, bar.center().y),
        8.0,
        Color32::WHITE,
    );

    let activated = focus::clicked(ui, &resp);
    if resp.dragged() || resp.clicked() || activated {
        Some(f)
    } else {
        None
    }
}

/// On Android the render API paints the picture into the framebuffer, so the
/// chrome belongs on a translucent gradient scrim over it. On desktop video is
/// a native child window above egui — any scrim drawn here would be hidden
/// behind it — so opaque bars outside the video rect are the correct choice.
const HAS_VIDEO: bool = cfg!(target_os = "android");

fn scrim(ui: &Ui, rect: Rect, top: bool) {
    if !HAS_VIDEO {
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgb(0x0D, 0x10, 0x15));
        let y = if top {
            rect.bottom() - 0.5
        } else {
            rect.top() + 0.5
        };
        ui.painter().hline(
            rect.left()..=rect.right(),
            y,
            Stroke::new(1.0_f32, Color32::from_rgb(0x26, 0x2E, 0x3A)),
        );
        return;
    }
    gradient(ui, rect, top);
}

fn gradient(ui: &Ui, rect: Rect, top: bool) {
    // Cheap two-step scrim: solid at the edge, transparent towards the video.
    let (a, b) = if top {
        (Color32::from_black_alpha(170), Color32::TRANSPARENT)
    } else {
        (Color32::TRANSPARENT, Color32::from_black_alpha(190))
    };
    let steps = 6;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let c = mix(a, b, (t0 + t1) * 0.5);
        let y0 = rect.top() + rect.height() * t0;
        let y1 = rect.top() + rect.height() * t1;
        ui.painter().rect_filled(
            Rect::from_min_max(pos2(rect.left(), y0), pos2(rect.right(), y1 + 0.5)),
            0.0,
            c,
        );
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}
