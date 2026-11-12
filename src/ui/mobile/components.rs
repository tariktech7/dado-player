//! Touch-first UI primitives.
//!
//! Everything is painted by hand instead of using egui's default widgets, so
//! the hit areas, radii and type sizes are exactly the ones from the spec.
//! Vector icons instead of emoji glyphs: they render identically on every
//! device and stay crisp at any density.

use egui::{
    pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui,
    Vec2,
};

use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::{App, View};
use crate::theme;

// ---------------------------------------------------------------- icons ----

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Tv,
    Film,
    Series,
    Search,
    Star,
    StarOn,
    Play,
    Pause,
    Back,
    Close,
    ChevronRight,
    Sliders,
    Volume,
    Fullscreen,
    SkipBack,
    SkipFwd,
    Check,
    User,
    Info,
    Eye,
    EyeOff,
    More,
    Trash,
}

/// Paint `ic` centred in `rect`.
pub fn icon(ui: &Ui, rect: Rect, ic: Icon, color: Color32) {
    let p = ui.painter();
    let r = rect.shrink(rect.width() * 0.16);
    let w = (r.width() * 0.105).max(1.6);
    let s = Stroke::new(w, color);
    let c = r.center();

    let poly = |pts: &[(f32, f32)]| -> Vec<Pos2> {
        pts.iter()
            .map(|&(x, y)| pos2(r.left() + r.width() * x, r.top() + r.height() * y))
            .collect()
    };
    let fill = |pts: Vec<Pos2>| {
        p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    };
    let line = |pts: Vec<Pos2>| {
        p.add(Shape::line(pts, s));
    };

    match ic {
        // Drawn, not typed: U+2605/U+2606 are font glyphs and would vanish or
        // change shape the moment the UI font family changes.
        Icon::Star | Icon::StarOn => {
            let ro = r.width() * 0.5;
            let ri = ro * 0.42;
            let mut pts: Vec<Pos2> = Vec::with_capacity(10);
            for i in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let rad = if i % 2 == 0 { ro } else { ri };
                pts.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
            }
            if ic == Icon::StarOn {
                // A star is not convex, so a fan from the first vertex would
                // tear. Emit one triangle per wedge instead (star-shaped about
                // its centre).
                for i in 0..10 {
                    p.add(Shape::convex_polygon(
                        vec![c, pts[i], pts[(i + 1) % 10]],
                        color,
                        Stroke::NONE,
                    ));
                }
            } else {
                let mut closed = pts.clone();
                closed.push(pts[0]);
                p.add(Shape::line(closed, s));
            }
        }
        Icon::Tv => {
            let body = Rect::from_min_max(r.min, pos2(r.max.x, r.top() + r.height() * 0.68));
            p.rect_stroke(body, Rounding::same(2.5), s);
            let by = r.bottom() - r.height() * 0.06;
            p.line_segment([pos2(c.x, body.bottom()), pos2(c.x, by)], s);
            p.line_segment(
                [
                    pos2(c.x - r.width() * 0.22, by),
                    pos2(c.x + r.width() * 0.22, by),
                ],
                s,
            );
        }
        Icon::Film => {
            p.rect_stroke(r, Rounding::same(3.0), s);
            fill(poly(&[(0.38, 0.30), (0.68, 0.50), (0.38, 0.70)]));
        }
        Icon::Series => {
            let back = Rect::from_min_max(
                pos2(r.left() + r.width() * 0.16, r.top()),
                pos2(r.max.x, r.bottom() - r.height() * 0.18),
            );
            let front = Rect::from_min_max(
                pos2(r.left(), r.top() + r.height() * 0.18),
                pos2(r.max.x - r.width() * 0.16, r.bottom()),
            );
            p.rect_stroke(back, Rounding::same(2.5), s);
            p.rect_stroke(front, Rounding::same(2.5), s);
        }
        Icon::Search => {
            p.circle_stroke(
                pos2(r.left() + r.width() * 0.42, r.top() + r.height() * 0.42),
                r.width() * 0.29,
                s,
            );
            p.line_segment(
                [
                    pos2(r.left() + r.width() * 0.63, r.top() + r.height() * 0.63),
                    pos2(r.right(), r.bottom()),
                ],
                s,
            );
        }
        Icon::Play => fill(poly(&[(0.32, 0.16), (0.84, 0.50), (0.32, 0.84)])),
        Icon::Pause => {
            p.rect_filled(
                Rect::from_min_max(
                    pos2(r.left() + r.width() * 0.26, r.top() + r.height() * 0.16),
                    pos2(r.left() + r.width() * 0.42, r.bottom() - r.height() * 0.16),
                ),
                1.5,
                color,
            );
            p.rect_filled(
                Rect::from_min_max(
                    pos2(r.left() + r.width() * 0.58, r.top() + r.height() * 0.16),
                    pos2(r.left() + r.width() * 0.74, r.bottom() - r.height() * 0.16),
                ),
                1.5,
                color,
            );
        }
        Icon::Back => line(poly(&[(0.62, 0.12), (0.26, 0.50), (0.62, 0.88)])),
        Icon::ChevronRight => line(poly(&[(0.38, 0.12), (0.74, 0.50), (0.38, 0.88)])),
        Icon::Close => {
            line(poly(&[(0.18, 0.18), (0.82, 0.82)]));
            line(poly(&[(0.82, 0.18), (0.18, 0.82)]));
        }
        // Bin: lid, handle, body, and a rib only at 24 dp and up. The rib is
        // size-conditional on purpose — at 20 dp four strokes put the body at
        // the density of the sliders glyph, which is the one icon in this set
        // that stops resolving at a glance. Three strokes read clean.
        Icon::Trash => {
            line(poly(&[(0.16, 0.26), (0.84, 0.26)]));
            line(poly(&[
                (0.38, 0.26),
                (0.41, 0.15),
                (0.59, 0.15),
                (0.62, 0.26),
            ]));
            line(poly(&[
                (0.26, 0.26),
                (0.31, 0.85),
                (0.69, 0.85),
                (0.74, 0.26),
            ]));
            if rect.width() >= 24.0 {
                line(poly(&[(0.50, 0.40), (0.50, 0.72)]));
            }
        }
        Icon::More => {
            for x in [0.20_f32, 0.50, 0.80] {
                p.circle_filled(
                    pos2(r.left() + r.width() * x, c.y),
                    (r.width() * 0.075).max(1.8),
                    color,
                );
            }
        }
        Icon::Sliders => {
            for (y, kx) in [(0.24_f32, 0.36_f32), (0.50, 0.66), (0.76, 0.44)] {
                let yy = r.top() + r.height() * y;
                p.line_segment([pos2(r.left(), yy), pos2(r.right(), yy)], s);
                p.circle_filled(pos2(r.left() + r.width() * kx, yy), w * 2.0, color);
            }
        }
        Icon::Volume => {
            fill(poly(&[
                (0.10, 0.38),
                (0.26, 0.38),
                (0.44, 0.18),
                (0.44, 0.82),
                (0.26, 0.62),
                (0.10, 0.62),
            ]));
            line(poly(&[(0.56, 0.34), (0.66, 0.50), (0.56, 0.66)]));
            line(poly(&[(0.70, 0.24), (0.86, 0.50), (0.70, 0.76)]));
        }
        Icon::Fullscreen => {
            let l = 0.30;
            let m = 0.06;
            for (cx, cy, dx, dy) in [
                (m, m, 1.0, 1.0),
                (1.0 - m, m, -1.0, 1.0),
                (m, 1.0 - m, 1.0, -1.0),
                (1.0 - m, 1.0 - m, -1.0, -1.0),
            ] {
                let x = r.left() + r.width() * cx;
                let y = r.top() + r.height() * cy;
                p.line_segment([pos2(x, y), pos2(x + r.width() * l * dx, y)], s);
                p.line_segment([pos2(x, y), pos2(x, y + r.height() * l * dy)], s);
            }
        }
        Icon::SkipFwd | Icon::SkipBack => {
            let flip = ic == Icon::SkipBack;
            let tri = if flip {
                vec![(0.72, 0.18), (0.28, 0.50), (0.72, 0.82)]
            } else {
                vec![(0.28, 0.18), (0.72, 0.50), (0.28, 0.82)]
            };
            fill(poly(&tri));
            let bx = if flip { 0.18 } else { 0.80 };
            p.rect_filled(
                Rect::from_min_max(
                    pos2(r.left() + r.width() * bx, r.top() + r.height() * 0.18),
                    pos2(
                        r.left() + r.width() * (bx + 0.09),
                        r.bottom() - r.height() * 0.18,
                    ),
                ),
                1.0,
                color,
            );
        }
        Icon::Check => line(poly(&[(0.20, 0.52), (0.42, 0.74), (0.80, 0.28)])),
        Icon::User => {
            p.circle_stroke(pos2(c.x, r.top() + r.height() * 0.30), r.width() * 0.19, s);
            line(poly(&[
                (0.20, 0.88),
                (0.28, 0.64),
                (0.50, 0.56),
                (0.72, 0.64),
                (0.80, 0.88),
            ]));
        }
        Icon::Info => {
            p.circle_stroke(c, r.width() * 0.42, s);
            p.circle_filled(pos2(c.x, r.top() + r.height() * 0.30), w * 1.3, color);
            p.line_segment(
                [
                    pos2(c.x, r.top() + r.height() * 0.44),
                    pos2(c.x, r.bottom() - r.height() * 0.22),
                ],
                s,
            );
        }
        Icon::Eye => {
            line(poly(&[
                (0.08, 0.50),
                (0.32, 0.26),
                (0.68, 0.26),
                (0.92, 0.50),
                (0.68, 0.74),
                (0.32, 0.74),
                (0.08, 0.50),
            ]));
            p.circle_filled(c, r.width() * 0.13, color);
        }
        Icon::EyeOff => {
            line(poly(&[
                (0.08, 0.50),
                (0.32, 0.26),
                (0.68, 0.26),
                (0.92, 0.50),
                (0.68, 0.74),
                (0.32, 0.74),
                (0.08, 0.50),
            ]));
            p.circle_filled(c, r.width() * 0.13, color);
            // diagonal slash across the eye
            line(poly(&[(0.12, 0.20), (0.88, 0.80)]));
        }
    }
}

