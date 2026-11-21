//! Desktop presentation of the Samsung shell, with native playback unchanged.
use crate::{
    app::{App, View},
    theme, ui,
};
use egui::{
    pos2, vec2, Align, Align2, Color32, Context, FontId, Id, LayerId, Layout, Order, Rect, Sense,
    Ui,
};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub reference_width: f32,
    pub navigation_expanded: f32,
    pub navigation_edge: f32,
    pub live_preview_edge: f32,
    pub navigation_collapsed: f32,
    pub category_width: f32,
    pub live_preview_width: f32,
    pub catalog_preview_width: f32,
    pub navigation_row_height: f32,
    pub navigation_font: f32,
    pub heading_font: f32,
    pub catalog_heading_font: f32,
    pub channel_row_height: f32,
    pub channel_name_font: f32,
    pub epg_font: f32,
    pub navigation_duration_ms: f32,
    pub navigation_hover_close_ms: f64,
    pub refresh_feedback_ms: f32,
    pub refresh_spin_ms: f32,
    pub remote_shortcuts: [RemoteShortcut; 4],
    pub navigation_easing: [f32; 4],
    pub player_top_height: f32,
    pub player_controls_height: f32,
    pub player_top_inset_x: f32,
    pub player_top_inset_y: f32,
    pub player_controls_inset_x: f32,
    pub player_controls_inset_y: f32,
    pub player_panel_color: String,
    pub player_button_color: String,
    pub player_primary_color: String,
    pub player_focus_color: String,
    pub player_text_color: String,
    pub player_muted_color: String,
    pub player_top_max_width: f32,
    pub player_controls_max_width: f32,
    pub player_border_color: String,
    pub player_button_height: f32,
    pub player_button_font: f32,
    pub player_popup_padding: f32,
    pub player_popup_gap: f32,
    pub player_popup_offset: f32,
    pub player_corner_radius: f32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteShortcut {
    key: String,
    color: String,
    de: String,
    en: String,
    player_de: Option<String>,
    player_en: Option<String>,
}
pub fn appearance() -> &'static Appearance {
    static A: OnceLock<Appearance> = OnceLock::new();
    A.get_or_init(|| {
        serde_json::from_str(include_str!("../../shared/shell-appearance.json"))
            .expect("bundled shell appearance")
    })
}
pub fn active(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(Id::new("shared-landscape-shell")))
        .unwrap_or_else(|| cfg!(windows) && !ui::mobile::active(ctx))
}
pub fn scale(ctx: &Context) -> f32 {
    let screen = safe_rect(ctx, false);
    (screen.width() / appearance().reference_width)
        .min(screen.height() / 840.0)
        .clamp(0.28, 1.25)
}
pub(super) fn safe_rect(ctx: &Context, keyboard: bool) -> Rect {
    let r = ctx.screen_rect();
    let i = crate::platform::insets();
    let ppp = ctx.pixels_per_point();
    let bottom = if keyboard {
        i.bottom
    } else {
        (i.bottom - i.ime).max(0.0)
    };
    Rect::from_min_max(
        pos2(r.left() + i.left / ppp, r.top() + i.top / ppp),
        pos2(
            (r.right() - i.right / ppp).max(r.left() + i.left / ppp + 1.0),
            (r.bottom() - bottom / ppp).max(r.top() + i.top / ppp + 1.0),
        ),
    )
}
pub(super) fn handheld() -> bool {
    cfg!(any(target_os = "android", target_os = "ios")) && !crate::platform::tv_device()
}

pub(super) fn handheld_type(size: f32, minimum: f32) -> f32 {
    if handheld() {
        (size * 1.14).max(minimum)
    } else {
        size
    }
}
pub(crate) fn category_width(width: f32, scale: f32) -> f32 {
    (appearance().category_width * scale).min(width * 0.33)
}

pub(super) fn search_field(
    ui: &mut Ui,
    value: &mut String,
    hint: &str,
    remote: bool,
) -> egui::Response {
    let height = (56.0 * scale(ui.ctx())).max(26.0);
    let response = ui.add_sized(
        [ui.available_width().max(1.0), height],
        egui::TextEdit::singleline(value).hint_text(hint),
    );
    if remote && ui::mobile::focus::clicked(ui, &response) {
        response.request_focus();
    }
    if response.gained_focus() {
        crate::platform::set_input_type(1);
        crate::platform::seed_ime(value);
    }
    #[cfg(target_os = "ios")]
    {
        if let Some(edited) = crate::platform::ios::bridge::edited(response.id.value()) {
            *value = edited;
        }
        crate::platform::ios::bridge::edit(ui, &response, hint, value, 0);
    }
    response
}

