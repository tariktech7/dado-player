//! Command palette (`Ctrl+K`).
//!
//! One search box over everything the app can do: jump to a view, toggle the
//! theme, log out, or open a channel / movie / series by name. Modelled on
//! IPTVnator's command palette, adapted to egui and our German UI.

use crate::app::{App, View};
use crate::theme;
use crate::types::{PlayItem, PlayKind, Theme};
use egui::{Align2, Color32, FontId, Key, RichText, Rounding, Sense, Stroke, Vec2};

#[derive(Clone)]
enum Action {
    View(View),
    Play(PlayItem),
    OpenSeries(u32),
    ToggleTheme,
    Logout,
}

struct Item {
    icon: &'static str,
    label: String,
    hint: String,
    action: Action,
}

fn collect(app: &App, raw: &str) -> Vec<Item> {
    let q = raw.trim().to_lowercase();
    // Labels match anywhere; keyword lists match word prefixes, so "fil" finds
    // "Filme" but not "Abmelden" (whose keyword list contains "profil").
    let hit = |hay: &str| q.is_empty() || hay.to_lowercase().contains(&q);
    let keys = |list: &str| {
        q.is_empty()
            || list
                .to_lowercase()
                .split_whitespace()
                .any(|w| w.starts_with(&q))
    };
    let mut out = Vec::new();

    // ---- navigation ----
    let nav: [(&str, &str, View); 8] = [
        ("⌂", "Startseite", View::Home),
        ("📺", "Live TV", View::Live),
        ("🎬", "Filme", View::Movies),
        ("📚", "Serien", View::Series),
        ("🔎", "Suche", View::Search),
        ("⭐", "Favoriten", View::Favorites),
        ("▶", "Fortsetzen", View::Continue),
        ("⚙", "Einstellungen", View::Settings),
    ];
    for (icon, label, v) in nav {
        if hit(label) {
            out.push(Item {
                icon,
                label: label.to_string(),
                hint: "Ansicht".into(),
                action: Action::View(v),
            });
        }
    }

    // ---- actions ----
    let dark = app.want_dark();
    let theme_label = if dark {
        "Design: auf Hell wechseln"
    } else {
        "Design: auf Dunkel wechseln"
    };
    if keys("design thema hell dunkel theme") || hit(theme_label) {
        out.push(Item {
            icon: "◐",
            label: theme_label.into(),
            hint: "Aktion".into(),
            action: Action::ToggleTheme,
        });
    }
    if keys("abmelden logout profil konto") {
        out.push(Item {
            icon: "⏻",
            label: "Abmelden".into(),
            hint: "Aktion".into(),
            action: Action::Logout,
        });
    }

    // ---- content (only with a query) ----
    if !q.is_empty() {
        for c in app
            .library
            .channels
            .iter()
            .filter(|c| c.name.to_lowercase().contains(&q))
            .take(6)
        {
            out.push(Item {
                icon: "📺",
                label: c.name.clone(),
                hint: "Live".into(),
                action: Action::Play(PlayItem {
                    title: c.name.clone(),
                    url: c.stream_url.clone(),
                    kind: PlayKind::Live,
                    progress_key: format!("live:{}", c.id),
                }),
            });
        }
        for m in app
            .library
            .movies
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&q))
            .take(6)
        {
            out.push(Item {
                icon: "🎬",
                label: m.name.clone(),
                hint: "Film".into(),
                action: Action::Play(PlayItem {
                    title: m.name.clone(),
                    url: m.stream_url.clone(),
                    kind: PlayKind::Vod,
                    progress_key: format!("vod:{}", m.id),
                }),
            });
        }
        for s in app
            .library
            .series
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&q))
            .take(6)
        {
            out.push(Item {
                icon: "📚",
                label: s.name.clone(),
                hint: "Serie".into(),
                action: Action::OpenSeries(s.id),
            });
        }
    }

    out
}

