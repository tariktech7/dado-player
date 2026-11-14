#![windows_subsystem = "windows"]

//! Self-extracting single-file launcher for Dado Player.
//!
//! The shipped `Dado-Player.exe` (the real eframe/egui app) plus its application-local
//! runtime DLLs are compressed and appended to THIS binary as a package. On
//! first launch the launcher extracts them into a private runtime directory
//! (`%LOCALAPPDATA%\DadoPlayer\runtime\rt-<id>`, or `DADO_RUNTIME_DIR`), then
//! runs the extracted app from there. This lets the user receive a ZIP that
//! contains exactly one `.exe`.
//!
//! Design constraints (see project memory):
//! * The launcher itself must import only OS DLLs, never the bundled ones. That
//!   is why it is built with `-C target-feature=+crt-static` (no VCRUNTIME140)
//!   and uses neither wgpu nor glutin (no d3dcompiler_47/opengl32 import). The
//!   Win32 calls it makes are raw `extern "system"` declarations against
//!   user32/gdi32/gdiplus — all system DLLs present on every Windows 10/11.
//! * The real app keeps its multi-backend child-process dispatcher untouched.
//! * `DADO_*` / `IPTV_TEST_*` environment variables are inherited by the child,
//!   so the same QA hooks work through the launcher.
//! * The boot splash is a borderless, chrome-less loading card (no title bar /
//!   minimize / close). It is a *layered* window: the whole frame (rounded card
//!   + logo + spinner) is composited into one off-screen ARGB bitmap and pushed
//!   atomically with `UpdateLayeredWindow`, so it never flickers and the logo's
//!   real transparency is honoured. Rendered with GDI+ for crisp, anti-aliased
//!   graphics. Extraction runs on a background thread so the spinner actually
//!   animates during the 2-4s first-run decompression.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use miniz_oxide::inflate::decompress_to_vec_zlib;

#[path = "../../package_footer.rs"]
mod package_footer;

// ---------------------------------------------------------------------------
// Win32 raw FFI (system DLLs only)
// ---------------------------------------------------------------------------
type HWND = *mut std::ffi::c_void;
type HINSTANCE = *mut std::ffi::c_void;
type HICON = *mut std::ffi::c_void;
type HMENU = *mut std::ffi::c_void;
type HCURSOR = *mut std::ffi::c_void;
type HBRUSH = *mut std::ffi::c_void;
type HDC = *mut std::ffi::c_void;
type HANDLE = *mut std::ffi::c_void;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type ATOM = u16;
type UINT = u32;
type BOOL = i32;

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
    time: u32,
    pt: POINT,
}

#[repr(C)]
struct WNDCLASSW {
    style: u32,
    lpfn_wnd_proc: WNDPROC,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
}

type WNDPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
struct BITMAPINFOHEADER {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct BITMAPINFO {
    bmi_header: BITMAPINFOHEADER,
    bmi_colors: [u32; 1],
}

#[repr(C)]
struct BLENDFUNCTION {
    blend_op: u8,
    blend_flags: u8,
    source_constant_alpha: u8,
    alpha_format: u8,
}

#[repr(C)]
struct SIZE {
    cx: i32,
    cy: i32,
}

// GDI+ startup input (x64 layout: 4 + 4 pad + 8 ptr + 4 + 4 = 24 bytes).
#[repr(C)]
struct GdiplusStartupInput {
    gdiplus_version: u32,
    debug_event_callback: usize,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

const IDC_APPSTARTING: u16 = 32650;
const CS_DROPSHADOW: u32 = 0x0002_0000;
const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_TOPMOST: u32 = 0x0000_0008;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
const SW_SHOWNORMAL: i32 = 1;
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;
const MB_ICONERROR: u32 = 0x10;
const MB_OK: u32 = 0x0;
const WM_DESTROY: u32 = 0x0002;
const WM_TIMER: u32 = 0x0113;
const WM_USER: u32 = 0x0400;
const WM_USER_DONE: u32 = WM_USER + 1;
const GWLP_USERDATA: i32 = -21;

// Splash card geometry / palette.
// The splash is a *layered* (per-pixel alpha) window: the whole frame — card,
// logo and spinner — is composited by the OS in one atomic op, so there is no
// flicker and the rounded corners / transparent logo come for free from the
// alpha channel. The visible card is 300x250; we give the offscreen bitmap a
// little extra margin so the soft drop shadow has room to spill outside it.
const DIB_W: i32 = 320;
const DIB_H: i32 = 270;
const CARD_X: i32 = 10;
const CARD_Y: i32 = 10;
const CARD_W: i32 = 300;
const CARD_H: i32 = 250;
const RADIUS: f32 = 16.0;
const LOGO_DX: f32 = (CARD_X + (CARD_W - 140) / 2) as f32; // centred horizontally
const LOGO_DY: f32 = (CARD_Y + (CARD_H - 140) / 2 - 22) as f32;
const SPIN_CX: f32 = (CARD_X + CARD_W / 2) as f32;
const SPIN_CY: f32 = (CARD_Y + CARD_H / 2 + 78) as f32;
const CARD_ARGB: u32 = 0xFF_2022_25; // dark charcoal, Discord-like
const ACCENT_ARGB: u32 = 0xFF_4D_A3_F0; // bright blue spinner on dark card

// Layered-window / alpha-blend plumbing.
const WS_EX_LAYERED: u32 = 0x0008_0000;
const ULW_ALPHA: u32 = 0x0000_0002;
const DIB_RGB_COLORS: u32 = 0;
const BI_RGB: u32 = 0;
const AC_SRC_OVER: u8 = 0;
const AC_SRC_ALPHA: u8 = 1;

#[link(name = "user32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> HINSTANCE;
    fn RegisterClassW(lp_wnd_class: *const WNDCLASSW) -> ATOM;
    fn CreateWindowExW(
        dw_ex_style: u32,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: u32,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        hwnd_parent: HWND,
        h_menu: HMENU,
        h_instance: HINSTANCE,
        lp_param: *mut std::ffi::c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn DestroyWindow(hwnd: HWND) -> BOOL;
    fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
    fn GetSystemMetrics(n_index: i32) -> i32;
    fn LoadCursorW(h_instance: HINSTANCE, lp_cursor_name: *const u16) -> HCURSOR;
    fn ShowWindow(hwnd: HWND, n_cmd_show: i32) -> BOOL;
    fn TranslateMessage(lp_msg: *const MSG) -> BOOL;
    fn MessageBoxW(hwnd: HWND, lp_text: *const u16, lp_caption: *const u16, u_type: UINT) -> i32;
    fn SetTimer(
        hwnd: HWND,
        n_id_event: usize,
        u_elapse: u32,
        lp_timer_func: *const std::ffi::c_void,
    ) -> usize;
    fn KillTimer(hwnd: HWND, n_id_event: usize) -> BOOL;
    fn PostMessageW(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> BOOL;
    fn GetMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: UINT,
        w_msg_filter_max: UINT,
    ) -> i32;
    fn SetProcessDPIAware() -> BOOL;
    fn GetDC(h: HWND) -> HDC;
    fn ReleaseDC(h: HWND, hdc: HDC) -> i32;
    fn UpdateLayeredWindow(
        hwnd: HWND,
        hdc_dst: HDC,
        ppt_dst: *const POINT,
        psize: *const SIZE,
        hdc_src: HDC,
        ppt_src: *const POINT,
        cr_key: u32,
        pblend: *const BLENDFUNCTION,
        flags: u32,
    ) -> BOOL;
    fn SetWindowLongPtrW(hwnd: HWND, index: i32, new_long: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: HDC) -> HDC;
    fn CreateDIBSection(
        hdc: HDC,
        pbmi: *const BITMAPINFO,
        usage: u32,
        ppv_bits: *mut *mut std::ffi::c_void,
        h_section: HANDLE,
        offset: u32,
    ) -> HANDLE;
    fn SelectObject(hdc: HDC, h: HANDLE) -> HANDLE;
    fn DeleteObject(h: HANDLE) -> BOOL;
    fn DeleteDC(hdc: HDC) -> BOOL;
}

#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(
        token: *mut usize,
        input: *const GdiplusStartupInput,
        output: *mut std::ffi::c_void,
    ) -> u32;
    fn GdiplusShutdown(token: usize);
    fn GdipCreateFromHDC(hdc: HDC, graphics: *mut *mut std::ffi::c_void) -> u32;
    fn GdipDeleteGraphics(graphics: *mut std::ffi::c_void) -> u32;
    fn GdipSetSmoothingMode(graphics: *mut std::ffi::c_void, mode: i32) -> u32;
    fn GdipSetInterpolationMode(graphics: *mut std::ffi::c_void, mode: i32) -> u32;
    fn GdipCreateSolidFill(color: u32, brush: *mut *mut std::ffi::c_void) -> u32;
    fn GdipDeleteBrush(brush: *mut std::ffi::c_void) -> u32;
    fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut *mut std::ffi::c_void) -> u32;
    fn GdipDeletePen(pen: *mut std::ffi::c_void) -> u32;
    fn GdipDrawArc(
        graphics: *mut std::ffi::c_void,
        pen: *mut std::ffi::c_void,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        start_angle: f32,
        sweep_angle: f32,
    ) -> u32;
    fn GdipCreateBitmapFromScan0(
        w: i32,
        h: i32,
        stride: i32,
        format: i32,
        scan0: *mut u8,
        bitmap: *mut *mut std::ffi::c_void,
    ) -> u32;
    fn GdipDisposeImage(image: *mut std::ffi::c_void) -> u32;
    fn GdipDrawImageRect(
        graphics: *mut std::ffi::c_void,
        image: *mut std::ffi::c_void,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> u32;
    fn GdipGraphicsClear(graphics: *mut std::ffi::c_void, color: u32) -> u32;
    fn GdipCreatePath(fill_mode: i32, path: *mut *mut std::ffi::c_void) -> u32;
    fn GdipAddPathArc(
        path: *mut std::ffi::c_void,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        start_angle: f32,
        sweep_angle: f32,
    ) -> u32;
    fn GdipClosePathFigure(path: *mut std::ffi::c_void) -> u32;
    fn GdipFillPath(
        graphics: *mut std::ffi::c_void,
        brush: *mut std::ffi::c_void,
        path: *mut std::ffi::c_void,
    ) -> u32;
    fn GdipDeletePath(path: *mut std::ffi::c_void) -> u32;
}

