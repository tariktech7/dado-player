//! Phone login / profile picker.
//!
//! Two bands, not one scrolling page: the form scrolls, the action bar does
//! not. The action bar is a bottom panel whose height includes the keyboard
//! inset, so "Anmelden" sits 16 dp above the soft keyboard and is reachable no
//! matter which field is focused. Without this the keyboard (≈330 dp) covers
//! the button completely — it is not clipped, it is *behind* the IME, because
//! GameActivity forces edge-to-edge and the window never resizes.

use egui::{pos2, Color32, Context, Frame, Rect, RichText, TopBottomPanel, Ui};

use super::components::{self, Icon, Kb};
use super::focus;
use super::tokens::*;
use super::Layout;
use crate::app::App;
use crate::types::ProfileKind;

/// Width of the landscape action column. One constant, used both for the panel
/// width and for narrowing the form's layout — the two used to disagree.
const SIDE_PANEL_W: f32 = 232.0;

pub fn show(app: &mut App, ctx: &Context, l: &Layout) {
    // Recomputed from the real TextEdit responses further down this frame.
    // `App::update()` reads it at the start of the NEXT frame to decide whether
    // remote keys belong to the IME or to the D-pad focus engine.
    crate::platform::set_text_focus(false);
    let page = page_color(ctx);
    // Landscape phone: a bottom action bar would eat a third of the usable
    // height, so the actions move into a right-hand column and the form keeps
    // the full height to scroll in.
    let side_mode = !l.portrait && l.w >= 560.0;

    // The right action column is part of the SCREEN, not of the form.
    // `Layout::content_w()` is derived from the full `l.w`, so handing the
    // untouched layout to the CentralPanel made the form 232 pt wider than its
    // own panel: it painted straight over the action column and completely hid
    // "Anmelden" on a landscape phone. Give each panel its own geometry.
    let (action_l, body_l) = if side_mode {
        let mut a = l.clone();
        // `l.pad` includes the cutout side inset (64.8 pt on dado_test). That
        // inset belongs to the outer screen edge, not to both sides of an
        // internal 232 pt column, where it would squeeze the button to ~102 pt.
        a.pad = PAD;
        let mut b = l.clone();
        b.w = (l.w - SIDE_PANEL_W).max(240.0);
        (a, b)
    } else {
        (l.clone(), l.clone())
    };

    // While the delete confirm is up, nothing underneath it may act. The scrim
    // absorbs the pointer, but `interact` results for the widgets painted
    // before it are already resolved this frame, so the handlers are gated
    // explicitly as well.
    let blocked = app.pending_delete.is_some();

    if side_mode {
        egui::SidePanel::right("m-login-action")
            .exact_width(SIDE_PANEL_W)
            .resizable(false)
            .frame(Frame::none().fill(page))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, page);
                ui.painter().vline(
                    r.left() + 0.5,
                    r.top()..=r.bottom(),
                    egui::Stroke::new(1.0_f32, hairline(ui)),
                );
                let content =
                    Rect::from_min_max(r.min, egui::pos2(r.max.x, r.max.y - l.bottom_inset));
                action(app, ui, &action_l, content, blocked);
            });
    } else {
        let bar_h = action_height(app, l);
        TopBottomPanel::bottom("m-login-action")
            .exact_height(bar_h + l.bottom_inset)
            .frame(Frame::none().fill(page))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, page);
                ui.painter().hline(
                    r.left()..=r.right(),
                    r.top() + 0.5,
                    egui::Stroke::new(1.0_f32, hairline(ui)),
                );
                // `Frame::inner_margin` does not shrink `max_rect`, so the
                // system bar inset has to be subtracted by hand — otherwise the
                // buttons are anchored to the physical screen edge and
                // disappear under the navigation bar.
                let content =
                    Rect::from_min_max(r.min, egui::pos2(r.max.x, r.max.y - l.bottom_inset));
                action(app, ui, &action_l, content, blocked);
            });
    }

    // ---- scrolling form ----
    egui::CentralPanel::default()
        .frame(Frame::none().fill(page))
        .show(ctx, |ui| {
            components::page_bg(ui);
            // Outside the ScrollArea on purpose: the safe-area gap must not
            // scroll away, or the brand slides under the status bar.
            ui.add_space(l.top_inset + 20.0);
            // Exact visible band of the form, measured from THIS frame's panel
            // geometry. The ScrollArea's inner `clip_rect` is derived from last
            // frame's panel height, so using it to decide "is the field
            // visible?" is one keyboard animation late.
            let viewport = ui.available_rect_before_wrap();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                // egui animates `scroll_to_rect` over 0.1-0.3 s by default.
                // With an IME edit replayed on every keystroke that animation
                // restarts constantly and the form appears to bounce. Apply
                // scroll targets instantly instead.
                .animated(false)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.add_space(
                            ((body_l.w - body_l.content_w().min(496.0)) * 0.5).max(body_l.pad),
                        );
                        ui.vertical(|ui| {
                            ui.set_width(body_l.content_w().min(496.0));
                            body(app, ui, &body_l, blocked, viewport);
                        });
                    });
                });
        });

    if blocked {
        confirm_delete(app, ctx);
    } else {
        // Back dismisses the dialog by clearing `pending_delete` in `App`, which
        // this module never sees. Clearing the focus flag here rather than only
        // on the two button paths keeps a back-dismissed dialog from re-opening
        // without focus the next time it is shown.
        ctx.data_mut(|d| d.remove::<bool>(confirm_shown_key()));
    }
}

