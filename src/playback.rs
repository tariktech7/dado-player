use crate::mpv::Mpv;

#[cfg(not(any(target_os = "android", target_os = "windows")))]
pub(crate) type Playback = Mpv;

#[cfg(any(target_os = "android", target_os = "windows"))]
pub(crate) use embedded::Playback;

#[cfg(any(target_os = "android", target_os = "windows"))]
mod embedded {
    use super::Mpv;
    use crate::{
        platform::vlc::Vlc,
        types::{DesktopAspect, PlayerBackend, Settings},
    };

    pub(crate) enum Playback {
        Mpv(Mpv),
        Vlc(Box<Vlc>),
    }

    macro_rules! delegate {
        ($($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;)+) => {
            $(pub(crate) fn $name(&self $(,$arg: $ty)*) $(-> $ret)? {
                match self {
                    Self::Mpv(player) => player.$name($($arg),*),
                    Self::Vlc(player) => player.$name($($arg),*),
                }
            })+
        };
    }

    impl Playback {
        pub(crate) fn new(window: isize, settings: &Settings) -> Result<Self, String> {
            match settings.effective_player_backend() {
                PlayerBackend::Vlc => {
                    Vlc::new(window, settings).map(|vlc| Self::Vlc(Box::new(vlc)))
                }
                _ => Mpv::new(window, settings).map(Self::Mpv),
            }
        }

        pub(crate) fn is_vlc(&self) -> bool {
            matches!(self, Self::Vlc(_))
        }

        pub(crate) fn error(&self) -> bool {
            matches!(self, Self::Vlc(player) if player.error())
        }

        #[cfg(windows)]
        pub(crate) fn set_video_size(&self, width: u32, height: u32) {
            if let Self::Vlc(player) = self {
                player.set_video_size(width, height);
            }
        }

        #[cfg(target_os = "android")]
        pub(crate) fn paint(&self, ui: &egui::Ui, rect: egui::Rect) {
            match self {
                Self::Mpv(_) => {
                    ui.painter()
                        .add(crate::platform::android::render::shape(rect));
                }
                Self::Vlc(player) => player.paint(ui, rect),
            }
        }

        pub(crate) fn load_at(&self, url: &str, seconds: f64) {
            match self {
                #[cfg(target_os = "android")]
                Self::Mpv(_) => crate::platform::android::render::request_load(url, seconds),
                #[cfg(windows)]
                Self::Mpv(player) => player.load_at(url, seconds),
                Self::Vlc(player) => player.load_at(url, seconds),
            }
        }

        #[cfg(target_os = "android")]
        pub(crate) fn load_after_render_ready(&self, url: &str, seconds: f64) {
            if let Self::Mpv(player) = self {
                player.load_at(url, seconds);
            }
        }

        #[cfg(target_os = "android")]
        delegate! { set_speed(speed: f32); }

        delegate! {
            configure(settings: &Settings);
            stop();
            disable_video();
            enable_video();
            set_desktop_aspect(aspect: DesktopAspect);
            set_pause(paused: bool);
            toggle_pause();
            seek_absolute(seconds: f64);
            seek_relative(seconds: f64);
            set_volume(volume: i64);
            position() -> f64;
            duration() -> f64;
            cache_time() -> f64;
            paused() -> bool;
            fps() -> f64;
            width() -> i64;
            height() -> i64;
            video_codec() -> String;
            audio_codec() -> String;
            container() -> String;
            current_audio() -> i64;
            current_sub() -> i64;
            track_list_json() -> Option<String>;
            set_audio_track(id: i64);
            set_sub_track(id: i64);
            disable_sub();
        }
    }
}
