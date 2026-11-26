//! JNI bridge for the things winit does not deliver on Android.
//!
//! Verified in the crate sources:
//! * `winit-0.29.15/src/platform_impl/android/mod.rs:903` — `set_ime_allowed()`
//!   is an **empty function**, so `egui` can never raise the soft keyboard.
//!   `android-activity` however exposes `show_soft_input()` / `hide_soft_input()`
//!   and a `gametextinput.Listener` that the Java `GameActivity` already
//!   implements. `GameActivity.java` overrides `stateChanged()` and forwards the
//!   full IME state here; the text is then replayed into `egui` as events.
//! * Edge-to-edge is forced by GameActivity 2.x, so the window never resizes for
//!   the keyboard or the gesture bar. The insets are pushed from
//!   `onApplyWindowInsets` instead.
//! * `AppCompatActivity` owns back handling, so the activity asks us whether the
//!   UI can pop a screen (`nativeCanGoBack`) before falling back to `finish()`.
//!
//! Rule for this module: **every `env.*` call made from the `android_main`
//! thread must clear its exception on the error path.** A pending exception
//! does not surface at the failing call — it aborts the process at the *next*
//! JNI call under CheckJNI (debug builds) and is silent in release. Never call
//! `FindClass` there either: with no Java frame on the stack it resolves against
//! the boot classloader and cannot see app classes. Cache global refs from a
//! JNI callback instead.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use jni::objects::{GlobalRef, JClass, JString};
use jni::JNIEnv;

use crate::platform::Insets;
use crate::platform::RemoteKey;

/// `GameActivity` as a global reference, cached on the first JNI callback.
///
/// This MUST NOT be obtained with `FindClass` from `set_input_type`: that runs
/// on the `android_main` thread, which has no Java frames, so `FindClass`
/// resolves against the boot classloader and throws
/// `ClassNotFoundException: com.tarik.dadoplayer.GameActivity`. Worse, a pending
/// exception poisons the thread — the *next* JNI call aborts the process under
/// CheckJNI (debug builds). `nativeOnInsets` runs at startup from a real Java
/// frame, so its `JClass` argument already carries the app classloader.
static ACTIVITY_CLASS: OnceLock<GlobalRef> = OnceLock::new();

pub fn player_brightness(requested: f32) -> Option<f32> {
    let app = super::entry::app_handle()?;
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
    let mut env = vm.get_env().ok()?;
    let class = ACTIVITY_CLASS.get()?;
    match env.call_static_method(
        class,
        "playerBrightness",
        "(F)F",
        &[jni::objects::JValue::Float(requested)],
    ) {
        Ok(value) => value.f().ok(),
        Err(_) => {
            let _ = env.exception_clear();
            None
        }
    }
}

static SYS: [AtomicI32; 4] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];
static CUT: [AtomicI32; 4] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];
static GEST: [AtomicI32; 4] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];
static IME_BOTTOM: AtomicI32 = AtomicI32::new(0);
/// Largest IME inset seen during the current keyboard session.
///
/// Android animates the keyboard in over ~15 `onApplyWindowInsets` callbacks
/// (`ime.bottom` walks 0 → 1 → 60 → ... → 883). Feeding that raw value into
/// panel heights resizes the whole login form on *every* animation frame;
/// combined with `scroll_to_rect` the form visibly bounces up and down while
/// the first characters are typed. Layout therefore consumes a monotonic
/// value and only drops back to zero once the IME has really been dismissed.
static IME_LATCH: AtomicI32 = AtomicI32::new(0);

static CAN_BACK: AtomicBool = AtomicBool::new(false);
static BACK_REQUESTED: AtomicBool = AtomicBool::new(false);
static KEYBOARD: AtomicBool = AtomicBool::new(false);
/// Consecutive frames asking to hide the keyboard. See `set_keyboard`.
static HIDE_FRAMES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// `UiModeManager.getCurrentModeType() == UI_MODE_TYPE_TELEVISION`, pushed once
/// from `GameActivity.onCreate`. Rust cannot query that service itself: it needs
/// a Java `Context`, and there is no winit/android-activity wrapper for it.
static TV_DEVICE: AtomicBool = AtomicBool::new(false);