fn page_color(ctx: &Context) -> Color32 {
    ctx.style().visuals.panel_fill
}

/// Height of the action bar without the keyboard inset.
fn action_height(_app: &App, l: &Layout) -> f32 {
    let mut h = 14.0 + 52.0 + 14.0; // padding + primary button
                                    // Always reserve the two error lines. Growing the bar by 40 pt the frame a
                                    // validation error appears used to yank the whole form (and the button the
                                    // user just tapped) — the visible "flicker" in the login bug report.
    h += 6.0 + 34.0;
    if l.ime <= 0.0 {
        h += 8.0 + 48.0; // secondary "Ohne Anbieter testen"
        h += 8.0 + 44.0; // "Oberfläche ansehen" (demo library)
    }
    h
}

/// Anchored from the bottom edge of `content` so the primary button never moves
/// when the error line appears or the keyboard changes height.
fn action(app: &mut App, ui: &mut Ui, l: &Layout, content: Rect, blocked: bool) {
    let x = content.left() + l.pad;
    let w = (content.width() - 2.0 * l.pad).max(80.0);

    let mut bottom = content.bottom() - 14.0;

    if l.ime <= 0.0 {
        // Text button, not a filled one: the primary action is the only filled
        // control on the screen, otherwise the eye has nowhere to land.
        let sr = Rect::from_min_size(egui::pos2(x, bottom - 48.0), egui::vec2(w, 48.0));
        let resp = ui.interact(sr, ui.id().with("test-stream"), egui::Sense::click());
        if resp.is_pointer_button_down_on() {
            ui.painter().rect_filled(sr, RADIUS_SM, pressed(ui));
        }
        let g = components::fit(
            ui,
            "Ohne Anbieter testen",
            medium(FS_BODY),
            accent_text(ui),
            w - 16.0,
        );
        ui.painter().galley(
            egui::pos2(
                sr.center().x - g.size().x * 0.5,
                sr.center().y - g.size().y * 0.5,
            ),
            g,
            accent_text(ui),
        );
        let test_hit = focus::clicked(ui, &resp);
        if test_hit && !app.busy && !blocked {
            app.play_test_stream();
        }
        bottom = sr.top() - 8.0;
    }

    let br = Rect::from_min_size(egui::pos2(x, bottom - 52.0), egui::vec2(w, 52.0));

    if let Some(e) = app.error.clone() {
        let g = components::wrap(
            ui,
            &e,
            reg(FS_LABEL),
            Color32::from_rgb(0xFF, 0xB4, 0xB4),
            w,
            2,
        );
        let y = br.top() - 6.0 - g.size().y;
        ui.painter()
            .galley(egui::pos2(x, y), g, Color32::from_rgb(0xFF, 0xB4, 0xB4));
    }

    let label = if app.busy { "Verbinde…" } else { "Anmelden" };
    let r = components::button_at(ui, br, label, true, !app.busy && !blocked);
    if focus::clicked(ui, &r) && !blocked && !app.busy {
        // Drop focus first: the form is about to be replaced by the library and
        // a lingering TextEdit would keep the keyboard up over it.
        let focused = ui.ctx().memory(|m| m.focused());
        if let Some(id) = focused {
            ui.ctx().memory_mut(|m| m.surrender_focus(id));
        }
        app.submit_form();
    }

    // Registered LAST on purpose. The cold-start remote path is verified as
    // "one OK -> test stream"; making the demo button control 0 would silently
    // change that behaviour. It also sits at the top of the action bar, away
    // from the primary action, so a mis-click cannot start a login.
    if l.ime <= 0.0 {
        let dr = Rect::from_min_size(egui::pos2(x, content.top() + 8.0), egui::vec2(w, 44.0));
        let resp = ui.interact(dr, ui.id().with("browse-demo"), egui::Sense::click());
        if resp.is_pointer_button_down_on() {
            ui.painter().rect_filled(dr, RADIUS_SM, pressed(ui));
        }
        let g = components::fit(
            ui,
            "Oberfläche ohne Anbieter ansehen",
            medium(FS_BODY - 1.0),
            accent_text(ui),
            w - 16.0,
        );
        ui.painter().galley(
            egui::pos2(
                dr.center().x - g.size().x * 0.5,
                dr.center().y - g.size().y * 0.5,
            ),
            g,
            accent_text(ui),
        );
        if focus::clicked(ui, &resp) && !app.busy && !blocked {
            app.load_demo();
        }
    }
}

