use crate::app::App;
use crate::types::{
    AppLanguage, ClockFormat, HwDec, InterfaceLayout, SettingsMode, Theme, VideoOut, VideoSync,
};
use egui::{RichText, Ui};

/// Every settings section with the words that should find it. One table so the
/// filter and the section headers can never drift apart.
const SECTIONS: &[(&str, &str)] = &[
    (
        "Sprachen",
        "sprachen audio ton untertitel subtitles language standard",
    ),
    (
        "Lizenz & Gerät",
        "lizenz aktivierung freischaltung test tage trial unlimited mac adresse geräte-id",
    ),
    (
        "Darstellung",
        "design farbschema theme dunkel hell system dark light aussehen appearance",
    ),
    (
        "Wiedergabe",
        "wiedergabe playback preset standard schwacher pc beste qualität profil",
    ),
    ("Puffer", "puffer cache buffering sekunden vorladen"),
    ("Start", "start autoplay zuletzt kanal automatisch"),
    (
        "Schnellauswahl",
        "schnellauswahl preset standard schonmodus qualität profile",
    ),
    (
        "Bild & Decoder",
        "bild decoder hwdec hardware video vo ausgabe vsync synchronisation interpolation deband",
    ),
    (
        "Ton",
        "ton audio wasapi exklusiv passthrough surround ac3 dts normalisieren lautstärke",
    ),
    ("Untertitel", "untertitel subtitles schrift größe font size"),
    (
        "Netzwerk & Puffer",
        "netzwerk timeout user agent tls zertifikat puffer cache http",
    ),
    ("Allgemein", "allgemein autoplay letzten kanal start"),
];

fn keywords(title: &str) -> &'static str {
    SECTIONS
        .iter()
        .find(|(t, _)| *t == title)
        .map(|(_, k)| *k)
        .unwrap_or("")
}

fn hits(q: &str, title: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    title.to_lowercase().contains(q) || keywords(title).contains(q)
}

/// Draws the section header when it matches the filter; returns whether it did.
fn sec(ui: &mut Ui, q: &str, title: &str) -> bool {
    if !hits(q, title) {
        return false;
    }
    section(ui, title);
    true
}

#[derive(serde::Deserialize)]
struct Section {
    id: String,
    de: String,
    en: String,
}

fn sections() -> &'static [Section] {
    static SECTIONS: std::sync::OnceLock<Vec<Section>> = std::sync::OnceLock::new();
    SECTIONS.get_or_init(|| {
        serde_json::from_str(include_str!("../../shared/settings-sections.json"))
            .expect("bundled settings sections")
    })
}

pub(crate) fn uses_ordered_focus(index: usize) -> bool {
    sections().get(index).is_some_and(|section| {
        matches!(
            section.id.as_str(),
            "clock" | "stream" | "player" | "profiles"
        )
    })
}

pub(crate) fn focus_option(ui: &Ui, response: &egui::Response) {
    let pending = egui::Id::new("settings-option-entry");
    if response.enabled() && ui.data_mut(|data| data.remove_temp::<bool>(pending)) == Some(true) {
        response.request_focus();
        super::mobile::focus::prefer_entry(response.id);
    }
}

fn activate(ui: &Ui, response: &egui::Response) -> bool {
    focus_option(ui, response);
    if super::mobile::active(ui.ctx()) {
        super::mobile::focus::clicked(ui, response)
    } else {
        response.clicked()
    }
}

fn action(ui: &mut Ui, label: &str) -> bool {
    let response = ui.button(label);
    activate(ui, &response)
}

fn choice_label(label: &str, current: bool) -> egui::RichText {
    if current {
        egui::RichText::new(format!("✓  {label}")).strong()
    } else {
        egui::RichText::new(label)
    }
}

fn select<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    ui.label(label);
    if !super::mobile::active(ui.ctx()) {
        let selected = options
            .iter()
            .find(|(option, _)| option == value)
            .map_or("–", |(_, label)| *label);
        let combo = egui::ComboBox::from_id_source(("setting-option", label))
            .selected_text(selected)
            .width(ui.available_width().min(360.0))
            .show_ui(ui, |ui| {
                for &(option, text) in options {
                    changed |= ui
                        .selectable_value(value, option, choice_label(text, *value == option))
                        .changed();
                }
            });
        focus_option(ui, &combo.response);
        return changed;
    }
    ui.horizontal_wrapped(|ui| {
        for &(option, text) in options {
            let response =
                ui.selectable_label(*value == option, choice_label(text, *value == option));
            if activate(ui, &response) && *value != option {
                *value = option;
                changed = true;
            }
        }
    });
    changed
}

