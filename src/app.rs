use crate::client;
use crate::platform::VideoSurface;
use crate::playback::Playback;
use crate::store;
use crate::types::*;
use crate::ui;
use egui::Context;
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const MAX_TEXTURE_CACHE_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEXTURE_CACHE_ITEMS: usize = 300;
/// Bounds for the lazily fetched EPG / episode detail caches. Without them a
/// long browsing session over a large provider grows two HashMaps without
/// limit — the exact opposite of the low-RAM target on phones and TV boxes.
const MAX_DETAIL_CACHE_ITEMS: usize = 128;
const MAX_DETAIL_ROWS: usize = 2_000;
const MAX_IN_FLIGHT_DETAIL_REQUESTS: usize = 64;

/// Insert into a FIFO-bounded detail cache. `order` may contain stale ids
/// (after a replacement); popped ids are simply ignored if already gone.
fn insert_bounded_detail<V>(
    map: &mut HashMap<u32, V>,
    order: &mut VecDeque<u32>,
    key: u32,
    value: V,
) {
    let replacing = map.contains_key(&key);
    order.retain(|existing| *existing != key);
    if !replacing {
        while map.len() >= MAX_DETAIL_CACHE_ITEMS {
            match order.pop_front() {
                Some(oldest) => {
                    map.remove(&oldest);
                }
                None => break,
            }
        }
    }
    map.insert(key, value);
    order.push_back(key);
}

fn mark_detail_failed(set: &mut HashSet<u32>, id: u32) {
    // Failure markers are bounded too: a provider that is completely offline
    // must not accumulate one id per channel forever.
    if set.len() >= MAX_DETAIL_CACHE_ITEMS {
        set.clear();
    }
    set.insert(id);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Login,
    Home,
    Live,
    Movies,
    Series,
    Search,
    Favorites,
    Continue,
    Settings,
    Player,
}

pub enum AppEvent {
    /// Boxed because a full `Library` is ~400 bytes; every other event is a
    /// few dozen bytes. Without the box, the enum (and every slot in the
    /// unbounded channel) is sized for the single largest variant.
    LoginDone(u64, Box<Result<(Profile, Library), String>>),
    CatalogRefreshed(u64, Box<Result<Library, String>>),
    Epg(u64, u32, Result<Vec<EpgEntry>, String>),
    SeriesInfo(u64, u32, Result<Vec<Episode>, String>),
    Image(u64, String, Result<egui::ColorImage, String>),
    EpisodeFrame(u64, String, Option<egui::ColorImage>),
}