// ------------------------------------------------------------ text bits ----

/// Single line, ellipsised at `max_w`.
pub fn fit(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = max_w.max(8.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    ui.fonts(|f| f.layout_job(job))
}

/// Up to `rows` lines, ellipsised.
pub fn wrap(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
    rows: usize,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = max_w.max(8.0);
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    ui.fonts(|f| f.layout_job(job))
}

/// Draw a single ellipsised line at `pos` (left, vertical centre).
pub fn draw_line(ui: &Ui, pos: Pos2, text: &str, font: FontId, color: Color32, max_w: f32) {
    let g = fit(ui, text, font, color, max_w);
    let y = pos.y - g.size().y * 0.5;
    ui.painter().galley(pos2(pos.x, y), g, color);
}

// --------------------------------------------------------------- app bar ----

#[derive(Default)]
pub struct BarOut {
    pub back: bool,
    pub action: bool,
}

/// Top app bar. `action` draws one icon button on the right.
///
/// `pad` is `Layout::pad`, not a literal: the title has to share its left edge
/// with the page content and with the chips below it. Hardcoding it here put
/// the title 8 dp right of the first row on every screen.
pub fn app_bar(
    ui: &mut Ui,
    title: &str,
    can_back: bool,
    action: Option<(Icon, &str)>,
    pad: f32,
) -> BarOut {
    let mut out = BarOut::default();
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, bar(ui));
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, hairline(ui)),
    );

    let mut x = rect.left() + pad;
    if can_back {
        let b = Rect::from_center_size(pos2(x + 24.0, rect.center().y), Vec2::splat(TOUCH));
        let resp = ui.interact(b, ui.id().with("bar-back"), Sense::click());
        icon(
            ui,
            Rect::from_center_size(b.center(), Vec2::splat(24.0)),
            Icon::Back,
            text(ui),
        );
        if focus::clicked(ui, &resp) {
            out.back = true;
        }
        x += 48.0;
    }

    if let Some((ic, tip)) = action {
        let b = Rect::from_center_size(
            pos2(rect.right() - pad - 24.0, rect.center().y),
            Vec2::splat(TOUCH),
        );
        let resp = ui
            .interact(b, ui.id().with("bar-action"), Sense::click())
            .on_hover_text(tip);
        icon(
            ui,
            Rect::from_center_size(b.center(), Vec2::splat(24.0)),
            ic,
            text(ui),
        );
        if focus::clicked(ui, &resp) {
            out.action = true;
        }
    }

    let title_right = if action.is_some() {
        rect.right() - pad - 48.0
    } else {
        rect.right() - pad
    };
    let title_w = (title_right - x).max(60.0);
    let g = fit(ui, title, bold(FS_TITLE), text(ui), title_w);
    let ty = rect.center().y - g.size().y * 0.5;
    ui.painter().galley(pos2(x, ty), g, text(ui));
    out
}

