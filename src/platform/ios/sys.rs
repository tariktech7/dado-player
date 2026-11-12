//! iOS system facade: appearance, geometry, and the video view handle.
//!
//! Replaces the `stubs` module in `platform::sys` for `target_os = "ios"` so
//! `app.rs` stays free of `cfg` branches.

use super::bridge;

/// Follow the system appearance. UIKit pushes it through `dado_app_set_env` on
/// every `traitCollectionDidChange`; before the first push we report dark, which
/// matches the app's default palette.
pub fn system_dark() -> bool {
    bridge::env().dark
}

/// iOS has no cursor. Kept for signature compatibility with the desktop path
/// (auto-hide logic uses it to decide whether the pointer is over the chrome).
pub fn cursor_pos() -> (i32, i32) {
    (0, 0)
}

/// Bounds of the root view in points, as `(x, y, w, h)`.
///
/// The desktop version takes an `HWND`; iOS has a single full-screen root view,
/// so the argument is ignored. `None` until UIKit has reported a layout, which
/// lets callers distinguish "not measured yet" from "zero-sized".
pub fn window_rect(_handle: isize) -> Option<(i32, i32, i32, i32)> {
    if !bridge::layout_seen() {
        return None;
    }
    let env = bridge::env();
    if env.width <= 0.0 || env.height <= 0.0 {
        return None;
    }
    Some((0, 0, env.width as i32, env.height as i32))
}
