//! A native child window used as the video output surface for libmpv (Windows).
//!
//! It uses its own window class with a **black** background brush: the default
//! `STATIC` class paints white, which flashes while a stream is still loading.

use crate::platform::VideoSurface;
use std::sync::Once;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SetWindowPos, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, WNDCLASSW,
    WS_CHILD, WS_CLIPCHILDREN,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static REGISTER: Once = Once::new();

fn class_name() -> Vec<u16> {
    wide("DadoVideoSurface")
}

fn ensure_class() {
    REGISTER.call_once(|| unsafe {
        let name = class_name();
        let brush = CreateSolidBrush(0x0000_0000); // COLORREF black
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: brush,
            lpszMenuName: std::ptr::null(),
            lpszClassName: name.as_ptr(),
        };
        RegisterClassW(&wc);
    });
}

impl VideoSurface {
    pub fn create(parent: HWND) -> Option<Self> {
        ensure_class();
        let class = class_name();
        let title = wide("");
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_CHILD | WS_CLIPCHILDREN,
                0,
                0,
                100,
                100,
                parent,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            None
        } else {
            Some(VideoSurface::Hwnd(hwnd as isize))
        }
    }

    pub fn position(&self, x: i32, y: i32, w: i32, h: i32) {
        let VideoSurface::Hwnd(hwnd) = self;
        let hwnd = *hwnd as HWND;
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                w.max(1),
                h.max(1),
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn hide(&self) {
        let VideoSurface::Hwnd(hwnd) = self;
        let hwnd = *hwnd as HWND;
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }

    /// Native handle (HWND as `isize`) for binding to mpv `wid` / hit-testing.
    pub fn native_handle(&self) -> isize {
        let VideoSurface::Hwnd(h) = self;
        *h
    }
}

impl Drop for VideoSurface {
    fn drop(&mut self) {
        let VideoSurface::Hwnd(h) = self;
        let h = *h as HWND;
        unsafe {
            DestroyWindow(h);
        }
    }
}