const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;
const SMOOTHING_ANTIALIAS_8X4: i32 = 4;
const INTERP_BICUBIC: i32 = 7;
const UNIT_PIXEL: i32 = 2;

// ---------------------------------------------------------------------------
// CRC32 (matches zlib.crc32, reflected, poly 0xEDB88320)
// ---------------------------------------------------------------------------
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

// ---------------------------------------------------------------------------
// Embedded logo (raw RGBA, 256x256, top-down). Exported by export_logo.py.
// ---------------------------------------------------------------------------
struct LogoBitmap {
    w: i32,
    h: i32,
    // Premultiplied BGRA (GDI+/DIB memory order). Built from the source RGBA so
    // the logo's existing transparency is preserved — no white backing.
    bgra: Vec<u8>,
}

static LOGO: OnceLock<Option<LogoBitmap>> = OnceLock::new();
// Spinner angle in degrees, advanced by the timer on the UI thread.
static SPIN_ANGLE: AtomicU32 = AtomicU32::new(0);
// GDI+ token, valid for the lifetime of the splash window.
static GDI_TOKEN: OnceLock<usize> = OnceLock::new();

fn decode_logo() -> Option<LogoBitmap> {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/logo.rgba"));
    let w = 256i32;
    let h = 256i32;
    if bytes.len() != (w * h * 4) as usize {
        return None;
    }
    let mut bgra = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks_exact(4) {
        let r = chunk[0] as f32;
        let g = chunk[1] as f32;
        let b = chunk[2] as f32;
        let a = chunk[3] as f32 / 255.0;
        // Straight RGBA -> premultiplied BGRA. Keeping the real alpha is what
        // makes the logo composite cleanly onto the transparent splash instead
        // of showing a white box.
        bgra.push((b * a) as u8);
        bgra.push((g * a) as u8);
        bgra.push((r * a) as u8);
        bgra.push(chunk[3]);
    }
    Some(LogoBitmap { w, h, bgra })
}

// Everything the layered-window compositor needs for one atomic frame push.
// It is stashed in the window's GWLP_USERDATA slot so the (stateless) window
// proc can reach it on each timer tick. There is exactly one splash per
// process, so a single boxed instance is sufficient.
struct SplashData {
    hwnd: HWND,
    hdc_screen: HDC,
    hdc_mem: HDC,
    hbmp: HANDLE,
    hbmp_old: HANDLE,
    g: *mut std::ffi::c_void,
    pt_dst: POINT,
    size: SIZE,
}

