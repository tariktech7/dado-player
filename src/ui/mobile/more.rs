//! "Mehr" bottom sheet: the destinations that do not fit into the tab bar.

use egui::{pos2, vec2, Align2, Color32, Context, Rect, Rounding, Sense};

use super::components::{self, Icon};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::{App, View};

pub fn sheet(app: &mut App, ctx: &Context, l: &Layout) {
    // The page underneath is scrim-covered: keep it out of the D-pad focus
    // order so OK cannot activate a hidden tab or list row behind this sheet.
    focus::begin_modal(0);
    let screen = ctx.screen_rect();
    let rows: [(Icon, &str, View); 4] = [
        (Icon::Play, "Fortsetzen", View::Continue),
        (Icon::Search, "Suche", View::Search),
        // Fortsetzen is a tab now; Favoriten took its place here (§20).
        (Icon::Star, "Favoriten", View::Favorites),
        (Icon::Sliders, "Einstellungen", View::Settings),
    ];
    let row_h = 56.0;
    let h = 44.0 + row_h * (rows.len() as f32 + 1.0) + l.bottom_inset + 28.0;

    egui::Area::new(egui::Id::new("more-sheet"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            let (bg, bg_resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(bg, 0.0, Color32::from_black_alpha(150));

            let sheet = Rect::from_min_size(
                pos2(screen.left(), screen.bottom() - h),
                vec2(screen.width(), h),
            );
            ui.painter().rect_filled(
                sheet,
                Rounding {
                    nw: 22.0,
                    ne: 22.0,
                    sw: 0.0,
                    se: 0.0,
                },
                bar(ui),
            );
            // drag handle
            let handle =
                Rect::from_center_size(pos2(sheet.center().x, sheet.top() + 12.0), vec2(36.0, 4.0));
            ui.painter()
                .rect_filled(handle, Rounding::same(2.0), muted(ui));

            let mut y = sheet.top() + 30.0;
            for (ic, label, view) in rows {
                let r = Rect::from_min_size(
                    pos2(sheet.left() + 8.0, y),
                    vec2(sheet.width() - 16.0, row_h),
                );
                let resp = ui.interact(r, ui.id().with(("more", label)), Sense::click());
                if resp.is_pointer_button_down_on() {
                    ui.painter().rect_filled(r, RADIUS_SM, pressed(ui));
                }
                components::icon(
                    ui,
                    Rect::from_center_size(pos2(r.left() + 32.0, r.center().y), vec2(24.0, 24.0)),
                    ic,
                    text(ui),
                );
                components::draw_line(
                    ui,
                    pos2(r.left() + 60.0, r.center().y),
                    label,
                    reg(FS_BODY),
                    text(ui),
                    r.width() - 100.0,
                );
                components::icon(
                    ui,
                    Rect::from_center_size(pos2(r.right() - 28.0, r.center().y), vec2(18.0, 18.0)),
                    Icon::ChevronRight,
                    muted(ui),
                );
                if focus::clicked(ui, &resp) {
                    app.view = view;
                    app.mobile_more = false;
                }
                y += row_h;
            }

            // logout
            let r = Rect::from_min_size(
                pos2(sheet.left() + 8.0, y),
                vec2(sheet.width() - 16.0, row_h),
            );
            let resp = ui.interact(r, ui.id().with("more-logout"), Sense::click());
            if resp.is_pointer_button_down_on() {
                ui.painter().rect_filled(r, RADIUS_SM, pressed(ui));
            }
            components::draw_line(
                ui,
                pos2(r.left() + 32.0, r.center().y),
                "Abmelden",
                reg(FS_BODY),
                crate::theme::DANGER,
                r.width() - 80.0,
            );
            if focus::clicked(ui, &resp) {
                app.mobile_more = false;
                app.logout();
            }

            ui.painter().text(
                pos2(sheet.center().x, sheet.bottom() - l.bottom_inset - 12.0),
                Align2::CENTER_CENTER,
                format!("Dado-Player {}", env!("CARGO_PKG_VERSION")),
                reg(FS_CAP),
                muted(ui),
            );

            // Tap above the sheet closes it.
            if bg_resp.clicked() {
                if let Some(p) = bg_resp.interact_pointer_pos() {
                    if p.y < sheet.top() {
                        app.mobile_more = false;
                    }
                }
            }
        });

    let _ = l;
}
