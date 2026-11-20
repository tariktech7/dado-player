use crate::app::App;
use crate::types::{HwDec, SettingsMode, Theme, VideoOut, VideoSync};
use crate::ui;
use egui::{RichText, Ui};

/// Content is kept in a readable column instead of stretching across the
/// whole window on wide screens.
const COL_W: f32 = 720.0;

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

pub fn show(app: &mut App, ui: &mut Ui) {
    if super::desktop::active(ui.ctx()) {
        show_desktop(app, ui);
        return;
    }
    let mut player_changed = false;
    let mut theme_changed = false;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_width();
            let col = avail.min(COL_W);
            let pad = ((avail - col) * 0.5).max(0.0);

            ui.horizontal_top(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_max_width(col);
                    column(app, ui, &mut player_changed, &mut theme_changed);
                });
            });
        });

    if player_changed || theme_changed {
        app.save_settings_now();
        ui.ctx().request_repaint();
    }
    if player_changed {
        // Recreate mpv so the new decoder/audio options are actually used.
        app.reset_player();
        app.info = Some("Player mit neuen Einstellungen neu gestartet.".to_string());
    }
}

fn desktop_card(ui: &mut Ui, row: usize, title: &str, add: impl FnOnce(&mut Ui)) {
    let s = super::desktop::scale(ui.ctx());
    let key = egui::Id::new((
        "desktop-settings-row",
        row,
        ui.available_width().round() as i32,
    ));
    let minimum = ui.ctx().data(|d| d.get_temp::<f32>(key).unwrap_or(0.0));
    let response = egui::Frame::none()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().window_stroke)
        .rounding(20.0 * s)
        .inner_margin(egui::Margin::same(22.0 * s))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(minimum);
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::proportional(34.0 * s),
            );
            if !title.is_empty() {
                ui.label(RichText::new(title).size(34.0 * s).strong());
                ui.add_space(12.0 * s);
            }
            add(ui);
        });
    let measured = response.response.rect.height() - 44.0 * s;
    if measured > minimum + 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(key, measured));
        ui.ctx().request_repaint();
    }
}

fn show_desktop(app: &mut App, ui: &mut Ui) {
    let s = super::desktop::scale(ui.ctx());
    let mut player_changed = false;
    let mut theme_changed = false;
    egui::ScrollArea::vertical().id_source("desktop-settings-page").auto_shrink([false,false]).show(ui,|ui|{
        ui.small(format!("DADO PLAYER · WINDOWS {}",crate::release_info::VERSION));
        ui.label(RichText::new("Dein Bildschirm. Dein Bild.").size(58.0*s).strong());
        ui.label("Einstellungen bleiben auf diesem Gerät gespeichert.");ui.add_space(20.0*s);
        ui.columns(2,|columns|{
            desktop_card(&mut columns[0],0,"",|ui|{
                #[cfg(target_os="windows")] crate::licensing::settings(ui);
            });
            desktop_card(&mut columns[1],0,"Darstellung",|ui|{
                let mut expert=app.settings.settings_mode==SettingsMode::Expert;
                if ui.checkbox(&mut expert,"Expertenmodus").changed(){app.settings.settings_mode=if expert{SettingsMode::Expert}else{SettingsMode::Easy};if !expert{app.settings.automatic_playback();player_changed=true;}theme_changed=true;}
                ui.small("Automatische Wiedergabe mit Dado-Steuerung. Weitere Optionen im Expertenmodus.");
                ui.label("Farbschema");
                egui::ComboBox::from_id_source("desktop-theme").width(ui.available_width()).selected_text(app.settings.theme.label()).show_ui(ui,|ui|{
                    for theme in Theme::all(){theme_changed|=ui.selectable_value(&mut app.settings.theme,theme,theme.label()).changed();}
                });
                theme_changed|=ui.checkbox(&mut app.settings.auto_play_last,"Letzten Sender beim Start öffnen").changed();
            });
        });
        ui.add_space(16.0*s);
        ui.columns(2,|columns|{
            desktop_card(&mut columns[0],1,"Wiedergabe",|ui|{
                ui.label("Integrierter Player · Dado-Steuerung");
                ui.small("Bild und Ton werden automatisch für diesen Rechner eingerichtet.");
                ui.label("Live-Format");
                let before=app.settings.live_format;
                egui::ComboBox::from_id_source("desktop-live-format").width(ui.available_width()).selected_text(before.label()).show_ui(ui,|ui|{
                    for format in crate::types::LiveFormat::all(){ui.selectable_value(&mut app.settings.live_format,format,format.label());}
                });
                if before!=app.settings.live_format{app.apply_live_format();player_changed=true;}
                ui.label("Puffer in Sekunden");
                player_changed|=ui.add(egui::Slider::new(&mut app.settings.cache_secs,0..=120)).changed();
                if ui.button("Automatische Einstellungen verwenden").clicked(){app.settings.automatic_playback();player_changed=true;}
            });
            desktop_card(&mut columns[1],1,"Ton & Untertitel",|ui|{
                ui.label("Bevorzugte Audiosprache");
                player_changed|=ui.add(egui::TextEdit::singleline(&mut app.settings.audio_language).hint_text("z. B. de, en, tr").desired_width(ui.available_width())).changed();
                ui.label("Bevorzugte Untertitelsprache");
                player_changed|=ui.add(egui::TextEdit::singleline(&mut app.settings.subtitle_language).hint_text("z. B. de").desired_width(ui.available_width())).changed();
                player_changed|=ui.checkbox(&mut app.settings.subtitles_enabled,"Untertitel aktivieren").changed();
                ui.label("Untertitelgröße");player_changed|=ui.add(egui::Slider::new(&mut app.settings.sub_size,16.0..=64.0)).changed();
            });
        });
        ui.add_space(16.0*s);
        ui.columns(2,|columns|{
            desktop_card(&mut columns[0],2,"Netzwerk",|ui|{
                ui.label("Verbindungszeitlimit in Sekunden");
                player_changed|=ui.add(egui::Slider::new(&mut app.settings.network_timeout_s,5..=90)).changed();
                if app.settings.settings_mode==SettingsMode::Expert {
                    ui.label("User-Agent");player_changed|=ui.text_edit_singleline(&mut app.settings.user_agent).changed();
                }else{ui.small("Automatische Verbindungs- und Formatauswahl. Anbieter-Zugangsdaten bleiben im Profil.");}
            });
            desktop_card(&mut columns[1],2,"",|ui|super::parental::entry(app,ui));
        });
        ui.add_space(16.0*s);
        ui.columns(2,|columns|{
            desktop_card(&mut columns[0],3,"App-Updates",|ui|{
                #[cfg(target_os="windows")]crate::platform::windows::update::show(ui);
                ui.small("Updates werden in der portablen Version über GitHub geprüft und installiert.");
                ui.label(format!("Version {}",crate::release_info::VERSION));
            });
            desktop_card(&mut columns[1],3,"Profile",|ui|app.cloud_profiles_ui(ui));
        });
        if app.settings.settings_mode==SettingsMode::Expert {
            ui.add_space(16.0*s);
            desktop_card(ui,4,"Weitere Wiedergabeoptionen",|ui|{
                ui.add(egui::TextEdit::singleline(&mut app.settings_q).hint_text("Experteneinstellung suchen …").desired_width(ui.available_width()));
                let query=app.settings_q.trim().to_lowercase();expert(app,ui,&query,&mut player_changed);
            });
        }
        ui.add_space(20.0*s);crate::fonts::license(ui);
    });
    if player_changed || theme_changed {
        app.save_settings_now();
        ui.ctx().request_repaint();
    }
    if player_changed {
        app.reset_player();
    }
}

