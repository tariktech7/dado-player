#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The desktop binary is a thin shell around the shared `dadoplayer` library
// crate. On Android the library is built as a `cdylib` and `android_main`
// (in `platform::android::entry`) is the real entry point instead.

#[cfg(target_os = "windows")]
fn main() -> eframe::Result<()> {
    dadoplayer::run()
}

#[cfg(not(target_os = "windows"))]
fn main() {}
