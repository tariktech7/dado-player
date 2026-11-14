//! D-pad / TV-remote focus engine for the hand-painted mobile UI.
//!
//! Every control in `src/ui/mobile/` is painted by hand with `ui.interact(...)`
//! / `allocate_exact_size`, so egui's built-in Tab/arrow-key focus never
//! applies. This module keeps one focus cursor per frame: `begin()` runs before
//! drawing, every actionable control registers itself through `clicked()`, and
//! `end()` closes the frame. egui draws on a single thread, so a thread-local
//! avoids threading `&mut` state through ~40 component calls.
//!
//! Two navigation models share that cursor:
//!
//! * **Linear** (phone): Up/Left = previous registered control, Down/Right =
//!   next. Works for vertical lists and row-major poster grids.
//! * **Spatial** (10-foot TV): Up/Down/Left/Right pick the geometrically
//!   nearest control in that direction, like Android's `FocusFinder`. This is
//!   what makes a left headers column plus horizontal card rails navigable
//!   without building an explicit 2-D grid model. The rects come from the
//!   *previous* frame, because `begin()` runs before any control exists.

use std::cell::RefCell;

use egui::{Align, Color32, Rect, Response, Rounding, Stroke, Ui};

use super::tokens;
use crate::platform::RemoteKey;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[derive(Default)]
struct State {
    /// True after the first remote key; hides the ring for touch users.
    active: bool,
    /// Index of the focused control in registration order.
    idx: usize,
    /// Controls registered last frame (for clamping / navigation bounds).
    count: usize,
    /// Controls registered this frame.
    seen: usize,
    /// An Ok/Enter key is waiting to activate the focused control.
    pending_ok: bool,
    /// A direction key was pressed this frame (scroll the focus into view).
    moved: bool,
    /// Fingerprint of the current screen; resetting it restarts focus at 0.
    screen: u64,
    focus_rect: Option<Rect>,
    /// A scrim-covered modal is open, so controls registered before
    /// `begin_modal()` must stay OUT of the D-pad focus order.
    modal: bool,
    /// `begin_modal()` has run; registrations from here on belong to the modal.
    modal_started: bool,
    /// Was a modal open on the previous frame? Used to focus the safe button
    /// exactly once when a dialog appears, without resetting navigation on
    /// every subsequent frame.
    modal_last: bool,
    /// Set on the frame a modal opens; consumed by `begin_modal()`.
    reset_modal_focus: bool,
    /// Geometric neighbour search (TV) instead of registration-order stepping.
    spatial: bool,
    /// Rects registered last frame, indexed exactly like `count`/`idx`.
    rects: Vec<Rect>,
    /// Rects being registered this frame.
    frame_rects: Vec<Rect>,
    /// A caller (BACK, section jump) asked for a specific control.
    queued_idx: Option<usize>,
}

#[derive(Clone, Copy)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Start a frame.
///
/// `keys` are the drained remote keys, `screen` is a fingerprint of the current
/// screen, `pointer_used` reports a real touch/click this frame, `modal` says a
/// scrim-covered overlay is up, and `spatial` selects the 10-foot TV focus
/// model. A real pointer turns the remote focus off so a phone user never sees
/// a stray ring.
pub fn begin(keys: &[RemoteKey], screen: u64, pointer_used: bool, modal: bool, spatial: bool) {
    STATE.with(|cell| {
        let mut s = cell.borrow_mut();
        s.spatial = spatial;
        s.frame_rects.clear();
        if pointer_used {
            s.active = false;
            s.pending_ok = false;
        }
        if screen != s.screen {
            s.screen = screen;
            s.idx = 0;
            // Do NOT zero `count`. It is the only upper bound available before
            // this frame's controls register; zeroing it used to clamp idx back
            // to 0 and swallow the FIRST direction key after every transition.
            // The new screen's real control count is unknown until `end()`, so
            // leave this frame unbounded and clamp there instead. Reusing the
            // OLD screen's count would still swallow the key when coming from
            // a screen with fewer controls.
            s.count = usize::MAX;
        }
        // Entering a modal resets focus onto its controls; leaving one restarts
        // on the underlying page. Both happen once, not on every frame, so
        // direction keys keep working while the dialog stays open.
        s.reset_modal_focus = modal && !s.modal_last;
        if !modal && s.modal_last {
            s.idx = 0;
        }
        s.modal_last = modal;
        s.modal = modal;
        s.modal_started = false;

        // A queued target comes from App::mobile_back() (before this call) or
        // from a section switch during the previous frame's draw. It wins over
        // the screen-change reset, so BACK lands on the header it came from.
        if let Some(i) = s.queued_idx.take() {
            s.idx = i;
            s.active = true;
            s.moved = true;
        }

        // Snapshot: `find_next` needs an immutable slice while `s` is borrowed
        // mutably. At most a few dozen rects, so a clone per frame is cheap.
        let rects = if spatial { s.rects.clone() } else { Vec::new() };
        for k in keys {
            match k {
                RemoteKey::Up | RemoteKey::Down | RemoteKey::Left | RemoteKey::Right => {
                    let dir = match k {
                        RemoteKey::Up => Dir::Up,
                        RemoteKey::Down => Dir::Down,
                        RemoteKey::Left => Dir::Left,
                        _ => Dir::Right,
                    };
                    if spatial {
                        if let Some(next) = find_next(&rects, s.idx, dir) {
                            s.idx = next;
                            s.moved = true;
                        }
                        // Even with no neighbour (edge of a rail) the first key
                        // must reveal the ring; it just does not move it.
                        s.active = true;
                    } else {
                        // Linear next/prev navigation is deliberate on phones:
                        // it works for vertical lists and for row-major poster
                        // grids without a 2-D grid model.
                        match dir {
                            Dir::Up | Dir::Left => s.idx = s.idx.saturating_sub(1),
                            _ => s.idx += 1,
                        }
                        s.active = true;
                        s.moved = true;
                    }
                }
                RemoteKey::Ok | RemoteKey::PlayPause => {
                    s.pending_ok = true;
                    s.active = true;
                }
                RemoteKey::Back => {
                    // BACK is already handled by the Java -> nativeRequestBack()
                    // -> App::mobile_back() chain. Popping here would fire twice.
                    s.active = true;
                }
            }
        }
        s.idx = s.idx.min(s.count.saturating_sub(1));
        s.seen = 0;
        s.focus_rect = None;
    });
}

