//! libmpv's OpenGL render API, drawn inside egui's frame.
//!
//! Why this module exists at all: libmpv cannot own the screen on Android. The
//! two ways that look like they should work both fail, and both were tried in
//! the emulator on 2026-11-03 (see MOBILE-BAUPLAN.md §17):
//!
//! 1. `wid` + `gpu-context=android` — mpv takes over the EGL surface that
//!    egui/glow already owns. The UI goes permanently black.
//! 2. `android-surface` — not an option in this libmpv build at all. The old
//!    code invented it and always failed with "option not found".
//!
//! The supported answer is the render API: mpv decodes and converts into a
//! texture inside *our* GL context, and egui composites it as one more shape.
//! Nothing takes over the surface, so the chrome keeps drawing on top.
//!
//! ## The JavaVM, and why we load a library we never call
//!
//! mpv's Android video path needs a registered `JavaVM` for MediaCodec. It gets
//! one through `av_jni_set_java_vm`, which is exported by **libavcodec.so**,
//! not by libmpv.so. Loading that library to call one function looks wrong
//! until you look at the link graph: `libmpv.so` carries a `DT_NEEDED` entry
//! naming `libavcodec.so`, so `dlopen` returns the instance mpv is already
//! using rather than a second copy. That is the whole reason this works. If
//! mpv had statically linked ffmpeg, this call would configure a decoder that
//! nothing ever uses and hwdec would fail with no diagnostic.
//!
//! ## Two ordering rules from libmpv/render.h
//!
//! 1. "The renderer needs to be created with mpv_render_context_create()
//!    before you start playback (or otherwise cause a VO to be created)."
//!    Creating it needs a current GL context, which in this app only exists
//!    inside the paint callback. So `loadfile` is queued here and released by
//!    `App::update()` once `READY` is set, never issued directly from `play()`.
//!    mpv says video init will otherwise "revert to a VO that creates its own
//!    window" — and this build has no window to create.
//! 2. "This function implicitly pulls a video frame from the internal queue
//!    and renders it. If no new frame is available, the previous frame is
//!    redrawn." egui clears the framebuffer on every frame, so gating
//!    render() on MPV_RENDER_UPDATE_FRAME leaves a black rectangle on every
//!    repaint that is not a new video frame. We render on every paint instead.
//!    `video-timing-offset=0` (set in mpv.rs) keeps that call from blocking
//!    the UI thread until the frame's display time.
//!
//! ## Why the FBO is the whole screen
//!
//! libmpv/render_gl.h on `mpv_opengl_fbo.w/h`: "This must refer to the size of
//! the framebuffer. This must always be set." With FBO 0 that is the entire
//! default framebuffer, not the callback rectangle. mpv derives its own GL
//! viewport from those dimensions, so trying to render into a sub-rect by
//! passing the callback size would place the picture in the wrong corner.
//! We give it the full framebuffer, let mpv letterbox to the video's aspect
//! ratio, and let egui draw the translucent chrome on top.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use egui_glow::glow::{self, HasContext as _};
use libloading::Library;

// ---------------------------------------------------------------- FFI ----

#[repr(C)]
struct OpenglInitParams {
    get_proc_address: Option<unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void>,
    get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
struct OpenglFbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    internal_format: c_int,
}

/// `mpv_render_param`. The array handed to mpv **must** end with
/// `{ type: 0, data: null }`: mpv reads until it sees `MPV_RENDER_PARAM_INVALID`
/// and, without the terminator, walks off the end of the array and returns
/// garbage instead of an error. The failure mode is a black rectangle with no
/// diagnostic, which is why the terminator is written out explicitly in both
/// places below rather than left implicit.
#[repr(C)]
struct RenderParam {
    type_: c_int,
    data: *mut c_void,
}

const PARAM_INVALID: c_int = 0;
const PARAM_API_TYPE: c_int = 1;
const PARAM_OPENGL_INIT_PARAMS: c_int = 2;
const PARAM_OPENGL_FBO: c_int = 3;
const PARAM_FLIP_Y: c_int = 4;

