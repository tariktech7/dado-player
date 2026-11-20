//! Plattform-Abstraktion für dado-player.
//!
//! Bündelt alles, was sich zwischen Desktop (Windows), Android und iOS
//! unterscheidet: die Video-Ausgabefläche, die Engine-Auswahl und kleine
//! System-Helfer. Die Zielplattform wählt per `cfg` den passenden Adapter.

use std::sync::atomic::{AtomicBool, Ordering};

/// A login text field currently owns the keyboard.
///
/// Set once per frame by `ui::mobile::login` from the real `TextEdit`
/// responses, so `App` can tell "the user is typing" apart from "the user is
/// navigating with a D-pad". `ctx.wants_keyboard_input()` is not precise
/// enough here: it is true for *any* focused widget, including the focused
/// "Abbrechen" button in the delete dialog.
static TEXT_FOCUS: AtomicBool = AtomicBool::new(false);

pub fn set_text_focus(v: bool) {
    TEXT_FOCUS.store(v, Ordering::Relaxed);
}

pub fn text_focus() -> bool {
    TEXT_FOCUS.load(Ordering::Relaxed)
}

#[cfg(target_os = "android")]
pub mod android;
// `ios-check` compiles the iOS module on a non-iOS host purely for type
// checking; it is not wired into the platform facade in that case (see the
// `pub use` block below).
#[cfg(any(target_os = "ios", feature = "ios-check"))]
pub mod ios;
#[cfg(target_os = "windows")]
pub mod windows;

// System-Fassade (theme/cursor/DPI/Rect) – re-exportiert pro Ziel aus dem
// passenden OS-Modul, damit `app.rs` plattformneutral greifen kann.
pub mod sys;

// Plattformneutrale Einstiegspunkte – delegieren je nach Ziel an das
// passende OS-Modul, damit `app.rs` keinen `cfg`-Code enthalten muss.
#[cfg(target_os = "android")]
pub use android::acquire_surface;
#[cfg(target_os = "ios")]
pub use ios::acquire_surface;
#[cfg(target_os = "windows")]
pub use windows::acquire_surface;

/// Plattform-neutrale Beschreibung der nativen Video-Ausgabefläche.
///
/// Pro Zielplattform existiert nur die jeweils passende Variante (die anderen
/// sind per `cfg` auskommentiert, damit sie off-target nicht kompilieren).
pub enum VideoSurface {
    #[cfg(target_os = "windows")]
    Hwnd(isize),
    #[cfg(target_os = "android")]
    ANativeWindow(*mut std::ffi::c_void),
    #[cfg(target_os = "ios")]
    UIView(*mut std::ffi::c_void),
}

/// System insets in physical pixels.
#[derive(Clone, Copy, Default)]
pub struct Insets {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
    /// Soft-keyboard height (already contained in `bottom`).
    pub ime: f32,
}

/// Hand the soft keyboard the text that is already in the focused field.
/// No-op on desktop, where the IME is driven by winit.
pub fn seed_ime(_text: &str) {
    #[cfg(target_os = "android")]
    {
        android::insets::seed(_text);
    }
}

/// Tell Android which keyboard to raise for the field that just gained focus.
/// `input_type` is an `android.text.InputType` bitmask. No-op on desktop.
pub fn set_input_type(_input_type: i32) {
    #[cfg(target_os = "android")]
    {
        android::insets::set_input_type(_input_type);
    }
}

/// Hand the stream to an external video player (MX Player / VLC / chooser).
///
/// Only Android has an `ACTION_VIEW` resolver; on desktop the caller falls
/// back to the built-in libmpv player. The error string is deliberately
/// generic: provider URLs carry username/password in the path, and nothing
/// returned from here may echo the URL into the UI.
pub fn open_external(
    _url: &str,
    _title: &str,
    _backend: crate::types::PlayerBackend,
    _key: &str,
    _position: u64,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::jni_bridge::open_external(_url, _title, _backend, _key, _position)
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("Externe Player sind nur auf Android verfügbar.".to_string())
    }
}

/// Status bar, gesture bar, cutout and keyboard insets. Android and iOS report
/// real values; the desktop window has no system chrome.
pub fn insets() -> Insets {
    #[cfg(target_os = "android")]
    {
        android::insets::current()
    }
    #[cfg(target_os = "ios")]
    {
        ios::insets()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Insets::default()
    }
}

/// A key from a TV remote / D-pad / hardware navigation cluster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteKey {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Back,
    PlayPause,
    Recall,
}

/// Is this a television device?
///
/// On Android the answer comes from `UiModeManager.getCurrentModeType()`, which
/// Java pushes into the JNI bridge from `GameActivity.onCreate` — Rust cannot
/// reach that service on its own. On desktop `IPTV_TV_DEVICE` is the QA hook;
/// the user-facing override lives in `Settings::tv_mode`.
pub fn tv_device() -> bool {
    #[cfg(target_os = "android")]
    {
        android::jni_bridge::tv_device()
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::var("IPTV_TV_DEVICE").is_ok()
    }
}

/// Drain all remote keys queued since the last frame. Always empty on desktop.
pub fn drain_remote_keys() -> Vec<RemoteKey> {
    #[cfg(target_os = "android")]
    {
        android::jni_bridge::drain_keys()
    }
    #[cfg(not(target_os = "android"))]
    {
        Vec::new()
    }
}