/// Wake the reactive UI when network/decode work finishes, including after the
/// cold-start repaint timer has stopped. No idle polling is necessary.
#[derive(Clone)]
struct EventSender {
    inner: UnboundedSender<AppEvent>,
    context: std::sync::Arc<std::sync::OnceLock<Context>>,
}
impl EventSender {
    fn send(&self, event: AppEvent) -> Result<(), tokio::sync::mpsc::error::SendError<AppEvent>> {
        self.inner.send(event)?;
        if let Some(ctx) = self.context.get() {
            ctx.request_repaint();
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ImageState {
    pub textures: HashMap<String, egui::TextureHandle>,
    /// Decoded RGBA bytes tracked for a strict aggregate texture-cache budget.
    pub texture_bytes: HashMap<String, usize>,
    pub total_texture_bytes: usize,
    pub pending: Vec<(String, egui::ColorImage)>,
    pub requested: HashSet<String>,
    pub failed: HashSet<String>,
    pub failed_order: std::collections::VecDeque<String>,
    pub order: std::collections::VecDeque<String>,
}

impl ImageState {
    fn evict_oldest(&mut self) -> bool {
        let Some(key) = self.order.pop_front() else {
            return false;
        };
        self.textures.remove(&key);
        if let Some(bytes) = self.texture_bytes.remove(&key) {
            self.total_texture_bytes = self.total_texture_bytes.saturating_sub(bytes);
        }
        self.requested.remove(&key);
        true
    }

    fn insert_texture(
        &mut self,
        key: String,
        texture: egui::TextureHandle,
        byte_len: usize,
        budget: usize,
    ) {
        if self.textures.contains_key(&key) {
            self.textures.remove(&key);
            if let Some(bytes) = self.texture_bytes.remove(&key) {
                self.total_texture_bytes = self.total_texture_bytes.saturating_sub(bytes);
            }
            self.order.retain(|old| old != &key);
        }
        if byte_len > budget {
            return;
        }
        while self.textures.len() >= MAX_TEXTURE_CACHE_ITEMS
            || self.total_texture_bytes.saturating_add(byte_len) > budget
        {
            if !self.evict_oldest() {
                return;
            }
        }
        self.total_texture_bytes += byte_len;
        self.texture_bytes.insert(key.clone(), byte_len);
        self.textures.insert(key.clone(), texture);
        self.order.push_back(key);
    }
}

pub struct Player {
    /// Declared before `surface` so mpv is torn down before its window.
    pub(crate) mpv: Playback,
    pub surface: VideoSurface,
    pub current_key: String,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
    /// Live stream / video metadata shown in the corner overlay.
    pub stream: StreamInfo,
    /// Selectable audio tracks (movies / series).
    pub audio_tracks: Vec<Track>,
    /// Selectable subtitle tracks (movies / series).
    pub sub_tracks: Vec<Track>,
    /// Currently selected audio / subtitle track id (mpv `aid` / `sid`).
    pub current_audio: i64,
    pub current_sub: i64,
    audio_preference_applied: bool,
    subtitle_preference_applied: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContinueTab {
    Live,
    Movies,
    Series,
}

pub struct App {
    pub parental: crate::parental::Controls,
    rt: Runtime,
    tx: EventSender,
    rx: UnboundedReceiver<AppEvent>,

    pub profiles: Vec<Profile>,
    pub active_profile: Option<Profile>,
    data_profile: Option<String>,
    pub library: Library,
    pub view: View,
    previous_view: View,
    #[cfg(windows)]
    desktop_origin_focus: Option<egui::Id>,
    pub(crate) desktop_return_item: Option<String>,
    /// "Mehr" bottom sheet of the phone UI is open.
    pub mobile_more: bool,
    /// Android/Live: channel whose expandable EPG sheet is open (None = closed).
    pub mobile_epg_channel: Option<u32>,
    /// The 10-foot TV shell is active this frame (MOBILE-BAUPLAN §23.10).
    /// Resolved every frame from `Settings::tv_mode` + `platform::tv_device()`,
    /// so changing the setting takes effect without a restart.
    pub tv: bool,
    /// Selected header in the TV shell (index into `tv::HEADERS`).
    pub tv_nav: usize,
    /// Visited TV sections. BACK pops this instead of always jumping to Live.
    pub tv_stack: Vec<usize>,
    /// Series whose episodes are shown as a rail; BACK clears it first.
    pub tv_series: Option<u32>,
    /// Letter-by-letter TV search text. A TV has no IME the user can reach with
    /// a D-pad, so search uses an on-screen letter row instead of a TextEdit.
    pub tv_search: String,
    /// Profile id awaiting confirmation before it is deleted. Lives on `App`
    /// rather than in egui memory because the Android back key is handled in
    /// `update()`, which has no `Context` to read memory from — and back has to
    /// cancel the dialog, not close the activity.
    pub pending_delete: Option<String>,
    pub busy: bool,
    /// Monotonic login request ID; stale completions after logout/new login are ignored.
    login_generation: u64,
    pub catalog_refreshing: bool,
    pub catalog_refresh_notice: Option<String>,
    pub(crate) catalog_refresh_result: Option<(bool, std::time::Instant)>,
    pub error: Option<String>,
    pub info: Option<String>,

    // login form
    pub login_kind: ProfileKind,
    editing_profile: Option<String>,
    pub f_name: String,
    pub f_host: String,
    pub f_user: String,
    pub f_pass: String,
    pub show_password: bool,
    pub f_m3u_url: String,
    pub f_m3u_file: String,

    // ui state
    pub live_cat: String,
    pub vod_cat: String,
    pub series_cat: String,
    pub search_q: String,
    pub live_q: String,
    pub catalog_queries: [String; 2],
    pub nav_collapsed: bool,
    pub description_open: Option<(String, String)>,
    pub selected_movie: Option<u32>,
    pub favorite_section: usize,
    pub live_selected: Option<u32>,
    pub previous_live: Option<PlayItem>,
    pub selected_series: Option<u32>,
    pub series_season: u32,
    pub series_open_revision: u64,
    pub live_page: usize,
    /// Horizontal EPG timeline offset from the current programme window, in hours.
    pub epg_offset_hours: i32,
    pub vod_page: usize,
    pub series_page: usize,

    pub favorites: HashSet<String>,
    pub progress: HashMap<String, u64>,
    pub durations: HashMap<String, u64>,
    /// Recently watched items (newest first) for the Continue view.
    pub history: Vec<HistEntry>,
    pub continue_tab: ContinueTab,
    pub settings: Settings,
    pub settings_section: Option<usize>,
    pub adding_profile: bool,
    pub settings_return_tile: Option<usize>,

    pub images: ImageState,
    episode_frame_request: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub connect: crate::connect::DeviceClient,
    pub epg: HashMap<u32, Vec<EpgEntry>>,
    epg_order: VecDeque<u32>,
    epg_requested: HashSet<u32>,
    /// Ids whose last EPG request failed. Kept so the UI can show a real
    /// error + retry instead of silently showing "no programme data" forever
    /// (and so `request_epg` does not hammer the provider every frame).
    epg_failed: HashSet<u32>,
    pub epg_error: Option<String>,
    pub series_episodes: HashMap<u32, Vec<Episode>>,
    series_order: VecDeque<u32>,
    series_requested: HashSet<u32>,
    series_failed: HashSet<u32>,
    pub series_error: Option<String>,

    pub current_play: Option<PlayItem>,
    /// Pending "Fortsetzen?" confirmation: (progress_key, item, position).
    /// Set when a watched movie/series is clicked; the UI renders the modal
    /// before anything actually starts. `None` = no dialog is open.
    pub pending_resume: Option<(String, PlayItem, u64)>,
    catalog_resume_series: Option<u32>,
    pending_history_key: Option<String>,
    pub(crate) untracked_playback: bool,
    /// Channel currently shown in the inline preview (list stays visible).
    pub preview: Option<PlayItem>,
    /// Channel ids in the order the Live list currently shows them (for ←/→).
    pub live_order: Vec<u32>,
    /// Ask the Live list to scroll this channel into view.
    pub scroll_to: Option<u32>,
    pending_fullscreen: Option<bool>,
    applied_dark: Option<(crate::types::Theme, bool, bool, bool)>,
    system_dark: bool,
    last_theme_poll: f64,
    preview_placed: bool,
    dark_mode_override: Option<bool>,
    /// Last video rect in device pixels, so we do not resize every frame.
    last_video_rect: Option<(i32, i32, i32, i32)>,
    /// Last window-region cutout set. Cached because `SetWindowRgn` invalidates
    /// and repaints the native surface every time it is called.
    last_video_cutouts: Option<Vec<(i32, i32, i32, i32)>>,
    /// Timestamp of the last mouse/key activity (player controls auto-hide).
    pub ui_activity: f64,
    last_progress_save: f64,
    /// True while the player chrome was visible on the previous frame. Used to
    /// tell "this input reveals the controls" apart from "this input presses a
    /// control" — otherwise the first OK after the menu faded out would land on
    /// the Back button and stop playback.
    pub player_chrome: bool,
    pub player_menu_navigation: bool,
    pub catalog_focus_play: bool,
    pub(crate) catalog_return_poster: Option<(bool, u32)>,
    /// Set for exactly one frame by `update()` when the input that just arrived
    /// only serves to bring the hidden chrome back. Consumed by the player.
    pub player_reveal: bool,
    last_cursor: (i32, i32),
    #[cfg(target_os = "windows")]
    pub(crate) desktop_overlay: crate::platform::windows::player_overlay::Overlay,
    pub player: Option<Player>,
    #[cfg(target_os = "android")]
    parked_mpv: Option<Playback>,
    pub player_error: Option<String>,
    main_hwnd: isize,
    pub fullscreen: bool,
    pub volume: i64,
    pub autostart_test: bool,

    /// Open the audio-track selector popover in the player corner overlay.
    pub audio_menu_open: bool,
    /// Open the subtitle-track selector popover in the player corner overlay.
    pub sub_menu_open: bool,
    /// Frame counter used to throttle the (JSON) track-list poll.
    frame_count: u64,

    /// Command palette (Ctrl+K): open state, query and highlighted row.
    pub palette_open: bool,
    pub palette_q: String,
    pub palette_sel: usize,

    /// Direct channel entry: digits typed in Live TV plus the time of the last
    /// keystroke (seconds, egui time base). Commits after a short idle.
    pub chan_jump: String,
    pub chan_jump_at: f64,

    /// Filter text for the settings page.
    pub settings_q: String,
}

/// Parse mpv's `track-list` JSON into audio/subtitle tracks plus the currently
/// selected ids. Returns `None` if the payload is not the expected array.
fn parse_tracks(json: &str, cur_a: i64, cur_s: i64) -> Option<(Vec<Track>, Vec<Track>, i64, i64)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let arr = v.as_array()?;
    let mut a: Vec<Track> = Vec::new();
    let mut s: Vec<Track> = Vec::new();
    for t in arr {
        let id = t.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
        let kind = t
            .get("type")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if kind != "audio" && kind != "sub" {
            continue;
        }
        let lang = t
            .get("lang")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let title = t
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let tr = Track {
            id,
            kind,
            lang,
            title,
        };
        if tr.kind == "audio" {
            a.push(tr);
        } else {
            s.push(tr);
        }
    }
    Some((a, s, cur_a, cur_s))
}

impl App {
    pub fn cloud_profiles_ui(&mut self, ui: &mut egui::Ui) {
        let remote = self.shared_remote_navigation();
        crate::connect::show(
            ui,
            &mut self.connect,
            self.rt.handle(),
            &mut self.profiles,
            remote,
            false,
        );
    }
    pub fn cloud_profiles_entry(&mut self, ui: &mut egui::Ui) -> bool {
        crate::connect::show(
            ui,
            &mut self.connect,
            self.rt.handle(),
            &mut self.profiles,
            self.tv,
            true,
        );
        self.connect.expanded()
    }
    pub fn new(rt: Runtime) -> Self {
        let (tx, rx) = unbounded_channel();
        let tx = EventSender {
            inner: tx,
            context: Default::default(),
        };
        let mut app = App {
            rt,
            tx,
            rx,
            profiles: store::load_profiles(),
            active_profile: None,
            data_profile: None,
            library: Library::default(),
            view: View::Login,
            previous_view: View::Live,
            #[cfg(windows)]
            desktop_origin_focus: None,
            desktop_return_item: None,
            mobile_more: false,
            mobile_epg_channel: None,
            tv: false,
            tv_nav: 0,
            tv_stack: Vec::new(),
            tv_series: None,
            tv_search: String::new(),
            pending_delete: None,
            parental: crate::parental::Controls::load(),
            busy: false,
            login_generation: 0,
            catalog_refreshing: false,
            catalog_refresh_notice: None,
            catalog_refresh_result: None,
            error: None,
            info: None,
            login_kind: ProfileKind::Xtream,
            editing_profile: None,
            f_name: String::new(),
            f_host: String::new(),
            f_user: String::new(),
            f_pass: String::new(),
            show_password: false,
            f_m3u_url: String::new(),
            f_m3u_file: String::new(),
            live_cat: "all".to_string(),
            vod_cat: "all".to_string(),
            series_cat: "all".to_string(),
            search_q: String::new(),
            live_q: String::new(),
            catalog_queries: [String::new(), String::new()],
            nav_collapsed: false,
            description_open: None,
            selected_movie: None,
            favorite_section: 0,
            live_selected: None,
            previous_live: None,
            selected_series: None,
            series_season: 0,
            series_open_revision: 0,
            live_page: 0,
            epg_offset_hours: 0,
            vod_page: 0,
            series_page: 0,
            favorites: store::load_favorites(),
            progress: store::load_progress(),
            durations: store::load_durations(),
            history: store::load_history(),
            continue_tab: ContinueTab::Movies,
            settings: store::load_settings(),
            settings_section: None,
            adding_profile: false,
            settings_return_tile: None,
            images: ImageState::default(),
            episode_frame_request: None,
            connect: crate::connect::DeviceClient::default(),
            epg: HashMap::new(),
            epg_order: VecDeque::new(),
            epg_requested: HashSet::new(),
            epg_failed: HashSet::new(),
            epg_error: None,
            series_episodes: HashMap::new(),
            series_order: VecDeque::new(),
            series_requested: HashSet::new(),
            series_failed: HashSet::new(),
            series_error: None,
            current_play: None,
            preview: None,
            live_order: Vec::new(),
            scroll_to: None,
            pending_fullscreen: None,
            applied_dark: None,
            system_dark: true,
            last_theme_poll: -10.0,
            preview_placed: false,
            dark_mode_override: None,
            last_video_rect: None,
            last_video_cutouts: None,
            ui_activity: 0.0,
            last_progress_save: 0.0,
            player_chrome: true,
            player_menu_navigation: false,
            catalog_focus_play: false,
            catalog_return_poster: None,
            player_reveal: false,
            last_cursor: (0, 0),
            #[cfg(target_os = "windows")]
            desktop_overlay: Default::default(),
            player: None,
            #[cfg(target_os = "android")]
            parked_mpv: None,
            player_error: None,
            main_hwnd: 0,
            fullscreen: false,
            volume: 100,
            autostart_test: std::env::var("IPTV_TEST_STREAM").is_ok(),
            audio_menu_open: false,
            sub_menu_open: false,
            frame_count: 0,
            pending_resume: None,
            catalog_resume_series: None,
            pending_history_key: None,
            untracked_playback: false,
            palette_open: false,
            palette_q: String::new(),
            palette_sel: 0,
            chan_jump: String::new(),
            chan_jump_at: 0.0,
            settings_q: String::new(),
        };
        app.favorites = app.favorites.iter().map(|key| favorite_key(key)).collect();
        // Layout check without a provider account (never persisted).
        let env_demo = std::env::var("IPTV_DEMO").is_ok();
        let env_view = std::env::var("IPTV_DEMO_VIEW").ok();
        let env_dark = std::env::var("IPTV_DARK")
            .ok()
            .and_then(|s| s.parse::<bool>().ok());
        if env_demo {
            app.load_demo();
            let requested = env_view.as_deref();
            app.view = match requested {
                Some("movies") => View::Movies,
                Some("series") => View::Series,
                Some("settings") => View::Settings,
                Some("continue") => View::Continue,
                Some("favorites") => View::Favorites,
                Some("search") => View::Search,
                Some("home") => View::Home,
                _ => View::Home,
            };
            if let Some(dark) = env_dark {
                app.dark_mode_override = Some(dark);
            }
            if app.view == View::Series {
                app.selected_series = Some(app.library.series.first().map(|s| s.id).unwrap_or(0));
                let demo_url = std::env::var("IPTV_TEST_URL").unwrap_or_default();
                app.series_episodes.insert(
                    2000,
                    (1..=6)
                        .map(|i| Episode {
                            id: 9000 + i,
                            series_id: 2000,
                            preview: String::new(),
                            season: if i <= 3 { 1 } else { 2 },
                            episode: if i <= 3 { i } else { i - 3 },
                            title: format!("Demo Episode {}", i),
                            container_extension: "mkv".to_string(),
                            stream_url: demo_url.clone(),
                        })
                        .collect(),
                );
                // One season, one episode. The hero's meta line has nothing to
                // add — the only sentence it could produce is „1 Folge", which
                // repeats the single row directly below it — so it must render
                // nothing at all. This series is the fixture for that rule.
                app.series_episodes.insert(
                    2001,
                    vec![Episode {
                        id: 9101,
                        series_id: 2001,
                        preview: String::new(),
                        season: 1,
                        episode: 1,
                        title: "Pilot".to_string(),
                        container_extension: "mkv".to_string(),
                        stream_url: demo_url.clone(),
                    }],
                );
            }
        }

        // Deterministic QA states. Read only when IPTV_DEMO is set and never
        // persisted, so loading / empty / partial-error / retry screens can be
        // captured reproducibly without a real provider or a network fault.
        if env_demo {
            if std::env::var("IPTV_DEMO_EMPTY").is_ok() {
                app.library.live_categories.clear();
                app.library.vod_categories.clear();
                app.library.series_categories.clear();
                app.library.channels.clear();
                app.library.movies.clear();
                app.library.series.clear();
                app.history.clear();
                app.favorites.clear();
                app.selected_series = None;
                app.series_episodes.clear();
            }
            if std::env::var("IPTV_DEMO_CATALOG_ISSUES").is_ok() {
                app.library.catalog_issues = vec![
                    CatalogIssue {
                        area: CatalogArea::Movies,
                        message:
                            "Zeitüberschreitung. Bitte Verbindung und Timeout-Einstellung prüfen."
                                .to_string(),
                        elapsed_ms: 20_000,
                    },
                    CatalogIssue {
                        area: CatalogArea::Series,
                        message: "Antwort des Anbieters hat ein unerwartetes Format.".to_string(),
                        elapsed_ms: 812,
                    },
                ];
            }
            if std::env::var("IPTV_DEMO_DETAIL_ERROR").is_ok() {
                let channel_id = app.library.channels.first().map(|c| c.id).unwrap_or(1);
                let series_id = app.library.series.first().map(|s| s.id).unwrap_or(2000);
                app.epg_failed.insert(channel_id);
                app.epg_error = Some(
                    "Server nicht erreichbar. Host, Port, DNS und Internetverbindung prüfen."
                        .to_string(),
                );
                app.series_failed.insert(series_id);
                app.series_error = Some(
                    "Zeitüberschreitung. Bitte Verbindung und Timeout-Einstellung prüfen."
                        .to_string(),
                );
                // The demo seeds episodes for this id; remove them so the
                // failure branch (not the episode list) is what gets drawn.
                app.series_episodes.remove(&series_id);
                app.selected_series = Some(series_id);
            }
            if std::env::var("IPTV_DEMO_LOADING").is_ok() {
                app.busy = true;
            }
        }

        // QA hooks for the command palette: open it (and optionally prefill the
        // query) without depending on synthetic keyboard input.
        if std::env::var("IPTV_DEMO_PALETTE").is_ok() {
            app.palette_open = true;
            if let Ok(q) = std::env::var("IPTV_DEMO_PALETTE_Q") {
                app.palette_q = q;
            }
        }
        if !env_demo && !cfg!(test) {
            if let Some(profile) = app.settings.startup_profile(&app.profiles).cloned() {
                app.start_login(profile);
            }
        }
        app.tv = app.settings.tv_mode.resolve(crate::platform::tv_device());
        app
    }

    /// Load the bundled sample library without a provider account.
    ///
    /// This is a real product path, not a QA backdoor: on a TV there is no way
    /// to type credentials with a D-pad, and a buyer should still be able to
    /// look at the 10-foot UI before subscribing. It also gives device QA a
    /// deterministic post-login screen without a live IPTV panel.
    pub fn load_demo(&mut self) {
        self.library = crate::demo::library();
        self.active_profile = Some(Profile::new("Demo", ProfileKind::Xtream));
        self.view = View::Home;
        if self.history.is_empty() {
            self.history = crate::demo::seed_history(&self.library);
            for h in self.history.clone() {
                if h.key.starts_with("vod:") {
                    self.progress.insert(h.key, 1234);
                }
            }
        }
        self.tv_stack.clear();
        self.tv_nav = 0;
        self.tv_series = None;
        if self.favorites.is_empty() {
            for ch in self.library.channels.iter().take(6) {
                self.favorites.insert(format!("live:{}", ch.id));
            }
            for movie in self.library.movies.iter().take(8) {
                self.favorites.insert(format!("vod:{}", movie.id));
            }
        }
    }

    pub(crate) fn shared_remote_navigation(&self) -> bool {
        self.parental.open
            || self.tv
            || (self.active_profile.is_some()
                && !self.palette_open
                && (self.view != View::Settings
                    || self
                        .settings_section
                        .is_none_or(ui::settings::uses_ordered_focus)))
    }

    pub fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let _ = self.tx.context.set(ctx.clone());
        if std::env::var_os("IPTV_DEMO").is_none() {
            self.connect
                .prepare(ctx, self.rt.handle(), &mut self.profiles);
        }
        #[cfg(target_os = "windows")]
        crate::licensing::tick(ctx);
        #[cfg(target_os = "ios")]
        if crate::platform::ios::bridge::take_player_closed() {
            self.stop_play();
        }
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::video_surface::register_repaint(ctx);
            if crate::platform::windows::video_surface::take_click() {
                if self.preview.is_some() {
                    self.promote_preview(false);
                } else if self.view == View::Player {
                    self.ui_activity = ctx.input(|i| i.time);
                    self.player_reveal = false;
                    ctx.request_repaint();
                }
            }
        }
        let now = ctx.input(|i| i.time);

        // Cold-start guard. eframe is reactive: on slow GL drivers the very
        // first frame can be dropped, and with no player yet there is nothing
        // else asking for a repaint. The UI then stays black until the user
        // happens to touch the screen. Keep a slow poll alive for the first
        // few seconds, then stop so idle screens do not burn battery.
        static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        let t0 = *STARTED.get_or_init(std::time::Instant::now);
        if t0.elapsed() < std::time::Duration::from_secs(12) {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        let tv = crate::platform::tv_device() || std::env::var_os("IPTV_TV").is_some();
        if tv && !self.tv {
            let section = if self.view == View::Player {
                self.previous_view
            } else {
                self.view
            };
            self.tv_nav = Self::tv_section(section);
            self.tv_stack.clear();
            self.nav_collapsed = false;
        }
        self.tv = tv;

        // Theme: "System" follows Windows, polled so a live OS switch is picked up.
        if now - self.last_theme_poll > 1.5 {
            self.last_theme_poll = now;
            let dark = crate::platform::sys::system_dark();
            if dark != self.system_dark {
                self.system_dark = dark;
                ctx.request_repaint();
            }
        }
        // Allow overriding the theme via environment for QA purposes.
        let want_dark = self
            .dark_mode_override
            .unwrap_or_else(|| self.settings.theme.resolve(self.system_dark));
        let appearance = (
            self.settings.theme,
            want_dark,
            ui::mobile::active(ctx),
            self.tv,
        );
        if self.applied_dark != Some(appearance) {
            self.applied_dark = Some(appearance);
            crate::theme::apply(ctx, want_dark);
            if self.settings.theme == crate::types::Theme::Warm && self.dark_mode_override.is_none()
            {
                crate::theme::apply_warm(ctx);
            }
            ctx.request_repaint();
        }

        // Command palette: Ctrl/Cmd+K toggles it from anywhere.
        let toggle_palette =
            ctx.input(|i| i.modifiers.command && !i.modifiers.alt && i.key_pressed(egui::Key::K));
        if toggle_palette {
            self.palette_open = !self.palette_open;
            if self.palette_open {
                self.palette_q.clear();
                self.palette_sel = 0;
            }
        }
        // The custom font families only exist after the next frame starts.
        if !crate::fonts::ready(ctx) {
            ctx.request_repaint();
            return;
        }
        self.preview_placed = false;

        // Remember the native surface handle for embedding the video surface.
        // Platform-neutral: on Windows this is the HWND, on Android the
        // ANativeWindow pointer. `acquire_surface` returns a *borrowed* handle
        // only – the host window must never be wrapped in an owning
        // `VideoSurface`, whose `Drop` would destroy it.
        if let Some(h) = crate::platform::acquire_surface(frame) {
            self.main_hwnd = h;
        }

        while let Ok(ev) = self.rx.try_recv() {
            self.handle_event(ev);
        }

        // `ui::mobile::login::show()` is the only writer of TEXT_FOCUS and it
        // runs *only* on the mobile login screen. As soon as that screen goes
        // away (successful login, player, TV shell, desktop UI) nothing would
        // ever reset the flag, so a stale `true` would swallow every D-pad/OK
        // key for the rest of the session. Clear it here — before BACK,
        // `set_can_back` and the remote-key gate all read it.
        let login_screen = crate::ui::mobile::active(ctx)
            && self.active_profile.is_none()
            && self.view != View::Player;
        if !login_screen {
            crate::platform::set_text_focus(false);
        }

        // ---- Android: soft keyboard, system back, insets ----
        #[cfg(target_os = "android")]
        {
            use crate::platform::android::jni_bridge;

            // Hand the Java callbacks a way to wake this loop: back is pushed
            // from the UI thread and an idle screen never schedules a frame, so
            // without this the request would wait for an unrelated touch.
            jni_bridge::bind_context(ctx);
            // The paint callback creates mpv's render context at the *end* of
            // a frame and wakes this loop; loadfile is only legal afterwards.
            self.flush_android_load();

            // Android supplies the authoritative full buffer. Select the
            // egui field's current contents before inserting that snapshot;
            // prefix-diff + Backspace replay is incorrect for cursor edits,
            // selection replacement, and paste.
            //
            // `Modifiers::COMMAND`, not `Modifiers::CTRL`: egui binds
            // select-all to `Key::A if modifiers.command`, and `CTRL` has
            // `command: false`. With the wrong one nothing was selected and
            // the snapshot was APPENDED at the caret - reproduced with Gboard
            // on the emulator: "W" -> "WWO" -> "WWOWOH" -> "WWOWOHWOHN".
            // That is the "typing looks like flicker / garbage" report.
            //
            // Also gate on a focused widget: without one, `Text` would be
            // inserted into whatever else listens to the event stream.
            if let Some(edit) = jni_bridge::take_edit() {
                if ctx.memory(|m| m.focused()).is_some() {
                    ctx.input_mut(|i| {
                        i.events.push(egui::Event::Key {
                            key: egui::Key::A,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::COMMAND,
                        });
                        i.events.push(egui::Event::Text(edit));
                    });
                    ctx.request_repaint();
                }
            }

            if jni_bridge::take_back() {
                if crate::platform::text_focus() || ctx.wants_keyboard_input() {
                    // BACK while editing ends text entry; it must not fall
                    // through to mobile_back() -> "leave the app". On a TV
                    // this is the only way out of a focused field with a D-pad.
                    let focused = ctx.memory(|m| m.focused());
                    if let Some(id) = focused {
                        ctx.memory_mut(|m| m.surrender_focus(id));
                    }
                    crate::platform::set_text_focus(false);
                } else {
                    if !crate::ui::desktop::color_shortcut(self, ctx, egui::Key::Escape) {
                        self.mobile_back();
                    }
                }
                ctx.request_repaint();
            }
            // AppCompatActivity asks before it finishes the activity.
            jni_bridge::set_can_back(
                self.active_profile.is_some()
                    || self.mobile_can_back()
                    || crate::platform::text_focus(),
            );
        }

        // Poll the player and keep playback progress + stream metadata in memory.
        if let Some(pl) = &mut self.player {
            #[cfg(any(target_os = "android", target_os = "windows"))]
            if (self.current_play.is_some() || self.preview.is_some()) && pl.mpv.error() {
                self.player_error = Some(
                    self.settings
                        .language
                        .text(
                            "VLC konnte diesen Stream nicht abspielen.",
                            "VLC could not play this stream.",
                        )
                        .to_owned(),
                );
            }
            let pos = pl.mpv.position();
            let dur = pl.mpv.duration();
            let paused = pl.mpv.paused();
            let key = pl.current_key.clone();
            pl.position = pos;
            pl.duration = dur;
            if self.settings.watch_history
                && !self.untracked_playback
                && !key.is_empty()
                && dur > 0.0
            {
                self.durations.insert(key.clone(), dur as u64);
            }
            pl.paused = paused;
            if self.settings.watch_history
                && !self.untracked_playback
                && !key.is_empty()
                && pos > 1.0
            {
                self.progress.insert(key, pos as u64);
            }

            // Stream info (cheap per-frame get_property reads).
            pl.stream = StreamInfo {
                width: pl.mpv.width(),
                height: pl.mpv.height(),
                fps: pl.mpv.fps(),
                video_codec: pl.mpv.video_codec(),
                audio_codec: pl.mpv.audio_codec(),
                container: pl.mpv.container(),
            };

            // Track list (JSON parse) — only for VOD/Series, throttled so we
            // don't re-parse every frame. Force a refresh until we have at
            // least the audio list, then settle to every ~15 frames.
            self.frame_count = self.frame_count.wrapping_add(1);
            if self.frame_count.is_multiple_of(15) || pl.audio_tracks.is_empty() {
                if let Some(json) = pl.mpv.track_list_json() {
                    if let Some((a, s, ca, cs)) =
                        parse_tracks(&json, pl.mpv.current_audio(), pl.mpv.current_sub())
                    {
                        pl.audio_tracks = a;
                        pl.sub_tracks = s;
                        pl.current_audio = ca;
                        pl.current_sub = cs;
                        if !pl.audio_preference_applied && !pl.audio_tracks.is_empty() {
                            if let Some(track) =
                                preferred_track(&pl.audio_tracks, &self.settings.audio_language)
                            {
                                pl.mpv.set_audio_track(track.id);
                                pl.current_audio = track.id;
                            }
                            pl.audio_preference_applied = true;
                        }
                        if !pl.subtitle_preference_applied
                            && (!pl.audio_tracks.is_empty() || !pl.sub_tracks.is_empty())
                        {
                            if !self.settings.subtitles_enabled {
                                pl.mpv.disable_sub();
                                pl.current_sub = -1;
                                pl.subtitle_preference_applied = true;
                            } else if let Some(track) =
                                preferred_track(&pl.sub_tracks, &self.settings.subtitle_language)
                                    .or_else(|| pl.sub_tracks.first())
                            {
                                pl.mpv.set_sub_track(track.id);
                                pl.current_sub = track.id;
                                pl.subtitle_preference_applied = true;
                            }
                        }
                    }
                }
            }
        }

        // Keyboard shortcuts (list, preview and full player).
        let now = ctx.input(|i| i.time);
        if self.player.is_some() && now - self.last_progress_save > 15.0 {
            self.save_user_data();
            self.last_progress_save = now;
        }
        self.handle_keys(ctx);

        self.upload_ready_images(ctx);

        if self.autostart_test && (self.main_hwnd != 0 || cfg!(target_os = "ios")) {
            self.autostart_test = false;
            self.play_test_stream();
        }

        // Hardware and platform remote keys share the same menu navigation.
        let mut keys = crate::platform::drain_remote_keys();
        #[cfg(windows)]
        keys.extend(ctx.data_mut(|data| {
            let id = egui::Id::new("desktop-remote-keys");
            let keys = data
                .get_temp::<Vec<crate::platform::RemoteKey>>(id)
                .unwrap_or_default();
            data.remove::<Vec<crate::platform::RemoteKey>>(id);
            keys
        }));
        if !crate::platform::text_focus() && !ctx.wants_keyboard_input() {
            keys.retain(|key| {
                use crate::platform::RemoteKey;
                let color = match key {
                    RemoteKey::Red => egui::Key::F1,
                    RemoteKey::Green => egui::Key::F2,
                    RemoteKey::Yellow => egui::Key::F3,
                    RemoteKey::Blue => egui::Key::F4,
                    _ => return true,
                };
                crate::ui::desktop::color_shortcut(self, ctx, color);
                false
            });
        }
        let recall = keys.contains(&crate::platform::RemoteKey::Recall)
            || ctx.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: egui::Key::Num0,
                            pressed: true,
                            repeat: false,
                            ..
                        }
                    )
                })
            });
        keys.retain(|key| *key != crate::platform::RemoteKey::Recall);
        let player_options_open =
            ui::player_desktop::menus_open(ctx) || self.audio_menu_open || self.sub_menu_open;
        if recall
            && !ctx.wants_keyboard_input()
            && !self.parental.open
            && !player_options_open
            && (self.view == View::Live
                || (self.view == View::Player
                    && self
                        .current_play
                        .as_ref()
                        .is_some_and(|item| item.kind == PlayKind::Live)))
        {
            self.recall_channel();
        }
        if self.view == View::Player
            && self
                .current_play
                .as_ref()
                .is_some_and(|p| p.kind == PlayKind::Live)
            && !ctx.wants_keyboard_input()
            && !self.parental.open
            && !player_options_open
        {
            use crate::platform::RemoteKey;
            if keys.contains(&RemoteKey::Ok) {
                self.open_live_list();
                ctx.request_repaint();
            }
            for key in &keys {
                if self.view != View::Player {
                    break;
                }
                match key {
                    RemoteKey::Up => self.step_channel(1),
                    RemoteKey::Down => self.step_channel(-1),
                    RemoteKey::Left | RemoteKey::Right => {
                        let delta = if *key == RemoteKey::Left { -1.0 } else { 1.0 };
                        if let Some(player) = &self.player {
                            player
                                .mpv
                                .seek_relative(delta * crate::release_info::SEEK_STEP_SECONDS);
                        }
                        self.ui_activity = ctx.input(|i| i.time);
                    }
                    _ => {}
                }
            }
            keys.retain(|k| {
                !matches!(
                    k,
                    RemoteKey::Up
                        | RemoteKey::Down
                        | RemoteKey::Left
                        | RemoteKey::Right
                        | RemoteKey::Ok
                )
            });
        }
        if (self.tv || self.view == View::Player) && keys.is_empty() {
            use crate::platform::RemoteKey;
            for (key, remote) in [
                (egui::Key::ArrowUp, RemoteKey::Up),
                (egui::Key::ArrowDown, RemoteKey::Down),
                (egui::Key::ArrowLeft, RemoteKey::Left),
                (egui::Key::ArrowRight, RemoteKey::Right),
                (egui::Key::Enter, RemoteKey::Ok),
            ] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                    keys.push(remote);
                }
            }
        }
        if self.view == View::Player
            && self
                .current_play
                .as_ref()
                .is_some_and(|p| p.kind != PlayKind::Live)
            && !player_options_open
        {
            use crate::platform::RemoteKey;
            let focus = egui::Id::new("player-focus-transport");
            let mut focus_transport =
                ctx.data(|data| data.get_temp::<bool>(focus).unwrap_or(false));
            if !self.player_chrome && !focus_transport {
                self.player_menu_navigation = false;
            }
            for key in &keys {
                if matches!(key, RemoteKey::Up | RemoteKey::Down) && !self.player_menu_navigation {
                    self.enter_player_navigation(ctx);
                    focus_transport = true;
                } else if !self.player_menu_navigation
                    && matches!(key, RemoteKey::Left | RemoteKey::Right)
                {
                    self.ui_activity = ctx.input(|i| i.time);
                    self.player_reveal = false;
                    if let Some(pl) = &self.player {
                        pl.mpv.seek_relative(if *key == RemoteKey::Left {
                            -crate::release_info::SEEK_STEP_SECONDS
                        } else {
                            crate::release_info::SEEK_STEP_SECONDS
                        });
                    }
                }
            }
            if !self.player_menu_navigation {
                keys.retain(|k| !matches!(k, RemoteKey::Left | RemoteKey::Right));
            }
            if focus_transport {
                keys.retain(|k| !matches!(k, RemoteKey::Up | RemoteKey::Down));
            }
        }
        // Any interaction with the player brings the chrome back and restarts
        // the auto-hide timer. Remote keys are drained here, before `draw()`,
        // so the player UI can no longer see them itself. Without this a TV
        // remote would hide the controls once and never be able to reveal them
        // again (the old Android workaround pinned the chrome forever, which
        // is exactly the "menu never goes away" report).
        if self.view == View::Player
            && (!keys.is_empty() || ctx.input(|i| i.pointer.any_pressed() || i.pointer.any_down()))
        {
            // Chrome was already gone before this input: the key/tap may only
            // reveal it. The player drops the activation for that one frame.
            if !self.player_chrome {
                self.player_reveal = true;
            }
            self.ui_activity = ctx.input(|i| i.time);
        }
        let view_before_draw = self.view;
        let current_play_before_draw = self
            .current_play
            .as_ref()
            .map(|item| item.progress_key.clone());
        let pending_resume_before_draw =
            self.pending_resume.as_ref().map(|(key, _, _)| key.clone());
        let returning_to_catalog = self.desktop_return_item.is_some() && self.view != View::Player;
        let mobile = cfg!(any(target_os = "android", target_os = "ios"))
            || self.shared_remote_navigation()
            || (cfg!(windows) && (self.view == View::Player || self.parental.open));
        if mobile {
            // Screen fingerprint: a new screen restarts focus at its first
            // control. View is Copy, so `as u64` is legal.
            // The TV section is part of the fingerprint: switching headers
            // changes the whole control set, so focus must restart (and the
            // queued header index from tv_goto() then wins over that reset).
            let setting_page = self.settings_section.map_or(0, |index| index as u64 + 1);
            let screen: u64 = (setting_page << 20)
                | (self.view as u64) << 10
                | ((self.tv_nav as u64) & 0x7) << 7
                | (self.tv_series.is_some() as u64) << 6
                | (self.tv as u64) << 5
                | (self.active_profile.is_none() as u64) << 4
                | (self.mobile_more as u64) << 3
                | (self.pending_delete.is_some() as u64) << 2
                | (self.parental.open as u64) << 1
                | self.catalog_detail().is_some() as u64;
            let pointer_used = ctx.input(|i| i.pointer.any_pressed() || i.pointer.any_down());
            let in_shell = self.active_profile.is_some() && self.view != View::Player;
            let modal = (!in_shell && self.pending_delete.is_some())
                || self.parental.open
                || self.pending_resume.is_some()
                || self.description_open.is_some()
                || (self.shared_remote_navigation() && ui::desktop::exit_prompt(ctx));

            // PlayPause is a global media key. On the player it must only toggle
            // pause, not also select the focused control: focus restarts at idx 0
            // (the Back button) on each new screen, so letting it set pending_ok
            // would stop playback. Keep it out of the focus engine there.
            let in_player = self.view == View::Player;
            // A focused login field owns the keyboard. D-pad/OK belong to the
            // IME (or to the hardware keyboard), NOT to the focus engine.
            // Without this, `Enter` while typing activates control 0 — which,
            // with the keyboard up, is "Anmelden" — and submits a half-filled
            // form. That is the "it flickers and nothing happens, I cannot get
            // past login" report: the validation error pops in, the action bar
            // grows by 40 pt, and the whole form jumps.
            let text_focus = crate::platform::text_focus();
            if text_focus {
                crate::ui::mobile::focus::clear_activation();
            }
            let play_pause = !text_focus && keys.contains(&crate::platform::RemoteKey::PlayPause);
            let focus_keys: Vec<crate::platform::RemoteKey> = if text_focus {
                Vec::new()
            } else if in_player {
                keys.iter()
                    .copied()
                    .filter(|k| *k != crate::platform::RemoteKey::PlayPause)
                    .collect()
            } else {
                keys.clone()
            };
            // Spatial focus is for the 10-foot *browse* shell. Login stays
            // linear (a form with no horizontal neighbours) and so does the
            // player (one row of transport controls).
            let spatial = in_player || self.shared_remote_navigation();
            crate::ui::mobile::focus::begin(&focus_keys, screen, pointer_used, modal, spatial);

            if in_player && play_pause {
                if let Some(pl) = &self.player {
                    pl.mpv.toggle_pause();
                }
            }

            self.draw(ctx);
            crate::ui::mobile::focus::end();
        } else {
            self.draw(ctx);
        }
        #[cfg(windows)]
        {
            if view_before_draw != View::Player && self.view == View::Player {
                self.desktop_origin_focus = ctx.memory(|m| m.focused());
            }
            if returning_to_catalog {
                if self.desktop_return_item.take().is_some() {
                    if let Some(origin) = self.desktop_origin_focus {
                        ctx.memory_mut(|m| m.request_focus(origin));
                    }
                }
                self.desktop_origin_focus = None;
                ctx.request_repaint();
            }
        }
        #[cfg(not(windows))]
        if returning_to_catalog && self.desktop_return_item.take().is_some() {
            ctx.request_repaint();
        }
        #[cfg(target_os = "ios")]
        crate::platform::ios::bridge::end_frame(ctx);
        if self.view != view_before_draw
            || self
                .current_play
                .as_ref()
                .map(|item| item.progress_key.clone())
                != current_play_before_draw
            || self.pending_resume.as_ref().map(|(key, _, _)| key.clone())
                != pending_resume_before_draw
        {
            ctx.request_repaint();
        }

        if let Some(v) = self.pending_fullscreen.take() {
            self.fullscreen = v;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(v));
        }

        // The video surface is a native child window drawn on top of egui.
        // Hide it again as soon as it was not placed in this frame.
        if self.preview.is_some() && !self.preview_placed {
            if let Some(pl) = &self.player {
                pl.surface.hide();
            }
        }
        if self.player.is_some() {
            if self.view == View::Player {
                // vo=libmpv only presents a frame when the client calls
                // mpv_render_context_render() from the egui paint callback.
                // A fixed 120 ms timer capped video at ~8 fps (below the 10 fps
                // test clip and far below live TV's 25/50 fps) and filled
                // mpv.log with "mpv_render_context_render() not being called
                // or stuck." Drive a repaint every frame while the player is
                // actually visible; outside the player view a slow poll is
                // enough for progress / EPG updates.
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(400));
            }
        }

        // winit's Android `set_ime_allowed()` is a no-op; drive the soft
        // keyboard from egui's own IME request instead (set by TextEdit).
        #[cfg(target_os = "android")]
        {
            // `ctx.output().ime` is a per-frame request, not a stable state. A
            // repaint for any other reason (an inset pass, a scroll, a network
            // event) can produce one frame with `ime == None` while the login
            // field is still focused. Hiding on that transient None dismissed
            // the keyboard mid-typing: the next finger tap landed on the UI
            // underneath, which on this screen is "Anmelden" — the reported
            // "it flickers and nothing happens, I cannot get past login".
            // `text_focus()` comes from the real TextEdit response this frame
            // and is the reliable signal that text entry is still in progress.
            let wants = ctx.output(|o| o.ime.is_some()) || crate::platform::text_focus();
            crate::platform::android::jni_bridge::set_keyboard(wants);
            // The IME inset is pushed from `onApplyWindowInsets` a few frames
            // after the keyboard starts animating, and egui only redraws on
            // input — so without this the layout would stay frozen with the
            // action bar still under the keyboard. Poll until the keyboard is
            // both closed and no longer reported.
            if wants || crate::platform::insets().ime > 0.0 {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    }

    fn draw(&mut self, ctx: &Context) {
        if !matches!(self.catalog_detail(), Some((true, _))) {
            self.cancel_episode_frame();
        }
        #[cfg(target_os = "windows")]
        if self.parental.open || self.view != View::Player || self.palette_open {
            self.desktop_overlay.hide();
        }
        #[cfg(target_os = "windows")]
        if crate::licensing::enabled() && !crate::licensing::allowed() {
            if self.current_play.is_some() {
                self.stop_play();
            }
            if self.preview.is_some() {
                self.close_preview();
            }
            crate::licensing::blocked_screen(ctx);
            return;
        }
        // Leaving the channel list closes the preview (and stops its audio).
        if self.view != View::Live && self.preview.is_some() {
            self.close_preview();
        }

        crate::ui::desktop::show(self, ctx);
        if self.parental.open {
            crate::ui::parental::show(self, ctx);
            return;
        }

        if let Some(series_id) = self.catalog_resume_series {
            if self.catalog_detail() != Some((true, series_id)) || self.series_failed(series_id) {
                self.catalog_resume_series = None;
            } else if self.series_episodes_for(series_id).is_some() {
                self.catalog_resume_series = None;
                if let Some((episode, position)) = self.series_resume_target(series_id) {
                    if position >= crate::release_info::RESUME_MINIMUM_SECONDS {
                        self.ask_resume(&format!("ep:{series_id}:{}", episode.id));
                    }
                }
            }
        }
        ui::continue_watching::resume_dialog(self, ctx);
        ui::library::description_dialog(self, ctx);
        ui::palette::show(self, ctx);
    }

    fn handle_event(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::LoginDone(generation, res) => {
                // A slow request from before logout or a newer login must never
                // replace the currently selected profile/library.
                if generation != self.login_generation {
                    return;
                }
                self.busy = false;
                match *res {
                    Ok((profile, lib)) => {
                        if let Some(existing) =
                            self.profiles.iter_mut().find(|p| p.id == profile.id)
                        {
                            *existing = profile.clone();
                        } else {
                            self.profiles.push(profile.clone());
                        }
                        let _ = store::save_profiles(&self.profiles);
                        self.switch_user_profile(&profile.id);
                        self.settings.default_profile = Some(profile.id.clone());
                        self.settings.signed_out = false;
                        self.save_settings_now();
                        self.active_profile = Some(profile);
                        self.library = lib;
                        self.reset_profile_catalog_state();
                        self.view = View::Home;
                        self.error = None;
                        self.clear_detail_caches();
                    }
                    Err(e) => {
                        self.error = Some(redact(&e));
                    }
                }
            }
            AppEvent::CatalogRefreshed(generation, result) => {
                if generation != self.login_generation {
                    return;
                }
                self.catalog_refreshing = false;
                let fully_refreshed = result
                    .as_ref()
                    .as_ref()
                    .is_ok_and(|library| library.catalog_issues.is_empty());
                self.catalog_refresh_result = Some((fully_refreshed, std::time::Instant::now()));
                let language = self.settings.language;
                match *result {
                    Ok(mut library) => {
                        use crate::types::CatalogArea::*;
                        for issue in &library.catalog_issues {
                            match issue.area {
                                LiveCategories => {
                                    library.live_categories =
                                        std::mem::take(&mut self.library.live_categories)
                                }
                                LiveStreams => {
                                    library.channels = std::mem::take(&mut self.library.channels)
                                }
                                MovieCategories => {
                                    library.vod_categories =
                                        std::mem::take(&mut self.library.vod_categories)
                                }
                                Movies => library.movies = std::mem::take(&mut self.library.movies),
                                SeriesCategories => {
                                    library.series_categories =
                                        std::mem::take(&mut self.library.series_categories)
                                }
                                Series => library.series = std::mem::take(&mut self.library.series),
                            }
                        }
                        self.login_generation = self.login_generation.wrapping_add(1);
                        self.library = library;
                        self.clear_detail_caches();
                        self.images = ImageState::default();
                        self.live_order.clear();
                        self.catalog_refresh_notice = Some(
                            if self.library.catalog_issues.is_empty() {
                                language.text("Liste aktualisiert", "Library refreshed")
                            } else {
                                language.text("Teilweise aktualisiert", "Partially refreshed")
                            }
                            .into(),
                        );
                    }
                    Err(error) => {
                        self.catalog_refresh_notice = Some(format!(
                            "{}: {}",
                            language.text("Aktualisierung fehlgeschlagen", "Refresh failed"),
                            redact(&error)
                        ));
                    }
                }
            }
            AppEvent::Epg(generation, id, res) => {
                // Ignore completions from a previous profile. IDs are provider-scoped,
                // so accepting a late response can attach old EPG data to a new catalog.
                if generation != self.login_generation {
                    return;
                }
                self.epg_requested.remove(&id);
                match res {
                    Ok(mut entries) => {
                        entries.truncate(MAX_DETAIL_ROWS);
                        self.epg_failed.remove(&id);
                        self.epg_error = None;
                        insert_bounded_detail(&mut self.epg, &mut self.epg_order, id, entries);
                    }
                    Err(e) => {
                        // An empty Vec used to be stored on failure, which made a
                        // network error indistinguishable from "no EPG available".
                        mark_detail_failed(&mut self.epg_failed, id);
                        self.epg_error = Some(redact(&e));
                    }
                }
            }
            AppEvent::SeriesInfo(generation, id, res) => {
                // Ignore completions from a previous profile. Series IDs can overlap
                // between providers just like channel IDs do.
                if generation != self.login_generation {
                    return;
                }
                self.series_requested.remove(&id);
                match res {
                    Ok(mut episodes) => {
                        episodes.truncate(MAX_DETAIL_ROWS);
                        self.series_failed.remove(&id);
                        self.series_error = None;
                        insert_bounded_detail(
                            &mut self.series_episodes,
                            &mut self.series_order,
                            id,
                            episodes,
                        );
                        if self
                            .pending_history_key
                            .as_ref()
                            .is_some_and(|key| key.starts_with(&format!("ep:{id}:")))
                        {
                            let key = self.pending_history_key.take().unwrap();
                            self.play_history_key(&key);
                        }
                    }
                    Err(e) => {
                        mark_detail_failed(&mut self.series_failed, id);
                        self.series_error = Some(redact(&e));
                        if self
                            .pending_history_key
                            .as_ref()
                            .is_some_and(|key| key.starts_with(&format!("ep:{id}:")))
                        {
                            self.pending_history_key = None;
                            self.info = Some(
                                "Die Folge konnte nicht geladen werden. Bitte erneut versuchen."
                                    .into(),
                            );
                        }
                    }
                }
            }
            AppEvent::EpisodeFrame(generation, key, result) => {
                let cancelled = self
                    .episode_frame_request
                    .take()
                    .is_none_or(|token| token.load(std::sync::atomic::Ordering::Relaxed));
                if !cancelled {
                    self.handle_event(AppEvent::Image(
                        generation,
                        key,
                        result.ok_or_else(|| "No episode frame".into()),
                    ));
                }
            }
            AppEvent::Image(generation, key, res) => {
                if generation != self.login_generation {
                    return;
                }
                self.images.requested.remove(&key);
                match res {
                    Ok(image) => self.images.pending.push((key, image)),
                    Err(_) => {
                        if self.images.failed.insert(key.clone()) {
                            self.images.failed_order.push_back(key);
                        }
                        while self.images.failed_order.len() > 300 {
                            if let Some(old) = self.images.failed_order.pop_front() {
                                self.images.failed.remove(&old);
                            }
                        }
                    }
                }
            }
        }
    }

    // ---- login ----

    pub fn catalog_generation(&self) -> u64 {
        self.login_generation
    }

    pub fn start_login(&mut self, profile: Profile) {
        self.catalog_refreshing = false;
        self.catalog_refresh_notice = None;
        self.catalog_refresh_result = None;
        if self.profiles.iter().any(|p| p.id == profile.id) {
            self.editing_profile = Some(profile.id.clone());
            self.login_kind = profile.kind;
            self.f_name = profile.name.clone();
            self.f_host = profile.host.clone();
            self.f_user = profile.username.clone();
            self.f_pass = profile.password.clone();
            self.f_m3u_url = profile.m3u_url.clone();
            self.f_m3u_file = profile.m3u_file.clone();
        }
        self.login_generation = self.login_generation.wrapping_add(1);
        let generation = self.login_generation;
        self.busy = true;
        self.error = None;
        self.info = None;
        let tx = self.tx.clone();
        let to = self.settings.network_timeout_s;
        let live_ext = self.settings.live_format.extension().to_string();
        self.rt.spawn(async move {
            let result = client::load_profile(&profile, to, &live_ext)
                .await
                .map(|lib| (profile, lib))
                .map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::LoginDone(generation, Box::new(result)));
        });
    }

    /// Reset state that is scoped to the currently loaded provider catalogue.
    /// Keeping a category id from a different profile can make VOD/Series look
    /// empty even though the newly loaded library contains items.
    fn reset_profile_catalog_state(&mut self) {
        self.pending_history_key = None;
        self.live_cat = "all".to_string();
        self.vod_cat = "all".to_string();
        self.series_cat = "all".to_string();
        self.search_q.clear();
        self.live_q.clear();
        self.catalog_queries = [String::new(), String::new()];
        self.nav_collapsed = false;
        self.description_open = None;
        self.selected_movie = None;
        self.catalog_return_poster = None;
        self.pending_resume = None;
        self.catalog_resume_series = None;
        self.live_selected = None;
        self.previous_live = None;
        self.selected_series = None;
        self.series_season = 0;
        self.live_page = 0;
        self.vod_page = 0;
        self.series_page = 0;
        self.epg_offset_hours = 0;
        self.tv_series = None;
        self.tv_search.clear();
        self.images = ImageState::default();
    }

    fn invalidate_login(&mut self) {
        self.catalog_refreshing = false;
        self.catalog_refresh_notice = None;
        self.catalog_refresh_result = None;
        self.login_generation = self.login_generation.wrapping_add(1);
        self.busy = false;
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn test_app() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut app = Self::new(rt);
        app.settings.interface_layout = crate::types::InterfaceLayout::Modern;
        app
    }

    fn form_profile(&mut self) -> Option<Profile> {
        let name = if self.f_name.trim().is_empty() {
            match self.login_kind {
                ProfileKind::Xtream => self.f_host.trim().to_string(),
                ProfileKind::M3u => {
                    if self.f_m3u_url.trim().is_empty() {
                        format!("Profil {}", self.profiles.len() + 1)
                    } else {
                        self.f_m3u_url.trim().to_string()
                    }
                }
            }
        } else {
            self.f_name.trim().to_string()
        };
        let mut p = Profile::new(&name, self.login_kind);
        match self.login_kind {
            ProfileKind::Xtream => {
                if self.f_host.trim().is_empty()
                    || self.f_user.trim().is_empty()
                    || self.f_pass.is_empty()
                {
                    self.error = Some("Host, Benutzer und Passwort sind erforderlich.".to_string());
                    return None;
                }
                p.host = self.f_host.trim().to_string();
                p.username = self.f_user.trim().to_string();
                p.password = self.f_pass.clone();
            }
            ProfileKind::M3u => {
                if self.f_m3u_url.trim().is_empty() && self.f_m3u_file.trim().is_empty() {
                    self.error = Some("M3U-URL oder Datei angeben.".to_string());
                    return None;
                }
                p.m3u_url = self.f_m3u_url.trim().to_string();
                p.m3u_file = self.f_m3u_file.trim().to_string();
            }
        }
        if let Some(id) = self.editing_profile.as_ref() {
            p.id = id.clone();
        }
        Some(p)
    }

    pub fn submit_form(&mut self) {
        if let Some(profile) = self.form_profile() {
            self.start_login(profile);
        }
    }

    pub(crate) fn begin_profile_add(&mut self) {
        self.adding_profile = true;
        self.editing_profile = None;
        self.error = None;
        self.login_kind = ProfileKind::Xtream;
        self.show_password = false;
        for field in [
            &mut self.f_name,
            &mut self.f_host,
            &mut self.f_user,
            &mut self.f_pass,
            &mut self.f_m3u_url,
            &mut self.f_m3u_file,
        ] {
            field.clear();
        }
    }

    pub(crate) fn save_added_profile(&mut self) {
        if let Some(mut profile) = self.form_profile() {
            if self.f_name.trim().is_empty() {
                profile.name = format!("Profil {}", self.profiles.len() + 1);
            }
            self.profiles.push(profile);
            if store::save_profiles(&self.profiles).is_err() {
                self.profiles.pop();
                self.error = Some("Profil konnte nicht gespeichert werden.".into());
                return;
            }
            self.adding_profile = false;
            self.error = None;
            self.f_pass.clear();
        }
    }

    pub fn delete_profile(&mut self, id: &str) {
        if self.settings.default_profile.as_deref() == Some(id) {
            self.settings.default_profile = None;
            self.settings.signed_out = true;
            self.save_settings_now();
        }
        self.profiles.retain(|p| p.id != id);
        let _ = store::save_profiles(&self.profiles);
    }

    /// Enter a TV section and remember where we came from.
    ///
    /// The left headers column is a real forward/back menu: every section we
    /// leave goes on the stack, so BACK retraces the path instead of dumping
    /// the user on "Live TV" no matter where they were.
    pub fn navigate(&mut self, view: View) {
        if view == View::Continue && !self.settings.watch_history {
            return;
        }
        if view != View::Settings || self.view != View::Settings {
            self.adding_profile = false;
            self.settings_section = None;
            self.settings_return_tile = None;
        }
        if self.tv {
            self.tv_goto(Self::tv_section(view));
        }
        self.view = view;
    }

    fn tv_section(view: View) -> usize {
        match view {
            View::Live => 0,
            View::Movies => 1,
            View::Series => 2,
            View::Continue => 3,
            View::Favorites => 4,
            View::Search => 5,
            View::Settings => 6,
            _ => 7,
        }
    }
    pub fn tv_goto(&mut self, nav: usize) {
        if nav == self.tv_nav {
            return;
        }
        self.tv_stack.push(self.tv_nav);
        if self.tv_stack.len() > 24 {
            self.tv_stack.remove(0);
        }
        self.tv_nav = nav;
        // Each section keeps its own category, selection and scroll identity.
        // The headers register first, so header `nav` is control index `nav`.
        crate::ui::mobile::focus::request_index(nav);
    }

    /// One step back. Returns false when there is nothing to pop (the platform
    /// may then close the app). Used by Android's back key.
    pub fn mobile_can_back(&self) -> bool {
        if self.parental.open {
            return true;
        }
        if (self.view == View::Settings && self.settings_section.is_some())
            || self.description_open.is_some()
            || (self.tv && self.preview.is_some())
            || self.pending_delete.is_some()
            || self.mobile_more
            || self.view == View::Player
            || (self.view == View::Series && self.selected_series.is_some())
            || (self.view == View::Movies && self.selected_movie.is_some())
        {
            return true;
        }
        if self.tv && self.active_profile.is_some() {
            let in_content = crate::ui::mobile::focus::index().unwrap_or(0)
                >= crate::ui::mobile::tv::HEADER_COUNT;
            return !self.tv_stack.is_empty() || self.tv_series.is_some() || in_content;
        }
        self.view != View::Live && self.view != View::Home && self.view != View::Login
    }

    pub(crate) fn close_settings_section(&mut self) -> bool {
        if self.view != View::Settings {
            return false;
        }
        if self.adding_profile {
            self.adding_profile = false;
            self.error = None;
            self.f_pass.clear();
            return true;
        }
        self.settings_return_tile = self.settings_section.take();
        self.settings_return_tile.is_some()
    }

    pub(crate) fn catalog_detail(&self) -> Option<(bool, u32)> {
        match self.view {
            View::Series => self.selected_series.map(|id| (true, id)),
            View::Movies => self.selected_movie.map(|id| (false, id)),
            _ => None,
        }
    }

    pub(crate) fn close_catalog_detail(&mut self) -> bool {
        let Some((series, _)) = self.catalog_detail() else {
            return false;
        };
        if series {
            self.selected_series = None;
        } else {
            self.selected_movie = None;
        }
        self.catalog_focus_play = false;
        self.catalog_resume_series = None;
        self.cancel_episode_frame();
        true
    }

    pub(crate) fn clear_app_cache(&mut self) -> bool {
        if self.busy || self.catalog_refreshing {
            return false;
        }
        // Reject outstanding image, EPG and episode responses from before the clear.
        self.login_generation = self.login_generation.wrapping_add(1);
        self.images = ImageState::default();
        self.clear_detail_caches();
        self.pending_history_key = None;
        true
    }

    pub fn mobile_back(&mut self) -> bool {
        if self.parental.open {
            self.parental.close();
            return true;
        }
        if self.description_open.take().is_some() {
            return true;
        }
        // A destructive confirm owns the back key: back cancels it and goes no
        // further. Without this the key would close the activity underneath the
        // dialog, which reads as "confirm".
        if self.pending_delete.is_some() {
            self.pending_delete = None;
            return true;
        }
        if self.pending_resume.take().is_some() {
            return true;
        }
        if self.close_settings_section() {
            return true;
        }
        if self.mobile_more {
            self.mobile_more = false;
            return true;
        }
        if self.mobile_epg_channel.is_some() {
            self.mobile_epg_channel = None;
            return true;
        }
        if self.view == View::Player {
            self.stop_play();
            return true;
        }
        if self.tv && self.preview.is_some() {
            self.close_preview();
            return true;
        }
        if self.close_catalog_detail() {
            return true;
        }
        if self.tv && self.active_profile.is_some() {
            // 1. An open series rail closes first.
            if self.tv_series.is_some() {
                self.tv_series = None;
                crate::ui::mobile::focus::request_index(self.tv_nav);
                return true;
            }
            // 2. Retrace visited sections.
            if let Some(prev) = self.tv_stack.pop() {
                self.tv_nav = prev;
                crate::ui::mobile::focus::request_index(prev);
                return true;
            }
            // 3. From the content area, step back onto the headers column.
            // Only when the stack is empty and focus is already on a header
            // does BACK mean "leave the app".
            if crate::ui::mobile::focus::index().unwrap_or(0) >= crate::ui::mobile::tv::HEADER_COUNT
            {
                crate::ui::mobile::focus::request_index(self.tv_nav);
                return true;
            }
            return false;
        }
        if self.view != View::Live && self.view != View::Home && self.view != View::Login {
            self.view = View::Home;
            return true;
        }
        false
    }

    pub fn logout(&mut self) {
        self.editing_profile = None;
        self.f_name.clear();
        self.f_host.clear();
        self.f_user.clear();
        self.f_pass.clear();
        self.f_m3u_url.clear();
        self.f_m3u_file.clear();
        self.settings.signed_out = true;
        self.save_settings_now();
        #[cfg(target_os = "android")]
        crate::platform::android::render::cancel_load();
        self.invalidate_login();
        self.reset_profile_catalog_state();
        self.active_profile = None;
        self.library = Library::default();
        self.view = View::Login;
        self.current_play = None;
        self.mobile_epg_channel = None;
        self.clear_detail_caches();
    }

    /// Drop all provider-scoped detail data when the active profile changes.
    /// Channel/series ids are provider-local, so keeping them would show the
    /// wrong programme or the wrong episode list after switching accounts.
    fn clear_detail_caches(&mut self) {
        self.epg.clear();
        self.epg_order.clear();
        self.epg_requested.clear();
        self.epg_failed.clear();
        self.epg_error = None;
        self.series_episodes.clear();
        self.series_order.clear();
        self.series_requested.clear();
        self.series_failed.clear();
        self.series_error = None;
    }

    // ---- playback ----

    /// Remember an item in the recently-watched list (newest first).
    pub fn visible_history(&self) -> &[HistEntry] {
        if self.settings.watch_history {
            &self.history
        } else {
            &[]
        }
    }

    fn record_history(&mut self, item: &PlayItem) {
        if !self.settings.watch_history {
            return;
        }
        let key = item.progress_key.clone();
        if key.is_empty() {
            return;
        }
        let (title, sub) = self
            .describe_key(&key)
            .unwrap_or_else(|| (item.title.clone(), String::new()));
        let ts = chrono::Utc::now().timestamp();
        self.history.retain(|h| h.key != key);
        self.history.insert(
            0,
            HistEntry {
                key,
                ts,
                title,
                sub,
            },
        );
        self.history.truncate(crate::release_info::HISTORY_LIMIT);
        self.save_user_data();
    }

    /// Human readable title for a progress key ("live:12", "vod:7", "ep:3:9").
    pub fn describe_key(&self, key: &str) -> Option<(String, String)> {
        if let Some(rest) = key.strip_prefix("live:") {
            let id = rest.parse::<u32>().ok()?;
            let c = self.library.channels.iter().find(|c| c.id == id)?;
            return Some((c.name.clone(), "Live-TV".to_string()));
        }
        if let Some(rest) = key.strip_prefix("vod:") {
            let id = rest.parse::<u32>().ok()?;
            let m = self.library.movies.iter().find(|m| m.id == id)?;
            return Some((m.name.clone(), "Film".to_string()));
        }
        if let Some(rest) = key.strip_prefix("series:") {
            let id = rest.parse::<u32>().ok()?;
            let series = self.library.series.iter().find(|s| s.id == id)?;
            return Some((series.name.clone(), "Serie".into()));
        }
        if let Some(rest) = key.strip_prefix("ep:") {
            let mut it = rest.split(':');
            let sid = it.next()?.parse::<u32>().ok()?;
            let eid = it.next()?.parse::<u32>().ok()?;
            let series = self.library.series.iter().find(|s| s.id == sid);
            let name = series.map(|s| s.name.clone()).unwrap_or_default();
            let ep = self
                .series_episodes
                .get(&sid)
                .and_then(|v| v.iter().find(|e| e.id == eid));
            let sub = match ep {
                Some(e) => format!("Staffel {} · Folge {}", e.season, e.episode),
                None => "Serie".to_string(),
            };
            return Some((name, sub));
        }
        None
    }

    /// Build a playable item for a progress key ("live:12", "vod:7", "ep:3:9").
    /// Used by the phone UI for favorites and the continue-watching list.
    pub fn resolve_play(&self, key: &str) -> Option<PlayItem> {
        if let Some(rest) = key.strip_prefix("live:") {
            let id: u32 = rest.parse().ok()?;
            let c = self.library.channels.iter().find(|c| c.id == id)?;
            return Some(PlayItem {
                title: c.name.clone(),
                url: c.stream_url.clone(),
                kind: PlayKind::Live,
                progress_key: key.to_string(),
            });
        }
        if let Some(rest) = key.strip_prefix("vod:") {
            let id: u32 = rest.parse().ok()?;
            let m = self.library.movies.iter().find(|m| m.id == id)?;
            return Some(PlayItem {
                title: m.name.clone(),
                url: m.stream_url.clone(),
                kind: PlayKind::Vod,
                progress_key: key.to_string(),
            });
        }
        if let Some(rest) = key.strip_prefix("ep:") {
            let mut it = rest.split(':');
            let sid: u32 = it.next()?.parse().ok()?;
            let eid: u32 = it.next()?.parse().ok()?;
            let e = self
                .series_episodes
                .get(&sid)?
                .iter()
                .find(|e| e.id == eid)?;
            let name = self
                .library
                .series
                .iter()
                .find(|s| s.id == sid)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            return Some(PlayItem {
                title: format!("{} · S{}F{}", name, e.season, e.episode),
                url: e.stream_url.clone(),
                kind: PlayKind::Episode,
                progress_key: key.to_string(),
            });
        }
        None
    }

    /// On Android, issue the queued `loadfile` once the render context exists.
    ///
    /// `mpv_render_context_create()` can only run with a current GL context,
    /// which this app has inside the paint callback. libmpv/render.h requires
    /// that context *before* playback starts (loadfile is what creates the VO),
    /// so `play()` queues the URL and this drains it on the first `update()`
    /// frame after the callback reported `READY`.
    #[cfg(target_os = "android")]
    fn flush_android_load(&mut self) {
        let Some((url, resume)) = crate::platform::android::render::take_ready_load() else {
            return;
        };
        let Some(pl) = &mut self.player else {
            return;
        };
        if !pl.mpv.is_vlc() {
            pl.mpv.load_after_render_ready(&url, resume);
        }
        eprintln!("[player] loadfile (render context ready)");
    }

    pub fn play(&mut self, item: PlayItem) {
        if !self.parental_gate(&item, false) {
            return;
        }
        self.play_authorized(item, false);
    }

    pub(crate) fn play_untracked(&mut self, item: PlayItem) {
        if !self.parental_gate(&item, false) {
            self.parental.untracked = true;
            return;
        }
        self.play_authorized(item, true);
    }

    fn play_authorized(&mut self, item: PlayItem, untracked: bool) {
        self.cancel_episode_frame();
        self.pending_history_key = None;
        #[cfg(target_os = "windows")]
        if crate::licensing::enabled() && !crate::licensing::allowed() {
            self.view = View::Settings;
            return;
        }
        self.remember_live(&item);
        self.preview = None;
        if self.view != View::Player {
            self.previous_view = self.view;
        }
        if item.kind == PlayKind::Live {
            let key = item.progress_key.clone();
            if let Some(p) = &mut self.active_profile {
                p.last_channel = Some(key.clone());
            }
            if let Some(id) = self.active_profile.as_ref().map(|p| p.id.clone()) {
                if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
                    p.last_channel = Some(key);
                }
            }
            let _ = store::save_profiles(&self.profiles);
        }

        let position = self.get_progress(&item.progress_key);
        let resume = if !untracked
            && item.kind != PlayKind::Live
            && position >= crate::release_info::RESUME_MINIMUM_SECONDS
        {
            position
        } else {
            0
        };

        self.untracked_playback = untracked;
        self.current_play = Some(item.clone());
        self.view = View::Player;
        // Force the player view to resize and reveal the native MPV child HWND
        // on its first frame, even if the previous preview used the same rect.
        self.last_video_rect = None;
        // `hide()` drops the native region; forget the cache so the next play
        // re-applies it instead of assuming it is still installed.
        self.last_video_cutouts = None;
        if !untracked {
            self.record_history(&item);
        }
        // Negative = "controls freshly opened, start the auto-hide timer now".
        self.ui_activity = -1.0;
        self.player_chrome = true;
        self.player_menu_navigation = false;
        self.player_reveal = false;

        if self.ensure_player() {
            let vol = self.volume;
            let radio = self.is_radio(&item.progress_key);
            if let Some(pl) = &mut self.player {
                pl.current_key = item.progress_key.clone();
                pl.mpv.configure(&self.settings);
                // Audio-only for radio stations (and back to video afterwards).
                if radio {
                    pl.mpv.disable_video();
                } else {
                    pl.mpv.enable_video();
                }
                // Android: `loadfile` must wait for the render context. It is
                // queued here and drained by flush_android_load() in update().
                pl.mpv.load_at(&item.url, resume as f64);
                pl.mpv.set_volume(vol);
                pl.mpv.set_pause(false);
                pl.position = 0.0;
                pl.duration = 0.0;
                pl.paused = false;
            }
            self.reset_track_state();
        }
    }

    /// "Ohne Anbieter testen": a locally generated test pattern, not a
    /// network stream. A video-output check needs a source that cannot fail
    /// for unrelated reasons (DNS, TLS, HLS playlist, a missing demuxer, or
    /// a `loadfile`-options parser that rejects the geometry string).
    ///
    /// The AVI is generated **at runtime** into the app's private files dir
    /// (Android) / the temp dir (desktop) by `crate::test_pattern::write`, so
    /// the button also works on a clean install — nothing has to be pushed
    /// into the APK or the data dir by QA first.
    ///
    /// The Android libmpv build ships `libavdevice.so` with **no input
    /// devices at all** (no `lavfi`, `android_camera`, `v4l2`, …), so
    /// `av://lavfi:testsrc` is rejected with "Unknown lavf format lavfi". And
    /// its `loadfile` options argument rejects the rawvideo geometry
    /// ("option must be an integer"). So we hand mpv a self-describing
    /// **uncompressed RGB24 AVI**: the AVI demuxer (`libavformat.so`) reads
    /// width/height/pixel-format from the file header, no network, no options.
    /// A per-frame tint acts as a frame counter, so two captures taken a
    /// second or two apart prove *live* rendering rather than a frozen frame.
    ///
    /// See MOBILE-BAUPLAN.md §17 / §23 for the device-capability matrix.
    pub fn play_test_stream(&mut self) {
        // QA hook: let the same autostart path drive a *real* network/provider
        // stream instead of the local test pattern. A local uncompressed AVI
        // proves the video output exists, but it cannot exercise DNS, TLS,
        // HLS playlists or codecs - which is exactly where provider channels
        // fail. Kept out of the UI, only reachable via IPTV_TEST_URL.
        if let Ok(url) = std::env::var("IPTV_TEST_URL") {
            if !url.trim().is_empty() {
                self.preview = None;
                self.current_play = Some(PlayItem {
                    title: "Teststream".to_string(),
                    url: url.clone(),
                    kind: PlayKind::Vod,
                    progress_key: String::new(),
                });
                self.view = View::Player;
                self.last_video_rect = None;
                // `hide()` drops the native region; forget the cache so the next play
                // re-applies it instead of assuming it is still installed.
                self.last_video_cutouts = None;
                self.ui_activity = -1.0;
                self.player_chrome = true;
                self.player_menu_navigation = false;
                self.player_reveal = false;
                if self.ensure_player() {
                    if let Some(pl) = &self.player {
                        pl.mpv.load_at(&url, 0.0);
                        pl.mpv.set_volume(self.volume);
                        pl.mpv.set_pause(false);
                    }
                    self.reset_track_state();
                }
                return;
            }
        }

        #[cfg(target_os = "ios")]
        {
            self.play(PlayItem {
                title: "Testbild (lokal)".into(),
                url: "dado:demo".into(),
                kind: PlayKind::Vod,
                progress_key: "demo:video".into(),
            });
            return;
        }
        // Local video-output check: an uncompressed RGB24 AVI whose geometry
        // (96x72, rgb24, 10fps) lives in its own strf BITMAPINFOHEADER, so
        // `loadfile` needs no per-file options. This sidesteps two libmpv
        // Android quirks at once: the `lavfi` input device is absent from
        // `libavdevice.so` (so `av://...` fails), and `loadfile`'s options
        // argument is rejected by this build ("option must be an integer").
        // A per-frame blue tint makes two captures seconds apart provably live.
        // The file is generated here so a clean install needs no QA asset.
        #[cfg(target_os = "android")]
        let path = std::path::PathBuf::from(
            crate::platform::android::jni_bridge::storage_dir()
                .unwrap_or("/data/data/com.tarik.dadoplayer/files"),
        )
        .join("test.avi");
        #[cfg(not(target_os = "android"))]
        let path = std::env::temp_dir().join("dado-player-test.avi");

        if let Err(e) = crate::test_pattern::write(&path) {
            self.player_error = Some(format!("Testbild konnte nicht erzeugt werden: {e}"));
            return;
        }

        #[cfg(target_os = "android")]
        let url = format!("file://{}", path.display());
        #[cfg(not(target_os = "android"))]
        let url = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));

        self.preview = None;
        if !self.ensure_player() {
            return;
        }
        self.current_play = Some(PlayItem {
            title: "Testbild (lokal)".to_string(),
            url: url.clone(),
            kind: PlayKind::Vod,
            progress_key: String::new(),
        });
        self.view = View::Player;
        // Negative = "controls freshly opened, start the auto-hide timer now".
        self.ui_activity = -1.0;
        self.player_chrome = true;
        self.player_menu_navigation = false;
        self.player_reveal = false;
        let vol = self.volume;
        if let Some(pl) = &mut self.player {
            pl.current_key = String::new();
            pl.mpv.load_at(&url, 0.0);
            pl.mpv.set_volume(vol);
            pl.mpv.set_pause(false);
            pl.position = 0.0;
            pl.duration = 0.0;
            pl.paused = false;
        }
        self.reset_track_state();
        self.record_history(&self.current_play.clone().unwrap());
    }

    /// Start (or switch) the inline preview while the channel list stays visible.
    pub fn select_live_channel(&mut self, item: PlayItem) {
        if self.is_previewing(&item.progress_key) {
            self.promote_preview(false);
        } else {
            self.start_preview(item);
        }
    }

    pub fn start_preview(&mut self, item: PlayItem) {
        if !self.parental_gate(&item, true) {
            return;
        }
        self.remember_live(&item);
        if item.kind == PlayKind::Live {
            let key = item.progress_key.clone();
            if let Some(p) = &mut self.active_profile {
                p.last_channel = Some(key.clone());
            }
            if let Some(id) = self.active_profile.as_ref().map(|p| p.id.clone()) {
                if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
                    p.last_channel = Some(key);
                }
            }
            let _ = store::save_profiles(&self.profiles);
        }

        let same = self
            .preview
            .as_ref()
            .map(|p| p.url == item.url)
            .unwrap_or(false);
        self.preview = Some(item.clone());
        if same {
            return;
        }
        if !self.ensure_player() {
            return;
        }
        let vol = self.volume;
        let radio = self.is_radio(&item.progress_key);
        if let Some(pl) = &mut self.player {
            pl.current_key = item.progress_key.clone();
            pl.mpv.set_desktop_aspect(crate::types::DesktopAspect::Fit);
            // Audio-only for radio stations (and back to video afterwards).
            if radio {
                pl.mpv.disable_video();
            } else {
                pl.mpv.enable_video();
            }
            #[cfg(target_os = "ios")]
            crate::platform::ios::bridge::native_player(false);
            pl.mpv.load_at(&item.url, 0.0);
            pl.mpv.set_volume(vol);
            pl.mpv.set_pause(false);
            pl.position = 0.0;
            pl.duration = 0.0;
            pl.paused = false;
        }
        self.record_history(&item);
    }

    /// Close the preview and release the stream (list stays where it is).
    pub fn close_preview(&mut self) {
        // A queued load is still legal even after the user left: it would be
        // drained by the next frame where READY is true. Drop it on every exit
        // path, not just when the player happens to exist.
        #[cfg(target_os = "android")]
        crate::platform::android::render::cancel_load();
        if self.preview.is_none() {
            return;
        }
        if let Some(pl) = &self.player {
            pl.mpv.stop();
            pl.surface.hide();
        }
        self.preview = None;
        // Force a fresh SetWindowPos (it also shows the surface again).
        self.last_video_rect = None;
        // `hide()` drops the native region; forget the cache so the next play
        // re-applies it instead of assuming it is still installed.
        self.last_video_cutouts = None;
    }

    /// Move the running preview into the full player window.
    pub fn promote_preview(&mut self, fullscreen: bool) {
        if let Some(item) = self.preview.take() {
            self.previous_view = View::Live;
            #[cfg(target_os = "ios")]
            if self.settings.ios_native_player {
                self.play(item);
                return;
            }
            self.current_play = Some(item);
            if let Some(player) = &self.player {
                player.mpv.set_desktop_aspect(self.settings.desktop_aspect);
            }
            self.last_video_rect = None;
            self.last_video_cutouts = None;
            self.ui_activity = -1.0;
            self.view = View::Player;
            if fullscreen {
                self.pending_fullscreen = Some(true);
            }
            self.reset_track_state();
        }
    }

    /// Throw away the mpv instance so new decoder/output settings take effect.
    pub fn reset_player(&mut self) {
        #[cfg(target_os = "windows")]
        self.desktop_overlay.hide();
        #[cfg(target_os = "android")]
        crate::platform::android::render::cancel_load();
        self.preview = None;
        self.player = None;
        self.last_video_rect = None;
        // `hide()` drops the native region; forget the cache so the next play
        // re-applies it instead of assuming it is still installed.
        self.last_video_cutouts = None;
        if self.view == View::Player {
            self.current_play = None;
            self.view = self.previous_view;
        }
        self.player_error = None;
    }

    fn current_live_id(&self) -> Option<u32> {
        let key = self
            .preview
            .as_ref()
            .map(|p| p.progress_key.as_str())
            .or_else(|| self.current_play.as_ref().map(|p| p.progress_key.as_str()))?;
        key.strip_prefix("live:").and_then(|s| s.parse().ok())
    }

    /// Jump `delta` channels forward/backwards in the visible Live list.
    pub fn step_channel(&mut self, delta: i32) {
        let order: Vec<u32> = if !self.live_order.is_empty() {
            self.live_order.clone()
        } else {
            self.library.channels.iter().map(|c| c.id).collect()
        };
        if order.is_empty() {
            return;
        }
        let cur = self.current_live_id();
        let next = match cur.and_then(|id| order.iter().position(|&x| x == id)) {
            Some(p) => {
                // Wrap around so zapping never gets stuck at the list ends.
                let len = order.len() as i32;
                let n = (p as i32 + delta).rem_euclid(len);
                order[n as usize]
            }
            None if delta < 0 => *order.last().unwrap_or(&order[0]),
            None => order[0],
        };
        if Some(next) == cur {
            return;
        }
        let ch = match self.library.channels.iter().find(|c| c.id == next) {
            Some(c) => c.clone(),
            None => return,
        };
        self.scroll_to = Some(next);
        let item = PlayItem {
            title: ch.name,
            url: ch.stream_url,
            kind: PlayKind::Live,
            progress_key: format!("live:{}", ch.id),
        };
        if self.view == View::Player {
            self.play(item);
        } else {
            self.start_preview(item);
        }
    }

    fn remember_live(&mut self, item: &PlayItem) {
        if item.kind != PlayKind::Live {
            return;
        }
        let id = item
            .progress_key
            .strip_prefix("live:")
            .and_then(|value| value.parse().ok());
        if let Some(last) = self.live_selected.filter(|last| Some(*last) != id) {
            self.previous_live = self.resolve_play(&format!("live:{last}"));
        }
        self.live_selected = id;
    }

    pub fn recall_channel(&mut self) {
        let item = self
            .previous_live
            .as_ref()
            .and_then(|previous| self.resolve_play(&previous.progress_key))
            .or_else(|| {
                if self.current_live_id().is_some() {
                    return None;
                }
                self.live_selected
                    .and_then(|id| self.resolve_play(&format!("live:{id}")))
            });
        if let Some(item) = item {
            if self.view == View::Player {
                self.play(item);
            } else {
                self.start_preview(item);
            }
        }
    }
    pub fn series_resume_target(&self, sid: u32) -> Option<(Episode, u64)> {
        let episodes = self.series_episodes_for(sid)?;
        let prefix = format!("ep:{sid}:");
        let id = self
            .visible_history()
            .iter()
            .find(|h| h.key.starts_with(&prefix))
            .and_then(|h| h.key.rsplit(':').next()?.parse::<u32>().ok());
        let first = episodes
            .iter()
            .min_by_key(|e| (e.season, e.episode, e.id))?;
        let episode = id
            .and_then(|id| episodes.iter().find(|e| e.id == id))
            .unwrap_or(first);
        let key = format!("ep:{sid}:{}", episode.id);
        let pos = self.get_progress(&key);
        let duration = self.durations.get(&key).copied().unwrap_or(0);
        if pos > 0
            && duration > 0
            && duration.saturating_sub(pos) <= crate::release_info::RESUME_END_MARGIN_SECONDS
        {
            let order = (episode.season, episode.episode, episode.id);
            let next = episodes
                .iter()
                .filter(|e| (e.season, e.episode, e.id) > order)
                .min_by_key(|e| (e.season, e.episode, e.id))
                .unwrap_or(first);
            return Some((next.clone(), 0));
        }
        Some((episode.clone(), pos))
    }
    pub fn next_episode(&mut self, key: &str) -> Option<PlayItem> {
        let (series, episode) = key.strip_prefix("ep:")?.split_once(':')?;
        let series_id = series.parse().ok()?;
        let episode_id = episode.parse::<u32>().ok()?;
        self.request_series(series_id);
        let episodes = self.series_episodes_for(series_id)?;
        let current = episodes.iter().find(|item| item.id == episode_id)?;
        let order = |item: &Episode| (item.season, item.episode, item.id);
        let next = episodes
            .iter()
            .filter(|item| order(item) > order(current))
            .min_by_key(|item| order(item))?;
        self.resolve_play(&format!("ep:{series_id}:{}", next.id))
    }
    pub fn play_series_resume(&mut self, sid: u32) {
        if let Some((e, pos)) = self.series_resume_target(sid) {
            let key = format!("ep:{sid}:{}", e.id);
            self.set_progress(&key, pos);
            self.play(PlayItem {
                title: format!("Staffel {} · Folge {}", e.season, e.episode),
                url: e.stream_url,
                kind: PlayKind::Episode,
                progress_key: key,
            });
        }
    }

    /// Spread texture creation over display frames instead of blocking one frame
    /// with a whole catalogue's network replies.
    fn upload_ready_images(&mut self, ctx: &Context) -> usize {
        let started = std::time::Instant::now();
        let mut uploaded = 0;
        while !self.images.pending.is_empty() && uploaded < 2 {
            let (key, image) = self.images.pending.remove(0);
            let bytes = image.pixels.len() * 4;
            let texture = ctx.load_texture(key.clone(), image, egui::TextureOptions::LINEAR);
            self.images
                .insert_texture(key, texture, bytes, MAX_TEXTURE_CACHE_BYTES);
            uploaded += 1;
            if started.elapsed() >= std::time::Duration::from_millis(2) {
                break;
            }
        }
        if !self.images.pending.is_empty() {
            ctx.request_repaint();
        }
        uploaded
    }

    fn enter_player_navigation(&mut self, ctx: &Context) {
        self.player_menu_navigation = true;
        self.player_reveal = false;
        self.ui_activity = ctx.input(|i| i.time);
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("player-focus-transport"), true));
    }

    fn handle_keys(&mut self, ctx: &Context) {
        if self.parental.open {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.parental.close();
            }
            return;
        }
        #[cfg(windows)]
        if crate::ui::desktop::handle_color_shortcut(self, ctx) {
            return;
        }
        // Never steal keys while a text field has focus.
        if ctx.wants_keyboard_input() && self.view != View::Player {
            return;
        }
        let (left, right, up, down, space, mute, full, esc, plus, minus, enter, audio, sub) = ctx
            .input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::Space),
                    i.key_pressed(egui::Key::M),
                    i.key_pressed(egui::Key::F),
                    i.key_pressed(egui::Key::Escape),
                    i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                    i.key_pressed(egui::Key::Minus),
                    i.events.iter().any(|event| {
                        matches!(
                            event,
                            egui::Event::Key {
                                key: egui::Key::Enter,
                                pressed: true,
                                repeat: false,
                                ..
                            }
                        )
                    }),
                    i.key_pressed(egui::Key::A),
                    i.key_pressed(egui::Key::S),
                )
            });

        let in_player = self.view == View::Player && self.current_play.is_some();
        let has_preview = self.preview.is_some();
        let in_live_list = self.view == View::Live;
        if !in_player && !has_preview && !in_live_list {
            return;
        }
        let is_live = has_preview
            || self
                .current_play
                .as_ref()
                .map(|p| p.kind == PlayKind::Live)
                .unwrap_or(false);
        let menus_open = self.audio_menu_open
            || self.sub_menu_open
            || crate::ui::player_desktop::menus_open(ctx);
        if enter && in_player && is_live && !menus_open {
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            self.open_live_list();
            ctx.request_repaint();
            return;
        }
        if in_player && is_live && !menus_open {
            if up {
                self.step_channel(1);
            }
            if down {
                self.step_channel(-1);
            }
            ctx.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
            });
        }

        if in_player && !is_live && !self.parental.open {
            if !self.player_chrome {
                self.player_menu_navigation = false;
            }
            if (up || down) && !menus_open {
                if !self.player_menu_navigation {
                    self.enter_player_navigation(ctx);
                }
                self.ui_activity = ctx.input(|i| i.time);
            }
        }
        // Volume remains available through +/- and the player controls.
        let mut vol = self.volume;
        let mut vol_changed = false;
        if plus {
            vol = (vol + 5).min(100);
            vol_changed = true;
        }
        if minus {
            vol = (vol - 5).max(0);
            vol_changed = true;
        }
        if mute {
            vol = if vol > 0 { 0 } else { 100 };
            vol_changed = true;
        }
        if vol_changed {
            self.volume = vol;
            if let Some(pl) = &self.player {
                pl.mpv.set_volume(vol);
            }
        }

        if space {
            if let Some(pl) = &self.player {
                pl.mpv.toggle_pause();
            }
        }

        // Enter moves the running preview into the big player.
        if enter && has_preview && !in_player && !self.tv {
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            self.promote_preview(false);
            return;
        }

        if full {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }
        if esc {
            #[cfg(target_os = "windows")]
            if in_player && crate::ui::player_desktop::dismiss_menus(self, ctx) {
                return;
            }
            if self.fullscreen {
                self.fullscreen = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            } else if in_player {
                self.stop_play();
                return;
            } else if has_preview {
                self.close_preview();
                return;
            }
        }

        // Audio / subtitle selector popovers (movies / series only, no mouse
        // movement needed to reveal them — they live in the auto-hiding menu).
        if in_player && !is_live {
            if audio {
                self.audio_menu_open = !self.audio_menu_open;
                self.sub_menu_open = false;
            }
            if sub {
                self.sub_menu_open = !self.sub_menu_open;
                self.audio_menu_open = false;
            }
        }

        // ---- arrows ----
        // Column widgets and spatial TV focus own browse navigation.
        if in_live_list && !in_player {
            return;
        }

        if in_player
            && (left || right)
            && !menus_open
            && (is_live || (!crate::ui::mobile::active(ctx) && !self.player_menu_navigation))
        {
            let delta = if left { -1 } else { 1 };
            if let Some(pl) = &self.player {
                pl.mpv
                    .seek_relative(delta as f64 * crate::release_info::SEEK_STEP_SECONDS);
            }
            self.ui_activity = ctx.input(|i| i.time);
            ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::NONE,
                    if left {
                        egui::Key::ArrowLeft
                    } else {
                        egui::Key::ArrowRight
                    },
                );
            });
        }
    }

    /// Position the native video surface inside the preview panel.
    pub fn position_preview(&mut self, rect: egui::Rect, ppp: f32) {
        self.preview_placed = true;
        if let Some(pl) = &self.player {
            let x = (rect.min.x * ppp).round() as i32;
            let y = (rect.min.y * ppp).round() as i32;
            let w = (rect.width() * ppp).round() as i32;
            let h = (rect.height() * ppp).round() as i32;
            pl.surface.position(x, y, w, h);
            #[cfg(windows)]
            pl.mpv.set_video_size(w.max(0) as u32, h.max(0) as u32);
        }
    }

    pub fn is_previewing(&self, key: &str) -> bool {
        self.preview
            .as_ref()
            .map(|p| p.progress_key == key)
            .unwrap_or(false)
    }

    /// The native video window swallows mouse events, so the player polls the
    /// real cursor to know when to bring its controls back.
    pub fn note_pointer_activity(&mut self) -> bool {
        let (cx, cy) = crate::platform::sys::cursor_pos();
        let moved = (cx, cy) != self.last_cursor;
        self.last_cursor = (cx, cy);
        if !moved {
            return false;
        }
        match crate::platform::sys::window_rect(self.main_hwnd) {
            Some((l, t, r, b)) => cx >= l && cx <= r && cy >= t && cy <= b,
            None => true,
        }
    }

    fn ensure_player(&mut self) -> bool {
        #[cfg(any(target_os = "android", target_os = "windows"))]
        if self.player.as_ref().is_some_and(|player| {
            player.mpv.is_vlc()
                != (self.settings.effective_player_backend() == crate::types::PlayerBackend::Vlc)
        }) {
            if let Some(player) = self.player.take() {
                player.mpv.stop();
                #[cfg(target_os = "android")]
                if !player.mpv.is_vlc() {
                    self.parked_mpv = Some(player.mpv);
                }
            }
            self.last_video_rect = None;
            self.last_video_cutouts = None;
        }
        if let Some(player) = &self.player {
            player.mpv.configure(&self.settings);
            self.player_error = None;
            return true;
        }
        // Android has no child window: mpv renders into egui's own GL context
        // and egui composites the frame, so there is no handle to require and
        // no `VideoSurface` to create. The `ANativeWindow` variant is a no-op
        // stub there (position/hide do nothing), kept only so `Player` has one
        // shape on both platforms.
        #[cfg(target_os = "android")]
        {
            let surface = VideoSurface::ANativeWindow(std::ptr::null_mut());
            let decoder =
                if self.settings.effective_player_backend() == crate::types::PlayerBackend::Mpv {
                    self.parked_mpv
                        .take()
                        .map(Ok)
                        .unwrap_or_else(|| Playback::new(0, &self.settings))
                } else {
                    Playback::new(0, &self.settings)
                };
            match decoder {
                Ok(mpv) => {
                    self.player = Some(Player {
                        mpv,
                        surface,
                        current_key: String::new(),
                        position: 0.0,
                        duration: 0.0,
                        paused: false,
                        stream: StreamInfo::default(),
                        audio_tracks: Vec::new(),
                        sub_tracks: Vec::new(),
                        current_audio: 0,
                        current_sub: 0,
                        audio_preference_applied: false,
                        subtitle_preference_applied: false,
                    });
                    self.player_error = None;
                    eprintln!("[player] Integrierten Android-Player initialisiert");
                    return true;
                }
                Err(e) => {
                    eprintln!("[player] Fehler: {}", e);
                    self.player_error = Some(e);
                    return false;
                }
            }
        }
        // iOS: the video is a host-owned AVPlayerLayer. There is no window
        // handle to require and no child view for Rust to create, so the
        // surface is a borrowed token and `Mpv::new` (the AVPlayer shim) does
        // the attaching.
        #[cfg(target_os = "ios")]
        {
            let surface = match VideoSurface::create(std::ptr::null_mut()) {
                Some(s) => s,
                None => {
                    self.player_error =
                        Some("Video-Fläche konnte nicht angebunden werden.".to_string());
                    return false;
                }
            };
            match Playback::new(0, &self.settings) {
                Ok(mpv) => {
                    self.player = Some(Player {
                        mpv,
                        surface,
                        current_key: String::new(),
                        position: 0.0,
                        duration: 0.0,
                        paused: false,
                        stream: StreamInfo::default(),
                        audio_tracks: Vec::new(),
                        sub_tracks: Vec::new(),
                        current_audio: 0,
                        current_sub: 0,
                        audio_preference_applied: false,
                        subtitle_preference_applied: false,
                    });
                    self.player_error = None;
                    eprintln!("[player] AVPlayer angebunden");
                    return true;
                }
                Err(e) => {
                    eprintln!("[player] Fehler: {}", e);
                    self.player_error = Some(e);
                    return false;
                }
            }
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            if self.main_hwnd == 0 {
                self.player_error = Some("Kein Fenster-Handle verfügbar.".to_string());
                return false;
            }
            let surface = match VideoSurface::create(self.main_hwnd as *mut std::ffi::c_void) {
                Some(s) => s,
                None => {
                    self.player_error =
                        Some("Video-Fenster konnte nicht erstellt werden.".to_string());
                    return false;
                }
            };
            match Playback::new(surface.native_handle(), &self.settings) {
                Ok(mpv) => {
                    self.player = Some(Player {
                        mpv,
                        surface,
                        current_key: String::new(),
                        position: 0.0,
                        duration: 0.0,
                        paused: false,
                        stream: StreamInfo::default(),
                        audio_tracks: Vec::new(),
                        sub_tracks: Vec::new(),
                        current_audio: 0,
                        current_sub: 0,
                        audio_preference_applied: false,
                        subtitle_preference_applied: false,
                    });
                    self.player_error = None;
                    eprintln!("[player] libmpv initialisiert");
                    true
                }
                Err(e) => {
                    eprintln!("[player] Fehler: {}", e);
                    self.player_error = Some(e);
                    false
                }
            }
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn paint_android_video(&self, ui: &egui::Ui, rect: egui::Rect) {
        if let Some(player) = &self.player {
            player.mpv.paint(ui, rect);
        }
    }

    pub fn position_video(&mut self, rect: egui::Rect, ppp: f32) {
        // Playback may start before the first player frame positions this child
        // HWND. Explicitly reset the cached rect on play() so the first frame
        // always sizes and reveals the surface, even when the previous item had
        // identical geometry.
        let x = (rect.min.x * ppp).round() as i32;
        let y = (rect.min.y * ppp).round() as i32;
        let w = (rect.width() * ppp).round() as i32;
        let h = (rect.height() * ppp).round() as i32;
        let r = (x, y, w, h);
        if self.last_video_rect == Some(r) {
            return;
        }
        self.last_video_rect = Some(r);
        if let Some(pl) = &self.player {
            pl.surface.position(x, y, w, h);
            #[cfg(windows)]
            pl.mpv.set_video_size(w.max(0) as u32, h.max(0) as u32);
        }
        if let Some(path) = qa_state_log_path() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(
                    f,
                    "{}  video_rect={}x{} @ {},{}",
                    chrono::Local::now().format("%H:%M:%S%.3f"),
                    w,
                    h,
                    x,
                    y
                );
            }
        }
    }

    /// Reveal egui player chrome THROUGH the native video window without
    /// resizing it.
    ///
    /// mpv re-fits and re-scales the picture only when the HWND size changes.
    /// `position_video()` therefore keeps one fixed rect for the whole
    /// playback, and the visible chrome is expressed as holes in the child
    /// window's region instead. The parent's egui pixels show through there,
    /// so the controls literally stand in front of a picture that never moves.
    ///
    /// `holes` are `(x, y, w, h)` in physical pixels, relative to the video
    /// rect's top-left. An empty slice restores the full, unclipped surface.
    pub fn set_video_cutouts(&mut self, holes: &[(i32, i32, i32, i32)]) {
        if self.last_video_cutouts.as_deref() == Some(holes) {
            return;
        }
        // No player yet (e.g. the first frame before `ensure_player`): do not
        // cache a region we could not install, or it would never be applied.
        let Some(pl) = &self.player else {
            return;
        };
        pl.surface.set_overlay_cutouts(holes);
        self.last_video_cutouts = Some(holes.to_vec());
    }

    /// Clear the per-playback track selector state: close the popovers and drop
    /// any stale track lists so a newly started item never shows the previous
    /// one's audio/subtitle choices.
    fn reset_track_state(&mut self) {
        self.audio_menu_open = false;
        self.sub_menu_open = false;
        if let Some(pl) = &mut self.player {
            pl.audio_tracks.clear();
            pl.sub_tracks.clear();
            pl.current_audio = 0;
            pl.current_sub = 0;
            pl.audio_preference_applied = false;
            pl.subtitle_preference_applied = false;
        }
    }

    fn queue_desktop_player_return(&mut self) {
        if let Some(item) = &self.current_play {
            self.desktop_return_item = Some(item.progress_key.clone());
            if self.previous_view == View::Live {
                self.scroll_to = item
                    .progress_key
                    .strip_prefix("live:")
                    .and_then(|id| id.parse().ok());
            }
        }
    }

    pub(crate) fn back_from_player(&mut self) {
        if self.previous_view == View::Live
            && !(cfg!(target_os = "ios") && self.settings.ios_native_player)
            && self
                .current_play
                .as_ref()
                .is_some_and(|item| item.kind == PlayKind::Live)
        {
            self.queue_desktop_player_return();
            #[cfg(windows)]
            self.desktop_overlay.hide();
            if let Some(player) = &self.player {
                player
                    .mpv
                    .set_desktop_aspect(crate::types::DesktopAspect::Fit);
                player.surface.hide();
            }
            self.preview = self.current_play.take();
            self.last_video_rect = None;
            self.last_video_cutouts = None;
            self.reset_track_state();
            self.view = View::Live;
            if self.fullscreen {
                self.pending_fullscreen = Some(false);
            }
            return;
        }
        self.stop_play();
    }

    pub(crate) fn open_live_list(&mut self) {
        if let Some(item) = &self.current_play {
            if let Some(channel) = item
                .progress_key
                .strip_prefix("live:")
                .and_then(|value| value.parse::<u32>().ok())
                .and_then(|id| self.library.channels.iter().find(|entry| entry.id == id))
            {
                let in_category = match self.live_cat.as_str() {
                    "all" => true,
                    "favorites" => self.is_favorite(&item.progress_key),
                    category => channel.category_id == category,
                };
                if !in_category {
                    self.live_cat = "all".into();
                }
                if !channel
                    .name
                    .to_lowercase()
                    .contains(&self.live_q.trim().to_lowercase())
                {
                    self.live_q.clear();
                }
            }
        }
        self.previous_view = View::Live;
        self.back_from_player();
    }

    pub fn stop_play(&mut self) {
        #[cfg(target_os = "windows")]
        self.desktop_overlay.hide();
        self.pending_history_key = None;
        #[cfg(target_os = "android")]
        crate::platform::android::render::cancel_load();
        let (key, pos) = if let Some(pl) = &self.player {
            (pl.current_key.clone(), pl.position)
        } else {
            (String::new(), 0.0)
        };
        if let Some(pl) = &mut self.player {
            pl.mpv.stop();
            pl.current_key.clear();
            pl.surface.hide();
        }
        self.last_video_rect = None;
        // `hide()` drops the native region; forget the cache so the next play
        // re-applies it instead of assuming it is still installed.
        self.last_video_cutouts = None;
        if !self.untracked_playback {
            if self.settings.watch_history && !key.is_empty() {
                self.progress.insert(key, pos as u64);
            }
            self.save_user_data();
            self.queue_desktop_player_return();
        }
        self.untracked_playback = false;
        self.reset_track_state();
        self.current_play = None;
        self.view = self.previous_view;
        if self.fullscreen {
            self.pending_fullscreen = Some(false);
        }
    }

    // ---- images ----

    pub(crate) fn cancel_episode_frame(&self) {
        if let Some(token) = &self.episode_frame_request {
            token.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub(crate) fn request_episode_frame(&mut self, key: &str, episode: &Episode) {
        if self.episode_frame_request.is_some()
            || self.current_play.is_some()
            || self.parental.open
            || self.pending_resume.is_some()
            || episode.stream_url.is_empty()
            || self.images.textures.contains_key(key)
            || self.images.failed.contains(key)
            || self
                .images
                .pending
                .iter()
                .any(|(pending, _)| pending == key)
        {
            return;
        }
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let key = key.to_owned();
        self.episode_frame_request = Some(cancel.clone());
        let settings = self.settings.clone();
        let url = episode.stream_url.clone();
        let sender = self.tx.clone();
        let generation = self.login_generation;
        self.rt.spawn_blocking(move || {
            #[cfg(any(target_os = "windows", target_os = "android"))]
            let frame = crate::mpv::episode_frame::frame(&url, &settings, &cancel);
            #[cfg(target_os = "ios")]
            let frame = crate::platform::ios::bridge::episode_frame(&url, &settings, &cancel);
            #[cfg(not(any(target_os = "windows", target_os = "android", target_os = "ios")))]
            let frame = None;
            let _ = sender.send(AppEvent::EpisodeFrame(generation, key, frame));
        });
    }

    pub fn request_image(&mut self, key: String, url: String) {
        if url.is_empty() {
            return;
        }
        if self.images.textures.contains_key(&key)
            || self.images.requested.contains(&key)
            || self.images.failed.contains(&key)
            || self
                .images
                .pending
                .iter()
                .any(|(pending, _)| pending == &key)
        {
            return;
        }
        // Cache insertion evicts old entries. A full cache must not prevent
        // newly scrolled-to covers from ever being requested.
        if self.images.requested.len() >= 4
            || self.images.requested.len() + self.images.pending.len() >= 8
        {
            return;
        }
        self.images.requested.insert(key.clone());
        let tx = self.tx.clone();
        let generation = self.login_generation;
        let to = self.settings.network_timeout_s;
        let edge = if key.starts_with("poster:") || key.starts_with("detail:") {
            640
        } else {
            1280
        };
        self.rt.spawn(async move {
            let res = fetch_image(&url, to, edge).await;
            let _ = tx.send(AppEvent::Image(
                generation,
                key,
                res.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn image(&self, key: &str) -> Option<&egui::TextureHandle> {
        self.images.textures.get(key)
    }

    // ---- favorites / progress ----

    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorites.contains(&favorite_key(key))
    }

    pub fn toggle_favorite(&mut self, key: &str) {
        let normalized = favorite_key(key);
        let key = normalized.as_str();
        if self.favorites.contains(key) {
            self.favorites.remove(key);
        } else {
            self.favorites.insert(key.to_string());
        }
        self.save_user_data();
    }

    pub fn get_progress(&self, key: &str) -> u64 {
        if self.settings.watch_history {
            self.progress.get(key).copied().unwrap_or_default()
        } else {
            0
        }
    }

    pub(crate) fn resume_position(&self, key: &str) -> Option<u64> {
        let position = self.get_progress(key);
        let duration = self.durations.get(key).copied().unwrap_or_default();
        (position >= crate::release_info::RESUME_MINIMUM_SECONDS
            && (duration == 0
                || duration.saturating_sub(position)
                    > crate::release_info::RESUME_END_MARGIN_SECONDS))
            .then_some(position)
    }

    pub fn ask_resume(&mut self, key: &str) {
        if let Some(item) = self.resolve_play(key) {
            self.offer_resume(item);
        }
    }

    fn offer_resume(&mut self, item: PlayItem) -> bool {
        if item.kind == PlayKind::Live {
            return false;
        }
        let Some(position) = self.resume_position(&item.progress_key) else {
            return false;
        };
        self.pending_resume = Some((item.progress_key.clone(), item, position));
        true
    }

    pub fn play_selected(&mut self, item: PlayItem) {
        if !self.offer_resume(item.clone()) {
            if item.kind != PlayKind::Live
                && self.get_progress(&item.progress_key)
                    >= crate::release_info::RESUME_MINIMUM_SECONDS
            {
                self.play_from_beginning(item);
            } else {
                self.play(item);
            }
        }
    }

    /// Clear the pending resume dialog without playing anything.
    pub fn clear_resume(&mut self) {
        self.pending_resume = None;
    }

    /// Play `item` from the very beginning: drop any saved progress first.
    pub fn play_from_beginning(&mut self, item: PlayItem) {
        if !self.parental_gate(&item, false) {
            self.parental.restart = true;
            return;
        }
        if self.settings.watch_history {
            self.progress.remove(&item.progress_key);
        }
        self.save_user_data();
        self.play_authorized(item, false);
    }

    pub fn set_progress(&mut self, key: &str, secs: u64) {
        if self.settings.watch_history {
            self.progress.insert(key.to_string(), secs);
        }
    }

    // ---- epg / series ----

    pub fn request_epg(&mut self, id: u32) {
        let is_xtream = self
            .active_profile
            .as_ref()
            .map(|p| p.kind == ProfileKind::Xtream)
            .unwrap_or(false);
        if !is_xtream
            || self.epg.contains_key(&id)
            || self.epg_requested.contains(&id)
            || self.epg_failed.contains(&id)
        {
            return;
        }
        if self.epg_requested.len() >= MAX_IN_FLIGHT_DETAIL_REQUESTS {
            return;
        }
        let p = match &self.active_profile {
            Some(p) => p.clone(),
            None => return,
        };
        self.epg_requested.insert(id);
        let tx = self.tx.clone();
        let to = self.settings.network_timeout_s;
        let generation = self.login_generation;
        self.rt.spawn(async move {
            let r = match client::xtream_full_epg(&p.host, &p.username, &p.password, id, to).await {
                Ok(items) if !items.is_empty() => Ok(items),
                _ => client::xtream_short_epg(&p.host, &p.username, &p.password, id, to).await,
            };
            let _ = tx.send(AppEvent::Epg(generation, id, r.map_err(|e| e.to_string())));
        });
    }

    pub fn epg_for(&self, id: u32) -> Option<&Vec<EpgEntry>> {
        self.epg.get(&id)
    }

    /// True while an EPG request for this channel is still in flight.
    pub fn epg_pending(&self, id: u32) -> bool {
        self.epg_requested.contains(&id) && !self.epg.contains_key(&id)
    }

    pub fn epg_failed(&self, id: u32) -> bool {
        self.epg_failed.contains(&id)
    }

    pub fn series_failed(&self, id: u32) -> bool {
        self.series_failed.contains(&id)
    }

    /// User-initiated retry after a failed EPG request. Clears the failure
    /// marker first — otherwise `request_epg` would refuse to re-fetch.
    pub fn retry_epg(&mut self, id: u32) {
        self.epg_failed.remove(&id);
        self.epg_error = None;
        self.epg.remove(&id);
        self.epg_order.retain(|existing| *existing != id);
        self.epg_requested.remove(&id);
        self.request_epg(id);
    }

    pub fn retry_series(&mut self, id: u32) {
        self.series_failed.remove(&id);
        self.series_error = None;
        self.series_episodes.remove(&id);
        self.series_order.retain(|existing| *existing != id);
        self.series_requested.remove(&id);
        self.request_series(id);
    }

    /// Re-run the full catalogue load for the active profile after a partial
    /// failure. One tap replaces the "log out and back in ten times" workaround.
    pub fn refresh_catalog(&mut self) {
        if self.busy || self.catalog_refreshing {
            return;
        }
        let Some(profile) = self.active_profile.clone() else {
            return;
        };
        self.catalog_refreshing = true;
        self.catalog_refresh_notice = None;
        self.catalog_refresh_result = None;
        let generation = self.login_generation;
        let timeout = self.settings.network_timeout_s;
        let extension = self.settings.live_format.extension().to_string();
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let result = client::load_profile(&profile, timeout, &extension)
                .await
                .map_err(|error| error.to_string());
            let _ = tx.send(AppEvent::CatalogRefreshed(generation, Box::new(result)));
        });
    }

    /// Is the item behind this progress key a radio station (M3U
    /// `radio="true"`)? Decides whether the player suppresses its video
    /// surface and shows the radio card instead.
    pub fn is_radio(&self, key: &str) -> bool {
        key.strip_prefix("live:")
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|id| self.library.channels.iter().find(|c| c.id == id))
            .map(|c| c.radio)
            .unwrap_or(false)
    }

    pub fn request_series(&mut self, series_id: u32) {
        let is_xtream = self
            .active_profile
            .as_ref()
            .map(|p| p.kind == ProfileKind::Xtream)
            .unwrap_or(false);
        if !is_xtream
            || self.series_episodes.contains_key(&series_id)
            || self.series_requested.contains(&series_id)
            || self.series_failed.contains(&series_id)
        {
            return;
        }
        if self.series_requested.len() >= MAX_IN_FLIGHT_DETAIL_REQUESTS {
            return;
        }
        let p = match &self.active_profile {
            Some(p) => p.clone(),
            None => return,
        };
        self.series_requested.insert(series_id);
        let tx = self.tx.clone();
        let to = self.settings.network_timeout_s;
        let stream_base = self.library.stream_base.clone();
        let generation = self.login_generation;
        self.rt.spawn(async move {
            let r = client::xtream_series_info(
                &p.host,
                &stream_base,
                &p.username,
                &p.password,
                series_id,
                to,
            )
            .await;
            let _ = tx.send(AppEvent::SeriesInfo(
                generation,
                series_id,
                r.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn series_episodes_for(&self, series_id: u32) -> Option<&Vec<Episode>> {
        self.series_episodes.get(&series_id)
    }

    /// Open a series detail page: select it, fetch its episode list, and switch
    /// to the Series view. Used from home rails where a poster represents a
    /// series (no single streamable URL exists for a whole series).
    pub fn open_series(&mut self, series_id: u32) {
        self.series_season = 0;
        self.series_open_revision = self.series_open_revision.wrapping_add(1);
        self.selected_series = Some(series_id);
        self.catalog_resume_series = Some(series_id);
        self.request_series(series_id);
        if self.tv {
            self.tv_goto(2);
        }
        self.view = View::Series;
    }

    /// Rebuild Xtream live URLs after "Live-Format" changes, without forcing a
    /// new login. M3U channels keep the exact URL the playlist supplied —
    /// rewriting those would silently break tokenised links.
    pub fn apply_live_format(&mut self) {
        if !self.library.loaded || self.library.channels.is_empty() {
            return;
        }
        let xtream = self
            .active_profile
            .as_ref()
            .map(|p| p.kind == ProfileKind::Xtream)
            .unwrap_or(false);
        if !xtream {
            return;
        }
        let Some(p) = self.active_profile.clone() else {
            return;
        };
        let sb = self.library.stream_base.clone();
        let ext = self.settings.live_format.extension();
        for ch in &mut self.library.channels {
            ch.stream_url = client::live_stream_url(&sb, &p.username, &p.password, ch.id, ext);
        }
    }

    pub fn save_settings_now(&mut self) {
        let _ = store::save_settings(&self.settings);
    }

    fn save_user_data(&self) {
        if let Some(id) = &self.data_profile {
            let _ = store::save_profile_data(
                id,
                &self.favorites,
                &self.progress,
                &self.durations,
                &self.history,
            );
        } else {
            let _ = store::save_favorites(&self.favorites);
            let _ = store::save_progress(&self.progress);
            let _ = store::save_durations(&self.durations);
            let _ = store::save_history(&self.history);
        }
    }
    fn switch_user_profile(&mut self, id: &str) {
        if self.data_profile.as_deref() == Some(id) {
            return;
        }
        if self.data_profile.is_some() {
            self.save_user_data();
        }
        let data = match store::load_profile_data(id, self.settings.default_profile.as_deref()) {
            Ok(data) => data,
            Err(_) => {
                self.info = Some(
                    "Profildaten konnten nicht geladen werden. Eine Sicherung wurde erhalten."
                        .into(),
                );
                store::ProfileData::default()
            }
        };
        self.favorites = data
            .favorites
            .into_iter()
            .map(|key| favorite_key(&key))
            .collect();
        self.progress = data.progress;
        self.durations = data.durations;
        self.history = data.history;
        self.data_profile = Some(id.to_string());
    }

    /// A continue action survives asynchronous episode loading.
    pub fn play_history_key(&mut self, key: &str) {
        if !self.settings.watch_history {
            return;
        }
        if let Some(sid) = key
            .strip_prefix("ep:")
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.parse::<u32>().ok())
        {
            if self.series_episodes_for(sid).is_none() {
                self.pending_history_key = Some(key.to_string());
                self.info = Some("Folge wird geladen …".into());
                if self.series_failed(sid) {
                    self.retry_series(sid);
                } else {
                    self.request_series(sid);
                }
                return;
            }
            if self.durations.get(key).is_some_and(|duration| {
                *duration > 0
                    && self.get_progress(key) > 0
                    && duration.saturating_sub(self.get_progress(key))
                        <= crate::release_info::RESUME_END_MARGIN_SECONDS
            }) {
                self.play_series_resume(sid);
                return;
            }
        }
        self.pending_history_key = None;
        if let Some(item) = self.resolve_play(key) {
            if item.kind == PlayKind::Vod
                && self.durations.get(key).is_some_and(|d| {
                    *d > 0
                        && self.get_progress(key) > 0
                        && d.saturating_sub(self.get_progress(key))
                            <= crate::release_info::RESUME_END_MARGIN_SECONDS
                })
            {
                self.play_from_beginning(item);
            } else {
                self.play(item);
            }
        } else {
            self.info = Some("Dieser Inhalt ist im aktuellen Profil nicht verfügbar.".into());
        }
    }

    /// True when the resolved theme is dark (used by the command palette).
    pub fn want_dark(&self) -> bool {
        if let Some(d) = self.dark_mode_override {
            return d;
        }
        self.settings.theme.resolve(self.system_dark)
    }
}

fn preferred_track<'a>(tracks: &'a [Track], language: &str) -> Option<&'a Track> {
    let wanted = crate::types::language_key(language);
    if wanted.is_empty() {
        return None;
    }
    tracks
        .iter()
        .find(|t| crate::types::language_key(&t.lang) == wanted)
}

/// Optional QA state log. Set `DADO_QA_STATE_LOG` to a file path to get one
/// line per *state change* (view, player, selected series, palette, busy).
/// This exists to prove that a click on the Home dashboard changes state in
/// the very same frame, instead of only after the user switches tabs — a
/// regression that pixel screenshots cannot distinguish from a slow repaint.
/// The path is resolved exactly once; a normal launch does no file I/O at all.
fn qa_state_log_path() -> Option<&'static std::path::PathBuf> {
    static PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    PATH.get_or_init(|| std::env::var_os("DADO_QA_STATE_LOG").map(std::path::PathBuf::from))
        .as_ref()
}

fn qa_state_snapshot(app: &App) -> String {
    let view = match app.view {
        View::Login => "login",
        View::Home => "home",
        View::Live => "live",
        View::Movies => "movies",
        View::Series => "series",
        View::Search => "search",
        View::Favorites => "favorites",
        View::Continue => "continue",
        View::Settings => "settings",
        View::Player => "player",
    };
    format!(
        "view={} player={} series={:?} palette={} audio={} subtitles={} busy={}",
        view,
        app.player.is_some(),
        app.selected_series,
        app.palette_open,
        app.audio_menu_open,
        app.sub_menu_open,
        app.busy
    )
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        if cfg!(target_os = "ios") {
            [0.0; 4]
        } else {
            visuals.window_fill().to_normalized_gamma_f32()
        }
    }
    #[cfg(windows)]
    fn raw_input_hook(&mut self, ctx: &Context, input: &mut egui::RawInput) {
        use crate::platform::RemoteKey;
        crate::platform::windows::player_overlay::read_input(ctx, input, self.view == View::Player);
        if self.view == View::Player
            || !self.shared_remote_navigation()
            || (crate::licensing::enabled() && !crate::licensing::allowed())
        {
            return;
        }
        let focused = ctx.memory(|memory| memory.focused());
        let editing = focused.is_some()
            && focused
                == ctx
                    .data(|data| data.get_temp::<egui::Id>(egui::Id::new("desktop-search-editor")));
        let mut keys = Vec::new();
        input.events.retain(|event| {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            if !modifiers.is_none() {
                return true;
            }
            let key = match key {
                egui::Key::ArrowUp => RemoteKey::Up,
                egui::Key::ArrowDown => RemoteKey::Down,
                egui::Key::ArrowLeft if !editing => RemoteKey::Left,
                egui::Key::ArrowRight if !editing => RemoteKey::Right,
                egui::Key::Enter if !editing => RemoteKey::Ok,
                _ => return true,
            };
            keys.push(key);
            false
        });
        if !keys.is_empty() {
            if let Some(id) = focused {
                ui::mobile::focus::adopt_id(id);
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("desktop-remote-keys"), keys));
        }
    }

    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        #[cfg(target_os = "ios")]
        if std::env::var_os("DADO_LAYOUT_SMOKE").is_some() {
            let env = crate::platform::ios::bridge::env();
            if env.width > 0.0 && env.height > 0.0 {
                let size = ctx.screen_rect().size() * ctx.zoom_factor();
                assert!(
                    (size.x - env.width as f32).abs() < 1.0
                        && (size.y - env.height as f32).abs() < 1.0,
                    "iOS drawable/UI mismatch: egui={size:?}, UIKit={}x{}",
                    env.width,
                    env.height
                );
            }
        }
        if let Some(path) = qa_state_log_path() {
            let before = qa_state_snapshot(self);
            App::update(self, ctx, frame);
            let after = qa_state_snapshot(self);
            if after != before {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = writeln!(
                        f,
                        "{}  {before} -> {after}",
                        chrono::Local::now().format("%H:%M:%S%.3f")
                    );
                }
            }
        } else {
            App::update(self, ctx, frame);
        }
    }
}

