use crate::client;
use crate::mpv::Mpv;
use crate::store;
use crate::types::*;
use crate::ui;
use crate::platform::VideoSurface;
use egui::Context;
use std::collections::{HashMap, HashSet};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Login,
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
    LoginDone(Result<(Profile, Library), String>),
    Epg(u32, Result<Vec<EpgEntry>, String>),
    SeriesInfo(u32, Result<Vec<Episode>, String>),
    Image(String, Result<(Vec<u8>, u32, u32), String>),
}

#[derive(Default)]
pub struct ImageState {
    pub textures: HashMap<String, egui::TextureHandle>,
    pub pending: Vec<(String, Vec<u8>, u32, u32)>,
    pub requested: HashSet<String>,
    pub failed: HashSet<String>,
    pub order: std::collections::VecDeque<String>,
}

pub struct Player {
    /// Declared before `surface` so mpv is torn down before its window.
    pub mpv: Mpv,
    pub surface: VideoSurface,
    pub current_key: String,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContinueTab {
    Live,
    Movies,
    Series,
}

pub struct App {
    rt: Runtime,
    tx: UnboundedSender<AppEvent>,
    rx: UnboundedReceiver<AppEvent>,

    pub profiles: Vec<Profile>,
    pub active_profile: Option<Profile>,
    pub library: Library,
    pub view: View,
    previous_view: View,
    pub busy: bool,
    pub error: Option<String>,
    pub info: Option<String>,

    // login form
    pub login_kind: ProfileKind,
    pub f_name: String,
    pub f_host: String,
    pub f_user: String,
    pub f_pass: String,
    pub f_m3u_url: String,
    pub f_m3u_file: String,

    // ui state
    pub live_cat: String,
    pub vod_cat: String,
    pub series_cat: String,
    pub search_q: String,
    pub live_q: String,
    pub selected_series: Option<u32>,
    pub series_season: u32,
    pub live_page: usize,
    pub vod_page: usize,
    pub series_page: usize,

    pub favorites: HashSet<String>,
    pub progress: HashMap<String, u64>,
    /// Recently watched items (newest first) for the Continue view.
    pub history: Vec<HistEntry>,
    pub continue_tab: ContinueTab,
    pub settings: Settings,

    pub images: ImageState,
    pub epg: HashMap<u32, Vec<EpgEntry>>,
    epg_requested: HashSet<u32>,
    pub series_episodes: HashMap<u32, Vec<Episode>>,
    series_requested: HashSet<u32>,

