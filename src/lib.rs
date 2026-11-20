//! Shared library crate for dado-player.
//!
//! Enthält alle Module (App, UI, Plattform, Player) sowie die Einstiegspunkte
//! für Desktop (`run`) und Android (`android_main` in `platform::android`).
//!
//! Als `cdylib` gebaut liefert es `libdadoplayer.so` für die Android-APK
//! (Einsteig via `android_main`); als `rlib` wird es von der Desktop-Binary
//! (`Dado-Player`) genutzt.

pub mod app;
pub mod client;
pub mod connect;
pub mod demo;
pub mod fonts;
#[cfg(target_os = "windows")]
pub mod licensing;
pub mod release_info;
// iOS has no libmpv; the AVPlayer shim implements the same public surface.
// Aliasing the *module* keeps `app.rs` free of platform branches.
#[cfg(not(target_os = "ios"))]
pub mod mpv;
#[cfg(target_os = "ios")]
#[path = "platform/ios/avplayer.rs"]
pub mod mpv;
pub mod parental;
pub mod platform;
pub mod player;
pub mod store;
pub mod test_pattern;
pub mod theme;
pub mod types;
pub mod ui;

/// Type-check-only build of the iOS shim on a non-iOS host.
///
/// The real iOS build needs an Apple SDK, which most development machines (and
/// CI runners without a macOS job) do not have. Without this, `platform/ios/avplayer.rs` and
/// `platform::ios` would never be compiled anywhere and would rot silently.
/// Enabled by `--features ios-check`; it wires nothing up, it only compiles.
#[cfg(all(feature = "ios-check", not(target_os = "ios")))]
#[path = "platform/ios/avplayer.rs"]
pub mod mpv_ios;

/// Desktop entry (Windows). Other desktop OSes are not a target.
#[cfg(target_os = "windows")]
pub fn run() -> eframe::Result<()> {
    crate::platform::windows::entry::run()
}

/// iOS entry. Called from the Swift host (`ios/Sources/DadoPlayer/main.swift`)
/// through the C symbol `dado_app_start`.
#[cfg(target_os = "ios")]
pub use platform::ios::entry::dado_app_start;
