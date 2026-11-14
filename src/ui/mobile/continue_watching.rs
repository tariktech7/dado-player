//! Phone "Fortsetzen" list (recently watched, newest first).

use egui::Ui;

use super::components::{self, Icon, Row};
use super::focus;
use super::tokens::GAP;
use super::Layout;
use crate::app::App;

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    crate::ui::continue_watching::category_tabs(app, ui);
    if app.history.is_empty() {
        components::empty_state(
            ui,
            Icon::Play,
            "Noch nichts gesehen",
            "Was du startest, erscheint hier.",
            None,
        );
        return;
    }

    let prefix = match app.continue_tab {
        crate::app::ContinueTab::Live => "live:",
        crate::app::ContinueTab::Movies => "vod:",
        crate::app::ContinueTab::Series => "ep:",
    };
    let mut play = None;
    egui::ScrollArea::vertical()
        .id_source(("continue", prefix))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            for h in app
                .history
                .clone()
                .into_iter()
                .filter(|h| h.key.starts_with(prefix))
            {
                if let Some(sid) = h
                    .key
                    .strip_prefix("ep:")
                    .and_then(|v| v.split(':').next())
                    .and_then(|v| v.parse::<u32>().ok())
                {
                    app.request_series(sid);
                }
                let pos = app.get_progress(&h.key);
                let sub = if pos > 1 {
                    format!("Weiter bei {}", crate::ui::fmt_time(pos as f64))
                } else if h.sub.is_empty() {
                    "Zuletzt gesehen".to_string()
                } else {
                    h.sub.clone()
                };
                let (img_key, img_url) = image_for(app, &h.key);
                let row = Row::new(&h.title).img(&img_key, &img_url).sub(&sub);
                let r = components::list_row(app, ui, row, l.pad);
                if focus::clicked(ui, &r) {
                    play = app.resolve_play(&h.key);
                }
            }
            // Bottom safe-area: clear the bottom nav bar (NAV_H) plus a gap so the
            // last row can scroll fully above the tab bar instead of straddling it.
            ui.add_space(GAP);
        });

    if let Some(item) = play {
        app.play(item);
    }
}

fn image_for(app: &App, key: &str) -> (String, String) {
    if let Some(rest) = key.strip_prefix("live:") {
        if let Ok(id) = rest.parse::<u32>() {
            if let Some(c) = app.library.channels.iter().find(|c| c.id == id) {
                return (format!("logo:{}", id), c.logo.clone());
            }
        }
    }
    if let Some(rest) = key.strip_prefix("vod:") {
        if let Ok(id) = rest.parse::<u32>() {
            if let Some(m) = app.library.movies.iter().find(|m| m.id == id) {
                return (format!("cover:{}", id), m.cover.clone());
            }
        }
    }
    if let Some(rest) = key.strip_prefix("ep:") {
        if let Some(sid) = rest.split(':').next().and_then(|s| s.parse::<u32>().ok()) {
            if let Some(s) = app.library.series.iter().find(|s| s.id == sid) {
                return (format!("scov:{}", sid), s.cover.clone());
            }
        }
    }
    (String::new(), String::new())
}
