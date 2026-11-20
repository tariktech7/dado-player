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
    pub navigation_easing: [f32; 4],
    pub player_top_height: f32,
    pub player_controls_height: f32,
    #[cfg(target_os = "windows")]
    pub player_top_inset_x: f32,
    #[cfg(target_os = "windows")]
    pub player_top_inset_y: f32,
    #[cfg(target_os = "windows")]
    pub player_controls_inset_x: f32,
    #[cfg(target_os = "windows")]
    pub player_controls_inset_y: f32,
    #[cfg(target_os = "windows")]
    pub player_panel_color: String,
    #[cfg(target_os = "windows")]
    pub player_button_color: String,
    #[cfg(target_os = "windows")]
    pub player_primary_color: String,
    #[cfg(target_os = "windows")]
    pub player_focus_color: String,
    #[cfg(target_os = "windows")]
    pub player_text_color: String,
    #[cfg(target_os = "windows")]
    pub player_muted_color: String,
    #[cfg(target_os = "windows")]
    pub player_top_max_width: f32,
    #[cfg(target_os = "windows")]
    pub player_controls_max_width: f32,
    #[cfg(target_os = "windows")]
    pub player_border_color: String,
    #[cfg(target_os = "windows")]
    pub player_button_height: f32,
    #[cfg(target_os = "windows")]
    pub player_button_font: f32,
    #[cfg(target_os = "windows")]
    pub player_popup_padding: f32,
    #[cfg(target_os = "windows")]
    pub player_popup_gap: f32,
    #[cfg(target_os = "windows")]
    pub player_popup_offset: f32,
    pub player_corner_radius: f32,
}
pub fn appearance() -> &'static Appearance {
    static A: OnceLock<Appearance> = OnceLock::new();
    A.get_or_init(|| {
        serde_json::from_str(include_str!("../../shared/shell-appearance.json"))
            .expect("bundled shell appearance")
    })
}
pub fn active(ctx: &Context) -> bool {
    cfg!(target_os = "windows") && !ui::mobile::active(ctx)
}
pub fn scale(ctx: &Context) -> f32 {
    (ctx.screen_rect().width() / appearance().reference_width).clamp(0.66, 1.25)
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
        ui::mobile::tokens::bold(appearance().heading_font * s),
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
        ui::mobile::tokens::bold(appearance().catalog_heading_font * s),
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
    ctx.style_mut(|style| {
        style.spacing.item_spacing = vec2(12.0 * s, 12.0 * s);
        style.spacing.interact_size.y = 48.0 * s;
        style.spacing.button_padding = vec2(16.0 * s, 9.0 * s);
        style.spacing.scroll.bar_width = 5.0;
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
                    ui::mobile::tokens::bold(size * s)
                } else {
                    FontId::proportional(size * s)
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

#[cfg(windows)]
pub(crate) fn remember_favorite_target(response: &egui::Response, key: &str) {
    if response.has_focus() {
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
                egui::Key::F1 | egui::Key::F2 | egui::Key::F3 | egui::Key::F4
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
    let player = app.view == View::Player;
    if !player && (app.active_profile.is_none() || !active(ctx)) {
        return false;
    }
    if app.pending_resume.is_some() || app.description_open.is_some() {
        if key == egui::Key::F1 {
            app.clear_resume();
            app.description_open = None;
        }
        return true;
    }
    let focus_navigation = |app: &mut App| {
        app.nav_collapsed = false;
        ctx.data_mut(|data| data.insert_temp(Id::new("desktop-navigation-focus"), true));
    };
    match key {
        egui::Key::F1 => {
            if player {
                if !super::player_desktop::dismiss_menus(app, ctx) {
                    app.stop_play();
                }
            } else {
                focus_navigation(app);
            }
        }
        egui::Key::F2 => {
            if player {
                app.stop_play();
                if app.view == View::Live {
                    return true;
                }
                app.desktop_return_item = None;
            }
            app.navigate(View::Live);
            focus_navigation(app);
        }
        egui::Key::F3 => {
            let focused = ctx.memory(|memory| memory.focused());
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

pub fn show(app: &mut App, ctx: &Context) {
    if app.view == View::Player && app.current_play.is_some() {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                #[cfg(target_os = "windows")]
                super::player_desktop::show(app, ui);
                #[cfg(not(target_os = "windows"))]
                ui::player::show(app, ui);
            });
        return;
    }
    if app.active_profile.is_none() {
        ui::login::show(app, ctx);
        return;
    }
    let a = appearance();
    let s = scale(ctx);
    style(ctx, s);
    let screen = ctx.screen_rect();
    let expanded = a.navigation_expanded * s;
    let collapsed = a.navigation_collapsed * s;
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && !ctx.wants_keyboard_input() {
        app.nav_collapsed = false;
    }
    let rail = |width| Rect::from_min_max(screen.min, pos2(screen.left() + width, screen.bottom()));
    let is_collapsed = navigation_is_collapsed(app, ctx, rail(collapsed), rail(expanded));
    let target = if is_collapsed { collapsed } else { expanded };
    let dx = motion(ctx, target);
    let visual = target + dx;
    let pad = 24.0 * s;
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(ctx.style().visuals.panel_fill))
        .show(ctx, |ui| {
            theme::ambient(ui, screen);
            let nav = Rect::from_min_max(screen.min, pos2(screen.left() + visual, screen.bottom()));
            ui.painter()
                .rect_filled(nav, 0.0, Color32::from_black_alpha(24));
            navigation(
                app,
                ui,
                nav,
                s,
                (visual - collapsed) / (expanded - collapsed),
                is_collapsed,
            );
            let body = Rect::from_min_max(
                pos2(screen.left() + target + pad, screen.top() + pad),
                pos2(screen.right() - pad, screen.bottom() - pad),
            );
            let view = app.view;
            if view == View::Live {
                ui::live::show_desktop(app, ui, body, s, dx);
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
            }
        });
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
            let refresh_label = if app.catalog_refreshing {
                language.text("Wird aktualisiert …", "Refreshing …")
            } else {
                language.text("Liste aktualisieren", "Refresh library")
            };
            for (label, view, symbol) in rows
                .into_iter()
                .map(|(label, view, icon)| (label, Some(view), icon))
                .chain(std::iter::once((refresh_label, None, Icon::Refresh)))
            {
                let (r, _) = nav.allocate_exact_size(
                    vec2(
                        nav.available_width(),
                        appearance().navigation_row_height * s - 8.0 * s,
                    ),
                    Sense::hover(),
                );
                let response = nav.interact(
                    r,
                    nav.id().with(view.map(|view| view as u64)),
                    Sense::click(),
                );
                let selected = view == Some(app.view);
                if focus_requested && selected {
                    response.request_focus();
                    response.scroll_to_me(None);
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
                icon(
                    nav,
                    Rect::from_center_size(
                        pos2(rect.left() + 40.0 * s, r.center().y),
                        vec2(30.0 * s, 30.0 * s),
                    ),
                    symbol,
                    color,
                );
                if label_alpha > 0.01 {
                    let label_x = rect.left() + 70.0 * s;
                    ui::mobile::components::draw_line(
                        nav,
                        pos2(label_x, r.center().y),
                        label,
                        FontId::proportional(if view.is_none() {
                            24.0 * s
                        } else {
                            appearance().navigation_font * s
                        }),
                        color.gamma_multiply(label_alpha),
                        (r.right() - label_x - 8.0 * s).max(0.0),
                    );
                }
                let help = if view.is_none() {
                    app.catalog_refresh_notice.as_deref().unwrap_or(label)
                } else {
                    label
                };
                if response.on_hover_text(help).clicked() {
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
            egui::RichText::new(format!("Windows {}", crate::release_info::VERSION))
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
