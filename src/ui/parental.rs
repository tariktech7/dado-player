use crate::{app::App, parental::scoped_key, types::PlayItem};
use egui::{Context, Ui};

fn button(ui: &mut Ui, label: &str) -> bool {
    let r = ui.add_sized(
        [ui.available_width().min(360.0), 38.0],
        egui::Button::new(label),
    );
    super::settings::focus_option(ui, &r);
    clicked(ui, &r)
}
fn clicked(ui: &Ui, r: &egui::Response) -> bool {
    crate::ui::mobile::focus::clicked(ui, r)
}
impl App {
    pub fn parental_gate(&mut self, item: &PlayItem, preview: bool) -> bool {
        let profile = self
            .active_profile
            .as_ref()
            .map(|p| p.id.as_str())
            .unwrap_or("");
        let key = scoped_key(profile, &item.progress_key);
        if self.parental.grant.take().as_ref() == Some(&key) {
            return true;
        }
        if !self.parental.policy.locked(profile, &item.progress_key) {
            return true;
        }
        if self.current_play.is_some() || self.view == crate::app::View::Player {
            self.stop_play();
        }
        self.close_preview();
        self.parental.close();
        self.parental.open = true;
        self.parental.pending = Some((item.clone(), preview));
        false
    }
}
pub(super) fn open_management(app: &mut App) {
    if app.current_play.is_some() {
        app.stop_play();
    }
    app.close_preview();
    app.parental.close();
    app.parental.open = true;
}