fn body(app: &mut App, ui: &mut Ui, l: &Layout, blocked: bool, viewport: Rect) {
    let mut gained: Option<Rect> = None;
    // Rect of the field that actually owns focus THIS frame, in current screen
    // coordinates. Never cache this across frames: see `follow_focus`.
    let mut focused_rect: Option<Rect> = None;

    // Above the brand: if the last write failed, the user is about to create a
    // profile that will not survive the process, and that is worth saying before
    // they type rather than after.
    components::storage_banner(ui, crate::store::storage_failed());

    // ---- brand ----
    ui.horizontal(|ui| {
        crate::ui::brand_mark(ui, 48.0);
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("Dado-Player").font(bold(FS_DISPLAY)));
            ui.label(
                RichText::new("Mit deinem Anbieter anmelden")
                    .font(reg(FS_SUB))
                    .color(muted(ui)),
            );
        });
    });

    // ---- saved profiles ----
    if !app.profiles.is_empty() {
        components::section(ui, "Gespeicherte Profile");
        let profiles = app.profiles.clone();
        for p in profiles {
            let kind = match p.kind {
                ProfileKind::Xtream => "Xtream",
                ProfileKind::M3u => "M3U",
            };
            let w = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 64.0), egui::Sense::click());
            if resp.is_pointer_button_down_on() {
                ui.painter().rect_filled(rect, RADIUS_SM, pressed(ui));
            }
            let avatar = Rect::from_center_size(
                egui::pos2(rect.left() + 30.0, rect.center().y),
                egui::Vec2::splat(44.0),
            );
            ui.painter()
                .circle_filled(avatar.center(), 22.0, accent_soft(ui));
            ui.painter().text(
                avatar.center(),
                egui::Align2::CENTER_CENTER,
                p.name
                    .chars()
                    .next()
                    .unwrap_or('•')
                    .to_uppercase()
                    .to_string(),
                bold(18.0),
                accent_text(ui),
            );
            components::draw_line(
                ui,
                egui::pos2(rect.left() + 62.0, rect.center().y - 9.0),
                &p.name,
                bold(FS_ROW),
                text(ui),
                w - 130.0,
            );
            components::draw_line(
                ui,
                egui::pos2(rect.left() + 62.0, rect.center().y + 11.0),
                kind,
                reg(FS_CAP),
                muted(ui),
                w - 130.0,
            );
            let row_hit = focus::clicked(ui, &resp);

            // Delete has its own 48 dp hit area, so a tap there never logs in.
            //
            // This is the only destructive control in the app that lives inside
            // another control's tap area: the row's job is "sign in", and this
            // sits at its right edge where a thumb lands while scrolling. It is
            // therefore routed through a confirm, and it is the reason
            // `delete_profile` is never called from a pointer handler anywhere
            // else. Do not add a second one — put a "Profile verwalten" row
            // under the list instead.
            let del = Rect::from_center_size(
                egui::pos2(rect.right() - 24.0, rect.center().y),
                egui::Vec2::splat(TOUCH),
            );
            let dr = ui.interact(del, ui.id().with(("del", &p.id)), egui::Sense::click());
            components::icon(
                ui,
                Rect::from_center_size(del.center(), egui::Vec2::splat(20.0)),
                Icon::Trash,
                danger(),
            );
            let del_hit = focus::clicked(ui, &dr);
            if del_hit && !blocked {
                app.pending_delete = Some(p.id.clone());
            } else if row_hit && !blocked {
                app.start_login(p.clone());
            }
        }
    }

    ui.add_space(18.0);

    // ---- source switch ----
    let sel = match app.login_kind {
        ProfileKind::Xtream => 0,
        ProfileKind::M3u => 1,
    };
    if let Some(i) = components::segmented(ui, &["Xtream Codes", "M3U / Playlist"], sel) {
        app.login_kind = if i == 0 {
            ProfileKind::Xtream
        } else {
            ProfileKind::M3u
        };
    }

    ui.add_space(14.0);
    match app.login_kind {
        ProfileKind::Xtream => {
            gained = field(
                ui,
                "Name (optional)",
                &mut app.f_name,
                "z.B. Wohnzimmer",
                Kb::Name,
                &mut focused_rect,
                &mut false, // show_password not used for Name field
            )
            .or(gained);
            ui.add_space(10.0);
            gained = field(
                ui,
                "Host",
                &mut app.f_host,
                "http://server:8080",
                Kb::Url,
                &mut focused_rect,
                &mut false, // show_password not used for Host field
            )
            .or(gained);
            ui.add_space(10.0);
            gained = field(
                ui,
                "Benutzer",
                &mut app.f_user,
                "Benutzername",
                Kb::Text,
                &mut focused_rect,
                &mut false, // show_password not used for User field
            )
            .or(gained);
            ui.add_space(10.0);
            gained = field(
                ui,
                "Passwort",
                &mut app.f_pass,
                "Passwort",
                Kb::Password,
                &mut focused_rect,
                &mut app.show_password,
            )
            .or(gained);
        }
        ProfileKind::M3u => {
            gained = field(
                ui,
                "M3U-URL",
                &mut app.f_m3u_url,
                "http://…/liste.m3u",
                Kb::Url,
                &mut focused_rect,
                &mut false, // show_password not used for M3U-URL field
            )
            .or(gained);
            ui.add_space(10.0);
            gained = field(
                ui,
                "Lokale Datei",
                &mut app.f_m3u_file,
                "/sdcard/liste.m3u",
                Kb::Url,
                &mut focused_rect,
                &mut false, // show_password not used for Local File field
            )
            .or(gained);
        }
    }

    if app.busy {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.add_space(6.0);
            ui.label(
                RichText::new("Lade Senderliste…")
                    .font(reg(FS_LABEL))
                    .color(muted(ui)),
            );
        });
    }

    ui.add_space(24.0);

    // Scroll only the field currently receiving input into view. An anchor at
    // the bottom of the entire form repeatedly repositions the ScrollArea while
    // the IME inset animates and can make the focused field jump or lose focus.
    follow_focus(ui, l, gained, focused_rect, viewport);
}

