//! iOS bridge: the C ABI declared in `ios/Sources/DadoHost/include/DadoBridge.h`.
//!
//! Two directions, and the naming encodes which:
//!
//! * `dado_app_*` — defined **here**, called by Swift.
//! * `dado_host_*` — declared **here**, implemented in Swift.
//!
//! Nothing in this file may assume the host has finished launching: UIKit can
//! call into us before the first frame, and Rust can call into the host before
//! the video view exists. Every entry point therefore degrades to a safe
//! default instead of unwrapping.

use std::ffi::{c_char, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Environment pushed from UIKit. Points, not pixels.
#[derive(Clone, Copy)]
pub struct IosEnv {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
    /// Keyboard overlap, already contained in `bottom`.
    pub ime: f64,
    pub dark: bool,
    pub scale: f64,
    /// Root view bounds in points. Used for aspect-ratio math only.
    pub width: f64,
    pub height: f64,
}

impl IosEnv {
    const fn new() -> Self {
        // A sensible pre-layout default: dark, 1x, no insets. UIKit corrects
        // this on the first `viewDidLayoutSubviews`; until then the UI must not
        // be laid out as if the whole screen were safe area on a notched phone,
        // but it also must not panic.
        IosEnv {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
            ime: 0.0,
            dark: true,
            scale: 1.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

static ENV: Mutex<IosEnv> = Mutex::new(IosEnv::new());

/// Latest environment snapshot. Cheap enough to call per frame.
pub fn env() -> IosEnv {
    *ENV.lock().unwrap_or_else(|e| e.into_inner())
}

/// Has UIKit reported a real layout yet? Used to distinguish "no insets because
/// there are none" from "no insets because we have not been told".
static LAYOUT_SEEN: AtomicBool = AtomicBool::new(false);

pub fn layout_seen() -> bool {
    LAYOUT_SEEN.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------- host API ---

extern "C" {
    fn dado_host_attach();
    fn dado_host_video_frame(x: f64, y: f64, w: f64, h: f64);
    fn dado_host_video_visible(visible: i32);
    fn dado_host_load(url: *const c_char);
    fn dado_host_stop();
    fn dado_host_pause(paused: i32);
    fn dado_host_seek(secs: f64);
    fn dado_host_volume(vol: i32);
    fn dado_host_position() -> f64;
    fn dado_host_duration() -> f64;
    fn dado_host_cache_time() -> f64;
    fn dado_host_paused() -> i32;
    fn dado_host_eof() -> i32;
    fn dado_host_width() -> i64;
    fn dado_host_height() -> i64;
    fn dado_host_fps() -> f64;
}

/// Create the host video view. Safe to call repeatedly, including every frame —
/// the host returns immediately once the view exists.
///
/// There is deliberately no shadow state on this side. An earlier version cached
/// the last frame rect to avoid redundant calls, which broke as soon as the host
/// created its view *late* (it retries until UIKit has a window): the cache said
/// "already sent" while the fresh view sat at full-screen size. Deduplication
/// lives in `VideoHost.setFrame`/`setVisible` instead, where the real view's
/// current state is known. One C call per frame is a direct jump, not worth
/// caching incorrectly.
pub fn attach() {
    unsafe { dado_host_attach() }
}

/// Place the video view. Quantisation to half a point keeps sub-pixel jitter out
/// of UIKit; the host drops frames that are already current.
pub fn video_frame(x: f32, y: f32, w: f32, h: f32) {
    unsafe { dado_host_video_frame(x as f64, y as f64, w as f64, h as f64) }
}

pub fn video_visible(visible: bool) {
    unsafe { dado_host_video_visible(visible as i32) }
}

pub fn load(url: &str) {
    // A `CString` would fail on interior NULs; provider URLs are percent-encoded
    // so this is defensive, not expected. Refusing to load is better than
    // truncating the URL silently.
    let Ok(c) = std::ffi::CString::new(url) else {
        eprintln!("dado: refusing to load a URL containing a NUL byte");
        return;
    };
    unsafe { dado_host_load(c.as_ptr()) }
}

pub fn stop() {
    unsafe { dado_host_stop() }
}

pub fn set_pause(paused: bool) {
    unsafe { dado_host_pause(paused as i32) }
}

pub fn seek(secs: f64) {
    unsafe { dado_host_seek(secs.max(0.0)) }
}

pub fn set_volume(vol: i64) {
    unsafe { dado_host_volume(vol.clamp(0, 100) as i32) }
}

pub fn position() -> f64 {
    unsafe { dado_host_position() }
}

pub fn duration() -> f64 {
    unsafe { dado_host_duration() }
}

pub fn cache_time() -> f64 {
    unsafe { dado_host_cache_time() }
}

pub fn paused() -> bool {
    unsafe { dado_host_paused() != 0 }
}

pub fn eof() -> bool {
    unsafe { dado_host_eof() != 0 }
}

pub fn video_size() -> (i64, i64) {
    unsafe { (dado_host_width(), dado_host_height()) }
}

pub fn fps() -> f64 {
    unsafe { dado_host_fps() }
}

// ------------------------------------------------------------- rust exports ---

/// Absolute path of the writable app directory. Called by Swift before
/// `dado_app_start`.
///
/// `directories` resolves `config_dir()` from `HOME`/`XDG_CONFIG_HOME` on Unix,
/// and returns `None` on iOS because there is no `$HOME`. Pointing those at the
/// sandbox before anything reads a path is the same trick the Android port uses
/// (`jni_bridge::apply_storage_env`) and keeps `store.rs` platform-neutral.
///
/// The pointer lint is allowed deliberately: this is a C entry point, so the
/// signature cannot be `unsafe` in a way that means anything to the caller, and
/// the host passes a NUL-terminated string it owns for the duration of the call.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn dado_app_set_storage_dir(path: *const c_char) {
    if path.is_null() {
        return;
    }
    let Ok(s) = unsafe { CStr::from_ptr(path) }.to_str() else {
        eprintln!("dado: storage dir was not valid UTF-8");
        return;
    };
    if s.is_empty() {
        return;
    }
    // Edition 2021: `set_var` is safe (it only became `unsafe` in 2024).
    std::env::set_var("HOME", s);
    std::env::set_var("XDG_CONFIG_HOME", s);
    // Belt and braces: `ProjectDirs::from("com", "Tarik", "Dado-Player")` appends
    // its own bundle-ish subpath under the config root, which is exactly what we
    // want inside the sandbox.
}

/// Push safe-area insets, appearance and scale from UIKit.
#[no_mangle]
pub extern "C" fn dado_app_set_env(
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
    ime: f64,
    dark: i32,
    scale: f64,
    width: f64,
    height: f64,
) {
    let mut env = ENV.lock().unwrap_or_else(|e| e.into_inner());
    env.top = top.max(0.0);
    env.bottom = bottom.max(0.0);
    env.left = left.max(0.0);
    env.right = right.max(0.0);
    env.ime = ime.max(0.0);
    env.dark = dark != 0;
    // A zero or absurd scale would divide-by-zero every geometry calculation
    // downstream. UIKit always reports 1..3; clamp to that and fall back to 1.
    env.scale = if scale.is_finite() && (0.5..=4.0).contains(&scale) {
        scale
    } else {
        1.0
    };
    env.width = if width.is_finite() && width > 0.0 {
        width
    } else {
        0.0
    };
    env.height = if height.is_finite() && height > 0.0 {
        height
    } else {
        0.0
    };
    drop(env);
    LAYOUT_SEEN.store(true, Ordering::Relaxed);
}

/// Report whether the UI currently owns the keyboard.
///
/// Forwards to the shared flag in `platform::mod` rather than keeping a second
/// copy: the login screen sets that one, and a private duplicate here would read
/// `false` forever.
pub fn set_text_focus(v: bool) {
    crate::platform::set_text_focus(v);
}

pub fn text_focus() -> bool {
    crate::platform::text_focus()
}

/// Current software-keyboard overlap in points, 0 when hidden.
pub fn ime_inset() -> f64 {
    env().ime
}

#[cfg(target_os = "ios")]
mod native_fields {
    use std::collections::HashMap;
    use std::ffi::{c_char, CStr, CString};
    use std::sync::Mutex;
    use std::sync::{LazyLock, OnceLock};
    static VALUES: LazyLock<Mutex<HashMap<u64, String>>> = LazyLock::new(Default::default);
    static FINISHED: Mutex<Option<(u64, bool)>> = Mutex::new(None);
    static CTX: OnceLock<egui::Context> = OnceLock::new();
    extern "C" {
        fn dado_host_edit(
            id: u64,
            title: *const c_char,
            value: *const c_char,
            kind: i32,
            x: f64,
            y: f64,
            w: f64,
            h: f64,
            mode: i32,
            dark: i32,
        );
    }
    pub fn edit(ui: &egui::Ui, resp: &egui::Response, title: &str, value: &str, kind: i32) {
        CTX.get_or_init(|| ui.ctx().clone());
        let id = resp.id.value();
        let order_key = egui::Id::new("inline-order");
        let frame = ui.ctx().frame_nr();
        ui.ctx().data_mut(|d| {
            let order = d.get_temp_mut_or_default::<(u64, Vec<egui::Id>)>(order_key);
            if order.0 != frame {
                order.0 = frame;
                order.1.clear();
            }
            order.1.push(resp.id);
        });
        let r = resp.rect.intersect(ui.clip_rect());
        let mode = if !resp.has_focus() || !r.is_positive() {
            -1
        } else if resp.clicked() || resp.gained_focus() {
            1
        } else {
            0
        };
        if resp.has_focus() {
            let frame = ui.ctx().frame_nr();
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("inline-frame"), frame));
        }
        if let (Ok(t), Ok(v)) = (CString::new(title), CString::new(value)) {
            unsafe {
                dado_host_edit(
                    id,
                    t.as_ptr(),
                    v.as_ptr(),
                    kind,
                    r.left() as f64,
                    r.top() as f64,
                    r.width() as f64,
                    r.height() as f64,
                    mode,
                    ui.visuals().dark_mode as i32,
                )
            }
        }
        let mut done = FINISHED.lock().unwrap();
        if done.as_ref().is_some_and(|(key, _)| *key == id) {
            let (_, next) = done.take().unwrap();
            if next {
                let previous = ui
                    .ctx()
                    .data(|d| d.get_temp::<Vec<egui::Id>>(egui::Id::new("inline-last")))
                    .unwrap_or_default();
                if let Some(following) = previous.iter().skip_while(|x| **x != resp.id).nth(1) {
                    ui.memory_mut(|m| m.request_focus(*following));
                } else {
                    resp.surrender_focus();
                }
            } else {
                resp.surrender_focus();
            }
        }
    }
    pub fn end_frame(ctx: &egui::Context) {
        let frame = ctx.frame_nr();
        ctx.data_mut(|d| {
            if let Some((seen, order)) =
                d.get_temp::<(u64, Vec<egui::Id>)>(egui::Id::new("inline-order"))
            {
                if seen == frame {
                    d.insert_temp(egui::Id::new("inline-last"), order);
                }
            }
        });
        if ctx.data(|d| d.get_temp::<u64>(egui::Id::new("inline-frame"))) != Some(frame) {
            unsafe {
                dado_host_edit(0, c"".as_ptr(), c"".as_ptr(), 0, 0., 0., 0., 0., -1, 0);
            }
        }
    }
    #[no_mangle]
    pub unsafe extern "C" fn dado_app_edited(id: u64, value: *const c_char) {
        if !value.is_null() {
            VALUES
                .lock()
                .unwrap()
                .insert(id, CStr::from_ptr(value).to_string_lossy().into_owned());
        }
        if let Some(ctx) = CTX.get() {
            ctx.request_repaint();
        }
    }
    #[no_mangle]
    pub extern "C" fn dado_app_edit_finished(id: u64, next: i32) {
        *FINISHED.lock().unwrap() = Some((id, next != 0));
        if let Some(ctx) = CTX.get() {
            ctx.request_repaint();
        }
    }
    pub fn edited(id: u64) -> Option<String> {
        VALUES.lock().unwrap().remove(&id)
    }
}
#[cfg(target_os = "ios")]
pub use native_fields::{edit, edited, end_frame};

static PLAYER_CLOSED: AtomicBool = AtomicBool::new(false);
#[no_mangle]
pub extern "C" fn dado_app_player_closed() {
    PLAYER_CLOSED.store(true, Ordering::Relaxed);
}
pub fn take_player_closed() -> bool {
    PLAYER_CLOSED.swap(false, Ordering::Relaxed)
}
