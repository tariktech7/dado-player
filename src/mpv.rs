//! Minimal safe-ish wrapper over the libmpv C API (loaded at runtime via libloading).
//! We only use the small subset needed for playback control + polling.

use crate::types::Settings;
use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
// `command()` terminates its argv with ptr::null() on EVERY target, including
// Android. This import used to be gated behind not(android) (it was only needed
// there by the desktop-only set_option path), which made the Android release
// build fail with E0433 while desktop `cargo check` stayed green.
use std::ptr;

const MPV_FORMAT_FLAG: c_int = 3;
const MPV_FORMAT_INT64: c_int = 4;
const MPV_FORMAT_STRING: c_int = 1;
const MPV_FORMAT_DOUBLE: c_int = 5;

type MpvCreate = unsafe extern "C" fn() -> *mut c_void;
type MpvInitialize = unsafe extern "C" fn(*mut c_void) -> c_int;
type MpvSetOption = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type MpvSetOptionString = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int;
type MpvCommand = unsafe extern "C" fn(*mut c_void, *const *const c_char) -> c_int;
type MpvGetProperty = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type MpvDestroy = unsafe extern "C" fn(*mut c_void);
type MpvErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
type MpvFree = unsafe extern "C" fn(*mut c_void);

struct Fns {
    create: MpvCreate,
    initialize: MpvInitialize,
    set_option: MpvSetOption,
    set_option_string: MpvSetOptionString,
    command: MpvCommand,
    get_property: MpvGetProperty,
    destroy: MpvDestroy,
    error_string: MpvErrorString,
    free: MpvFree,
}

/// Resolve every symbol once. Both platform paths need the same set, and a
/// missing one is a packaging error worth naming rather than a null call.
fn load_fns(lib: &Library) -> Result<Fns, String> {
    unsafe {
        macro_rules! sym {
            ($name:literal, $t:ty) => {
                *lib.get::<$t>(concat!($name, "\0").as_bytes())
                    .map_err(|e| format!("{}: {}", $name, e))?
            };
        }
        Ok(Fns {
            create: sym!("mpv_create", MpvCreate),
            initialize: sym!("mpv_initialize", MpvInitialize),
            set_option: sym!("mpv_set_option", MpvSetOption),
            set_option_string: sym!("mpv_set_option_string", MpvSetOptionString),
            command: sym!("mpv_command", MpvCommand),
            get_property: sym!("mpv_get_property", MpvGetProperty),
            destroy: sym!("mpv_terminate_destroy", MpvDestroy),
            error_string: sym!("mpv_error_string", MpvErrorString),
            free: sym!("mpv_free", MpvFree),
        })
    }
}

pub struct Mpv {
    /// `Option` so the Android drop path can hand the handle over instead of
    /// `dlclose`-ing libmpv underneath a still-live mpv instance.
    _lib: Option<Library>,
    ctx: *mut c_void,
    f: Fns,
}

/// TLS policy for provider streams.
///
/// Xtream panels commonly expose HLS/TS on a second port (`https_port`) whose
/// certificate is self-signed or issued for a different hostname. libmpv uses
/// curl for those requests; with verification enabled curl refuses the
/// connection before a single byte arrives and every channel stays black.
/// Reproduced on 2026-11-06: `TLS certificate verification failed` for a
/// `https://…:8443/live/...m3u8` URL, while the same URL with `tls-verify=no`
/// decodes and renders immediately. Verification stays opt-in in expert mode
/// so a panel with a real certificate can be used strictly.
fn apply_tls_policy(me: &Mpv, s: &crate::types::Settings) {
    if !s.tls_verify {
        me.set_opt_str("tls-verify", "no");
    }
}