// ------------------------------------------------------------ bottom nav ----

const TABS: [(Icon, &str, View); 4] = [
    (Icon::Tv, "Live", View::Live),
    (Icon::Film, "Filme", View::Movies),
    (Icon::Series, "Serien", View::Series),
    // Fortsetzen is a daily destination; Favoriten is a shelf you visit now and
    // then, so it moved into the "Mehr" sheet (MOBILE-BAUPLAN §20).
    (Icon::Play, "Fortsetzen", View::Continue),
];

/// Bottom navigation. Returns true when the "Mehr" sheet should open.
// The index is deliberately numeric: it positions the cell, indexes `TABS` and
// is the stable egui widget ID for that tab. Iterating `TABS` would lose the
// fifth "Mehr" cell and the ID stability.
#[allow(clippy::needless_range_loop)]
pub fn nav_bar(app: &mut App, ui: &mut Ui) -> bool {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, bar(ui));
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.top() + 0.5,
        Stroke::new(1.0_f32, hairline(ui)),
    );

    let n = 5;
    let cell_w = rect.width() / n as f32;
    let mut open_more = false;

    for i in 0..n {
        let cell = Rect::from_min_size(
            pos2(rect.left() + cell_w * i as f32, rect.top()),
            vec2(cell_w, rect.height()),
        );
        let (ic, label, view) = if i < 4 {
            TABS[i]
        } else {
            (Icon::More, "Mehr", View::Settings)
        };
        let selected = if i < 4 {
            app.view == view
        } else {
            // Favoriten lives in the sheet now, so "Mehr" stays lit on it.
            matches!(app.view, View::Settings | View::Search | View::Favorites) || app.mobile_more
        };

        let resp = ui.interact(cell, ui.id().with(("nav", i)), Sense::click());
        if resp.is_pointer_button_down_on() {
            ui.painter().rect_filled(cell, 0.0, pressed(ui));
        }

        let color = if selected { accent_text(ui) } else { muted(ui) };
        // Material-style pill behind the active icon.
        let pill =
            Rect::from_center_size(pos2(cell.center().x, cell.top() + 18.0), vec2(52.0, 30.0));
        if selected {
            ui.painter()
                .rect_filled(pill, Rounding::same(15.0), accent_soft(ui));
        }
        icon(
            ui,
            Rect::from_center_size(pill.center(), Vec2::splat(22.0)),
            ic,
            color,
        );
        let g = fit(ui, label, medium(FS_CAP), color, cell_w - 6.0);
        let ty = cell.bottom() - 20.0 - g.size().y * 0.5;
        ui.painter()
            .galley(pos2(cell.center().x - g.size().x * 0.5, ty), g, color);

        if focus::clicked(ui, &resp) {
            if i < 4 {
                app.view = view;
                app.mobile_more = false;
            } else {
                open_more = true;
            }
        }
    }
    open_more
}

// ------------------------------------------------------------ list row ----

pub struct Row<'a> {
    pub img_key: &'a str,
    pub img_url: &'a str,
    pub title: &'a str,
    pub sub: &'a str,
    pub trailing: Option<&'a str>,
    /// Highlighted as "currently playing".
    pub playing: bool,
    /// Draw the round play button on the right.
    pub show_play: bool,
    /// Now/Next progress through the current programme, 0..1. `None` = no bar.
    /// Set only by Live rows (MOBILE-BAUPLAN §23.6 C).
    pub progress: Option<f32>,
}