/// One field, reporting the rect of the field that just gained focus.
fn field(
    ui: &mut Ui,
    label: &str,
    text: &mut String,
    hint: &str,
    kb: Kb,
    focused: &mut Option<Rect>,
    show_password: &mut bool,
) -> Option<Rect> {
    let r = components::field(ui, label, text, hint, kb, show_password);
    // Report real TextEdit focus (not egui's generic "some widget has focus")
    // so remote D-pad/Enter keys are left alone while the user is typing.
    if r.has_focus() {
        crate::platform::set_text_focus(true);
        *focused = Some(r.rect);
    }
    // Register with the remote focus engine; activating a field opens the IME
    // just like a tap, so a cold start with only a D-pad can still type.
    if focus::clicked(ui, &r) {
        r.request_focus();
    }
    if r.gained_focus() {
        crate::platform::seed_ime(text);
        Some(r.rect)
    } else {
        None
    }
}

/// Keep the focused field above the action bar / soft keyboard.
///
/// The rect MUST be re-measured every frame. The old version cached the rect
/// from the focus-change frame and re-issued `scroll_to_rect(Align::Max)` on
/// every frame while the IME was up. That rect is in screen coordinates, so it
/// goes stale the moment the ScrollArea moves: egui kept "aligning" to an old
/// position, the form jumped back to the top, and the still-focused field ended
/// up clipped underneath the action bar. The keyboard stayed up, the user saw
/// neither cursor nor typed text — the reported "it flickers, nothing happens".
///
/// Now we only scroll when the field's *current* rect is outside the current
/// clip rect, and we use `align = None` ("scroll just enough"). Once the field
/// is visible the call is skipped entirely, so the scroll offset is stable even
/// while the IME inset is still animating in.
fn follow_focus(
    ui: &mut Ui,
    l: &Layout,
    gained: Option<Rect>,
    focused: Option<Rect>,
    viewport: Rect,
) {
    let Some(r) = focused.or(gained) else {
        return;
    };
    let mut target = r;
    // Include the floating label above the field and a little air below it, so
    // the field is not merely "technically visible" flush against the edge.
    target.min.y -= 10.0;
    target.max.y += if l.ime > 0.0 { 16.0 } else { 10.0 };
    // Leave a margin at the bottom: the action bar starts directly below
    // `viewport`, and a field touching it reads as clipped.
    let visible = Rect::from_min_max(
        viewport.min,
        pos2(
            viewport.max.x,
            (viewport.max.y - 8.0).max(viewport.min.y + 8.0),
        ),
    );
    if visible.contains_rect(target) {
        return;
    }
    // Re-issued on every frame where the field is not fully visible. egui
    // panels are one frame behind when the keyboard appears, so a one-shot
    // target is often clamped to zero and never corrected. With
    // `.animated(false)` repeated targets converge in a single frame instead
    // of producing a visible slide.
    ui.scroll_to_rect(target, None);
}

