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
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, SystemParametersInfoW, MB_ICONERROR, MB_OK, SPI_GETWORKAREA,
};

/// The dispatcher passes the path of a "first frame rendered" marker file to
/// its child through this environment variable.
const READY_ENV: &str = "DADO_READY_FILE";

pub fn run() -> eframe::Result<()> {
    // ------------------------------------------------------------------
    // Graphics backend dispatcher.
    //
    // No single GPU API is available on every Windows machine (driverless VMs,
    // RDP sessions, virtual display adapters, broken OpenGL ICDs, no Vulkan
    // loader, ...). We therefore try Direct3D 12 -> Vulkan -> OpenGL, each in a
    // *fresh process* because winit cannot recreate its event loop in-process.
    // The first backend that draws a frame wins; an error dialog appears only
    // if all of them failed. Nothing here depends on a particular driver or on
    // any bundled software rasterizer - all three use the system's own APIs.
    //
    // `DADO_STAGE` marks the child process that runs one backend.
    // `DADO_RENDERER` (glow|wgpu|vulkan|dx12) forces one backend, no chain.
    // `DADO_READY_FILE` is the private launcher -> child handshake.
    // ------------------------------------------------------------------
    if std::env::var_os("DADO_STAGE").is_none() {
        return run_dispatcher();
    }
    run_backend()
}

/// Try every graphics backend in turn, each as a child process.
fn run_dispatcher() -> eframe::Result<()> {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        // If we cannot resolve our own path we cannot spawn children; just try
        // the default backend in-process.
        Err(_) => return run_backend(),
    };

    // A forced backend skips the automatic chain.
    let stages: Vec<&str> = match std::env::var("DADO_RENDERER").ok().as_deref() {
        Some("glow") => vec!["glow"],
        Some("wgpu") | Some("dx12") => vec!["dx12"],
        Some("vulkan") => vec!["vulkan"],
        // Direct3D 12 first: it is the hardware-accelerated path on Windows and
        // also has a guaranteed software fallback (WARP). Vulkan is the second
        // GPU path, OpenGL only the last resort (it needs a working driver).
        Some(_) | None => vec!["dx12", "vulkan", "glow"],
    };
    let tried = stages
        .iter()
        .map(|s| match *s {
            "dx12" => "Direct3D 12",
            "vulkan" => "Vulkan",
            "glow" => "OpenGL",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(", ");

    // Each launch gets a fresh log. wgpu logs a line per frame at debug level,
    // so an append-only log would otherwise grow by megabytes per minute and
    // slow down the GUI on every machine. `read_log_tail` still sees everything
    // the current launch produced.
    if let Some(dir) = exe.parent() {
        let _ = std::fs::remove_file(dir.join("Dado-Player-startup.log"));
    }

    // Verify the bundled MPV runtime once here, so a missing DLL is reported
    // clearly instead of being retried for every graphics backend.
    if std::env::var("IPTV_SELFTEST").is_err() {
        if let Err(message) = crate::mpv::check_library_available() {
            show_startup_error(&format!(
                "MPV-Laufzeit konnte nicht geladen werden:\n{message}"
            ));
            return Ok(());
        }
    }

    // The child writes this marker on its first rendered egui frame. It lives
    // in the temp directory so it stays writable even when the EXE is installed
    // under a read-only "C:\Program Files". "Process is still alive after N
    // seconds" is not a reliable start signal: a hung GPU stack or a backend
    // that dies just after the window appears both look identical to a working
    // app. The marker makes the fallback deterministic on every machine.
    let ready_file =
        std::env::temp_dir().join(format!("Dado-Player-ready-{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&ready_file);
    const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
    const POLL: std::time::Duration = std::time::Duration::from_millis(200);

    let mut last_log = String::new();
    for stage in &stages {
        let _ = std::fs::remove_file(&ready_file);
        let mut cmd = std::process::Command::new(&exe);
        cmd.env("DADO_STAGE", stage).env(READY_ENV, &ready_file);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                last_log = read_log_tail(&exe);
                continue;
            }
        };

        let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
        let mut started = false;
        loop {
            if ready_file.exists() {
                started = true;
                break;
            }
            match child.try_wait() {
                // Exited before the first frame -> this backend could not start.
                Ok(Some(_)) => break,
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        // Safety net: if the marker could not be written but the
                        // process is alive, keep it instead of killing a working
                        // window on a locked-down machine.
                        started = true;
                        break;
                    }
                    std::thread::sleep(POLL);
                }
                Err(_) => break,
            }
        }

        if started {
            // Exit code 0 = the user closed the window. Anything else is a
            // crash; try the next backend rather than silently disappearing.
            match child.wait() {
                Ok(status) if status.success() => {
                    let _ = std::fs::remove_file(&ready_file);
                    return Ok(());
                }
                _ => append_log(
                    &exe,
                    &format!("Backend {stage} stopped after rendering; trying the next backend"),
                ),
            }
        } else {
            // Failed or hung before it could draw a frame.
            let _ = child.kill();
            let _ = child.wait();
        }
        last_log = read_log_tail(&exe);
    }

    let _ = std::fs::remove_file(&ready_file);
    // Every backend failed.
    let detail = if last_log.trim().is_empty() {
        "Keine Details verfügbar.".to_string()
    } else {
        last_log.trim().to_string()
    };
    show_startup_error(&format!(
        "Dado-Player konnte keine Grafik-Schnittstelle starten.\n\nGetestet: {tried}.\n\nLetzter Fehler:\n{detail}\n\nVollständiges Protokoll: Dado-Player-startup.log neben der EXE."
    ));
    Ok(())
}

