//! Small Windows system helpers (theme, cursor, DPI).

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Read the user's animation preference; never changes system settings.
pub fn reduced_motion() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
    };
    let mut enabled = 1i32;
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&mut enabled as *mut i32).cast(),
            0,
        ) != 0
            && enabled == 0
    }
}

/// Current mouse position in screen pixels.
pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut p) };
    if ok != 0 {
        (p.x, p.y)
    } else {
        (0, 0)
    }
}

/// Outer rect (left, top, right, bottom) of a window in screen pixels.
pub fn window_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    if hwnd == 0 {
        return None;
    }
    let mut r: RECT = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetWindowRect(hwnd as HWND, &mut r) };
    if ok != 0 {
        Some((r.left, r.top, r.right, r.bottom))
    } else {
        None
    }
}

/// True when Windows is set to dark mode for apps.
/// Falls back to `true` (the app's own dark design) if the value is missing.
pub fn system_dark() -> bool {
    let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let name = wide("AppsUseLightTheme");
    let mut val: u32 = 1;
    let mut size: u32 = 4;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut val as *mut u32 as *mut std::ffi::c_void,
            &mut size,
        )
    };
    if rc == 0 {
        val == 0
    } else {
        true
    }
}