fn switch(ui: &mut Ui, label: &str, value: &mut bool) -> bool {
    let before = *value;
    let response = ui.checkbox(value, label);
    if activate(ui, &response) && *value == before {
        *value = !before;
    }
    *value != before
}

pub fn show(app: &mut App, ui: &mut Ui) {
    use super::mobile::{components, focus, tokens};
    use components::Icon;

    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let scale = super::desktop::scale(ui.ctx());
    let mut changed = false;
    let mut playback_changed = false;
    let draft = ui.id().with("settings-draft");
    let available = ui.available_size();
    let section_key = egui::Id::new("last-settings-section");
    let pending = egui::Id::new("settings-option-entry");
    ui.data_mut(|data| {
        let previous = data.remove_temp::<usize>(section_key);
        if let Some(index) = app.settings_section {
            data.insert_temp(section_key, index);
            if previous != Some(index) {
                data.insert_temp(pending, true);
            }
        }
    });
    egui::ScrollArea::vertical()
        .id_source(("settings-page", app.settings_section))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(index) = app.settings_section {
                let section = &sections()[index];
                let ordered = uses_ordered_focus(index);
                if ordered {
                    focus::set_ordered_zone(1);
                } else {
                    focus::set_zone(1);
                }
                let mobile = super::mobile::active(ui.ctx());
                centered_contents(
                    ui,
                    if section.id == "profiles" {
                        available.x * 0.72
                    } else {
                        (available.x * 0.85)
                            .min(super::desktop::appearance().settings_panel_width * scale.max(0.8))
                    },
                    draft.with(index),
                    Some(300.0 * scale),
                    |ui| {
                        panel(
                            app,
                            ui,
                            section,
                            (available.y * 0.055).clamp(23.0, 45.0),
                            draft.with(index).with("choice"),
                            mobile,
                            &mut changed,
                            &mut playback_changed,
                        );
                    },
                );
            } else {
                centered_contents(ui, available.x * 0.95, draft.with("overview"), None, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let heading_top = ui.next_widget_position().y;
                    ui.vertical_centered(|ui| {
                        ui.heading(
                            egui::RichText::new(t("Einstellungen", "Settings"))
                                .size((available.y * 0.06).clamp(24.0, 52.0)),
                        );
                    });
                    ui.add_space((18.0 * scale).max(10.0));
                    let width = ui.available_width();
                    let columns = if width < 580.0 { 2 } else { 3 };
                    let gap = (18.0 * scale).max(10.0);
                    let tile_width =
                        ((width - (columns - 1) as f32 * gap) / columns as f32 - 2.0).max(1.0);
                    let tiles: Vec<_> = sections()
                        .iter()
                        .enumerate()
                        .filter(|(_, section)| section.id != "license" || cfg!(windows))
                        .filter(|(_, section)| {
                            t(&section.de, &section.en)
                                .to_lowercase()
                                .contains(&app.settings_q.trim().to_lowercase())
                        })
                        .collect();
                    let rows = tiles.len().div_ceil(columns).max(1);
                    let heading_height = ui.next_widget_position().y - heading_top;
                    let row_gaps = gap * (rows - 1) as f32;
                    // Reserve the heading's rendered height and both outer margins
                    // before assigning any space to the tiles.
                    let tile_height = ((available.y - heading_height - row_gaps - 28.0)
                        / rows as f32)
                        .clamp((90.0 * scale).max(50.0), (240.0 * scale).max(82.0));
                    focus::set_zone(1);
                    for (row_index, row) in tiles.chunks(columns).enumerate() {
                        if row_index > 0 {
                            ui.add_space(gap);
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for &(index, section) in row {
                                let label = t(&section.de, &section.en);
                                let icon = match section.id.as_str() {
                                    "clock" => Icon::Clock,
                                    "updates" => Icon::Refresh,
                                    "player" | "playback" => Icon::Play,
                                    "stream" => Icon::Tv,
                                    "audio" => Icon::Volume,
                                    "profiles" => Icon::User,
                                    "cache" => Icon::Trash,
                                    "parental" => Icon::EyeOff,
                                    "license" => Icon::Info,
                                    _ => Icon::Settings,
                                };
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(tile_width, tile_height),
                                    egui::Sense::click(),
                                );
                                if app.settings_return_tile.is_none() {
                                    focus::prefer_entry(response.id);
                                }
                                if app.settings_return_tile == Some(index) {
                                    app.settings_return_tile = None;
                                    response.request_focus();
                                    focus::request_id(response.id);
                                }
                                let style = ui.style().interact(&response);
                                let focused = response.has_focus()
                                    || focus::focused_id() == Some(response.id);
                                ui.painter().rect(
                                    rect,
                                    (super::desktop::appearance().settings_panel_radius * scale)
                                        .max(14.0),
                                    style.bg_fill,
                                    if focused {
                                        egui::Stroke::new(3.0, crate::theme::accent(ui))
                                    } else {
                                        style.bg_stroke
                                    },
                                );
                                let icon_size = (tile_height * 0.30).min(tile_width * 0.28);
                                components::icon(
                                    ui,
                                    egui::Rect::from_center_size(
                                        egui::pos2(
                                            rect.center().x,
                                            rect.top() + tile_height * 0.34,
                                        ),
                                        egui::Vec2::splat(icon_size),
                                    ),
                                    icon,
                                    crate::theme::accent(ui),
                                );
                                let font_size = (31.0 * scale).max(16.0);
                                let font = tokens::bold(font_size);
                                let label_width = ui
                                    .painter()
                                    .layout_no_wrap(
                                        label.into(),
                                        font.clone(),
                                        ui.visuals().text_color(),
                                    )
                                    .size()
                                    .x;
                                let font = tokens::bold(
                                    font_size
                                        * ((tile_width - 12.0).max(1.0) / label_width).min(1.0),
                                );
                                ui.painter().text(
                                    egui::pos2(rect.center().x, rect.top() + tile_height * 0.73),
                                    egui::Align2::CENTER_CENTER,
                                    label,
                                    font,
                                    ui.visuals().text_color(),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::WidgetType::Button, label)
                                });
                                let clicked = if app.shared_remote_navigation() {
                                    focus::clicked(ui, &response)
                                } else {
                                    response.clicked()
                                };
                                if clicked {
                                    app.settings_section = Some(index);
                                    if section.id == "cache" {
                                        ui.ctx().data_mut(|data| {
                                            data.remove::<bool>(egui::Id::new(
                                                "settings-cache-cleared",
                                            ))
                                        });
                                    }
                                    ui.ctx().data_mut(|data| {
                                        data.remove::<usize>(draft.with(index).with("choice"))
                                    });
                                    ui.ctx().request_repaint();
                                }
                            }
                        });
                    }
                });
            }
        });
    ui.data_mut(|data| data.remove::<bool>(pending));
    if changed || playback_changed {
        app.save_settings_now();
        ui.ctx().request_repaint();
    }
    if playback_changed && !super::mobile::active(ui.ctx()) {
        app.reset_player();
    }
}

