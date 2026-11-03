//! Shared library crate for dado-player.
//!
//! Enthält alle Module (App, UI, Plattform, Player) sowie die Einstiegspunkte
//! für Desktop (`run`) und Android (`android_main` in `platform::android`).
//!
//! Als `cdylib` gebaut liefert es `libdadoplayer.so` für die Android-APK
//! (Einsteig via `android_main`); als `rlib` wird es von der Desktop-Binary
//! (`Dado-Player`) genutzt.

pub mod app;
pub mod platform;
pub mod client;
pub mod demo;
pub mod mpv;
pub mod store;
pub mod theme;
pub mod types;
pub mod ui;
pub mod player;

/// Desktop-Einstieg (Windows). Andere Desktop-OS sind nicht Zielplattform.
#[cfg(target_os = "windows")]
pub fn run() -> eframe::Result<()> {
    crate::platform::windows::entry::run()
}