// ---------------------------------------------------------------------------
// Splash compositor (layered window, GDI+, anti-aliased)
// ---------------------------------------------------------------------------
// Fill a rounded rectangle path. The corners are left transparent in the DIB,
// which is exactly what gives the card its rounded shape via the alpha channel
// (no window region / no white backing required).
unsafe fn fill_rounded_rect(
    g: *mut std::ffi::c_void,
    brush: *mut std::ffi::c_void,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
) {
    let mut path: *mut std::ffi::c_void = std::ptr::null_mut();
    if GdipCreatePath(0, &mut path) != 0 {
        return;
    }
    let d = 2.0 * r;
    GdipAddPathArc(path, x, y, d, d, 180.0, 90.0);
    GdipAddPathArc(path, x + w - d, y, d, d, 270.0, 90.0);
    GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0.0, 90.0);
    GdipAddPathArc(path, x, y + h - d, d, d, 90.0, 90.0);
    GdipClosePathFigure(path);
    GdipFillPath(g, brush, path);
    GdipDeletePath(path);
}

// Composite the whole frame (transparent background + rounded card + logo +
// spinner) into the off-screen DIB and push it to the screen in ONE atomic
// UpdateLayeredWindow call. Because nothing is ever painted directly to the
// visible window DC, there is no flicker.
unsafe fn composite_splash(hwnd: HWND) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SplashData;
    if ptr.is_null() {
        return;
    }
    let d = &*ptr;
    GdipGraphicsClear(d.g, 0x0000_0000);
    GdipSetSmoothingMode(d.g, SMOOTHING_ANTIALIAS_8X4);
    GdipSetInterpolationMode(d.g, INTERP_BICUBIC);

    // Rounded card, opaque dark fill on transparent surroundings.
    let mut brush: *mut std::ffi::c_void = std::ptr::null_mut();
    if GdipCreateSolidFill(CARD_ARGB, &mut brush) == 0 {
        fill_rounded_rect(
            d.g,
            brush,
            CARD_X as f32,
            CARD_Y as f32,
            CARD_W as f32,
            CARD_H as f32,
            RADIUS,
        );
        GdipDeleteBrush(brush);
    }

    // Logo, centred in the upper portion of the card.
    if let Some(logo) = LOGO.get().and_then(|o| o.as_ref()) {
        let mut bmp: *mut std::ffi::c_void = std::ptr::null_mut();
        let stride = logo.w * 4;
        if GdipCreateBitmapFromScan0(
            logo.w,
            logo.h,
            stride,
            PIXEL_FORMAT_32BPP_ARGB,
            logo.bgra.as_ptr() as *mut u8,
            &mut bmp,
        ) == 0
        {
            GdipDrawImageRect(d.g, bmp, LOGO_DX, LOGO_DY, 140.0, 140.0);
            GdipDisposeImage(bmp);
        }
    }

    // Spinner, anti-aliased, below the logo.
    let angle = SPIN_ANGLE.load(Ordering::SeqCst) as f32;
    let mut pen: *mut std::ffi::c_void = std::ptr::null_mut();
    if GdipCreatePen1(ACCENT_ARGB, 4.0, UNIT_PIXEL, &mut pen) == 0 {
        let r = 18.0f32;
        GdipDrawArc(
            d.g,
            pen,
            SPIN_CX - r,
            SPIN_CY - r,
            2.0 * r,
            2.0 * r,
            angle,
            300.0,
        );
        GdipDeletePen(pen);
    }

    let pt_src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        blend_op: AC_SRC_OVER,
        blend_flags: 0,
        source_constant_alpha: 255,
        alpha_format: AC_SRC_ALPHA,
    };
    UpdateLayeredWindow(
        d.hwnd,
        d.hdc_screen,
        &d.pt_dst,
        &d.size,
        d.hdc_mem,
        &pt_src,
        0,
        &blend,
        ULW_ALPHA,
    );
}

// ---------------------------------------------------------------------------
// Splash window: borderless, chrome-less loading card
// ---------------------------------------------------------------------------
unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    _w_param: WPARAM,
    _l_param: LPARAM,
) -> LRESULT {
    match msg {
        // Advance the spinner and re-composite the single layered frame.
        WM_TIMER => {
            SPIN_ANGLE.fetch_add(14, Ordering::SeqCst);
            composite_splash(hwnd);
            0
        }
        WM_DESTROY => 0,
        _ => DefWindowProcW(hwnd, msg, _w_param, _l_param),
    }
}

struct Splash(HWND);