#[cfg(test)]
mod scroll_profile {
    use super::App;

    #[test]
    fn decoded_cover_waiting_for_upload_is_not_requested_again() {
        let mut app = App::test_app();
        app.images.pending.push((
            "poster:1".into(),
            egui::ColorImage::new([1, 1], egui::Color32::WHITE),
        ));
        app.request_image(
            "poster:1".into(),
            "https://provider.invalid/cover.jpg".into(),
        );
        assert!(app.images.requested.is_empty());
        assert_eq!(app.images.pending.len(), 1);
    }

    #[test]
    fn virtual_categories_keep_remote_navigation_across_viewport_boundaries() {
        let mut app = App::test_app();
        app.tv = true;
        app.library.vod_categories = (0..120)
            .map(|i| crate::types::Category {
                id: format!("provider-{i}"),
                name: format!("Kategorie {i}"),
            })
            .collect();
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx, true);
        for frame in 0..415 {
            let keys = if frame > 10 && frame <= 410 && (frame - 11) % 10 == 0 {
                vec![crate::platform::RemoteKey::Down]
            } else if frame == 414 {
                vec![crate::platform::RemoteKey::Ok]
            } else {
                vec![]
            };
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 800.0),
                    )),
                    time: Some(f64::from(frame) / 120.0),
                    ..Default::default()
                },
                |ctx| {
                    crate::ui::mobile::focus::begin(&keys, 0x6404, false, false, true);
                    egui::CentralPanel::default()
                        .show(ctx, |ui| crate::ui::library::show(&mut app, ui, false));
                    crate::ui::mobile::focus::end();
                },
            );
            if frame == 9 {
                crate::ui::mobile::focus::request_index(0);
            }
        }
        assert_eq!(app.vod_cat, "provider-37");
    }

    #[test]
    fn poster_decode_limits_upload_size_without_stretching_or_upscaling() {
        for (w, h, expected) in [
            (1000, 1600, (400, 640)),
            (1600, 900, (640, 360)),
            (32, 16, (32, 16)),
        ] {
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgba8(w, h)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .unwrap();
            let image = super::decode_image(encoded.get_ref(), 640).unwrap();
            assert_eq!((image.width() as u32, image.height() as u32), expected);
            assert_eq!(image.pixels.len(), image.width() * image.height());
            assert!(
                image
                    .pixels
                    .iter()
                    .all(|v| *v == egui::Color32::TRANSPARENT),
                "Transparency must survive downsampling"
            );
        }
    }

    #[test]
    fn catalogue_scroll_work_is_bounded_by_viewport() {
        let mut small_catalogue_peak = 0;
        for category_count in [20, 1000] {
            let mut app = App::test_app();
            app.library.vod_categories = (0..category_count)
                .map(|i| crate::types::Category {
                    id: i.to_string(),
                    name: format!("Kategorie {i} Filme und Unterhaltung"),
                })
                .collect();
            app.library.movies = (0..20000)
                .map(|i| crate::types::VodMovie {
                    id: i,
                    name: format!("Film {i:05}"),
                    cover: String::new(),
                    category_id: "1".into(),
                    rating: String::new(),
                    added: None,
                    container_extension: "mp4".into(),
                    stream_url: String::new(),
                    plot: String::new(),
                })
                .collect();
            let ctx = egui::Context::default();
            crate::theme::apply(&ctx, true);
            let mut times = Vec::new();
            let mut shapes = 0;
            let mut peak_shapes = 0;
            for frame in 0..90 {
                let start = std::time::Instant::now();
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1280.0, 800.0),
                        )),
                        time: Some(f64::from(frame) / 120.0),
                        events: vec![
                            egui::Event::PointerMoved(egui::pos2(600.0, 400.0)),
                            egui::Event::Scroll(egui::vec2(0.0, -12.0)),
                        ],
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .show(ctx, |ui| crate::ui::library::show(&mut app, ui, false));
                    },
                );
                shapes = output.shapes.len();
                peak_shapes = peak_shapes.max(shapes);
                assert!(
                    shapes < 260,
                    "Offscreen categories generated {shapes} paint shapes"
                );
                let _ = ctx.tessellate(output.shapes, output.pixels_per_point);
                if frame > 15 {
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
            }
            times.sort_by(f64::total_cmp);
            if category_count == 20 {
                small_catalogue_peak = peak_shapes;
            } else {
                assert!(
                    peak_shapes <= small_catalogue_peak + 16,
                    "Offscreen categories inflated the visible shape budget: {small_catalogue_peak} → {peak_shapes}"
                );
            }
            println!("CATALOGUE CPU categories={category_count} movies=20000 shapes={shapes} median_ms={:.2} p95_ms={:.2}",times[times.len()/2],times[times.len()*95/100]);
        }
    }
}