// `MPV_RENDER_UPDATE_FRAME` (1 << 0) is deliberately NOT used as a gate here.
// See the module comment: egui clears the framebuffer on every paint, and
// mpv_render_context_render() redraws the last frame when no new one exists.
// There is also no `MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME` entry: that enum
// value is an ABI detail we would have to pin to the exact libmpv build, while
// the documented global option `video-timing-offset=0` achieves the same
// "never block the UI thread" result without it.

type CreateFn = unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *mut RenderParam) -> c_int;
type SetUpdateFn =
    unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn(*mut c_void)>, *mut c_void);
type RenderFn = unsafe extern "C" fn(*mut c_void, *mut RenderParam) -> c_int;
type UpdateFn = unsafe extern "C" fn(*mut c_void) -> u64;
type FreeFn = unsafe extern "C" fn(*mut c_void);

struct Fns {
    create: CreateFn,
    set_update: SetUpdateFn,
    render: RenderFn,
    update: UpdateFn,
    free: FreeFn,
}

struct Ctx {
    ctx: *mut c_void,
    fns: Fns,
    /// Kept alive for the lifetime of the render context: dropping it would
    /// `dlclose` libmpv underneath a live mpv handle.
    _lib: Library,
}

// The context is only ever touched from inside the paint callback, and the
// mutex below serialises access. mpv itself is thread-safe.
unsafe impl Send for Ctx {}

static CTX: Mutex<Option<Ctx>> = Mutex::new(None);
/// The `mpv_handle` the render context should attach to, or null.
static SOURCE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
/// Set by `detach()`: the next paint callback frees the render context, because
/// that is the only place with the GL context current.
static FREE_PENDING: AtomicBool = AtomicBool::new(false);
/// `mpv_render_context_create()` has succeeded. Nothing may call `loadfile`
/// before this is true — see the module comment, rule 1.
static READY: AtomicBool = AtomicBool::new(false);
/// A `loadfile` waiting for the render context: URL plus resume position.
static PENDING: Mutex<Option<PendingLoad>> = Mutex::new(None);

pub fn flush_android_load() {
    take_ready_load();
}
/// Last error, shown in the player instead of a silent black rectangle.
static LAST_ERR: Mutex<Option<String>> = Mutex::new(None);

struct PendingLoad {
    url: String,
    resume: f64,
}

fn set_err(e: String) {
    eprintln!("[render] {}", e);
    if let Ok(mut slot) = LAST_ERR.lock() {
        *slot = Some(e);
    }
}

/// The most recent render error, if any. The player shows it rather than
/// pretending the stream is loading. Cleared by the next successful render.
pub fn last_error() -> Option<String> {
    LAST_ERR.lock().ok().and_then(|s| s.clone())
}

// ------------------------------------------------------------ lifecycle ----

/// Point the renderer at an initialised `mpv_handle`.
///
/// Called from `Mpv::new` on Android after `mpv_initialize`. The render context
/// itself is *not* created here: `mpv_render_context_create` needs a current GL
/// context, and the only place that is guaranteed is inside the paint callback.
pub fn attach(mpv: *mut c_void) {
    SOURCE.store(mpv, Ordering::SeqCst);
    READY.store(false, Ordering::SeqCst);
    if let Ok(mut p) = PENDING.lock() {
        *p = None;
    }
}

/// Detach and schedule the render context for destruction.
///
/// mpv requires the render context to be freed *before* the mpv handle is
/// destroyed, and it must be freed with the GL context current. Both of those
/// point at the same place: the next paint callback. So this only marks the
/// work and asks for a frame — freeing here, on the UI thread, would be a
/// use-after-free of the GL context.
pub fn detach() {
    if SOURCE.swap(ptr::null_mut(), Ordering::SeqCst).is_null() {
        return;
    }
    READY.store(false, Ordering::SeqCst);
    if let Ok(mut p) = PENDING.lock() {
        *p = None;
    }
    FREE_PENDING.store(true, Ordering::SeqCst);
    super::jni_bridge::wake_ui();
}

pub fn is_attached() -> bool {
    !SOURCE.load(Ordering::SeqCst).is_null()
}

/// True once the render context exists. `App::update()` uses this to know when
/// it is legal to issue `loadfile`.
pub fn is_ready() -> bool {
    READY.load(Ordering::SeqCst)
}

