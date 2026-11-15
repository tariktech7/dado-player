//! iOS adapter for the shared Mpv API. The Swift host chooses AVPlayer for
//! HLS and embedded VLCKit for file streams, with Dado controls by default.
//! Track/state/control commands cross the same stable C ABI for either engine.
//! Unavailable codec/cache/FPS metadata stays empty rather than being invented.

#![allow(dead_code)]

use crate::platform::ios::bridge;
use crate::types::Settings;

pub struct Mpv {
    volume: std::sync::atomic::AtomicI64,
}

impl Mpv {
    /// No window handle exists on iOS; the video view is the host's.
    pub fn new(_wid: isize, s: &Settings) -> Result<Self, String> {
        bridge::attach();
        bridge::native_player(s.ios_native_player);
        bridge::configure(s);
        Ok(Mpv {
            volume: std::sync::atomic::AtomicI64::new(100),
        })
    }

    /// Kept for signature parity with the libmpv wrapper. There is no mpv
    /// handle; callers that need one are Android-only (render API).
    pub fn handle(&self) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    pub fn load_at(&self, url: &str, seconds: f64) {
        self.load(url);
        if seconds > 0.0 {
            self.seek_absolute(seconds);
        }
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

    pub fn configure(&self, settings: &Settings) {
        bridge::native_player(settings.ios_native_player);
        bridge::configure(settings);
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
        bridge::current_track(0)
    }

    pub fn current_sub(&self) -> i64 {
        bridge::current_track(1)
    }

    /// Track IDs and language metadata come from the currently active engine.
    pub fn track_list_json(&self) -> Option<String> {
        bridge::tracks()
    }

    pub fn set_audio_track(&self, id: i64) {
        bridge::select_track(0, id);
    }

    pub fn set_sub_track(&self, id: i64) {
        bridge::select_track(1, id);
    }

    pub fn disable_sub(&self) {
        bridge::select_track(1, -1);
    }
}

/// iOS bundles AVFoundation; there is no external library to probe for.
pub fn check_library_available() -> Result<(), String> {
    Ok(())
}
