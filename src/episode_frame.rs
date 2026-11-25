use super::{load_fns, load_library, Mpv};
use crate::types::Settings;
use std::{
    ffi::{c_int, c_void},
    ptr,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[repr(C)]
struct Param(c_int, *mut c_void);

type Render = unsafe extern "C" fn(*mut c_void, *mut Param) -> c_int;
type Free = unsafe extern "C" fn(*mut c_void);

struct Surface {
    context: *mut c_void,
    free: Free,
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { (self.free)(self.context) };
    }
}

pub(crate) fn frame(
    url: &str,
    settings: &Settings,
    cancel: &AtomicBool,
) -> Option<egui::ColorImage> {
    let _ = reqwest::Url::parse(url)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))?;
    #[cfg(target_os = "android")]
    crate::platform::android::render::ensure_java_vm().ok()?;
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let lib = load_library().ok()?;
    let f = load_fns(&lib).ok()?;
    let ctx = unsafe { (f.create)() };
    if ctx.is_null() {
        return None;
    }
    let player = Mpv {
        _lib: Some(lib),
        ctx,
        f,
        #[cfg(target_os = "android")]
        owns_android_renderer: false,
    };
    for (option, value) in [
        ("config", "no"),
        ("terminal", "no"),
        ("vo", "libmpv"),
        ("hwdec", "no"),
        ("audio", "no"),
        ("sid", "no"),
        ("pause", "yes"),
        ("start", "5"),
        ("osd-level", "0"),
        ("network-timeout", "4"),
    ] {
        player.set_opt_str(option, value);
    }
    super::apply_tls_policy(&player, settings);
    if !settings.user_agent.trim().is_empty() {
        player.set_opt_str("user-agent", settings.user_agent.trim());
    }
    if unsafe { (player.f.initialize)(ctx) } < 0 {
        return None;
    }

    let lib = player._lib.as_ref()?;
    let (create, render, update, free) = unsafe {
        type Create = unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *mut Param) -> c_int;
        type Update = unsafe extern "C" fn(*mut c_void) -> u64;
        (
            *lib.get::<Create>(b"mpv_render_context_create\0").ok()?,
            *lib.get::<Render>(b"mpv_render_context_render\0").ok()?,
            *lib.get::<Update>(b"mpv_render_context_update\0").ok()?,
            *lib.get::<Free>(b"mpv_render_context_free\0").ok()?,
        )
    };
    let mut raw = ptr::null_mut();
    let mut init = [
        Param(1, c"sw".as_ptr().cast_mut().cast()),
        Param(0, ptr::null_mut()),
    ];
    if unsafe { create(&mut raw, ctx, init.as_mut_ptr()) } < 0 || raw.is_null() {
        return None;
    }
    let surface = Surface { context: raw, free };
    if player.command(&["loadfile", url]) < 0 {
        return None;
    }

    const WIDTH: usize = 320;
    const HEIGHT: usize = 180;
    let mut size = [WIDTH as c_int, HEIGHT as c_int];
    let mut stride = WIDTH * 4;
    let mut pixels = vec![0u32; WIDTH * HEIGHT];
    let mut params = [
        Param(17, size.as_mut_ptr().cast()),
        Param(18, c"rgb0".as_ptr().cast_mut().cast()),
        Param(19, (&mut stride as *mut usize).cast()),
        Param(20, pixels.as_mut_ptr().cast()),
        Param(0, ptr::null_mut()),
    ];
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline && !cancel.load(Ordering::Relaxed) {
        if unsafe { update(surface.context) } & 1 == 1
            && player.width() > 0
            && unsafe { render(surface.context, params.as_mut_ptr()) } >= 0
        {
            let pixels = pixels
                .into_iter()
                .map(|pixel| {
                    let [r, g, b, _] = pixel.to_ne_bytes();
                    egui::Color32::from_rgb(r, g, b)
                })
                .collect();
            return Some(egui::ColorImage {
                size: [WIDTH, HEIGHT],
                pixels,
            });
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}