/// Queue a `loadfile` until the render context exists.
///
/// Replaces any earlier queued load, which also makes replaying the same URL
/// work: every `request_load` is a new request, not a comparison against the
/// last loaded URL.
pub fn request_load(url: &str, resume: f64) {
    if let Ok(mut p) = PENDING.lock() {
        *p = Some(PendingLoad {
            url: url.to_string(),
            resume,
        });
    }
    super::jni_bridge::wake_ui();
}

/// Drop a queued load (leaving the player, changing settings).
pub fn cancel_load() {
    if let Ok(mut p) = PENDING.lock() {
        *p = None;
    }
}

/// Take the queued load, but only once the render context exists.
pub fn take_ready_load() -> Option<(String, f64)> {
    if !READY.load(Ordering::SeqCst) {
        return None;
    }
    PENDING
        .lock()
        .ok()
        .and_then(|mut p| p.take())
        .map(|p| (p.url, p.resume))
}

// ------------------------------------------------------------- javavm ----

static VM_DONE: AtomicBool = AtomicBool::new(false);
static AVCODEC: OnceLock<Library> = OnceLock::new();

/// Register the process's `JavaVM` with the ffmpeg that mpv is linked against.
///
/// Idempotent and retryable: it is called right before `mpv_initialize`, and if
/// the VM has not been captured yet it returns an error rather than latching a
/// failure, so the next playback attempt tries again.
pub fn ensure_java_vm() -> Result<(), String> {
    if VM_DONE.load(Ordering::SeqCst) {
        return Ok(());
    }
    let vm = super::jni_bridge::java_vm().ok_or_else(|| {
        "JavaVM noch nicht erfasst (nativeOnStorage ist nicht gelaufen)".to_string()
    })?;
    // `av_jni_set_java_vm` lives in libavcodec.so, not in libmpv.so. libmpv
    // carries `DT_NEEDED libavcodec.so`, so this dlopen resolves to the same
    // instance mpv decodes with — see the module comment.
    let lib = AVCODEC.get_or_init(|| {
        unsafe { Library::new("libavcodec.so") }.expect("libavcodec.so fehlt in der APK")
    });
    unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> = lib
            .get(b"av_jni_set_java_vm\0")
            .map_err(|e| format!("av_jni_set_java_vm: {e}"))?;
        let rc = f(vm);
        if rc != 0 {
            return Err(format!("av_jni_set_java_vm: {}", rc));
        }
    }
    VM_DONE.store(true, Ordering::SeqCst);
    Ok(())
}

// ------------------------------------------------------ gl proc lookup ----

static EGL: OnceLock<Option<Library>> = OnceLock::new();
static GLES: OnceLock<Option<Library>> = OnceLock::new();

/// `get_proc_address` for `mpv_opengl_init_params`.
///
/// mpv asks for its GL entry points by name. glow does not expose its loader,
/// so this goes to `eglGetProcAddress` first — on Android it resolves both core
/// GLES and extension functions — and falls back to `dlsym` on libGLESv2 for
/// the handful of core symbols some drivers refuse to hand out through EGL.
unsafe extern "C" fn get_proc_address(_ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }
    let cname = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    if cname.is_empty() {
        return ptr::null_mut();
    }

    if let Some(lib) = EGL
        .get_or_init(|| unsafe { Library::new("libEGL.so") }.ok())
        .as_ref()
    {
        if let Ok(f) = unsafe {
            lib.get::<unsafe extern "C" fn(*const c_char) -> *mut c_void>(b"eglGetProcAddress\0")
        } {
            let p = unsafe { f(name) };
            if !p.is_null() {
                return p;
            }
        }
    }

    if let Some(lib) = GLES
        .get_or_init(|| unsafe { Library::new("libGLESv2.so") }.ok())
        .as_ref()
    {
        let sym = CString::new(cname).ok();
        if let Some(sym) = sym {
            if let Ok(p) = unsafe { lib.get::<unsafe extern "C" fn()>(sym.as_bytes_with_nul()) } {
                return *p as *mut c_void;
            }
        }
    }
    ptr::null_mut()
}

