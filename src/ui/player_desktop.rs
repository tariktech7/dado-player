//! Samsung-style Windows controls. Native mpv keeps the same video rectangle
//! while the composited panes appear, change, and hide.
use crate::app::App;
use crate::platform::windows::player_overlay::cursor_inside;
use crate::types::{DesktopAspect, PlayKind};
use egui::{
    pos2, vec2, Align, Align2, Color32, Context, FontId, Id, LayerId, Layout, Order, Rect, Sense,
    Stroke, Ui,
};
use std::sync::OnceLock;
use std::time::Duration;

struct Palette {
    panel: Color32,
    button: Color32,
    primary: Color32,
    border: Color32,
    focus: Color32,
    text: Color32,
    muted: Color32,
}

impl Palette {
    fn get() -> &'static Self {
        static COLORS: OnceLock<Palette> = OnceLock::new();
        COLORS.get_or_init(|| {
            let a = super::desktop::appearance();
            let parse = |hex: &str| Color32::from_hex(hex).expect("bundled player palette");
            Self {
                panel: parse(&a.player_panel_color),
                button: parse(&a.player_button_color),
                primary: parse(&a.player_primary_color),
                border: parse(&a.player_border_color),
                focus: parse(&a.player_focus_color),
                text: parse(&a.player_text_color),
                muted: parse(&a.player_muted_color),
            }
        })
    }
}

fn pane<R>(
    ctx: &Context,
    name: &'static str,
    rect: Rect,
    scale: f32,
    render: impl FnOnce(&mut Ui) -> R,
) -> (R, (LayerId, Rect)) {
    let id = Id::new(("desktop-player", name));
    let layer = LayerId::new(Order::Foreground, id);
    let result = egui::Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .constrain(false)
        .movable(false)
        .show(ctx, |area| {
            area.set_min_size(rect.size());
            area.set_clip_rect(rect);
            let p = Palette::get();
            area.painter().rect(
                rect.shrink(1.0),
                super::desktop::appearance().player_corner_radius * scale,
                p.panel,
                Stroke::new(1.0_f32, p.border),
            );
            render(area)
        })
        .inner;
    (result, (layer, rect))
}

fn button(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    rect: Rect,
    label: &str,
    size: f32,
    primary: bool,
) -> bool {
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    let p = Palette::get();
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().rect(
        rect,
        size * 0.48,
        if primary { p.primary } else { p.button },
        if response.hovered() || response.has_focus() {
            Stroke::new(2.0_f32, p.focus)
        } else {
            Stroke::NONE
        },
    );
    ui.painter()
        .with_clip_rect(rect.shrink2(vec2(4.0, 0.0)))
        .text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(size),
            p.text,
        );
    response.clicked()
}

fn line(ui: &Ui, at: egui::Pos2, width: f32, text: &str, size: f32, bold: bool) {
    if width > 0.0 {
        super::mobile::components::draw_line(
            ui,
            at,
            text,
            if bold {
                super::mobile::tokens::bold(size)
            } else {
                FontId::proportional(size)
            },
            if bold {
                Palette::get().text
            } else {
                Palette::get().muted
            },
            width,
        );
    }
}

fn aspect_label(value: DesktopAspect, english: bool) -> &'static str {
    match (value, english) {
        (DesktopAspect::Auto, false) => "Automatisch",
        (DesktopAspect::Auto, true) => "Automatic",
        (DesktopAspect::Fit, false) => "Einpassen",
        (DesktopAspect::Fill, false) => "Ausfüllen",
        (DesktopAspect::Stretch, false) => "Strecken",
        (DesktopAspect::Fit, true) => "Fit",
        (DesktopAspect::Fill, true) => "Fill",
        (DesktopAspect::Stretch, true) => "Stretch",
    }
}

fn popup_row(
    ctx: &Context,
    name: &'static str,
    bottom: Rect,
    scale: f32,
    entries: &[(&str, f32, bool)],
) -> (Option<usize>, (LayerId, Rect)) {
    let a = super::desktop::appearance();
    let pad = a.player_popup_padding * scale;
    let gap = a.player_popup_gap * scale;
    let height = a.player_button_height * scale;
    let width = entries.iter().map(|(_, w, _)| w * scale).sum::<f32>()
        + gap * entries.len().saturating_sub(1) as f32
        + 2.0 * pad;
    let y = bottom.top() - a.player_popup_offset * scale - height - 2.0 * pad;
    let rect = Rect::from_min_size(
        pos2(bottom.center().x - width * 0.5, y),
        vec2(width, height + 2.0 * pad),
    );
    pane(ctx, name, rect, scale, |ui| {
        let mut x = rect.left() + pad;
        let mut selection = None;
        for (index, &(label, width, selected)) in entries.iter().enumerate() {
            let bounds = Rect::from_min_size(pos2(x, y + pad), vec2(width * scale, height));
            if button(
                ui,
                index,
                bounds,
                label,
                a.player_button_font * scale,
                selected,
            ) {
                selection = Some(index);
            }
            x += width * scale + gap;
        }
        selection
    })
}

