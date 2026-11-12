//! Phone "Fortsetzen" list (recently watched, newest first).

use egui::{Align2, Color32, FontId, Rect, Sense, Ui, Vec2};

use super::components::{self, Icon, Row};
use super::focus;
use super::tokens::{GAP, NAV_H};
use super::Layout;
use crate::app::App;

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
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

    let mut play = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            for h in app.history.clone() {
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
            ui.add_space(NAV_H + GAP);
        });

    if let Some(item) = play {
        let pos = app.get_progress(&item.progress_key);
        if item.kind != crate::types::PlayKind::Live && pos > 1 {
            app.pending_resume = Some((item.progress_key.clone(), item, pos));
        } else {
            app.play(item);
        }
    }
    mobile_resume_dialog(app, ui);
}

fn mobile_resume_dialog(app: &mut App, ui: &mut Ui) {
    let Some((key, item, pos)) = app.pending_resume.clone() else {
        return;
    };
    let screen = ui.ctx().screen_rect();
    let mut resume = false;
    let mut restart = false;
    let mut cancel = false;
    egui::Area::new(egui::Id::new("mobile-continue-resume-dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(screen.size());
            let scrim = ui.interact(screen, ui.id().with("mobile-resume-scrim"), Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(190));
            let w = 360.0f32.min(screen.width() - 32.0).max(260.0);
            let h = 220.0;
            let card = Rect::from_center_size(screen.center(), Vec2::new(w, h));
            let fill = if ui.visuals().dark_mode {
                crate::theme::PANEL_DARK
            } else {
                crate::theme::PANEL_LIGHT
            };
            ui.painter().rect_filled(card, 16.0, fill);
            ui.painter().text(
                card.min + Vec2::new(24.0, 24.0),
                Align2::LEFT_TOP,
                "Wiedergabe fortsetzen?",
                FontId::proportional(20.0),
                ui.visuals().text_color(),
            );
            ui.painter().text(
                card.min + Vec2::new(24.0, 66.0),
                Align2::LEFT_TOP,
                format!(
                    "{}\nZuletzt gesehen bei {}",
                    item.title,
                    crate::ui::fmt_time(pos as f64)
                ),
                FontId::proportional(16.0),
                ui.visuals().text_color(),
            );
            let bw = (w - 64.0) / 2.0;
            let y = card.bottom() - 72.0;
            let r1 = Rect::from_min_size(egui::pos2(card.left() + 24.0, y), Vec2::new(bw, 48.0));
            let r2 =
                Rect::from_min_size(egui::pos2(card.left() + 40.0 + bw, y), Vec2::new(bw, 48.0));
            resume = ui
                .put(
                    r1,
                    egui::Button::new("Fortsetzen").fill(crate::theme::accent_fill(ui)),
                )
                .clicked();
            restart = ui
                .put(r2, egui::Button::new("Von vorne beginnen"))
                .clicked();
            if scrim.clicked() {
                cancel = true;
            }
        });
    let escape = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
    if resume {
        app.pending_resume = None;
        app.play(item);
    } else if restart {
        app.pending_resume = None;
        app.progress.remove(&key);
        let _ = crate::store::save_progress(&app.progress);
        app.play(item);
    } else if cancel || escape {
        app.pending_resume = None;
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
