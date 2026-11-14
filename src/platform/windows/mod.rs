//! Windows-spezifische Plattform-Helfer.

pub mod entry;
pub mod identity;
pub mod sys;
pub mod update;
pub mod video_surface;

use eframe::Frame;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Liefert das native Handle des **Host-Fensters** (Win32-HWND) aus dem
/// egui-Frame.
///
/// Wichtig: Es wird bewusst nur ein `isize` zurückgegeben und **keine**
/// `VideoSurface`. `VideoSurface` besitzt ein Fenster und ruft in `Drop`
/// `DestroyWindow` auf – würde man damit das Host-Fenster verpacken, würde der
/// Player beim Verlassen des `if let`-Blocks sein eigenes Hauptfenster
/// zerstören (Surface/GL-Kontext werden danach ungültig).
pub fn acquire_surface(frame: &mut Frame) -> Option<isize> {
    if let Ok(wh) = frame.window_handle() {
        if let RawWindowHandle::Win32(w) = wh.as_raw() {
            return Some(w.hwnd.get());
        }
    }
    None
}