    pub current_play: Option<PlayItem>,
    /// Channel currently shown in the inline preview (list stays visible).
    pub preview: Option<PlayItem>,
    /// Channel ids in the order the Live list currently shows them (for ←/→).
    pub live_order: Vec<u32>,
    /// Ask the Live list to scroll this channel into view.
    pub scroll_to: Option<u32>,
    pending_fullscreen: Option<bool>,
    applied_dark: Option<bool>,
    system_dark: bool,
    last_theme_poll: f64,
    preview_placed: bool,
    /// Last video rect in device pixels, so we do not resize every frame.
    last_video_rect: Option<(i32, i32, i32, i32)>,
    /// Timestamp of the last mouse/key activity (player controls auto-hide).
    pub ui_activity: f64,
    last_cursor: (i32, i32),
    pub player: Option<Player>,
    pub player_error: Option<String>,
    main_hwnd: isize,
    pub fullscreen: bool,
    pub volume: i64,
    autostart_test: bool,
}

impl App {
    pub fn new(rt: Runtime) -> Self {
        let (tx, rx) = unbounded_channel();
        let mut app = App {
            rt,
            tx,
            rx,
            profiles: store::load_profiles(),
            active_profile: None,
            library: Library::default(),
            view: View::Login,
            previous_view: View::Live,
            busy: false,
            error: None,
            info: None,
            login_kind: ProfileKind::Xtream,
            f_name: String::new(),
            f_host: String::new(),
            f_user: String::new(),
            f_pass: String::new(),
            f_m3u_url: String::new(),
            f_m3u_file: String::new(),
            live_cat: "all".to_string(),
            vod_cat: "all".to_string(),
            series_cat: "all".to_string(),
            search_q: String::new(),
            live_q: String::new(),
            selected_series: None,
            series_season: 0,
            live_page: 0,
            vod_page: 0,
            series_page: 0,
            favorites: store::load_favorites(),
            progress: store::load_progress(),
            history: store::load_history(),
            continue_tab: ContinueTab::Movies,
            settings: store::load_settings(),
            images: ImageState::default(),
            epg: HashMap::new(),
            epg_requested: HashSet::new(),
            series_episodes: HashMap::new(),
            series_requested: HashSet::new(),
            current_play: None,
            preview: None,
            live_order: Vec::new(),
            scroll_to: None,
            pending_fullscreen: None,
            applied_dark: None,
            system_dark: true,
            last_theme_poll: -10.0,
            preview_placed: false,
            last_video_rect: None,
            ui_activity: 0.0,
            last_cursor: (0, 0),
            player: None,
            player_error: None,
            main_hwnd: 0,
            fullscreen: false,
            volume: 100,
            autostart_test: std::env::var("IPTV_TEST_STREAM").is_ok(),
        };
        // Layout check without a provider account (never persisted).
        if std::env::var("IPTV_DEMO").is_ok() {
            app.library = crate::demo::library();
            app.active_profile = Some(Profile::new("Demo", ProfileKind::Xtream));
            app.view = match std::env::var("IPTV_DEMO_VIEW").as_deref() {
                Ok("movies") => View::Movies,
                Ok("series") => View::Series,
                Ok("settings") => View::Settings,
                Ok("continue") => View::Continue,
                _ => View::Live,
            };
            if app.history.is_empty() {
                app.history = crate::demo::seed_history(&app.library);
                for h in app.history.clone() {
                    if h.key.starts_with("vod:") {
                        app.progress.insert(h.key, 1234);
                    }
                }
            }
            if app.view == View::Series {
                app.selected_series = Some(app.library.series.first().map(|s| s.id).unwrap_or(0));
                app.series_episodes.insert(
                    2000,
                    (1..=6)
                        .map(|i| Episode {
                            id: 9000 + i,
                            series_id: 2000,
                            season: if i <= 3 { 1 } else { 2 },
                            episode: if i <= 3 { i } else { i - 3 },
                            title: format!("Demo Episode {}", i),
                            container_extension: "mkv".to_string(),
                            stream_url: String::new(),
                        })
                        .collect(),
                );
            }
        }
        app
    }

    pub fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);

        // Theme: "System" follows Windows, polled so a live OS switch is picked up.
        if now - self.last_theme_poll > 1.5 {
            self.last_theme_poll = now;
            let dark = crate::platform::sys::system_dark();
            if dark != self.system_dark {
                self.system_dark = dark;
                ctx.request_repaint();
            }
        }
        let want_dark = self.settings.theme.resolve(self.system_dark);
        if self.applied_dark != Some(want_dark) {
            self.applied_dark = Some(want_dark);
            // egui only repaints on input; without this the new colours would
            // not show up until the next mouse move.
            ctx.request_repaint();
        }
        crate::theme::apply(ctx, want_dark);
        self.preview_placed = false;

        // Remember the native surface handle for embedding the video surface.
        // Platform-neutral: on Windows this is the HWND, on Android the
        // ANativeWindow pointer – both are exposed via `native_handle()`.
        if let Some(s) = crate::platform::acquire_surface(frame) {
            self.main_hwnd = s.native_handle();
        }

        while let Ok(ev) = self.rx.try_recv() {
            self.handle_event(ev);
        }

        // Poll the player and keep playback progress in memory.
        let polled = self.player.as_ref().map(|pl| {
            (
                pl.mpv.position(),
                pl.mpv.duration(),
                pl.mpv.paused(),
                pl.current_key.clone(),
            )
        });
        if let Some((pos, dur, paused, key)) = polled {
            if let Some(pl) = &mut self.player {
                pl.position = pos;
                pl.duration = dur;
                pl.paused = paused;
            }
            if !key.is_empty() && pos > 1.0 {
                self.progress.insert(key, pos as u64);
            }
        }

        // Keyboard shortcuts (list, preview and full player).
        self.handle_keys(ctx);

        if !self.images.pending.is_empty() {
            let pending = std::mem::take(&mut self.images.pending);
            for (key, rgba, w, h) in pending {
                let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                let tex = ctx.load_texture(key.clone(), ci, egui::TextureOptions::LINEAR);
                self.images.textures.insert(key.clone(), tex);
                self.images.order.push_back(key);
            }
            // Bound memory: evict oldest textures (FIFO) above the cap.
            while self.images.order.len() > 300 {
                if let Some(old) = self.images.order.pop_front() {
                    self.images.textures.remove(&old);
                    self.images.requested.remove(&old);
                }
            }
        }

