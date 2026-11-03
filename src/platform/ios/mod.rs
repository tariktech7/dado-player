//! iOS-Plattformmodul (Stub für den Port).
//!
//! Bewusst minimale, per `cfg(target_os = "ios")` abgetrennte Gerüst-Datei.
//! eframe bietet in 0.27 keinen eigenen UIKit-Backend – der iOS-Port bootet
//! über winit; das ist noch zu verifizieren (siehe Entscheidungsbericht).

pub mod video_surface;
pub mod entry;

use eframe::Frame;
use crate::platform::VideoSurface;

/// Liefert die native Video-Fläche (UIView) aus dem egui-Frame.
/// Noch nicht implementiert – Platzhalter für den iOS-Port.
pub fn acquire_surface(_frame: &mut Frame) -> Option<VideoSurface> {
    None
}
