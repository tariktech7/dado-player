//! Phone favorites list.

use egui::{pos2, vec2, Rect, Ui};

use super::components::{self, Icon, Row};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    if app.favorites.is_empty() {
        components::empty_state(
            ui,
            Icon::Star,
            "Noch keine Favoriten",
            "Tippe in einer Liste auf den Stern.",
            None,
        );
        return;
    }

    let mut keys: Vec<String> = app.favorites.iter().cloned().collect();
    // Sort by display title, not by the internal key: "live:44" / "ep:12:3"
    // sorts alphabetically into an order that looks random to the user.
    keys.sort_by_cached_key(|k| {
        app.describe_key(k)
            .map(|(title, _)| title.to_lowercase())
            .unwrap_or_else(|| k.clone())
    });

    let mut play = None;
    let mut remove: Option<String> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            for key in &keys {
                let Some((title, sub)) = app.describe_key(key) else {
                    continue;
                };
                let (img_key, img_url) = image_for(app, key);
                let row = Row::new(&title).img(&img_key, &img_url).sub(&sub).no_play();
                let resp = components::list_row(app, ui, row, l.pad);
                let row_hit = focus::clicked(ui, &resp);

                // star on the right (own 48 dp hit area)
                let star = Rect::from_center_size(
                    pos2(resp.rect.right() - 28.0, resp.rect.center().y),
                    vec2(TOUCH, TOUCH),
                );
                let sr = ui.interact(star, ui.id().with(("unfav", key)), egui::Sense::click());
                components::icon(
                    ui,
                    Rect::from_center_size(star.center(), vec2(24.0, 24.0)),
                    Icon::StarOn,
                    accent_text(ui),
                );
                let star_hit = focus::clicked(ui, &sr);
                if star_hit {
                    remove = Some(key.clone());
                } else if row_hit {
                    play = app.resolve_play(key);
                }
            }
            // Bottom safe-area: clear the bottom nav bar (NAV_H) plus a gap so the
            // last row can scroll fully above the tab bar instead of straddling it.
            ui.add_space(NAV_H + GAP);
        });

    if let Some(k) = remove {
        app.toggle_favorite(&k);
    }
    if let Some(item) = play {
        app.play(item);
    }
    let _ = l;
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
