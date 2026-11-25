use crate::app::{App, ContinueTab};
use crate::types::{PlayItem, PlayKind};
use crate::ui;
use egui::{RichText, Sense, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new("Fortsetzen").size(22.0).strong(),
        ));
        ui.add_space(14.0);
        category_tabs(app, ui);
    });
    ui.add_space(6.0);

    match app.continue_tab {
        ContinueTab::Live => live_tab(app, ui),
        ContinueTab::Movies => movies_tab(app, ui),
        ContinueTab::Series => series_tab(app, ui),
    }
}

pub fn resume_dialog(app: &mut App, ctx: &egui::Context) {
    let Some((_, item, pos)) = app.pending_resume.clone() else {
        ctx.data_mut(|data| data.remove::<bool>(egui::Id::new("resume-dialog-focused")));
        return;
    };
    ui::mobile::focus::begin_modal(0);
    let screen = ctx.screen_rect();
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let timestamp = format!("{:02}:{:02}:{:02}", pos / 3600, pos / 60 % 60, pos % 60);
    let mut resume = false;
    let mut restart = false;
    let mut cancel = false;
    egui::Area::new(egui::Id::new("continue-resume-dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());
            let scrim = ui.interact(screen, ui.id().with("resume-scrim"), Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(170));
            let w = 420.0f32.min(screen.width() - 32.0).max(280.0);
            let fill = ui.visuals().window_fill;

            // Position centered
            let card_rect = egui::Rect::from_center_size(screen.center(), Vec2::new(w, 220.0));
            let mut child = ui.child_ui(card_rect, egui::Layout::top_down(egui::Align::Min));
            egui::Frame::none()
                .fill(fill)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    if ui.visuals().dark_mode {
                        egui::Color32::from_white_alpha(24)
                    } else {
                        egui::Color32::from_black_alpha(20)
                    },
                ))
                .rounding(egui::Rounding::same(14.0))
                .inner_margin(egui::Margin::same(20.0))
                .show(&mut child, |ui| {
                    ui.set_width(w - 40.0);
                    ui.add(egui::Label::new(
                        RichText::new(t("Wiedergabe fortsetzen?", "Resume playback?"))
                            .size(18.0)
                            .strong(),
                    ));
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(RichText::new(&item.title).size(14.0).strong()).wrap(true),
                    );
                    ui.add_space(2.0);
                    ui.add(egui::Label::new(
                        RichText::new(format!(
                            "{} {timestamp}",
                            t("Zuletzt gesehen bei", "Last watched at")
                        ))
                        .size(12.5)
                        .color(crate::theme::muted(ui)),
                    ));
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        let btn_w = (ui.available_width() - 10.0) / 2.0;
                        let resume_button = ui.add_sized(
                            [btn_w, 40.0],
                            egui::Button::new(t("Fortsetzen", "Resume"))
                                .fill(crate::theme::accent_fill(ui)),
                        );
                        resume = ui::mobile::focus::clicked(ui, &resume_button);
                        let restart_button = ui.add_sized(
                            [btn_w, 40.0],
                            egui::Button::new(t("Von vorne anfangen", "Start from beginning")),
                        );
                        restart = ui::mobile::focus::clicked(ui, &restart_button);
                        let focus_id = egui::Id::new("resume-dialog-focused");
                        if !ctx.data(|data| data.get_temp::<bool>(focus_id).unwrap_or(false)) {
                            ctx.data_mut(|data| data.insert_temp(focus_id, true));
                            ui::mobile::focus::request_id(resume_button.id);
                            resume_button.request_focus();
                        }
                    });
                    let dismiss = ui.button(t("Abbrechen", "Cancel"));
                    cancel |= ui::mobile::focus::clicked(ui, &dismiss);
                });

            if scrim.clicked() {
                cancel = true;
            }
        });
    let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if resume {
        app.pending_resume = None;
        app.play(item);
    } else if restart {
        app.pending_resume = None;
        app.play_from_beginning(item);
    } else if cancel || escape {
        app.pending_resume = None;
    }
}