/// Find the best neighbour in `dir` from the currently focused rect.
///
/// Scoring mirrors Android's `FocusFinder` in spirit: a candidate must lie in
/// the requested direction, then the primary distance dominates, the
/// perpendicular offset is a secondary cost, and leaving the current "beam"
/// (no overlap on the other axis) is penalised heavily. Ties go to the earlier
/// registration, which keeps rails deterministic.
fn find_next(rects: &[Rect], from: usize, dir: Dir) -> Option<usize> {
    let cur = *rects.get(from)?;
    let c = cur.center();
    let mut best: Option<(f32, usize)> = None;
    for (i, r) in rects.iter().enumerate() {
        if i == from {
            continue;
        }
        let cc = r.center();
        let (major, minor, overlap) = match dir {
            Dir::Right => (cc.x - c.x, (cc.y - c.y).abs(), overlap_y(cur, *r)),
            Dir::Left => (c.x - cc.x, (cc.y - c.y).abs(), overlap_y(cur, *r)),
            Dir::Down => (cc.y - c.y, (cc.x - c.x).abs(), overlap_x(cur, *r)),
            Dir::Up => (c.y - cc.y, (cc.x - c.x).abs(), overlap_x(cur, *r)),
        };
        // 1 pt tolerance: controls in the same row/column can differ by a
        // fraction of a point after scroll offsets are applied.
        if major <= 1.0 {
            continue;
        }
        let beam_penalty = if overlap > 0.0 { 0.0 } else { minor * 2.0 };
        let score = major + minor * 1.2 + beam_penalty;
        if best.is_none_or(|(b, _)| score < b - 0.01) {
            best = Some((score, i));
        }
    }
    best.map(|(_, i)| i)
}

fn overlap_x(a: Rect, b: Rect) -> f32 {
    (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0)
}
fn overlap_y(a: Rect, b: Rect) -> f32 {
    (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0)
}

/// Mark the start of the topmost modal layer.
///
/// Everything registered before this call this frame is covered by the scrim:
/// it is silently dropped from the focus order (and cannot be remote-activated),
/// so the dialog's first control becomes index 0. `initial` picks which modal
/// control starts focused — pass 1 for a delete dialog whose safe "Abbrechen"
/// is the second control, so a half-asleep D-pad press cannot delete a profile.
/// The value is applied only on the frame the modal opens.
pub fn begin_modal(initial: usize) {
    STATE.with(|cell| {
        let mut s = cell.borrow_mut();
        s.modal_started = true;
        if s.reset_modal_focus {
            s.idx = initial;
            s.reset_modal_focus = false;
        }
    });
}

