//! Windows-Einstiegspunkt.
//!
//! Enthält die ursprüngliche `main()`-Logik, damit `main.rs` nur noch an die
//! plattformspezifische Entry delegieren kann.

use crate::app;
use crate::client;
use crate::types::*;
use eframe::egui;
use tokio::runtime::Runtime;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

pub fn run() -> eframe::Result<()> {
    if std::env::var("IPTV_SELFTEST").is_ok() {
        run_selftest();
        return Ok(());
    }
    let rt = Runtime::new().expect("tokio runtime");
    let mut viewport = initial_viewport();
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Dado-Player",
        options,
        Box::new(move |_cc| Box::new(app::App::new(rt))),
    )
}

/// Brand icon, embedded so the exe stays self-contained.
fn app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../../../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

/// Pick a window size that always fits inside the usable desktop area.
/// Without this the player's control bar can end up hidden behind the taskbar
/// on smaller screens or on TVs with a high scaling factor.
fn initial_viewport() -> egui::ViewportBuilder {
    let fallback = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 760.0])
        .with_min_inner_size([880.0, 540.0]);

    let mut r: RECT = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0)
    };
    if ok == 0 {
        return fallback;
    }
    let dpi = unsafe { GetDpiForSystem() };
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    let (ww, wh) = (
        (r.right - r.left) as f32 / scale,
        (r.bottom - r.top) as f32 / scale,
    );
    if ww < 320.0 || wh < 240.0 {
        return fallback;
    }
    let margin = 18.0_f32;
    // Reserve room for the title bar and the invisible resize border, otherwise
    // the window frame pushes the client area past the work area again.
    let frame = 48.0_f32;
    let w = (ww - margin * 2.0).min(1400.0);
    let h = (wh - margin * 2.0 - frame).min(880.0).max(480.0);
    let x = (r.left as f32 / scale + (ww - w) * 0.5).max(0.0);
    let y = (r.top as f32 / scale + (wh - h - frame) * 0.5).max(0.0);
    egui::ViewportBuilder::default()
        .with_inner_size([w, h])
        .with_min_inner_size([w.min(880.0), h.min(540.0)])
        .with_position([x, y])
}

/// Diagnostic path: validate Xtream login + library loading without saving anything.
/// Credentials are read from env vars only (XT_HOST / XT_USER / XT_PASS).
fn run_selftest() {
    let host = std::env::var("XT_HOST").unwrap_or_default();
    let user = std::env::var("XT_USER").unwrap_or_default();
    let pass = std::env::var("XT_PASS").unwrap_or_default();
    if host.is_empty() {
        println!("ERR XT_HOST fehlt");
        return;
    }
    let rt = Runtime::new().expect("tokio runtime");
    let mut p = Profile::new("selftest", ProfileKind::Xtream);
    p.host = host;
    p.username = user;
    p.password = pass;
    match rt.block_on(client::load_profile(&p, 25)) {
        Ok(lib) => println!(
            "OK channels={} livecats={} movies={} vodcats={} series={} seriascats={} stream_base={}",
            lib.channels.len(),
            lib.live_categories.len(),
            lib.movies.len(),
            lib.vod_categories.len(),
            lib.series.len(),
            lib.series_categories.len(),
            lib.stream_base
        ),
        Err(e) => println!("ERR {}", e),
    }
}