pub(super) fn shortcut_legend(
    painter: &egui::Painter,
    screen: Rect,
    scale: f32,
    language: crate::types::AppLanguage,
    text: Color32,
    background: Option<Color32>,
    show_key: bool,
) -> Rect {
    let s = scale.min(screen.width() / 520.0);
    let labels = appearance().remote_shortcuts.each_ref().map(|shortcut| {
        let label = if background.is_some() {
            language.text(
                shortcut.player_de.as_deref().unwrap_or(&shortcut.de),
                shortcut.player_en.as_deref().unwrap_or(&shortcut.en),
            )
        } else {
            language.text(&shortcut.de, &shortcut.en)
        };
        let galley = painter.layout_no_wrap(
            if show_key {
                format!("{}  {label}", shortcut.key)
            } else {
                label.into()
            },
            FontId::proportional(16.0 * s),
            text,
        );
        (
            galley,
            Color32::from_hex(&shortcut.color).expect("bundled shortcut color"),
        )
    });
    let gap = 18.0 * s;
    let width = labels
        .iter()
        .map(|(label, _)| label.size().x + 14.0 * s)
        .sum::<f32>()
        + 3.0 * gap
        + 24.0 * s;
    let bounds = Rect::from_center_size(
        pos2(screen.center().x, screen.bottom() - 17.0 * s),
        vec2(width, 24.0 * s),
    );
    if let Some(fill) = background {
        painter.rect_filled(bounds, 12.0 * s, fill);
    }
    let mut x = bounds.left() + 12.0 * s;
    for (label, color) in labels {
        painter.circle_filled(pos2(x + 4.0 * s, bounds.center().y), 4.0 * s, color);
        let size = label.size();
        painter.galley(
            pos2(x + 14.0 * s, bounds.center().y - size.y / 2.0),
            label,
            text,
        );
        x += size.x + 14.0 * s + gap;
    }
    bounds
}