pub fn tv_device() -> bool {
    TV_DEVICE.load(Ordering::Relaxed)
}

/// Last `InputType` we pushed to Java (`-1` = never pushed).
static INPUT_TYPE: AtomicI32 = AtomicI32::new(-1);

/// Remote / D-pad keys queued by `nativeOnKey` (Java UI thread) and drained
/// once per frame on the `android_main` thread. Bounded at 32 entries so a
/// held key cannot grow it without bound.
static KEY_QUEUE: Mutex<VecDeque<RemoteKey>> = Mutex::new(VecDeque::new());

/// The app's private files directory, pushed from `GameActivity.onCreate`.
static STORAGE_DIR: OnceLock<String> = OnceLock::new();

/// Handle to the egui context, so a JNI callback running on the Java UI thread
/// can wake the render loop.
///
/// This is not a convenience: egui only repaints when something asks it to, and
/// an idle screen (the login form, a list nobody is touching) schedules no
/// frame at all. `nativeRequestBack` therefore used to set [`BACK_REQUESTED`]
/// and have it sit unread until the next unrelated touch — the profile-delete
/// dialog stayed on screen because nothing was ever going to look at the flag.
/// `egui::Context` is `Send + Sync` and `request_repaint()` is documented as
/// callable from any thread, which is exactly what this is for.
static UI_CTX: OnceLock<egui::Context> = OnceLock::new();

/// Called once per frame from `App::update`; only the first call does anything.
pub fn bind_context(ctx: &egui::Context) {
    let _ = UI_CTX.set(ctx.clone());
}

/// Wake the render loop from a JNI callback thread.
///
/// Public because mpv's render-update callback needs it too: mpv produces
/// frames on its own thread, and egui only paints when something asks. Without
/// this the first frame is drawn and every later one is never requested — a
/// still picture under a live UI.
pub fn wake_ui() {
    if let Some(c) = UI_CTX.get() {
        c.request_repaint();
    }
}

/// The process's `JavaVM`, captured from the first JNI callback we get.
///
/// `JNIEnv::get_java_vm()` on an existing callback is used rather than
/// `AndroidApp::vm_as_ptr()`: it needs no new plumbing and does not depend on
/// android-activity's internals. `nativeOnStorage` runs from `onCreate`, before
/// any playback can start, so the value is there by the time mpv needs it.
static JAVA_VM: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

pub fn java_vm() -> Option<*mut std::ffi::c_void> {
    let p = JAVA_VM.load(Ordering::SeqCst);
    if p.is_null() {
        None
    } else {
        Some(p)
    }
}

fn capture_vm(env: &mut JNIEnv) {
    if !JAVA_VM.load(Ordering::Relaxed).is_null() {
        return;
    }
    if let Ok(vm) = env.get_java_vm() {
        JAVA_VM.store(
            vm.get_java_vm_pointer() as *mut std::ffi::c_void,
            Ordering::SeqCst,
        );
    }
}

/// Last IME buffer we have seen (used to diff composing text).
static IME_TEXT: Mutex<Option<String>> = Mutex::new(None);
static IME_PREV: Mutex<String> = Mutex::new(String::new());

/// Monotonic IME height for layout. See [`IME_LATCH`].
fn layout_ime() -> f32 {
    let raw = IME_BOTTOM.load(Ordering::Relaxed);
    if raw > 0 {
        let prev = IME_LATCH.load(Ordering::Relaxed);
        if raw > prev {
            IME_LATCH.store(raw, Ordering::Relaxed);
            return raw as f32;
        }
        return prev as f32;
    }
    if !KEYBOARD.load(Ordering::Relaxed) {
        // Keyboard is gone for good: start the next session from a clean slate.
        IME_LATCH.store(0, Ordering::Relaxed);
        return 0.0;
    }
    // Key pressed but no inset has arrived yet (the show animation has not
    // started). Keep whatever we had so the panel does not collapse for a
    // frame in the middle of a session.
    IME_LATCH.load(Ordering::Relaxed) as f32
}