/// Run the GUI with the single backend named by `DADO_STAGE`.
fn run_backend() -> eframe::Result<()> {
    let stage = std::env::var("DADO_STAGE").unwrap_or_else(|_| "dx12".to_string());

    // Keep a small diagnostic log beside the EXE so a startup failure is
    // actionable even though a GUI-subsystem binary has no console.
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("Dado-Player-startup.log")));
    let log = log_path
        .clone()
        .map(|p| std::sync::Arc::new(std::sync::Mutex::new(p)));
    startup_log(&log, &format!("Dado-Player startup begins (stage={stage})"));

    // wgpu/winit report the *real* reason a backend failed only through `log`
    // (e.g. the DXGI HRESULT behind "Invalid surface"). Without a logger that
    // information is thrown away, so route it into the startup log.
    if let Some(path) = log_path {
        if log::set_boxed_logger(Box::new(FileLogger {
            path: std::sync::Mutex::new(path),
        }))
        .is_ok()
        {
            // Debug-level wgpu logging is extremely verbose (one line per
            // frame). Keep it opt-in via DADO_DEBUG=1; warnings and errors are
            // always captured, which is all a backend failure needs.
            log::set_max_level(if std::env::var("DADO_DEBUG").is_ok() {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Warn
            });
        }
    }

    // In backend mode a startup panic just ends this attempt (exit code 2) so
    // the dispatcher can try the next backend; the dispatcher reports the final
    // failure to the user with the log tail.
    let panic_log = log.clone();
    let panic_stage = stage.clone();
    std::panic::set_hook(Box::new(move |panic| {
        let detail = if let Some(s) = panic.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = panic.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unbekannter Fehler".to_string()
        };
        let location = panic
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unbekannt".to_string());
        if let Some(log) = panic_log.as_ref() {
            write_startup_log(
                log,
                &format!("Startup panic (stage={panic_stage}, {location}): {detail}"),
            );
        }
        std::process::exit(2);
    }));

    if std::env::var("IPTV_SELFTEST").is_ok() {
        run_selftest();
        return Ok(());
    }

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            startup_log(&log, &format!("Tokio runtime failed: {e}"));
            std::process::exit(5);
        }
    };
    startup_log(&log, "Tokio runtime ready; creating native window");

    let mut viewport = initial_viewport();
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let (renderer, backends) = backend_for_stage(&stage);
    startup_log(
        &log,
        &format!("Starting GUI stage={stage} renderer={renderer:?} backends={backends:?}"),
    );
    let mut options = eframe::NativeOptions {
        renderer,
        viewport,
        // The MPV child HWND is independent from this top-level UI renderer.
        ..Default::default()
    };
    if matches!(renderer, eframe::Renderer::Wgpu) {
        options.wgpu_options.supported_backends = backends;
    }
    // The wrapper signals the dispatcher once the first frame is actually
    // drawn. All other `eframe::App` methods keep their defaults because the
    // inner app only overrides `update`.
    let ready_file = std::env::var_os(READY_ENV).map(std::path::PathBuf::from);
    match eframe::run_native(
        "Dado-Player",
        options,
        Box::new(move |_cc| {
            Box::new(StartupApp {
                inner: app::App::new(rt),
                ready: ready_file,
            })
        }),
    ) {
        Ok(()) => {
            startup_log(&log, "Native window closed normally");
            Ok(())
        }
        Err(e) => {
            startup_log(
                &log,
                &format!("Native GUI startup failed (stage={stage}): {e}"),
            );
            std::process::exit(4);
        }
    }
}

