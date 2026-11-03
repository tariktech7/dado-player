//! Minimal safe-ish wrapper over the libmpv C API (loaded at runtime via libloading).
//! We only use the small subset needed for playback control + polling.

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::ptr;

const MPV_FORMAT_FLAG: c_int = 3;
const MPV_FORMAT_INT64: c_int = 4;
const MPV_FORMAT_DOUBLE: c_int = 5;

type MpvCreate = unsafe extern "C" fn() -> *mut c_void;
type MpvInitialize = unsafe extern "C" fn(*mut c_void) -> c_int;
type MpvSetOption = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type MpvSetOptionString = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int;
type MpvCommand = unsafe extern "C" fn(*mut c_void, *const *const c_char) -> c_int;
type MpvGetProperty = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type MpvDestroy = unsafe extern "C" fn(*mut c_void);
type MpvErrorString = unsafe extern "C" fn(c_int) -> *const c_char;

struct Fns {
    set_option: MpvSetOption,
    set_option_string: MpvSetOptionString,
    command: MpvCommand,
    get_property: MpvGetProperty,
    destroy: MpvDestroy,
    error_string: MpvErrorString,
}

pub struct Mpv {
    _lib: Library,
    ctx: *mut c_void,
    f: Fns,
}

impl Mpv {
    pub fn new(wid: isize, s: &crate::types::Settings) -> Result<Self, String> {
        let lib = load_library()?;
        unsafe {
            let create: MpvCreate = *lib
                .get(b"mpv_create\0")
                .map_err(|e| format!("mpv_create: {}", e))?;
            let init: MpvInitialize = *lib
                .get(b"mpv_initialize\0")
                .map_err(|e| format!("mpv_initialize: {}", e))?;
            let set_option: MpvSetOption = *lib
                .get(b"mpv_set_option\0")
                .map_err(|e| format!("mpv_set_option: {}", e))?;
            let set_option_string: MpvSetOptionString = *lib
                .get(b"mpv_set_option_string\0")
                .map_err(|e| format!("mpv_set_option_string: {}", e))?;
            let command: MpvCommand = *lib
                .get(b"mpv_command\0")
                .map_err(|e| format!("mpv_command: {}", e))?;
            let get_property: MpvGetProperty = *lib
                .get(b"mpv_get_property\0")
                .map_err(|e| format!("mpv_get_property: {}", e))?;
            let destroy: MpvDestroy = *lib
                .get(b"mpv_terminate_destroy\0")
                .map_err(|e| format!("mpv_terminate_destroy: {}", e))?;
            let error_string: MpvErrorString = *lib
                .get(b"mpv_error_string\0")
                .map_err(|e| format!("mpv_error_string: {}", e))?;

            let ctx = create();
            if ctx.is_null() {
                return Err("mpv_create() lieferte null".to_string());
            }

            let me = Mpv {
                _lib: lib,
                ctx,
                f: Fns {
                    set_option,
                    set_option_string,
                    command,
                    get_property,
                    destroy,
                    error_string,
                },
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
            me.set_opt_str("network-timeout", &s.network_timeout_s.clamp(5, 120).to_string());
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

            // Attach to the provided surface handle.
            // Windows: embed into a child window via `wid`.
            // Android: render directly into the ANativeWindow via `android-surface`.
            #[cfg(target_os = "android")]
            {
                let mut surf: i64 = wid as i64;
                let name = CString::new("android-surface").unwrap();
                let rc = (me.f.set_option)(
                    me.ctx,
                    name.as_ptr(),
                    MPV_FORMAT_INT64,
                    &mut surf as *mut i64 as *mut c_void,
                );
                if rc < 0 {
                    let e = me.err(rc);
                    return Err(format!("android-surface-Option: {}", e));
                }
            }
            #[cfg(not(target_os = "android"))]
            {
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
            }

            let rc = init(me.ctx);
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

    pub fn load(&self, url: &str) {
        let _ = self.command(&["loadfile", url]);
    }

    pub fn stop(&self) {
        let _ = self.command(&["stop"]);
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
}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe {
            (self.f.destroy)(self.ctx);
        }
    }
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
            if let Ok(lib) = unsafe { Library::new(p) } {
                return Ok(lib);
            }
        }
    }
    for n in names {
        if let Ok(lib) = unsafe { Library::new(n) } {
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