pub fn entry(app: &mut App, ui: &mut Ui) {
    ui.heading("Elternsperre");
    if crate::ui::desktop::active(ui.ctx()) {
        ui.add_space(12.0 * crate::ui::desktop::scale(ui.ctx()));
    }
    ui.label(if app.parental.policy.enabled() {
        "Aktiv · gesperrte Inhalte benötigen die Eltern-PIN"
    } else {
        "Vierstellige PIN für Sender, Filme und Serien einrichten"
    });
    if button(
        ui,
        if app.parental.policy.enabled() {
            "Elternsperre verwalten"
        } else {
            "Eltern-PIN einrichten"
        },
    ) {
        open_management(app);
    }
    ui.add_space(12.0);
}
pub fn show(app: &mut App, ctx: &Context) {
    crate::ui::mobile::focus::begin_modal(0);
    let safe = super::desktop::safe_rect(ctx, true);
    let pin_view = !app.parental.policy.enabled()
        || app.parental.changing
        || !app.parental.authenticated
        || app.parental.pending.is_some();
    let compact = safe.height() < 450.0;
    let padding = if compact { 14.0 } else { 24.0 };
    let width = (if pin_view { 396.0_f32 } else { 620.0_f32 }).min(safe.width() - 2.0 * padding);
    let offset = safe.center() - ctx.screen_rect().center();
    let layer = egui::Area::new(egui::Id::new("parental-dialog"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, offset)
        .constrain_to(safe)
        .movable(false);
    ctx.move_to_top(layer.layer());
    layer.show(ctx, |ui| {
        // Paint the scrim on the dialog layer so the view underneath stays visible.
        ctx.layer_painter(ui.layer_id()).rect_filled(
            ctx.screen_rect(),
            0.0,
            egui::Color32::from_black_alpha(138),
        );
        let frame = egui::Frame::window(ui.style())
            .fill(ui.visuals().window_fill)
            .stroke(egui::Stroke::new(
                1.0_f32,
                crate::theme::accent(ui).gamma_multiply(0.65),
            ))
            .rounding(egui::Rounding::same(18.0))
            .inner_margin(egui::Margin::same(padding));
        frame.show(ui, |ui| {
            ui.set_width(width);
            egui::ScrollArea::vertical()
                .max_height((safe.height() - padding * 2.0 - 16.0).max(80.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if pin_view {
                        ui.vertical_centered(|ui| pin_panel(app, ui, compact));
                    } else {
                        management_panel(app, ui);
                    }
                });
        });
    });
}

fn pin_panel(app: &mut App, ui: &mut Ui, compact: bool) {
    ui.heading("Elternsperre");
    ui.add_space(if compact { 1.0 } else { 5.0 });
    let setup = !app.parental.policy.enabled() || app.parental.changing;
    ui.label(if setup {
        if app.parental.first_pin.is_empty() {
            "Neue vierstellige PIN eingeben"
        } else {
            "Neue PIN bestätigen"
        }
    } else {
        "Eltern-PIN eingeben"
    });
    if let Some((item, _)) = &app.parental.pending {
        ui.add(egui::Label::new(&item.title).truncate(true));
    }
    ui.label(
        egui::RichText::new(format!(
            "{}{}",
            "●  ".repeat(app.parental.pin.len()),
            "○  ".repeat(4 - app.parental.pin.len())
        ))
        .size(24.0)
        .color(crate::theme::accent_text(ui)),
    );
    if !app.parental.error.is_empty() {
        ui.colored_label(egui::Color32::from_rgb(240, 120, 138), &app.parental.error);
    }
    let keypad_width = ui.available_width().min(324.0);
    let spacing = if compact { 4.0 } else { 8.0 };
    let key_size = [
        (keypad_width - 2.0 * spacing) / 3.0,
        if compact { 38.0 } else { 46.0 },
    ];
    ui.allocate_ui_with_layout(
        egui::vec2(keypad_width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Grid::new("parent-pin-pad")
                .num_columns(3)
                .spacing(egui::vec2(spacing, spacing))
                .show(ui, |ui| {
                    for (i, n) in [
                        "1", "2", "3", "4", "5", "6", "7", "8", "9", "", "0", "Löschen",
                    ]
                    .iter()
                    .enumerate()
                    {
                        if n.is_empty() {
                            ui.allocate_space(egui::vec2(key_size[0], key_size[1]));
                        } else {
                            let r = ui.add_sized(key_size, egui::Button::new(*n));
                            if clicked(ui, &r) {
                                if *n == "Löschen" {
                                    app.parental.pin.pop();
                                } else if app.parental.pin.len() < 4 {
                                    app.parental.pin.push_str(n);
                                }
                            }
                        }
                        if i % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
        },
    );
    ui.add_space(spacing);
    let caption = if setup {
        if app.parental.first_pin.is_empty() {
            "Weiter"
        } else {
            "PIN speichern"
        }
    } else {
        "Entsperren"
    };
    let actions = ui
        .allocate_ui_with_layout(
            egui::vec2(
                (238.0 + ui.spacing().item_spacing.x).min(ui.available_width()),
                0.0,
            ),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let confirm = ui.add(
                    egui::Button::new(
                        egui::RichText::new(caption)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .min_size(egui::vec2(136.0, 38.0))
                    .fill(crate::theme::accent_fill(ui)),
                );
                let cancel =
                    ui.add(egui::Button::new("Abbrechen").min_size(egui::vec2(102.0, 38.0)));
                (clicked(ui, &confirm), clicked(ui, &cancel))
            },
        )
        .inner;
    if actions.1 {
        app.parental.close();
    } else if actions.0 {
        if setup {
            if !crate::parental::valid_pin(&app.parental.pin) {
                app.parental.error = "Bitte genau vier Ziffern eingeben.".into();
            } else if app.parental.first_pin.is_empty() {
                app.parental.first_pin = std::mem::take(&mut app.parental.pin);
                app.parental.error.clear();
            } else if app.parental.first_pin != app.parental.pin {
                app.parental.pin.clear();
                app.parental.first_pin.clear();
                app.parental.error = "PINs stimmen nicht überein. Bitte erneut einrichten.".into();
            } else {
                let mut policy = app.parental.policy.clone();
                let result = policy
                    .set_pin(&app.parental.pin)
                    .and_then(|_| crate::store::save_parental(&policy));
                match result {
                    Ok(()) => {
                        app.parental.policy = policy;
                        app.parental.pin.clear();
                        app.parental.first_pin.clear();
                        app.parental.changing = false;
                        app.parental.authenticated = true;
                        app.parental.error.clear();
                    }
                    Err(_) => app.parental.error = "PIN konnte nicht gespeichert werden.".into(),
                }
            }
        } else if app.parental.check() {
            if let Some((item, preview)) = app.parental.pending.take() {
                let key = scoped_key(
                    app.active_profile
                        .as_ref()
                        .map(|p| p.id.as_str())
                        .unwrap_or(""),
                    &item.progress_key,
                );
                let restart = app.parental.restart;
                let untracked = app.parental.untracked;
                app.parental.close();
                app.parental.grant = Some(key);
                if preview {
                    app.start_preview(item);
                } else if untracked {
                    app.play_untracked(item);
                } else if restart {
                    app.play_from_beginning(item);
                } else {
                    app.play(item);
                }
            } else {
                app.parental.authenticated = true;
            }
        }
    }
}

fn management_panel(app: &mut App, ui: &mut Ui) {
    ui.heading("Elternsperre");
    if button(ui, "Schließen") {
        app.parental.close();
        return;
    }
    if !app.parental.error.is_empty() {
        ui.colored_label(egui::Color32::from_rgb(240, 120, 138), &app.parental.error);
    }
    ui.label("Gesperrte Serien schützen alle Staffeln und Folgen. Auswahl gilt für das jeweilige Anbieterprofil auf diesem Gerät.");
    if button(ui, "PIN ändern") {
        app.parental.changing = true;
        return;
    }
    if button(ui, "Elternsperre ausschalten") {
        let policy = Default::default();
        if crate::store::save_parental(&policy).is_ok() {
            app.parental.policy = policy;
            app.parental.close();
        } else {
            app.parental.error = "Änderung konnte nicht gespeichert werden.".into();
        }
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for (i, label) in ["Sender", "Filme", "Serien"].iter().enumerate() {
            let r = ui.selectable_label(app.parental.kind == i, *label);
            if clicked(ui, &r) {
                app.parental.kind = i;
            }
        }
    });
    ui.add(super::text_field(
        &mut app.parental.query,
        "Inhalte suchen …",
        None,
        |edit| edit,
    ));
    let q = app.parental.query.to_lowercase();
    let signature = format!(
        "{}:{}:{}:{}:{}:{:p}",
        app.parental.kind,
        q,
        app.library.channels.len(),
        app.library.movies.len(),
        app.library.series.len(),
        app.library.channels.as_ptr()
    );
    if app.parental.catalog_cache.as_ref().map(|c| c.0.as_str()) != Some(signature.as_str()) {
        let rows: Vec<(String, String)> = match app.parental.kind {
            0 => app
                .library
                .channels
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&q))
                .map(|c| (format!("live:{}", c.id), c.name.clone()))
                .collect(),
            1 => app
                .library
                .movies
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&q))
                .map(|c| (format!("vod:{}", c.id), c.name.clone()))
                .collect(),
            _ => app
                .library
                .series
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&q))
                .map(|c| (format!("series:{}", c.id), c.name.clone()))
                .collect(),
        };
        app.parental.page = 0;
        app.parental.catalog_cache = Some((signature, rows));
    }
    let cached = &app.parental.catalog_cache.as_ref().unwrap().1;
    let count = cached.len();
    let start = app.parental.page * 40;
    let rows = cached
        .iter()
        .skip(start)
        .take(40)
        .cloned()
        .collect::<Vec<_>>();
    if rows.is_empty() {
        ui.label("Keine passenden Inhalte. Zuerst ein Anbieterprofil anmelden.");
    }
    let profile = app
        .active_profile
        .as_ref()
        .map(|p| p.id.clone())
        .unwrap_or_default();
    egui::ScrollArea::vertical()
        .id_source(("parental-list", app.parental.page, app.parental.kind))
        .max_height(420.0)
        .show(ui, |ui| {
            for (key, name) in &rows {
                let scoped = scoped_key(&profile, key);
                let locked = app.parental.policy.blocked.contains(&scoped);
                if button(
                    ui,
                    &format!(
                        "{} {}",
                        if locked {
                            "Gesperrt ·"
                        } else {
                            "Freigegeben ·"
                        },
                        name
                    ),
                ) {
                    let mut next = app.parental.policy.clone();
                    if locked {
                        next.blocked.remove(&scoped);
                    } else {
                        next.blocked.insert(scoped);
                    }
                    if crate::store::save_parental(&next).is_ok() {
                        app.parental.policy = next;
                    } else {
                        app.parental.error = "Sperre konnte nicht gespeichert werden.".into();
                    }
                }
            }
        });
    if app.parental.page > 0 && button(ui, "Vorherige Inhalte") {
        app.parental.page -= 1;
    }
    if start + rows.len() < count && button(ui, "Weitere Inhalte") {
        app.parental.page += 1;
    }
}
