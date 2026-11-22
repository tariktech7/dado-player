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
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, Ui};
use std::{collections::HashMap, sync::Arc};

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
    if app.view == View::Movies && app.selected_movie.take().is_some()
        || app.view == View::Series && app.selected_series.take().is_some()
    {
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
                let response = ui.button(language.text("‹ Übersicht", "‹ Dashboard"));
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
    if app.view == View::Live {
        return super::live::show_desktop(app, ui, body, scale, 0.0);
    }
    let view = app.view;
    focus::set_zone(1);
    desktop::column(ui, body, "classic-content", 0.0, |ui| {
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
    let label_size = desktop::handheld_type(appearance.classic_dashboard_title_size * scale, 22.0);
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
        tokens::reg(label_size),
        theme::muted(ui),
    );
    let media_start = top + heading_size + 2.0 * gap;
    let action_height = desktop::handheld_size(66.0 * scale, 43.0);
    let actions_y = (body.bottom() - action_height - gap).max(media_start + gap * 3.0);
    let media_end = actions_y - gap;
    let tile_height =
        (media_end - media_start).min(desktop::appearance().classic_dashboard_height * scale);
    let tiles_top = (media_start + media_end - tile_height) / 2.0;
    let tiles_bottom = tiles_top + tile_height;
    let split = left + (right - left) * 0.46;
    let middle = tiles_top + (tiles_bottom - tiles_top) / 2.0;
    let media = [
        (
            View::Live,
            t("Live-TV", "Live TV"),
            Icon::Tv,
            Rect::from_min_max(pos2(left, tiles_top), pos2(split, tiles_bottom)),
            app.library.channels.len(),
        ),
        (
            View::Movies,
            t("Filme", "Movies"),
            Icon::Film,
            Rect::from_min_max(
                pos2(split + gap, tiles_top),
                pos2(right, middle - gap * 0.5),
            ),
            app.library.movies.len(),
        ),
        (
            View::Series,
            t("Serien", "Series"),
            Icon::Series,
            Rect::from_min_max(
                pos2(split + gap, middle + gap * 0.5),
                pos2(right, tiles_bottom),
            ),
            app.library.series.len(),
        ),
    ];
    let mut next = None;
    for (view, label, icon, rect, count) in media {
        let response = ui.interact(rect, ui.id().with(view as u8), Sense::click());
        let fill = if view == View::Live || response.hovered() {
            theme::selection_surface(ui)
        } else {
            ui.visuals().window_fill
        };
        ui.painter()
            .rect(rect, 16.0 * scale, fill, ui.visuals().window_stroke);
        let inset = (rect.width() * 0.08).min(34.0 * scale);
        let icon_size = (rect.height() * 0.6)
            .min(appearance.classic_dashboard_icon_size * scale)
            .max(30.0);
        let center = rect.left_center() + vec2(inset + icon_size / 2.0, 0.0);
        components::icon(
            ui,
            Rect::from_center_size(center, vec2(icon_size, icon_size)),
            icon,
            theme::accent(ui),
        );
        let label_x = center.x + icon_size / 2.0 + gap;
        let label_width = (rect.right() - label_x - inset).max(1.0);
        components::draw_line(
            ui,
            pos2(label_x, rect.center().y - label_size * 0.25),
            label,
            tokens::bold(label_size),
            ui.visuals().text_color(),
            label_width,
        );
        components::draw_line(
            ui,
            pos2(label_x, rect.center().y + label_size * 0.6),
            &count.to_string(),
            tokens::reg(label_size * 0.52),
            theme::muted(ui),
            label_width,
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, label));
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                16.0 * scale,
                Stroke::new((4.0 * scale).max(2.0), theme::accent(ui)),
            );
        }
        if activate(ui, &response, app.tv) {
            next = Some(view);
        }
    }
    let actions = [
        (View::Continue, t("Fortsetzen", "Continue"), false),
        (View::Favorites, t("Favoriten", "Favorites"), false),
        (View::Settings, t("Profile", "Profiles"), true),
        (View::Settings, t("Einstellungen", "Settings"), false),
    ];
    let width = ((right - left - gap * 3.0) / 4.0).max(1.0);
    focus::set_zone(2);
    for (index, (view, label, profiles)) in actions.into_iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(left + index as f32 * (width + gap), actions_y),
            vec2(width, action_height),
        );
        let response = ui.put(
            rect,
            egui::Button::new(egui::RichText::new(label).size(if desktop::handheld() {
                14.0
            } else {
                label_size * 0.67
            }))
            .min_size(rect.size()),
        );
        if activate(ui, &response, app.tv) {
            if profiles {
                settings::open_profiles(ui.ctx());
            } else if view == View::Settings {
                ui.ctx()
                    .data_mut(|data| data.remove::<usize>(Id::new("settings-current-section")));
            }
            next = Some(view);
        }
    }
    if let Some(view) = next {
        app.navigate(view);
        focus::request_index(0);
        ui.ctx().request_repaint();
    }
}
