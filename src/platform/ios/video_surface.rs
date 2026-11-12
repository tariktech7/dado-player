//! Native video surface for iOS.
//!
//! Unlike Windows there is no child HWND to own: the video is an
//! `AVPlayerLayer`-backed `UIView` that the Swift host creates and keeps. Rust
//! never owns that object, it only *places* it. That mirrors the Windows
//! ownership rule (`acquire_surface` hands out a borrowed handle, never an owned
//! one) — an owned wrapper here would double-release a UIKit object and take the
//! app down on the first frame.

use crate::platform::VideoSurface;

use super::bridge;

impl VideoSurface {
    /// Attach to the host-owned video view.
    ///
    /// The argument is ignored: on iOS the view is created by the Swift host on
    /// demand (`dado_host_attach`), not passed in from a window handle. It is
    /// kept in the signature so `App` can construct a surface the same way on
    /// every platform.
    pub fn create(_handle: *mut std::ffi::c_void) -> Option<Self> {
        bridge::attach();
        Some(VideoSurface::UIView(std::ptr::null_mut()))
    }

    /// Place the native view.
    ///
    /// The caller (`App::position_video`) works in **physical pixels**: it takes
    /// an egui rect in points and multiplies by `pixels_per_point`, which is what
    /// the Windows child HWND needs. UIKit wants points, so this divides by the
    /// screen scale again. Without that division the video would be drawn 2-3x
    /// too large and clipped on every Retina device.
    ///
    /// `bridge::video_frame` deduplicates, so calling this every frame is free
    /// after the first.
    pub fn position(&self, x: i32, y: i32, w: i32, h: i32) {
        let s = bridge::env().scale as f32;
        let s = if s > 0.0 { s } else { 1.0 };
        bridge::video_frame(x as f32 / s, y as f32 / s, w as f32 / s, h as f32 / s);
        bridge::video_visible(true);
    }

    /// No-op on iOS.
    ///
    /// On Windows the chrome sits *above* the video in a child window, so the
    /// only way to show UI over it is to punch holes. On iOS the egui surface is
    /// transparent and drawn over the native video view, so the chrome is
    /// already visible without any region surgery. Keeping the method means the
    /// shared player code does not need a platform branch.
    pub fn set_overlay_cutouts(&self, _holes: &[(i32, i32, i32, i32)]) {}

    pub fn hide(&self) {
        bridge::video_visible(false);
    }

    /// The UIKit view is owned by the host, so there is no valid raw handle to
    /// hand out. Returning 0 is deliberate: any caller that treats this as a
    /// usable view pointer would be wrong on iOS anyway.
    pub fn native_handle(&self) -> isize {
        0
    }
}

impl Drop for VideoSurface {
    fn drop(&mut self) {
        // The host owns the view. Dropping the Rust-side handle must not release
        // it — see the module comment.
    }
}