impl<'a> Row<'a> {
    pub fn new(title: &'a str) -> Self {
        Row {
            img_key: "",
            img_url: "",
            title,
            sub: "",
            trailing: None,
            playing: false,
            show_play: true,
            progress: None,
        }
    }
    pub fn img(mut self, key: &'a str, url: &'a str) -> Self {
        self.img_key = key;
        self.img_url = url;
        self
    }
    /// Builder setter for the row subtitle. Not the arithmetic `Sub` trait.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(mut self, s: &'a str) -> Self {
        self.sub = s;
        self
    }
    pub fn trailing(mut self, s: &'a str) -> Self {
        self.trailing = Some(s);
        self
    }
    pub fn playing(mut self, p: bool) -> Self {
        self.playing = p;
        self
    }
    pub fn no_play(mut self) -> Self {
        self.show_play = false;
        self
    }
    /// Now/Next progress through the current programme (0..1). Only Live rows
    /// set it (MOBILE-BAUPLAN §23.6 C). Clamped here so a bad EPG entry can
    /// never paint a bar wider than the row.
    pub fn progress(mut self, frac: f32) -> Self {
        self.progress = Some(frac.clamp(0.0, 1.0));
        self
    }
}

/// One 64 dp list row: thumbnail (or letter avatar), two text lines, action.
///
/// The hit area stays full-bleed; only the content column starts at `pad`, so
/// the avatar lines up with the app bar title and the chips. It used to sit at
/// a literal 8 dp, which made four different left edges on one screen.
pub fn list_row(app: &mut App, ui: &mut Ui, row: Row<'_>, pad: f32) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click());

    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, RADIUS_SM, pressed(ui));
    } else if row.playing {
        ui.painter().rect_filled(rect, RADIUS_SM, accent_soft(ui));
    }
    // §23.6 B: the currently playing item gets an accent ring, not just a
    // tinted background. On a 10-foot screen a tint alone disappears.
    if row.playing {
        ui.painter()
            .rect_stroke(rect.shrink(1.0), RADIUS_SM, Stroke::new(SEL_RING, accent()));
    }

    // 22 = half of the 44 dp avatar, so the avatar's *left edge* lands exactly
    // on `pad` and shares an edge with the chips, the app bar title and the
    // count line. A centre of `pad + 24` put the box 2 dp to the right of
    // everything else — small, but it is the difference between one edge and
    // two, and the whole point of passing `pad` in is to have one.
    let avatar = Rect::from_center_size(
        pos2(rect.left() + pad + 22.0, rect.center().y),
        Vec2::splat(44.0),
    );
    // §23.6 B: only the *painted* artwork grows; the allocated hit rect and
    // the text column keep using `avatar`, so nothing shifts and the 48 dp
    // target is unchanged.
    let art = if row.playing {
        Rect::from_center_size(avatar.center(), avatar.size() * SEL_SCALE)
    } else {
        avatar
    };
    thumb(app, ui, row.img_key, row.img_url, art, row.title);

    let right_pad = if row.show_play { pad + 40.0 } else { pad };
    let text_x = avatar.right() + 12.0;
    let text_w = (rect.right() - right_pad - text_x).max(40.0);

    let title_font = bold(FS_ROW);
    let has_sub = !row.sub.is_empty();
    if has_sub {
        draw_line(
            ui,
            pos2(text_x, rect.center().y - 10.0),
            row.title,
            title_font,
            text(ui),
            text_w,
        );
        draw_line(
            ui,
            pos2(text_x, rect.center().y + 11.0),
            row.sub,
            reg(FS_SUB),
            muted(ui),
            text_w,
        );
    } else {
        draw_line(
            ui,
            pos2(text_x, rect.center().y),
            row.title,
            title_font,
            text(ui),
            text_w,
        );
    }

    if let Some(t) = row.trailing {
        let g = fit(ui, t, reg(FS_CAP), muted(ui), 70.0);
        ui.painter().galley(
            pos2(
                rect.right() - right_pad - 6.0 - g.size().x,
                rect.center().y - g.size().y * 0.5,
            ),
            g,
            muted(ui),
        );
    }

    if row.show_play {
        let c = pos2(rect.right() - 30.0, rect.center().y);
        let playing_now = row.playing;
        ui.painter().circle_filled(
            c,
            18.0,
            if playing_now {
                accent_fill()
            } else {
                ui.visuals().widgets.inactive.bg_fill
            },
        );
        icon(
            ui,
            Rect::from_center_size(c, Vec2::splat(18.0)),
            Icon::Play,
            if playing_now {
                Color32::WHITE
            } else {
                text(ui)
            },
        );
    }

    // §23.6 C: Now/Next progress lives at the bottom of the row, under the
    // text column and clear of the 44 dp artwork. 3 dp tall so it reads as a
    // thin scrub line rather than a second control.
    if let Some(f) = row.progress {
        progress_bar(
            ui,
            Rect::from_min_size(pos2(text_x, rect.bottom() - 7.0), vec2(text_w, 3.0)),
            f,
        );
    }

    resp
}

/// Thumbnail with letter-avatar fallback.
pub fn thumb(app: &mut App, ui: &Ui, key: &str, url: &str, rect: Rect, title: &str) {
    ui.painter()
        .rect_filled(rect, Rounding::same(10.0), surface(ui));
    if !url.is_empty() {
        app.request_image(key.to_string(), url.to_string());
        if let Some(tex) = app.image(key) {
            ui.painter().image(
                tex.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            return;
        }
    }
    let letter = title
        .chars()
        .next()
        .unwrap_or('•')
        .to_uppercase()
        .to_string();
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        letter,
        bold(rect.height() * 0.42),
        muted(ui),
    );
}