fn centered_contents(
    ui: &mut Ui,
    max_width: f32,
    id: egui::Id,
    estimate: Option<f32>,
    draw: impl FnOnce(&mut Ui),
) {
    let width = (ui.available_width() - 16.0)
        .min(max_width.max(520.0))
        .max(1.0);
    let inset = ((ui.available_width() - width) * 0.5).max(0.0);
    let height_key = id.with("height");
    let height = estimate.map(|estimate| {
        ui.data(|data| data.get_temp::<f32>(height_key))
            .unwrap_or(estimate)
    });
    ui.add_space(height.map_or(8.0, |height| {
        ((ui.available_height() - height) * 0.5).max(12.0)
    }));
    let row = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(inset);
        ui.push_id(id, |ui| {
            ui.vertical(|ui| {
                ui.set_width(width);
                draw(ui);
            })
        });
    });
    let actual_height = row.response.rect.height();
    if height.is_some_and(|height| (height - actual_height).abs() > 1.0) {
        ui.data_mut(|data| data.insert_temp(height_key, actual_height));
        ui.ctx().request_repaint();
    }
    ui.add_space(12.0);
}

pub(super) fn open_profiles(app: &mut App) {
    app.settings_section = sections()
        .iter()
        .position(|section| section.id == "profiles");
}

fn quick_choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    key: egui::Id,
    current: T,
    options: &[(T, &str)],
    save_label: &str,
) -> Option<T> {
    let mut index = ui
        .ctx()
        .data(|data| data.get_temp::<usize>(key))
        .or_else(|| options.iter().position(|(value, _)| *value == current))
        .unwrap_or(0);
    ui.add_space(8.0);
    for (slot, (value, label)) in options.iter().enumerate() {
        let response = ui.add_sized(
            [ui.available_width(), ui.spacing().interact_size.y.max(40.0)],
            egui::SelectableLabel::new(slot == index, choice_label(label, *value == current)),
        );
        focus_option(ui, &response);
        if super::mobile::focus::clicked(ui, &response) {
            index = slot;
            ui.ctx().data_mut(|data| data.insert_temp(key, index));
        }
        ui.add_space(10.0);
    }
    ui.add_space(10.0);
    let save = ui.button(save_label);
    if super::mobile::focus::clicked(ui, &save) {
        ui.ctx().data_mut(|data| data.remove::<usize>(key));
        return options.get(index).map(|(value, _)| *value);
    }
    None
}

