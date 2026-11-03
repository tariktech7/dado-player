//! Native Video-Fläche für Android (ANativeWindow). Stub.
//!
//! Die echte Implementierung erzeugt/positioniert ein `ANativeWindow` und
//! übergibt es an libmpv via `android-surface`. Hier nur das Gerüst.

use crate::platform::VideoSurface;

impl VideoSurface {
    pub fn create(window: *mut std::ffi::c_void) -> Option<Self> {
        Some(VideoSurface::ANativeWindow(window))
    }

    pub fn position(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    pub fn hide(&self) {}

    /// Native handle (ANativeWindow-Pointer als `isize`) für libmpv.
    pub fn native_handle(&self) -> isize {
        match self {
            VideoSurface::ANativeWindow(p) => *p as isize,
        }
    }
}

impl Drop for VideoSurface {
    fn drop(&mut self) {}
}
