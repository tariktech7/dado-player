//! Android-Plattformmodul (Stub für den Port).
//!
//! Wird erst im Android-Port mit echter Implementierung gefüllt. Bewusst
//! minimale, sauber per `cfg(target_os = "android")` abgetrennte Gerüst-Datei.

pub mod video_surface;
pub mod entry;

use eframe::Frame;
use crate::platform::VideoSurface;

/// Liefert die native Video-Fläche (ANativeWindow) aus dem laufenden
/// `AndroidApp`. mpv nutzt sie über die `android-surface`-Option.
pub fn acquire_surface(_frame: &mut Frame) -> Option<VideoSurface> {
    crate::platform::android::entry::native_window().map(VideoSurface::ANativeWindow)
}