fn profiles(app: &mut App, ui: &mut Ui) {
    use super::mobile::{components::Kb, focus, login};
    let t = app.settings.language;
    let scale = super::desktop::scale(ui.ctx());
    let row_height = (66.0 * scale).max(48.0);
    let padding = 6.0;
    let mut select = None;
    for (index, profile) in app.profiles.iter().enumerate() {
        ui.push_id(&profile.id, |ui| {
            let active = app
                .active_profile
                .as_ref()
                .is_some_and(|p| p.id == profile.id);
            egui::Frame::none()
                .stroke(ui.visuals().widgets.inactive.bg_stroke)
                .rounding(ui.visuals().widgets.inactive.rounding)
                .inner_margin(egui::Margin::symmetric(14.0, padding))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let select_width =
                            (184.0 * scale).max(112.0).min(ui.available_width() * 0.45);
                        let height = row_height - 2.0 * padding;
                        let name_width =
                            (ui.available_width() - select_width - ui.spacing().item_spacing.x)
                                .max(1.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(name_width, height),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&profile.name)
                                            .text_style(egui::TextStyle::Button)
                                            .strong(),
                                    )
                                    .truncate(true),
                                );
                            },
                        );
                        let response = ui
                            .add_enabled_ui(!active && !app.busy, |ui| {
                                ui.add_sized(
                                    [select_width, height],
                                    egui::Button::new(t.text("Auswählen", "Select")),
                                )
                            })
                            .inner;
                        focus_option(ui, &response);
                        if focus::clicked(ui, &response) {
                            select = Some(index);
                        }
                    });
                });
        });
    }
    if let Some(index) = select {
        app.start_login(app.profiles[index].clone());
    }
    if app.busy {
        ui.spinner();
    }
    let add = ui.add_sized(
        [ui.available_width(), row_height],
        egui::Button::new(t.text("Profil hinzufügen", "Add profile")),
    );
    focus_option(ui, &add);
    if focus::clicked(ui, &add) {
        app.begin_profile_add();
    }
    if app.adding_profile {
        for (kind, title) in [
            (crate::types::ProfileKind::Xtream, "Xtream Codes"),
            (crate::types::ProfileKind::M3u, "M3U"),
        ] {
            let response = ui.selectable_label(app.login_kind == kind, title);
            if focus::clicked(ui, &response) {
                app.login_kind = kind;
            }
        }
        let mut focused = None;
        let mut show = false;
        login::field(
            ui,
            t.text("Profilname", "Profile name"),
            &mut app.f_name,
            "",
            Kb::Text,
            &mut focused,
            &mut show,
        );
        if app.login_kind == crate::types::ProfileKind::Xtream {
            login::field(
                ui,
                t.text("Server-Adresse", "Server address"),
                &mut app.f_host,
                "https://…",
                Kb::Url,
                &mut focused,
                &mut show,
            );
            login::field(
                ui,
                t.text("Benutzername", "Username"),
                &mut app.f_user,
                "",
                Kb::Text,
                &mut focused,
                &mut show,
            );
            login::field(
                ui,
                t.text("Passwort", "Password"),
                &mut app.f_pass,
                "",
                Kb::Password,
                &mut focused,
                &mut app.show_password,
            );
        } else {
            login::field(
                ui,
                t.text("M3U-Adresse", "M3U address"),
                &mut app.f_m3u_url,
                "https://…",
                Kb::Url,
                &mut focused,
                &mut show,
            );
            #[cfg(windows)]
            login::field(
                ui,
                t.text("Oder lokale Playlistdatei", "Or local playlist file"),
                &mut app.f_m3u_file,
                "",
                Kb::Text,
                &mut focused,
                &mut show,
            );
        }
        ui.horizontal(|ui| {
            let save = ui.button(t.text("Profil speichern", "Save profile"));
            let cancel = ui.button(t.text("Abbrechen", "Cancel"));
            if focus::clicked(ui, &save) {
                app.save_added_profile();
            }
            if focus::clicked(ui, &cancel) {
                app.close_settings_section();
            }
        });
        if !app.adding_profile {
            focus::request_id(add.id);
            ui.ctx().request_repaint();
        }
    }
    app.cloud_profiles_ui(ui, row_height);
    if let Some(error) = &app.error {
        ui.label(error);
    }
}

