//! Platform system facade.
//!
//! Real implementations live in the OS modules (`windows::sys`, `ios::sys`).
//! Targets without one fall back to inert stubs so shared code still compiles.

#[cfg(target_os = "windows")]
pub use super::windows::sys::*;

#[cfg(target_os = "ios")]
pub use super::ios::sys::*;

#[cfg(not(any(target_os = "windows", target_os = "ios")))]
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

#[cfg(not(any(target_os = "windows", target_os = "ios")))]
pub use stubs::*;