// ------------------------------------------------------------ painting ----

/// mpv has a new frame: ask egui for one too.
///
/// This runs on mpv's thread, not the UI thread. `request_repaint` is the one
/// egui call that is safe from any thread, and `wake_ui` wraps it together with
/// the `egui::Context` that actually paints.
unsafe extern "C" fn on_update(_cb_ctx: *mut c_void) {
    super::jni_bridge::wake_ui();
}

fn load_fns() -> Result<(Library, Fns), String> {
    let lib = unsafe { Library::new("libmpv.so") }.map_err(|e| format!("libmpv.so: {e}"))?;
    unsafe {
        let create = *lib
            .get::<CreateFn>(b"mpv_render_context_create\0")
            .map_err(|e| format!("mpv_render_context_create: {e}"))?;
        let set_update = *lib
            .get::<SetUpdateFn>(b"mpv_render_context_set_update_callback\0")
            .map_err(|e| format!("mpv_render_context_set_update_callback: {e}"))?;
        let render = *lib
            .get::<RenderFn>(b"mpv_render_context_render\0")
            .map_err(|e| format!("mpv_render_context_render: {e}"))?;
        let update = *lib
            .get::<UpdateFn>(b"mpv_render_context_update\0")
            .map_err(|e| format!("mpv_render_context_update: {e}"))?;
        let free = *lib
            .get::<FreeFn>(b"mpv_render_context_free\0")
            .map_err(|e| format!("mpv_render_context_free: {e}"))?;
        Ok((
            lib,
            Fns {
                create,
                set_update,
                render,
                update,
                free,
            },
        ))
    }
}

