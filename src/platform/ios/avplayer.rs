//! iOS adapter for the shared Mpv API. The Swift host chooses AVPlayer for
//! HLS and embedded VLCKit for file streams, with Dado controls by default.
//! Track/state/control commands cross the same stable C ABI for either engine.
//! Unavailable codec/cache/FPS metadata stays empty rather than being invented.

#![allow(dead_code)]

use crate::platform::ios::bridge;
use crate::types::Settings;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Mpv {
    volume: std::sync::atomic::AtomicI64,
    pending_seek: Mutex<Option<(f64, Instant)>>,
}

impl Mpv {
    /// No window handle exists on iOS; the video view is the host's.
    pub fn new(_wid: isize, s: &Settings) -> Result<Self, String> {
        bridge::attach();
        bridge::native_player(s.ios_native_player);
        bridge::configure(s);
        bridge::set_aspect(s.desktop_aspect);
        Ok(Mpv {
            volume: std::sync::atomic::AtomicI64::new(100),
            pending_seek: Mutex::new(None),
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
        *self.pending_seek.lock().unwrap() = None;
        bridge::load(url);
    }

    /// AVPlayer has no "disable video" switch that keeps audio running for a
    /// live stream. Ignoring the call is better than stopping playback: the
    /// caller uses this for radio-style channels, which still have a poster and
    /// a working audio track.
    pub fn disable_video(&self) {}

    pub fn enable_video(&self) {}

    pub fn stop(&self) {
        *self.pending_seek.lock().unwrap() = None;
        bridge::stop();
    }

    pub fn configure(&self, settings: &Settings) {
        bridge::native_player(settings.ios_native_player);
        bridge::configure(settings);
        bridge::set_aspect(settings.desktop_aspect);
    }

    pub fn set_desktop_aspect(&self, aspect: crate::types::DesktopAspect) {
        bridge::set_aspect(aspect);
    }

    pub fn set_pause(&self, paused: bool) {
        bridge::set_pause(paused);
    }

    pub fn toggle_pause(&self) {
        bridge::set_pause(!bridge::paused());
    }

    pub fn seek_absolute(&self, secs: f64) {
        if !secs.is_finite() {
            return;
        }
        let duration = bridge::duration();
        let target = secs.clamp(
            0.0,
            if duration > 0.0 {
                (duration - 0.25).max(0.0)
            } else {
                f64::INFINITY
            },
        );
        *self.pending_seek.lock().unwrap() = Some((target, Instant::now()));
        bridge::seek(target);
    }

    pub fn seek_relative(&self, secs: f64) {
        let position = bridge::position();
        // AVPlayer and VLC can still report the old decoder position after a seek.
        let pending = *self.pending_seek.lock().unwrap();
        let base = pending
            .filter(|(target, started)| {
                started.elapsed() < Duration::from_millis(1500) && (target - position).abs() > 1.0
            })
            .map_or(position, |(target, _)| target);
        self.seek_absolute(base + secs);
    }

    pub fn set_volume(&self, vol: i64) {
        let vol = vol.clamp(0, 100);
        self.volume.store(vol, std::sync::atomic::Ordering::Relaxed);
        bridge::set_volume(vol);
    }

    pub fn set_speed(&self, speed: f32) {
        bridge::set_speed(speed);
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
