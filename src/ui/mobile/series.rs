//! Phone series browser + full-screen series detail page.

use egui::{vec2, Sense, Ui};

use super::components::{self, Icon, PosterItem, Row};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::types::{Episode, PlayItem, PlayKind};

const CHUNK: usize = 60;

pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    if app.selected_series.is_some() {
        detail(app, ui, l);
        return;
    }
    if !app.library.loaded {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.spinner();
        });
        return;
    }

    let mut labels: Vec<(String, bool)> =
        Vec::with_capacity(app.library.series_categories.len() + 1);
    labels.push(("Alle".to_string(), app.series_cat == "all"));
    for c in &app.library.series_categories {
        labels.push((c.name.clone(), app.series_cat == c.id));
    }
    if let Some(i) = components::chip_row(ui, &labels, l.pad) {
        app.series_cat = if i == 0 {
            "all".to_string()
        } else {
            app.library.series_categories[i - 1].id.clone()
        };
        app.series_page = 0;
    }
    ui.add_space(8.0);

    let cat = if app.series_cat == "all" {
        None
    } else {
        Some(app.series_cat.clone())
    };
    let idx: Vec<usize> = app
        .library
        .series
        .iter()
        .enumerate()
        .filter(|(_, s)| cat.as_ref().is_none_or(|id| &s.category_id == id))
        .map(|(i, _)| i)
        .collect();

    if idx.is_empty() {
        components::empty_state(
            ui,
            Icon::Series,
            "Keine Serien",
            "In dieser Kategorie ist nichts hinterlegt.",
            None,
        );
        return;
    }

    let state_id = ui.id().with("series-chunk");
    let (saved_cat, mut shown) = ui
        .data(|d| d.get_temp::<(String, usize)>(state_id))
        .unwrap_or((app.series_cat.clone(), CHUNK));
    if saved_cat != app.series_cat {
        shown = CHUNK;
    }
    let n = idx.len().min(shown);

    let items: Vec<PosterItem> = idx[..n]
        .iter()
        .map(|&i| {
            let s = &app.library.series[i];
            PosterItem {
                key: format!("scov:{}", s.id),
                url: s.cover.clone(),
                title: s.name.clone(),
                meta: String::new(),
            }
        })
        .collect();

    let mut open: Option<u32> = None;
    let mut load_more = false;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(2.0);
            if let Some(i) = components::poster_grid(app, ui, l, &items) {
                open = Some(app.library.series[idx[i]].id);
            }
            if n < idx.len() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    // Mobile: clear the bottom nav bar (NAV_H) plus a gap.
                    ui.add_space(NAV_H + GAP);
                    let r = components::button(ui, "Mehr laden", l.content_w(), false, true);
                    if focus::clicked(ui, &r) {
                        load_more = true;
                    }
                });
            }
            // Mobile: clear the bottom nav bar (NAV_H) plus a gap.
            ui.add_space(NAV_H + GAP);
        });

    if load_more {
        shown = (shown + CHUNK).min(idx.len());
    }
    ui.data_mut(|d| d.insert_temp(state_id, (app.series_cat.clone(), shown)));

    if let Some(id) = open {
        app.selected_series = Some(id);
        app.series_season = 0;
        app.request_series(id);
    }
}

/// `{n} Staffeln · {m} Folgen` for the hero's meta line.
///
/// `None` while the episodes are still loading, and `None` for an empty list —
/// the caller renders nothing rather than a placeholder. Singular forms are
/// spelled out because „1 Staffeln" is the kind of detail that makes a page look
/// machine-generated, and a single-season series drops the season count
/// entirely: „1 Staffel" is noise when there is only one.
///
/// A single episode is also `None`. The only thing this line could say is
/// „1 Folge", and the one row below it already says that. A line that exists to
/// repeat the next line is not information, it is furniture.
fn episode_summary(eps: Option<&[Episode]>) -> Option<String> {
    let list = eps?;
    if list.len() <= 1 {
        return None;
    }
    let mut seasons: Vec<u32> = list.iter().map(|e| e.season).collect();
    seasons.sort_unstable();
    seasons.dedup();
    let count = |n: usize, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
    };
    let episodes = count(list.len(), "Folge", "Folgen");
    Some(if seasons.len() > 1 {
        format!(
            "{} · {}",
            count(seasons.len(), "Staffel", "Staffeln"),
            episodes
        )
    } else {
        episodes
    })
}

/// The episode number as a subtitle — but only when the title does not already
/// say it.
///
/// Providers either name episodes („Der Fall") or number them („Folge 3"). The
/// row used to print `Staffel 2 · Folge 3` under *both*, which on a numbered
/// provider produced `Folge 3` above `Folge 3`. One condition, nothing lost.
fn episode_subtitle(e: &Episode) -> Option<String> {
    let t = e.title.trim().to_lowercase();
    let n = e.episode;
    let numbered = t == format!("folge {n}")
        || t == format!("folge {n}.")
        || t == format!("f{n}")
        || t == format!("episode {n}")
        || t == n.to_string();
    if numbered {
        None
    } else {
        Some(format!("Staffel {} · Folge {}", e.season, e.episode))
    }
}