// --------------------------------------------------------------- poster ----

/// Poster card with title underneath. Returns the click response.
pub fn poster_card(
    app: &mut App,
    ui: &mut Ui,
    key: &str,
    url: &str,
    title: &str,
    meta: &str,
    w: f32,
) -> Response {
    let img_h = (w * 1.5).round();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, img_h + 44.0), Sense::click());
    let img = Rect::from_min_size(rect.min, vec2(w, img_h));

    ui.painter()
        .rect_filled(img, Rounding::same(RADIUS_SM), surface(ui));
    let mut painted = false;
    if !url.is_empty() {
        app.request_image(key.to_string(), url.to_string());
        if let Some(tex) = app.image(key) {
            ui.painter().image(
                tex.id(),
                img,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            painted = true;
        }
    }
    if !painted {
        // Same treatment as `thumb()`: a centred initial on the surface colour.
        // A provider that omits `cover` is common and a bare filled rectangle
        // reads as a broken image. `thumb()` already did this, so the same
        // empty `cover` rendered two different ways on two screens — the detail
        // hero showed a letter tile while the grid showed a blank panel.
        // Size is fixed rather than proportional to the card: at poster height
        // (1.5 × width) a `0.42 × height` letter would be ~74 pt in a 3-column
        // grid and would read as artwork rather than as a placeholder.
        let letter = title
            .chars()
            .next()
            .unwrap_or('•')
            .to_uppercase()
            .to_string();
        let size = if w < 120.0 { 20.0 } else { FS_DISPLAY };
        ui.painter().text(
            img.center(),
            Align2::CENTER_CENTER,
            letter,
            bold(size),
            muted(ui),
        );
    }
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_stroke(
            img,
            Rounding::same(RADIUS_SM),
            Stroke::new(SEL_RING, accent()),
        );
    }

    let t = fit(ui, title, bold(FS_LABEL + 1.0), text(ui), w);
    ui.painter()
        .galley(pos2(img.left(), img.bottom() + 8.0), t, text(ui));
    if !meta.is_empty() {
        let m = fit(ui, meta, reg(FS_CAP), muted(ui), w);
        ui.painter()
            .galley(pos2(img.left(), img.bottom() + 26.0), m, muted(ui));
    }
    resp
}

/// One entry of a poster grid.
pub struct PosterItem {
    pub key: String,
    pub url: String,
    pub title: String,
    pub meta: String,
}

/// Responsive poster grid. Returns the index that was tapped.
pub fn poster_grid(
    app: &mut App,
    ui: &mut Ui,
    layout: &Layout,
    items: &[PosterItem],
) -> Option<usize> {
    let cols = layout.cols(100.0);
    let cw =
        ((layout.content_w() - layout.gap * (cols.saturating_sub(1)) as f32) / cols as f32).floor();
    let mut clicked = None;
    let mut i = 0usize;
    while i < items.len() {
        ui.horizontal(|ui| {
            // `add_space` and the automatic item spacing both apply *between*
            // children, so the laid-out gutter was `gap + 8` = 20 dp while `cw`
            // above was computed from `gap` alone. Three columns therefore
            // consumed `pad + 3·cw + 2·20` = 410 dp of a 411 dp screen: the
            // right-hand column ran to the edge with no page margin, and the
            // gutter did not match `GAP`. Zeroing the automatic spacing makes
            // `gap` the gutter and the arithmetic true again.
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(layout.pad);
            for c in 0..cols {
                if i >= items.len() {
                    break;
                }
                let it = &items[i];
                let resp = poster_card(app, ui, &it.key, &it.url, &it.title, &it.meta, cw);
                if focus::clicked(ui, &resp) {
                    clicked = Some(i);
                }
                if c + 1 != cols {
                    ui.add_space(layout.gap);
                }
                i += 1;
            }
        });
        ui.add_space(8.0);
    }
    clicked
}

// ---------------------------------------------------------------- chips ----
/// Horizontally scrolling category chips. Returns the clicked index.
///
/// `pad` is `Layout::pad` so the first chip shares the page margin; a literal
/// 16 here is right only while the side inset is zero.
pub fn chip_row(ui: &mut Ui, labels: &[(String, bool)], pad: f32) -> Option<usize> {
    let mut clicked = None;
    egui::ScrollArea::horizontal()
        .id_source("chips")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(pad);
                for (i, (label, sel)) in labels.iter().enumerate() {
                    let r = chip(ui, label, *sel);
                    if focus::clicked(ui, &r) {
                        clicked = Some(i);
                    }
                }
                ui.add_space(pad);
            });
        });
    clicked
}

pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let font = medium(FS_LABEL + 1.0);
    let g = fit(ui, label, font.clone(), text(ui), 220.0);
    let w = g.size().x + 30.0;
    // 48 dp touch target, 36 dp visible pill — the pill is painted centred so
    // the row keeps its visual height without shrinking the hit area.
    let (rect, resp) = ui.allocate_exact_size(vec2(w, TOUCH), Sense::click());
    let pill = Rect::from_center_size(rect.center(), vec2(w, 36.0));
    let bg = if selected { accent_fill() } else { surface(ui) };
    ui.painter().rect_filled(pill, Rounding::same(18.0), bg);
    let color = if selected { Color32::WHITE } else { text(ui) };
    let g = fit(ui, label, font, color, w - 20.0);
    ui.painter().galley(
        pos2(
            rect.center().x - g.size().x * 0.5,
            rect.center().y - g.size().y * 0.5,
        ),
        g,
        color,
    );
    resp
}