impl Splash {
    fn new() -> Self {
        unsafe {
            SetProcessDPIAware();
            let hinst = GetModuleHandleW(std::ptr::null_mut());
            let class: Vec<u16> = "DadoSplash"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let wnd: WNDCLASSW = WNDCLASSW {
                style: CS_DROPSHADOW,
                lpfn_wnd_proc: Some(wndproc),
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: hinst,
                h_icon: std::ptr::null_mut(),
                h_cursor: LoadCursorW(std::ptr::null_mut(), IDC_APPSTARTING as *const u16),
                hbr_background: std::ptr::null_mut(),
                lpsz_menu_name: std::ptr::null(),
                lpsz_class_name: class.as_ptr(),
            };
            if RegisterClassW(&wnd) == 0 {
                return Splash(std::ptr::null_mut());
            }
            let cx = GetSystemMetrics(SM_CXSCREEN);
            let cy = GetSystemMetrics(SM_CYSCREEN);
            // The off-screen DIB is a little larger than the visible card so the
            // card (drawn at CARD_X/CARD_Y) ends up centred on screen.
            let x = (cx - DIB_W) / 2;
            let y = (cy - DIB_H) / 2;
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                x,
                y,
                DIB_W,
                DIB_H,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinst,
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                return Splash(std::ptr::null_mut());
            }

            // Start GDI+ once for the splash's lifetime.
            let mut token = 0usize;
            let input = GdiplusStartupInput {
                gdiplus_version: 1,
                debug_event_callback: 0,
                suppress_background_thread: 0,
                suppress_external_codecs: 0,
            };
            if GdiplusStartup(&mut token, &input, std::ptr::null_mut()) != 0 {
                DestroyWindow(hwnd);
                return Splash(std::ptr::null_mut());
            }
            let _ = GDI_TOKEN.set(token);

            // Off-screen 32-bit ARGB DIB: the entire frame is composited here
            // and pushed atomically via UpdateLayeredWindow (no flicker).
            let hdc_screen = GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                GdiplusShutdown(token);
                DestroyWindow(hwnd);
                return Splash(std::ptr::null_mut());
            }
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                GdiplusShutdown(token);
                DestroyWindow(hwnd);
                return Splash(std::ptr::null_mut());
            }
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmi_header.bi_size = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmi_header.bi_width = DIB_W;
            bmi.bmi_header.bi_height = DIB_H;
            bmi.bmi_header.bi_planes = 1;
            bmi.bmi_header.bi_bit_count = 32;
            bmi.bmi_header.bi_compression = BI_RGB;
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbmp = CreateDIBSection(
                hdc_mem,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if hbmp.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                GdiplusShutdown(token);
                DestroyWindow(hwnd);
                return Splash(std::ptr::null_mut());
            }
            let hbmp_old = SelectObject(hdc_mem, hbmp);

            let mut g: *mut std::ffi::c_void = std::ptr::null_mut();
            if GdipCreateFromHDC(hdc_mem, &mut g) != 0 {
                SelectObject(hdc_mem, hbmp_old);
                DeleteObject(hbmp);
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                GdiplusShutdown(token);
                DestroyWindow(hwnd);
                return Splash(std::ptr::null_mut());
            }

            let data = SplashData {
                hwnd,
                hdc_screen,
                hdc_mem,
                hbmp,
                hbmp_old,
                g,
                pt_dst: POINT { x, y },
                size: SIZE {
                    cx: DIB_W,
                    cy: DIB_H,
                },
            };
            let boxed = Box::new(data);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);

            // Initial composite so the card is visible immediately, then the
            // timer keeps the spinner turning while extraction runs.
            composite_splash(hwnd);
            SetTimer(hwnd, 1, 30, std::ptr::null());
            ShowWindow(hwnd, SW_SHOWNORMAL);
            Splash(hwnd)
        }
    }

    fn hwnd(&self) -> HWND {
        self.0
    }
}