fn column(app: &mut App, ui: &mut Ui, player_changed: &mut bool, theme_changed: &mut bool) {
    crate::ui::parental::entry(app, ui);
    app.cloud_profiles_ui(ui);
    ui.add_space(6.0);
    ui.add(egui::Label::new(
        RichText::new("Einstellungen").size(22.0).strong(),
    ));
    ui.add_space(8.0);

    // ---------- filter ----------
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.settings_q)
                .hint_text("Einstellung suchen… (z. B. Puffer, Ton, Decoder)")
                .desired_width(330.0),
        );
        if !app.settings_q.is_empty() && ui.button("×").clicked() {
            app.settings_q.clear();
        }
    });

    // Read after the field so the list filters on the same frame as the input.
    let q_owned = app.settings_q.trim().to_lowercase();
    let q = q_owned.as_str();

    ui.horizontal(|ui| {
        if !q.is_empty() {
            let n = SECTIONS.iter().filter(|(t, _)| hits(q, t)).count();
            ui.add(egui::Label::new(
                RichText::new(format!("{} passende Bereiche", n))
                    .size(11.0)
                    .weak(),
            ));
        }
    });
    ui.add_space(10.0);
    #[cfg(target_os = "windows")]
    if hits(q, "Lizenz & Gerät") {
        crate::licensing::settings(ui);
    }

    if !q.is_empty() && !SECTIONS.iter().any(|(t, _)| hits(q, t)) {
        ui.add_space(8.0);
        ui.add(egui::Label::new(
            RichText::new(format!("Nichts gefunden für „{}“.", app.settings_q.trim()))
                .size(13.0)
                .weak(),
        ));
        ui.add_space(6.0);
        if ui.button("Filter löschen").clicked() {
            app.settings_q.clear();
        }
    }

    // ---------- easy / expert ----------
    let mode = app.settings.settings_mode;
    let labels = ["Einfach", "Experten"];
    let cur = if mode == SettingsMode::Easy { 0 } else { 1 };
    if let Some(i) = ui::segmented(ui, "settings_mode", &labels, cur) {
        app.settings.settings_mode = if i == 0 {
            SettingsMode::Easy
        } else {
            SettingsMode::Expert
        };
        if i == 0 {
            app.settings.automatic_playback();
            *player_changed = true;
        }
        app.save_settings_now();
    }
    ui.add_space(4.0);
    ui.add(egui::Label::new(
        RichText::new(match mode {
            SettingsMode::Easy => {
                "Nur das Wesentliche. Für Bild- und Ton-Feintuning auf „Experten“ wechseln."
            }
            SettingsMode::Expert => {
                "Alle Optionen. Je nach Grafikkarte, Treiber und Windows-Version kann \
                 eine andere Kombination besser laufen."
            }
        })
        .size(11.5)
        .weak(),
    ));

    if sec(ui, q, "Sprachen") {
        ui.label("Bevorzugte Audiosprache");
        *player_changed |= ui
            .text_edit_singleline(&mut app.settings.audio_language)
            .changed();
        ui.label("Bevorzugte Untertitelsprache");
        *player_changed |= ui
            .text_edit_singleline(&mut app.settings.subtitle_language)
            .changed();
        *player_changed |= ui
            .checkbox(
                &mut app.settings.subtitles_enabled,
                "Untertitel standardmäßig aktiv",
            )
            .changed();
        ui.small("Sprachcodes z. B. de, en, tr. Gilt ab der nächsten Wiedergabe.");
    }

    // ---------- appearance ----------
    if sec(ui, q, "Darstellung") {
        ui.horizontal(|ui| {
            ui.add_sized(
                [170.0, 24.0],
                egui::Label::new(RichText::new("Farbschema").size(13.0)),
            );
            let cur = Theme::all()
                .iter()
                .position(|t| *t == app.settings.theme)
                .unwrap_or(0);
            if let Some(i) = ui::segmented(ui, "theme", &["Blau", "Dunkel", "Hell", "System"], cur)
            {
                app.settings.theme = Theme::all()[i];
                *theme_changed = true;
            }
        });
        ui.add_space(2.0);
        ui.add(egui::Label::new(
            RichText::new("„System“ übernimmt automatisch die Windows-Einstellung.")
                .size(10.5)
                .weak(),
        ));
    }

    if app.settings.settings_mode == SettingsMode::Easy {
        easy(app, ui, q, player_changed);
    } else {
        expert(app, ui, q, player_changed);
    }

    ui.add_space(18.0);
    ui.separator();
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui::brand_mark(ui, 30.0);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(
                RichText::new("Dado-Player").size(15.0).strong(),
            ));
            ui.add(egui::Label::new(
                RichText::new(format!(
                    "Version {} · Rust + libmpv",
                    include_str!("../../windows/VERSION").trim()
                ))
                .size(11.0)
                .weak(),
            ));
        });
    });
    #[cfg(target_os = "windows")]
    crate::platform::windows::update::show(ui);
    crate::fonts::license(ui);
    ui.add_space(4.0);
    ui.add(egui::Label::new(
        RichText::new("Änderungen werden automatisch gespeichert.")
            .size(10.5)
            .weak(),
    ));
    ui.add_space(18.0);
}

