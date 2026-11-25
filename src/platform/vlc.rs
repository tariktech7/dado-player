//! LibVLC 3 backend shared by Windows and Android.
use crate::types::{DesktopAspect, Settings};
use libloading::Library;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
#[cfg(windows)]
use std::ptr::null;
use std::ptr::NonNull;
use std::time::{Duration, Instant};

type Handle = *mut c_void;

#[repr(C)]
struct TrackDescription {
    id: c_int,
    name: *const c_char,
    next: *mut TrackDescription,
}

macro_rules! api {
    ($($field:ident: $symbol:ident => fn($($arg:ty),*) $(-> $ret:ty)?;)+) => {
        struct Api {
            $($field: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)+
            _library: Library,
        }
        impl Api {
            fn new(library: Library) -> Result<Self, libloading::Error> {
                Ok(Self {
                    $($field: unsafe { *library.get(concat!(stringify!($symbol), "\0").as_bytes())? },)+
                    _library: library,
                })
            }
        }
    };
}

api! {
    new: libvlc_new => fn(c_int, *const *const c_char) -> Handle;
    release: libvlc_release => fn(Handle);
    player_new: libvlc_media_player_new => fn(Handle) -> Handle;
    player_release: libvlc_media_player_release => fn(Handle);
    media_new: libvlc_media_new_location => fn(Handle, *const c_char) -> Handle;
    media_option: libvlc_media_add_option => fn(Handle, *const c_char);
    media_release: libvlc_media_release => fn(Handle);
    set_media: libvlc_media_player_set_media => fn(Handle, Handle);
    play: libvlc_media_player_play => fn(Handle) -> c_int;
    stop: libvlc_media_player_stop => fn(Handle);
    pause: libvlc_media_player_set_pause => fn(Handle, c_int);
    get_time: libvlc_media_player_get_time => fn(Handle) -> i64;
    set_time: libvlc_media_player_set_time => fn(Handle, i64);
    get_length: libvlc_media_player_get_length => fn(Handle) -> i64;
    get_state: libvlc_media_player_get_state => fn(Handle) -> c_int;
    seekable: libvlc_media_player_is_seekable => fn(Handle) -> c_int;
    fps: libvlc_media_player_get_fps => fn(Handle) -> f32;
    size: libvlc_video_get_size => fn(Handle, c_uint, *mut c_uint, *mut c_uint) -> c_int;
    set_rate: libvlc_media_player_set_rate => fn(Handle, f32) -> c_int;
    set_volume: libvlc_audio_set_volume => fn(Handle, c_int) -> c_int;
    audio: libvlc_audio_get_track => fn(Handle) -> c_int;
    sub: libvlc_video_get_spu => fn(Handle) -> c_int;
    set_audio: libvlc_audio_set_track => fn(Handle, c_int) -> c_int;
    set_sub: libvlc_video_set_spu => fn(Handle, c_int) -> c_int;
    set_video: libvlc_video_set_track => fn(Handle, c_int) -> c_int;
    audio_tracks: libvlc_audio_get_track_description => fn(Handle) -> *mut TrackDescription;
    sub_tracks: libvlc_video_get_spu_description => fn(Handle) -> *mut TrackDescription;
    free_tracks: libvlc_track_description_list_release => fn(*mut TrackDescription);
    aspect: libvlc_video_set_aspect_ratio => fn(Handle, *const c_char);
    crop: libvlc_video_set_crop_geometry => fn(Handle, *const c_char);
    mouse: libvlc_video_set_mouse_input => fn(Handle, c_uint);
    key: libvlc_video_set_key_input => fn(Handle, c_uint);
}

pub(crate) struct Vlc {
    api: Api,
    instance: NonNull<c_void>,
    player: NonNull<c_void>,
    options: RefCell<Vec<CString>>,
    pending_seek: Cell<Option<(f64, Instant)>>,
    video_disabled: Cell<bool>,
    paused: Cell<bool>,
    pending_volume: Cell<Option<c_int>>,
    aspect: Cell<DesktopAspect>,
    window_ratio: Cell<Option<(u32, u32)>>,
    #[cfg(target_os = "android")]
    frames: Box<frames::Frames>,
    #[cfg(target_os = "android")]
    texture: RefCell<Option<egui::TextureHandle>>,
}

