//! Engine-abstrakte Player-Schnittstelle.
//!
//! In v1 ist nur der libmpv-Backend (Desktop/Windows) real; die Android/iOS
//! Backends (MpvBackend auf Android via ANativeWindow, AvPlayerBackend auf iOS)
//! werden im jeweiligen Port nachgezogen. Die Trait-Methoden spiegeln 1:1 die
//! bisher direkt am `Mpv`-Wrapper gerufenen Funktionen – dieser Schritt ist
//! reine Einführung der Abstraktionsebene, es ändert sich kein Verhalten.

#![allow(dead_code)] // Foundation code: Backends/Consumers folgen im Port.

use crate::mpv::Mpv;
use crate::platform::VideoSurface;
use crate::types::Settings;

/// Engine-unabhängige Steuerung eines Wiedergabe-Backends.
///
/// Methodensignatur identisch zur bisherigen `Mpv`-API.
pub trait PlayerBackend {
    fn load(&self, url: &str);
    fn stop(&self);
    fn set_pause(&self, paused: bool);
    fn toggle_pause(&self);
    fn seek_absolute(&self, secs: f64);
    fn seek_relative(&self, secs: f64);
    fn set_volume(&self, vol: i64);
    fn position(&self) -> f64;
    fn duration(&self) -> f64;
    fn cache_time(&self) -> f64;
    fn paused(&self) -> bool;
    fn eof(&self) -> bool;
    /// Bindet die native Video-Fläche an das Backend.
    fn attach_surface(&self, surface: VideoSurface);
}

/// Thin wrapper um den bestehenden libmpv-`Mpv`-Wrapper (Desktop/Windows).
///
/// Keine neue Logik: jeder Aufruf delegiert 1:1. `attach_surface` ist auf
/// Desktop ein No-op, weil das Fenster-Handle (HWND) bereits in `Mpv::new`
/// über die `wid`-Option verdrahtet wird.
pub struct MpvBackend {
    inner: Mpv,
}

impl MpvBackend {
    pub fn new(wid: isize, s: &Settings) -> Result<Self, String> {
        Mpv::new(wid, s).map(|inner| MpvBackend { inner })
    }
}

impl PlayerBackend for MpvBackend {
    fn load(&self, url: &str) {
        self.inner.load(url);
    }
    fn stop(&self) {
        self.inner.stop();
    }
    fn set_pause(&self, paused: bool) {
        self.inner.set_pause(paused);
    }
    fn toggle_pause(&self) {
        self.inner.toggle_pause();
    }
    fn seek_absolute(&self, secs: f64) {
        self.inner.seek_absolute(secs);
    }
    fn seek_relative(&self, secs: f64) {
        self.inner.seek_relative(secs);
    }
    fn set_volume(&self, vol: i64) {
        self.inner.set_volume(vol);
    }
    fn position(&self) -> f64 {
        self.inner.position()
    }
    fn duration(&self) -> f64 {
        self.inner.duration()
    }
    fn cache_time(&self) -> f64 {
        self.inner.cache_time()
    }
    fn paused(&self) -> bool {
        self.inner.paused()
    }
    fn eof(&self) -> bool {
        self.inner.eof()
    }
    fn attach_surface(&self, _surface: VideoSurface) {
        // Desktop: HWND bereits via `wid` in Mpv::new gebunden.
    }
}