/// Register one actionable control and return whether it was activated.
///
/// Call this at the END of a component (or immediately after the `Response` is
/// obtained at the call site), so the focus ring is painted *after* the widget's
/// own painting. Activation is a real pointer click OR the remote Ok key on the
/// focused item; the pending key is consumed so exactly one item fires per press.
pub fn clicked(ui: &Ui, resp: &Response) -> bool {
    STATE.with(|cell| {
        let mut s = cell.borrow_mut();
        // Covered by a modal scrim: drop it from the focus order entirely. It
        // must not take an index (or the dialog's controls would start at 20+),
        // must not paint a ring behind the dialog, and must not be activatable
        // by the remote. Returning false also suppresses a stray pointer click
        // on a disabled underlying control — this is what stops "Anmelden"
        // from submitting while the delete confirm is open.
        if s.modal && !s.modal_started {
            return false;
        }
        let i = s.seen;
        s.seen += 1;
        s.frame_rects.push(resp.rect);
        let focused = s.active && i == s.idx;

        let raw = resp.clicked();
        if focused {
            // A clearly visible ring plus a translucent accent wash. The wash
            // uses accent_soft's RGB but a low alpha so content stays readable
            // (and a full-screen scrim can never wash out the whole page).
            let r = resp.rect.shrink(1.0);
            let soft = tokens::accent_soft(ui);
            let wash_alpha = if s.spatial { 72 } else { 48 };
            let wash = Color32::from_rgba_unmultiplied(soft.r(), soft.g(), soft.b(), wash_alpha);
            ui.painter()
                .rect_filled(r, Rounding::same(tokens::RADIUS_SM), wash);
            // Two strokes: an accent halo for dark surfaces plus a crisp,
            // theme-aware high-contrast line that stays visible on a filled
            // accent button (where accent-on-accent would vanish). Compact TV
            // rows use a restrained high-contrast ring that leaves the text clear.
            let halo = if s.spatial {
                tokens::SEL_RING + 1.5
            } else {
                tokens::SEL_RING + 3.0
            };
            let line = if s.spatial {
                tokens::SEL_RING
            } else {
                tokens::SEL_RING + 1.0
            };
            let rounding = Rounding::same(tokens::RADIUS_SM);
            ui.painter()
                .rect_stroke(r, rounding, Stroke::new(halo, tokens::accent()));
            ui.painter()
                .rect_stroke(r, rounding, Stroke::new(line, tokens::focus_ring(ui)));
            s.focus_rect = Some(resp.rect);
            if s.moved {
                ui.scroll_to_rect(resp.rect, Some(Align::Center));
            }
        }

        let activated = raw || (focused && s.pending_ok);
        if focused && s.pending_ok {
            s.pending_ok = false;
        }
        activated
    })
}

/// Will the *next* `clicked()` registration be the focused one?
///
/// Lets a TV card scale its artwork up while focused without changing the
/// allocated hit rect. Must be called immediately before `clicked()`.
pub fn next_is_focused() -> bool {
    STATE.with(|cell| {
        let s = cell.borrow();
        if s.modal && !s.modal_started {
            return false;
        }
        s.active && s.seen == s.idx
    })
}

/// Close the frame: remember how many controls were registered and clear the
/// per-frame flags. `count` feeds next frame's clamping and navigation bounds.
pub fn end() {
    STATE.with(|cell| {
        let mut s = cell.borrow_mut();
        s.count = s.seen;
        clamp_to_seen(&mut s);
        s.seen = 0;
        s.moved = false;
        s.rects = std::mem::take(&mut s.frame_rects);
        // Keep an unconsumed Ok press for ONE more frame if no control was
        // focused at all (e.g. the list shrank under the cursor). `idx` has just
        // been clamped to a real control, so the press activates something next
        // frame instead of silently dying. If a focused control did register,
        // it already consumed `pending_ok` inside `clicked()`.
        if s.focus_rect.is_some() || s.count == 0 {
            s.pending_ok = false;
        }
    });
}

/// Clamp the cursor against the count of controls that ACTUALLY registered.
/// Running this in `end()` rather than `begin()` is what lets a freshly opened
/// screen honour its first direction key: `begin()` only has the previous
/// frame's count to work with.
fn clamp_to_seen(s: &mut State) {
    if s.count > 0 {
        s.idx = s.idx.min(s.count - 1);
    } else {
        s.idx = 0;
    }
}

/// Drop a queued Ok/Enter press without activating anything.
///
/// Used while a login text field has focus: a remote/hardware `Enter` must
/// never reach the focus engine, because with the IME up the first registered
/// control is "Anmelden" — the key would submit a half-filled form instead of
/// finishing text entry. Called before `focus::begin()`.
pub fn clear_activation() {
    STATE.with(|cell| cell.borrow_mut().pending_ok = false);
}

/// Whether remote focus is currently active (a remote user is navigating).
pub fn active() -> bool {
    STATE.with(|cell| cell.borrow().active)
}

/// Index of the focused control in registration order, if focus is active.
///
/// The TV shell uses this to look up the hero metadata of the focused card, and
/// `App::mobile_back()` uses it to decide whether BACK should return from the
/// content area to the headers column.
pub fn index() -> Option<usize> {
    STATE.with(|cell| {
        let s = cell.borrow();
        s.active.then_some(s.idx)
    })
}

/// Ask `begin()` to place focus on control `i`.
///
/// Safe to call from `App::mobile_back()` (before `begin`) and from a UI
/// callback during `draw` (it then takes effect on the next frame).
pub fn request_index(i: usize) {
    STATE.with(|cell| cell.borrow_mut().queued_idx = Some(i));
}

/// Rect of the focused control this frame, if any.
pub fn focus_rect() -> Option<Rect> {
    STATE.with(|cell| cell.borrow().focus_rect)
}