pub fn current() -> Insets {
    let v = |a: &[AtomicI32; 4], i: usize| a[i].load(Ordering::Relaxed) as f32;
    let ime = layout_ime();
    Insets {
        top: v(&SYS, 0).max(v(&CUT, 0)),
        // `systemGestures` is the touch-interception region for the back
        // gesture, not space the system reserves for chrome. Only its *bottom*
        // edge approximates something visible (the gesture handle), so it is
        // the only edge that may contribute. Feeding it into left/right cost
        // 29.7 dp per side on a 411 dp phone - 15.5% of the width - for nothing
        // visible. Measured on dado_test: gest.left/right = 78 px = 29.7 dp
        // while sys and cutout were both 0.
        bottom: v(&SYS, 1).max(v(&CUT, 1)).max(v(&GEST, 1)).max(ime),
        left: v(&SYS, 2).max(v(&CUT, 2)),
        right: v(&SYS, 3).max(v(&CUT, 3)),
        ime,
    }
}

/// Complete IME snapshot. Android provides the field buffer, so replay the
/// complete value rather than guessing a cursor-local edit from a prefix diff.
pub type Edit = String;

/// Take the newest complete text snapshot from Android's input connection.
pub fn take_edit() -> Option<Edit> {
    let new = IME_TEXT.lock().ok()?.take()?;
    let mut prev = IME_PREV.lock().ok()?;
    if *prev == new {
        return None;
    }
    *prev = new.clone();
    Some(new)
}

/// Forget the IME buffer (called when the keyboard closes).
///
/// Both halves matter: [`IME_PREV`] is the diff baseline and [`IME_TEXT`] is a
/// queued, not-yet-replayed snapshot. Leaving a pending snapshot around after
/// a reset means the next `take_edit()` applies the OLD field's text to the
/// NEW field.
pub fn reset_ime() {
    if let Ok(mut slot) = IME_TEXT.lock() {
        *slot = None;
    }
    if let Ok(mut p) = IME_PREV.lock() {
        p.clear();
    }
}

/// Show/hide the soft keyboard. Only acts on a change.
///
/// Hiding is debounced by a few frames. `show` is derived from a per-frame
/// egui request, so a single frame that did not paint the focused `TextEdit`
/// used to call `hide_soft_input()` + `reset_ime()` while the user was still
/// typing. Besides the visible flicker that desynchronises the IME buffer from
/// the egui field: the next keystroke is then replayed against an empty
/// baseline, duplicating or dropping text. Showing remains immediate so the
/// keyboard appears on the first tap.
pub fn set_keyboard(show: bool) {
    const HIDE_AFTER_FRAMES: u32 = 3;
    if show {
        HIDE_FRAMES.store(0, Ordering::Relaxed);
        if KEYBOARD.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(app) = super::entry::app_handle() {
            app.show_soft_input(false);
        }
        return;
    }
    if !KEYBOARD.load(Ordering::Relaxed) {
        return;
    }
    if HIDE_FRAMES.fetch_add(1, Ordering::Relaxed) + 1 < HIDE_AFTER_FRAMES {
        return;
    }
    HIDE_FRAMES.store(0, Ordering::Relaxed);
    if !KEYBOARD.swap(false, Ordering::Relaxed) {
        return;
    }
    if let Some(app) = super::entry::app_handle() {
        app.hide_soft_input(false);
    }
    reset_ime();
}