fn play_or_ask(app: &mut App, item: PlayItem) {
    app.play_history_key(&item.progress_key);
}

fn empty(ui: &mut Ui, text: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Label::new(RichText::new(text).size(13.0).weak()));
    });
}

/// Recently watched channels, in the same row style as the Live list.
fn live_tab(app: &mut App, ui: &mut Ui) {
    let keys: Vec<(String, String)> = app
        .visible_history()
        .iter()
        .filter(|h| h.key.starts_with("live:"))
        .map(|h| (h.key.clone(), h.title.clone()))
        .collect();
    if keys.is_empty() {
        empty(ui, "Noch kein Sender gesehen.");
        return;
    }

    let mut play: Option<PlayItem> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (key, title) in keys {
                let id: Option<u32> = key.strip_prefix("live:").and_then(|s| s.parse().ok());
                let ch =
                    id.and_then(|id| app.library.channels.iter().find(|c| c.id == id).cloned());

                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui::ROW_H),
                    Sense::click(),
                );
                if resp.hovered() {
                    ui.painter()
                        .rect_filled(rect, 10.0, ui.visuals().widgets.hovered.weak_bg_fill);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let mut row = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center));
                row.add_space(10.0);
                let logo = ch.as_ref().map(|c| c.logo.clone()).unwrap_or_default();
                ui::image_or_placeholder(
                    app,
                    &mut row,
                    &format!("logo:{}", id.unwrap_or(0)),
                    &logo,
                    Vec2::splat(38.0),
                );
                row.add_space(10.0);
                row.vertical(|ui| {
                    ui::one_line(ui, &title, 15.0, true);
                    ui.add(egui::Label::new(
                        RichText::new("Live-TV · zuletzt gesehen").size(11.5).weak(),
                    ));
                });
                row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui.button("▶").on_hover_text("Abspielen").clicked() {
                        if let Some(c) = &ch {
                            play = Some(PlayItem {
                                title: c.name.clone(),
                                url: c.stream_url.clone(),
                                kind: PlayKind::Live,
                                progress_key: format!("live:{}", c.id),
                            });
                        }
                    }
                });
                if resp.clicked() {
                    if let Some(c) = &ch {
                        play = Some(PlayItem {
                            title: c.name.clone(),
                            url: c.stream_url.clone(),
                            kind: PlayKind::Live,
                            progress_key: format!("live:{}", c.id),
                        });
                    }
                }
                ui.add_space(2.0);
            }
        });

    if let Some(item) = play {
        app.play(item);
    }
}

