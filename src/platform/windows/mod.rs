//! Windows-spezifische Plattform-Helfer.

pub mod video_surface;
pub mod sys;
pub mod entry;

use eframe::Frame;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::platform::VideoSurface;

/// Liefert die native Video-Fläche (Win32-HWND) aus dem egui-Frame.
/// Spiegelt das bisherige `frame.window_handle()`-Lesen in `app.rs`.
pub fn acquire_surface(frame: &mut Frame) -> Option<VideoSurface> {
    if let Ok(wh) = frame.window_handle() {
        if let RawWindowHandle::Win32(w) = wh.as_raw() {
            return Some(VideoSurface::Hwnd(w.hwnd.get() as isize));
        }
    }
    None
}