async fn fetch_image(url: &str, timeout_s: u64, max_edge: u32) -> anyhow::Result<egui::ColorImage> {
    const MAX_ENCODED_BYTES: usize = 8 * 1024 * 1024;

    let c = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_s.max(5)))
        .build()?;
    let mut response = c.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_ENCODED_BYTES as u64)
    {
        anyhow::bail!("image response exceeds size limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > MAX_ENCODED_BYTES {
            anyhow::bail!("image response exceeds size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    // Decode/resize away from both the UI and asynchronous network workers.
    // The renderer never needs a full provider-sized poster texture.
    tokio::task::spawn_blocking(move || decode_image(&bytes, max_edge)).await?
}

fn decode_image(bytes: &[u8], max_edge: u32) -> anyhow::Result<egui::ColorImage> {
    const MAX_DECODED_BYTES: u64 = 8 * 1024 * 1024;
    anyhow::ensure!(max_edge > 0, "image edge must be positive");
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODED_BYTES);
    reader.limits(limits);
    let img = reader.decode()?;
    let (w, h) = (img.width(), img.height());
    if u64::from(w).saturating_mul(u64::from(h)).saturating_mul(4) > MAX_DECODED_BYTES {
        anyhow::bail!("decoded image exceeds size limit");
    }
    let img = if w.max(h) > max_edge {
        img.thumbnail(max_edge, max_edge)
    } else {
        img
    };
    let (w, h) = (img.width(), img.height());
    let rgba = img.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        &rgba,
    ))
}