fn ease(x: f32) -> f32 {
    let [x1, y1, x2, y2] = appearance().navigation_easing;
    let curve = |t: f32, a: f32, b: f32| {
        3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t * t * b + t.powi(3)
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..16 {
        let t = (lo + hi) * 0.5;
        if curve(t, x1, x2) < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    curve((lo + hi) * 0.5, y1, y2)
}
#[derive(Clone, Copy)]
struct Motion {
    target: f32,
    from: f32,
    started: f64,
}
impl Motion {
    fn offset(self, now: f64) -> f32 {
        let t = ((now - self.started) * 1000.0 / f64::from(appearance().navigation_duration_ms))
            .clamp(0.0, 1.0) as f32;
        if t >= 1.0 {
            0.0
        } else {
            self.from * (1.0 - ease(t))
        }
    }
    fn retarget(&mut self, target: f32, now: f64) {
        if (target - self.target).abs() > 0.1 {
            self.from = self.target + self.offset(now) - target;
            self.target = target;
            self.started = now;
        }
    }
}
fn motion(ctx: &Context, target: f32) -> f32 {
    let now = ctx.input(|i| i.time);
    let id = Id::new("desktop-shell-motion");
    let mut m = ctx.data(|d| d.get_temp::<Motion>(id)).unwrap_or(Motion {
        target,
        from: 0.0,
        started: now,
    });
    if reduced_motion(ctx) {
        m = Motion {
            target,
            from: 0.0,
            started: now,
        };
    } else {
        m.retarget(target, now);
    }
    let offset = m.offset(now);
    if offset.abs() > 0.05 {
        ctx.request_repaint();
    }
    ctx.data_mut(|d| {
        d.insert_temp(id, m);
        d.insert_temp(Id::new("desktop-column-offset"), offset);
    });
    offset
}
pub fn reduced_motion(ctx: &Context) -> bool {
    #[cfg(target_os = "windows")]
    {
        let now = ctx.input(|i| i.time);
        let id = Id::new("desktop-reduced-motion");
        if let Some((when, value)) = ctx.data(|d| d.get_temp::<(f64, bool)>(id)) {
            if now - when < 1.0 {
                return value;
            }
        }
        let value = crate::platform::windows::sys::reduced_motion();
        ctx.data_mut(|d| d.insert_temp(id, (now, value)));
        value
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = ctx;
        false
    }
}
pub fn offset(ctx: &Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(Id::new("desktop-column-offset")))
        .unwrap_or(0.0)
}
pub fn heading(ui: &mut Ui, text: &str) -> Rect {
    let s = scale(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0 * s), Sense::hover());
    ui::mobile::components::draw_line(
        ui,
        pos2(r.left(), r.center().y),
        text,
        ui::mobile::tokens::bold(handheld_type(appearance().heading_font * s, 16.0)),
        ui.visuals().text_color(),
        r.width(),
    );
    r
}
pub fn catalog_heading(ui: &mut Ui, text: &str) -> Rect {
    let s = scale(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0 * s), Sense::hover());
    ui::mobile::components::draw_line(
        ui,
        pos2(r.left(), r.center().y),
        text,
        ui::mobile::tokens::bold(handheld_type(appearance().catalog_heading_font * s, 13.0)),
        ui.visuals().text_color(),
        r.width(),
    );
    r
}
/// FLIP: calculate layout once at its destination, then translate its paint/input
/// layer. Text is never scaled and a reversal starts at the current position.
pub fn column<R>(
    ui: &mut Ui,
    rect: Rect,
    name: &str,
    dx: f32,
    render: impl FnOnce(&mut Ui) -> R,
) -> R {
    let layer = LayerId::new(Order::Middle, Id::new(("desktop-column", ui.id(), name)));
    ui.ctx().set_transform_layer(
        layer,
        egui::emath::TSTransform::from_translation(vec2(dx, 0.0)),
    );
    egui::Area::new(layer.id)
        .order(Order::Middle)
        .fixed_pos(rect.min)
        .constrain(false)
        .show(ui.ctx(), |area| {
            area.set_min_size(rect.size());
            // The rect is already known: skip Area's invisible measurement
            // frame so a freshly opened catalogue paints immediately.
            let content = if name.ends_with("categories")
                || name == "live-channels"
                || name == "catalog-posters"
            {
                rect.shrink(6.0 * scale(ui.ctx()))
            } else {
                rect
            };
            let mut child = Ui::new(
                ui.ctx().clone(),
                layer,
                layer.id,
                content,
                rect.intersect(ui.clip_rect()),
            );
            render(&mut child)
        })
        .inner
}
fn style(ctx: &Context, s: f32) {
    let minimum_control_height = if handheld() { 28.0 } else { 23.0 };
    ctx.style_mut(|style| {
        style.spacing.item_spacing = vec2(12.0 * s, 12.0 * s);
        style.spacing.interact_size.y = (48.0 * s).max(minimum_control_height);
        style.spacing.button_padding = vec2(16.0 * s, 9.0 * s);
        style.animation_time = appearance().navigation_duration_ms / 1000.0;
        for (kind, size) in [
            (egui::TextStyle::Body, 22.0),
            (egui::TextStyle::Button, 22.0),
            (egui::TextStyle::Small, 18.0),
            (egui::TextStyle::Heading, 32.0),
        ] {
            style.text_styles.insert(
                kind.clone(),
                if kind == egui::TextStyle::Heading {
                    ui::mobile::tokens::bold(handheld_type((size * s).max(12.0), 16.0))
                } else {
                    FontId::proportional(handheld_type((size * s).max(11.0), 12.0))
                },
            );
        }
    });
}
#[derive(Clone, Copy, Default)]
enum NavigationHover {
    #[default]
    Closed,
    Inside,
    Leaving(f64),
}

impl NavigationHover {
    fn update(self, inside: bool, now: f64) -> Self {
        if inside {
            return Self::Inside;
        }
        match self {
            Self::Inside => Self::Leaving(now + appearance().navigation_hover_close_ms / 1000.0),
            Self::Leaving(deadline) if now < deadline => self,
            _ => Self::Closed,
        }
    }
}

fn navigation_is_collapsed(app: &App, ctx: &Context, compact: Rect, expanded: Rect) -> bool {
    let id = Id::new("desktop-navigation-hover");
    let previous = ctx
        .data(|data| data.get_temp::<NavigationHover>(id))
        .unwrap_or_default();
    let next = if app.nav_collapsed {
        let bounds = if matches!(previous, NavigationHover::Closed) {
            compact
        } else {
            expanded
        };
        let (inside, now) = ctx.input(|input| {
            (
                input
                    .pointer
                    .hover_pos()
                    .is_some_and(|pos| bounds.contains(pos)),
                input.time,
            )
        });
        let next = previous.update(inside, now);
        if let NavigationHover::Leaving(deadline) = next {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(deadline - now));
        }
        next
    } else {
        NavigationHover::Closed
    };
    ctx.data_mut(|data| data.insert_temp(id, next));
    app.nav_collapsed && matches!(next, NavigationHover::Closed)
}