// ------------------------------------------------------- delete confirm ----

/// Has the "Profil löschen" dialog already taken focus this time round?
fn confirm_shown_key() -> egui::Id {
    egui::Id::new("m-confirm-shown")
}

/// Confirmation for deleting a saved profile.
///
/// `delete_profile` rewrites `profiles.json` and cannot be undone, and the
/// trigger is a 48 dp target sitting inside the row whose job is to sign in
/// with that profile. One deliberate second tap is the whole point of this
/// dialog; the destructive action is the *text* button and "Abbrechen" is the
/// filled one, so the visually dominant control is the harmless one.
/// The profile name as it may appear *inside a sentence*.
///
/// Bounded on the variable, not on the sentence. The galley still carries a
/// three-row cap, but a row cap is a net over the whole sentence: it catches a
/// long name by cutting the sentence's tail, so a 60-character name loses the
/// verb instead of losing the name. Capping the name keeps the instruction
/// intact and puts the ellipsis where the reader expects it — on the name.
///
/// Counted in `chars`, not bytes: `len()` on „Сериал" or „Björk-Guðmundsdóttir"
/// is roughly double the visible length, and slicing a `&str` by byte offset
/// panics when it lands inside a multi-byte code point.
fn dialog_name(name: &str) -> String {
    const MAX: usize = 20;
    if name.chars().count() <= MAX {
        return name.to_string();
    }
    let mut out: String = name.chars().take(MAX - 1).collect();
    out.push('…');
    out
}