/// Map a stage name to the eframe renderer and wgpu backends to use.
#[cfg(target_os = "windows")]
fn backend_for_stage(stage: &str) -> (eframe::Renderer, wgpu::Backends) {
    match stage {
        "glow" => (eframe::Renderer::Glow, wgpu::Backends::empty()),
        "vulkan" => (eframe::Renderer::Wgpu, wgpu::Backends::VULKAN),
        _ => (eframe::Renderer::Wgpu, wgpu::Backends::DX12),
    }
}

/// Read the last few non-empty lines of the startup log (for the error dialog).
fn read_log_tail(exe: &std::path::Path) -> String {
    let Some(dir) = exe.parent() else {
        return String::new();
    };
    let path = dir.join("Dado-Player-startup.log");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return String::new();
    };
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(6);
    lines[start..].join("\n")
}

/// Appends every `log` record (wgpu, winit, our own code) to the startup log.
struct FileLogger {
    path: std::sync::Mutex<std::path::PathBuf>,
}

impl log::Log for FileLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let line = format!(
            "[{}] {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
        if let Ok(path) = self.path.lock() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&*path)
            {
                let _ = writeln!(
                    f,
                    "{}  {line}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
    }

    fn flush(&self) {}
}

fn startup_log(log: &Option<std::sync::Arc<std::sync::Mutex<std::path::PathBuf>>>, line: &str) {
    if let Some(log) = log.as_ref() {
        write_startup_log(log, line);
    }
}

/// Append one line from the dispatcher process (which has no logger installed).
fn append_log(exe: &std::path::Path, line: &str) {
    let Some(dir) = exe.parent() else {
        return;
    };
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("Dado-Player-startup.log"))
    {
        let _ = writeln!(
            f,
            "{}  {line}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
    }
}

/// Signals the parent dispatcher when the very first frame has been drawn.
///
/// A GPU backend can fail during surface configuration, adapter selection or
/// shader compilation - all *before* egui draws anything. Watching for this
/// marker is therefore a much better "did it start?" test than watching the
/// process, and it works identically on fast and slow machines.
struct StartupApp {
    inner: app::App,
    ready: Option<std::path::PathBuf>,
}

impl eframe::App for StartupApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(path) = self.ready.take() {
            let _ = std::fs::write(path, b"ready");
        }
        eframe::App::update(&mut self.inner, ctx, frame);
    }
}

fn write_startup_log(log: &std::sync::Arc<std::sync::Mutex<std::path::PathBuf>>, line: &str) {
    if let Ok(path) = log.lock() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&*path)
        {
            let _ = writeln!(
                f,
                "{}  {line}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
}

/// Show an actionable error even in a GUI-subsystem build (no console).
fn show_startup_error(message: &str) {
    let title: Vec<u16> = "Dado-Player — Startfehler"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
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
    let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0) };
    if ok == 0 {
        return fallback;
    }
    let scale = system_dpi() as f32 / 96.0;
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
    let h = (wh - margin * 2.0 - frame).clamp(480.0, 880.0);
    let x = (r.left as f32 / scale + (ww - w) * 0.5).max(0.0);
    let y = (r.top as f32 / scale + (wh - h - frame) * 0.5).max(0.0);
    egui::ViewportBuilder::default()
        .with_inner_size([w, h])
        .with_min_inner_size([w.min(880.0), h.min(540.0)])
        .with_position([x, y])
}

/// Resolve `GetDpiForSystem` at runtime instead of importing it statically.
///
/// That API only exists on Windows 10 1607+. A direct `windows-sys` call puts
/// it into the PE import table, so on older Windows the *loader* refuses to
/// start the EXE at all ("procedure entry point not found") - before any of our
/// backend fallback can run. Older systems fall back to the 96 DPI default.
fn system_dpi() -> u32 {
    type GetDpiForSystemFn = unsafe extern "system" fn() -> u32;
    if let Ok(lib) = unsafe { libloading::Library::new("user32.dll") } {
        if let Ok(f) = unsafe { lib.get::<GetDpiForSystemFn>(b"GetDpiForSystem\0") } {
            let dpi = unsafe { f() };
            if dpi > 0 {
                return dpi;
            }
        }
    }
    96
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
    match rt.block_on(client::load_profile(&p, 25, "ts")) {
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