pub(crate) fn remember_favorite_target(response: &egui::Response, key: &str) {
    if response.has_focus() || ui::mobile::focus::focused_id() == Some(response.id) {
        response.ctx.data_mut(|data| {
            data.insert_temp(
                Id::new("desktop-favorite-target"),
                (response.id, key.to_owned()),
            );
        });
    }
}

#[cfg(windows)]
pub(crate) fn handle_color_shortcut(app: &mut App, ctx: &Context) -> bool {
    let Some(key) = ctx.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
                ..
            } if matches!(
                key,
                egui::Key::Escape | egui::Key::F1 | egui::Key::F2 | egui::Key::F3 | egui::Key::F4
            ) =>
            {
                Some(*key)
            }
            _ => None,
        })
    }) else {
        return false;
    };
    if crate::licensing::enabled() && !crate::licensing::allowed() {
        return false;
    }
    color_shortcut(app, ctx, key)
}

pub(crate) fn color_shortcut(app: &mut App, ctx: &Context, key: egui::Key) -> bool {
    let player = app.view == View::Player;
    if !player && (app.active_profile.is_none() || !active(ctx)) {
        return false;
    }
    if app.parental.open || app.pending_resume.is_some() || app.description_open.is_some() {
        if matches!(key, egui::Key::Escape | egui::Key::F1) {
            if app.parental.open {
                app.parental.close();
            } else {
                app.clear_resume();
                app.description_open = None;
            }
        }
        return true;
    }
    let focus_navigation = |app: &mut App| {
        app.nav_collapsed = false;
        ctx.data_mut(|data| data.insert_temp(Id::new("desktop-navigation-focus"), true));
    };
    let exit = Id::new("desktop-exit-prompt");
    if ctx
        .data(|data| data.get_temp::<bool>(exit))
        .unwrap_or(false)
    {
        if matches!(key, egui::Key::Escape | egui::Key::F1) {
            ctx.data_mut(|data| data.remove::<bool>(exit));
        }
        return true;
    }
    match key {
        egui::Key::Escape | egui::Key::F1 => {
            if player {
                if !super::player_desktop::dismiss_menus(app, ctx) {
                    app.back_from_player();
                }
            } else {
                let focused = ui::mobile::focus::focused_id()
                    .or_else(|| ctx.memory(|memory| memory.focused()));
                let nav_focus =
                    ctx.data(|data| data.get_temp::<Id>(Id::new("desktop-nav-focused-id")));
                if nav_focus.is_some() && focused == nav_focus {
                    ctx.data_mut(|data| data.insert_temp(exit, true));
                } else {
                    focus_navigation(app);
                }
            }
        }
        egui::Key::F2 => {
            if player {
                app.back_from_player();
                if app.view == View::Live {
                    return true;
                }
                app.desktop_return_item = None;
            }
            app.navigate(View::Live);
            focus_navigation(app);
        }
        egui::Key::F3 => {
            let focused =
                ui::mobile::focus::focused_id().or_else(|| ctx.memory(|memory| memory.focused()));
            let target = if player {
                app.current_play
                    .as_ref()
                    .map(|item| item.progress_key.clone())
            } else {
                ctx.data(|data| data.get_temp::<(Id, String)>(Id::new("desktop-favorite-target")))
                    .filter(|(id, _)| Some(*id) == focused)
                    .map(|(_, key)| key)
                    .or_else(|| match app.view {
                        View::Live => app.live_selected.map(|id| format!("live:{id}")),
                        View::Movies => app.selected_movie.map(|id| format!("vod:{id}")),
                        View::Series => app.selected_series.map(|id| format!("series:{id}")),
                        _ => None,
                    })
            };
            if let Some(target) = target {
                app.toggle_favorite(&target);
                if player {
                    app.ui_activity = ctx.input(|input| input.time);
                    app.player_reveal = false;
                }
            }
        }
        egui::Key::F4 => {
            if player {
                super::player_desktop::toggle_options_shortcut(app, ctx);
            } else {
                app.navigate(View::Settings);
                focus_navigation(app);
            }
        }
        _ => unreachable!(),
    }
    ctx.request_repaint();
    true
}

pub(crate) fn exit_prompt(ctx: &Context) -> bool {
    ctx.data(|data| {
        data.get_temp::<bool>(Id::new("desktop-exit-prompt"))
            .unwrap_or(false)
    })
}

