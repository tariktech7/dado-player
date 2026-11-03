//! Plattform-Abstraktion für dado-player.
//!
//! Bündelt alles, was sich zwischen Desktop (Windows), Android und iOS
//! unterscheidet: die Video-Ausgabefläche, die Engine-Auswahl und kleine
//! System-Helfer. In v1 ist nur Windows real implementiert; Android/iOS sind
//! per `cfg(...)` abgetrennt und werden vom jeweiligen Port ausgefüllt.

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "ios")]
pub mod ios;

// System-Fassade (theme/cursor/DPI/Rect) – re-exportiert pro Ziel aus dem
// passenden OS-Modul, damit `app.rs` plattformneutral greifen kann.
pub mod sys;

// Plattformneutrale Einstiegspunkte – delegieren je nach Ziel an das
// passende OS-Modul, damit `app.rs` keinen `cfg`-Code enthalten muss.
#[cfg(target_os = "windows")]
pub use windows::acquire_surface;
#[cfg(target_os = "android")]
pub use android::acquire_surface;
#[cfg(target_os = "ios")]
pub use ios::acquire_surface;

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