/// Start a fresh IME session for the text field that just gained focus.
///
/// `set_text_input_state` alone was not enough. It updates the NATIVE
/// GameTextInput state and our Rust diff baseline, but Gboard keeps its own
/// composing buffer and suggestion history until the input connection is
/// restarted. Measured on the emulator, switching from "Benutzer" (`probe`)
/// to an empty "Passwort" and typing `probe` produced TEN masked characters
/// and the suggestion `probeprobe`: the keyboard appended to the previous
/// field's text and reported that as the new full snapshot. Because the Rust
/// baseline had already been re-seeded to `""`, the whole stale prefix was
/// replayed as fresh input.
///
/// The Java `seedIme` hook clears/refills the connection's `Editable` and
/// calls `InputMethodManager.restartInput`, so the keyboard re-queries the
/// real text. It runs on the UI thread in one runnable; both the native state
/// command and the Java runnable write the SAME text, so their relative order
/// is irrelevant.
pub fn seed(text: &str) {
    let Some(app) = super::entry::app_handle() else {
        return;
    };
    let len = text.chars().count();
    app.set_text_input_state(android_activity::input::TextInputState {
        text: text.to_string(),
        selection: android_activity::input::TextSpan {
            start: len,
            end: len,
        },
        compose_region: None,
    });

    if let Some(class) = ACTIVITY_CLASS.get() {
        // `get_env` only reads this thread's env (no attach/detach), and
        // `JavaVM` has no destructor in jni 0.21.
        let Ok(vm) = (unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) })
        else {
            return;
        };
        let Ok(mut env) = vm.get_env() else {
            return;
        };
        match env.new_string(text) {
            Ok(s) => {
                let obj: jni::objects::JObject = s.into();
                if env
                    .call_static_method(
                        class,
                        "seedIme",
                        "(Ljava/lang/String;)V",
                        &[jni::objects::JValue::Object(&obj)],
                    )
                    .is_err()
                {
                    // A pending exception aborts the process at the NEXT JNI
                    // call under CheckJNI. Never leave one behind.
                    let _ = env.exception_clear();
                }
            }
            Err(_) => {
                let _ = env.exception_clear();
            }
        }
    }

    // A snapshot queued by the previous field before the tap must not be
    // replayed into this one. Then re-base the diff on the new field's text.
    if let Ok(mut slot) = IME_TEXT.lock() {
        *slot = None;
    }
    if let Ok(mut p) = IME_PREV.lock() {
        *p = text.to_string();
    }
}

/// Tell Java which keyboard the focused field wants. Only acts on a change.
///
/// `GameActivity`'s `EditorInfo` is created once and shared by reference with
/// the input connection, so writing to it takes effect on the next
/// `onCreateInputConnection`. There is no other hook: the surface view that
/// actually creates the connection is package-private.
pub fn set_input_type(input_type: i32) {
    if INPUT_TYPE.swap(input_type, Ordering::Relaxed) == input_type {
        return;
    }
    let Some(app) = super::entry::app_handle() else {
        return;
    };
    // `get_env` only reads the current thread's env (no attach, no detach), and
    // `JavaVM` has no destructor in jni 0.21 — this cannot tear down the VM.
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) })
    else {
        return;
    };
    let Ok(mut env) = vm.get_env() else {
        return;
    };
    // Cached at startup by `nativeOnInsets`. Empty only if no inset pass has
    // happened yet — then the field keeps Java's default keyboard type, which is
    // the same behaviour a release build already has (CheckJNI off, no abort).
    // Degrading to the status quo is deliberate; crashing is not.
    let Some(class) = ACTIVITY_CLASS.get() else {
        return;
    };
    // `&GlobalRef` implements `Desc<JClass>`, so no local ref is needed here.
    if env
        .call_static_method(
            class,
            "setImeFields",
            "(I)V",
            &[jni::objects::JValue::Int(input_type)],
        )
        .is_err()
    {
        // Never leave an exception pending: on the `android_main` thread the
        // next JNI call would abort the process instead of returning an error.
        let _ = env.exception_clear();
    }
}

