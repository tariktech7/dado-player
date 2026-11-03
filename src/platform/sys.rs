//! Plattform-System-Fassade.
//!
//! Reale Implementierungen liegen in den OS-spezifischen Modulen
//! (`windows::sys`); Nicht-Windows-Ziele sind vorerst Stubs und werden im
//! jeweiligen Port ausgefüllt.

#[cfg(target_os = "windows")]
pub use super::windows::sys::*;

#[cfg(not(target_os = "windows"))]
pub mod stubs {
    pub fn system_dark() -> bool {
        true
    }
    pub fn cursor_pos() -> (i32, i32) {
        (0, 0)
    }
    pub fn window_rect(_hwnd: isize) -> Option<(i32, i32, i32, i32)> {
        None
    }
}

#[cfg(not(target_os = "windows"))]
pub use stubs::*;