pub fn show(app: &mut App, ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(Id::new("shared-landscape-shell"), true));
    if app.view == View::Player && app.current_play.is_some() {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                super::player_desktop::show(app, ui);
            });
        return;
    }
    if app.active_profile.is_none() {
        if cfg!(any(target_os = "android", target_os = "ios")) {
            ui::mobile::show_landscape_login(app, ctx);
        } else {
            ui::login::show(app, ctx);
        }
        return;
    }
    let a = appearance();
    let s = scale(ctx);
    style(ctx, s);
    let screen = safe_rect(ctx, true);
    let background = ctx.screen_rect();
    let expanded = a.navigation_expanded * s;
    let collapsed = a.navigation_collapsed * s;
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && !ctx.wants_keyboard_input() {
        app.nav_collapsed = false;
    }
    let left = screen.left() + a.navigation_edge * s;
    let rail = |width| {
        Rect::from_min_max(
            pos2(left, screen.top()),
            pos2(left + width, screen.bottom()),
        )
    };
    let is_collapsed = navigation_is_collapsed(app, ctx, rail(collapsed), rail(expanded));
    let target = if is_collapsed { collapsed } else { expanded };
    let dx = motion(ctx, target);
    let visual = target + dx;
    let pad = 24.0 * s;
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            let backdrop = ui.painter().add(egui::Shape::Noop);
            let nav = rail(visual);
            ui.painter().rect_filled(
                Rect::from_min_max(screen.min, nav.max),
                0.0,
                Color32::from_black_alpha(24),
            );
            ui::mobile::focus::set_zone(0);
            navigation(
                app,
                ui,
                nav,
                s,
                (visual - collapsed) / (expanded - collapsed),
                is_collapsed,
            );
            let body = Rect::from_min_max(
                pos2(left + target + pad, screen.top() + pad),
                pos2(
                    screen.right()
                        - if app.view == View::Live {
                            a.live_preview_edge * s
                        } else {
                            pad
                        },
                    (screen.bottom() - 30.0 * s).max(screen.top() + pad + 1.0),
                ),
            );
            let view = app.view;
            ui::mobile::focus::set_zone(1);
            let preview = if view == View::Live {
                ui::live::show_desktop(app, ui, body, s, dx)
            } else {
                column(
                    ui,
                    body,
                    "content",
                    if matches!(view, View::Movies | View::Series) {
                        0.0
                    } else {
                        dx
                    },
                    |ui| {
                        ui.push_id(("section", view as u64), |ui| match view {
                            View::Home => ui::home::show(app, ui),
                            View::Movies => ui::vod::show(app, ui),
                            View::Series => ui::series::show(app, ui),
                            View::Search => ui::search::show(app, ui),
                            View::Favorites => ui::favorites::show(app, ui),
                            View::Continue => ui::continue_watching::show(app, ui),
                            View::Settings => ui::settings::show(app, ui),
                            _ => {}
                        });
                    },
                );
                None
            };
            ui.painter().set(
                backdrop,
                theme::ambient_shape(ui, background, preview.filter(|_| cfg!(target_os = "ios"))),
            );
            if cfg!(windows) || app.tv {
                shortcut_legend(
                    ui.painter(),
                    screen,
                    s,
                    app.settings.language,
                    theme::muted(ui),
                    None,
                    cfg!(windows),
                );
            }
        });
    if exit_prompt(ctx) {
        let mut dismiss = false;
        let mut quit = false;
        egui::Window::new(
            app.settings
                .language
                .text("Dado Player beenden?", "Quit Dado Player?"),
        )
        .id(Id::new("desktop-exit-dialog"))
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            if app.tv {
                ui::mobile::focus::begin_modal(0);
            }
            ui.horizontal(|ui| {
                let keep = ui.button(app.settings.language.text("Weitersehen", "Keep watching"));
                let stop = ui.button(app.settings.language.text("Beenden", "Quit"));
                dismiss = if app.tv {
                    ui::mobile::focus::clicked(ui, &keep)
                } else {
                    keep.clicked()
                };
                quit = if app.tv {
                    ui::mobile::focus::clicked(ui, &stop)
                } else {
                    stop.clicked()
                };
            });
        });
        if dismiss || quit {
            ctx.data_mut(|data| data.remove::<bool>(Id::new("desktop-exit-prompt")));
        }
        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