fn panel(
    app: &mut App,
    ui: &mut Ui,
    section: &Section,
    heading_size: f32,
    draft: egui::Id,
    mobile: bool,
    changed: &mut bool,
    playback_changed: &mut bool,
) {
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let save = t("Änderungen speichern", "Save changes");
    let scale = super::desktop::scale(ui.ctx());
    let inset = (32.0 * scale).max(16.0);
    let spacing = (32.0 * scale).max(17.0);
    egui::Frame::none()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().window_stroke)
        .rounding(
            (super::desktop::appearance().settings_panel_radius * super::desktop::scale(ui.ctx()))
                .max(16.0),
        )
        .inner_margin(egui::Margin::same(inset))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = spacing;
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new(t(&section.de, &section.en)).size(heading_size));
            });
            ui.add_space((inset + 10.0 - spacing).max(0.0));
            match section.id.as_str() {
                "clock" => {
                    if let Some(value) = quick_choice(
                        ui,
                        draft,
                        app.settings.clock_format,
                        &[
                            (ClockFormat::H24, "24 h · 18:30"),
                            (ClockFormat::H12, "12 h · 06:30 PM"),
                        ],
                        save,
                    ) {
                        app.settings.clock_format = value;
                        *changed = true;
                        app.close_settings_section();
                    }
                }
                "stream" => {
                    use crate::types::LiveFormat;
                    let options = [
                        (LiveFormat::Automatic, t("Standard", "Default")),
                        (LiveFormat::Ts, "MPEG-TS (.ts)"),
                        (LiveFormat::Hls, "HLS (.m3u8)"),
                    ];
                    ui.label(t(
                        "Gilt für Xtream-Live-TV. M3U verwendet das Format der Wiedergabeliste.",
                        "Applies to Xtream live TV. M3U uses the playlist's stream format.",
                    ));
                    if let Some(value) =
                        quick_choice(ui, draft, app.settings.live_format, &options, save)
                    {
                        app.settings.live_format = value;
                        app.apply_live_format();
                        *changed = true;
                        app.close_settings_section();
                    }
                }
                "player" => {
                    #[cfg(any(target_os = "android", target_os = "windows"))]
                    if let Some(value) = quick_choice(
                        ui,
                        draft,
                        app.settings.effective_player_backend(),
                        &crate::types::PlayerBackend::all().map(|player| (player, player.label())),
                        save,
                    ) {
                        app.settings.player_backend = value;
                        *changed = true;
                        app.close_settings_section();
                    }
                    #[cfg(target_os = "ios")]
                    {
                        use crate::types::IosPlaybackEngine as Engine;
                        let current = (
                            app.settings.ios_native_player,
                            app.settings.ios_playback_engine,
                        );
                        let options = [
                            (
                                (false, Engine::Auto),
                                t("Dado Player · automatisch", "Dado Player · automatic"),
                            ),
                            ((true, Engine::Apple), "Apple Player"),
                            ((false, Engine::Apple), "Dado Player · AVPlayer"),
                            ((false, Engine::Vlc), "Dado Player · VLC"),
                        ];
                        if let Some((native, engine)) =
                            quick_choice(ui, draft, current, &options, save)
                        {
                            app.settings.settings_mode = SettingsMode::Expert;
                            app.settings.ios_native_player = native;
                            app.settings.ios_playback_engine = engine;
                            crate::platform::ios::bridge::native_player(native);
                            *changed = true;
                            app.close_settings_section();
                        }
                    }
                    #[cfg(not(any(
                        target_os = "android",
                        target_os = "ios",
                        target_os = "windows"
                    )))]
                    ui.label(t("Integrierter Dado Player", "Built-in Dado Player"));
                }
                "cache" => {
                    ui.label(t(
                        "Temporäre Bilder, Programmdaten und Episodeninfos leeren?",
                        "Clear temporary images, programme data and episode details?",
                    ));
                    ui.label(t(
                        "Profile, Favoriten, Verlauf und Lizenz bleiben erhalten.",
                        "Profiles, favourites, history and licence are kept.",
                    ));
                    if ui
                        .add_enabled_ui(!app.busy && !app.catalog_refreshing, |ui| {
                            action(ui, t("Jetzt leeren", "Clear now"))
                        })
                        .inner
                        && app.clear_app_cache()
                    {
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(egui::Id::new("settings-cache-cleared"), true)
                        });
                    }
                    if ui
                        .ctx()
                        .data(|data| data.get_temp::<bool>(egui::Id::new("settings-cache-cleared")))
                        .unwrap_or(false)
                    {
                        ui.label(t("Cache geleert.", "Cache cleared."));
                    }
                }
                "general" => {
                    *changed |= select(
                        ui,
                        t("Oberfläche", "Interface layout"),
                        &mut app.settings.interface_layout,
                        &[
                            (InterfaceLayout::Modern, t("Modern", "Modern")),
                            (InterfaceLayout::Classic, t("Klassisch", "Classic")),
                        ],
                    );
                    *changed |= select(
                        ui,
                        t("App-Sprache", "App language"),
                        &mut app.settings.language,
                        &[(AppLanguage::De, "Deutsch"), (AppLanguage::En, "English")],
                    );
                    let themes = [
                        (Theme::Warm, t("Blau", "Blue")),
                        (Theme::Dark, t("Dunkel", "Dark")),
                        (Theme::Light, t("Hell", "Light")),
                        (Theme::System, "System"),
                    ];
                    *changed |= select(
                        ui,
                        t("Farbschema", "Appearance"),
                        &mut app.settings.theme,
                        &themes,
                    );
                    if mobile && !super::desktop::active(ui.ctx()) {
                        use crate::types::TvMode;
                        *changed |= select(
                            ui,
                            t("TV-Modus", "TV mode"),
                            &mut app.settings.tv_mode,
                            &[
                                (TvMode::Auto, t("Automatisch", "Automatic")),
                                (TvMode::Always, t("Ein", "On")),
                                (TvMode::Never, t("Aus", "Off")),
                            ],
                        );
                    }
                }
                "playback" => {
                    if switch(
                        ui,
                        t(
                            "Fortsetzen und Wiedergabeverlauf",
                            "Continue watching and watch history",
                        ),
                        &mut app.settings.watch_history,
                    ) {
                        if !app.settings.watch_history {
                            app.clear_resume();
                            if app.series_cat == "continue" {
                                app.series_cat = "all".into();
                            }
                            if app.vod_cat == "continue" {
                                app.vod_cat = "all".into();
                            }
                        }
                        *changed = true;
                    }
                    *changed |= switch(
                        ui,
                        t(
                            "Letzten Sender beim Start öffnen",
                            "Open the last channel at startup",
                        ),
                        &mut app.settings.auto_play_last,
                    );
                    if action(
                        ui,
                        t(
                            "Automatische Wiedergabe wiederherstellen",
                            "Restore automatic playback",
                        ),
                    ) {
                        app.settings.automatic_playback();
                        *playback_changed = true;
                    }
                }
                "audio" => {
                    *playback_changed |= language_field(
                        ui,
                        t("Bevorzugte Audiosprache", "Preferred audio language"),
                        &mut app.settings.audio_language,
                        language,
                    );
                    *playback_changed |= language_field(
                        ui,
                        t("Untertitelsprache", "Subtitle language"),
                        &mut app.settings.subtitle_language,
                        language,
                    );
                    *playback_changed |= switch(
                        ui,
                        t("Untertitel automatisch", "Enable subtitles automatically"),
                        &mut app.settings.subtitles_enabled,
                    );
                    *playback_changed |= select(
                        ui,
                        t("Untertitelgröße", "Subtitle size"),
                        &mut app.settings.sub_size,
                        &[
                            (26.0, t("Klein", "Small")),
                            (32.0, t("Normal", "Normal")),
                            (40.0, t("Groß", "Large")),
                            (48.0, t("Sehr groß", "Extra large")),
                        ],
                    );
                }
                "profiles" => profiles(app, ui),
                "parental" => super::parental::entry(app, ui),
                "updates" => {
                    ui.scope(|ui| {
                        ui.label(format!("Dado Player {}", crate::release_info::RELEASE_NAME));
                        #[cfg(target_os = "windows")]
                        ui.label(format!(
                            "Installierter Build {}",
                            crate::release_info::WINDOWS_BUILD
                        ));
                        #[cfg(target_os = "windows")]
                        crate::platform::windows::update::show(ui);
                        #[cfg(any(target_os = "ios", target_os = "android"))]
                        super::mobile::update::show(ui);
                        ui.add_space(10.0);
                        crate::fonts::license(ui);
                    });
                }
                "license" => {
                    #[cfg(target_os = "windows")]
                    crate::licensing::settings(ui);
                }
                "advanced" => {
                    let mut enabled = app.settings.settings_mode == SettingsMode::Expert;
                    if switch(ui, t("Expertenmodus", "Expert mode"), &mut enabled) {
                        app.settings.settings_mode = if enabled {
                            SettingsMode::Expert
                        } else {
                            SettingsMode::Easy
                        };
                        if !enabled {
                            app.settings.automatic_playback();
                            *playback_changed = true;
                        }
                        *changed = true;
                    }
                    if enabled {
                        if mobile {
                            super::mobile::settings::advanced(app, ui, playback_changed);
                        } else {
                            let before = app.settings.live_format;
                            *playback_changed |= select(
                                ui,
                                t("Live-Format", "Live stream format"),
                                &mut app.settings.live_format,
                                &crate::types::LiveFormat::all().map(|v| (v, v.label())),
                            );
                            if before != app.settings.live_format {
                                app.apply_live_format();
                            }
                            ui.add(super::text_field(
                                &mut app.settings_q,
                                t("Option suchen …", "Find an option …"),
                                None,
                                |edit| edit,
                            ));
                            let query = app.settings_q.trim().to_lowercase();
                            expert(app, ui, &query, playback_changed);
                        }
                    }
                }
                _ => {}
            }
        });
}

