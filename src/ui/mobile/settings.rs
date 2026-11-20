//! Phone settings: grouped rows, one control per line, no dense forms.

use egui::Ui;

use super::components;
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
#[cfg(not(target_os = "ios"))]
use crate::types::{HwDec, PlayerBackend};
use crate::types::{LiveFormat, SettingsMode, Theme, TvMode};

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    let mut changed = false;
    let mut live_format_changed = false;
    // Once per frame, not per write: see `storage_banner`.
    let storage_failed = crate::store::storage_failed();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(l.pad);
                ui.vertical(|ui| {
                    ui.set_width(l.content_w());
                    components::storage_banner(ui, storage_failed);
                    crate::ui::parental::entry(app,ui);
                    app.cloud_profiles_ui(ui);
                    #[cfg(target_os = "windows")]
                    crate::licensing::settings(ui);

                    components::section(ui, "Einstellungen");
                    ui.horizontal_wrapped(|ui| {
                        for (label, mode) in [("Automatisch", SettingsMode::Easy), ("Expertenmodus", SettingsMode::Expert)] {
                            let r = components::chip(ui, label, app.settings.settings_mode == mode);
                            if focus::clicked(ui, &r) {
                                app.settings.settings_mode = mode;
                                if mode == SettingsMode::Easy { app.settings.automatic_playback(); }
                                changed = true;
                            }
                        }
                    });
                    let expert = app.settings.settings_mode == SettingsMode::Expert;
                    if !expert { ui.small("Integrierte Wiedergabe mit automatischer Formatauswahl. Weitere Player- und Netzwerkoptionen im Expertenmodus."); }

                    // ---- appearance ----
                    components::section(ui, "Darstellung");
                    let labels: Vec<(String, bool)> = Theme::all()
                        .iter()
                        .map(|t| (t.label().to_string(), app.settings.theme == *t))
                        .collect();
                    ui.horizontal_wrapped(|ui| {
                        for (i, (label, sel)) in labels.iter().enumerate() {
                            let r = components::chip(ui, label, *sel);
                            if focus::clicked(ui, &r) {
                                app.settings.theme = Theme::all()[i];
                                changed = true;
                            }
                        }
                    });

                    // ---- 10-foot layout ----
                    // "Automatisch" follows UiModeManager. The explicit values
                    // matter: plenty of TV boxes do not declare the leanback
                    // feature, and a phone casting to a TV must stay touch-first.
                    components::section(ui, "Fernsehen");
                    ui.label(
                        egui::RichText::new(
                            "TV-Modus: linke Menüleiste, Kartenreihen, größere Schrift",
                        )
                        .font(reg(FS_LABEL))
                        .color(muted(ui)),
                    );
                    ui.add_space(6.0);
                    let tvs: Vec<(String, bool)> = TvMode::all()
                        .iter()
                        .map(|m| (m.label().to_string(), app.settings.tv_mode == *m))
                        .collect();
                    ui.horizontal_wrapped(|ui| {
                        for (i, (label, sel)) in tvs.iter().enumerate() {
                            let r = components::chip(ui, label, *sel);
                            if focus::clicked(ui, &r) {
                                app.settings.tv_mode = TvMode::all()[i];
                                changed = true;
                            }
                        }
                    });

                    // ---- playback ----
                    components::section(ui, "Wiedergabe");
                    #[cfg(target_os = "ios")]
                    if expert {
                        ui.label("Player-Oberfläche");
                        ui.horizontal_wrapped(|ui| {
                            for (label, native) in [("Dado Player", false), ("Apple Player", true)]
                            {
                                let r = components::chip(
                                    ui,
                                    label,
                                    app.settings.ios_native_player == native,
                                );
                                if focus::clicked(ui, &r) {
                                    app.settings.ios_native_player = native;
                                    crate::platform::ios::bridge::native_player(native);
                                    changed = true;
                                }
                            }
                        });
                        ui.label("Decoder");
                        ui.horizontal_wrapped(|ui| {
                            use crate::types::IosPlaybackEngine as Engine;
                            for (label, engine) in [("Automatisch", Engine::Auto), ("Apple AVPlayer", Engine::Apple), ("VLC", Engine::Vlc)] {
                                let r = components::chip(ui, label, app.settings.ios_playback_engine == engine);
                                if focus::clicked(ui, &r) { app.settings.ios_playback_engine = engine; changed = true; }
                            }
                        });
                        ui.small("Apple-Oberfläche erfordert AVPlayer; dessen unterstützte Formate gelten dann weiterhin.");
                        ui.label(
                            egui::RichText::new("Gilt ab der nächsten Wiedergabe.")
                                .font(reg(FS_CAP))
                                .color(muted(ui)),
                        );
                    }
                    if let Some(v) = components::switch_row(
                        ui,
                        "Zuletzt gesehenen Sender starten",
                        app.settings.auto_play_last,
                    ) {
                        app.settings.auto_play_last = v;
                        changed = true;
                    }
                    if expert {
                    if components::stepper_row(
                        ui,
                        "Puffer",
                        &mut app.settings.cache_secs,
                        15,
                        15,
                        300,
                        "s",
                    ) {
                        changed = true;
                    }
                    if components::stepper_row(
                        ui,
                        "Zeitlimit Netzwerk",
                        &mut app.settings.network_timeout_s,
                        5,
                        5,
                        120,
                        "s",
                    ) {
                        changed = true;
                    }


                    #[cfg(not(target_os = "ios"))]
                    {
                        // ---- picture ----
                        components::section(ui, "Bild");
                        let hw: [(HwDec, &str); 3] = [
                            (HwDec::AutoSafe, "Automatisch"),
                            (HwDec::Auto, "Immer"),
                            (HwDec::Off, "Aus"),
                        ];
                        ui.horizontal_wrapped(|ui| {
                            for (mode, label) in hw {
                                let r = components::chip(ui, label, app.settings.hwdec == mode);
                                if focus::clicked(ui, &r) {
                                    app.settings.hwdec = mode;
                                    changed = true;
                                }
                            }
                        });
                        ui.add_space(8.0);

                        // ---- player backend ----
                        components::section(ui, "Player");
                        ui.label(
                            egui::RichText::new(
                                "Standard: integrierter Player. Externe Player öffnen den Stream in \
                             einer anderen App; „Integriert\" nutzt den eingebauten \
                             libmpv-Player (auf dem Desktop immer aktiv).",
                            )
                            .font(reg(FS_CAP))
                            .color(muted(ui)),
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            for mode in PlayerBackend::all() {
                                let r = components::chip(
                                    ui,
                                    mode.label(),
                                    app.settings.player_backend == mode,
                                );
                                if focus::clicked(ui, &r) {
                                    app.settings.player_backend = mode;
                                    changed = true;
                                }
                            }
                        });
                        ui.add_space(8.0);
                    }
                    // ---- live stream container ----
                    components::section(ui, "Live-Format");
                    ui.label(
                        egui::RichText::new(
                            "Wenn Sender nicht starten, hier zwischen MPEG-TS und \
                             HLS umschalten. Anbieter schalten nur eines von \
                             beiden frei; die Änderung gilt sofort, ohne neu \
                             anzumelden.",
                        )
                        .font(reg(FS_CAP))
                        .color(muted(ui)),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for fmt in LiveFormat::all() {
                            let r =
                                components::chip(ui, fmt.label(), app.settings.live_format == fmt);
                            if focus::clicked(ui, &r) && app.settings.live_format != fmt {
                                app.settings.live_format = fmt;
                                live_format_changed = true;
                                changed = true;
                            }
                        }
                    });
                    ui.add_space(8.0);
                    #[cfg(not(target_os = "ios"))]
                    if let Some(v) =
                        components::switch_row(ui, "Entrauschen (deband)", app.settings.deband)
                    {
                        app.settings.deband = v;
                        changed = true;
                    }

                    } // expert playback options
                    components::section(ui, "Ton & Untertitel");
                    let mut reveal = false;
                    changed |= components::field(ui, "Bevorzugte Audiosprache", &mut app.settings.audio_language, "z. B. de, en, tr", components::Kb::Text, &mut reveal).changed();
                    changed |= components::field(ui, "Bevorzugte Untertitelsprache", &mut app.settings.subtitle_language, "Leer = erste verfügbare Spur", components::Kb::Text, &mut reveal).changed();
                    if let Some(value) = components::switch_row(ui, "Untertitel standardmäßig aktiv", app.settings.subtitles_enabled) { app.settings.subtitles_enabled = value; changed = true; }
                    ui.label("Untertitelgröße");
                    changed |= ui.add(egui::Slider::new(&mut app.settings.sub_size, 20.0..=120.0)).changed();
                    ui.small("Sprachen und Größe gelten ab der nächsten Wiedergabe im integrierten Player. Externe Android-Player verwenden eigene Einstellungen.");
                    if expert {
                    components::section(ui, "Netzwerk");
                    changed |= components::field(ui, "Player-User-Agent", &mut app.settings.user_agent, "Leer = Standard", components::Kb::Text, &mut reveal).changed();
                    #[cfg(target_os = "ios")]
                    ui.small("Ein eigener Player-User-Agent wird ab iOS 16 unterstützt.");

                    }
                    // ---- profile ----
                    components::section(ui, "Profil");
                    if let Some(p) = app.active_profile.clone() {
                        let w = ui.available_width();
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(w, 56.0), egui::Sense::hover());
                        components::draw_line(
                            ui,
                            egui::pos2(rect.left() + 2.0, rect.center().y),
                            &p.name,
                            bold(FS_BODY),
                            text(ui),
                            w - 8.0,
                        );
                    }
                    ui.add_space(8.0);
                    let w = ui.available_width();
                    let r = components::button(ui, "Abmelden", w, false, true);
                    if focus::clicked(ui, &r) {
                        app.logout();
                        return;
                    }

                    #[cfg(any(target_os = "ios", target_os = "android"))]
                    super::update::show(ui);
                    // ---- about ----
                    components::section(ui, "Über");
                    crate::fonts::license(ui);
                    ui.label(
                        egui::RichText::new("Dado Player")
                            .font(reg(FS_LABEL))
                            .color(muted(ui)),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("Keine Telemetrie. Keine Werbung.")
                            .font(reg(FS_CAP))
                            .color(muted(ui)),
                    );
                    ui.add_space(24.0);
                });
            });
            // Mobile: clear the bottom nav bar (NAV_H) plus a gap.
            ui.add_space(GAP);
        });

    if live_format_changed {
        app.apply_live_format();
    }
    if changed {
        app.save_settings_now();
    }
}