pub(crate) fn dismiss_menus(app: &mut App, ctx: &Context) -> bool {
    let mut active = app.audio_menu_open || app.sub_menu_open;
    app.audio_menu_open = false;
    app.sub_menu_open = false;
    ctx.data_mut(|data| {
        for key in ["desktop-player-aspect-menu", "desktop-player-options-menu"] {
            let id = Id::new(key);
            active |= data.get_temp::<bool>(id).unwrap_or(false);
            data.insert_temp(id, false);
        }
    });
    active
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let full = ui.available_rect_before_wrap();
    ui.painter().rect_filled(full, 0.0, Color32::BLACK);
    let Some(item) = app.current_play.clone() else {
        return;
    };
    let live = item.kind == PlayKind::Live;
    let ppp = ctx.pixels_per_point();
    let a = super::desktop::appearance();
    let s = (full.width() / a.reference_width)
        .clamp(0.66, 1.25)
        .min(full.width() / 610.0)
        .min(full.height() / 600.0);
    let top_size = vec2(
        (full.width() - 2.0 * a.player_top_inset_x * s).min(a.player_top_max_width * s),
        a.player_top_height * s,
    );
    let title = Rect::from_center_size(
        pos2(
            full.center().x,
            full.top() + a.player_top_inset_y * s + top_size.y / 2.0,
        ),
        top_size,
    );
    let bottom_size = vec2(
        (full.width() - 2.0 * a.player_controls_inset_x * s).min(a.player_controls_max_width * s),
        a.player_controls_height * s,
    );
    let bottom = Rect::from_center_size(
        pos2(
            full.center().x,
            full.bottom() - a.player_controls_inset_y * s - bottom_size.y / 2.0,
        ),
        bottom_size,
    );
    let radio = app.is_radio(&item.progress_key);
    let video = app.player.as_ref().map(|p| p.surface.native_handle());
    app.position_video(
        if radio {
            Rect::from_min_size(pos2(-50.0, -50.0), vec2(1.0, 1.0))
        } else {
            full
        },
        ppp,
    );
    if radio {
        super::player::draw_radio_card(app, ui, full);
    }
    let time = ctx.input(|i| i.time);
    let keep = std::env::var_os("DADO_KEEP_UI").is_some();
    let aspect_menu = Id::new("desktop-player-aspect-menu");
    let options_menu = Id::new("desktop-player-options-menu");
    let mut options_open = ctx
        .data(|data| data.get_temp(options_menu))
        .unwrap_or(false);
    let mut aspect_open = ctx.data(|data| data.get_temp(aspect_menu)).unwrap_or(false);
    if app.ui_activity < 0.0
        || app.note_pointer_activity()
        || ctx.input(|i| i.pointer.any_down())
        || (app.player_chrome
            && video.is_some_and(|handle| {
                cursor_inside(handle, title, ppp) || cursor_inside(handle, bottom, ppp)
            }))
        || app.audio_menu_open
        || app.sub_menu_open
        || aspect_open
        || options_open
    {
        app.ui_activity = time;
    }
    let remaining = crate::release_info::CONTROLS_HIDE_SECONDS - (time - app.ui_activity);
    let reveal_only = std::mem::take(&mut app.player_reveal);
    let visible = (keep || remaining > 0.0) && !reveal_only;
    app.player_chrome = visible;
    if !visible && app.player_error.is_none() {
        app.desktop_overlay.hide();
        app.set_video_cutouts(&[]);
        app.audio_menu_open = false;
        app.sub_menu_open = false;
        app.player_menu_navigation = false;
        dismiss_menus(app, &ctx);
        ctx.set_cursor_icon(egui::CursorIcon::None);
        return;
    }
    if !keep && remaining > 0.0 {
        ctx.request_repaint_after(Duration::from_secs_f64(remaining));
    }

    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let is_english = t("de", "en") == "en";
    let mut layers = Vec::with_capacity(4);
    let (back, layer) = pane(&ctx, "top", title, s, |ui| {
        let center = title.center().y;
        let control = |offset: f32, width: f32| {
            Rect::from_min_size(
                pos2(title.left() + offset * s, center - 20.0 * s),
                vec2(width, 40.0) * s,
            )
        };
        let back = button(
            ui,
            "back",
            control(12.0, 113.0),
            t("← Zurück", "← Back"),
            22.0 * s,
            false,
        );
        let favorite_x = title.width() / s - 108.0;
        let favorite = app.is_favorite(&item.progress_key);
        if button(
            ui,
            "favorite",
            control(favorite_x, 42.0),
            if favorite { "★" } else { "☆" },
            24.0 * s,
            favorite,
        ) {
            app.toggle_favorite(&item.progress_key);
        }
        if button(
            ui,
            "fullscreen",
            control(title.width() / s - 54.0, 42.0),
            "⛶",
            22.0 * s,
            false,
        ) {
            app.fullscreen = !app.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.fullscreen));
        }
        let label = if live {
            "LIVE"
        } else if item.kind == PlayKind::Episode {
            t("SERIE", "SERIES")
        } else {
            t("FILM", "MOVIE")
        };
        let left = title.left() + 144.0 * s;
        let available = (favorite_x - 160.0) * s;
        line(
            ui,
            pos2(left, center - 12.0 * s),
            available,
            label,
            15.0 * s,
            false,
        );
        line(
            ui,
            pos2(left, center + 9.0 * s),
            available,
            &item.title,
            25.0 * s,
            true,
        );
        back
    });
    if back {
        dismiss_menus(app, &ctx);
        app.stop_play();
        return;
    }
    layers.push(layer);

    let (position, duration, paused) = app
        .player
        .as_ref()
        .map(|player| (player.position, player.duration, player.paused))
        .unwrap_or_default();
    let names = [
        if live {
            t("‹ Sender", "‹ Channel")
        } else {
            "-10 s"
        },
        if paused {
            t("Abspielen", "Play")
        } else {
            "Pause"
        },
        if live {
            t("Sender ›", "Channel ›")
        } else {
            "+10 s"
        },
        t("Optionen", "Options"),
    ];
    let (action, layer) = pane(&ctx, "transport", bottom, s, |ui| {
        let widths = [104.0, 132.0, 104.0, 112.0];
        let mut x = bottom.center().x - (widths.iter().sum::<f32>() + 34.0) * s / 2.0;
        let mut action = None;
        for (i, (name, width)) in names.into_iter().zip(widths).enumerate() {
            let r = Rect::from_min_size(
                pos2(x, bottom.top() + 10.0 * s),
                vec2(width, a.player_button_height) * s,
            );
            if button(
                ui,
                i,
                r,
                name,
                a.player_button_font * s,
                i == 1 || (i == 3 && options_open),
            ) {
                action = Some(i);
            }
            x += (width + if i == 2 { 18.0 } else { 8.0 }) * s;
        }
        let y = bottom.bottom() - 22.0 * s;
        let guide = if live {
            let utc = chrono::Utc::now().timestamp();
            item.progress_key
                .strip_prefix("live:")
                .and_then(|id| id.parse().ok())
                .and_then(|id| app.epg_for(id))
                .and_then(|entries| entries.iter().find(|e| e.start <= utc && utc < e.end))
                .map(|e| {
                    (
                        e.title.as_str(),
                        (utc - e.start) as f32 / (e.end - e.start).max(1) as f32,
                    )
                })
        } else {
            None
        };
        let start = if live {
            "● LIVE".into()
        } else {
            super::fmt_time(position)
        };
        let end = if live {
            guide.map_or("", |(title, _)| title).to_owned()
        } else {
            super::fmt_time(duration)
        };
        let program_width = if live && !end.is_empty() {
            bottom.width() * 0.32
        } else {
            64.0 * s
        };
        line(
            ui,
            pos2(bottom.left() + 24.0 * s, y),
            110.0 * s,
            &start,
            23.0 * s,
            false,
        );
        line(
            ui,
            pos2(bottom.right() - program_width - 16.0 * s, y),
            program_width,
            &end,
            23.0 * s,
            false,
        );
        let track = Rect::from_min_max(
            pos2(bottom.left() + 92.0 * s, y - 3.0 * s),
            pos2(bottom.right() - program_width - 32.0 * s, y + 3.0 * s),
        );
        let fraction = guide
            .map(|(_, value)| value)
            .unwrap_or_else(|| {
                if duration > 0.0 {
                    (position / duration) as f32
                } else {
                    0.0
                }
            })
            .clamp(0.0, 1.0);
        let p = Palette::get();
        ui.painter()
            .rect_filled(track, 3.0 * s, Color32::from_rgb(128, 161, 194));
        ui.painter().rect_filled(
            Rect::from_min_size(track.min, vec2(track.width() * fraction, track.height())),
            3.0 * s,
            p.focus,
        );
        if !live && duration > 0.0 {
            let hit = ui.interact(
                track.expand2(vec2(0.0, 10.0 * s)),
                ui.id().with("seek"),
                Sense::click_and_drag(),
            );
            if (hit.clicked() || hit.drag_stopped()) && track.width() > 0.0 {
                if let (Some(pointer), Some(player)) =
                    (hit.interact_pointer_pos(), app.player.as_ref())
                {
                    let target = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
                    player.mpv.seek_absolute(duration * f64::from(target));
                }
            }
        }
        action
    });
    layers.push(layer);
    let step = crate::release_info::SEEK_STEP_SECONDS;
    match action {
        Some(0 | 2) => {
            let direction = if action == Some(0) { -1 } else { 1 };
            if live {
                app.step_channel(direction);
            } else if let Some(player) = &app.player {
                player.mpv.seek_relative(step * f64::from(direction));
            }
        }
        Some(1) => {
            if let Some(player) = &app.player {
                player.mpv.toggle_pause();
            }
        }
        Some(3) => {
            options_open = !options_open;
            app.audio_menu_open = false;
            app.sub_menu_open = false;
            aspect_open = false;
        }
        _ => {}
    }
    if options_open {
        let (selection, layer) = popup_row(
            &ctx,
            "options",
            bottom,
            s,
            &[
                (t("Ton & Untertitel", "Audio & subtitles"), 202.0, false),
                (t("Bildformat", "Aspect ratio"), 146.0, false),
                (t("Beenden", "Stop"), 128.0, false),
            ],
        );
        layers.push(layer);
        if let Some(selection) = selection {
            options_open = false;
            match selection {
                0 => app.audio_menu_open = true,
                1 => aspect_open = true,
                _ => {
                    dismiss_menus(app, &ctx);
                    app.stop_play();
                    return;
                }
            }
        }
    }
    ctx.data_mut(|data| data.insert_temp(options_menu, options_open));
    if aspect_open {
        let aspects = [
            DesktopAspect::Auto,
            DesktopAspect::Fit,
            DesktopAspect::Fill,
            DesktopAspect::Stretch,
        ];
        let entries = aspects.map(|value| {
            (
                aspect_label(value, is_english),
                140.0,
                app.settings.desktop_aspect == value,
            )
        });
        let (selected, layer) = popup_row(&ctx, "aspect", bottom, s, &entries);
        layers.push(layer);
        if let Some(value) = selected.and_then(|index| aspects.get(index).copied()) {
            app.settings.desktop_aspect = value;
            if let Some(player) = &app.player {
                player.mpv.set_desktop_aspect(value);
            }
            app.save_settings_now();
            aspect_open = false;
        }
    }
    ctx.data_mut(|data| data.insert_temp(aspect_menu, aspect_open));
    if app.audio_menu_open || app.sub_menu_open {
        layers.push(track_menu(app, &ctx, title, bottom, s));
    }
    if let Some(error) = &app.player_error {
        let rect = Rect::from_center_size(full.center(), vec2(820.0, 132.0) * s);
        let (_, layer) = pane(&ctx, "error", rect, s, |ui| {
            line(
                ui,
                rect.left_center() + vec2(28.0 * s, 0.0),
                rect.width() - 56.0 * s,
                error,
                25.0 * s,
                true,
            );
        });
        layers.push(layer);
    }
    let composited = !app.palette_open
        && video.is_some_and(|handle| app.desktop_overlay.paint(&ctx, handle, &layers));
    if composited {
        app.set_video_cutouts(&[]);
    } else {
        // If the window system cannot composite a layer, keep every player
        // control usable with the existing native video region mechanism.
        let holes: Vec<_> = layers
            .iter()
            .map(|(_, r)| {
                let origin = (r.min - full.min) * ppp;
                (
                    (origin.x.round() as i32),
                    (origin.y.round() as i32),
                    (r.width() * ppp).round() as i32,
                    (r.height() * ppp).round() as i32,
                )
            })
            .collect();
        app.set_video_cutouts(&holes);
    }
}