fn language_field(ui: &mut Ui, label: &str, value: &mut String, language: AppLanguage) -> bool {
    #[derive(serde::Deserialize)]
    struct Language {
        code: String,
        de: String,
        en: String,
    }
    static LANGUAGES: std::sync::OnceLock<Vec<Language>> = std::sync::OnceLock::new();
    let languages = LANGUAGES.get_or_init(|| {
        serde_json::from_str(include_str!("../../shared/playback-languages.json"))
            .expect("bundled playback languages")
    });
    let mut selected = languages
        .iter()
        .position(|option| option.code == *value)
        .unwrap_or(languages.len());
    let mut choices: Vec<_> = languages
        .iter()
        .enumerate()
        .map(|(i, option)| (i, language.text(&option.de, &option.en)))
        .collect();
    choices.push((languages.len(), language.text("Andere …", "Other …")));
    let custom_key = ui.id().with(("custom-language", label));
    let editing = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(custom_key).unwrap_or(false));
    if editing {
        selected = languages.len();
    }
    let mut changed = false;
    if select(ui, label, &mut selected, &choices) {
        let custom = selected == languages.len();
        ui.ctx()
            .data_mut(|data| data.insert_temp(custom_key, custom));
        if let Some(choice) = languages.get(selected) {
            value.clone_from(&choice.code);
            changed = true;
        }
    }
    if selected == languages.len() {
        let custom_label = language.text("Eigener Sprachcode", "Custom language code");
        if super::mobile::active(ui.ctx()) {
            changed |= super::mobile::components::field(
                ui,
                custom_label,
                value,
                "ISO 639",
                super::mobile::components::Kb::Text,
                &mut false,
            )
            .changed();
        } else {
            ui.label(custom_label);
            changed |= ui
                .add(super::text_field(value, "ISO 639", None, |edit| edit))
                .changed();
        }
    }
    changed
}