fn confirm_delete(app: &mut App, ctx: &Context) {
    let Some(id) = app.pending_delete.clone() else {
        return;
    };
    let name = app
        .profiles
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Profil".to_string());

    // Switch the D-pad focus order to this dialog. `1` starts on "Abbrechen":
    // control 0 is the destructive "Löschen", and a TV user must never land on
    // it with the first OK after the dialog appears.
    focus::begin_modal(1);

    let screen = ctx.screen_rect();
    let card_w = 320.0_f32.min(screen.width() - 32.0);
    let inner_w = card_w - 48.0;

    let mut cancel = false;
    let mut confirm = false;

    egui::Area::new(egui::Id::new("m-confirm-delete"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            let (_, scrim) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(160));

            // Copy is specified, not improvised. Title, both body sentences and
            // both button labels were given verbatim by design; the next person
            // to "tidy up" the wording should know it was chosen. Two things in
            // it are deliberate: the destructive button says "Löschen" rather
            // than repeating the title, and the second sentence names what
            // *survives* — `delete_profile` rewrites `profiles.json` only and
            // never touches favourites or progress, so that sentence is true.
            //
            // Body first: the card is centred, so its height has to be known
            // before its top edge can be placed.
            let shown = dialog_name(&name);
            let body_1 = format!("Zugangsdaten für „{shown}“ werden von diesem Gerät entfernt.");
            let body_2 = "Favoriten und Fortschritt bleiben erhalten.";
            // Two galleys, not one string with a newline: `wrap` lays out with
            // `simple_singleline`, which has `break_on_newline = false`. Stacking
            // them at the first one's exact height keeps them line-spaced, so the
            // sentence break is guaranteed without looking like a paragraph gap.
            let g1 = components::wrap(ui, &body_1, reg(FS_SUB), muted(ui), inner_w, 3);
            let g2 = components::wrap(ui, body_2, reg(FS_SUB), muted(ui), inner_w, 2);
            let body_h = g1.size().y + g2.size().y;
            let card_h = 24.0 + 24.0 + 8.0 + body_h + 20.0 + 48.0 + 8.0 + 52.0 + 24.0;

            let card = Rect::from_center_size(
                screen.center(),
                egui::vec2(card_w, card_h.min(screen.height() - 48.0)),
            );
            ui.painter().rect_filled(card, RADIUS, bar(ui));

            let x = card.left() + 24.0;
            let mut y = card.top() + 24.0;

            let title = components::fit(ui, "Profil löschen?", bold(FS_HEAD), text(ui), inner_w);
            ui.painter().galley(egui::pos2(x, y), title, text(ui));
            y += 24.0 + 8.0;

            ui.painter().galley(egui::pos2(x, y), g1.clone(), muted(ui));
            y += g1.size().y;
            ui.painter().galley(egui::pos2(x, y), g2.clone(), muted(ui));
            y += g2.size().y + 20.0;

            // Destructive: text button, not filled — a red slab reads as the
            // primary action on a screen that has no primary action.
            let dr = Rect::from_min_size(egui::pos2(x, y), egui::vec2(inner_w, 48.0));
            let dresp = ui.interact(dr, ui.id().with("confirm-delete"), egui::Sense::click());
            if dresp.is_pointer_button_down_on() {
                ui.painter().rect_filled(dr, RADIUS_SM, pressed(ui));
            }
            let dl = components::fit(ui, "Löschen", bold(FS_BODY), danger(), inner_w - 16.0);
            ui.painter().galley(
                egui::pos2(
                    dr.center().x - dl.size().x * 0.5,
                    dr.center().y - dl.size().y * 0.5,
                ),
                dl,
                danger(),
            );
            if focus::clicked(ui, &dresp) {
                confirm = true;
            }
            y += 48.0 + 8.0;

            let cr = Rect::from_min_size(egui::pos2(x, y), egui::vec2(inner_w, 52.0));
            let cresp = components::button_at(ui, cr, "Abbrechen", true, true);
            // Focus the safe button once, on the frame the dialog appears.
            if ui
                .data(|d| d.get_temp::<bool>(confirm_shown_key()))
                .is_none()
            {
                cresp.request_focus();
                ui.data_mut(|d| d.insert_temp(confirm_shown_key(), true));
            }
            if focus::clicked(ui, &cresp) {
                cancel = true;
            }

            if scrim.clicked() {
                cancel = true;
            }
        });

    if ui_escape(ctx) {
        cancel = true;
    }

    if confirm {
        app.pending_delete = None;
        app.delete_profile(&id);
        ctx.data_mut(|d| d.remove::<bool>(confirm_shown_key()));
        let focused = ctx.memory(|m| m.focused());
        if let Some(f) = focused {
            ctx.memory_mut(|m| m.surrender_focus(f));
        }
    } else if cancel {
        app.pending_delete = None;
        ctx.data_mut(|d| d.remove::<bool>(confirm_shown_key()));
    }
}