impl Mpv {
    /// Android: video goes through the render API, not through a window.
    ///
    /// Verified in the emulator on 2026-11-03 (see MOBILE-BAUPLAN.md §17), and
    /// the reason `new_android` below looks nothing like `new_desktop`:
    /// 1. libmpv has NO `android-surface` option — the old code invented it and
    ///    always failed with "option not found".
    /// 2. Pointing `wid` at the GameActivity ANativeWindow plus
    ///    `gpu-context=android` makes mpv take over the EGL surface that
    ///    egui/glow owns: the UI goes permanently black.
    /// 3. mpv's android video path needs a registered JavaVM. `JNI_OnLoad` is
    ///    not in this libmpv build, so the VM has to be registered explicitly
    ///    via `av_jni_set_java_vm` before `mpv_initialize` — see
    ///    `platform::android::render`.
    ///
    /// So on Android there is no `wid` at all. The render API *is* the video
    /// output: `vo=libmpv` feeds frames into the render context that egui
    /// paints, and `loadfile` is deferred until that context exists.
    pub fn new(wid: isize, s: &crate::types::Settings) -> Result<Self, String> {
        #[cfg(target_os = "android")]
        {
            let _ = wid;
            Self::new_android(s)
        }
        #[cfg(not(target_os = "android"))]
        {
            Self::new_desktop(wid, s)
        }
    }

    /// The `mpv_handle` for the render API. Only meaningful on Android, where
    /// `render::attach` needs it; harmless elsewhere.
    pub fn handle(&self) -> *mut c_void {
        self.ctx
    }

    #[cfg(target_os = "android")]
    fn new_android(s: &crate::types::Settings) -> Result<Self, String> {
        let lib = load_library()?;
        let f = load_fns(&lib)?;
        unsafe {
            let ctx = (f.create)();
            if ctx.is_null() {
                return Err("mpv_create() lieferte null".to_string());
            }
            let me = Mpv {
                _lib: Some(lib),
                ctx,
                f,
            };

            // No `wid`, no `gpu-context`: the render API supplies the GL
            // context and the framebuffer. `vo=gpu` would instead make mpv
            // create its own output/window and leave render_context_render()
            // with no frames. libmpv/render.h: "Video initialization will fail
            // if the render context was not initialized yet, or it will revert
            // to a VO that creates its own window." There is no window here.
            // `gpu-api` belongs to vo=gpu and is deliberately not set.
            me.set_opt_str("vo", "libmpv");
            // The render context must exist *before* loadfile creates the VO;
            // platform::android::render queues the load until its first paint
            // callback has created it. See render.rs, rule 1.
            //
            // egui clears the framebuffer on every frame, so render() is
            // called on every paint. mpv normally blocks in render() until the
            // frame's display time; render.h names this global option as the
            // way to remove that headroom so the UI thread never stalls.
            me.set_opt_str("video-timing-offset", "0");
            // auto-safe falls back to software decoding on its own. If a
            // device's MediaCodec is broken, `hwdec=no` is the one-line change.
            me.set_opt_str("hwdec", "auto-safe");
            me.set_opt_str("ao", "audiotrack");
            me.set_opt_str("video-sync", s.video_sync.mpv());
            me.set_opt_str("keep-open", "no");
            me.set_opt_str("osc", "no");
            me.set_opt_str("osd-level", "0");
            me.set_opt_str("input-default-bindings", "no");
            me.set_opt_str("input-vo-keyboard", "no");
            me.set_opt_str("cache", "yes");
            me.set_opt_str("cache-secs", &s.cache_secs.clamp(5, 600).to_string());
            me.set_opt_str(
                "network-timeout",
                &s.network_timeout_s.clamp(5, 120).to_string(),
            );
            me.set_opt_str("sub-font-size", &s.sub_size.clamp(20.0, 120.0).to_string());
            if s.audio_passthrough {
                me.set_opt_str("audio-spdif", "ac3,eac3,dts,dts-hd,truehd");
            }
            if s.audio_normalize {
                me.set_opt_str("af", "dynaudnorm");
            }
            let ua = if s.user_agent.trim().is_empty() {
                "Mozilla/5.0 (Dado-Player)"
            } else {
                s.user_agent.trim()
            };
            me.set_opt_str("user-agent", ua);
            apply_tls_policy(&me, s);

            // libmpv has terminal output disabled on Android, so a failed
            // loadfile would otherwise be a silent black rectangle.
            // --log-file is the one logging mechanism that still writes mpv's
            // own diagnostics there. HOME points at filesDir (set by
            // nativeOnStorage before any player is created).
            if let Ok(dir) = std::env::var("HOME") {
                me.set_opt_str("log-file", &format!("{dir}/mpv.log"));
                me.set_opt_str("msg-level", "all=v");
            }

            // Must happen before mpv_initialize: mpv builds its decoder chain
            // during initialize and reads the VM at that point.
            crate::platform::android::render::ensure_java_vm()?;

            let rc = (me.f.initialize)(me.ctx);
            if rc < 0 {
                let e = me.err(rc);
                (me.f.destroy)(me.ctx);
                return Err(format!("mpv_initialize: {}", e));
            }

            me.set_volume(100);
            crate::platform::android::render::attach(me.ctx);
            Ok(me)
        }
    }