impl Drop for Splash {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                KillTimer(self.0, 1);
                let ptr = GetWindowLongPtrW(self.0, GWLP_USERDATA) as *mut SplashData;
                if !ptr.is_null() {
                    // Reclaim the boxed data so we can free its GDI handles.
                    let data = Box::from_raw(ptr);
                    GdipDeleteGraphics(data.g);
                    SelectObject(data.hdc_mem, data.hbmp_old);
                    DeleteObject(data.hbmp);
                    DeleteDC(data.hdc_mem);
                    ReleaseDC(std::ptr::null_mut(), data.hdc_screen);
                }
                DestroyWindow(self.0);
                if let Some(tok) = GDI_TOKEN.get() {
                    GdiplusShutdown(*tok);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Package parsing
// ---------------------------------------------------------------------------
struct Entry {
    name: String,
    raw_size: u64,
    raw_crc: u32,
    comp: Vec<u8>,
}

fn parse_blob(blob: &[u8]) -> Result<Vec<Entry>, String> {
    if blob.len() < 12 || &blob[0..8] != b"DADOBLOB" {
        return Err("Paket-Header ungültig".into());
    }
    let count = u32::from_le_bytes(blob[8..12].try_into().unwrap()) as usize;
    let mut pos = 12;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 2 > blob.len() {
            return Err("Paket abgeschnitten (Name-Länge)".into());
        }
        let name_len = u16::from_le_bytes([blob[pos], blob[pos + 1]]) as usize;
        pos += 2;
        if pos + name_len > blob.len() {
            return Err("Paket abgeschnitten (Name)".into());
        }
        let name = String::from_utf8_lossy(&blob[pos..pos + name_len]).into_owned();
        pos += name_len;
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains(['/', '\\'])
            || Path::new(&name).is_absolute()
        {
            return Err("Paket enthält einen ungültigen Dateinamen".into());
        }
        if pos + 20 > blob.len() {
            return Err("Paket abgeschnitten (Metadaten)".into());
        }
        let raw_size = u64::from_le_bytes(blob[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let raw_crc = u32::from_le_bytes(blob[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let comp_size = u64::from_le_bytes(blob[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let comp_size = usize::try_from(comp_size)
            .map_err(|_| "Paket zu groß für diese Plattform".to_string())?;
        let end = pos
            .checked_add(comp_size)
            .ok_or_else(|| "Paket-Grenzen ungültig".to_string())?;
        if end > blob.len() {
            return Err("Paket abgeschnitten (Daten)".into());
        }
        let comp = blob[pos..end].to_vec();
        pos = end;
        out.push(Entry {
            name,
            raw_size,
            raw_crc,
            comp,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------
fn runtime_dir(content_id: &str) -> PathBuf {
    if let Ok(v) = std::env::var("DADO_RUNTIME_DIR") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("TEMP"))
        .unwrap_or_else(|_| r"C:\Windows\Temp".to_string());
    PathBuf::from(base)
        .join("DadoPlayer")
        .join("runtime")
        .join(format!("rt-{content_id}"))
}

fn needs_extraction(entries: &[Entry], content_id: &str) -> bool {
    let dir = runtime_dir(content_id);
    entries.iter().any(|e| !cached_file_valid(&dir, e))
}

fn cached_file_valid(dir: &Path, entry: &Entry) -> bool {
    let path = dir.join(&entry.name);
    if std::fs::metadata(&path).map(|m| m.len()).ok() != Some(entry.raw_size) {
        return false;
    }
    std::fs::read(path).is_ok_and(|bytes| crc32(&bytes) == entry.raw_crc)
}

fn atomic_replace(tmp: &Path, dest: &Path) -> Result<(), String> {
    match std::fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            // Cross-volume fallback: copy then drop the temp copy.
            std::fs::copy(tmp, dest).map_err(|copy_err| {
                format!(
                    "Ersetzen von {} fehlgeschlagen (Umbenennen: {}; Kopieren: {})",
                    dest.display(),
                    rename_err,
                    copy_err
                )
            })?;
            std::fs::remove_file(tmp).map_err(|cleanup_err| {
                format!(
                    "Temporäre Datei {} konnte nach dem Kopieren nicht gelöscht werden: {}",
                    tmp.display(),
                    cleanup_err
                )
            })
        }
    }
}

fn ensure_extracted(entries: &[Entry], content_id: &str) -> Result<PathBuf, String> {
    let dir = runtime_dir(content_id);
    let need_extract = entries.iter().any(|e| !cached_file_valid(&dir, e));

    if need_extract {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Kann Laufzeitverzeichnis nicht anlegen: {e}"))?;
        for e in entries {
            let dest = dir.join(&e.name);
            // Reuse an already-correct file.
            if dest.exists() {
                if let Ok(bytes) = std::fs::read(&dest) {
                    if bytes.len() as u64 == e.raw_size && crc32(&bytes) == e.raw_crc {
                        continue;
                    }
                }
                let _ = std::fs::remove_file(&dest);
            }
            let raw = decompress_to_vec_zlib(&e.comp)
                .map_err(|err| format!("Dekompression von {} fehlgeschlagen: {:?}", e.name, err))?;
            if crc32(&raw) != e.raw_crc {
                return Err(format!("CRC-Prüfung für {} fehlgeschlagen", e.name));
            }
            let pid = std::process::id();
            let tmp = dir.join(format!("{}.tmp-{}", e.name, pid));
            let mut fh = std::fs::File::create(&tmp)
                .map_err(|e| format!("Schreiben von {} fehlgeschlagen: {}", tmp.display(), e))?;
            fh.write_all(&raw)
                .map_err(|e| format!("Schreiben von {} fehlgeschlagen: {}", tmp.display(), e))?;
            fh.flush()
                .map_err(|e| format!("Speichern von {} fehlgeschlagen: {}", tmp.display(), e))?;
            drop(fh);
            atomic_replace(&tmp, &dest)?;
        }
    }

    Ok(dir.join("Dado-Player.exe"))
}

// ---------------------------------------------------------------------------
// Error dialog
// ---------------------------------------------------------------------------
fn show_error(msg: &str) {
    unsafe {
        let title: Vec<u16> = "Dado Player"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let text: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_ICONERROR | MB_OK,
        );
    }
}

// ---------------------------------------------------------------------------
// App launch
// ---------------------------------------------------------------------------
fn spawn_app(payload: &Path) -> Result<(), String> {
    let stage = tempfile::Builder::new()
        .prefix("dado-update-")
        .tempdir()
        .map_err(|e| format!("Temporärer Ordner: {e}"))?;
    let status = std::process::Command::new(payload)
        .env("DADO_UPDATE_DIR", stage.path())
        .current_dir(payload.parent().unwrap())
        .status()
        .map_err(|e| format!("Player starten: {e}"))?;
    let update = stage.path().join("verified.exe");
    if status.success() && update.is_file() {
        install_update(&update).map_err(|e| format!("Portables Update: {e}"))?;
    }
    drop(stage);
    std::process::exit(status.code().unwrap_or(1))
}

fn install_update(update: &Path) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let parent = exe.parent().unwrap();
    // Copy to the destination volume before attempting to rename the running launcher.
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut std::fs::File::open(update)?, staged.as_file_mut())?;
    staged.as_file().sync_all()?;
    let backup = exe.with_extension("exe.previous");
    std::fs::rename(&exe, &backup)?;
    if let Err(error) = staged.persist_noclobber(&exe) {
        std::fs::rename(&backup, &exe)?;
        return Err(error.error);
    }
    if let Err(error) = std::process::Command::new(&exe).spawn() {
        std::fs::rename(&backup, &exe)?;
        return Err(error);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------
fn run() -> Result<(), String> {
    // Decode the embedded logo once, before any painting.
    let _ = LOGO.set(decode_logo());

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let data = std::fs::read(&exe).map_err(|e| format!("lesen {}: {e}", exe.display()))?;
    if data.len() < 64 {
        return Err("Launcher ist beschädigt (zu klein)".into());
    }

    let footer_at = package_footer::footer_offset(&data)?;
    let footer = &data[footer_at..footer_at + 64];
    if &footer[0..8] != b"DADOPK01" {
        return Err("Kein eingebettetes Paket gefunden (falscher Launcher).".into());
    }
    let blob_offset = u64::from_le_bytes(footer[8..16].try_into().unwrap());
    let blob_len = u64::from_le_bytes(footer[16..24].try_into().unwrap());
    let content_id = String::from_utf8_lossy(&footer[28..52]).trim().to_string();
    let crc_stored = u32::from_le_bytes(footer[60..64].try_into().unwrap());
    let crc_calc = crc32(&footer[0..60]);
    if crc_calc != crc_stored {
        return Err("Paket-Prüfsumme ungültig (beschädigter Download?).".into());
    }
    let blob_offset =
        usize::try_from(blob_offset).map_err(|_| "Paket-Grenzen ungültig".to_string())?;
    let blob_len = usize::try_from(blob_len).map_err(|_| "Paket-Grenzen ungültig".to_string())?;
    let end = blob_offset
        .checked_add(blob_len)
        .ok_or_else(|| "Paket-Grenzen ungültig".to_string())?;
    if end != footer_at || blob_offset > footer_at {
        return Err("Paket-Grenzen ungültig".into());
    }

    let blob = &data[blob_offset..end];
    let entries = parse_blob(blob)?;

    let needed = needs_extraction(&entries, &content_id);
    let result: Arc<Mutex<Option<Result<PathBuf, String>>>> = Arc::new(Mutex::new(None));

    if needed {
        let splash = Splash::new();
        if splash.hwnd().is_null() {
            // Splash unavailable: extract synchronously, no animation.
            let payload = ensure_extracted(&entries, &content_id)?;
            return spawn_app(&payload);
        }
        let hwnd_usize = splash.hwnd() as usize;
        let cid = content_id.clone();
        let rb = result.clone();
        let handle = thread::spawn(move || {
            let r = ensure_extracted(&entries, &cid);
            *rb.lock().unwrap() = Some(r);
            unsafe {
                PostMessageW(hwnd_usize as HWND, WM_USER_DONE, 0, 0);
            }
        });
        // Message loop: keep painting the spinner until extraction finishes.
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            let r = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
            if r == 0 || r == -1 {
                break;
            }
            if msg.message == WM_USER_DONE {
                break;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let _ = handle.join();
        drop(splash);
    }

    let payload = match result.lock().unwrap().take() {
        Some(r) => r?,
        None => runtime_dir(&content_id).join("Dado-Player.exe"),
    };
    spawn_app(&payload)
}

fn main() {
    if let Err(msg) = run() {
        show_error(&msg);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod update_tests {
    #[test]
    fn gdiplus_uses_pointer_sized_token_without_overwriting_neighbors() {
        #[repr(C)]
        struct Guard {
            before: usize,
            token: usize,
            after: usize,
        }
        let mut guard = Guard {
            before: 0x1357_2468,
            token: 0,
            after: 0x2468_1357,
        };
        let input = super::GdiplusStartupInput {
            gdiplus_version: 1,
            debug_event_callback: 0,
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        unsafe {
            assert_eq!(
                super::GdiplusStartup(&mut guard.token, &input, std::ptr::null_mut()),
                0
            );
            assert_ne!(guard.token, 0);
            assert_eq!(guard.before, 0x1357_2468);
            assert_eq!(guard.after, 0x2468_1357);
            super::GdiplusShutdown(guard.token);
        }
    }
    #[test]
    fn detects_same_size_corruption_in_cached_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let entry = super::Entry {
            name: "runtime.dll".into(),
            raw_size: 4,
            raw_crc: super::crc32(b"good"),
            comp: Vec::new(),
        };
        let path = dir.path().join(&entry.name);
        std::fs::write(&path, b"good").unwrap();
        assert!(super::cached_file_valid(dir.path(), &entry));
        std::fs::write(&path, b"evil").unwrap();
        assert!(!super::cached_file_valid(dir.path(), &entry));
    }
    use super::*;
    use std::process::Command;

    // Run the production replacement routine against an executing copy of this
    // harness, so Windows image locks and the original filename are exercised.
    #[test]
    fn update_child() {
        if let Some(candidate) = std::env::var_os("DADO_TEST_CANDIDATE") {
            let result = install_update(Path::new(&candidate));
            assert_eq!(
                result.is_ok(),
                std::env::var_os("DADO_EXPECT_SUCCESS").is_some()
            );
        }
    }

    #[test]
    fn replaces_a_running_launcher_and_restores_it_on_restart_failure() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Dado Player.exe");
        let candidate = dir.path().join("candidate.exe");
        let marker = dir.path().join("started.txt");
        let original = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        std::fs::write(&exe, &original).unwrap();
        std::fs::write(&candidate, b"invalid executable for rollback test").unwrap();
        let child = |success: bool| {
            let mut command = Command::new(&exe);
            command
                .args(["--exact", "update_tests::update_child"])
                .env("DADO_TEST_CANDIDATE", &candidate)
                .env("DADO_TEST_MARKER", &marker);
            if success {
                command.env("DADO_EXPECT_SUCCESS", "1");
            }
            assert!(command.status().unwrap().success());
        };
        child(false);
        assert_eq!(std::fs::read(&exe).unwrap(), original);
        let source = dir.path().join("replacement.rs");
        std::fs::write(&source, r#"fn main(){std::fs::write(std::env::var_os("DADO_TEST_MARKER").unwrap(),b"started").unwrap();}"#).unwrap();
        assert!(Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&candidate)
            .status()
            .unwrap()
            .success());
        child(true);
        for _ in 0..100 {
            if marker.is_file() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(marker.is_file());
        assert_eq!(
            std::fs::read(exe.with_extension("exe.previous")).unwrap(),
            original
        );
        assert_eq!(
            std::fs::read(exe).unwrap(),
            std::fs::read(candidate).unwrap()
        );
    }
}