fn favorite_key(key: &str) -> String {
    if let Some(sid) = key
        .strip_prefix("ep:")
        .and_then(|rest| rest.split(':').next())
    {
        format!("series:{sid}")
    } else {
        key.to_owned()
    }
}

/// Provider URLs carry credentials in the query string. Never show them in the
/// UI (screenshots, support chats, …).
fn redact(s: &str) -> String {
    fn mask(input: &str, key: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(i) = rest.find(key) {
            let (head, tail) = rest.split_at(i + key.len());
            out.push_str(head);
            let end = tail.find(['&', ' ', ')', '"']).unwrap_or(tail.len());
            out.push_str("***");
            rest = &tail[end..];
        }
        out.push_str(rest);
        out
    }
    mask(&mask(s, "password="), "username=")
}

#[cfg(test)]
mod tests {
    #[test]
    fn refreshing_keeps_the_view_preview_and_unsuccessful_catalogue_areas() {
        let mut app = App::test_app();
        app.load_demo();
        app.view = View::Live;
        app.live_cat = "favorites".into();
        let channel = app.library.channels[0].id;
        app.preview = app.resolve_play(&format!("live:{channel}"));
        let generation = app.catalog_generation();
        let channels = app.library.channels.len();
        app.catalog_refreshing = true;
        app.handle_event(AppEvent::CatalogRefreshed(
            generation.wrapping_add(1),
            Box::new(Ok(Library::default())),
        ));
        assert!(app.catalog_refreshing);
        let library = Library {
            catalog_issues: vec![crate::types::CatalogIssue {
                area: crate::types::CatalogArea::LiveStreams,
                message: "API unavailable".into(),
                elapsed_ms: 5,
            }],
            ..Library::default()
        };
        app.handle_event(AppEvent::CatalogRefreshed(
            generation,
            Box::new(Ok(library)),
        ));
        assert!(!app.catalog_refreshing);
        assert_eq!(app.library.channels.len(), channels);
        assert!(app.view == View::Live);
        assert_eq!(app.live_cat, "favorites");
        assert_eq!(app.current_live_id(), Some(channel));
    }

