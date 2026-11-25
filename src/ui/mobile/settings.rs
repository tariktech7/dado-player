//! Playback options specific to the native mobile hosts.

use super::{components, focus, tokens::*};
#[cfg(not(target_os = "ios"))]
use crate::types::HwDec;
use crate::{app::App, types::LiveFormat};
use egui::Ui;

pub fn advanced(app: &mut App, ui: &mut Ui, changed: &mut bool) {
    let mut live_format_changed = false;
    let mut reveal = false;
    #[cfg(target_os = "ios")]
    {
        ui.label("Player-Oberfläche");
        ui.horizontal_wrapped(|ui| {
            for (label, native) in [("Dado Player", false), ("Apple Player", true)] {
                let r = components::chip(ui, label, app.settings.ios_native_player == native);
                if focus::clicked(ui, &r) {
                    app.settings.ios_native_player = native;
                    crate::platform::ios::bridge::native_player(native);
                    *changed = true;
                }
            }
        });
        ui.label("Decoder");
        ui.horizontal_wrapped(|ui| {
            use crate::types::IosPlaybackEngine as Engine;
            for (label, engine) in [
                ("Automatisch", Engine::Auto),
                ("Apple AVPlayer", Engine::Apple),
                ("VLC", Engine::Vlc),
            ] {
                let r = components::chip(ui, label, app.settings.ios_playback_engine == engine);
                if focus::clicked(ui, &r) {
                    app.settings.ios_playback_engine = engine;
                    *changed = true;
                }
            }
        });
        ui.small("Apple-Oberfläche erfordert AVPlayer; dessen unterstützte Formate gelten dann weiterhin.");
        ui.label(
            egui::RichText::new("Gilt ab der nächsten Wiedergabe.")
                .font(reg(FS_CAP))
                .color(muted(ui)),
        );
    }

    if components::stepper_row(ui, "Puffer", &mut app.settings.cache_secs, 15, 15, 300, "s") {
        *changed = true;
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
        *changed = true;
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
                    *changed = true;
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
            let r = components::chip(ui, fmt.label(), app.settings.live_format == fmt);
            if focus::clicked(ui, &r) && app.settings.live_format != fmt {
                app.settings.live_format = fmt;
                live_format_changed = true;
                *changed = true;
            }
        }
    });
    ui.add_space(8.0);
    #[cfg(not(target_os = "ios"))]
    if let Some(v) = components::switch_row(ui, "Entrauschen (deband)", app.settings.deband) {
        app.settings.deband = v;
        *changed = true;
    }

    {
        components::section(ui, "Netzwerk");
        let response = components::field(
            ui,
            "Player-User-Agent",
            &mut app.settings.user_agent,
            "Leer = Standard",
            components::Kb::Text,
            &mut reveal,
        );
        crate::ui::settings::edit_option(ui, &response);
        *changed |= response.changed();
        #[cfg(target_os = "ios")]
        ui.small("Ein eigener Player-User-Agent wird ab iOS 16 unterstützt.");
    }
    if live_format_changed {
        app.apply_live_format();
    }
}
