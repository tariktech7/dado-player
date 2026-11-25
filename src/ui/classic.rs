use super::{
    desktop,
    mobile::{
        components::{self, Icon},
        focus, tokens,
    },
    settings,
};
use crate::{
    app::{App, View},
    theme,
    types::InterfaceLayout,
};
use egui::{pos2, vec2, Align2, Color32, Id, Mesh, Rect, Rgba, Sense, Stroke, Ui};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TileColor {
    top: String,
    bottom: String,
    accent: String,
    icon_ink: String,
    icon_light: String,
    icon_shade: String,
}

fn tile_color(view: View) -> [Color32; 6] {
    static COLORS: OnceLock<HashMap<String, TileColor>> = OnceLock::new();
    let colors = COLORS.get_or_init(|| {
        serde_json::from_str(include_str!("../../shared/classic-tile-palette.json"))
            .expect("bundled tile colors")
    });
    let key = match view {
        View::Live => "live",
        View::Movies => "vod",
        _ => "series",
    };
    let color = &colors[key];
    [
        &color.top,
        &color.bottom,
        &color.accent,
        &color.icon_ink,
        &color.icon_light,
        &color.icon_shade,
    ]
    .map(|value| Color32::from_hex(value).expect("bundled tile color"))
}

fn tile_surface(ui: &Ui, rect: Rect, radius: f32, top: Color32, bottom: Color32) {
    let radius = radius.min(rect.height() * 0.5).min(rect.width() * 0.5);
    let mut mesh = Mesh::default();
    let mut rows: Vec<_> = (0..=8)
        .map(|step| egui::lerp(rect.top()..=rect.bottom(), step as f32 / 8.0))
        .collect();
    for step in 1..=12 {
        let angle = step as f32 * std::f32::consts::FRAC_PI_2 / 12.0;
        let offset = radius * (1.0 - angle.cos());
        rows.extend([rect.top() + offset, rect.bottom() - offset]);
    }
    rows.sort_by(f32::total_cmp);
    rows.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    for (step, y) in rows.into_iter().enumerate() {
        let fraction = (y - rect.top()) / rect.height();
        let corner = (radius - (y - rect.top()).min(rect.bottom() - y)).max(0.0);
        let inset = radius - (radius * radius - corner * corner).max(0.0).sqrt();
        let fill = Rgba::from(top) * (1.0 - fraction) + Rgba::from(bottom) * fraction;
        let glow = (1.0 - fraction).powi(3) * 0.14;
        let highlight = fill * (1.0 - glow) + Rgba::from_white_alpha(fill.a()) * glow;
        mesh.colored_vertex(pos2(rect.left() + inset, y), Color32::from(fill));
        mesh.colored_vertex(pos2(rect.center().x, y), Color32::from(highlight));
        mesh.colored_vertex(pos2(rect.right() - inset, y), Color32::from(fill));
        if step > 0 {
            let index = ((step - 1) * 3) as u32;
            for left in [index, index + 1] {
                mesh.add_triangle(left, left + 1, left + 3);
                mesh.add_triangle(left + 1, left + 4, left + 3);
            }
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

pub(super) fn list_surface(ui: &Ui, rect: Rect, selected: bool, highlighted: bool) {
    let radius = 9.0;
    let rect = rect.shrink(1.0);
    let fill = if selected || highlighted {
        theme::selection_surface(ui)
    } else {
        ui.visuals().window_fill
    };
    ui.painter().rect_filled(
        rect.translate(vec2(0.0, 2.0)),
        radius,
        Color32::from_black_alpha(37),
    );
    ui.painter()
        .rect(rect, radius, fill, ui.visuals().window_stroke);
    if selected || highlighted {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(
                if highlighted { 2.5_f32 } else { 1.3_f32 },
                theme::accent(ui),
            ),
        );
    }
}

pub(super) fn category_counts<'a>(
    ui: &Ui,
    generation: u64,
    categories: impl Iterator<Item = &'a str>,
) -> Arc<HashMap<String, usize>> {
    let id = ui.id().with("classic-category-counts");
    if let Some((previous, counts)) =
        ui.data(|data| data.get_temp::<(u64, Arc<HashMap<String, usize>>)>(id))
    {
        if previous == generation {
            return counts;
        }
    }
    let mut counts = HashMap::new();
    for category in categories {
        if let Some(count) = counts.get_mut(category) {
            *count += 1;
        } else {
            counts.insert(category.to_owned(), 1);
        }
    }
    let counts = Arc::new(counts);
    ui.data_mut(|data| data.insert_temp(id, (generation, counts.clone())));
    counts
}

pub(super) fn enabled(app: &App) -> bool {
    app.settings.interface_layout == InterfaceLayout::Classic
}

pub(super) fn back(app: &mut App, ctx: &egui::Context) {
    if app.view == View::Settings && app.close_settings_section() {
        ctx.request_repaint();
        return;
    }
    if app.close_catalog_detail() {
        return;
    }
    if app.view == View::Home {
        ctx.data_mut(|data| data.insert_temp(Id::new("desktop-exit-prompt"), true));
    } else {
        app.navigate(View::Home);
        ctx.request_repaint();
    }
}

pub(super) fn show_header(app: &mut App, ui: &mut Ui, header: Rect, scale: f32, classic: bool) {
    let header_height = header.height();
    let control_height = (header_height * 0.82).max(28.0);
    focus::set_header_zone();
    desktop::column(ui, header, "classic-header", 0.0, |ui| {
        let language = app.settings.language;
        ui.horizontal_centered(|ui| {
            if app.view != View::Home {
                let in_settings = app.view == View::Settings && app.settings_section.is_some();
                let response = desktop::compact_back_button(
                    ui,
                    if in_settings && app.adding_profile {
                        language.text("‹ Profile", "‹ Profiles")
                    } else if in_settings {
                        language.text("‹ Alle Einstellungen", "‹ All settings")
                    } else {
                        language.text("‹ Zurück", "‹ Back")
                    },
                    header_height - 8.0,
                );
                if activate(ui, &response, app.shared_remote_navigation()) {
                    back(app, ui.ctx());
                }
            }
            if classic {
                ui.heading("Dado Player");
                #[cfg(windows)]
                crate::licensing::trial_badge(ui, false);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut search_editor = None;
                ui.label(
                    app.settings
                        .clock_format
                        .format(chrono::Utc::now().timestamp()),
                );
                for (icon, label) in [
                    (
                        Icon::Refresh,
                        language.text("Liste aktualisieren", "Refresh library"),
                    ),
                    (Icon::Search, language.text("Suche", "Search")),
                ] {
                    if icon == Icon::Search {
                        let search_width = ui
                            .available_width()
                            .mul_add(0.55, 0.0)
                            .clamp(85.0, 300.0 * scale.max(0.65));
                        let remote = app.shared_remote_navigation();
                        let query = match app.view {
                            View::Live => &mut app.live_q,
                            View::Movies => &mut app.catalog_queries[0],
                            View::Series => &mut app.catalog_queries[1],
                            View::Settings => &mut app.settings_q,
                            _ => &mut app.search_q,
                        };
                        let response = ui
                            .allocate_ui_with_layout(
                                vec2(search_width, control_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    desktop::search_field(
                                        ui,
                                        query,
                                        language.text("Suchen", "Search"),
                                        remote,
                                        control_height,
                                    )
                                },
                            )
                            .inner;
                        let request = ui.data_mut(|data| {
                            let id = Id::new("header-search-request");
                            let requested = data.get_temp::<bool>(id).unwrap_or(false);
                            data.remove::<bool>(id);
                            requested
                        });
                        if app.view == View::Home && response.changed() {
                            app.navigate(View::Search);
                            ui.data_mut(|data| {
                                data.insert_temp(Id::new("header-search-request"), true)
                            });
                        } else if request {
                            response.request_focus();
                            focus::request_id(response.id);
                        }
                        search_editor = Some(response);
                    }
                    let size = control_height;
                    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        (desktop::appearance().control_radius * scale).max(10.0),
                        ui.visuals().widgets.inactive.bg_fill,
                    );
                    if icon == Icon::Refresh {
                        desktop::refresh_icon(
                            app,
                            ui,
                            rect.shrink(size * 0.18),
                            theme::accent(ui),
                            response.id,
                        );
                    } else {
                        components::icon(
                            ui,
                            rect.shrink(size * 0.13),
                            icon,
                            ui.visuals().text_color(),
                        );
                    }
                    response
                        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, label));
                    if icon == Icon::Refresh {
                        if activate(ui, &response, app.shared_remote_navigation()) {
                            app.refresh_catalog();
                        }
                        response.on_hover_text(label);
                    } else if response.clicked() {
                        if app.view == View::Home {
                            app.navigate(View::Search);
                            ui.data_mut(|data| {
                                data.insert_temp(Id::new("header-search-request"), true)
                            });
                        } else if let Some(input) = &search_editor {
                            input.request_focus();
                            focus::request_id(input.id);
                        }
                        ui.ctx().request_repaint();
                    }
                }
            });
        });
    });
}