/// Movies as a poster grid – identical look to the "Filme" view.
fn movies_tab(app: &mut App, ui: &mut Ui) {
    let keys: Vec<(String, String)> = app
        .visible_history()
        .iter()
        .filter(|h| h.key.starts_with("vod:"))
        .map(|h| (h.key.clone(), h.title.clone()))
        .collect();
    if keys.is_empty() {
        empty(ui, "Noch keinen Film gesehen.");
        return;
    }

    let mut play: Option<PlayItem> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (per_row, gap) = ui::grid_metrics(ui);
            for chunk in keys.chunks(per_row) {
                ui.horizontal_top(|ui| {
                    for (key, title) in chunk {
                        let id: Option<u32> = key.strip_prefix("vod:").and_then(|s| s.parse().ok());
                        let movie = id
                            .and_then(|id| app.library.movies.iter().find(|m| m.id == id).cloned());
                        let cover = movie.as_ref().map(|m| m.cover.clone()).unwrap_or_default();
                        let at = app.progress.get(key).copied().unwrap_or(0);

                        ui.vertical(|ui| {
                            ui.set_width(ui::CARD_W);
                            let resp = ui::poster(
                                app,
                                ui,
                                &format!("cover:{}", id.unwrap_or(0)),
                                &cover,
                                false,
                            );
                            if resp.clicked() {
                                if let Some(m) = &movie {
                                    play = Some(PlayItem {
                                        title: m.name.clone(),
                                        url: m.stream_url.clone(),
                                        kind: PlayKind::Vod,
                                        progress_key: key.clone(),
                                    });
                                }
                            }
                            ui.add_space(4.0);
                            ui::one_line(ui, title, 13.0, false);
                            if at > 0 {
                                ui.add(egui::Label::new(
                                    RichText::new(format!(
                                        "Weiter bei {}",
                                        ui::fmt_time(at as f64)
                                    ))
                                    .size(11.0)
                                    .color(crate::theme::accent_text(ui)),
                                ));
                            }
                        });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(10.0);
            }
        });

    if let Some(item) = play {
        play_or_ask(app, item);
    }
}

/// Series as a poster grid; a click continues the last watched episode.
fn series_tab(app: &mut App, ui: &mut Ui) {
    let query = app.search_q.trim().to_lowercase();
    let entries: Vec<(String, String, String)> =
        crate::types::resume_history(app.visible_history())
            .filter(|h| h.key.starts_with("ep:"))
            .filter(|h| query.is_empty() || h.title.to_lowercase().contains(&query))
            .map(|h| (h.key.clone(), h.title.clone(), h.sub.clone()))
            .collect();
    if entries.is_empty() {
        empty(ui, "Noch keine Serie gesehen.");
        return;
    }

    let mut play: Option<PlayItem> = None;
    let mut history_key: Option<String> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (per_row, gap) = ui::grid_metrics(ui);
            for chunk in entries.chunks(per_row) {
                ui.horizontal_top(|ui| {
                    for (key, title, sub) in chunk {
                        let parts: Vec<&str> =
                            key.strip_prefix("ep:").unwrap_or("").split(':').collect();
                        let sid = parts.first().and_then(|s| s.parse::<u32>().ok());
                        let eid = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                        let series = sid.and_then(|sid| {
                            app.library.series.iter().find(|s| s.id == sid).cloned()
                        });
                        let cover = series.as_ref().map(|s| s.cover.clone()).unwrap_or_default();
                        let episode = sid.zip(eid).and_then(|(sid, eid)| {
                            app.series_episodes
                                .get(&sid)
                                .and_then(|v| v.iter().find(|e| e.id == eid).cloned())
                        });

                        ui.vertical(|ui| {
                            ui.set_width(ui::CARD_W);
                            let resp = ui::poster(
                                app,
                                ui,
                                &format!("cover:s{}", sid.unwrap_or(0)),
                                &cover,
                                false,
                            );
                            if resp.clicked() {
                                match &episode {
                                    Some(e) => {
                                        play = Some(PlayItem {
                                            title: title.clone(),
                                            url: e.stream_url.clone(),
                                            kind: PlayKind::Episode,
                                            progress_key: key.clone(),
                                        });
                                    }
                                    None => history_key = Some(key.clone()),
                                }
                            }
                            ui.add_space(4.0);
                            ui::one_line(ui, title, 13.0, false);
                            ui.add(egui::Label::new(
                                RichText::new(sub)
                                    .size(11.0)
                                    .color(crate::theme::accent_text(ui)),
                            ));
                        });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(10.0);
            }
        });

    if let Some(item) = play {
        play_or_ask(app, item);
    } else if let Some(key) = history_key {
        app.play_history_key(&key);
    }
}

pub fn category_tabs(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for (tab, label) in [
            (ContinueTab::Live, "Live-TV"),
            (ContinueTab::Series, "Serien"),
            (ContinueTab::Movies, "Filme"),
        ] {
            let r = ui.selectable_label(app.continue_tab == tab, label);
            let hit = if app.shared_remote_navigation() {
                crate::ui::mobile::focus::clicked(ui, &r)
            } else {
                r.clicked()
            };
            if hit {
                app.continue_tab = tab;
            }
        }
    });
}