fn track_menu(
    app: &mut App,
    ctx: &Context,
    title: Rect,
    bottom: Rect,
    scale: f32,
) -> (LayerId, Rect) {
    let s = scale;
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let h = (bottom.top() - title.bottom() - 30.0 * s).clamp(160.0 * s, 450.0 * s);
    let rect = Rect::from_min_size(
        pos2(bottom.center().x - 330.0 * s, bottom.top() - h - 14.0 * s),
        vec2(660.0 * s, h),
    );
    let (_, layer) = pane(ctx, "tracks", rect, s, |ui| {
        let row =
            |x: f32, w: f32| Rect::from_min_size(rect.min + vec2(x, 18.0) * s, vec2(w, 48.0) * s);
        if button(
            ui,
            "audio",
            row(18.0, 194.0),
            t("Ton", "Audio"),
            23.0 * s,
            app.audio_menu_open,
        ) {
            app.audio_menu_open = true;
            app.sub_menu_open = false;
        }
        if button(
            ui,
            "subtitles",
            row(226.0, 240.0),
            t("Untertitel", "Subtitles"),
            23.0 * s,
            app.sub_menu_open,
        ) {
            app.audio_menu_open = false;
            app.sub_menu_open = true;
        }
        if button(
            ui,
            "close",
            row(480.0, 162.0),
            t("Schließen", "Close"),
            23.0 * s,
            false,
        ) {
            app.audio_menu_open = false;
            app.sub_menu_open = false;
        }
        let area = Rect::from_min_max(
            rect.min + vec2(20.0, 82.0) * s,
            rect.max - vec2(20.0, 18.0) * s,
        );
        let mut content = ui.child_ui(area, Layout::top_down(Align::Min));
        let visuals = content.visuals_mut();
        let p = Palette::get();
        visuals.override_text_color = Some(p.text);
        for (widget, color) in [
            (&mut visuals.widgets.inactive, p.button),
            (&mut visuals.widgets.hovered, p.primary),
            (&mut visuals.widgets.active, p.primary),
        ] {
            widget.bg_fill = color;
            widget.weak_bg_fill = color;
            widget.fg_stroke.color = p.text;
            widget.bg_stroke = Stroke::new(1.0_f32, p.focus);
            widget.rounding = 8.0.into();
        }
        egui::ScrollArea::vertical()
            .max_height(area.height())
            .show(&mut content, |ui| {
                if app.audio_menu_open {
                    if ui
                        .add(
                            egui::Slider::new(&mut app.volume, 0..=100)
                                .text(t("Lautstärke", "Volume")),
                        )
                        .changed()
                    {
                        if let Some(player) = &app.player {
                            player.mpv.set_volume(app.volume);
                        }
                    }
                }
                if let Some(player) = &app.player {
                    let subtitles = app.sub_menu_open;
                    if subtitles
                        && ui
                            .selectable_label(player.current_sub == 0, t("Aus", "Off"))
                            .clicked()
                    {
                        player.mpv.disable_sub();
                    }
                    let (tracks, selected) = if subtitles {
                        (&player.sub_tracks, player.current_sub)
                    } else {
                        (&player.audio_tracks, player.current_audio)
                    };
                    for track in tracks {
                        if ui
                            .selectable_label(track.id == selected, track.label())
                            .clicked()
                        {
                            if subtitles {
                                player.mpv.set_sub_track(track.id);
                            } else {
                                player.mpv.set_audio_track(track.id);
                            }
                        }
                    }
                }
            });
    });
    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(rgb: [f64; 3]) -> f64 {
        rgb.into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(c, w)| {
                w * if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            })
            .sum()
    }

    #[test]
    fn all_player_labels_keep_contrast_over_the_brightest_backdrop() {
        let p = Palette::get();
        let rgb = |color: Color32| {
            let [r, g, b, _] = color.to_srgba_unmultiplied();
            [r, g, b].map(|value| f64::from(value) / 255.0)
        };
        let alpha = f64::from(p.panel.a()) / 255.0;
        assert!(alpha > 0.7 && alpha < 0.95);
        // White maximizes every RGB channel after compositing.
        let bright_scene = rgb(p.panel).map(|value| value * alpha + (1.0 - alpha));
        for (foreground, background) in [
            (p.muted, bright_scene),
            (p.text, bright_scene),
            (p.text, rgb(p.button)),
            (p.text, rgb(p.primary)),
        ] {
            let ratio = (luminance(rgb(foreground)) + 0.05) / (luminance(background) + 0.05);
            assert!(ratio > 4.5, "player text contrast: {ratio:.2}");
        }
    }
}