        if self.autostart_test && self.main_hwnd != 0 {
            self.autostart_test = false;
            self.play_test_stream();
        }

        self.draw(ctx);

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
            let ms = if self.view == View::Player { 120 } else { 400 };
            ctx.request_repaint_after(std::time::Duration::from_millis(ms));
        }
    }

    fn draw(&mut self, ctx: &Context) {
        // Leaving the channel list closes the preview (and stops its audio).
        if self.view != View::Live && self.preview.is_some() {
            self.close_preview();
        }

        if self.view == View::Player && self.current_play.is_some() {
            // Frameless so the video can use the whole window edge to edge.
            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx, |ui| ui::player::show(self, ui));
            return;
        }

        if self.active_profile.is_none() {
            ui::login::show(self, ctx);
            return;
        }

        egui::SidePanel::left("nav")
            .resizable(true)
            .default_width(224.0)
            .min_width(186.0)
            .show(ctx, |ui| ui::sidebar::show(self, ui));

        if self.view == View::Live && self.preview.is_some() {
            egui::SidePanel::right("preview")
                .resizable(true)
                .default_width(440.0)
                .min_width(300.0)
                .max_width(780.0)
                .show(ctx, |ui| ui::live::preview_panel(self, ui));
        }

        if self.view == View::Series && self.selected_series.is_some() {
            egui::SidePanel::right("series_detail")
                .resizable(true)
                .default_width(430.0)
                .min_width(320.0)
                .max_width(720.0)
                .show(ctx, |ui| ui::series::detail_panel(self, ui));
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Live => ui::live::show(self, ui),
            View::Movies => ui::vod::show(self, ui),
            View::Series => ui::series::show(self, ui),
            View::Search => ui::search::show(self, ui),
            View::Favorites => ui::favorites::show(self, ui),
            View::Continue => ui::continue_watching::show(self, ui),
            View::Settings => ui::settings::show(self, ui),
            View::Player => ui::player::show(self, ui),
            View::Login => {}
        });
    }

    fn handle_event(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::LoginDone(res) => {
                self.busy = false;
                match res {
                    Ok((profile, lib)) => {
                        if let Some(existing) = self.profiles.iter_mut().find(|p| p.id == profile.id) {
                            *existing = profile.clone();
                        } else {
                            self.profiles.push(profile.clone());
                        }
                        let _ = store::save_profiles(&self.profiles);
                        self.active_profile = Some(profile);
                        self.library = lib;
                        self.view = View::Live;
                        self.error = None;
                        self.epg.clear();
                        self.epg_requested.clear();
                        self.series_episodes.clear();
                        self.series_requested.clear();
                    }
                    Err(e) => {
                        self.error = Some(redact(&e));
                    }
                }
            }
            AppEvent::Epg(id, res) => {
                self.epg.insert(id, res.unwrap_or_default());
            }
            AppEvent::SeriesInfo(id, res) => {
                self.series_episodes.insert(id, res.unwrap_or_default());
            }
            AppEvent::Image(key, res) => match res {
                Ok((rgba, w, h)) => self.images.pending.push((key, rgba, w, h)),
                Err(_) => {
                    self.images.failed.insert(key);
                }
            },
        }
    }

    // ---- login ----

    pub fn start_login(&mut self, profile: Profile) {
        self.busy = true;
        self.error = None;
        self.info = None;
        let tx = self.tx.clone();
        let to = self.settings.network_timeout_s;
        self.rt.spawn(async move {
            match client::load_profile(&profile, to).await {
                Ok(lib) => {
                    let _ = tx.send(AppEvent::LoginDone(Ok((profile, lib))));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::LoginDone(Err(e.to_string())));
                }
            }
        });
    }

    pub fn submit_form(&mut self) {
        let name = if self.f_name.trim().is_empty() {
            match self.login_kind {
                ProfileKind::Xtream => self.f_host.trim().to_string(),
                ProfileKind::M3u => self.f_m3u_url.trim().to_string(),
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
                    return;
                }
                p.host = self.f_host.trim().to_string();
                p.username = self.f_user.trim().to_string();
                p.password = self.f_pass.clone();
            }
            ProfileKind::M3u => {
                if self.f_m3u_url.trim().is_empty() && self.f_m3u_file.trim().is_empty() {
                    self.error = Some("M3U-URL oder Datei angeben.".to_string());
                    return;
                }
                p.m3u_url = self.f_m3u_url.trim().to_string();
                p.m3u_file = self.f_m3u_file.trim().to_string();
            }
        }
        self.start_login(p);
    }

    pub fn delete_profile(&mut self, id: &str) {
        self.profiles.retain(|p| p.id != id);
        let _ = store::save_profiles(&self.profiles);
    }

    pub fn logout(&mut self) {
        self.active_profile = None;
        self.library = Library::default();
        self.view = View::Login;
        self.current_play = None;
        self.epg.clear();
        self.epg_requested.clear();
        self.series_episodes.clear();
        self.series_requested.clear();
    }

    // ---- playback ----

    /// Remember an item in the recently-watched list (newest first).
    fn record_history(&mut self, item: &PlayItem) {
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
        self.history.truncate(60);
        let _ = store::save_history(&self.history);
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

    pub fn play(&mut self, item: PlayItem) {
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

        let resume = if item.kind != PlayKind::Live {
            self.get_progress(&item.progress_key)
        } else {
            0
        };

        self.current_play = Some(item.clone());
        self.view = View::Player;
        self.record_history(&item);
        // Negative = "controls freshly opened, start the auto-hide timer now".
        self.ui_activity = -1.0;

        if self.ensure_player() {
            let vol = self.volume;
            if let Some(pl) = &mut self.player {
                pl.current_key = item.progress_key.clone();
                pl.mpv.load(&item.url);
                if resume > 0 {
                    pl.mpv.seek_absolute(resume as f64);
                }
                pl.mpv.set_volume(vol);
                pl.mpv.set_pause(false);
                pl.position = 0.0;
                pl.duration = 0.0;
                pl.paused = false;
            }
        }
    }

    pub fn play_test_stream(&mut self) {
        self.play(PlayItem {
            title: "Teststream (Big Buck Bunny)".to_string(),
            url: "https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8".to_string(),
            kind: PlayKind::Vod,
            progress_key: String::new(),
        });
    }

    /// Start (or switch) the inline preview while the channel list stays visible.
    pub fn start_preview(&mut self, item: PlayItem) {
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
        if let Some(pl) = &mut self.player {
            pl.current_key = item.progress_key.clone();
            pl.mpv.load(&item.url);
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
    }

    /// Move the running preview into the full player window.
    pub fn promote_preview(&mut self, fullscreen: bool) {
        if let Some(item) = self.preview.take() {
            self.previous_view = View::Live;
            self.current_play = Some(item);
            self.view = View::Player;
            if fullscreen {
                self.pending_fullscreen = Some(true);
            }
        }
    }

    /// Throw away the mpv instance so new decoder/output settings take effect.
    pub fn reset_player(&mut self) {
        self.preview = None;
        self.player = None;
        self.last_video_rect = None;
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

    fn handle_keys(&mut self, ctx: &Context) {
        // Never steal keys while a text field has focus.
        if ctx.wants_keyboard_input() {
            return;
        }
        let (left, right, up, down, space, mute, full, esc, plus, minus, enter) = ctx.input(|i| {
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
                i.key_pressed(egui::Key::Enter),
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

        // ---- volume: ↑/↓ in the player, +/- everywhere ----
        let mut vol = self.volume;
        let mut vol_changed = false;
        if (in_player && up) || plus {
            vol = (vol + 5).min(100);
            vol_changed = true;
        }
        if (in_player && down) || minus {
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
        if enter && has_preview && !in_player {
            self.promote_preview(false);
            return;
        }

        if full {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }
        if esc {
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

        // ---- arrows ----
        // Main menu: ↑/↓ (and ←/→) zap through the channels and load the preview.
        if in_live_list && !in_player {
            if up || left {
                self.step_channel(-1);
            }
            if down || right {
                self.step_channel(1);
            }
            return;
        }

        if left || right {
            let delta = if left { -1 } else { 1 };
            if is_live {
                self.step_channel(delta);
            } else if let Some(pl) = &self.player {
                pl.mpv.seek_relative(delta as f64 * 10.0);
            }
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
        if self.player.is_some() {
            return true;
        }
        if self.main_hwnd == 0 {
            self.player_error = Some("Kein Fenster-Handle verfügbar.".to_string());
            return false;
        }
        let surface = match VideoSurface::create(self.main_hwnd as *mut std::ffi::c_void) {
            Some(s) => s,
            None => {
                self.player_error = Some("Video-Fenster konnte nicht erstellt werden.".to_string());
                return false;
            }
        };
        match Mpv::new(surface.native_handle(), &self.settings) {
            Ok(mpv) => {
                self.player = Some(Player {
                    mpv,
                    surface,
                    current_key: String::new(),
                    position: 0.0,
                    duration: 0.0,
                    paused: false,
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

    pub fn position_video(&mut self, rect: egui::Rect, ppp: f32) {
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
        }
    }

    pub fn stop_play(&mut self) {
        let (key, pos) = if let Some(pl) = &self.player {
            (pl.current_key.clone(), pl.position)
        } else {
            (String::new(), 0.0)
        };
        if let Some(pl) = &self.player {
            pl.mpv.stop();
            pl.surface.hide();
        }
        self.last_video_rect = None;
        if !key.is_empty() {
            self.progress.insert(key, pos as u64);
        }
        let _ = store::save_progress(&self.progress);
        self.current_play = None;
        self.view = self.previous_view;
        if self.fullscreen {
            self.pending_fullscreen = Some(false);
        }
    }

    // ---- images ----

    pub fn request_image(&mut self, key: String, url: String) {
        if url.is_empty() {
            return;
        }
        if self.images.textures.contains_key(&key)
            || self.images.requested.contains(&key)
            || self.images.failed.contains(&key)
        {
            return;
        }
        // Bound memory: don't load more than ~600 images at once.
        if self.images.textures.len() + self.images.requested.len() >= 600 {
            return;
        }
        self.images.requested.insert(key.clone());
        let tx = self.tx.clone();
        let to = self.settings.network_timeout_s;
        self.rt.spawn(async move {
            let res = fetch_image(&url, to).await;
            let _ = tx.send(AppEvent::Image(key, res.map_err(|e| e.to_string())));
        });
    }

    pub fn image(&self, key: &str) -> Option<&egui::TextureHandle> {
        self.images.textures.get(key)
    }

    // ---- favorites / progress ----

    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorites.contains(key)
    }

    pub fn toggle_favorite(&mut self, key: &str) {
        if self.favorites.contains(key) {
            self.favorites.remove(key);
        } else {
            self.favorites.insert(key.to_string());
        }
        let _ = store::save_favorites(&self.favorites);
    }

    pub fn get_progress(&self, key: &str) -> u64 {
        *self.progress.get(key).unwrap_or(&0)
    }

    pub fn set_progress(&mut self, key: &str, secs: u64) {
        self.progress.insert(key.to_string(), secs);
    }

    // ---- epg / series ----

    pub fn request_epg(&mut self, id: u32) {
        let is_xtream = self
            .active_profile
            .as_ref()
            .map(|p| p.kind == ProfileKind::Xtream)
            .unwrap_or(false);
        if !is_xtream || self.epg.contains_key(&id) || self.epg_requested.contains(&id) {
            return;
        }
        if self.epg_requested.len() >= 300 {
            return;
        }
        let p = match &self.active_profile {
            Some(p) => p.clone(),
            None => return,
        };
        self.epg_requested.insert(id);
        let tx = self.tx.clone();
        let to = self.settings.network_timeout_s;
        self.rt.spawn(async move {
            let r = client::xtream_short_epg(&p.host, &p.username, &p.password, id, to).await;
            let _ = tx.send(AppEvent::Epg(id, r.map_err(|e| e.to_string())));
        });
    }

    pub fn epg_for(&self, id: u32) -> Option<&Vec<EpgEntry>> {
        self.epg.get(&id)
    }

    /// True while an EPG request for this channel is still in flight.
    pub fn epg_pending(&self, id: u32) -> bool {
        self.epg_requested.contains(&id) && !self.epg.contains_key(&id)
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
        {
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
            let _ = tx.send(AppEvent::SeriesInfo(series_id, r.map_err(|e| e.to_string())));
        });
    }

    pub fn series_episodes_for(&self, series_id: u32) -> Option<&Vec<Episode>> {
        self.series_episodes.get(&series_id)
    }

    pub fn save_settings_now(&mut self) {
        let _ = store::save_settings(&self.settings);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        App::update(self, ctx, frame);
    }
}

async fn fetch_image(url: &str, timeout_s: u64) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    let c = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_s.max(5)))
        .build()?;
    let bytes = c.get(url).send().await?.error_for_status()?.bytes().await?;
    let img = image::load_from_memory(&bytes)?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Ok((rgba.into_raw(), w, h))
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
            let end = tail
                .find(|c: char| c == '&' || c == ' ' || c == ')' || c == '"')
                .unwrap_or(tail.len());
            out.push_str("***");
            rest = &tail[end..];
        }
        out.push_str(rest);
        out
    }
    mask(&mask(s, "password="), "username=")
}
