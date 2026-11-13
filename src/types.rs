use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProfileKind {
    #[default]
    Xtream,
    M3u,
}

/// A saved connection (the user brings their own provider).
///
/// `Debug` is implemented by hand: the derived version printed the plaintext
/// password and the full M3U URL (which usually embeds `username`/`password`
/// as query parameters). Crash dumps, `log::debug!` output and support
/// screenshots must never contain provider credentials.
#[derive(Clone, Serialize, Deserialize)]
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

impl fmt::Debug for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("host", &self.host)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("m3u_url", &"<redacted>")
            .field("m3u_file", &self.m3u_file)
            .field("last_channel", &self.last_channel)
            .finish()
    }
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

/// Which presentation the library screens use on a TV / set-top box.
///
/// `Auto` follows Android's `UiModeManager` (MOBILE-BAUPLAN §23.10). `Always`
/// and `Never` are real user overrides: many Android TV boxes and HDMI sticks do
/// not declare `android.software.leanback`, and a phone connected to a TV should
/// stay touch-first. Keeping the override in settings also makes the 10-foot
/// layout testable on a normal handset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TvMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl TvMode {
    pub fn label(self) -> &'static str {
        match self {
            TvMode::Auto => "Automatisch",
            TvMode::Always => "Immer",
            TvMode::Never => "Aus",
        }
    }
    pub fn all() -> [TvMode; 3] {
        [TvMode::Auto, TvMode::Always, TvMode::Never]
    }
    pub fn resolve(self, device_is_tv: bool) -> bool {
        match self {
            TvMode::Auto => device_is_tv,
            TvMode::Always => true,
            TvMode::Never => false,
        }
    }
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

/// Which app actually plays the stream.
///
/// On Android the built-in libmpv build is deliberately conservative (it ships
/// without several demuxers and hardware decoders), so Live-TS and series
/// containers can fail while a normal `.mp4` movie plays fine. MX Player is the
/// default because it brings its own codec set and works on the widest range of
/// devices. VLC and the system chooser are real alternatives, not aliases, and
/// the internal player stays selectable for offline/local test patterns.
///
/// `MxPlayer` MUST remain the `Default`: `Settings::player_backend` uses
/// `#[serde(default)]`, so an old settings.json without the field resolves to
/// this variant. Making `Mpv` the default again would silently restore the
/// broken behaviour on every upgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlayerBackend {
    /// MX Player (free `…ad`, falling back to Pro `…pro`).
    #[default]
    MxPlayer,
    /// VLC for Android.
    Vlc,
    /// Android's resolver / chooser — any installed video player.
    System,
    /// Built-in libmpv. Also the only backend available on desktop.
    Mpv,
    /// MX Player integrated capability - embedded player without needing to install MX Player app.
    MxPlayerIntegrated,
    /// MX Player native integration capability - fallback for MX Player when not installed.
    MxPlayerFallback,
}

impl PlayerBackend {
    pub fn label(self) -> &'static str {
        match self {
            PlayerBackend::MxPlayer => "MX Player",
            PlayerBackend::Vlc => "VLC",
            PlayerBackend::System => "Systemauswahl",
            PlayerBackend::Mpv => "Integriert",
            PlayerBackend::MxPlayerIntegrated => "MX Player (integriert)",
            PlayerBackend::MxPlayerFallback => "MX Player (Fallback)",
        }
    }
    pub fn all() -> [PlayerBackend; 6] {
        [
            PlayerBackend::MxPlayer,
            PlayerBackend::Vlc,
            PlayerBackend::System,
            PlayerBackend::Mpv,
            PlayerBackend::MxPlayerIntegrated,
            PlayerBackend::MxPlayerFallback,
        ]
    }
    pub fn is_external(self) -> bool {
        match self {
            PlayerBackend::Mpv | PlayerBackend::MxPlayerIntegrated => false,
            PlayerBackend::MxPlayerFallback => true,
            _ => true,
        }
    }
    /// Wire format for the JNI `openExternalPlayer` call. Keep in sync with the
    /// `BACKEND_*` constants in GameActivity.java.
    pub fn android_code(self) -> i32 {
        match self {
            PlayerBackend::MxPlayer => 0,
            PlayerBackend::Vlc => 1,
            PlayerBackend::System => 2,
            PlayerBackend::Mpv => 3,
            PlayerBackend::MxPlayerIntegrated => 4,
            PlayerBackend::MxPlayerFallback => 5,
        }
    }
}