// ---------------------------------------------------------------- easy

fn easy(app: &mut App, ui: &mut Ui, q: &str, changed: &mut bool) {
    if sec(ui, q, "Wiedergabe") {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Standard")
                .on_hover_text("Läuft auf fast jedem Gerät")
                .clicked()
            {
                app.settings.preset_standard();
                *changed = true;
            }
            if ui
                .button("Schwacher PC")
                .on_hover_text("Wenig GPU-Last, kleinerer Puffer")
                .clicked()
            {
                app.settings.preset_light();
                *changed = true;
            }
            if ui
                .button("Beste Qualität")
                .on_hover_text("Ruhigeres Bild, mehr GPU-Last")
                .clicked()
            {
                app.settings.preset_quality();
                *changed = true;
            }
        });
    }

    if sec(ui, q, "Puffer") {
        ui.horizontal(|ui| {
            ui.add_sized(
                [170.0, 24.0],
                egui::Label::new(RichText::new("Puffergröße").size(13.0)),
            );
            if ui
                .add(egui::Slider::new(&mut app.settings.cache_secs, 10..=120).suffix(" s"))
                .on_hover_text(
                    "Mehr Puffer = stabiler bei schwacher Leitung, aber langsamerer Start.",
                )
                .changed()
            {
                *changed = true;
            }
        });
    }

    if sec(ui, q, "Start")
        && ui
            .checkbox(
                &mut app.settings.auto_play_last,
                "Zuletzt gesehenen Kanal beim Start öffnen",
            )
            .changed()
    {
        app.save_settings_now();
    }
}

// ---------------------------------------------------------------- expert

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

#[cfg(test)]
mod desktop_alignment_tests {
    #[test]
    fn paired_settings_cards_share_height_after_content_measurement() {
        let ctx = egui::Context::default();
        let mut heights = [0.0; 2];
        for _ in 0..4 {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            ));
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.columns(2, |columns| {
                        for i in 0..2 {
                            let top = columns[i].cursor().top();
                            super::desktop_card(&mut columns[i], 0, "Settings", |ui| {
                                ui.allocate_space(egui::vec2(
                                    100.0,
                                    if i == 0 { 170.0 } else { 50.0 },
                                ));
                            });
                            heights[i] = columns[i].cursor().top() - top;
                        }
                    });
                });
            });
        }
        assert!((heights[0] - heights[1]).abs() < 1.0, "{heights:?}");
    }
}