fn detail(app: &mut App, ui: &mut Ui, l: &Layout) {
    let Some(sid) = app.selected_series else {
        return;
    };
    app.request_series(sid);

    let Some(s) = app.library.series.iter().find(|s| s.id == sid).cloned() else {
        app.selected_series = None;
        return;
    };

    let mut play: Option<PlayItem> = None;

    // Loaded before the hero rather than after it: the hero's meta line counts
    // seasons and episodes, and the chips below need the same value. One clone
    // for both instead of two lookups.
    let eps = app.series_episodes_for(sid).cloned();
    let meta = episode_summary(eps.as_deref());

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Computed once for the whole hero. The three compact decisions
            // below are one decision: shrinking the cover alone leaves the text
            // column holding three plot rows, which keeps the hero over 115 dp
            // and still eats the episode list.
            let compact = l.compact_height();
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                // Mobile: clear the bottom nav bar (NAV_H) plus a gap.
                ui.add_space(NAV_H + GAP);
                let cover_w = if compact { COVER_W_COMPACT } else { COVER_W };
                let (rect, _) =
                    ui.allocate_exact_size(vec2(cover_w, cover_w * 1.5), Sense::hover());
                components::thumb(app, ui, &format!("scov:{}", s.id), &s.cover, rect, &s.name);

                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.set_width(l.content_w() - cover_w - 14.0);
                    // The hero carries the title and the app bar does not (see
                    // `bar_content`). The bar's `fit()` truncates with an
                    // ellipsis, so keeping the name here is what makes the full
                    // name reachable for a long series; the bar is chrome, this
                    // is the page's identity.
                    ui.label(egui::RichText::new(&s.name).font(bold(FS_HEAD)));
                    ui.add_space(4.0);
                    if !s.plot.is_empty() {
                        // Capped by rows, not characters: a character cap is
                        // blind to width and to font scale, so it could not
                        // bound how tall the block got. `allocate_exact_size`
                        // reserves the galley's height so the season list
                        // below cannot overlap it.
                        //
                        // `PROSE_MAX_W` bounds the *measure* independently of
                        // the window: landscape gives this column 655 dp, which
                        // is ~100 characters per line and unreadable.
                        let g = components::wrap(
                            ui,
                            &s.plot,
                            reg(FS_SUB),
                            muted(ui),
                            ui.available_width().min(PROSE_MAX_W),
                            if compact {
                                PLOT_ROWS_COMPACT
                            } else {
                                PLOT_ROWS
                            },
                        );
                        let (rect, _) = ui.allocate_exact_size(g.size(), egui::Sense::hover());
                        ui.painter().galley(rect.min, g, muted(ui));
                    }
                    // Fills the hole the cover leaves. A hero with a title and
                    // three lines of plot under-fills a 144 dp poster at *any*
                    // cover size, and an empty column reads as a missing
                    // element rather than as whitespace. Nothing at all while
                    // the episodes load: an empty line that fills in later is
                    // worse than a gap that closes later.
                    //
                    // Omitted entirely in compact rather than shrunk: the meta
                    // line is the third element of the column and the only one
                    // that is redundant with the season list directly below it,
                    // which repeats the season and numbers every episode. Two
                    // plot rows plus this line is 115 dp of text under a 96 dp
                    // cover, and 115 is exactly what leaves no room for a row.
                    if !compact {
                        if let Some(m) = &meta {
                            ui.add_space(8.0);
                            let g = components::fit(
                                ui,
                                m,
                                reg(FS_LABEL),
                                muted(ui),
                                ui.available_width(),
                            );
                            let (rect, _) = ui.allocate_exact_size(g.size(), egui::Sense::hover());
                            ui.painter().galley(rect.min, g, muted(ui));
                        }
                    }
                });
            });

            ui.add_space(14.0);

            // ---- seasons ----
            let mut seasons: Vec<u32> = eps
                .as_ref()
                .map(|v| v.iter().map(|e| e.season).collect())
                .unwrap_or_default();
            seasons.sort_unstable();
            seasons.dedup();
            if app.series_season == 0 || !seasons.contains(&app.series_season) {
                app.series_season = seasons.first().copied().unwrap_or(1);
            }

            if seasons.len() > 1 {
                let labels: Vec<(String, bool)> = seasons
                    .iter()
                    .map(|&s| (format!("Staffel {}", s), s == app.series_season))
                    .collect();
                if let Some(i) = components::chip_row(ui, &labels, l.pad) {
                    app.series_season = seasons[i];
                }
                ui.add_space(6.0);
            }

            match eps {
                None if app.series_failed(sid) => {
                    let retry = components::empty_state(
                        ui,
                        Icon::Series,
                        "Episoden nicht verfügbar",
                        app.series_error
                            .as_deref()
                            .unwrap_or("Der Anbieter hat nicht geantwortet."),
                        Some("Erneut versuchen"),
                    );
                    if retry {
                        app.retry_series(sid);
                        ui.ctx().request_repaint();
                    }
                }
                None => {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                    });
                }
                Some(list) => {
                    let season = app.series_season;
                    let rows: Vec<_> = list
                        .iter()
                        .filter(|e| e.season == season)
                        .cloned()
                        .collect();
                    if rows.is_empty() {
                        components::empty_state(
                            ui,
                            Icon::Series,
                            "Keine Folgen",
                            "Für diese Staffel sind keine Folgen hinterlegt.",
                            None,
                        );
                    }
                    for e in rows {
                        let key = format!("ep:{}:{}", e.series_id, e.id);
                        let sub = episode_subtitle(&e);
                        let mut row = Row::new(&e.title).playing(app.get_progress(&key) > 0);
                        if let Some(s) = &sub {
                            row = row.sub(s);
                        }
                        let r = components::list_row(app, ui, row, l.pad);
                        if focus::clicked(ui, &r) {
                            play = Some(PlayItem {
                                title: format!("{} · S{}F{}", s.name, e.season, e.episode),
                                url: e.stream_url.clone(),
                                kind: PlayKind::Episode,
                                progress_key: key,
                            });
                        }
                    }
                }
            }
            // Mobile: clear the bottom nav bar (NAV_H) plus a gap.
            ui.add_space(NAV_H + GAP);
        });

    if let Some(item) = play {
        app.play(item);
    }
}