/// Xtream live streams are served under two different endpoints. `.ts` is a
/// single continuous MPEG-TS stream; `.m3u8` is an HLS playlist that has to be
/// fetched and whose segments are fetched again. Panels differ in which one
/// they actually enable, and the stripped-down libmpv build used on Android
/// handles raw TS far more reliably. TS is therefore the default, but HLS
/// stays one tap away because some providers only publish the playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LiveFormat {
    #[default]
    Ts,
    Hls,
}

impl LiveFormat {
    pub fn label(self) -> &'static str {
        match self {
            LiveFormat::Ts => "MPEG-TS (.ts)",
            LiveFormat::Hls => "HLS (.m3u8)",
        }
    }
    pub fn all() -> [LiveFormat; 2] {
        [LiveFormat::Ts, LiveFormat::Hls]
    }
    /// File suffix used in `/live/<user>/<pass>/<id>.<ext>`.
    pub fn extension(self) -> &'static str {
        match self {
            LiveFormat::Ts => "ts",
            LiveFormat::Hls => "m3u8",
        }
    }
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
    #[serde(default)]
    pub signed_out: bool,

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
    /// Verify the provider's TLS certificate for HLS/TS requests.
    ///
    /// Defaults to `false` on purpose: most Xtream panels terminate their
    /// stream port (often a separate `https_port`) with a self-signed or
    /// hostname-mismatched certificate. libmpv uses curl for those requests,
    /// and curl aborts before the first byte when verification is on - the
    /// player then just stays black with no visible error. Users of a panel
    /// with a real certificate can turn verification back on in expert mode.
    /// `#[serde(default)]` keeps older settings.json files working: they
    /// simply get the tolerant behaviour.
    #[serde(default)]
    pub tls_verify: bool,
    /// Which player backend to use for playback. MX Player is the default on
    /// Android; the internal libmpv stays available as a fallback and is the
    /// only backend on desktop. See `PlayerBackend` for why the default matters.
    /// MX Player now supports multiple modes:
    /// - MxPlayer: Standard MX Player app (requires installation)
    /// - MxPlayerIntegrated: Embedded player capabilities (no app installation required)
    /// - MxPlayerFallback: System fallback when MX Player is not available
    #[serde(default)]
    pub player_backend: PlayerBackend,
    /// Container used to build Xtream live URLs. M3U playlists keep whatever
    /// URL the playlist supplied; only Xtream channels are rebuilt.
    #[serde(default)]
    pub live_format: LiveFormat,
    /// 10-foot TV layout: Auto follows the device, Always/Never override it.
    #[serde(default)]
    pub tv_mode: TvMode,
    /// Bumped when defaults change in a way that needs migrating old files.
    #[serde(default)]
    pub settings_version: u32,
}

fn default_cache() -> u64 {
    45
}
fn default_timeout() -> u64 {
    20
}
fn default_true() -> bool {
    true
}
fn default_sub_size() -> f32 {
    55.0
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            cache_secs: default_cache(),
            network_timeout_s: default_timeout(),
            theme: Theme::System,
            settings_mode: SettingsMode::Easy,
            auto_play_last: true,
            default_profile: None,
            signed_out: false,
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
            tls_verify: false,
            player_backend: PlayerBackend::MxPlayer,
            live_format: LiveFormat::Ts,
            tv_mode: TvMode::Auto,
            settings_version: 2,
        }
    }
}

impl Settings {
    pub fn startup_profile<'a>(&self, profiles: &'a [Profile]) -> Option<&'a Profile> {
        if self.signed_out {
            return None;
        }
        match self.default_profile.as_deref() {
            Some(id) => profiles.iter().find(|p| p.id == id),
            None if profiles.len() == 1 => profiles.first(),
            _ => None,
        }
    }

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
    /// Optional provider timestamp for recently added channels.
    pub added: Option<i64>,
    pub name: String,
    pub logo: String,
    pub category_id: String,
    pub epg_channel_id: String,
    pub stream_url: String,
    /// M3U `radio="true"`: audio-only stream. The player then shows a radio
    /// card instead of the video surface (IPTVnator parity).
    pub radio: bool,
}

#[derive(Debug, Clone)]
pub struct VodMovie {
    pub id: u32,
    pub name: String,
    pub cover: String,
    pub category_id: String,
    pub rating: String,
    /// Provider timestamp for sorting the newest titles first (Unix seconds).
    pub added: Option<i64>,
    pub container_extension: String,
    pub stream_url: String,
    pub plot: String,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub id: u32,
    pub name: String,
    /// Optional provider timestamp for recently added series.
    pub added: Option<i64>,
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

/// One of the six Xtream catalogue endpoints. Keeping the area identity (and
/// not just an error string) lets the UI say *which* part of the library is
/// incomplete and lets a retry re-request exactly the failed groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogArea {
    LiveCategories,
    LiveStreams,
    MovieCategories,
    Movies,
    SeriesCategories,
    Series,
}

