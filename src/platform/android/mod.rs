//! Android-Plattformmodul (Stub für den Port).
//!
//! Wird erst im Android-Port mit echter Implementierung gefüllt. Bewusst
//! minimale, sauber per `cfg(target_os = "android")` abgetrennte Gerüst-Datei.

pub mod entry;
pub mod jni_bridge;
/// libmpv's OpenGL render API: video as one egui shape.
pub mod render;
pub mod video_surface;

/// System insets (status bar / gesture bar / keyboard) pushed from Java.
pub use jni_bridge as insets;

use eframe::Frame;

/// Liefert das native Handle der Video-Fläche (ANativeWindow) aus dem laufenden
/// `AndroidApp`. mpv nutzt sie über die `android-surface`-Option.
///
/// Rückgabe ist bewusst nur ein `isize` (kein besitzendes `VideoSurface`),
/// damit der Aufrufer das Host-Fenster nicht versehentlich freigibt.
pub fn acquire_surface(_frame: &mut Frame) -> Option<isize> {
    crate::platform::android::entry::native_window().map(|p| p as isize)
}