fn create(mpv: *mut c_void) -> Result<Ctx, String> {
    let (lib, fns) = load_fns()?;
    let api = CString::new("opengl").unwrap();
    let mut init = OpenglInitParams {
        get_proc_address: Some(get_proc_address),
        get_proc_address_ctx: ptr::null_mut(),
    };
    let mut params = [
        RenderParam {
            type_: PARAM_API_TYPE,
            data: api.as_ptr() as *mut c_void,
        },
        RenderParam {
            type_: PARAM_OPENGL_INIT_PARAMS,
            data: &mut init as *mut _ as *mut c_void,
        },
        // Terminator. Without it mpv reads past the array.
        RenderParam {
            type_: PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];
    let mut ctx: *mut c_void = ptr::null_mut();
    let rc = unsafe { (fns.create)(&mut ctx, mpv, params.as_mut_ptr()) };
    if rc < 0 {
        return Err(format!("mpv_render_context_create: {}", rc));
    }
    if ctx.is_null() {
        return Err("mpv_render_context_create: null".to_string());
    }
    unsafe { (fns.set_update)(ctx, Some(on_update), ptr::null_mut()) };
    Ok(Ctx {
        ctx,
        fns,
        _lib: lib,
    })
}

/// The paint callback body. Runs with the GL context current.
///
/// egui clears the framebuffer at the start of every frame, so the picture is
/// drawn on *every* callback, not only when `mpv_render_context_update()`
/// reports a new frame. mpv redraws the previous frame when no new one is
/// queued, which is what makes that cheap.
fn paint(info: egui::PaintCallbackInfo, painter: &egui_glow::Painter) {
    let gl = painter.gl();

    // Deferred teardown from `detach()`. This is the only place the render
    // context can be freed safely.
    if FREE_PENDING.swap(false, Ordering::SeqCst) {
        if let Ok(mut slot) = CTX.lock() {
            if let Some(c) = slot.take() {
                unsafe {
                    (c.fns.free)(c.ctx);
                    if let Some(t) = TARGET.lock().unwrap().take() {
                        gl.delete_framebuffer(t.fbo);
                        gl.delete_texture(t.color);
                        gl.delete_program(t.program);
                        gl.delete_vertex_array(t.vao);
                    }
                };
            }
        }
        READY.store(false, Ordering::SeqCst);
        return;
    }

    let src = SOURCE.load(Ordering::SeqCst);
    if src.is_null() {
        return;
    }

    // `epaint::PaintCallbackInfo::screen_size_px` is the *full* framebuffer
    // in physical pixels, which is exactly what mpv_opengl_fbo.w/h want for
    // FBO 0. `viewport_in_pixels()` describes only the callback rectangle and
    // is the wrong number here.
    let (sw, sh) = (info.screen_size_px[0] as i32, info.screen_size_px[1] as i32);
    if sw <= 0 || sh <= 0 {
        return;
    }

    let mut slot = match CTX.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if slot.is_none() {
        match create(src) {
            Ok(c) => *slot = Some(c),
            Err(e) => {
                set_err(e);
                return;
            }
        }
        // render.h: the context must exist before `loadfile` creates a VO.
        // The queued load is drained by App::update(), so wake that loop now.
        READY.store(true, Ordering::SeqCst);
        super::jni_bridge::wake_ui();
    }
    let Some(c) = slot.as_ref() else { return };

    // Drain mpv's update bookkeeping. Optional without ADVANCED_CONTROL, but
    // harmless. Its return value is deliberately NOT a gate — see above.
    unsafe { (c.fns.update)(c.ctx) };

    let viewport = info.viewport_in_pixels();
    let partial = viewport.width_px != sw || viewport.height_px != sh;
    let mut target = TARGET.lock().unwrap();
    if partial {
        if viewport.width_px <= 0 || viewport.height_px <= 0 {
            return;
        }
        if target
            .as_ref()
            .is_none_or(|t| t.width != viewport.width_px || t.height != viewport.height_px)
        {
            unsafe {
                if let Some(old) = target.take() {
                    gl.delete_framebuffer(old.fbo);
                    gl.delete_texture(old.color);
                    gl.delete_program(old.program);
                    gl.delete_vertex_array(old.vao);
                }
                let Ok(fbo) = gl.create_framebuffer() else {
                    set_err("Vorschau-Framebuffer fehlt".into());
                    return;
                };
                let Ok(color) = gl.create_texture() else {
                    gl.delete_framebuffer(fbo);
                    set_err("Vorschau-Puffer fehlt".into());
                    return;
                };
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
                gl.bind_texture(glow::TEXTURE_2D, Some(color));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    viewport.width_px,
                    viewport.height_px,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    None,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(color),
                    0,
                );
                let valid =
                    gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.bind_texture(glow::TEXTURE_2D, None);
                if !valid {
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(color);
                    set_err("Vorschau-Framebuffer unvollständig".into());
                    return;
                }
                let program = match copy_program(gl) {
                    Ok(p) => p,
                    Err(e) => {
                        gl.delete_framebuffer(fbo);
                        gl.delete_texture(color);
                        set_err(e);
                        return;
                    }
                };
                let vao = match gl.create_vertex_array() {
                    Ok(v) => v,
                    Err(e) => {
                        gl.delete_framebuffer(fbo);
                        gl.delete_texture(color);
                        gl.delete_program(program);
                        set_err(e);
                        return;
                    }
                };
                *target = Some(Target {
                    fbo,
                    color,
                    program,
                    vao,
                    width: viewport.width_px,
                    height: viewport.height_px,
                });
            }
        }
    }
    let selected = if partial { target.as_ref() } else { None };
    let (render_width, render_height) = selected.map(|t| (t.width, t.height)).unwrap_or((sw, sh));
    let mut fbo = OpenglFbo {
        fbo: selected.map(|t| t.fbo.0.get() as i32).unwrap_or(0),
        w: render_width,
        h: render_height,
        internal_format: 0,
    };
    // OpenGL's framebuffer origin is bottom-left and mpv's is top-left.
    // `FLIP_Y=1` is what makes the picture upright; if a frame ever comes out
    // upside down, this single value is the knob.
    let mut flip: c_int = 1;
    let mut params = [
        RenderParam {
            type_: PARAM_OPENGL_FBO,
            data: &mut fbo as *mut _ as *mut c_void,
        },
        RenderParam {
            type_: PARAM_FLIP_Y,
            data: &mut flip as *mut _ as *mut c_void,
        },
        RenderParam {
            type_: PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];

    unsafe {
        // render_gl.h explicitly excludes glViewport and glScissor from the
        // state mpv restores, and egui continues drawing right after this.
        let mut old_vp = [0i32; 4];
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut old_vp);
        let mut old_scissor_box = [0i32; 4];
        gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut old_scissor_box);
        let old_scissor = gl.is_enabled(glow::SCISSOR_TEST);
        let old_blend = gl.is_enabled(glow::BLEND);
        let old_depth = gl.is_enabled(glow::DEPTH_TEST);

        // Full framebuffer, matching fbo.w/h. Whether mpv sets its own
        // viewport from those dimensions or honours this one, both agree.
        gl.viewport(0, 0, render_width, render_height);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::BLEND);

        let rc = (c.fns.render)(c.ctx, params.as_mut_ptr());
        if rc < 0 {
            set_err(format!("mpv_render_context_render: {}", rc));
        } else if let Ok(mut slot) = LAST_ERR.lock() {
            *slot = None;
        }

        if let Some(t) = selected {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(
                viewport.left_px,
                viewport.from_bottom_px,
                viewport.width_px,
                viewport.height_px,
            );
            gl.use_program(Some(t.program));
            gl.bind_vertex_array(Some(t.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(t.color));
            gl.uniform_1_i32(gl.get_uniform_location(t.program, "image").as_ref(), 0);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                set_err(format!("Vorschau-Kopie: OpenGL {error}"));
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        gl.viewport(old_vp[0], old_vp[1], old_vp[2], old_vp[3]);
        gl.scissor(
            old_scissor_box[0],
            old_scissor_box[1],
            old_scissor_box[2],
            old_scissor_box[3],
        );
        set_enabled(gl, glow::SCISSOR_TEST, old_scissor);
        set_enabled(gl, glow::BLEND, old_blend);
        set_enabled(gl, glow::DEPTH_TEST, old_depth);
    }
}