fn expert(app: &mut App, ui: &mut Ui, q: &str, changed: &mut bool) {
    // ---------- presets ----------
    if sec(ui, q, "Schnellauswahl") {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Standard").clicked() {
                app.settings.preset_standard();
                *changed = true;
            }
            if ui
                .button("Schonmodus (schwacher PC)")
                .on_hover_text("Wenig GPU-Last, kleiner Puffer")
                .clicked()
            {
                app.settings.preset_light();
                *changed = true;
            }
            if ui
                .button("Maximale Qualität")
                .on_hover_text("Interpolation + Debanding, größerer Puffer")
                .clicked()
            {
                app.settings.preset_quality();
                *changed = true;
            }
        });
    }

    // ---------- picture / decoder ----------
    if sec(ui, q, "Bild & Decoder") {
        combo_row(ui, "Hardware-Dekodierung", "hwdec", |ui| {
            egui::ComboBox::from_id_source("hwdec")
                .selected_text(app.settings.hwdec.label())
                .width(230.0)
                .show_ui(ui, |cb| {
                    for v in [
                        HwDec::AutoSafe,
                        HwDec::Auto,
                        HwDec::Off,
                        HwDec::D3d11va,
                        HwDec::Nvdec,
                        HwDec::Vulkan,
                    ] {
                        if cb
                            .selectable_value(&mut app.settings.hwdec, v, v.label())
                            .clicked()
                        {
                            *changed = true;
                        }
                    }
                });
        });
        ui.add(egui::Label::new(
            RichText::new(
                "Bei Problemen: „Aus (nur CPU)“ ist am kompatibelsten, braucht aber mehr Leistung.",
            )
            .size(10.5)
            .weak(),
        ));

        combo_row(ui, "Video-Ausgabe", "vo", |ui| {
            egui::ComboBox::from_id_source("vo")
                .selected_text(app.settings.vo.label())
                .width(230.0)
                .show_ui(ui, |cb| {
                    for v in [VideoOut::Gpu, VideoOut::GpuNext, VideoOut::Direct3d] {
                        if cb
                            .selectable_value(&mut app.settings.vo, v, v.label())
                            .clicked()
                        {
                            *changed = true;
                        }
                    }
                });
        });

        combo_row(ui, "Bildsynchronisation", "vsync", |ui| {
            egui::ComboBox::from_id_source("vsync")
                .selected_text(app.settings.video_sync.label())
                .width(230.0)
                .show_ui(ui, |cb| {
                    for v in [VideoSync::Audio, VideoSync::DisplayResample] {
                        if cb
                            .selectable_value(&mut app.settings.video_sync, v, v.label())
                            .clicked()
                        {
                            *changed = true;
                        }
                    }
                });
        });

        *changed |= ui
            .checkbox(
                &mut app.settings.interpolation,
                "Interpolation (flüssigere Bewegung, mehr GPU-Last)",
            )
            .changed();
        *changed |= ui
            .checkbox(&mut app.settings.deband, "Debanding (weniger Farbstufen)")
            .changed();
    }

    // ---------- audio ----------
    if sec(ui, q, "Ton") {
        *changed |= ui
            .checkbox(
                &mut app.settings.audio_exclusive,
                "Exklusiver Audio-Modus (WASAPI)",
            )
            .on_hover_text("Direkter Zugriff auf die Soundkarte. Kann besser klingen, aber andere Apps verlieren den Ton.")
            .changed();
        *changed |= ui
            .checkbox(
                &mut app.settings.audio_passthrough,
                "Surround durchreichen (AC3/DTS/TrueHD)",
            )
            .on_hover_text("Nur sinnvoll mit AV-Receiver über HDMI.")
            .changed();
        *changed |= ui
            .checkbox(
                &mut app.settings.audio_normalize,
                "Lautstärke normalisieren",
            )
            .on_hover_text("Gleicht leise und laute Sender an.")
            .changed();
    }

    // ---------- subtitles ----------
    if sec(ui, q, "Untertitel") {
        ui.horizontal(|ui| {
            ui.add_sized(
                [170.0, 24.0],
                egui::Label::new(RichText::new("Schriftgröße").size(13.0)),
            );
            *changed |= ui
                .add(egui::Slider::new(&mut app.settings.sub_size, 20.0..=120.0))
                .changed();
        });
    }

    // ---------- network ----------
    if sec(ui, q, "Netzwerk & Puffer") {
        ui.horizontal(|ui| {
            ui.add_sized(
                [170.0, 24.0],
                egui::Label::new(RichText::new("Puffer (Sekunden)").size(13.0)),
            );
            if ui
                .add(egui::Slider::new(&mut app.settings.cache_secs, 5..=180))
                .on_hover_text(
                    "Mehr Puffer = stabiler bei schwacher Leitung, aber langsamerer Start.",
                )
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.add_sized(
                [170.0, 24.0],
                egui::Label::new(RichText::new("Netzwerk-Timeout").size(13.0)),
            );
            *changed |= ui
                .add(egui::Slider::new(&mut app.settings.network_timeout_s, 5..=60).suffix(" s"))
                .changed();
        });
        ui.add_space(4.0);
        ui.add(egui::Label::new(
            RichText::new("User-Agent (leer = Standard)")
                .size(11.5)
                .weak(),
        ));
        if ui
            .add(super::text_field(
                &mut app.settings.user_agent,
                "z.B. VLC/3.0.20 LibVLC/3.0.20",
                None,
                |edit| edit.desired_width(360.0),
            ))
            .changed()
        {
            *changed = true;
        }
        *changed |= ui
            .checkbox(
                &mut app.settings.tls_verify,
                "TLS-Zertifikat des Anbieters streng prüfen",
            )
            .on_hover_text(
                "Standardmäßig aus: viele IPTV-Anbieter nutzen selbstsignierte \
                 Zertifikate auf ihrem Stream-Port. Mit aktivierter Prüfung bleibt \
                 der Player dann schwarz. Nur einschalten, wenn der Anbieter ein \
                 gültiges Zertifikat hat.",
            )
            .changed();
        ui.add(egui::Label::new(
            RichText::new(
                "Bei selbstsignierten Anbieter-Zertifikaten (häufig bei IPTV) muss diese Option aus bleiben.",
            )
            .size(10.5)
            .weak(),
        ));
    }

    // ---------- general ----------
    if sec(ui, q, "Allgemein")
        && ui
            .checkbox(
                &mut app.settings.auto_play_last,
                "Letzten Kanal beim Start automatisch abspielen",
            )
            .changed()
    {
        app.save_settings_now();
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(16.0);
    let color = crate::theme::muted(ui);
    ui.add(egui::Label::new(
        RichText::new(title.to_uppercase())
            .size(10.5)
            .strong()
            .color(color),
    ));
    ui.add_space(6.0);
}

fn combo_row(ui: &mut Ui, label: &str, id: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [170.0, 22.0],
            egui::Label::new(RichText::new(label).size(13.0)),
        );
        ui.push_id(id, add);
    });
    ui.add_space(2.0);
}
