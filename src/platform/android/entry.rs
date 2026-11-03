//! Android-Einstiegspunkt.
//!
//! Wird von GameActivity (über das `android-game-activity`-Crate) aufgerufen.
//! Hält das `AndroidApp`-Handle in einer `OnceLock`, damit `acquire_surface`
//! zur Laufzeit die `ANativeWindow` für libmpv beschaffen kann.

use std::sync::OnceLock;

use android_activity::AndroidApp;

static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    let _ = ANDROID_APP.set(app);
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Dado-Player",
        native_options,
        Box::new(|_cc| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            Ok(Box::new(crate::app::App::new(rt)))
        }),
    );
}

/// Liefert das `ANativeWindow` der aktuellen GameActivity als rohes Pointer.
///
/// mpv bindet es über die `android-surface`-Option. `None`, falls (noch) kein
/// natives Fenster verfügbar ist.
pub fn native_window() -> Option<*mut std::ffi::c_void> {
    let app = ANDROID_APP.get()?;
    let window = app.native_window()?;
    Some(window.ptr() as *mut std::ffi::c_void)
}