/// Tell Java whether the UI can consume a back press.
pub fn set_can_back(v: bool) {
    CAN_BACK.store(v, Ordering::Relaxed);
}

pub fn take_back() -> bool {
    BACK_REQUESTED.swap(false, Ordering::Relaxed)
}

/// Drain all remote keys queued since the last frame. A poisoned mutex yields
/// an empty Vec rather than panicking on the render thread.
pub fn drain_keys() -> Vec<RemoteKey> {
    match KEY_QUEUE.lock() {
        Ok(mut q) => q.drain(..).collect(),
        Err(_) => Vec::new(),
    }
}

/// Point the `directories` crate at the app's private files directory.
///
/// **Both variables, not just `XDG_CONFIG_HOME`.** `directories` 5.0.1 compiles
/// its Linux branch on Android, and that branch gates the entire directory
/// computation behind a home directory:
///
/// ```ignore
/// // directories-5.0.1/src/lin.rs
/// pub fn base_dirs() -> Option<BaseDirs> {
///     if let Some(home_dir) = dirs_sys::home_dir() {
///         let config_dir = env::var_os("XDG_CONFIG_HOME")
///             .and_then(dirs_sys::is_absolute_path)
///             .unwrap_or_else(|| home_dir.join(".config"));
/// ```
///
/// and `dirs-sys` hard-codes `home_dir()` to `None` on Android:
///
/// ```ignore
/// #[cfg(any(target_os = "android", target_os = "ios", target_os = "emscripten"))]
/// unsafe fn fallback() -> Option<OsString> { None }
/// ```
///
/// So `XDG_CONFIG_HOME` alone is read *inside* a branch that never runs. That
/// is a fix which changes nothing, produces no error and logs nothing —
/// `HOME` is what gets past the `if let`, and the XDG variable then decides the
/// exact directory (`filesDir/dado-player`, since `ProjectDirs` appends its
/// project path).
///
/// Idempotent: writing the same two values twice is a no-op.
pub fn apply_storage_env(dir: &str) {
    // Edition 2021, so `set_var` is safe here (it only became `unsafe` in 2024).
    std::env::set_var("HOME", dir);
    std::env::set_var("XDG_CONFIG_HOME", dir);
}

/// The files directory pushed from Java, if the callback has already run.
pub fn storage_dir() -> Option<&'static str> {
    STORAGE_DIR.get().map(String::as_str)
}

// ---------------------------------------------------------------- exports ---