    #[test]
    fn previous_channel_is_retained_across_category_changes_and_preview_close() {
        let mut app = App::test_app();
        app.load_demo();
        let first = app
            .resolve_play(&format!("live:{}", app.library.channels[0].id))
            .unwrap();
        let second = app
            .resolve_play(&format!("live:{}", app.library.channels[1].id))
            .unwrap();
        app.remember_live(&first);
        app.preview = Some(first.clone());
        app.live_cat = "favorites".into();
        app.close_preview();
        app.remember_live(&second);
        assert_eq!(
            app.previous_live.as_ref().map(|item| &item.progress_key),
            Some(&first.progress_key)
        );
        app.remember_live(&first);
        assert_eq!(
            app.previous_live.as_ref().map(|item| &item.progress_key),
            Some(&second.progress_key)
        );
    }

    #[test]
    fn parental_gate_covers_play_preview_and_series_without_starting_media() {
        let mut app = App::test_app();
        app.active_profile = Some(Profile::new("Parent fixture", ProfileKind::M3u));
        let pid = app.active_profile.as_ref().unwrap().id.clone();
        app.parental.policy.set_pin("0123").unwrap();
        for (key, kind) in [
            ("live:1", PlayKind::Live),
            ("vod:2", PlayKind::Vod),
            ("ep:3:4", PlayKind::Episode),
        ] {
            app.parental
                .policy
                .blocked
                .insert(crate::parental::scoped_key(&pid, key));
            let item = PlayItem {
                title: "Protected".into(),
                url: "https://example.invalid/test".into(),
                kind,
                progress_key: key.into(),
            };
            app.play(item.clone());
            assert!(app.parental.open);
            assert!(app.player.is_none());
            assert!(app.current_play.is_none());
            app.mobile_back();
            assert!(!app.parental.open);
            app.start_preview(item.clone());
            assert!(app.parental.open);
            assert!(app.preview.is_none());
            app.parental.close();
            let grant = crate::parental::scoped_key(&pid, key);
            app.parental.grant = Some(grant);
            assert!(app.parental_gate(&item, false));
            assert!(!app.parental_gate(&item, false));
            app.parental.close();
        }
        assert!(app.parental.policy.locked(&pid, "ep:3:999"));
    }

