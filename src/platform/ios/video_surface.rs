//! Native Video-Fläche für iOS (UIView). Stub.

use crate::platform::VideoSurface;

impl VideoSurface {
    pub fn create(_view: *mut std::ffi::c_void) -> Option<Self> {
        Some(VideoSurface::UIView(std::ptr::null_mut()))
    }

    pub fn position(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    pub fn hide(&self) {}

    /// Native handle (UIView-Pointer als `isize`).
    pub fn native_handle(&self) -> isize {
        match self {
            VideoSurface::UIView(p) => *p as isize,
        }
    }
}

impl Drop for VideoSurface {
    fn drop(&mut self) {}
}