impl Vlc {
    pub(crate) fn new(window: isize, settings: &Settings) -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        let library = {
            let executable = std::env::current_exe().map_err(|err| err.to_string())?;
            let runtime = executable.parent().ok_or("VLC: fehlender Programmpfad")?;
            if !runtime.join("plugins/access/libhttp_plugin.dll").is_file() {
                return Err("VLC: Die mitgelieferten Plugins fehlen.".into());
            }
            // Search for libvlccore next to the bundled libvlc, then system DLLs.
            unsafe {
                libloading::os::windows::Library::load_with_flags(
                    runtime.join("libvlc.dll"),
                    0x00000100 | 0x00001000,
                )
            }
            .map(Library::from)
        };
        #[cfg(target_os = "android")]
        let library = {
            let _ = window;
            crate::platform::android::jni_bridge::prepare_vlc()?;
            unsafe { Library::new("libvlc.so") }
        };
        let api = library
            .and_then(Api::new)
            .map_err(|err| format!("LibVLC: {err}"))?;
        let args = [
            c"--no-video-title-show".as_ptr(),
            c"--no-osd".as_ptr(),
            c"--ignore-config".as_ptr(),
        ];
        let instance = NonNull::new(unsafe { (api.new)(args.len() as c_int, args.as_ptr()) })
            .ok_or("LibVLC konnte nicht initialisiert werden.")?;
        let Some(player) = NonNull::new(unsafe { (api.player_new)(instance.as_ptr()) }) else {
            unsafe { (api.release)(instance.as_ptr()) };
            return Err("VLC-Player konnte nicht erstellt werden.".into());
        };
        let this = Self {
            api,
            instance,
            player,
            options: RefCell::new(Vec::new()),
            pending_seek: Cell::new(None),
            video_disabled: Cell::new(false),
            paused: Cell::new(false),
            pending_volume: Cell::new(None),
            aspect: Cell::new(DesktopAspect::Auto),
            window_ratio: Cell::new(None),
            #[cfg(target_os = "android")]
            frames: Box::default(),
            #[cfg(target_os = "android")]
            texture: RefCell::new(None),
        };
        #[cfg(target_os = "windows")]
        unsafe {
            type SetWindow = unsafe extern "C" fn(Handle, Handle);
            let set_window = this
                .api
                ._library
                .get::<SetWindow>(b"libvlc_media_player_set_hwnd\0")
                .map_err(|err| err.to_string())?;
            set_window(this.ptr(), window as Handle);
        }
        #[cfg(target_os = "android")]
        this.frames.attach(&this.api._library, this.ptr())?;
        unsafe {
            (this.api.mouse)(this.ptr(), 0);
            (this.api.key)(this.ptr(), 0);
        }
        this.configure(settings);
        Ok(this)
    }

    fn ptr(&self) -> Handle {
        self.player.as_ptr()
    }

    pub(crate) fn configure(&self, settings: &Settings) {
        let mut options =
            vec![CString::new(format!(":network-caching={}", settings.cache_secs * 1000)).unwrap()];
        if !settings.user_agent.is_empty() {
            if let Ok(agent) = CString::new(format!(":http-user-agent={}", settings.user_agent)) {
                options.push(agent);
            }
        }
        #[cfg(target_os = "android")]
        options.push(c":avcodec-hw=none".to_owned());
        *self.options.borrow_mut() = options;
        self.set_desktop_aspect(settings.desktop_aspect);
    }

    pub(crate) fn load_at(&self, url: &str, seconds: f64) {
        self.stop();
        #[cfg(target_os = "android")]
        self.texture.borrow_mut().take();
        let Ok(url) = CString::new(url) else {
            return;
        };
        let media = unsafe { (self.api.media_new)(self.instance.as_ptr(), url.as_ptr()) };
        if media.is_null() {
            return;
        }
        unsafe {
            for option in self.options.borrow().iter() {
                (self.api.media_option)(media, option.as_ptr());
            }
            if self.video_disabled.get() {
                (self.api.media_option)(media, c":no-video".as_ptr());
            }
            (self.api.set_media)(self.ptr(), media);
            (self.api.media_release)(media);
            (self.api.play)(self.ptr());
        }
        self.apply_aspect();
        if seconds > 0.0 {
            self.seek_absolute(seconds);
        }
    }

    pub(crate) fn stop(&self) {
        self.pending_seek.set(None);
        self.paused.set(false);
        unsafe {
            (self.api.stop)(self.ptr());
        }
        #[cfg(target_os = "android")]
        self.frames.clear();
    }
    pub(crate) fn disable_video(&self) {
        self.video_disabled.set(true);
        unsafe {
            (self.api.set_video)(self.ptr(), -1);
        }
    }
    pub(crate) fn enable_video(&self) {
        self.video_disabled.set(false);
    }
    pub(crate) fn set_pause(&self, paused: bool) {
        self.paused.set(paused);
        unsafe {
            (self.api.pause)(self.ptr(), paused as c_int);
        }
    }
    pub(crate) fn toggle_pause(&self) {
        self.set_pause(!self.paused());
    }
    pub(crate) fn paused(&self) -> bool {
        self.paused.get()
    }
    pub(crate) fn set_volume(&self, volume: i64) {
        self.pending_volume.set(Some(volume.clamp(0, 100) as c_int));
        self.apply_volume();
    }

    fn apply_volume(&self) {
        if let Some(volume) = self.pending_volume.get() {
            if unsafe { (self.api.set_volume)(self.ptr(), volume) } == 0 {
                self.pending_volume.set(None);
            }
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn set_speed(&self, speed: f32) {
        unsafe {
            (self.api.set_rate)(self.ptr(), speed);
        }
    }
    pub(crate) fn duration(&self) -> f64 {
        unsafe { (self.api.get_length)(self.ptr()) }.max(0) as f64 / 1000.0
    }
    pub(crate) fn position(&self) -> f64 {
        self.apply_volume();
        if let Some((seconds, requested)) = self.pending_seek.get() {
            if requested.elapsed() > Duration::from_secs(10) {
                self.pending_seek.set(None);
            } else if unsafe { (self.api.seekable)(self.ptr()) } != 0 {
                self.pending_seek.set(None);
                unsafe {
                    (self.api.set_time)(self.ptr(), (seconds * 1000.0) as i64);
                }
                return seconds;
            } else {
                return seconds;
            }
        }
        unsafe { (self.api.get_time)(self.ptr()) }.max(0) as f64 / 1000.0
    }
    pub(crate) fn seek_absolute(&self, seconds: f64) {
        if seconds.is_finite() {
            self.pending_seek
                .set(Some((seconds.max(0.0), Instant::now())));
        }
    }
    pub(crate) fn seek_relative(&self, seconds: f64) {
        let base = self
            .pending_seek
            .get()
            .map_or_else(|| self.position(), |(pos, _)| pos);
        self.seek_absolute(base + seconds);
    }
    pub(crate) fn set_desktop_aspect(&self, aspect: DesktopAspect) {
        self.aspect.set(aspect);
        self.apply_aspect();
    }
    pub(crate) fn set_video_size(&self, width: u32, height: u32) {
        let ratio = (width > 0 && height > 0).then_some((width, height));
        if self.window_ratio.replace(ratio) != ratio {
            self.apply_aspect();
        }
    }
    fn apply_aspect(&self) {
        #[cfg(target_os = "windows")]
        {
            let ratio = self
                .window_ratio
                .get()
                .and_then(|(w, h)| CString::new(format!("{w}:{h}")).ok());
            let ratio = ratio.as_ref().map_or(null(), |v| v.as_ptr());
            let mode = self.aspect.get();
            unsafe {
                (self.api.aspect)(
                    self.ptr(),
                    if mode == DesktopAspect::Stretch {
                        ratio
                    } else {
                        null()
                    },
                );
                (self.api.crop)(
                    self.ptr(),
                    if mode == DesktopAspect::Fill {
                        ratio
                    } else {
                        null()
                    },
                );
            }
        }
    }
    pub(crate) fn cache_time(&self) -> f64 {
        0.0
    }
    pub(crate) fn fps(&self) -> f64 {
        unsafe { (self.api.fps)(self.ptr()) }.max(0.0) as f64
    }
    fn dimensions(&self) -> (i64, i64) {
        let (mut width, mut height) = (0, 0);
        unsafe {
            (self.api.size)(self.ptr(), 0, &mut width, &mut height);
        }
        (width.into(), height.into())
    }
    pub(crate) fn width(&self) -> i64 {
        self.dimensions().0
    }
    pub(crate) fn height(&self) -> i64 {
        self.dimensions().1
    }
    pub(crate) fn video_codec(&self) -> String {
        String::new()
    }
    pub(crate) fn audio_codec(&self) -> String {
        String::new()
    }
    pub(crate) fn container(&self) -> String {
        String::new()
    }
    pub(crate) fn error(&self) -> bool {
        // libvlc_Error from the stable LibVLC 3 public state enum.
        unsafe { (self.api.get_state)(self.ptr()) == 7 }
    }
    pub(crate) fn current_audio(&self) -> i64 {
        unsafe { (self.api.audio)(self.ptr()) }.into()
    }
    pub(crate) fn current_sub(&self) -> i64 {
        unsafe { (self.api.sub)(self.ptr()) }.into()
    }
    pub(crate) fn set_audio_track(&self, id: i64) {
        unsafe {
            (self.api.set_audio)(self.ptr(), id as c_int);
        }
    }
    pub(crate) fn set_sub_track(&self, id: i64) {
        unsafe {
            (self.api.set_sub)(self.ptr(), id as c_int);
        }
    }
    pub(crate) fn disable_sub(&self) {
        self.set_sub_track(-1);
    }
    pub(crate) fn track_list_json(&self) -> Option<String> {
        let mut tracks = Vec::new();
        for (kind, query) in [
            ("audio", self.api.audio_tracks),
            ("sub", self.api.sub_tracks),
        ] {
            let head = unsafe { query(self.ptr()) };
            let mut next = head;
            while let Some(track) = unsafe { next.as_ref() } {
                if track.id >= 0 && !track.name.is_null() {
                    let title = unsafe { CStr::from_ptr(track.name) }.to_string_lossy();
                    tracks.push(serde_json::json!({"type": kind, "id": track.id, "title": title}));
                }
                next = track.next;
            }
            if !head.is_null() {
                unsafe {
                    (self.api.free_tracks)(head);
                }
            }
        }
        serde_json::to_string(&tracks).ok()
    }

    #[cfg(target_os = "android")]
    pub(crate) fn paint(&self, ui: &egui::Ui, viewport: egui::Rect) {
        let mut texture = self.texture.borrow_mut();
        if let Some(image) = self.frames.take() {
            if let Some(texture) = texture.as_mut() {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                *texture = Some(ui.ctx().load_texture(
                    "vlc-frame",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        let Some(texture) = texture.as_ref() else {
            return;
        };
        let scale = viewport.size() / texture.size_vec2();
        let size = match self.aspect.get() {
            DesktopAspect::Stretch => viewport.size(),
            DesktopAspect::Fill => texture.size_vec2() * scale.x.max(scale.y),
            _ => texture.size_vec2() * scale.x.min(scale.y),
        };
        ui.painter().with_clip_rect(viewport).image(
            texture.id(),
            egui::Rect::from_center_size(viewport.center(), size),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

impl Drop for Vlc {
    fn drop(&mut self) {
        self.stop();
        unsafe {
            (self.api.player_release)(self.ptr());
            (self.api.release)(self.instance.as_ptr());
        }
    }
}

#[cfg(target_os = "android")]
#[path = "vlc_frames.rs"]
mod frames;

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, PeekMessageW, MSG, PM_REMOVE, WS_POPUP,
    };

    #[test]
    #[ignore = "requires the packaged Windows LibVLC runtime beside the test binary"]
    fn windows_vlc_plays_and_seeks() {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                null(),
                WS_POPUP,
                0,
                0,
                640,
                360,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            )
        };
        assert!(!window.is_null());

        let player = Vlc::new(window as isize, &Settings::default()).unwrap();
        let clip =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ios/Resources/Playback.mp4");
        let url = reqwest::Url::from_file_path(clip).unwrap();
        player.load_at(url.as_str(), 2.0);
        player.set_volume(0);

        let deadline = Instant::now() + Duration::from_secs(12);
        let mut decoded_and_seeked = false;
        while Instant::now() < deadline {
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } != 0 {
                unsafe { DispatchMessageW(&msg) };
            }
            player.position();
            if player.duration() > 2.0
                && player.width() > 0
                && unsafe { (player.api.get_time)(player.ptr()) } >= 2_000
            {
                decoded_and_seeked = true;
                break;
            }
            if player.error() {
                break;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        drop(player);
        unsafe { DestroyWindow(window) };
        assert!(decoded_and_seeked);
    }
}
