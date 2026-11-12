//! AVPlayer-backed stand-in for the libmpv wrapper, used on iOS.
//!
//! `src/mpv.rs` is libmpv FFI. iOS ships no libmpv: there is no `.so`/`.dylib`
//! to `libloading`, the license obligations differ, and AVFoundation already
//! decodes HLS/HEVC/H.264 in hardware. So this file implements the *same public
//! surface* as `Mpv` and forwards to the Swift host, which owns an `AVPlayer`.
//!
//! Why not a `PlayerBackend` impl: `app.rs` stores a concrete `Mpv` in
//! `App::player` and calls ~30 methods on it. Aliasing the module on iOS keeps
//! `app.rs` free of platform branches; converting the whole app to
//! `dyn PlayerBackend` is the cleaner long-term move and is noted in the plan,
//! but it is a Windows/Android regression risk that this port does not need to
//! take today.
//!
//! Everything that AVPlayer cannot report — codec names, container format,
//! track lists — returns an empty value instead of inventing one. The UI already
//! treats empty stream info as "unknown" and hides those rows.

#![allow(dead_code)]

use crate::platform::ios::bridge;
use crate::types::Settings;

pub struct Mpv {
    volume: std::sync::atomic::AtomicI64,
}

impl Mpv {
    /// No window handle exists on iOS; the video view is the host's.
    pub fn new(_wid: isize, _s: &Settings) -> Result<Self, String> {
        bridge::attach();
        Ok(Mpv {
            volume: std::sync::atomic::AtomicI64::new(100),
        })
    }

    /// Kept for signature parity with the libmpv wrapper. There is no mpv
    /// handle; callers that need one are Android-only (render API).
    pub fn handle(&self) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    pub fn load(&self, url: &str) {
        bridge::load(url);
    }

    /// AVPlayer has no "disable video" switch that keeps audio running for a
    /// live stream. Ignoring the call is better than stopping playback: the
    /// caller uses this for radio-style channels, which still have a poster and
    /// a working audio track.
    pub fn disable_video(&self) {}

    pub fn enable_video(&self) {}

    pub fn stop(&self) {
        bridge::stop();
    }

    pub fn set_pause(&self, paused: bool) {
        bridge::set_pause(paused);
    }

    pub fn toggle_pause(&self) {
        bridge::set_pause(!bridge::paused());
    }

    pub fn seek_absolute(&self, secs: f64) {
        bridge::seek(secs);
    }

    pub fn seek_relative(&self, secs: f64) {
        // AVPlayer seeks to an absolute time; a relative seek is current + delta.
        // Clamp at 0 here rather than letting the host deal with negatives.
        let target = bridge::position() + secs;
        bridge::seek(target.max(0.0));
    }

    pub fn set_volume(&self, vol: i64) {
        let vol = vol.clamp(0, 100);
        self.volume.store(vol, std::sync::atomic::Ordering::Relaxed);
        bridge::set_volume(vol);
    }

    pub fn position(&self) -> f64 {
        bridge::position()
    }

    pub fn duration(&self) -> f64 {
        bridge::duration()
    }

    pub fn cache_time(&self) -> f64 {
        bridge::cache_time()
    }

    pub fn paused(&self) -> bool {
        bridge::paused()
    }

    pub fn eof(&self) -> bool {
        bridge::eof()
    }

    pub fn width(&self) -> i64 {
        bridge::video_size().0
    }

    pub fn height(&self) -> i64 {
        bridge::video_size().1
    }

    pub fn fps(&self) -> f64 {
        bridge::fps()
    }

    /// AVPlayer does not expose codec names. Empty means "unknown" to the UI.
    pub fn video_codec(&self) -> String {
        String::new()
    }

    pub fn audio_codec(&self) -> String {
        String::new()
    }

    pub fn container(&self) -> String {
        String::new()
    }

    pub fn current_audio(&self) -> i64 {
        0
    }

    pub fn current_sub(&self) -> i64 {
        0
    }

    /// No track list over the current bridge. The audio/subtitle popover is
    /// hidden when this returns `None`, which is the correct degradation.
    pub fn track_list_json(&self) -> Option<String> {
        None
    }

    pub fn set_audio_track(&self, _id: i64) {}

    pub fn set_sub_track(&self, _id: i64) {}

    pub fn disable_sub(&self) {}
}

/// iOS bundles AVFoundation; there is no external library to probe for.
pub fn check_library_available() -> Result<(), String> {
    Ok(())
}