// -------------------------------------------------------------- buttons ----

/// Two-to-four way switch on a surface track. Returns the newly tapped index.
///
/// Used instead of two filled buttons: only the primary action on a screen may
/// be accent-filled, otherwise the eye has nowhere to land.
pub fn segmented(ui: &mut Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, TOUCH), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(RADIUS_SM), surface(ui));
    let n = labels.len().max(1) as f32;
    let seg_w = (rect.width() - 8.0) / n;
    let mut hit = None;
    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(
            pos2(rect.left() + 4.0 + seg_w * i as f32, rect.top() + 4.0),
            vec2(seg_w, rect.height() - 8.0),
        );
        let r = ui.interact(seg, ui.id().with(("seg", i)), Sense::click());
        let on = i == selected;
        if on {
            ui.painter()
                .rect_filled(seg, Rounding::same(9.0), accent_fill());
        } else if r.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(seg, Rounding::same(9.0), pressed(ui));
        }
        let color = if on { Color32::WHITE } else { text(ui) };
        let g = fit(ui, label, medium(FS_LABEL + 1.0), color, seg_w - 12.0);
        ui.painter().galley(
            pos2(
                seg.center().x - g.size().x * 0.5,
                seg.center().y - g.size().y * 0.5,
            ),
            g,
            color,
        );
        if focus::clicked(ui, &r) {
            hit = Some(i);
        }
    }
    hit
}

pub fn button(ui: &mut Ui, label: &str, w: f32, primary: bool, enabled: bool) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 52.0), Sense::hover());
    button_at(ui, rect, label, primary, enabled)
}

/// Same button, painted into a rect the caller already owns. Needed where the
/// layout is anchored from the bottom edge (the login action bar) instead of
/// flowing top-down.
pub fn button_at(ui: &mut Ui, rect: Rect, label: &str, primary: bool, enabled: bool) -> Response {
    // A disabled button must not be clickable, not merely greyed out.
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let resp = ui.interact(
        rect,
        ui.id().with(("btn", rect.min.x as i32, rect.min.y as i32)),
        sense,
    );
    let (bg, fg) = if !enabled {
        (surface(ui), muted(ui))
    } else if primary {
        (accent_fill(), Color32::WHITE)
    } else {
        (surface(ui), text(ui))
    };
    let bg = if enabled && resp.is_pointer_button_down_on() {
        if primary {
            Color32::from_rgb(0x2E, 0x5B, 0xBE)
        } else {
            pressed(ui)
        }
    } else {
        bg
    };
    ui.painter()
        .rect_filled(rect, Rounding::same(RADIUS_SM + 2.0), bg);
    let g = fit(ui, label, bold(FS_BODY), fg, rect.width() - 24.0);
    ui.painter().galley(
        pos2(
            rect.center().x - g.size().x * 0.5,
            rect.center().y - g.size().y * 0.5,
        ),
        g,
        fg,
    );
    resp
}

/// 48 dp round icon button. Returns whether it was activated.
pub fn icon_button(ui: &mut Ui, ic: Icon, size: f32, filled: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let r = size * 0.5;
    if filled {
        ui.painter().circle_filled(rect.center(), r, accent_fill());
    } else if resp.is_pointer_button_down_on() {
        ui.painter()
            .circle_filled(rect.center(), r, Color32::from_white_alpha(28));
    }
    icon(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(size * 0.52)),
        ic,
        if filled { Color32::WHITE } else { text(ui) },
    );
    focus::clicked(ui, &resp)
}

// ---------------------------------------------------------------- field ----

/// Which soft keyboard a field should raise. Passed to Android's
/// `EditorInfo.inputType`; on desktop it only decides the masking.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kb {
    /// Free text, sentence capitals.
    Text,
    /// Profile name — word capitals.
    Name,
    /// Host or URL — the IME gets a dedicated "/" and ".com" key.
    Url,
    /// Masked, no suggestions, no learning.
    Password,
}

impl Kb {
    /// `android.text.InputType` bitmask for `EditorInfo.inputType`.
    pub fn android_input_type(self) -> i32 {
        const CLASS_TEXT: i32 = 0x0000_0001;
        const VARIATION_URI: i32 = 0x0000_0010;
        const VARIATION_PASSWORD: i32 = 0x0000_0080;
        const FLAG_CAP_WORDS: i32 = 0x0000_1000;
        const FLAG_NO_SUGGESTIONS: i32 = 0x0008_0000;
        match self {
            // NO capitalisation flag. `Kb::Text` is the Xtream username (and
            // the search box). Gboard honours TYPE_TEXT_FLAG_CAP_SENTENCES by
            // shifting the first letter, so typing `probe` produced `Probe`.
            // Provider usernames are case-sensitive: that single letter is
            // enough to fail auth and leave the user stuck on the login mask.
            // The shared EditorInfo now really is re-read on field focus (see
            // GameActivity.seedIme), so this flag is no longer harmless.
            Kb::Text => CLASS_TEXT,
            // Profile names are display strings, not credentials — word caps
            // are a convenience there.
            Kb::Name => CLASS_TEXT | FLAG_CAP_WORDS,
            Kb::Url => CLASS_TEXT | VARIATION_URI,
            Kb::Password => CLASS_TEXT | VARIATION_PASSWORD | FLAG_NO_SUGGESTIONS,
        }
    }
}