pub(super) fn show(app: &mut App, ui: &mut Ui, canvas: Rect, scale: f32) -> Option<Rect> {
    let a = desktop::appearance();
    let gap = desktop::handheld_size(a.classic_gap * scale, 8.0);
    let header_height = desktop::handheld_size(a.classic_header_height * scale, 40.0);
    let header = Rect::from_min_size(canvas.min, vec2(canvas.width(), header_height));
    show_header(app, ui, header, scale, true);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(30));
    let body = Rect::from_min_max(pos2(canvas.left(), header.bottom() + gap), canvas.max);
    let page_dx = desktop::page_offset(app, ui.ctx(), scale);
    if app.view == View::Live {
        return super::live::show_desktop(app, ui, body, scale, page_dx);
    }
    let view = app.view;
    focus::set_zone(1);
    desktop::column(ui, body, "classic-content", page_dx, |ui| {
        ui.push_id(("classic", view as u64), |ui| {
            if view == View::Home {
                dashboard(app, ui, gap, scale);
            } else {
                desktop::section(app, ui, view);
            }
        });
    });
    None
}

fn activate(ui: &Ui, response: &egui::Response, remote: bool) -> bool {
    if remote {
        focus::clicked(ui, response)
    } else {
        response.clicked()
    }
}

fn dashboard(app: &mut App, ui: &mut Ui, gap: f32, scale: f32) {
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let body = ui.available_rect_before_wrap();
    let appearance = desktop::appearance();
    let label_size = desktop::handheld_type(appearance.classic_dashboard_title_size * scale, 23.0);
    let heading_size = desktop::handheld_type(44.0 * scale, 25.0);
    let profile = app
        .active_profile
        .as_ref()
        .map_or("", |profile| profile.name.as_str());
    let inset = (body.width() * 0.035).min(68.0 * scale);
    let left = body.left() + inset;
    let right = body.right() - inset;
    let top = body.top() + gap * 2.0;
    ui.painter().text(
        pos2(left, top),
        Align2::LEFT_TOP,
        t("Bibliothek", "Library"),
        tokens::bold(heading_size),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        pos2(right, top + gap),
        Align2::RIGHT_TOP,
        profile,
        tokens::reg(heading_size * 0.6),
        theme::muted(ui),
    );
    let media_start = top + heading_size + 2.0 * gap;
    let action_height = desktop::handheld_size(appearance.classic_action_height * scale, 62.0);
    let media_gap = desktop::handheld_size(appearance.classic_dashboard_gap * scale, 10.0);
    let media_end = (body.bottom() - gap).max(media_start + gap);
    let options_height = action_height + media_gap;
    let tile_height = (media_end - media_start - options_height)
        .clamp(1.0, appearance.classic_dashboard_height * scale);
    let spare = (media_end - media_start - tile_height - options_height).max(0.0);
    let tiles_top = media_start + spare * 0.24;
    let tiles_bottom = tiles_top + tile_height;
    let actions_y = tiles_bottom + media_gap + spare * 0.52;
    let media_width = (right - left - media_gap * 2.0).max(3.0);
    let live_width = media_width * appearance.classic_dashboard_live_share;
    let other_width = (media_width - live_width) * 0.5;
    let movies_x = left + live_width + media_gap;
    let series_x = movies_x + other_width + media_gap;
    let media = [
        (
            View::Live,
            t("Live-TV", "Live TV"),
            Icon::Tv,
            Rect::from_min_max(pos2(left, tiles_top), pos2(left + live_width, tiles_bottom)),
        ),
        (
            View::Movies,
            t("Filme", "Movies"),
            Icon::Film,
            Rect::from_min_max(
                pos2(movies_x, tiles_top),
                pos2(movies_x + other_width, tiles_bottom),
            ),
        ),
        (
            View::Series,
            t("Serien", "Series"),
            Icon::Series,
            Rect::from_min_max(pos2(series_x, tiles_top), pos2(right, tiles_bottom)),
        ),
    ];
    let mut next = None;
    for (view, label, icon, rect) in media {
        let response = ui.interact(rect, ui.id().with(view as u8), Sense::click());
        let [top_color, bottom_color, accent, icon_ink, icon_light, icon_shade] = tile_color(view);
        let radius = (22.0 * scale).max(10.0);
        let painter = ui.painter();
        let focused = response.has_focus() || focus::focused_id() == Some(response.id);
        let active = focused || response.hovered();
        let lift = ui.ctx().animate_bool_with_time(
            response.id.with("tile-lift"),
            active,
            if desktop::reduced_motion(ui.ctx()) {
                0.0
            } else {
                0.15
            },
        ) * (6.0 * scale);
        let raised = rect.translate(vec2(0.0, -lift));
        painter.rect_filled(raised, radius, bottom_color);
        tile_surface(ui, raised, radius, top_color, bottom_color);
        let rect = raised;
        painter.rect_stroke(
            rect.shrink(0.7),
            radius - 0.7,
            Stroke::new((1.6 * scale).max(0.8), Color32::from_white_alpha(95)),
        );
        if active {
            painter.rect_stroke(
                rect.shrink(1.3),
                radius - 1.0,
                Stroke::new(
                    if focused {
                        (4.0 * scale).max(2.0)
                    } else {
                        (2.0 * scale).max(1.5)
                    },
                    accent,
                ),
            );
        }
        let label_size = label_size.min(rect.height() * 0.24);
        let icon_size = (rect.height() * 0.54)
            .min(appearance.classic_dashboard_icon_size * scale)
            .min(rect.width() * 0.60);
        let center = pos2(rect.center().x, rect.top() + rect.height() * 0.37);
        let plate = Rect::from_center_size(center, vec2(icon_size, icon_size));
        let plate_rounding = icon_size * 0.28;
        let halo = (icon_size * 0.79).min(rect.height() * 0.34);
        painter.circle_filled(center, halo, Color32::from_white_alpha(10));
        painter.circle_stroke(
            center,
            halo,
            Stroke::new((1.1 * scale).max(0.65), Color32::from_white_alpha(36)),
        );
        tile_surface(ui, plate, plate_rounding, icon_light, icon_shade);
        painter.rect_stroke(
            plate,
            plate_rounding,
            Stroke::new((1.8 * scale).max(0.8), Color32::from_white_alpha(92)),
        );
        components::icon(ui, plate.shrink(icon_size * 0.11), icon, icon_ink);
        let content = painter.with_clip_rect(rect.shrink(4.0));
        let title_pos = pos2(center.x, rect.top() + rect.height() * 0.74);
        content.text(
            title_pos + vec2(0.0, 2.0 * scale),
            Align2::CENTER_CENTER,
            label,
            tokens::bold(label_size),
            Color32::from_black_alpha(135),
        );
        content.text(
            title_pos,
            Align2::CENTER_CENTER,
            label,
            tokens::bold(label_size),
            Color32::WHITE,
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, label));
        if view == View::Live {
            focus::prefer_next_when_inactive();
        }
        if activate(ui, &response, app.shared_remote_navigation()) {
            next = Some(view);
        }
    }
    let actions = [
        (
            View::Continue,
            t("Fortsetzen", "Continue"),
            Icon::Play,
            false,
        ),
        (
            View::Favorites,
            t("Favoriten", "Favorites"),
            Icon::Star,
            false,
        ),
        (View::Settings, t("Profile", "Profiles"), Icon::User, true),
        (
            View::Settings,
            t("Einstellungen", "Settings"),
            Icon::Settings,
            false,
        ),
    ];
    let action_count = if app.settings.watch_history { 4.0 } else { 3.0 };
    let action_gap = media_gap * 0.75;
    let width = ((right - left - action_gap * (action_count - 1.0)) / action_count).max(1.0);
    let mut open_profiles = false;
    for (index, (view, label, icon, profiles)) in actions
        .into_iter()
        .filter(|(view, _, _, _)| *view != View::Continue || app.settings.watch_history)
        .enumerate()
    {
        let rect = Rect::from_min_size(
            pos2(left + index as f32 * (width + action_gap), actions_y),
            vec2(width, action_height),
        );
        let response = ui.interact(rect, ui.id().with(("shortcut", index)), Sense::click());
        let focused = response.has_focus() || focus::focused_id() == Some(response.id);
        let style = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            13.0 * scale,
            style.bg_fill,
            if focused {
                Stroke::new((4.0 * scale).max(2.0), theme::accent(ui))
            } else {
                style.bg_stroke
            },
        );
        let size = (action_height * 0.36).min(width * 0.33);
        components::icon(
            ui,
            Rect::from_center_size(
                pos2(rect.center().x, rect.top() + action_height * 0.36),
                vec2(size, size),
            ),
            icon,
            theme::accent(ui),
        );
        let font = tokens::bold(desktop::handheld_type(32.0 * scale, 14.0));
        let galley =
            ui.painter()
                .layout_no_wrap(label.into(), font.clone(), ui.visuals().text_color());
        let font = egui::FontId::new(
            font.size * ((width - 8.0).max(1.0) / galley.size().x).min(1.0),
            font.family,
        );
        ui.painter().text(
            pos2(rect.center().x, rect.top() + action_height * 0.73),
            Align2::CENTER_CENTER,
            label,
            font,
            ui.visuals().text_color(),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, label));
        if activate(ui, &response, app.shared_remote_navigation()) {
            open_profiles = profiles;
            next = Some(view);
        }
    }
    if let Some(view) = next {
        app.navigate(view);
        if open_profiles {
            settings::open_profiles(app);
        }
        focus::request_index(0);
        ui.ctx().request_repaint();
    }
}