#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeOnImeText(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
    _sel_start: jni::sys::jint,
    _sel_end: jni::sys::jint,
    _comp_start: jni::sys::jint,
    _comp_end: jni::sys::jint,
    dismissed: jni::sys::jboolean,
) {
    // Never fall back to "": a failed read would look like the user cleared the
    // field and we would backspace the whole text out of it.
    let Ok(raw) = env.get_string(&text) else {
        return;
    };
    let s: String = raw.into();
    if dismissed != 0 {
        if let Ok(mut p) = IME_PREV.lock() {
            p.clear();
        }
    }
    if let Ok(mut slot) = IME_TEXT.lock() {
        *slot = Some(s);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeOnInsets(
    mut env: JNIEnv,
    class: JClass,
    sys_top: jni::sys::jint,
    sys_bottom: jni::sys::jint,
    sys_left: jni::sys::jint,
    sys_right: jni::sys::jint,
    cut_top: jni::sys::jint,
    cut_bottom: jni::sys::jint,
    cut_left: jni::sys::jint,
    cut_right: jni::sys::jint,
    gest_top: jni::sys::jint,
    gest_bottom: jni::sys::jint,
    gest_left: jni::sys::jint,
    gest_right: jni::sys::jint,
    ime_bottom: jni::sys::jint,
) {
    let sys = [sys_top, sys_bottom, sys_left, sys_right];
    let cut = [cut_top, cut_bottom, cut_left, cut_right];
    let gest = [gest_top, gest_bottom, gest_left, gest_right];

    // Second chance at the VM, in case the storage callback ever lands after
    // the first insets pass. Both are no-ops once the value is stored.
    capture_vm(&mut env);

    // Cache the activity class while a Java frame is on the stack, so that
    // `set_input_type` never has to call `FindClass` from `android_main`.
    if ACTIVITY_CLASS.get().is_none() {
        if let Ok(g) = env.new_global_ref(class) {
            let _ = ACTIVITY_CLASS.set(g);
        }
    }

    for i in 0..4 {
        SYS[i].store(sys[i], Ordering::Relaxed);
        CUT[i].store(cut[i], Ordering::Relaxed);
        GEST[i].store(gest[i], Ordering::Relaxed);
    }
    IME_BOTTOM.store(ime_bottom, Ordering::Relaxed);
}

#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeOnStorage(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) {
    // Earliest reliable point to grab the VM: `onCreate`, before any playback.
    capture_vm(&mut env);
    let Ok(raw) = env.get_string(&path) else {
        // Never leave an exception pending on a JNI callback thread.
        let _ = env.exception_clear();
        return;
    };
    let dir: String = raw.into();
    // Applied here rather than only from `android_main` because this runs from
    // `onCreate`, which is strictly earlier than any Rust `store` call — Rust
    // code cannot run before `android_main`. `android_main` re-applies it as a
    // fallback in case this callback ever lands late; see `entry.rs`.
    apply_storage_env(&dir);
    let _ = STORAGE_DIR.set(dir);
}

#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeCanGoBack(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jboolean {
    CAN_BACK.load(Ordering::Relaxed) as jni::sys::jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeRequestBack(
    _env: JNIEnv,
    _class: JClass,
) {
    BACK_REQUESTED.store(true, Ordering::Relaxed);
    // Without this the flag waits for a frame that an idle UI never schedules.
    wake_ui();
}

/// Forward a TV remote / D-pad key from Java into the per-frame queue.
///
/// This callback runs on the Java UI thread. It deliberately touches NO
/// `JNIEnv` method: no `FindClass`, no call that can throw, no pending
/// exception left behind. It only manipulates Rust-side statics and wakes the
/// render loop, which is safe from any thread.
#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeOnKey(
    _env: JNIEnv,
    _class: JClass,
    keycode: jni::sys::jint,
    action: jni::sys::jint,
) {
    // ACTION_DOWN == 0. Ignore ACTION_UP (1) and anything else. Key repeats
    // arrive as further ACTION_DOWN events; that is intentional and gives
    // smooth list scrolling.
    if action != 0 {
        return;
    }
    let key = match keycode {
        19 => RemoteKey::Up,    // DPAD_UP
        20 => RemoteKey::Down,  // DPAD_DOWN
        21 => RemoteKey::Left,  // DPAD_LEFT
        22 => RemoteKey::Right, // DPAD_RIGHT
        // Keep this table in sync with the switch in GameActivity.dispatchKeyEvent:
        // every keycode Java consumes must land here, otherwise the key is
        // swallowed and a remote user sees nothing happen.
        23 | 66 | 160 => RemoteKey::Ok, // DPAD_CENTER / ENTER / NUMPAD_ENTER
        7 | 144 => RemoteKey::Recall,   // KEYCODE_0 / KEYCODE_NUMPAD_0
        85 | 126 | 127 => RemoteKey::PlayPause, // MEDIA_PLAY_PAUSE / PLAY / PAUSE
        183 => RemoteKey::Red,
        184 => RemoteKey::Green,
        185 => RemoteKey::Yellow,
        186 => RemoteKey::Blue,
        // BACK is not forwarded by Java today: it goes through the existing
        // onBackPressed() -> nativeRequestBack() -> App::mobile_back() chain
        // instead, so popping a second time here would fire twice. The mapping
        // stays for completeness/future use; focus::begin only marks remote
        // focus active on Back, it never pops.
        4 => RemoteKey::Back,
        _ => return,
    };
    if let Ok(mut q) = KEY_QUEUE.lock() {
        if q.len() >= 32 {
            q.pop_front(); // drop the oldest so a held key cannot grow the queue
        }
        q.push_back(key);
    }
    // An idle egui loop schedules no frame on its own; ask for a repaint.
    wake_ui();
}