/// Escape cancels. Android's back key arrives through `jni_bridge::take_back`
/// and is handled in `App::mobile_back`, which clears `pending_delete`.
fn ui_escape(ctx: &Context) -> bool {
    ctx.input(|i| i.key_pressed(egui::Key::Escape))
}

#[cfg(test)]
mod tests {
    use super::dialog_name;

    #[test]
    fn short_names_pass_through() {
        assert_eq!(dialog_name("Wohnzimmer"), "Wohnzimmer");
        assert_eq!(dialog_name("Profil"), "Profil");
    }

    #[test]
    fn exactly_twenty_is_not_truncated() {
        let n = "a".repeat(20);
        assert_eq!(dialog_name(&n), n);
    }

    #[test]
    fn twenty_one_loses_its_tail_and_gains_one_glyph() {
        let n = "a".repeat(21);
        let out = dialog_name(&n);
        assert_eq!(out.chars().count(), 20);
        assert!(out.ends_with('…'));
        assert_eq!(out, format!("{}…", "a".repeat(19)));
    }

    /// The reason the count is `chars()` and the slice is `chars().take()`:
    /// 20 Cyrillic characters are 40 bytes, so a byte cap would cut this name at
    /// half its length, and `&name[..19]` would panic on the 10th character.
    #[test]
    fn multibyte_names_are_counted_in_characters() {
        let n = "Сериал".repeat(4); // 24 chars, 48 bytes
        assert_eq!(n.chars().count(), 24);
        assert_eq!(n.len(), 48);
        let out = dialog_name(&n);
        assert_eq!(out.chars().count(), 20);
        assert!(out.ends_with('…'));
        assert!(out.chars().all(|c| c.is_alphabetic() || c == '…'));
    }

    #[test]
    fn umlauts_survive_intact() {
        let out = dialog_name("Gästezimmer im Südflügel");
        assert_eq!(out.chars().count(), 20);
        assert!(out.starts_with("Gästezimmer im Sü"));
    }
}
