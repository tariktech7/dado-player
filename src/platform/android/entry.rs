//! Android-Einstiegspunkt.
//!
//! Wird von GameActivity (über das `android-game-activity`-Crate) aufgerufen.
//! Hält das `AndroidApp`-Handle in einer `OnceLock`, damit `acquire_surface`
//! zur Laufzeit die `ANativeWindow` für libmpv beschaffen kann.

use std::sync::OnceLock;

use android_activity::AndroidApp;

static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

/// Shared handle for the JNI bridge (keyboard, insets, back).
pub fn app_handle() -> Option<&'static AndroidApp> {
    ANDROID_APP.get()
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    // Point `directories` at the app's private files dir before anything reads
    // a path. `nativeOnStorage` normally did this already from `onCreate`;
    // re-applying is idempotent and covers the case where the Java callback
    // has not landed yet (the inset bridge already has to degrade on that same
    // ordering, so it is not contractual).
    if let Some(dir) = crate::platform::android::jni_bridge::storage_dir() {
        crate::platform::android::jni_bridge::apply_storage_env(dir);
    }

    // Keep one handle for mpv's native-window lookup (see `native_window()`);
    // the original is moved into the event-loop hook below.
    let _ = ANDROID_APP.set(app.clone());

    let mut native_options = eframe::NativeOptions::default();

    // Cargo.toml enables eframe's `wgpu` feature for the Windows backend
    // dispatcher and `glow` for Android; feature unification means *both* are
    // on for the Android build. eframe 0.27 then makes `Renderer::default()`
    // return `Wgpu` ("if both features are enabled, prefer wgpu"). That is
    // wrong here twice over:
    //   * wgpu requests a 64 KiB `max_uniform_buffer_binding_size`; several
    //     Android GL/Vulkan drivers (and the SwiftShader emulator) only expose
    //     16 KiB, so `run_native` fails with `LimitsExceeded` and the activity
    //     closes to a black screen.
    //   * the mpv video frame is drawn through `egui_glow::CallbackFn`, which
    //     only the glow painter executes. Under wgpu it is silently dropped.
    // Pin the renderer that the video path actually requires.
    native_options.renderer = eframe::Renderer::Glow;

    // winit 0.29 (pulled in by eframe) requires the `AndroidApp` to be attached
    // to the event-loop builder on Android, otherwise `EventLoop::build()` panics
    // with "An `AndroidApp` as passed to android_main() is required ...".
    native_options.event_loop_builder = Some(Box::new(move |builder| {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        builder.with_android_app(app);
    }));

    // A discarded Result used to turn a failed GL/context init into a silent
    // black screen followed by `finish()`: `android_main` returns, the glue
    // destroys the app, and logcat contains no clue. Say it out loud.
    if let Err(e) = eframe::run_native(
        "Dado-Player",
        native_options,
        Box::new(|_cc| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            // eframe 0.27's AppCreator returns `Box<dyn App>` directly.
            Box::new(crate::app::App::new(rt))
        }),
    ) {
        eprintln!("dado: eframe::run_native failed: {e:?}");
    }
}

/// Liefert das `ANativeWindow` der aktuellen GameActivity als rohes Pointer.
///
/// mpv bindet es über die `android-surface`-Option. `None`, falls (noch) kein
/// natives Fenster verfügbar ist.
pub fn native_window() -> Option<*mut std::ffi::c_void> {
    let app = ANDROID_APP.get()?;
    let window = app.native_window()?;
    // `ptr()` yields `NonNull<ANativeWindow>`; take the raw pointer from it.
    Some(window.ptr().as_ptr() as *mut std::ffi::c_void)
}
