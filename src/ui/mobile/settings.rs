//! Phone settings: grouped rows, one control per line, no dense forms.

use egui::Ui;

use super::components;
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::types::{HwDec, LiveFormat, PlayerBackend, Theme, TvMode};

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
                    if let Some(v) = components::switch_row(
                        ui,
                        "Zuletzt gesehenen Sender starten",
                        app.settings.auto_play_last,
                    ) {
                        app.settings.auto_play_last = v;
                        changed = true;
                    }
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

                    #[cfg(target_os = "ios")]
                    ui.label("Wiedergabe mit Apple AVPlayer");
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
                                "Standard: MX Player. Externe Player öffnen den Stream in \
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
                    ui.label(
                        egui::RichText::new(format!("Dado-Player {}", env!("CARGO_PKG_VERSION")))
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
            ui.add_space(NAV_H + GAP);
        });

    if live_format_changed {
        app.apply_live_format();
    }
    if changed {
        app.save_settings_now();
    }
}