fn refresh_icon(app: &App, ui: &Ui, rect: Rect, color: Color32, id: Id) {
    let a = appearance();
    let feedback = app.catalog_refresh_result.and_then(|(success, at)| {
        let remaining = a.refresh_feedback_ms / 1000.0 - at.elapsed().as_secs_f32();
        (remaining > 0.0).then(|| {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f32(remaining));
            success
        })
    });
    let motion = !reduced_motion(ui.ctx());
    let transition = if motion { 0.24 } else { 0.0 };
    let success = ui.ctx().animate_bool_with_time(
        id.with("refresh-success"),
        feedback == Some(true),
        transition,
    );
    let error = ui.ctx().animate_bool_with_time(
        id.with("refresh-error"),
        feedback == Some(false),
        transition,
    );
    let result = success.max(error);
    let time = ui.input(|input| input.time);
    let angle = if motion && app.catalog_refreshing {
        ui.ctx().request_repaint();
        (time * 1000.0 / f64::from(a.refresh_spin_ms) * std::f64::consts::TAU) as f32
    } else {
        0.0
    };
    let origin = rect.center();
    let size = rect.width() / 24.0;
    let draw = |points: &[(f32, f32)], angle: f32, scale: f32, tint: Color32| {
        let (sin, cos) = angle.sin_cos();
        let points = points
            .iter()
            .map(|&(x, y)| origin + vec2(cos * x - sin * y, sin * x + cos * y) * (size * scale))
            .collect();
        ui.painter().add(egui::Shape::line(
            points,
            egui::Stroke::new(1.8 * size, tint),
        ));
    };
    if result < 0.999 {
        let arc: Vec<_> = (0..=24)
            .map(|i| {
                let a = -0.46 - i as f32 * 5.36 / 24.0;
                (8.0 * a.cos(), 8.0 * a.sin())
            })
            .collect();
        let tint = color.gamma_multiply(1.0 - result);
        let rotation = angle + result * 0.61;
        draw(&arc, rotation, 1.0 - 0.45 * result, tint);
        draw(
            &[(8.0, -9.0), (8.0, -4.0), (3.0, -4.0)],
            rotation,
            1.0 - 0.45 * result,
            tint,
        );
    }
    if success > 0.001 {
        draw(
            &[(-7.0, 0.0), (-2.5, 4.5), (7.0, -5.0)],
            -0.44 * (1.0 - success),
            0.55 + 0.45 * success,
            Color32::from_rgb(133, 217, 160).gamma_multiply(success),
        );
    }
    if error > 0.001 {
        let tint = Color32::from_rgb(255, 155, 155).gamma_multiply(error);
        let rotation = -0.44 * (1.0 - error);
        let scale = 0.55 + 0.45 * error;
        draw(&[(-6.0, -6.0), (6.0, 6.0)], rotation, scale, tint);
        draw(&[(6.0, -6.0), (-6.0, 6.0)], rotation, scale, tint);
    }
}

