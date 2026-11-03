use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileKind {
    Xtream,
    M3u,
}

impl Default for ProfileKind {
    fn default() -> Self {
        ProfileKind::Xtream
    }
}

/// A saved connection (the user brings their own provider).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub kind: ProfileKind,
    // Xtream
    pub host: String,
    pub username: String,
    pub password: String,
    // M3U
    pub m3u_url: String,
    pub m3u_file: String,
    #[serde(default)]
    pub last_channel: Option<String>,
}

impl Profile {
    pub fn new(name: &str, kind: ProfileKind) -> Self {
        Profile {
            id: uuid_v4(),
            name: name.to_string(),
            kind,
            host: String::new(),
            username: String::new(),
            password: String::new(),
            m3u_url: String::new(),
            m3u_file: String::new(),
            last_channel: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum Theme {
    /// Follow the Windows setting (default).
    #[default]
    System,
    Dark,
    Light,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Dark => "Dunkel",
            Theme::Light => "Hell",
        }
    }
    pub fn all() -> [Theme; 3] {
        [Theme::System, Theme::Dark, Theme::Light]
    }
    /// Resolve to "is dark" using the current Windows setting.
    pub fn resolve(self, system_dark: bool) -> bool {
        match self {
            Theme::System => system_dark,
            Theme::Dark => true,
            Theme::Light => false,
        }
    }
}

/// Settings are shown either reduced ("Einfach") or with every knob ("Experte").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SettingsMode {
    #[default]
    Easy,
    Expert,
}

/// One entry of the "recently watched" history that feeds the Continue view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistEntry {
    /// progress key, e.g. "live:12", "vod:44", "ep:3:99"
    pub key: String,
    /// unix timestamp of the last interaction
    pub ts: i64,
    /// display title (channel / movie / series name)
    pub title: String,
    /// secondary line, e.g. "Staffel 2 · Folge 4"
    #[serde(default)]
    pub sub: String,
}

/// Hardware decoding backend. Different machines/drivers behave differently,
/// so the user can switch if playback stutters or stays black.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HwDec {
    #[default]
    AutoSafe,
    Auto,
    Off,
    D3d11va,
    Nvdec,
    Vulkan,
}

impl HwDec {
    pub fn mpv(self) -> &'static str {
        match self {
            HwDec::AutoSafe => "auto-safe",
            HwDec::Auto => "auto",
            HwDec::Off => "no",
            HwDec::D3d11va => "d3d11va",
            HwDec::Nvdec => "nvdec",
            HwDec::Vulkan => "vulkan",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            HwDec::AutoSafe => "Automatisch (empfohlen)",
            HwDec::Auto => "Automatisch (aggressiv)",
            HwDec::Off => "Aus (nur CPU)",
            HwDec::D3d11va => "D3D11VA (Windows)",
            HwDec::Nvdec => "NVDEC (NVIDIA)",
            HwDec::Vulkan => "Vulkan",
        }
    }
}

/// Video output driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VideoOut {
    #[default]
    Gpu,
    GpuNext,
    Direct3d,
}

impl VideoOut {
    pub fn mpv(self) -> &'static str {
        match self {
            VideoOut::Gpu => "gpu",
            VideoOut::GpuNext => "gpu-next",
            VideoOut::Direct3d => "direct3d",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            VideoOut::Gpu => "GPU (stabil)",
            VideoOut::GpuNext => "GPU Next (moderner)",
            VideoOut::Direct3d => "Direct3D (Kompatibilität)",
        }
    }
}

/// A/V sync mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VideoSync {
    #[default]
    Audio,
    DisplayResample,
}

impl VideoSync {
    pub fn mpv(self) -> &'static str {
        match self {
            VideoSync::Audio => "audio",
            VideoSync::DisplayResample => "display-resample",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            VideoSync::Audio => "Audio (Standard)",
            VideoSync::DisplayResample => "Display-Resample (flüssiger)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_cache", alias = "cache_mb")]
    pub cache_secs: u64,
    #[serde(default = "default_timeout")]
    pub network_timeout_s: u64,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub settings_mode: SettingsMode,
    #[serde(default = "default_true")]
    pub auto_play_last: bool,
    #[serde(default)]
    pub default_profile: Option<String>,

    // ---- advanced playback (mpv) ----
    #[serde(default)]
    pub hwdec: HwDec,
    #[serde(default)]
    pub vo: VideoOut,
    #[serde(default)]
    pub video_sync: VideoSync,
    #[serde(default)]
    pub interpolation: bool,
    #[serde(default)]
    pub deband: bool,
    #[serde(default)]
    pub audio_exclusive: bool,
    #[serde(default)]
    pub audio_passthrough: bool,
    #[serde(default)]
    pub audio_normalize: bool,
    #[serde(default = "default_sub_size")]
    pub sub_size: f32,
    #[serde(default)]
    pub user_agent: String,
    /// Bumped when defaults change in a way that needs migrating old files.
    #[serde(default)]
    pub settings_version: u32,
}

