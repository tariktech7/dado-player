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
struct TileColor {
    top: String,
    bottom: String,
    accent: String,
}

fn tile_color(view: View) -> [Color32; 3] {
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
    [&color.top, &color.bottom, &color.accent]
        .map(|value| Color32::from_hex(value).expect("bundled tile color"))
}

fn tile_surface(ui: &Ui, rect: Rect, radius: f32, top: Color32, bottom: Color32) {
    let radius = radius.min(rect.height() * 0.5).min(rect.width() * 0.5);
    let mut mesh = Mesh::default();
    let strips = 20;
    for step in 0..=strips {
        let fraction = step as f32 / strips as f32;
        let y = egui::lerp(rect.top()..=rect.bottom(), fraction);
        let corner = (radius - (y - rect.top()).min(rect.bottom() - y)).max(0.0);
        let inset = radius - (radius * radius - corner * corner).max(0.0).sqrt();
        let fill =
            Color32::from(Rgba::from(top) * (1.0 - fraction) + Rgba::from(bottom) * fraction);
        mesh.colored_vertex(pos2(rect.left() + inset, y), fill);
        mesh.colored_vertex(pos2(rect.right() - inset, y), fill);
        if step > 0 {
            let index = (step - 1) * 2;
            mesh.add_triangle(index, index + 1, index + 2);
            mesh.add_triangle(index + 1, index + 2, index + 3);
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
    if app.close_catalog_detail() {
        return;
    }
    if app.view == View::Home {
        ctx.data_mut(|data| data.insert_temp(Id::new("desktop-exit-prompt"), true));
    } else {
        app.navigate(View::Home);
        focus::request_index(0);
        ctx.request_repaint();
    }
}

pub(super) fn show(app: &mut App, ui: &mut Ui, canvas: Rect, scale: f32) -> Option<Rect> {
    let a = desktop::appearance();
    let gap = desktop::handheld_size(a.classic_gap * scale, 8.0);
    let header_height = desktop::handheld_size(a.classic_header_height * scale, 40.0);
    let header = Rect::from_min_size(canvas.min, vec2(canvas.width(), header_height));
    focus::set_zone(0);
    desktop::column(ui, header, "classic-header", 0.0, |ui| {
        let language = app.settings.language;
        ui.horizontal_centered(|ui| {
            if app.view != View::Home {
                let response =
                    desktop::back_button(ui, language.text("‹ Übersicht", "‹ Dashboard"));
                if activate(ui, &response, app.tv) {
                    app.navigate(View::Home);
                    focus::request_index(0);
                }
            }
            ui.heading("Dado Player");
            #[cfg(windows)]
            crate::licensing::trial_badge(ui, false);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                    let size = header_height * 0.77;
                    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        9.0 * scale,
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
                    if activate(ui, &response, app.tv) {
                        if icon == Icon::Refresh {
                            app.refresh_catalog();
                        } else {
                            app.navigate(View::Search);
                        }
                    }
                    response.on_hover_text(label);
                }
            });
        });
    });
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
    let tiles_top =
        media_start + ((media_end - media_start - tile_height - options_height) * 0.5).max(0.0);
    let tiles_bottom = tiles_top + tile_height;
    let actions_y = tiles_bottom + media_gap;
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
        let [top_color, bottom_color, accent] = tile_color(view);
        let radius = (22.0 * scale).max(10.0);
        let painter = ui.painter();
        let focused = response.has_focus() || focus::focused_id() == Some(response.id);
        let depth = (9.0 * scale).max(3.0);
        painter.rect_filled(
            rect.translate(vec2(0.0, depth * 1.7)),
            radius,
            Color32::from_black_alpha(45),
        );
        painter.rect_filled(
            rect.translate(vec2(0.0, depth)),
            radius,
            bottom_color.gamma_multiply(0.58),
        );
        tile_surface(ui, rect, radius, top_color, bottom_color);
        painter.rect_stroke(
            rect.shrink(0.7),
            radius,
            Stroke::new((1.6 * scale).max(0.8), Color32::from_white_alpha(95)),
        );
        if focused || response.hovered() {
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
        painter.rect_filled(
            plate.translate(vec2(0.0, depth * 0.65)),
            plate_rounding,
            Color32::from_black_alpha(47),
        );
        tile_surface(
            ui,
            plate,
            plate_rounding,
            Color32::from_white_alpha(69),
            Color32::from_white_alpha(18),
        );
        painter.rect_stroke(
            plate,
            plate_rounding,
            Stroke::new((1.8 * scale).max(0.8), Color32::from_white_alpha(92)),
        );
        components::icon(ui, plate, icon, accent);
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
        if activate(ui, &response, app.tv) {
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
    let width = ((right - left - gap * 3.0) / 4.0).max(1.0);
    let mut open_profiles = false;
    focus::set_zone(2);
    for (index, (view, label, icon, profiles)) in actions.into_iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(left + index as f32 * (width + gap), actions_y),
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
        if activate(ui, &response, app.tv) {
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