/// Full-width text field with a floating label. Returns the TextEdit response
/// so callers can react to focus (scrolling). Raising the keyboard and telling
/// Android which keyboard to raise both happen here, so no caller can forget.
pub fn field(
    ui: &mut Ui,
    label: &str,
    text: &mut String,
    hint: &str,
    kb: Kb,
    show_password: &mut bool,
) -> Response {
    if !label.is_empty() {
        ui.label(
            egui::RichText::new(label)
                .font(reg(FS_LABEL))
                .color(muted(ui)),
        );
        ui.add_space(6.0);
    }
    // Reserve a real 48 dp column for the password eye BEFORE laying out the
    // TextEdit. The old version painted the icon on top of a full-width text
    // field; egui gives a click to the FIRST widget whose rect contains it,
    // so every tap went to the TextEdit and the eye looked broken. It was also
    // drawn in `from_white_alpha(120)`, which is invisible on a light theme —
    // the reason users reported the icon as missing altogether.
    let eye_w = if kb == Kb::Password { TOUCH } else { 0.0 };
    let text_w = (ui.available_width() - eye_w).max(48.0);
    let mut te = egui::TextEdit::singleline(text)
        .frame(false)
        .hint_text(hint)
        .desired_width(f32::INFINITY);
    if kb == Kb::Password {
        te = te.password(!*show_password);
    }
    let resp = ui.add_sized([text_w, 24.0], te);
    #[cfg(target_os = "ios")]
    {
        let id = resp.id.value();
        if let Some(value) = crate::platform::ios::bridge::edited(id) {
            *text = value;
        }
        if resp.clicked() || resp.gained_focus() {
            let kind = match kb {
                Kb::Url => 1,
                Kb::Password => 2,
                _ => 0,
            };
            crate::platform::ios::bridge::edit(
                id,
                if label.is_empty() { hint } else { label },
                text,
                kind,
            );
        }
    }
    // Focus ring: with no hover on touch, a filled-but-identical box is the only
    // feedback the user gets that the field is the one the keyboard is typing
    // into.
    if resp.has_focus() {
        ui.painter().rect_stroke(
            resp.rect,
            Rounding::same(RADIUS_SM),
            Stroke::new(SEL_RING, accent_text(ui)),
        );
    }
    if resp.gained_focus() {
        crate::platform::set_input_type(kb.android_input_type());
    }
    if kb == Kb::Password {
        // Sits in the reserved column, vertically centred on the text row.
        let eye_rect = Rect::from_min_max(
            pos2(resp.rect.right(), resp.rect.center().y - TOUCH * 0.5),
            pos2(
                resp.rect.right() + eye_w,
                resp.rect.center().y + TOUCH * 0.5,
            ),
        );
        let eye_resp = ui.interact(eye_rect, ui.id().with("password_toggle"), Sense::click());
        let pill = Rect::from_center_size(eye_rect.center(), vec2(TOUCH - 8.0, TOUCH - 8.0));
        ui.painter()
            .rect_filled(pill, Rounding::same(10.0), accent_soft(ui));
        ui.painter().rect_stroke(
            pill,
            Rounding::same(10.0),
            Stroke::new(1.0_f32, hairline(ui)),
        );
        // State icon: an open eye means "tap to reveal", a crossed eye means
        // "tap to hide". `text(ui)`/`accent_text(ui)` follow the theme instead
        // of assuming a dark background.
        let eye_icon = if *show_password {
            Icon::EyeOff
        } else {
            Icon::Eye
        };
        // The `&mut String` parameter is also named `text` and shadows the
        // imported `tokens::text` colour helper, so qualify it explicitly.
        let eye_color = if *show_password {
            accent_text(ui)
        } else {
            super::tokens::text(ui)
        };
        icon(
            ui,
            Rect::from_center_size(eye_rect.center(), vec2(24.0, 24.0)),
            eye_icon,
            eye_color,
        );
        if focus::clicked(ui, &eye_resp) {
            *show_password = !*show_password;
            // Tapping the eye is not "done typing": without this the TextEdit
            // loses focus, the IME hides and the user has to tap the field
            // again before entering the next character.
            resp.request_focus();
        }
    }
    resp
}

// ----------------------------------------------------------------- rows ----

/// Label + switch, whole row tappable.
pub fn switch_row(ui: &mut Ui, label: &str, value: bool) -> Option<bool> {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 56.0), Sense::click());
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, RADIUS_SM, pressed(ui));
    }
    draw_line(
        ui,
        pos2(rect.left() + 2.0, rect.center().y),
        label,
        reg(FS_BODY),
        text(ui),
        w - 80.0,
    );
    let track =
        Rect::from_center_size(pos2(rect.right() - 34.0, rect.center().y), vec2(46.0, 26.0));
    ui.painter().rect_filled(
        track,
        Rounding::same(13.0),
        if value {
            accent_fill()
        } else {
            ui.visuals().widgets.inactive.bg_fill
        },
    );
    let kx = if value {
        track.right() - 13.0
    } else {
        track.left() + 13.0
    };
    ui.painter()
        .circle_filled(pos2(kx, track.center().y), 10.0, Color32::WHITE);
    if focus::clicked(ui, &resp) {
        Some(!value)
    } else {
        None
    }
}

