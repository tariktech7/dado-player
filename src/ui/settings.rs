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

fn activate(ui: &Ui, response: &egui::Response) -> bool {
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
        egui::ComboBox::from_id_source(("setting-option", label))
            .selected_text(selected)
            .width(ui.available_width().min(360.0))
            .show_ui(ui, |ui| {
                for &(option, text) in options {
                    changed |= ui.selectable_value(value, option, text).changed();
                }
            });
        return changed;
    }
    ui.horizontal_wrapped(|ui| {
        for &(option, text) in options {
            let response = ui.selectable_label(*value == option, text);
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
    let width = ui.available_width();
    let mut changed = false;
    let mut playback_changed = false;
    ui.heading(t("Einstellungen", "Settings"));
    let draft = ui.id().with("settings-draft");
    egui::ScrollArea::vertical()
        .id_source(("settings-page", app.settings_section))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(index) = app.settings_section {
                let section = &sections()[index];
                focus::set_zone(1);
                let back = ui.button(t("‹ Alle Einstellungen", "‹ All settings"));
                if app.settings_return_tile.take().is_some() {
                    back.request_focus();
                    focus::request_id(back.id);
                }
                if activate(ui, &back) {
                    app.close_settings_section();
                    return;
                }
                ui.heading(t(&section.de, &section.en));
                let mobile = super::mobile::active(ui.ctx());
                ui.push_id(draft.with(index), |ui| {
                    ui.set_width(ui.available_width().min((920.0 * scale).max(360.0)));
                    panel(
                        app,
                        ui,
                        &section.id,
                        draft.with(index).with("choice"),
                        mobile,
                        &mut changed,
                        &mut playback_changed,
                    );
                });
            } else {
                let columns = if width < 580.0 { 2 } else { 3 };
                let gap = (18.0 * scale).max(10.0);
                let tile_width =
                    ((width - (columns - 1) as f32 * gap) / columns as f32 - 2.0).max(1.0);
                let tile_height = (150.0 * scale).max(73.0);
                let tiles: Vec<_> = sections()
                    .iter()
                    .enumerate()
                    .filter(|(_, section)| section.id != "license" || cfg!(windows))
                    .collect();
                focus::set_zone(1);
                for row in tiles.chunks(columns) {
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
                            if app.settings_return_tile == Some(index) {
                                app.settings_return_tile = None;
                                response.request_focus();
                                focus::request_id(response.id);
                            }
                            let style = ui.style().interact(&response);
                            let focused =
                                response.has_focus() || focus::focused_id() == Some(response.id);
                            ui.painter().rect(
                                rect,
                                14.0,
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
                                    egui::pos2(rect.center().x, rect.top() + tile_height * 0.34),
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
                                font_size * ((tile_width - 12.0).max(1.0) / label_width).min(1.0),
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
                            if activate(ui, &response) {
                                app.settings_section = Some(index);
                                app.settings_return_tile = Some(index);
                                if section.id == "cache" {
                                    ui.ctx().data_mut(|data| {
                                        data.remove::<bool>(egui::Id::new("settings-cache-cleared"))
                                    });
                                }
                                ui.ctx().data_mut(|data| {
                                    data.remove::<usize>(draft.with(index).with("choice"))
                                });
                                ui.ctx().request_repaint();
                            }
                        }
                    });
                    ui.add_space(gap);
                }
            }
        });
    if changed || playback_changed {
        app.save_settings_now();
        ui.ctx().request_repaint();
    }
    if playback_changed && !super::mobile::active(ui.ctx()) {
        app.reset_player();
    }
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
    for (slot, (_, label)) in options.iter().enumerate() {
        let response = ui.selectable_label(slot == index, *label);
        if activate(ui, &response) {
            index = slot;
            ui.ctx().data_mut(|data| data.insert_temp(key, index));
        }
    }
    ui.add_space(10.0);
    if action(ui, save_label) {
        ui.ctx().data_mut(|data| data.remove::<usize>(key));
        return options.get(index).map(|(value, _)| *value);
    }
    None
}

fn panel(
    app: &mut App,
    ui: &mut Ui,
    section: &str,
    draft: egui::Id,
    mobile: bool,
    changed: &mut bool,
    playback_changed: &mut bool,
) {
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let save = t("Änderungen speichern", "Save changes");
    egui::Frame::none()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().window_stroke)
        .rounding(14.0)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match section {
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
                    let default = crate::types::Settings::default().live_format;
                    let options = [
                        (None, t("Standard", "Default")),
                        (Some(LiveFormat::Ts), "MPEG-TS (.ts)"),
                        (Some(LiveFormat::Hls), "HLS (.m3u8)"),
                    ];
                    ui.label(t(
                        "Gilt für Xtream-Live-TV. M3U verwendet das Format der Wiedergabeliste.",
                        "Applies to Xtream live TV. M3U uses the playlist's stream format.",
                    ));
                    if let Some(value) =
                        quick_choice(ui, draft, Some(app.settings.live_format), &options, save)
                    {
                        app.settings.live_format = value.unwrap_or(default);
                        app.apply_live_format();
                        *changed = true;
                        app.close_settings_section();
                    }
                }
                "player" => {
                    #[cfg(target_os = "android")]
                    if let Some(value) = quick_choice(
                        ui,
                        draft,
                        app.settings.effective_player_backend(),
                        &crate::types::PlayerBackend::all().map(|player| (player, player.label())),
                        save,
                    ) {
                        app.settings.settings_mode = SettingsMode::Expert;
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
                    #[cfg(not(any(target_os = "android", target_os = "ios")))]
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
                "profiles" => {
                    if let Some(profile) = &app.active_profile {
                        ui.strong(&profile.name);
                        ui.label(match profile.kind {
                            crate::types::ProfileKind::Xtream => "Xtream Codes",
                            crate::types::ProfileKind::M3u => "M3U",
                        });
                        if profile.kind == crate::types::ProfileKind::Xtream {
                            ui.label(&profile.username);
                        }
                    }
                    app.cloud_profiles_ui(ui);
                    if action(ui, t("Abmelden", "Sign out")) {
                        app.logout();
                    }
                }
                "parental" => super::parental::entry(app, ui),
                "updates" => {
                    ui.label(format!("Dado Player {}", crate::release_info::VERSION));
                    #[cfg(target_os = "windows")]
                    crate::platform::windows::update::show(ui);
                    #[cfg(any(target_os = "ios", target_os = "android"))]
                    super::mobile::update::show(ui);
                    crate::fonts::license(ui);
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
                            ui.add(
                                egui::TextEdit::singleline(&mut app.settings_q)
                                    .hint_text(t("Option suchen …", "Find an option …")),
                            );
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
                .add(egui::TextEdit::singleline(value).hint_text("ISO 639"))
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
            .add(
                egui::TextEdit::singleline(&mut app.settings.user_agent)
                    .hint_text("z.B. VLC/3.0.20 LibVLC/3.0.20")
                    .desired_width(360.0),
            )
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