fn default_cache() -> u64 { 45 }
fn default_timeout() -> u64 { 20 }
fn default_true() -> bool { true }
fn default_sub_size() -> f32 { 55.0 }

impl Default for Settings {
    fn default() -> Self {
        Settings {
            cache_secs: default_cache(),
            network_timeout_s: default_timeout(),
            theme: Theme::System,
            settings_mode: SettingsMode::Easy,
            auto_play_last: true,
            default_profile: None,
            hwdec: HwDec::AutoSafe,
            vo: VideoOut::Gpu,
            video_sync: VideoSync::Audio,
            interpolation: false,
            deband: false,
            audio_exclusive: false,
            audio_passthrough: false,
            audio_normalize: false,
            sub_size: default_sub_size(),
            user_agent: String::new(),
            settings_version: 1,
        }
    }
}

impl Settings {
    /// Conservative defaults: works on almost every machine.
    pub fn preset_standard(&mut self) {
        self.hwdec = HwDec::AutoSafe;
        self.vo = VideoOut::Gpu;
        self.video_sync = VideoSync::Audio;
        self.interpolation = false;
        self.deband = false;
        self.audio_exclusive = false;
        self.audio_passthrough = false;
        self.audio_normalize = false;
        self.cache_secs = 45;
    }

    /// For weak laptops / slow connections: less GPU work, smaller buffer.
    pub fn preset_light(&mut self) {
        self.hwdec = HwDec::AutoSafe;
        self.vo = VideoOut::Gpu;
        self.video_sync = VideoSync::Audio;
        self.interpolation = false;
        self.deband = false;
        self.audio_exclusive = false;
        self.audio_passthrough = false;
        self.audio_normalize = false;
        self.cache_secs = 20;
    }

    /// For strong GPUs: smoothest motion and best scaling.
    pub fn preset_quality(&mut self) {
        self.hwdec = HwDec::Auto;
        self.vo = VideoOut::GpuNext;
        self.video_sync = VideoSync::DisplayResample;
        self.interpolation = true;
        self.deband = true;
        self.cache_secs = 90;
    }
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: u32,
    pub num: u32,
    pub name: String,
    pub logo: String,
    pub category_id: String,
    pub epg_channel_id: String,
    pub stream_url: String,
}

#[derive(Debug, Clone)]
pub struct VodMovie {
    pub id: u32,
    pub name: String,
    pub cover: String,
    pub category_id: String,
    pub rating: String,
    pub container_extension: String,
    pub stream_url: String,
    pub plot: String,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub id: u32,
    pub name: String,
    pub cover: String,
    pub category_id: String,
    pub plot: String,
}

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: u32,
    pub series_id: u32,
    pub season: u32,
    pub episode: u32,
    pub title: String,
    pub container_extension: String,
    pub stream_url: String,
}

#[derive(Debug, Clone)]
pub struct EpgEntry {
    pub title: String,
    pub description: String,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    /// Base URL used to build stream URLs (prefers the provider's https_port).
    pub stream_base: String,
    pub live_categories: Vec<Category>,
    pub vod_categories: Vec<Category>,
    pub series_categories: Vec<Category>,
    pub channels: Vec<Channel>,
    pub movies: Vec<VodMovie>,
    pub series: Vec<Series>,
    pub loaded: bool,
}

/// What the user is currently playing (kept tiny; sent to the player).
#[derive(Debug, Clone)]
pub struct PlayItem {
    pub title: String,
    pub url: String,
    pub kind: PlayKind,
    /// resume key for progress persistence
    pub progress_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayKind {
    Live,
    Vod,
    Episode,
}

/// Small helper to generate a unique id without extra deps.
pub fn uuid_v4() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    h ^= nanos as u64;
    h = h.wrapping_mul(0x0000_0100_0000_01b3);
    format!("{:016x}", h)
}