impl CatalogArea {
    pub fn label(self) -> &'static str {
        match self {
            CatalogArea::LiveCategories => "Live-Kategorien",
            CatalogArea::LiveStreams => "Senderliste",
            CatalogArea::MovieCategories => "Film-Kategorien",
            CatalogArea::Movies => "Filmliste",
            CatalogArea::SeriesCategories => "Serien-Kategorien",
            CatalogArea::Series => "Serienliste",
        }
    }
}

/// A non-fatal failure of a single catalogue endpoint. The message is already
/// credential-safe (`client::network_error` / `api_json` never echo the URL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogIssue {
    pub area: CatalogArea,
    pub message: String,
    pub elapsed_ms: u64,
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
    /// Per-area load diagnostics. Empty lists are NOT recorded here: a provider
    /// that legitimately has no series must not look like a broken login.
    pub catalog_issues: Vec<CatalogIssue>,
}

impl Library {
    /// Human-readable summary for the UI. Lists only genuinely failed areas.
    pub fn catalog_issue_summary(&self) -> String {
        if self.catalog_issues.is_empty() {
            return String::new();
        }
        let names: Vec<&str> = self.catalog_issues.iter().map(|i| i.area.label()).collect();
        format!(
            "Teile des Katalogs konnten nicht geladen werden: {}.",
            names.join(", ")
        )
    }
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

/// A single selectable audio or subtitle track reported by mpv.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Track {
    pub id: i64,
    pub kind: String,  // "audio" | "sub"
    pub lang: String,  // e.g. "de", "en"
    pub title: String, // human title if present
}

impl Track {
    /// Display label: "DE · German" / "EN" / "Track 2".
    pub fn label(&self) -> String {
        let mut s = String::new();
        if !self.lang.is_empty() {
            s.push_str(&self.lang.to_uppercase());
        }
        if !self.title.is_empty() {
            if !s.is_empty() {
                s.push_str(" · ");
            }
            s.push_str(&self.title);
        }
        if s.is_empty() {
            s.push_str(&format!("Track {}", self.id));
        }
        s
    }
}

/// Live stream / video metadata shown in the corner overlay.
#[derive(Debug, Clone, Default)]
pub struct StreamInfo {
    pub width: i64,
    pub height: i64,
    pub fps: f64,
    pub video_codec: String,
    pub audio_codec: String,
    /// Container format (mp4, mkv, ts, …); mostly meaningful for VOD/Series.
    pub container: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_debug_never_leaks_credentials() {
        let mut p = Profile::new("Mein Anbieter", ProfileKind::Xtream);
        p.host = "http://panel.example:8080".to_string();
        p.username = "alice".to_string();
        p.password = "super-secret-42".to_string();
        p.m3u_url =
            "http://panel.example/get.php?username=alice&password=super-secret-42".to_string();

        let rendered = format!("{:?}", p);
        assert!(!rendered.contains("super-secret-42"), "{rendered}");
        assert!(!rendered.contains("get.php"), "{rendered}");
        assert!(rendered.contains("redacted"));
        // Non-secret fields stay useful for diagnosing a bad profile.
        assert!(rendered.contains("alice"));
        assert!(rendered.contains("panel.example"));
    }

    #[test]
    fn catalog_issue_summary_names_only_failed_areas() {
        let mut lib = Library::default();
        assert!(
            lib.catalog_issue_summary().is_empty(),
            "empty lists are not errors"
        );

        lib.catalog_issues.push(CatalogIssue {
            area: CatalogArea::Series,
            message: "Zeitüberschreitung".to_string(),
            elapsed_ms: 20000,
        });
        let summary = lib.catalog_issue_summary();
        assert!(summary.contains("Serienliste"), "{summary}");
        assert!(!summary.contains("Senderliste"), "{summary}");
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    #[test]
    fn old_profiles_resume_only_if_unambiguous_and_logout_wins() {
        let mut s: Settings = serde_json::from_str("{}").unwrap();
        let a = Profile::new("a", ProfileKind::M3u);
        let mut b = Profile::new("b", ProfileKind::Xtream);
        b.id = "second".into();
        assert!(s.startup_profile(&[]).is_none());
        assert_eq!(
            s.startup_profile(std::slice::from_ref(&a)).unwrap().id,
            a.id
        );
        let both = [a, b];
        assert!(s.startup_profile(&both).is_none());
        s.default_profile = Some("second".into());
        assert_eq!(s.startup_profile(&both).unwrap().name, "b");
        s.signed_out = true;
        assert!(s.startup_profile(&both).is_none());
    }
}