fn navigation(app: &mut App, ui: &mut Ui, rect: Rect, s: f32, visibility: f32, is_collapsed: bool) {
    use ui::mobile::components::{icon, Icon};
    let mut nav = ui.child_ui_with_id_source(
        rect,
        Layout::top_down(Align::Min),
        "samsung-desktop-navigation",
    );
    nav.set_clip_rect(rect);
    let text = nav.visuals().text_color();
    let muted = theme::muted(&nav);
    let label_alpha = visibility.clamp(0.0, 1.0);
    let focus_requested = ui.data_mut(|data| {
        let id = Id::new("desktop-navigation-focus");
        let requested = data.get_temp::<bool>(id).unwrap_or(false);
        data.remove::<bool>(id);
        requested
    });
    let mark = Rect::from_center_size(
        pos2(rect.left() + 40.0 * s, rect.top() + 53.0 * s),
        vec2(44.0 * s, 44.0 * s),
    );
    let mut mark_ui = nav.child_ui(mark, Layout::top_down(Align::Min));
    ui::brand_mark(&mut mark_ui, 44.0 * s);
    if handheld() {
        let row = Rect::from_center_size(mark.center(), Vec2::splat(70.0 * s));
        if nav
            .interact(
                row,
                nav.id().with("toggle-handheld-navigation"),
                Sense::click(),
            )
            .clicked()
        {
            app.nav_collapsed = !app.nav_collapsed;
            nav.ctx().request_repaint();
        }
    }
    if label_alpha > 0.01 {
        nav.painter().text(
            pos2(rect.left() + 76.0 * s, rect.top() + 53.0 * s),
            Align2::LEFT_CENTER,
            "Dado Player",
            ui::mobile::tokens::bold(27.0 * s),
            text.gamma_multiply(label_alpha),
        );
    }
    #[cfg(target_os = "windows")]
    {
        let badge = Rect::from_min_max(
            pos2(rect.left() + 8.0 * s, rect.top() + 82.0 * s),
            pos2(rect.right() - 8.0 * s, rect.top() + 104.0 * s),
        );
        let mut b = nav.child_ui(badge, Layout::top_down(Align::Min));
        crate::licensing::trial_badge(&mut b, is_collapsed);
    }
    let list_rect = Rect::from_min_max(
        pos2(rect.left() + 8.0 * s, rect.top() + 112.0 * s),
        pos2(rect.right() - 8.0 * s, rect.bottom() - 140.0 * s),
    );
    let mut list = nav.child_ui(list_rect, Layout::top_down(Align::Min));
    list.set_clip_rect(list_rect.intersect(rect));
    egui::ScrollArea::vertical()
        .id_source("desktop-navigation-list")
        .auto_shrink([false, false])
        .show(&mut list, |nav| {
            nav.spacing_mut().item_spacing.y = 8.0 * s;
            let rows = [
                (
                    app.settings.language.text("Startseite", "Home"),
                    View::Home,
                    Icon::Home,
                ),
                ("Live TV", View::Live, Icon::Tv),
                (
                    app.settings.language.text("Filme", "Movies"),
                    View::Movies,
                    Icon::Film,
                ),
                (
                    app.settings.language.text("Serien", "Series"),
                    View::Series,
                    Icon::Series,
                ),
                (
                    app.settings.language.text("Fortsetzen", "Continue"),
                    View::Continue,
                    Icon::Play,
                ),
                (
                    app.settings.language.text("Favoriten", "Favorites"),
                    View::Favorites,
                    Icon::Star,
                ),
                (
                    app.settings.language.text("Suche", "Search"),
                    View::Search,
                    Icon::Search,
                ),
                (
                    app.settings.language.text("Einstellungen", "Settings"),
                    View::Settings,
                    Icon::Settings,
                ),
            ];
            let language = app.settings.language;
            let refresh_label = language.text("Liste aktualisieren", "Refresh library");
            let spacing = nav.spacing().item_spacing.y;
            let total_rows = (rows.len() + 1) as f32;
            let row_height = (appearance().navigation_row_height * s - spacing)
                .min(((list_rect.height() + spacing) / total_rows - spacing).max(23.0));
            for (label, view, symbol) in rows
                .into_iter()
                .map(|(label, view, icon)| (label, Some(view), icon))
                .chain(std::iter::once((refresh_label, None, Icon::Refresh)))
            {
                let (r, _) = nav
                    .allocate_exact_size(vec2(nav.available_width(), row_height), Sense::hover());
                let response = nav.interact(
                    r,
                    nav.id().with(view.map(|view| view as u64)),
                    Sense::click(),
                );
                let selected = view == Some(app.view);
                if focus_requested && selected {
                    response.request_focus();
                    if app.tv {
                        ui::mobile::focus::request_id(response.id);
                    }
                    response.scroll_to_me(None);
                }
                if response.has_focus() || ui::mobile::focus::focused_id() == Some(response.id) {
                    nav.data_mut(|data| {
                        data.insert_temp(Id::new("desktop-nav-focused-id"), response.id)
                    });
                }
                let color = if selected { theme::accent(nav) } else { text };
                let hover = nav.ctx().animate_bool_with_time(
                    response.id.with("hover"),
                    selected || response.hovered() || response.has_focus(),
                    if reduced_motion(nav.ctx()) { 0.0 } else { 0.12 },
                );
                if hover > 0.0 {
                    nav.painter().rect_filled(
                        r,
                        16.0 * s,
                        (if selected {
                            theme::selection_surface(nav)
                        } else {
                            theme::card_hover(nav)
                        })
                        .gamma_multiply(hover),
                    );
                }
                let icon_rect = Rect::from_center_size(
                    pos2(rect.left() + 40.0 * s, r.center().y),
                    vec2(30.0 * s, 30.0 * s),
                );
                if view.is_none() {
                    refresh_icon(app, nav, icon_rect, color, response.id);
                } else {
                    icon(nav, icon_rect, symbol, color);
                }
                if label_alpha > 0.01 {
                    let label_x = rect.left() + 70.0 * s;
                    ui::mobile::components::draw_line(
                        nav,
                        pos2(label_x, r.center().y),
                        label,
                        FontId::proportional((appearance().navigation_font * s).max(11.0)),
                        color.gamma_multiply(label_alpha),
                        (r.right() - label_x - 8.0 * s).max(0.0),
                    );
                }
                let help = if view.is_none() {
                    if app.catalog_refreshing {
                        language.text("Wird aktualisiert …", "Refreshing …")
                    } else if app.catalog_refresh_result.is_some_and(|(_, at)| {
                        at.elapsed().as_millis() < appearance().refresh_feedback_ms as u128
                    }) {
                        app.catalog_refresh_notice.as_deref().unwrap_or(label)
                    } else {
                        label
                    }
                } else {
                    label
                };
                let response = response.on_hover_text(help);
                if if app.tv {
                    ui::mobile::focus::clicked(nav, &response)
                } else {
                    response.clicked()
                } {
                    app.nav_collapsed = false;
                    if let Some(view) = view {
                        app.view = view;
                    } else {
                        app.refresh_catalog();
                    }
                }
            }
        });
    if label_alpha > 0.01 {
        let name = app
            .active_profile
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("");
        let footer = Rect::from_min_max(
            pos2(rect.left() + 20.0 * s, rect.bottom() - 130.0 * s),
            pos2(rect.right() - 12.0 * s, rect.bottom() - 12.0 * s),
        );
        let mut footer_ui = nav.child_ui(footer, Layout::top_down(Align::Min));
        footer_ui.label(
            egui::RichText::new(name)
                .color(muted.gamma_multiply(label_alpha))
                .size(18.0 * s),
        );
        footer_ui.label(
            egui::RichText::new(format!("Version {}", crate::release_info::VERSION))
                .color(muted.gamma_multiply(label_alpha))
                .size(16.0 * s),
        );
        footer_ui.horizontal(|ui| {
            if ui.small_button("Abmelden").clicked() {
                app.logout();
            }
            if ui
                .small_button("⌕")
                .on_hover_text("Schnellsuche · Strg+K")
                .clicked()
            {
                app.nav_collapsed = false;
                app.palette_open = true;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_live_catalogue_keeps_layout_work_bounded_while_scrolling() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let mut app = App::new(rt);
        app.library = crate::demo::library();
        let template = app.library.channels[0].clone();
        app.library.channels = (0..20_000)
            .map(|i| {
                let mut c = template.clone();
                c.id = i;
                c.name = format!("Live Probe {i}");
                c.logo.clear();
                c
            })
            .collect();
        app.active_profile = Some(crate::types::Profile::new(
            "Fixture",
            crate::types::ProfileKind::M3u,
        ));
        app.view = View::Live;
        app.live_cat = "all".into();
        for (width, height) in [(1024.0, 640.0), (1440.0, 900.0), (1920.0, 1080.0)] {
            let ctx = Context::default();
            theme::apply(&ctx, false);
            theme::apply_warm(&ctx);
            for frame in 0..8 {
                super::super::live::take_row_count();
                let out = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(width, height),
                        )),
                        time: Some(frame as f64 / 120.0),
                        events: if frame == 0 {
                            vec![]
                        } else {
                            vec![
                                egui::Event::PointerMoved(pos2(width * 0.5, height * 0.5)),
                                egui::Event::Scroll(vec2(0.0, -240.0)),
                            ]
                        },
                        ..Default::default()
                    },
                    |ctx| show(&mut app, ctx),
                );
                let count = super::super::live::take_row_count();
                assert!(count > 0 && count < 100, "{count} rows laid out from 20000");
                assert!(!out.shapes.is_empty());
            }
        }
    }
    #[test]
    fn hovering_opens_temporarily_and_reentry_cancels_the_collapse() {
        let now = 1.0;
        let delay = appearance().navigation_hover_close_ms / 1000.0;
        let open = NavigationHover::Closed.update(true, now);
        assert!(matches!(open, NavigationHover::Inside));
        assert!(matches!(
            open.update(true, now + 2.0 * delay),
            NavigationHover::Inside
        ));

        let leaving = open.update(false, now);
        assert!(matches!(
            leaving.update(false, now + delay * 0.5),
            NavigationHover::Leaving(_)
        ));
        assert!(matches!(
            leaving.update(false, now + delay),
            NavigationHover::Closed
        ));

        let reentered = leaving.update(true, now + delay * 0.5);
        let left_again = reentered.update(false, now + delay);
        assert!(matches!(
            left_again.update(false, now + delay * 1.5),
            NavigationHover::Leaving(_)
        ));
        assert!(matches!(
            left_again.update(false, now + delay * 2.01),
            NavigationHover::Closed
        ));
    }
    #[test]
    fn reversing_navigation_keeps_the_current_visual_position_and_settles() {
        let mut motion = Motion {
            target: 276.0,
            from: 0.0,
            started: 0.0,
        };
        motion.retarget(80.0, 1.0);
        assert!((motion.target + motion.offset(1.0) - 276.0).abs() < 0.1);
        let before = motion.target + motion.offset(1.06);
        motion.retarget(276.0, 1.06);
        assert!((motion.target + motion.offset(1.06) - before).abs() < 0.1);
        assert_eq!(motion.offset(1.4), 0.0);
    }
}
