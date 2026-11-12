//! iOS platform module.
//!
//! Structure mirrors the Android module: an entry point, a native-surface
//! implementation, and a bridge to the host framework (JNI on Android, a C ABI
//! to Swift/UIKit here).
//!
//! The playback engine is deliberately **not** libmpv. iOS has no shipped libmpv
//! binary, no `libloading` story, and licensing/packaging cost that AVPlayer
//! avoids. `crate::mpv` is aliased to an AVPlayer shim on this target (see
//! `src/mpv_ios.rs`) so the shared app code keeps one player shape.

pub mod bridge;
pub mod sys;

// Target-only. These implement `VideoSurface` / `eframe::App` wiring, which
// would collide with the Windows implementations when the module is compiled on
// a non-iOS host for the `ios-check` type check. The ABI layer (`bridge`) and
// the player shim (`mpv_ios`) are the parts that need the check; they live
// above and stay host-independent.
#[cfg(target_os = "ios")]
pub mod entry;
#[cfg(target_os = "ios")]
pub mod video_surface;

use eframe::Frame;

/// Attach the host video view and return no handle.
///
/// This is called every frame from `App::update` (`app.rs`), which is the only
/// reliable "the window exists now" signal we get. The attach is what makes the
/// Swift host create its `RootViewController` — and that controller is the only
/// caller of `dado_app_set_env`. Without this call the environment would not be
/// pushed until the user starts a stream, so on launch the login screen would be
/// painted under the notch with the wrong colour scheme.
///
/// Always returns `None`: the video view belongs to the Swift host and is placed
/// through `bridge::video_frame`, not through a handle. Returning `None` keeps
/// the "borrowed, never owned" contract that `platform::mod` documents.
pub fn acquire_surface(_frame: &mut Frame) -> Option<isize> {
    bridge::attach();
    None
}

/// Insets in **physical pixels**, matching `platform::Insets`.
///
/// UIKit reports points, so everything is multiplied by the screen scale. The
/// rest of the UI works in pixels (the Windows path gets pixels straight from
/// `GetWindowRect`), so converting here keeps the shared layout math honest.
pub fn insets() -> crate::platform::Insets {
    let env = bridge::env();
    let s = env.scale as f32;
    crate::platform::Insets {
        top: env.top as f32 * s,
        bottom: env.bottom as f32 * s,
        left: env.left as f32 * s,
        right: env.right as f32 * s,
        ime: env.ime as f32 * s,
    }
}