    #[cfg(not(target_os = "android"))]
    fn new_desktop(wid: isize, s: &crate::types::Settings) -> Result<Self, String> {
        let lib = load_library()?;
        let f = load_fns(&lib)?;
        unsafe {
            let ctx = (f.create)();
            if ctx.is_null() {
                return Err("mpv_create() lieferte null".to_string());
            }

            let me = Mpv {
                _lib: Some(lib),
                ctx,
                f,
            };

            // Options must be set before mpv_initialize().
            me.set_opt_str("vo", s.vo.mpv());
            me.set_opt_str("hwdec", s.hwdec.mpv());
            me.set_opt_str("video-sync", s.video_sync.mpv());
            me.set_opt_str("keep-open", "no");
            me.set_opt_str("osc", "no");
            me.set_opt_str("osd-level", "0");
            me.set_opt_str("input-default-bindings", "no");
            me.set_opt_str("input-vo-keyboard", "no");
            me.set_opt_str("cache", "yes");
            me.set_opt_str("cache-secs", &s.cache_secs.clamp(5, 600).to_string());
            me.set_opt_str(
                "network-timeout",
                &s.network_timeout_s.clamp(5, 120).to_string(),
            );
            me.set_opt_str("sub-font-size", &s.sub_size.clamp(20.0, 120.0).to_string());
            me.set_opt_str("interpolation", if s.interpolation { "yes" } else { "no" });
            if s.interpolation {
                me.set_opt_str("tscale", "oversample");
            }
            me.set_opt_str("deband", if s.deband { "yes" } else { "no" });

            // Audio: exclusive WASAPI is the biggest "does it work here?" switch.
            // WASAPI is Windows-only, so skip it on Android.
            #[cfg(not(target_os = "android"))]
            if s.audio_exclusive {
                me.set_opt_str("ao", "wasapi");
                me.set_opt_str("audio-exclusive", "yes");
            }
            if s.audio_passthrough {
                me.set_opt_str("audio-spdif", "ac3,eac3,dts,dts-hd,truehd");
            }
            if s.audio_normalize {
                me.set_opt_str("af", "dynaudnorm");
            }

            let ua = if s.user_agent.trim().is_empty() {
                "Mozilla/5.0 (Dado-Player)"
            } else {
                s.user_agent.trim()
            };
            me.set_opt_str("user-agent", ua);
            apply_tls_policy(&me, s);

            // mpv's own diagnostics are strictly opt-in.
            //
            // On Windows the GUI subsystem has no console, so `eprintln!`
            // from load() goes nowhere and a failed stream used to be a
            // silent black rectangle. `log-file` fixes that - but mpv logs
            // the *full* loadfile URL, and Xtream URLs carry username and
            // password in the path. Writing that to %TEMP% on every launch
            // would leak provider credentials, so the file is only created
            // when DADO_DEBUG=1 is set explicitly, and any leftover debug
            // log is removed on a normal start.
            #[cfg(not(target_os = "android"))]
            {
                let log = std::env::temp_dir().join("Dado-Player-mpv.log");
                let _ = std::fs::remove_file(&log);
                if std::env::var("DADO_DEBUG").is_ok() {
                    me.set_opt_str("log-file", &log.to_string_lossy().replace('\\', "/"));
                    me.set_opt_str("msg-level", "all=v");
                }
            }

            // Attach to the provided surface handle: embed into a child window
            // via `wid`. (Android never reaches this code — see `Mpv::new`.)
            let mut w: i64 = wid as i64;
            let name = CString::new("wid").unwrap();
            let rc = (me.f.set_option)(
                me.ctx,
                name.as_ptr(),
                MPV_FORMAT_INT64,
                &mut w as *mut i64 as *mut c_void,
            );
            if rc < 0 {
                let e = me.err(rc);
                return Err(format!("wid-Option: {}", e));
            }

            let rc = (me.f.initialize)(me.ctx);
            if rc < 0 {
                let e = me.err(rc);
                return Err(format!("mpv_initialize: {}", e));
            }

            me.set_volume(100);
            Ok(me)
        }
    }