/// Device form factor, pushed from Java once at startup. Plain `jboolean`, so
/// like `nativeOnKey` this callback needs no `JNIEnv` method and cannot leave a
/// pending exception behind.
#[no_mangle]
pub extern "system" fn Java_com_tarik_dadoplayer_GameActivity_nativeOnTvDevice(
    _env: JNIEnv,
    _class: JClass,
    is_tv: jni::sys::jboolean,
) {
    TV_DEVICE.store(is_tv != 0, Ordering::Relaxed);
}

pub fn copy_device_address(address: &str) -> anyhow::Result<()> {
    use anyhow::{ensure, Context};
    let app = super::entry::app_handle().context("Android-Activity fehlt")?;
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
    let mut env = vm.attach_current_thread()?;
    let class = ACTIVITY_CLASS.get().context("Activity-Klasse fehlt")?;
    let result = env.with_local_frame(2, |env| -> jni::errors::Result<bool> {
        let value = jni::objects::JObject::from(env.new_string(address)?);
        env.call_static_method(
            class,
            "copyDeviceAddress",
            "(Ljava/lang/String;)Z",
            &[jni::objects::JValue::Object(&value)],
        )?
        .z()
    });
    if result.is_err() {
        let _ = env.exception_clear();
    }
    ensure!(result?, "Zwischenablage nicht verfügbar");
    Ok(())
}

pub fn mobile_update(action: i32) -> anyhow::Result<()> {
    use anyhow::{ensure, Context};
    let app = super::entry::app_handle().context("Android-Activity fehlt")?;
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
    let mut env = vm.attach_current_thread()?;
    let class = ACTIVITY_CLASS.get().context("Activity-Klasse fehlt")?;
    let result = env.call_static_method(
        class,
        "mobileUpdate",
        "(I)I",
        &[jni::objects::JValue::Int(action)],
    );
    if result.is_err() {
        let _ = env.exception_clear();
    }
    ensure!(
        result?.i()? == 1,
        "Paket konnte nicht an Android übergeben werden"
    );
    Ok(())
}

pub fn mobile_update_status() -> anyhow::Result<String> {
    use anyhow::Context;
    let app = super::entry::app_handle().context("Android-Activity fehlt")?;
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
    let mut env = vm.attach_current_thread()?;
    let class = ACTIVITY_CLASS.get().context("Activity-Klasse fehlt")?;
    let status = env.call_static_method(class, "mobileUpdateStatus", "()Ljava/lang/String;", &[]);
    if status.is_err() {
        let _ = env.exception_clear();
    }
    let status = JString::from(status?.l()?);
    let value = env.get_string(&status)?.into();
    env.delete_local_ref(status)?;
    Ok(value)
}

pub(crate) fn prepare_vlc() -> Result<(), String> {
    fn prepare() -> anyhow::Result<()> {
        use anyhow::{ensure, Context};
        let app = super::entry::app_handle().context("Android-Activity fehlt")?;
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
        let mut env = vm.attach_current_thread()?;
        let class = ACTIVITY_CLASS.get().context("Activity-Klasse fehlt")?;
        let result = env.call_static_method(class, "prepareEmbeddedVlc", "()Z", &[]);
        if result.is_err() {
            let _ = env.exception_clear();
        }
        ensure!(result?.z()?, "LibVLC konnte nicht geladen werden");
        Ok(())
    }
    prepare().map_err(|err| format!("VLC: {err}"))
}