/// Label + "- value +" stepper.
pub fn stepper_row(
    ui: &mut Ui,
    label: &str,
    value: &mut u64,
    step: u64,
    min: u64,
    max: u64,
    unit: &str,
) -> bool {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 56.0), Sense::hover());
    draw_line(
        ui,
        pos2(rect.left() + 2.0, rect.center().y),
        label,
        reg(FS_BODY),
        text(ui),
        w - 160.0,
    );

    let minus = Rect::from_center_size(
        pos2(rect.right() - 124.0, rect.center().y),
        Vec2::splat(TOUCH),
    );
    let plus = Rect::from_center_size(
        pos2(rect.right() - 24.0, rect.center().y),
        Vec2::splat(TOUCH),
    );
    let r_minus = ui.interact(minus, ui.id().with((label, "m")), Sense::click());
    let r_plus = ui.interact(plus, ui.id().with((label, "p")), Sense::click());
    for (r, txt) in [(minus, "−"), (plus, "+")] {
        ui.painter()
            .rect_filled(r.shrink(4.0), Rounding::same(10.0), surface(ui));
        ui.painter()
            .text(r.center(), Align2::CENTER_CENTER, txt, bold(20.0), text(ui));
    }
    let val = format!("{} {}", value, unit);
    let g = fit(ui, &val, bold(FS_LABEL), text(ui), 70.0);
    ui.painter().galley(
        pos2(
            (minus.right() + plus.left()) * 0.5 - g.size().x * 0.5,
            rect.center().y - g.size().y * 0.5,
        ),
        g,
        text(ui),
    );

    let mut changed = false;
    if focus::clicked(ui, &r_minus) {
        *value = value.saturating_sub(step).max(min);
        changed = true;
    }
    if focus::clicked(ui, &r_plus) {
        *value = (*value + step).min(max);
        changed = true;
    }
    changed
}

/// Small muted section header.
/// Non-blocking banner for a failed write. Fills the caller's content width.
///
/// `failed` is passed in rather than read here: the caller reads
/// `store::storage_failed()` **once per frame**, at the top of the screen. Read
/// per write instead, a failure that happens mid-frame renders the banner one
/// frame late and looks like a layout bug.
///
/// Not dismissible — it clears itself on the next successful write, and a close
/// button would only let someone dismiss the single signal that data is being
/// lost. No retry button either: the next regular save is the retry, and a
/// permission error does not improve by being retried on demand.
pub fn storage_banner(ui: &mut Ui, failed: bool) {
    if !failed {
        return;
    }
    // Wording is specified (MOBILE-BAUPLAN §22.2) and event-triggered on
    // purpose: it describes *the last change*, not the state of the device,
    // because that is exactly what the flag tracks. Do not widen it back into a
    // claim about the device — a single failed write does not support one.
    const TEXT: &str = "Die letzte Änderung konnte nicht gespeichert werden. \
                        Sie gilt nur bis zum Beenden der App.";
    let w = ui.available_width();
    let g = wrap(ui, TEXT, reg(FS_SUB), text(ui), (w - 24.0).max(80.0), 3);
    let h = g.size().y + 24.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(RADIUS_SM), accent_soft(ui));
    ui.painter()
        .galley(pos2(rect.left() + 12.0, rect.top() + 12.0), g, text(ui));
    ui.add_space(12.0);
}

pub fn section(ui: &mut Ui, label: &str) {
    ui.add_space(14.0);
    ui.label(
        egui::RichText::new(label)
            .font(medium(FS_LABEL))
            .color(muted(ui)),
    );
    ui.add_space(2.0);
}

/// Empty / error state. Returns true when the optional action was tapped.
pub fn empty_state(ui: &mut Ui, ic: Icon, title: &str, body: &str, action: Option<&str>) -> bool {
    let mut hit = false;
    let w = ui.available_width();
    ui.add_space(56.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, 64.0), Sense::hover());
    icon(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(52.0)),
        ic,
        muted(ui),
    );
    ui.add_space(10.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(title)
                .font(bold(FS_HEAD))
                .color(text(ui)),
        );
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(egui::RichText::new(body).font(reg(FS_SUB)).color(muted(ui)))
                .wrap(true),
        );
        if let Some(a) = action {
            ui.add_space(18.0);
            let r = button(ui, a, 200.0, true, true);
            if focus::clicked(ui, &r) {
                hit = true;
            }
        }
    });
    hit
}

/// Thin progress bar (continue-watching).
pub fn progress_bar(ui: &Ui, rect: Rect, frac: f32) {
    let f = frac.clamp(0.0, 1.0);
    ui.painter()
        .rect_filled(rect, Rounding::same(rect.height() * 0.5), surface(ui));
    if f > 0.0 {
        let w = (rect.width() * f).max(rect.height());
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(w, rect.height())),
            Rounding::same(rect.height() * 0.5),
            accent_fill(),
        );
    }
}

/// Draws the shared page background.
pub fn page_bg(ui: &Ui) {
    ui.painter().rect_filled(ui.max_rect(), 0.0, bg(ui));
}

/// Small "LIVE" badge.
pub fn live_badge(ui: &Ui, rect: Rect) {
    ui.painter()
        .rect_filled(rect, Rounding::same(4.0), theme::DANGER);
    // Dark label on red: white on this red only reaches ~3.4:1.
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "LIVE",
        medium(FS_CAP),
        Color32::from_rgb(0x0D, 0x10, 0x15),
    );
}

/// Standard page frame used by every screen: full-bleed background.
pub fn page(ui: &mut Ui, layout: &Layout, add: impl FnOnce(&mut Ui)) {
    let mut inner = ui.available_rect_before_wrap();
    inner.min.y += 0.0;
    let mut child = ui.child_ui(inner, egui::Layout::top_down(egui::Align::Min));
    child.set_width(layout.w - 0.0);
    page_bg(&child);
    add(&mut child);
}