    fn err(&self, code: c_int) -> String {
        unsafe {
            let p = (self.f.error_string)(code);
            if p.is_null() {
                format!("Fehler {}", code)
            } else {
                CStr::from_ptr(p).to_string_lossy().to_string()
            }
        }
    }

    fn set_opt_str(&self, name: &str, value: &str) {
        if let (Ok(n), Ok(v)) = (CString::new(name), CString::new(value)) {
            unsafe {
                (self.f.set_option_string)(self.ctx, n.as_ptr(), v.as_ptr());
            }
        }
    }

    fn command(&self, args: &[&str]) -> c_int {
        let cstrs: Vec<CString> = args
            .iter()
            .map(|a| CString::new(*a).unwrap_or_default())
            .collect();
        let mut ptrs: Vec<*const c_char> = cstrs.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        unsafe { (self.f.command)(self.ctx, ptrs.as_ptr()) }
    }

    pub fn load_at(&self, url: &str, seconds: f64) {
        let _ = self.command(&["set", "start", &seconds.max(0.0).to_string()]);
        self.load_command(url);
    }
    pub fn load(&self, url: &str) {
        self.load_at(url, 0.0);
    }
    fn load_command(&self, url: &str) {
        let rc = self.command(&["loadfile", url]);
        if rc < 0 {
            // Never log the URL: provider URLs carry credentials in the query.
            eprintln!("[player] loadfile: {}", self.err(rc));
        }
    }

    /// Audio-only playback (M3U `radio="true"`): mpv skips the video track
    /// entirely, so no VO is created and the embedded surface stays black.
    /// Must be (re)set before `load()`; `enable_video()` restores playback for
    /// the next normal item.
    pub fn disable_video(&self) {
        let _ = self.command(&["set", "vid", "no"]);
    }

    pub fn enable_video(&self) {
        let _ = self.command(&["set", "vid", "auto"]);
    }

    pub fn stop(&self) {
        let _ = self.command(&["stop"]);
    }

    pub fn configure(&self, settings: &Settings) {
        for (key, value) in [
            ("cache-secs", settings.cache_secs.clamp(5, 600).to_string()),
            (
                "network-timeout",
                settings.network_timeout_s.clamp(5, 120).to_string(),
            ),
            (
                "sub-font-size",
                settings.sub_size.clamp(20.0, 120.0).to_string(),
            ),
            (
                "user-agent",
                if settings.user_agent.trim().is_empty() {
                    "Mozilla/5.0 (Dado-Player)".into()
                } else {
                    settings.user_agent.trim().into()
                },
            ),
        ] {
            let _ = self.command(&["set", key, &value]);
        }
    }

    pub fn set_pause(&self, paused: bool) {
        let _ = self.command(&["set", "pause", if paused { "yes" } else { "no" }]);
    }

    pub fn toggle_pause(&self) {
        let _ = self.command(&["cycle", "pause"]);
    }

    pub fn seek_absolute(&self, secs: f64) {
        let s = format!("{}", secs.max(0.0));
        let _ = self.command(&["seek", &s, "absolute"]);
    }

    pub fn seek_relative(&self, secs: f64) {
        let s = format!("{}", secs);
        let _ = self.command(&["seek", &s, "relative"]);
    }

    pub fn set_volume(&self, vol: i64) {
        let v = vol.clamp(0, 100).to_string();
        let _ = self.command(&["set", "volume", &v]);
    }

