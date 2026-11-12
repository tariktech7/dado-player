//! iOS entry point.
//!
//! Unlike Android there is no `android_main` equivalent that the platform calls
//! for us: on iOS the process starts in Swift (`main.swift`), which calls
//! [`dado_app_start`] exactly once. That function hands the main thread to
//! winit, and winit's iOS backend drives `UIApplicationMain` from there.
//!
//! This is why the Swift host must not use `@main` + SwiftUI: two owners of
//! `UIApplicationMain` is not a supported arrangement.

use std::sync::Once;

/// Renderer choice on iOS is not cosmetic.
///
/// * eframe's `glow` backend goes through `glutin`, and glutin has **no iOS
///   backend** — it cannot create a GL context for a `UIView`.
/// * The `wgpu` backend does: `wgpu-hal`'s Metal backend accepts a
///   `raw-window-handle` from winit's `UIView` and creates a `CAMetalLayer`.
///
/// So iOS pins `Renderer::Wgpu`. This is the opposite of Android, where the
/// renderer is pinned to `Glow` because (a) the mpv frame is composited through
/// `egui_glow::CallbackFn` and (b) some Android GL drivers reject wgpu's 64 KiB
/// uniform-buffer request. Do not "unify" the two.
const IOS_RENDERER: eframe::Renderer = eframe::Renderer::Wgpu;

static STARTED: Once = Once::new();

/// Boots the Rust application. Called from Swift `main.swift`.
///
/// Does not return under normal operation: winit runs the UIKit application
/// loop. If it *does* return, startup failed — we print the reason and return so
/// Swift can fall back to a visible error instead of a silent black screen.
#[no_mangle]
pub extern "C" fn dado_app_start() {
    STARTED.call_once(|| {
        if let Err(e) = run() {
            eprintln!("dado: eframe::run_native failed on iOS: {e:?}");
        }
    });
}

fn run() -> eframe::Result<()> {
    // The video view is a native `AVPlayerLayer` owned by the Swift host, placed
    // by `dado_host_video_frame`. egui draws on top of it. That means the egui
    // window must be transparent where the video shows through — the host sets
    // the video view behind the UI, so the UI simply does not paint there.
    let native_options = eframe::NativeOptions {
        renderer: IOS_RENDERER,
        viewport: egui::ViewportBuilder::default().with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "Dado Player",
        native_options,
        Box::new(|_cc| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            Box::new(crate::app::App::new(rt))
        }),
    )
}
