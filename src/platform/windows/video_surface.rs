//! A native child window used as the video output surface for libmpv (Windows).
//!
//! It uses its own window class with a **black** background brush: the default
//! `STATIC` class paints white, which flashes while a stream is still loading.

use crate::platform::VideoSurface;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, CreateSolidBrush, DeleteObject, SetWindowRgn, RGN_DIFF,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClassNameW, GetClientRect, RegisterClassW,
    SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW,
    SW_HIDE, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN,
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
            lpfnWndProc: Some(video_window_proc),
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
    /// `parent` is the application's own main HWND. It is only handed to the
    /// OS (`CreateWindowExW`), never dereferenced by Rust code, and the only
    /// caller lives inside this crate — so `pub(crate)` is both sufficient and
    /// avoids Clippy's `not_unsafe_ptr_arg_deref` false positive for a public
    /// function taking a raw window handle.
    pub(crate) fn create(parent: HWND) -> Option<Self> {
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

    /// Clip the *visible* part of the video window WITHOUT changing its size.
    ///
    /// This HWND is a `WS_CHILD` and therefore paints ON TOP of the egui
    /// parent surface. mpv re-fits and re-scales the picture whenever the
    /// window size changes, so the old approach (shrink the HWND whenever
    /// player chrome appeared, hid or a menu opened) made the image jump on
    /// every UI action. Now `position_video()` keeps one fixed rect for the
    /// whole playback and the chrome is expressed as holes in the window
    /// region: the parent's egui pixels show through there while mpv keeps
    /// rendering the frame at exactly one scale.
    ///
    /// `holes` are `(x, y, w, h)` in HWND CLIENT coordinates (physical
    /// pixels), relative to the video rect's top-left. An empty slice removes
    /// the region and restores the full, unclipped surface.
    pub fn set_overlay_cutouts(&self, holes: &[(i32, i32, i32, i32)]) {
        let VideoSurface::Hwnd(hwnd) = self;
        let hwnd = *hwnd as HWND;
        unsafe {
            if holes.is_empty() {
                // A NULL region restores the whole rectangle; the system frees
                // the previous HRGN itself.
                SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
                return;
            }
            let mut rc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut rc) == 0 || rc.right <= 0 || rc.bottom <= 0 {
                return;
            }
            let full = CreateRectRgn(0, 0, rc.right, rc.bottom);
            if full.is_null() {
                return;
            }
            for &(hx, hy, hw, hh) in holes {
                if hw <= 0 || hh <= 0 {
                    continue;
                }
                let cut = CreateRectRgn(hx, hy, hx.saturating_add(hw), hy.saturating_add(hh));
                if cut.is_null() {
                    continue;
                }
                // RGN_DIFF = full - cut. `CombineRgn` explicitly allows the
                // destination to also be one of its sources.
                CombineRgn(full, full, cut, RGN_DIFF);
                DeleteObject(cut);
            }
            // After a successful SetWindowRgn the SYSTEM owns `full` and frees
            // it together with the window. Deleting it here would corrupt the
            // live region; only clean up when the call failed.
            if SetWindowRgn(hwnd, full, 1) == 0 {
                DeleteObject(full);
            }
        }
    }

    pub fn hide(&self) {
        let VideoSurface::Hwnd(hwnd) = self;
        let hwnd = *hwnd as HWND;
        unsafe {
            // Drop any popover cutout before hiding, so the next stream starts
            // with a full, unclipped surface.
            SetWindowRgn(hwnd, std::ptr::null_mut(), 0);
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
        // Safety net: only ever destroy windows this module created. Wrapping a
        // foreign handle (e.g. the egui host window) in a `VideoSurface` used to
        // make `Drop` call `DestroyWindow` on the app's own main window, which
        // silently killed the surface/GL context a frame later.
        let mut buf = [0u16; 64];
        let len = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return;
        }
        // `class_name()` ends in a NUL; compare against its exact character
        // count. A foreign window class can be longer than our buffer/name, so
        // an unguarded `class_name()[..len]` would panic - and with
        // `panic = "abort"` that would kill the whole player.
        let class = class_name();
        let len = len as usize;
        if len != class.len() - 1 || len > buf.len() || buf[..len] != class[..len] {
            return;
        }
        unsafe {
            DestroyWindow(h);
        }
    }
}

static VIDEO_CLICKED: AtomicBool = AtomicBool::new(false);
static REPAINT: OnceLock<egui::Context> = OnceLock::new();
pub fn register_repaint(ctx: &egui::Context) {
    let _ = REPAINT.set(ctx.clone());
}
pub fn take_click() -> bool {
    VIDEO_CLICKED.swap(false, Ordering::Relaxed)
}
unsafe extern "system" fn video_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP {
        VIDEO_CLICKED.store(true, Ordering::Relaxed);
        // The child video HWND receives this instead of winit. Wake egui now,
        // rather than waiting for a later click on the parent window.
        windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(
            windows_sys::Win32::UI::WindowsAndMessaging::GetParent(hwnd),
        );
        if let Some(ctx) = REPAINT.get() {
            ctx.request_repaint();
        }
    } else if message == windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE {
        if let Some(ctx) = REPAINT.get() {
            ctx.request_repaint();
        }
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}