    fn get_double(&self, name: &str) -> Option<f64> {
        let n = CString::new(name).ok()?;
        let mut val: f64 = 0.0;
        let rc = unsafe {
            (self.f.get_property)(
                self.ctx,
                n.as_ptr(),
                MPV_FORMAT_DOUBLE,
                &mut val as *mut f64 as *mut c_void,
            )
        };
        if rc >= 0 {
            Some(val)
        } else {
            None
        }
    }

    fn get_flag(&self, name: &str) -> Option<bool> {
        let n = CString::new(name).ok()?;
        let mut val: c_int = 0;
        let rc = unsafe {
            (self.f.get_property)(
                self.ctx,
                n.as_ptr(),
                MPV_FORMAT_FLAG,
                &mut val as *mut c_int as *mut c_void,
            )
        };
        if rc >= 0 {
            Some(val != 0)
        } else {
            None
        }
    }

    pub fn position(&self) -> f64 {
        self.get_double("time-pos").unwrap_or(0.0)
    }

    pub fn duration(&self) -> f64 {
        self.get_double("duration").unwrap_or(0.0)
    }

    /// Seconds buffered ahead of the play position (for the "loading" part of
    /// the progress bar).
    pub fn cache_time(&self) -> f64 {
        self.get_double("demuxer-cache-time").unwrap_or(0.0)
    }

    pub fn paused(&self) -> bool {
        self.get_flag("pause").unwrap_or(false)
    }

    pub fn eof(&self) -> bool {
        self.get_flag("eof-reached").unwrap_or(false)
    }

    // ------------------------------------------------------------------
    // Stream metadata (read every frame into the corner overlay). Getters
    // return 0 / "" when the property is not yet available (before the first
    // decoded frame), so the UI simply shows "…" until then.
    // ------------------------------------------------------------------

    fn get_int64(&self, name: &str) -> Option<i64> {
        let n = CString::new(name).ok()?;
        let mut val: i64 = 0;
        let rc = unsafe {
            (self.f.get_property)(
                self.ctx,
                n.as_ptr(),
                MPV_FORMAT_INT64,
                &mut val as *mut i64 as *mut c_void,
            )
        };
        if rc >= 0 {
            Some(val)
        } else {
            None
        }
    }

    fn get_string(&self, name: &str) -> Option<String> {
        let n = CString::new(name).ok()?;
        let mut val: *mut c_char = std::ptr::null_mut();
        let rc = unsafe {
            (self.f.get_property)(
                self.ctx,
                n.as_ptr(),
                MPV_FORMAT_STRING,
                &mut val as *mut *mut c_char as *mut c_void,
            )
        };
        if rc >= 0 && !val.is_null() {
            let s = unsafe { CStr::from_ptr(val).to_string_lossy().to_string() };
            unsafe { (self.f.free)(val as *mut c_void) };
            Some(s)
        } else {
            None
        }
    }

    /// Coded video resolution (0 before the first frame decodes).
    pub fn width(&self) -> i64 {
        self.get_int64("width").unwrap_or(0)
    }

    pub fn height(&self) -> i64 {
        self.get_int64("height").unwrap_or(0)
    }

    /// Estimated displayed frame rate.
    pub fn fps(&self) -> f64 {
        self.get_double("estimated-vf-fps").unwrap_or(0.0)
    }

    pub fn video_codec(&self) -> String {
        self.get_string("video-codec").unwrap_or_default()
    }

    pub fn audio_codec(&self) -> String {
        self.get_string("audio-codec").unwrap_or_default()
    }

    /// Container format (mp4, mkv, ts, …).
    pub fn container(&self) -> String {
        self.get_string("file-format").unwrap_or_default()
    }

    /// Currently selected audio track id (0 = none).
    pub fn current_audio(&self) -> i64 {
        self.get_int64("aid").unwrap_or(0)
    }

    /// Currently selected subtitle track id (0 = none).
    pub fn current_sub(&self) -> i64 {
        self.get_int64("sid").unwrap_or(0)
    }