unsafe fn set_enabled(gl: &glow::Context, cap: u32, on: bool) {
    if on {
        gl.enable(cap);
    } else {
        gl.disable(cap);
    }
}

/// The video area as one egui shape.
///
/// Full-screen playback renders directly into the window. Smaller previews
/// render into a texture and draw inside the supplied rectangle.
pub fn shape(rect: egui::Rect) -> egui::Shape {
    egui::Shape::Callback(egui::PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(paint)),
    })
}

struct Target {
    fbo: glow::NativeFramebuffer,
    color: glow::NativeTexture,
    program: glow::NativeProgram,
    vao: glow::NativeVertexArray,
    width: i32,
    height: i32,
}
static TARGET: Mutex<Option<Target>> = Mutex::new(None);

/// Draw the rendered preview texture inside the destination viewport.
unsafe fn copy_program(gl: &glow::Context) -> Result<glow::NativeProgram, String> {
    const VERTEX: &str = r#"#version 300 es
precision highp float;
out vec2 uv;
void main() {
    vec2 position = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    uv = position;
    gl_Position = vec4(position * 2.0 - 1.0, 0.0, 1.0);
}
"#;
    const FRAGMENT: &str = r#"#version 300 es
precision highp float;
uniform sampler2D image;
in vec2 uv;
out vec4 color;
void main() {
    color = texture(image, uv);
}
"#;
    let program = gl.create_program()?;
    let mut shaders = Vec::new();
    for (kind, source) in [
        (glow::VERTEX_SHADER, VERTEX),
        (glow::FRAGMENT_SHADER, FRAGMENT),
    ] {
        let shader = match gl.create_shader(kind) {
            Ok(shader) => shader,
            Err(error) => {
                for shader in shaders {
                    gl.delete_shader(shader);
                }
                gl.delete_program(program);
                return Err(error);
            }
        };
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let error = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            for shader in shaders {
                gl.delete_shader(shader);
            }
            gl.delete_program(program);
            return Err(error);
        }
        gl.attach_shader(program, shader);
        shaders.push(shader);
    }
    gl.link_program(program);
    for shader in shaders {
        gl.detach_shader(program, shader);
        gl.delete_shader(shader);
    }
    if !gl.get_program_link_status(program) {
        let error = gl.get_program_info_log(program);
        gl.delete_program(program);
        return Err(error);
    }
    Ok(program)
}