fn execute(app: &mut App, action: Action) {
    match action {
        Action::View(v) => app.view = v,
        Action::Play(item) => app.play(item),
        Action::OpenSeries(id) => {
            app.selected_series = Some(id);
            app.series_season = 0;
            app.view = View::Series;
            app.request_series(id);
        }
        Action::ToggleTheme => {
            let dark = app.want_dark();
            app.settings.theme = if dark { Theme::Light } else { Theme::Dark };
            app.save_settings_now();
        }
        Action::Logout => app.logout(),
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.palette_open {
        return;
    }
    let screen = ctx.screen_rect();
    let mut close = false;
    let mut run: Option<usize> = None;

    // Built from the current query; typing updates the list on the next frame.
    let items = collect(app, &app.palette_q);
    if !items.is_empty() && app.palette_sel >= items.len() {
        app.palette_sel = 0;
    }

    let w = 560.0_f32.min((screen.width() - 48.0).max(280.0));
    let row_h = 34.0;
    let list_h = (items.len().max(1) as f32 * row_h).min(screen.height() * 0.5);

    // 1) Dimmed backdrop. Created before the card so it paints underneath it.
    let scrim_clicked = egui::Area::new(egui::Id::new("dado-palette-scrim"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(150));
            resp.clicked()
        })
        .inner;

    // 2) The card itself. `Area::anchor` would centre it in `available_rect()`
    //    (the area right of the sidebar), not in the window, so the position is
    //    computed from `screen_rect()` and pinned with `fixed_pos`.
    let card_w = w + 24.0; // content + frame margins
    let card_left = (screen.center().x - card_w * 0.5).max(screen.left() + 8.0);
    let card_top = screen.top() + 64.0;
    let card = egui::Area::new(egui::Id::new("dado-palette"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(card_left, card_top))
        .show(ctx, |ui| {
            let fill = ui.visuals().window_fill;
            let border = if ui.visuals().dark_mode {
                Color32::from_white_alpha(26)
            } else {
                theme::BORDER_LIGHT
            };
            egui::Frame::none()
                .fill(fill)
                .stroke(Stroke::new(1.0_f32, border))
                .rounding(Rounding::same(14.0))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.set_width(w);

                    ui.add(super::text_field(
                        &mut app.palette_q,
                        "Ansicht, Sender, Film oder Serie…",
                        None,
                        |edit| edit.desired_width(w),
                    ))
                    .request_focus();
                    ui.add_space(6.0);

                    let (down, up, enter, esc) = ui.input(|i| {
                        (
                            i.key_pressed(Key::ArrowDown),
                            i.key_pressed(Key::ArrowUp),
                            i.key_pressed(Key::Enter),
                            i.key_pressed(Key::Escape),
                        )
                    });
                    if esc {
                        close = true;
                    }
                    if !items.is_empty() {
                        if down {
                            app.palette_sel = (app.palette_sel + 1) % items.len();
                        }
                        if up {
                            app.palette_sel = app.palette_sel.saturating_sub(1);
                        }
                        if enter {
                            run = Some(app.palette_sel);
                        }
                    }

                    if items.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Nichts gefunden.")
                                .size(13.0)
                                .color(theme::muted(ui)),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(list_h)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (i, it) in items.iter().enumerate() {
                                    let selected = i == app.palette_sel;
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), row_h - 2.0),
                                        Sense::click(),
                                    );
                                    let bg = if selected {
                                        theme::selection_surface(ui)
                                    } else if resp.hovered() {
                                        theme::card_hover(ui)
                                    } else {
                                        Color32::TRANSPARENT
                                    };
                                    ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                                    if selected {
                                        ui.painter().rect_stroke(
                                            rect,
                                            Rounding::same(8.0),
                                            Stroke::new(1.0_f32, theme::accent(ui)),
                                        );
                                    }

                                    let col = if selected {
                                        theme::accent_text(ui)
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    ui.painter().text(
                                        egui::pos2(rect.left() + 10.0, rect.center().y),
                                        Align2::LEFT_CENTER,
                                        it.icon,
                                        FontId::proportional(14.0),
                                        col,
                                    );
                                    let hint_galley = ui.painter().layout_no_wrap(
                                        it.hint.clone(),
                                        FontId::proportional(11.0),
                                        theme::muted(ui),
                                    );
                                    let hint_w = hint_galley.size().x;
                                    ui.painter().galley(
                                        egui::pos2(
                                            rect.right() - 10.0 - hint_w,
                                            rect.center().y - hint_galley.size().y * 0.5,
                                        ),
                                        hint_galley,
                                        theme::muted(ui),
                                    );

                                    let max_label_w =
                                        (rect.width() - 34.0 - hint_w - 20.0).max(40.0);
                                    let label_clip = egui::Rect::from_min_max(
                                        egui::pos2(rect.left() + 34.0, rect.top()),
                                        egui::pos2(rect.left() + 34.0 + max_label_w, rect.bottom()),
                                    );
                                    ui.painter().with_clip_rect(label_clip).text(
                                        egui::pos2(rect.left() + 34.0, rect.center().y),
                                        Align2::LEFT_CENTER,
                                        &it.label,
                                        FontId::proportional(14.0),
                                        col,
                                    );

                                    if resp.hovered() {
                                        app.palette_sel = i;
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if resp.clicked() {
                                        run = Some(i);
                                    }
                                }
                            });
                    }

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("↑ ↓ wählen   ·   Enter öffnen   ·   Esc schließen")
                            .size(10.5)
                            .color(theme::muted(ui)),
                    );
                })
        });

    // A click anywhere outside the card dismisses the palette.
    if scrim_clicked {
        let outside = ctx
            .pointer_interact_pos()
            .is_none_or(|p| !card.response.rect.contains(p));
        if outside {
            close = true;
        }
    }

    if let Some(i) = run {
        if let Some(it) = items.get(i) {
            let action = it.action.clone();
            execute(app, action);
        }
        close = true;
    }
    if close {
        app.palette_open = false;
        app.palette_q.clear();
        app.palette_sel = 0;
    }
}