    /// mpv `track-list` as a JSON string (audio + sub + video tracks).
    pub fn track_list_json(&self) -> Option<String> {
        self.get_string("track-list")
    }

    // ------------------------------------------------------------------
    // Track selection (movies / series only).
    // ------------------------------------------------------------------

    /// Select an audio track by id (mpv `aid`).
    pub fn set_audio_track(&self, id: i64) {
        let _ = self.command(&["set", "aid", &id.to_string()]);
    }

    /// Select a subtitle track by id (mpv `sid`).
    pub fn set_sub_track(&self, id: i64) {
        let _ = self.command(&["set", "sid", &id.to_string()]);
    }

    /// Turn subtitles off (mpv `sid=no`).
    pub fn disable_sub(&self) {
        let _ = self.command(&["set", "sid", "no"]);
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        {
            // mpv requires the render context to be freed before its own
            // handle is destroyed, and the render context may only be freed
            // with the GL context current. Neither can happen here, so hand
            // the teardown to the next paint callback and leave the mpv handle
            // alive: this player is a process-lifetime singleton (nothing sets
            // `App::player` back to `None`), so the handle would only be
            // destroyed at exit anyway.
            if crate::platform::android::render::is_attached() {
                crate::platform::android::render::detach();
                // The mpv handle outlives this struct, so libmpv must stay
                // mapped. Leaking one `dlopen` reference is the price; the
                // alternative is `dlclose` under a live handle.
                if let Some(lib) = self._lib.take() {
                    std::mem::forget(lib);
                }
                return;
            }
        }
        unsafe {
            (self.f.destroy)(self.ctx);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn check_library_available() -> Result<(), String> {
    load_library().map(drop)
}

fn load_library() -> Result<Library, String> {
    #[cfg(target_os = "android")]
    let names = ["libmpv.so"];
    #[cfg(not(target_os = "android"))]
    let names = ["libmpv-2.dll", "mpv-1.dll", "libmpv.dll", "mpv-2.dll"];
    // `dist/` ist der einzige Ort, an dem die DLLs liegen (portable Auslieferung).
    // Der cwd-Fallback hält `cargo run` aus dem Projektwurzelverzeichnis lauffähig.
    let subdirs = ["dist", "resources"];
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for n in names {
                paths.push(dir.join(n));
                for s in subdirs {
                    paths.push(dir.join(s).join(n));
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for n in names {
            paths.push(cwd.join(n));
            for s in subdirs {
                paths.push(cwd.join(s).join(n));
            }
        }
    }
    for p in &paths {
        if p.exists() {
            if let Ok(lib) = open_library(p) {
                return Ok(lib);
            }
        }
    }
    for n in names {
        if let Ok(lib) = open_library(n) {
            return Ok(lib);
        }
    }
    let msg = if cfg!(target_os = "android") {
        "libmpv.so nicht gefunden (in die APK jniLibs legen)".to_string()
    } else {
        "libmpv-2.dll nicht gefunden – bitte neben die .exe legen.".to_string()
    };
    Err(msg)
}

/// Load the exact DLL/so path and, on Windows, add that DLL's parent to the
/// dependency search path first. The mpv bundle's FFmpeg and codec DLLs live
/// beside libmpv-2.dll, so loading only the top-level DLL by absolute path is
/// not sufficient when the app is extracted into an arbitrary directory.
#[cfg(target_os = "windows")]
fn open_library(path: impl AsRef<std::ffi::OsStr>) -> Result<Library, libloading::Error> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

    let path = std::path::Path::new(path.as_ref());
    if let Some(parent) = path.parent() {
        let wide: Vec<u16> = parent.as_os_str().encode_wide().chain(Some(0)).collect();
        // The bundled mpv DLLs depend on sibling FFmpeg/codec DLLs. This is a
        // portable app, so use the extracted folder, never an install path.
        unsafe {
            SetDllDirectoryW(wide.as_ptr());
        }
    }
    unsafe { Library::new(path) }
}

#[cfg(not(target_os = "windows"))]
fn open_library(path: impl AsRef<std::ffi::OsStr>) -> Result<Library, libloading::Error> {
    unsafe { Library::new(path) }
}