    #[test]
    fn phone_tabs_keep_movies_and_series_separate_with_overlapping_provider_ids() {
        fn text(shape: &egui::epaint::Shape, out: &mut String) {
            match shape {
                egui::epaint::Shape::Text(t) => {
                    out.push_str(&t.galley.job.text);
                    out.push('\n');
                }
                egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| text(s, out)),
                _ => {}
            }
        }
        let mut app = App::test_app();
        app.active_profile = Some(Profile::new("Fixture", ProfileKind::M3u));
        app.library.loaded = true;
        app.library.movies = vec![crate::types::VodMovie {
            id: 42,
            name: "ONLY_MOVIE".into(),
            category_id: "1".into(),
            cover: String::new(),
            rating: String::new(),
            added: None,
            container_extension: "mp4".into(),
            stream_url: String::new(),
            plot: String::new(),
        }];
        app.library.series = vec![crate::types::Series {
            id: 42,
            name: "ONLY_SERIES".into(),
            category_id: "1".into(),
            cover: String::new(),
            added: None,
            plot: String::new(),
        }];
        app.vod_cat = "all".into();
        app.series_cat = "all".into();
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx, false);
        crate::theme::apply_warm(&ctx);
        for (view, expected, absent) in [
            (View::Movies, "ONLY_MOVIE", "ONLY_SERIES"),
            (View::Series, "ONLY_SERIES", "ONLY_MOVIE"),
            (View::Movies, "ONLY_MOVIE", "ONLY_SERIES"),
        ] {
            app.view = view;
            for _ in 0..3 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(393.0, 852.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| crate::ui::mobile::show(&mut app, ctx),
                );
                let mut labels = String::new();
                for shape in &output.shapes {
                    text(&shape.shape, &mut labels);
                }
                assert!(labels.contains(expected), "{labels}");
                assert!(!labels.contains(absent), "{labels}");
            }
        }
    }
    #[test]
    fn phone_catalogue_has_full_page_details_and_edge_back() {
        use egui::{epaint::Shape, pos2, vec2, Event, PointerButton, RawInput, Rect};
        fn labels(shape: &Shape, out: &mut Vec<(String, Rect)>) {
            match shape {
                Shape::Text(t) => out.push((
                    t.galley.job.text.clone(),
                    Rect::from_min_size(t.pos, t.galley.size()),
                )),
                Shape::Vec(s) => s.iter().for_each(|s| labels(s, out)),
                _ => {}
            }
        }
        for (w, h) in [
            (320.0, 568.0),
            (393.0, 852.0),
            (852.0, 393.0),
            (768.0, 1024.0),
        ] {
            let mut app = App::test_app();
            app.active_profile = Some(Profile::new("Phone fixture", ProfileKind::M3u));
            app.library.loaded = true;
            app.library.series = vec![crate::types::Series {
                id: 42,
                name: "Testserie".into(),
                added: None,
                cover: String::new(),
                category_id: "1".into(),
                plot: "Eine Beschreibung für die mobile Detailseite.".into(),
            }];
            app.series_episodes.insert(
                42,
                (1..=30)
                    .map(|n| crate::types::Episode {
                        id: n,
                        series_id: 42,
                        preview: String::new(),
                        season: 1,
                        episode: n,
                        title: "provider.file.name".into(),
                        container_extension: "mp4".into(),
                        stream_url: String::new(),
                    })
                    .collect(),
            );
            app.view = View::Series;
            app.tv = false;
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            crate::theme::apply(&ctx, true);
            let draw = |app: &mut App, events: Vec<Event>| {
                let output = ctx.run(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(w, h))),
                        events,
                        ..Default::default()
                    },
                    |ctx| crate::ui::mobile::show(app, ctx),
                );
                let mut text = Vec::new();
                for s in output.shapes {
                    if app.selected_series.is_none() && (w < 720.0 || h < 400.0) {
                        let mut row_labels = Vec::new();
                        labels(&s.shape, &mut row_labels);
                        if row_labels.iter().any(|(t, _)| t == "Testserie") {
                            assert!(
                                s.clip_rect.bottom() <= h - crate::ui::mobile::tokens::NAV_H + 1.0,
                                "Poster must not paint over bottom navigation: {:?}",
                                s.clip_rect
                            );
                        }
                    }
                    labels(&s.shape, &mut text);
                }
                text
            };
            draw(&mut app, vec![]);
            let text = draw(&mut app, vec![]);
            assert!(!text
                .iter()
                .any(|(t, _)| t == "Kategorien" || t == "Serie auswählen"));
            let poster = text.iter().find(|(t, _)| t == "Testserie").unwrap().1;
            assert!(poster.left() >= 12.0 && poster.right() <= w - 12.0);
            app.open_series(42);
            draw(&mut app, vec![]);
            let text = draw(&mut app, vec![]);
            assert!(text.iter().any(|(t, _)| t == "Staffel 1"));
            assert!(!text.iter().any(|(t, _)| t == "Live" || t == "Mehr"));
            // A vertical gesture starting at the edge must keep the detail open.
            let pointer = |x, y, pressed| Event::PointerButton {
                pos: pos2(x, y),
                button: PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            draw(&mut app, vec![pointer(8.0, 180.0, true)]);
            draw(&mut app, vec![pointer(12.0, 80.0, false)]);
            assert_eq!(app.selected_series, Some(42));
            draw(&mut app, vec![pointer(8.0, 180.0, true)]);
            draw(&mut app, vec![pointer(150.0, 185.0, false)]);
            assert_eq!(app.selected_series, None, "edge swipe at {w}x{h}");
            assert!(app.view == View::Series);
            app.view = View::Movies;
            app.selected_movie = Some(77);
            assert!(app.mobile_can_back());
            assert!(app.mobile_back());
            assert!(app.view == View::Movies);
            assert_eq!(app.selected_movie, None);
        }
    }

    #[test]
    fn tv_live_navigation_and_preview_fit_the_standard_tv_canvas() {
        use egui::epaint::Shape;
        fn labels(shape: &Shape, out: &mut Vec<(String, egui::Rect)>) {
            match shape {
                Shape::Text(t) => out.push((
                    t.galley.job.text.clone(),
                    egui::Rect::from_min_size(t.pos, t.galley.size()),
                )),
                Shape::Vec(shapes) => shapes.iter().for_each(|s| labels(s, out)),
                _ => {}
            }
        }
        let mut app = App::test_app();
        app.active_profile = Some(Profile::new("TV fixture", ProfileKind::M3u));
        let playlist = format!("#EXTM3U\n{}", (1..=16).map(|i| format!("#EXTINF:-1 group-title=\"Sport\",Dado Kanal {i:02}\nhttps://example.invalid/{i}\n")).collect::<String>());
        app.library = crate::client::parse_m3u(&playlist);
        let now = chrono::Utc::now().timestamp();
        for channel in &app.library.channels {
            app.epg.insert(
                channel.id,
                vec![
                    crate::types::EpgEntry {
                        title: "Heute".into(),
                        description: String::new(),
                        start: now - 60,
                        end: now + 60,
                    },
                    crate::types::EpgEntry {
                        title: "Morgen".into(),
                        description: String::new(),
                        start: now + 60,
                        end: now + 120,
                    },
                ],
            );
        }
        app.tv = true;
        app.tv_nav = 0;
        app.view = View::Live;
        app.preview = Some(PlayItem {
            title: "Vorschausender".into(),
            url: "https://example.invalid/test".into(),
            kind: PlayKind::Live,
            progress_key: "live:1".into(),
        });
        for (width, height) in [(960.0, 540.0), (1280.0, 720.0)] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            crate::theme::apply(&ctx, true);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, height),
                    )),
                    ..Default::default()
                },
                |ctx| crate::ui::mobile::show(&mut app, ctx),
            );
            let mut text = Vec::new();
            for shape in &output.shapes {
                labels(&shape.shape, &mut text);
            }
            for required in ["Kategorien", "Einstellungen", "Vorschau"] {
                let (_, rect) = text
                    .iter()
                    .find(|(name, _)| name == required)
                    .unwrap_or_else(|| panic!("Missing {required}: {text:?}"));
                assert!(
                    rect.right() <= width * 0.95 + 1.0 && rect.bottom() <= height * 0.95 + 1.0,
                    "{required} is clipped at {width}x{height}: {rect:?}"
                );
            }
            let channels: Vec<_> = text
                .iter()
                .filter(|(name, rect)| {
                    name.starts_with("Dado Kanal ") && rect.center().y + 25.0 <= height * 0.95
                })
                .collect();
            assert!(
                channels.len() >= 10,
                "Only {} complete channel rows at {width}x{height}: {text:?}",
                channels.len()
            );
            let rect_for = |name: &str| {
                text.iter()
                    .filter(|(t, _)| t == name)
                    .map(|(_, rect)| *rect)
                    .min_by(|a, b| a.top().total_cmp(&b.top()))
                    .unwrap()
            };
            assert!(
                (rect_for("Alle Sender").center().y - rect_for("Dado Player").center().y).abs()
                    < 2.0
            );
            assert!(
                (rect_for("Kategorien").center().y - rect_for("Alle Sender").center().y).abs()
                    < 2.0,
                "Category/title alignment: {:?} {:?}",
                rect_for("Kategorien"),
                rect_for("Alle Sender")
            );
            let current = rect_for("Jetzt: Heute");
            let next = rect_for("Danach: Morgen");
            assert!((current.center().y - next.center().y).abs() < 1.0);
            assert!(
                next.left() - current.right() >= 23.0,
                "EPG columns overlap: {current:?} {next:?}"
            );
        }
    }

    #[test]
    fn windows_live_preview_uses_shared_width_and_leaves_room_for_epg() {
        let mut app = App::test_app();
        app.active_profile = Some(Profile::new("Desktop fixture", ProfileKind::M3u));
        app.view = View::Live;

        let shared_width = crate::ui::desktop::appearance().live_preview_width;
        for (width, height, expected_width) in [
            (1920.0, 1080.0, shared_width),
            (1280.0, 420.0, 356.0),
            (740.0, 360.0, shared_width * 740.0 / 1920.0),
        ] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
            let draw = || egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let _ = ctx.run(draw(), |ctx| crate::ui::desktop::show(&mut app, ctx));
            let output = ctx.run(draw(), |ctx| crate::ui::desktop::show(&mut app, ctx));
            let video = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == egui::Color32::BLACK
                            && (rect.rect.aspect_ratio() - 16.0 / 9.0).abs() < 0.001 =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .max_by(|a, b| a.width().total_cmp(&b.width()))
                .expect("16:9 live preview");
            assert!((video.width() - expected_width).abs() < 2.0, "{video:?}");
            assert!(screen.contains_rect(video));
            assert!(screen.bottom() - video.bottom() > height * 0.30);
        }
    }

    #[test]
    fn mobile_home_keeps_channels_compact_and_favorites_separate_at_phone_sizes() {
        use egui::epaint::Shape;
        fn texts(shape: &Shape, out: &mut Vec<String>) {
            match shape {
                Shape::Text(t) => out.push(t.galley.job.text.clone()),
                Shape::Vec(shapes) => shapes.iter().for_each(|s| texts(s, out)),
                _ => {}
            }
        }
        let mut app = App::test_app();
        app.active_profile = Some(Profile::new("UI test", ProfileKind::M3u));
        app.library = crate::client::parse_m3u("#EXTM3U\n#EXTINF:-1,History only\nhttps://example.invalid/1\n#EXTINF:-1,Favorite only\nhttps://example.invalid/2");
        app.favorites = ["live:2".to_owned()].into_iter().collect();
        app.history = vec![crate::types::HistEntry {
            key: "live:1".into(),
            ts: 1,
            title: "History only".into(),
            sub: String::new(),
        }];
        app.view = View::Home;
        for (width, height) in [
            (320.0, 568.0),
            (393.0, 852.0),
            (440.0, 956.0),
            (852.0, 393.0),
        ] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            crate::theme::apply(&ctx, true);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, height),
                    )),
                    ..Default::default()
                },
                |ctx| crate::ui::mobile::show(&mut app, ctx),
            );
            let mut labels = Vec::new();
            for shape in &output.shapes {
                texts(&shape.shape, &mut labels);
            }
            assert!(
                labels.iter().any(|t| t == "Zuletzt gesehene Sender"),
                "{width}x{height}: {labels:?}"
            );
            assert!(labels.iter().any(|t| t == "Das läuft auf deinen Favoriten"));
            assert!(!labels
                .iter()
                .any(|t| t == "Abspielen" || t == "Weiter ansehen · Live-TV"));
            // The navigation's painted background must reach the full viewport
            // bottom; a safe-area-sized renderer used to leave a black strip.
            assert!(output.shapes.iter().any(|s| matches!(&s.shape, Shape::Rect(r) if (r.rect.bottom()-height).abs()<0.1 && r.rect.width()>width-1.0)));
        }
        assert!(app.favorites.contains("live:2"));
        assert!(!app.favorites.contains("live:1"));
    }

    #[test]
    fn channel_selection_previews_then_promotes_only_the_same_channel() {
        let mut app = App::test_app();
        app.view = View::Live;
        let item = PlayItem {
            title: "Channel".into(),
            url: "https://example.invalid/live.m3u8".into(),
            kind: PlayKind::Live,
            progress_key: "live:1".into(),
        };
        app.select_live_channel(item.clone());
        assert!(app.view == View::Live);
        assert!(app.is_previewing("live:1"));
        let mut second = item.clone();
        second.progress_key = "live:2".into();
        second.url = "https://example.invalid/second.m3u8".into();
        app.select_live_channel(second);
        assert!(app.view == View::Live);
        assert!(app.is_previewing("live:2"));
        app.select_live_channel(item.clone());
        assert!(app.view == View::Live);
        app.select_live_channel(item);
        assert!(app.view == View::Player);
        assert!(app.preview.is_none());
        assert_eq!(app.current_play.as_ref().unwrap().progress_key, "live:1");
    }

    use super::{
        preferred_track, App, AppEvent, ImageState, View, MAX_DETAIL_CACHE_ITEMS, MAX_DETAIL_ROWS,
    };
    use crate::types::{
        EpgEntry, Episode, Library, PlayItem, PlayKind, Profile, ProfileKind, Track,
    };

    #[test]
    fn old_profile_images_do_not_overwrite_the_new_provider() {
        let mut app = App::test_app();
        app.login_generation = 2;
        app.images.requested.insert("logo:1".into());
        app.handle_event(AppEvent::Image(
            1,
            "logo:1".into(),
            Ok(egui::ColorImage::new([1, 1], egui::Color32::WHITE)),
        ));
        assert!(app.images.pending.is_empty());
        assert!(app.images.requested.contains("logo:1"));
        app.handle_event(AppEvent::Image(
            2,
            "logo:1".into(),
            Ok(egui::ColorImage::new([1, 1], egui::Color32::WHITE)),
        ));
        assert_eq!(app.images.pending.len(), 1);
        assert!(!app.images.requested.contains("logo:1"));
    }

    #[test]
    fn background_results_wake_an_idle_ui_without_another_touch() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let app = App::test_app();
        let ctx = egui::Context::default();
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = wakes.clone();
        ctx.set_request_repaint_callback(move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
        });
        for _ in 0..4 {
            let _ = ctx.run(egui::RawInput::default(), |_| {});
        }
        wakes.store(0, Ordering::Relaxed);
        let _ = app.tx.context.set(ctx);
        let sender = app.tx.clone();
        std::thread::spawn(move || {
            assert!(sender
                .send(AppEvent::Image(
                    0,
                    "late-cover".into(),
                    Err("fixture".into())
                ))
                .is_ok());
        })
        .join()
        .unwrap();
        assert!(
            wakes.load(Ordering::Relaxed) > 0,
            "late image/login/EPG results must wake the event loop"
        );
        assert!(!app.rx.is_empty());
    }

    #[test]
    fn texture_budget_eviction_and_replacement_keep_accounting_consistent() {
        let ctx = egui::Context::default();
        let mut state = ImageState::default();
        let budget = 8;

        for (key, color) in [("one", egui::Color32::RED), ("two", egui::Color32::GREEN)] {
            let image = egui::ColorImage::new([1, 1], color);
            let texture = ctx.load_texture(key, image, egui::TextureOptions::LINEAR);
            state.insert_texture(key.to_string(), texture, 4, budget);
        }
        assert_eq!(state.total_texture_bytes, 8);
        assert_eq!(state.order.len(), 2);

        let replacement = ctx.load_texture(
            "one",
            egui::ColorImage::new([1, 1], egui::Color32::BLUE),
            egui::TextureOptions::LINEAR,
        );
        state.insert_texture("one".to_string(), replacement, 4, budget);
        assert_eq!(state.total_texture_bytes, 8);
        assert_eq!(state.order.iter().filter(|key| *key == "one").count(), 1);
        assert_eq!(state.textures.len(), 2);

        let third = ctx.load_texture(
            "three",
            egui::ColorImage::new([1, 1], egui::Color32::WHITE),
            egui::TextureOptions::LINEAR,
        );
        state.insert_texture("three".to_string(), third, 4, budget);
        assert_eq!(state.total_texture_bytes, 8);
        assert!(!state.textures.contains_key("two"));
        assert_eq!(
            state.order.iter().cloned().collect::<Vec<_>>(),
            ["one", "three"]
        );
        assert_eq!(state.texture_bytes.values().sum::<usize>(), 8);

        let oversized = ctx.load_texture(
            "oversized",
            egui::ColorImage::new([1, 1], egui::Color32::BLACK),
            egui::TextureOptions::LINEAR,
        );
        state.insert_texture("oversized".to_string(), oversized, 9, budget);
        assert_eq!(state.total_texture_bytes, 8);
        assert!(!state.textures.contains_key("oversized"));
    }

    #[test]
    fn stale_login_completion_cannot_replace_current_catalog() {
        let mut app = App::test_app();
        app.login_generation = 7;
        app.busy = true;
        app.library.loaded = true;
        app.view = View::Movies;
        app.active_profile = Some(Profile::new("current", ProfileKind::M3u));
        app.live_cat = "sports".to_string();

        let stale_profile = Profile::new("stale", ProfileKind::M3u);
        app.handle_event(AppEvent::LoginDone(
            6,
            Box::new(Ok((stale_profile, Library::default()))),
        ));

        assert!(app.busy);
        assert!(app.view == View::Movies);
        assert_eq!(app.live_cat, "sports");
        assert_eq!(app.active_profile.as_ref().unwrap().name, "current");
        assert!(app.library.loaded);
    }

    #[test]
    fn clearing_app_cache_rejects_stale_results_and_keeps_the_active_profile() {
        let mut app = App::test_app();
        app.load_demo();
        let profile = app.active_profile.as_ref().unwrap().id.clone();
        let channels = app.library.channels.len();
        let cached_generation = app.login_generation;
        app.epg.insert(1, Vec::new());
        app.series_episodes.insert(1, Vec::new());
        app.images.failed.insert("logo:1".into());
        app.catalog_refreshing = true;
        assert!(!app.clear_app_cache());
        app.catalog_refreshing = false;
        assert!(app.clear_app_cache());
        app.handle_event(AppEvent::Epg(cached_generation, 1, Ok(Vec::new())));
        app.handle_event(AppEvent::SeriesInfo(cached_generation, 1, Ok(Vec::new())));
        assert!(app.epg.is_empty());
        assert!(app.series_episodes.is_empty());
        assert!(app.images.failed.is_empty());
        assert_eq!(app.active_profile.as_ref().unwrap().id, profile);
        assert_eq!(app.library.channels.len(), channels);
    }

    #[test]
    fn stale_epg_and_series_completions_are_ignored() {
        let mut app = App::test_app();
        app.login_generation = 12;
        app.epg_requested.insert(42);
        app.series_requested.insert(9);

        let epg = EpgEntry {
            title: "Old programme".to_string(),
            description: String::new(),
            start: 100,
            end: 200,
        };
        let episode = Episode {
            id: 1,
            series_id: 9,
            preview: String::new(),
            season: 1,
            episode: 1,
            title: "Old episode".to_string(),
            container_extension: "mp4".to_string(),
            stream_url: "http://old/episode.mp4".to_string(),
        };

        app.handle_event(AppEvent::Epg(11, 42, Ok(vec![epg])));
        app.handle_event(AppEvent::SeriesInfo(11, 9, Ok(vec![episode])));

        // A stale completion must not clear request state belonging to the
        // current generation; provider IDs can overlap across profiles.
        assert!(app.epg_requested.contains(&42));
        assert!(app.series_requested.contains(&9));
        assert!(!app.epg.contains_key(&42));
        assert!(!app.series_episodes.contains_key(&9));
    }

    #[test]
    fn detail_caches_are_bounded_and_oversized_replies_are_truncated() {
        let mut app = App::test_app();
        let generation = app.login_generation;

        for id in 0..(MAX_DETAIL_CACHE_ITEMS as u32 + 40) {
            app.handle_event(AppEvent::Epg(generation, id, Ok(vec![])));
            app.handle_event(AppEvent::SeriesInfo(generation, id, Ok(vec![])));
        }
        assert!(app.epg.len() <= MAX_DETAIL_CACHE_ITEMS);
        assert!(app.series_episodes.len() <= MAX_DETAIL_CACHE_ITEMS);
        assert!(app.epg_order.len() <= MAX_DETAIL_CACHE_ITEMS);
        assert!(app.series_order.len() <= MAX_DETAIL_CACHE_ITEMS);
        assert!(app.epg.contains_key(&(MAX_DETAIL_CACHE_ITEMS as u32 + 39)));
        assert!(!app.epg.contains_key(&0), "oldest entries must be evicted");

        let huge = vec![
            EpgEntry {
                title: "x".to_string(),
                description: String::new(),
                start: 1,
                end: 2,
            };
            MAX_DETAIL_ROWS + 25
        ];
        app.handle_event(AppEvent::Epg(generation, 9000, Ok(huge)));
        assert_eq!(app.epg_for(9000).map(|v| v.len()), Some(MAX_DETAIL_ROWS));
    }

    #[test]
    fn failed_detail_requests_are_reported_instead_of_faking_empty_data() {
        let mut app = App::test_app();
        let generation = app.login_generation;
        app.epg_requested.insert(42);
        app.series_requested.insert(9);

        app.handle_event(AppEvent::Epg(generation, 42, Err("boom".to_string())));
        app.handle_event(AppEvent::SeriesInfo(
            generation,
            9,
            Err("offline".to_string()),
        ));

        assert!(app.epg_failed(42));
        assert!(app.series_failed(9));
        assert!(
            app.epg_for(42).is_none(),
            "an error must not look like empty EPG"
        );
        assert!(app.series_episodes_for(9).is_none());
        assert_eq!(app.epg_error.as_deref(), Some("boom"));
        assert_eq!(app.series_error.as_deref(), Some("offline"));
        assert!(!app.epg_requested.contains(&42));
        assert!(!app.series_requested.contains(&9));

        // A later success clears the failure marker again.
        app.handle_event(AppEvent::Epg(generation, 42, Ok(vec![])));
        assert!(!app.epg_failed(42));
        assert!(app.epg_for(42).is_some());
        assert!(app.epg_error.is_none());
    }

    #[test]
    fn detail_failures_never_clear_state_of_a_newer_login() {
        let mut app = App::test_app();
        app.login_generation = 5;
        app.epg_requested.insert(42);
        app.handle_event(AppEvent::Epg(4, 42, Err("boom".to_string())));
        assert!(!app.epg_failed(42));
        assert!(app.epg_error.is_none());
        assert!(app.epg_requested.contains(&42));
    }

    /// The headline bug: on the first login only Live-TV showed up and users
    /// had to log out/in repeatedly. A successful login must install *all*
    /// six catalogue areas at once, land on Home and drop every filter that
    /// belonged to the previous profile.
    #[test]
    fn first_login_installs_all_catalog_areas_and_resets_profile_scoped_state() {
        let mut app = App::test_app();
        let generation = app.login_generation;
        app.view = View::Login;
        app.busy = true;
        app.live_cat = "sports".to_string();
        app.vod_cat = "kids".to_string();
        app.series_cat = "drama".to_string();
        app.search_q = "alter suchbegriff".to_string();
        app.live_q = "ard".to_string();
        app.live_page = 7;
        app.vod_page = 4;
        app.series_page = 3;
        app.selected_series = Some(99);
        app.series_season = 4;
        app.epg_offset_hours = 6;
        app.epg.insert(42, vec![]);
        app.series_episodes.insert(9, vec![]);

        let profile = Profile::new("provider", ProfileKind::Xtream);
        app.handle_event(AppEvent::LoginDone(
            generation,
            Box::new(Ok((profile, crate::demo::library()))),
        ));

        assert!(!app.busy);
        assert!(app.view == View::Home);
        assert!(app.library.loaded);
        assert!(!app.library.live_categories.is_empty());
        assert!(!app.library.vod_categories.is_empty());
        assert!(!app.library.series_categories.is_empty());
        assert!(!app.library.channels.is_empty());
        assert!(!app.library.movies.is_empty());
        assert!(!app.library.series.is_empty());
        assert_eq!(app.live_cat, "all");
        assert_eq!(app.vod_cat, "all");
        assert_eq!(app.series_cat, "all");
        assert!(app.search_q.is_empty());
        assert!(app.live_q.is_empty());
        assert_eq!(app.live_page, 0);
        assert_eq!(app.vod_page, 0);
        assert_eq!(app.series_page, 0);
        assert_eq!(app.selected_series, None);
        assert_eq!(app.series_season, 0);
        assert_eq!(app.epg_offset_hours, 0);
        assert!(app.epg.is_empty(), "detail caches must be profile-scoped");
        assert!(app.series_episodes.is_empty());
        assert!(app.error.is_none());
    }

    #[test]
    fn logout_during_inflight_login_rejects_the_late_result() {
        let mut app = App::test_app();
        let old_generation = app.login_generation;
        app.busy = true;
        app.library.loaded = true;
        app.view = View::Movies;

        app.invalidate_login();
        app.handle_event(AppEvent::LoginDone(
            old_generation,
            Box::new(Ok((
                Profile::new("late", ProfileKind::M3u),
                crate::demo::library(),
            ))),
        ));

        assert!(!app.busy);
        assert!(app.view == View::Movies);
        assert_eq!(app.active_profile.as_ref().map(|p| p.name.as_str()), None);
        assert!(
            app.library.channels.is_empty(),
            "a late result must not silently install a catalogue after logout"
        );
    }

    #[test]
    fn concurrent_logins_keep_only_the_latest_generation() {
        let mut app = App::test_app();
        let first = app.login_generation;
        let second = first + 1;
        app.login_generation = second;

        let old = Profile::new("slow provider", ProfileKind::M3u);
        app.handle_event(AppEvent::LoginDone(
            first,
            Box::new(Ok((old, crate::demo::library()))),
        ));
        assert!(app.active_profile.is_none());
        assert!(app.library.channels.is_empty());

        let latest = Profile::new("fast provider", ProfileKind::Xtream);
        app.handle_event(AppEvent::LoginDone(
            second,
            Box::new(Ok((latest, crate::demo::library()))),
        ));
        assert_eq!(
            app.active_profile.as_ref().map(|p| p.name.as_str()),
            Some("fast provider")
        );
        assert!(!app.library.channels.is_empty());
        assert!(!app.library.movies.is_empty());
        assert!(!app.library.series.is_empty());
    }
    #[test]
    fn parental_dialog_is_centered_over_the_current_screen() {
        for (width, height) in [(1280.0, 720.0), (760.0, 360.0)] {
            let mut app = App::test_app();
            app.active_profile = Some(Profile::new("Dialog fixture", ProfileKind::M3u));
            app.library = crate::demo::library();
            app.view = View::Home;
            app.parental = Default::default();
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            crate::theme::apply(&ctx, true);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
            let draw = |app: &mut App| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| app.draw(ctx),
                )
            };
            let background = draw(&mut app);
            assert!(background.shapes.len() > 20);
            app.parental.open = true;
            for _ in 0..3 {
                draw(&mut app);
            }
            let frame = draw(&mut app);
            let dialog = ctx
                .memory(|memory| memory.area_rect(egui::Id::new("parental-dialog")))
                .unwrap();
            assert!(
                dialog.center().distance(screen.center()) <= 2.0,
                "{dialog:?} in {screen:?}"
            );
            assert!(screen.contains_rect(dialog));
            assert!(frame.shapes.len() > background.shapes.len());
            assert!(frame.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Rect(rect)
                    if rect.rect == screen && rect.fill == egui::Color32::from_black_alpha(138))
            }));
        }
    }

    #[test]
    fn opening_a_movie_offers_resume_only_for_a_meaningful_position() {
        let mut app = App::test_app();
        app.library = crate::demo::library();
        let key = format!("vod:{}", app.library.movies[0].id);
        app.durations.insert(key.clone(), 4200);
        app.set_progress(&key, 59);
        app.ask_resume(&key);
        assert!(app.pending_resume.is_none());

        app.set_progress(&key, 60);
        app.ask_resume(&key);
        assert_eq!(app.pending_resume.as_ref().map(|entry| entry.2), Some(60));
        assert!(app.mobile_back());
        assert_eq!(app.get_progress(&key), 60);
        assert!(app.current_play.is_none());

        app.set_progress(&key, 4140);
        app.ask_resume(&key);
        assert!(app.pending_resume.is_none());
    }

    #[test]
    fn series_resume_advances_at_sixty_seconds_across_seasons() {
        let mut app = App::test_app();
        app.history.clear();
        app.progress.clear();
        app.durations.clear();
        let episode = |id, season, number| Episode {
            id,
            series_id: 77,
            preview: String::new(),
            season,
            episode: number,
            title: "avx.provider.raw.filename".into(),
            container_extension: "mp4".into(),
            stream_url: "https://example.invalid/video.mp4".into(),
        };
        app.series_episodes.insert(
            77,
            vec![episode(3, 2, 1), episode(1, 1, 1), episode(2, 1, 2)],
        );
        assert_eq!(app.series_resume_target(77).unwrap().0.id, 1);
        app.history.push(crate::types::HistEntry {
            key: "ep:77:2".into(),
            ts: 0,
            title: "old".into(),
            sub: String::new(),
        });
        app.progress.insert("ep:77:2".into(), 1739);
        app.durations.insert("ep:77:2".into(), 1800);
        assert_eq!(
            app.series_resume_target(77).map(|(e, p)| (e.id, p)),
            Some((2, 1739))
        );
        app.progress.insert("ep:77:2".into(), 1740);
        assert_eq!(
            app.series_resume_target(77).map(|(e, p)| (e.id, p)),
            Some((3, 0))
        );
        app.durations.clear();
        assert_eq!(
            app.series_resume_target(77).map(|(e, p)| (e.id, p)),
            Some((2, 1740))
        );
    }
    #[test]
    fn untracked_series_playback_preserves_the_continue_target() {
        let mut app = App::test_app();
        app.view = View::Series;
        app.history = vec![crate::types::HistEntry {
            key: "ep:77:1".into(),
            title: "Saved episode".into(),
            sub: String::new(),
            ts: 1,
        }];
        app.progress =
            std::collections::HashMap::from([("ep:77:1".into(), 213), ("ep:77:2".into(), 121)]);
        let episode = |id, season| Episode {
            id,
            series_id: 77,
            preview: String::new(),
            season,
            episode: 1,
            title: String::new(),
            container_extension: "mp4".into(),
            stream_url: String::new(),
        };
        app.series_episodes
            .insert(77, vec![episode(1, 1), episode(2, 2)]);
        let target = app.series_resume_target(77).map(|(e, pos)| (e.id, pos));
        let item = app.resolve_play("ep:77:2").unwrap();
        app.play_untracked(item);
        assert!(app.untracked_playback);
        app.stop_play();
        assert_eq!(
            app.series_resume_target(77).map(|(e, pos)| (e.id, pos)),
            target
        );
        assert_eq!(app.history.len(), 1);
        assert_eq!(app.get_progress("ep:77:2"), 121);
        assert!(app.view == View::Series);
    }

    #[test]
    fn continue_waits_for_episode_details_and_explicit_other_play_cancels_it() {
        let mut app = App::test_app();
        app.active_profile = None;
        app.login_generation = 11;
        app.progress.insert("ep:77:2".into(), 321);
        app.play_history_key("ep:77:2");
        assert_eq!(app.pending_history_key.as_deref(), Some("ep:77:2"));
        let episode = Episode {
            id: 2,
            series_id: 77,
            preview: String::new(),
            season: 1,
            episode: 2,
            title: "provider.filename".into(),
            container_extension: "mp4".into(),
            stream_url: "https://example.invalid/video.mp4".into(),
        };
        app.handle_event(AppEvent::SeriesInfo(11, 77, Ok(vec![episode])));
        assert!(app.pending_history_key.is_none());
        assert_eq!(
            app.current_play.as_ref().map(|p| p.progress_key.as_str()),
            Some("ep:77:2")
        );
        assert_eq!(app.get_progress("ep:77:2"), 321);
        app.play_history_key("ep:99:7");
        assert!(app.pending_history_key.is_some());
        app.play(PlayItem {
            title: "Other".into(),
            url: "https://example.invalid/live".into(),
            kind: PlayKind::Live,
            progress_key: "live:1".into(),
        });
        assert!(app.pending_history_key.is_none());
    }
    #[test]
    fn track_preferences_accept_language_aliases_without_matching_unrelated_titles() {
        let tracks = vec![
            Track {
                id: 1,
                kind: "audio".into(),
                lang: "eng".into(),
                title: "Deutsch commentary".into(),
            },
            Track {
                id: 2,
                kind: "audio".into(),
                lang: "deu-DE".into(),
                title: "".into(),
            },
        ];
        assert_eq!(preferred_track(&tracks, "de").map(|t| t.id), Some(2));
        assert_eq!(preferred_track(&tracks, "en-US").map(|t| t.id), Some(1));
        assert!(preferred_track(&tracks, "").is_none());
        assert!(preferred_track(&tracks, "tr").is_none());
    }
    #[test]
    fn image_replies_are_uploaded_in_small_frame_batches_without_losing_covers() {
        let mut app = App::test_app();
        let ctx = egui::Context::default();
        for i in 0..9 {
            app.images.pending.push((
                format!("budget-{i}"),
                egui::ColorImage::new([2, 2], egui::Color32::WHITE),
            ));
        }
        let mut total = 0;
        while !app.images.pending.is_empty() {
            let uploaded = app.upload_ready_images(&ctx);
            assert!((1..=2).contains(&uploaded));
            total += uploaded;
            assert_eq!(app.images.pending.len(), 9 - total);
        }
        assert_eq!(app.images.textures.len(), 9);
    }

    #[cfg(windows)]
    #[test]
    fn back_from_live_player_keeps_preview_and_restores_the_channel() {
        let mut app = App::test_app();
        app.load_demo();
        app.view = View::Live;
        let channel = app.library.channels[0].id;
        let key = format!("live:{channel}");
        app.play(app.resolve_play(&key).unwrap());
        app.back_from_player();
        assert!(app.view == View::Live);
        assert!(app.current_play.is_none());
        assert_eq!(
            app.preview.as_ref().map(|item| item.progress_key.as_str()),
            Some(key.as_str())
        );
        assert_eq!(app.desktop_return_item.as_deref(), Some(key.as_str()));
        assert_eq!(app.scroll_to, Some(channel));
    }

    #[cfg(windows)]
    #[test]
    fn live_ok_returns_from_home_to_the_current_channel() {
        let mut app = App::test_app();
        app.load_demo();
        app.view = View::Home;
        app.live_cat = "different category".into();
        app.live_q = "different channel".into();
        let id = app.library.channels[0].id;
        let key = format!("live:{id}");
        app.play(app.resolve_play(&key).unwrap());
        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.handle_keys(ctx),
        );
        assert!(app.view == View::Live);
        assert_eq!(app.live_cat, "all");
        assert!(app.live_q.is_empty());
        assert_eq!(app.scroll_to, Some(id));
        assert_eq!(
            app.preview.as_ref().map(|item| item.progress_key.as_str()),
            Some(key.as_str())
        );
    }

    #[cfg(windows)]
    #[test]
    fn disabling_watch_history_hides_progress_and_does_not_record_playback() {
        let mut app = App::test_app();
        app.load_demo();
        let movie = app.library.movies[0].id;
        let key = format!("vod:{movie}");
        let item = app.resolve_play(&key).unwrap();
        app.set_progress(&key, 100);
        let original_count = app.history.len();
        app.settings.watch_history = false;
        assert!(app.visible_history().is_empty());
        assert_eq!(app.get_progress(&key), 0);
        app.record_history(&item);
        app.set_progress(&key, 200);
        assert_eq!(app.history.len(), original_count);
        assert_eq!(app.progress.get(&key), Some(&100));
    }

    #[test]
    fn color_shortcuts_use_focused_rows_and_do_not_repeat_favorites() {
        let mut app = App::test_app();
        app.load_demo();
        app.favorites.clear();
        let ctx = egui::Context::default();
        let keypress = |app: &mut App, key, repeat| {
            let event = |pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat,
                modifiers: egui::Modifiers::NONE,
            };
            let events = if repeat {
                vec![event(true)]
            } else {
                vec![event(false), event(true)]
            };
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.handle_keys(ctx),
            );
        };
        keypress(&mut app, egui::Key::F2, false);
        assert!(app.view == View::Live);
        let channel = app.library.channels[0].id;
        let focused_key = format!("live:{}", app.library.channels[1].id);
        let focused = egui::Id::new("fixture-focused-row");
        app.live_selected = Some(channel);
        ctx.memory_mut(|memory| memory.request_focus(focused));
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new("desktop-favorite-target"),
                (focused, focused_key.clone()),
            )
        });
        keypress(&mut app, egui::Key::F3, false);
        assert!(app.is_favorite(&focused_key));
        assert!(!app.is_favorite(&format!("live:{channel}")));
        keypress(&mut app, egui::Key::F3, true);
        assert!(app.is_favorite(&focused_key));
        app.nav_collapsed = true;
        keypress(&mut app, egui::Key::F1, false);
        assert!(app.parental.open);
        assert!(app.nav_collapsed);
        keypress(&mut app, egui::Key::F1, false);
        assert!(app.parental.open);
        keypress(&mut app, egui::Key::Escape, false);
        assert!(!app.parental.open);
        keypress(&mut app, egui::Key::Escape, false);
        assert!(!app.nav_collapsed);
        keypress(&mut app, egui::Key::F4, false);
        assert!(app.view == View::Live);
        app.navigate(View::Movies);
        keypress(&mut app, egui::Key::F4, false);
        assert!(app.view == View::Movies);
        assert_eq!(
            ctx.data(|data| data.get_temp::<bool>(egui::Id::new(("classic-catalog-sort", false)))),
            Some(true)
        );
        app.play(app.resolve_play(&format!("live:{channel}")).unwrap());
        app.settings.desktop_aspect = crate::types::DesktopAspect::Fit;
        for expected in [
            crate::types::DesktopAspect::Stretch,
            crate::types::DesktopAspect::Auto,
            crate::types::DesktopAspect::Fit,
        ] {
            keypress(&mut app, egui::Key::F4, false);
            assert_eq!(app.settings.desktop_aspect, expected);
            assert!(!crate::ui::player_desktop::menus_open(&ctx));
            assert!(app.view == View::Player);
        }
    }

    #[test]
    fn password_keeps_focus_when_device_address_changes_login_layout() {
        use crate::ui::mobile::components::{field, Kb};
        let ctx = egui::Context::default();
        let mut password = String::new();
        let mut show_password = false;
        let mut original = None;
        for address_loaded in [false, true] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if address_loaded {
                        ui.label("Geräteadresse (MAC): 02:11:22:33:44:55");
                        ui.label("Profile aktualisieren");
                    }
                    let response = field(
                        ui,
                        "Passwort",
                        &mut password,
                        "Passwort",
                        Kb::Password,
                        &mut show_password,
                    );
                    if let Some(id) = original {
                        assert_eq!(response.id, id);
                        assert!(response.has_focus());
                    } else {
                        response.request_focus();
                        original = Some(response.id);
                    }
                });
            });
        }
    }

    #[test]
    fn interface_layout_defaults_to_classic_and_round_trips_modern() {
        use crate::types::{InterfaceLayout, Settings};
        let mut settings = serde_json::to_value(Settings::default()).unwrap();
        settings.as_object_mut().unwrap().remove("interface_layout");
        let mut restored: Settings = serde_json::from_value(settings).unwrap();
        assert_eq!(restored.interface_layout, InterfaceLayout::Classic);
        restored.interface_layout = InterfaceLayout::Modern;
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
        assert_eq!(restored.interface_layout, InterfaceLayout::Modern);
    }

    #[test]
    fn live_keyboard_seek_preserves_the_channel_and_vertical_arrows_zap() {
        let mut app = App::test_app();
        app.load_demo();
        app.active_profile = None;
        let first = app.library.channels[0].id;
        let second = app.library.channels[1].id;
        let item = app.resolve_play(&format!("live:{first}")).unwrap();
        app.play(item);
        let ctx = egui::Context::default();
        for (key, channel) in [
            (egui::Key::ArrowLeft, first),
            (egui::Key::ArrowRight, first),
            (egui::Key::ArrowUp, second),
            (egui::Key::ArrowDown, first),
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| app.handle_keys(ctx),
            );
            assert_eq!(app.current_live_id(), Some(channel));
        }
    }

    #[test]
    fn file_player_arrows_enter_menu_without_changing_volume_and_hide_resets_mode() {
        let mut app = App::test_app();
        app.view = View::Player;
        app.current_play = Some(PlayItem {
            title: "File".into(),
            url: "https://example.invalid/movie.mkv".into(),
            kind: PlayKind::Vod,
            progress_key: "vod:7".into(),
        });
        let ctx = egui::Context::default();
        for key in [egui::Key::ArrowDown, egui::Key::ArrowUp] {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| app.handle_keys(ctx),
            );
            assert!(app.player_menu_navigation);
            assert_eq!(app.volume, 100);
        }
        app.player_chrome = false;
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.handle_keys(ctx),
        );
        assert!(!app.player_menu_navigation);
    }

    #[test]
    fn poster_catalogue_only_draws_visible_titles_and_opens_saved_season() {
        use egui::epaint::Shape;
        fn texts(s: &Shape, out: &mut Vec<String>) {
            match s {
                Shape::Text(t) => out.push(t.galley.job.text.clone()),
                Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                _ => {}
            }
        }
        let mut app = App::test_app();
        app.library = crate::demo::library();
        app.active_profile = Some(Profile::new("UI test", ProfileKind::M3u));
        let template = app.library.movies[0].clone();
        app.library.movies = (0..20_000)
            .map(|i| {
                let mut m = template.clone();
                m.id = i;
                m.name = format!("Poster {i}");
                m.cover.clear();
                m
            })
            .collect();
        app.vod_cat = "all".into();
        app.catalog_queries[0].clear();
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        crate::theme::apply(&ctx, true);
        let frame = |app: &mut App, series| {
            let frame_time = ctx.input(|input| input.time) + 0.35;
            ctx.run(
                egui::RawInput {
                    time: Some(frame_time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| crate::ui::library::show(app, ui, series));
                },
            )
        };
        let out = frame(&mut app, false);
        let mut all = Vec::new();
        for s in out.shapes {
            texts(&s.shape, &mut all);
        }
        let posters = all.iter().filter(|t| t.starts_with("Poster ")).count();
        assert!(
            posters > 0 && posters < 100,
            "{posters} titles drawn from 20000"
        );
        app.selected_movie = Some(0);
        app.catalog_focus_play = true;
        let _ = frame(&mut app, false);
        assert!(!app.catalog_focus_play);
        let play_focus = ctx.memory(|m| m.focused()).expect("Dado play button");
        app.view = View::Movies;
        assert!(app.close_catalog_detail());
        app.desktop_return_item = Some("vod:17999".into());
        let _ = frame(&mut app, false);
        assert!(app.desktop_return_item.is_none());
        assert_ne!(ctx.memory(|m| m.focused()), Some(play_focus));
        assert!(
            app.current_play.is_none(),
            "Selecting a poster focuses play, it must not start playback"
        );
        app.library.series[0].id = 77;
        app.selected_series = Some(77);
        app.series_season = 0;
        app.series_episodes.insert(
            77,
            (1..=12)
                .flat_map(|season| {
                    (1..=20).map(move |episode| Episode {
                        id: season * 100 + episode,
                        series_id: 77,
                        preview: String::new(),
                        season,
                        episode,
                        title: "avx.raw.provider.filename".into(),
                        container_extension: "mp4".into(),
                        stream_url: String::new(),
                    })
                })
                .collect(),
        );
        app.history = vec![crate::types::HistEntry {
            key: "ep:77:506".into(),
            title: "Saved".into(),
            sub: String::new(),
            ts: 1,
        }];
        app.progress.insert("ep:77:506".into(), 300);
        app.durations.insert("ep:77:506".into(), 1800);
        let _ = frame(&mut app, true);
        let out = frame(&mut app, true);
        let mut all = Vec::new();
        for s in out.shapes {
            texts(&s.shape, &mut all);
        }
        assert_eq!(app.series_season, 5);
        assert!(all.iter().any(|t| t == "Folge 6"));
        assert!(!all.iter().any(|t| t.contains("avx.raw")));
    }
}
