use crate::types::{Profile, Settings};
use anyhow::Result;
#[cfg(not(test))]
use directories::ProjectDirs;
use std::collections::{HashMap, HashSet};
#[cfg(not(test))]
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set when the most recent write to disk failed, cleared by the next one that
/// succeeds. Read once per frame by the phone UI and shown as a banner.
///
/// It lives here rather than at the five `save_*` call sites because
/// `write_json` is the single choke point they all share — one place, every
/// file, and any future save inherits it. The alternative (a subtraction at ten
/// call sites) is how a silent failure stays silent.
static STORAGE_FAILED: AtomicBool = AtomicBool::new(false);

/// Did the last write fail? Call once per frame, not per write.
pub fn storage_failed() -> bool {
    STORAGE_FAILED.load(Ordering::Relaxed)
}

fn app_dir() -> PathBuf {
    #[cfg(not(test))]
    if std::env::var_os("IPTV_DEMO").is_some() {
        if let Some(path) = std::env::var_os("DADO_QA_DATA_DIR") {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                log_resolved_once(&path);
                return path;
            }
        }
    }
    #[cfg(test)]
    return std::env::temp_dir().join(format!(
        "dado-player-unit-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    #[cfg(not(test))]
    {
        let resolved =
            ProjectDirs::from("com", "Tarik", "Dado-Player").map(|d| d.config_dir().to_path_buf());
        let dir = resolved.clone().unwrap_or_else(|| {
            // A silent fallback to a directory that cannot be written is worse than
            // an error: everything downstream looks like it worked. Say it once.
            warn_fallback();
            PathBuf::from(".")
        });
        log_resolved_once(&dir);
        // The legacy project names are desktop-only. On Android there is no older
        // install to migrate from, and resolving three project dirs per call is
        // pointless work on a path that runs for every read and write.
        #[cfg(not(target_os = "android"))]
        migrate_legacy(&dir);
        dir
    }
}

pub fn license_path() -> PathBuf {
    app_dir().join("device-license.json")
}
static EPISODE_PREVIEWS: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn episode_preview_path(identity: &str) -> PathBuf {
    let hash = ring::digest::digest(&ring::digest::SHA256, identity.as_bytes());
    let name: String = hash
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    app_dir()
        .join("episode-previews")
        .join(format!("{name}.jpg"))
}

pub(crate) fn save_episode_preview(
    path: &std::path::Path,
    bytes: &[u8],
    cancel: &AtomicBool,
) -> std::io::Result<()> {
    let _lock = EPISODE_PREVIEWS.lock().unwrap();
    if cancel.load(Ordering::Acquire) || path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pending = path.with_extension("tmp");
    std::fs::write(&pending, bytes)?;
    std::fs::rename(pending, path)
}

pub(crate) fn clear_episode_previews() -> std::io::Result<()> {
    let _lock = EPISODE_PREVIEWS.lock().unwrap();
    match std::fs::remove_dir_all(app_dir().join("episode-previews")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

pub fn load_parental() -> crate::parental::Policy {
    let path = app_dir().join("parental.json");
    if !path.exists() {
        return Default::default();
    }
    // Damaged existing protection must not silently turn parental controls off.
    read_json(&path).unwrap_or_else(|| crate::parental::Policy {
        hash: vec![0; 32],
        blocked: ["*".to_owned()].into_iter().collect(),
        ..Default::default()
    })
}
pub fn save_parental(policy: &crate::parental::Policy) -> Result<()> {
    write_json(&app_dir().join("parental.json"), policy)
}
pub fn load_connect_state() -> serde_json::Value {
    read_json(&app_dir().join("connect-device.json")).unwrap_or(serde_json::Value::Null)
}
pub fn save_connect_state(value: &impl serde::Serialize) -> Result<()> {
    let path = app_dir().join("connect-device.json");
    write_json(&path, value)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Print the resolved data directory once, so a failed save can be diagnosed
/// from logcat instead of guessed at.
#[cfg(not(test))]
fn log_resolved_once(dir: &Path) {
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        eprintln!("dado: data dir = {}", dir.display());
    }
}

#[cfg(not(test))]
fn warn_fallback() {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "dado: no config directory could be resolved; falling back to the \
             process working directory. Nothing will persist unless HOME or \
             XDG_CONFIG_HOME is set (see jni_bridge::apply_storage_env)."
        );
    }
}

/// The app was renamed twice. Copy any existing data forward once so saved
/// profiles, favorites and resume points are not lost.
#[cfg(not(test))]
fn migrate_legacy(new_dir: &PathBuf) {
    const FILES: [&str; 4] = [
        "profiles.json",
        "settings.json",
        "favorites.json",
        "progress.json",
    ];
    if new_dir.join("profiles.json").exists() {
        return;
    }
    let candidates = [
        ProjectDirs::from("com", "Tarik", "Tarik-IPTV").map(|d| d.config_dir().to_path_buf()),
        ProjectDirs::from("com", "iptvapp", "iptvapp").map(|d| d.config_dir().join("iptv-app")),
    ];
    for legacy in candidates.into_iter().flatten() {
        if !legacy.join("profiles.json").exists() {
            continue;
        }
        if std::fs::create_dir_all(new_dir).is_err() {
            return;
        }
        for f in FILES {
            let src = legacy.join(f);
            if src.exists() {
                let _ = std::fs::copy(&src, new_dir.join(f));
            }
        }
        return;
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    // A missing file is the normal first-start case. A *corrupt* file is not:
    // silently falling back to defaults used to look like "all my profiles are
    // gone" with no clue in the log. Say so once, without echoing file contents
    // (profiles.json contains plaintext credentials).
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!(
                "dado: {} contains invalid JSON; starting with defaults for this file",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            None
        }
    }
}

fn write_json<T: serde::Serialize + ?Sized>(path: &PathBuf, value: &T) -> Result<()> {
    let r = write_json_inner(path, value);
    // Every `save_*` in this module routes through here, so this one line makes
    // a failure anywhere visible and gives any future save the same behaviour
    // for free. A successful write clears it: the banner describes the last
    // attempt, not a permanent state.
    STORAGE_FAILED.store(r.is_err(), Ordering::Relaxed);
    r
}

fn write_json_inner<T: serde::Serialize + ?Sized>(path: &PathBuf, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec_pretty(value)?;
    // Write-then-rename: a crash, a killed process or a full disk must never
    // leave a half-written profiles.json. `fs::rename` replaces the target on
    // Windows as well (MoveFileEx with MOVEFILE_REPLACE_EXISTING), so upgrades
    // overwrite the previous file atomically.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e.into())
        }
    }
}

pub fn load_profiles() -> Vec<Profile> {
    let mut dir = app_dir();
    dir.push("profiles.json");
    read_json(&dir).unwrap_or_default()
}

pub fn save_profiles(profiles: &[Profile]) -> Result<()> {
    let mut dir = app_dir();
    dir.push("profiles.json");
    write_json(&dir, profiles)
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct StoredSettings {
    #[serde(flatten)]
    settings: Settings,
    #[serde(default)]
    live_format_automatic: bool,
}

pub fn load_settings() -> Settings {
    let mut dir = app_dir();
    dir.push("settings.json");
    let stored: StoredSettings = read_json(&dir).unwrap_or_default();
    let mut s = stored.settings;
    if stored.live_format_automatic {
        s.live_format = crate::types::LiveFormat::Automatic;
    }
    // Older versions always stored an explicit "Dark". The theme now follows
    // Windows by default, so migrate once (the user can still pick Dunkel/Hell).
    if s.settings_version < 1 {
        s.theme = crate::types::Theme::System;
        s.settings_version = 1;
    }
    // v2: MX Player became the default. v1 wrote `player_backend` to disk on
    // every save even when nobody touched the setting, and its default was
    // `Mpv`. `#[serde(default)]` only helps files that are MISSING the key,
    // not files that contain `"player_backend":"Mpv"`, so a one-time switch is
    // needed. Users who deliberately chose the internal player can re-select
    // it in Einstellungen → Player; the bumped version means this runs once.
    if s.settings_version < 2 {
        if s.player_backend == crate::types::PlayerBackend::Mpv {
            s.player_backend = crate::types::PlayerBackend::MxPlayer;
        }
        s.settings_version = 2;
    }
    #[cfg(target_os = "ios")]
    if s.settings_version < 4 {
        s.live_format = crate::types::LiveFormat::Hls;
        s.settings_version = 4;
    }
    if s.settings_version < 5 {
        if s.settings_mode == crate::types::SettingsMode::Easy {
            s.automatic_playback();
        }
        s.settings_version = 5;
    }
    if s.settings_version < 6 {
        if s.theme == crate::types::Theme::System {
            s.theme = crate::types::Theme::Warm;
        }
        if s.player_backend == crate::types::PlayerBackend::MxPlayerIntegrated {
            s.player_backend = crate::types::PlayerBackend::Mpv;
        }
        if s.player_backend == crate::types::PlayerBackend::MxPlayerFallback {
            s.player_backend = crate::types::PlayerBackend::MxPlayer;
        }
        s.settings_version = 6;
    }
    s
}

pub fn save_settings(s: &Settings) -> Result<()> {
    use crate::types::LiveFormat;
    let mut dir = app_dir();
    dir.push("settings.json");
    let mut settings = s.clone();
    let live_format_automatic = s.live_format == LiveFormat::Automatic;
    if live_format_automatic {
        settings.live_format = if s.live_format.extension() == "m3u8" {
            LiveFormat::Hls
        } else {
            LiveFormat::Ts
        };
    }
    write_json(
        &dir,
        &StoredSettings {
            settings,
            live_format_automatic,
        },
    )
}

pub fn load_favorites() -> HashSet<String> {
    let mut dir = app_dir();
    dir.push("favorites.json");
    read_json(&dir).unwrap_or_default()
}

pub fn save_favorites(favs: &HashSet<String>) -> Result<()> {
    let mut dir = app_dir();
    dir.push("favorites.json");
    write_json(&dir, favs)
}

pub fn load_progress() -> HashMap<String, u64> {
    let mut dir = app_dir();
    dir.push("progress.json");
    read_json(&dir).unwrap_or_default()
}

pub fn save_progress(p: &HashMap<String, u64>) -> Result<()> {
    let mut dir = app_dir();
    dir.push("progress.json");
    write_json(&dir, p)
}

pub fn load_durations() -> HashMap<String, u64> {
    read_json(&app_dir().join("durations.json")).unwrap_or_default()
}
pub fn save_durations(values: &HashMap<String, u64>) -> Result<()> {
    write_json(&app_dir().join("durations.json"), values)
}

pub fn load_history() -> Vec<crate::types::HistEntry> {
    let mut dir = app_dir();
    dir.push("history.json");
    read_json(&dir).unwrap_or_default()
}

pub fn save_history(h: &[crate::types::HistEntry]) -> Result<()> {
    let mut dir = app_dir();
    dir.push("history.json");
    write_json(&dir, h)
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct ProfileData {
    pub favorites: HashSet<String>,
    pub progress: HashMap<String, u64>,
    pub durations: HashMap<String, u64>,
    pub history: Vec<crate::types::HistEntry>,
}
fn profile_data_path(id: &str) -> PathBuf {
    let digest = ring::digest::digest(&ring::digest::SHA256, id.as_bytes());
    let name: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
    app_dir().join("profile-data").join(format!("{name}.json"))
}
pub fn load_profile_data(id: &str, legacy_owner_hint: Option<&str>) -> Result<ProfileData> {
    let path = profile_data_path(id);
    if path.exists() {
        if let Some(data) = read_json(&path) {
            return Ok(data);
        }
        // Preserve unreadable bytes before any later user action can save again.
        let backup = path.with_extension(format!(
            "unreadable-{}",
            chrono::Utc::now().timestamp_millis()
        ));
        let _ = std::fs::copy(&path, backup);
        anyhow::bail!("Profildaten konnten nicht gelesen werden");
    }
    let owner_path = app_dir().join("legacy-profile-owner.json");
    let owner: String =
        read_json(&owner_path).unwrap_or_else(|| legacy_owner_hint.unwrap_or(id).to_string());
    if !owner_path.exists() {
        write_json(&owner_path, &owner)?;
    }
    let data = if owner == id {
        ProfileData {
            favorites: load_favorites(),
            progress: load_progress(),
            durations: load_durations(),
            history: load_history(),
        }
    } else {
        ProfileData::default()
    };
    save_profile_data(
        id,
        &data.favorites,
        &data.progress,
        &data.durations,
        &data.history,
    )?;
    Ok(data)
}
pub fn save_profile_data(
    id: &str,
    favorites: &HashSet<String>,
    progress: &HashMap<String, u64>,
    durations: &HashMap<String, u64>,
    history: &[crate::types::HistEntry],
) -> Result<()> {
    #[derive(serde::Serialize)]
    struct Snapshot<'a> {
        favorites: &'a HashSet<String>,
        progress: &'a HashMap<String, u64>,
        durations: &'a HashMap<String, u64>,
        history: &'a [crate::types::HistEntry],
    }
    write_json(
        &profile_data_path(id),
        &Snapshot {
            favorites,
            progress,
            durations,
            history,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn automatic_stream_format_survives_save_without_changing_the_legacy_extension() {
        use crate::types::LiveFormat;
        let settings = Settings::default();
        save_settings(&settings).unwrap();
        assert_eq!(load_settings().live_format, LiveFormat::Automatic);
        let legacy: Settings = read_json(&app_dir().join("settings.json")).unwrap();
        assert_ne!(legacy.live_format, LiveFormat::Automatic);
        assert_eq!(
            legacy.live_format.extension(),
            settings.live_format.extension()
        );
    }

    #[test]
    fn profile_data_migrates_legacy_once_and_never_leaks_matching_provider_ids() {
        let favorites = HashSet::from(["live:1".to_string()]);
        save_favorites(&favorites).unwrap();
        save_progress(&HashMap::from([("vod:1".into(), 77)])).unwrap();
        let legacy = std::fs::read(app_dir().join("favorites.json")).unwrap();
        let first = load_profile_data("first-provider", Some("first-provider")).unwrap();
        assert!(first.favorites.contains("live:1"));
        assert_eq!(first.progress.get("vod:1"), Some(&77));
        let second = load_profile_data("second-provider", Some("second-provider")).unwrap();
        assert!(second.favorites.is_empty());
        assert!(second.progress.is_empty());
        save_profile_data(
            "second-provider",
            &HashSet::from(["vod:1".into()]),
            &HashMap::from([("vod:1".into(), 999)]),
            &HashMap::new(),
            &[],
        )
        .unwrap();
        assert_eq!(
            load_profile_data("first-provider", None)
                .unwrap()
                .progress
                .get("vod:1"),
            Some(&77)
        );
        assert_eq!(
            load_profile_data("second-provider", None)
                .unwrap()
                .progress
                .get("vod:1"),
            Some(&999)
        );
        assert_eq!(
            std::fs::read(app_dir().join("favorites.json")).unwrap(),
            legacy
        );
    }

    fn temp_file(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dado-store-{}-{}-{}.json",
            std::process::id(),
            label,
            nanos
        ))
    }

    #[test]
    fn missing_file_is_a_normal_first_start() {
        let path = temp_file("missing");
        let _ = std::fs::remove_file(&path);
        let parsed: Option<Vec<String>> = read_json(&path);
        assert!(parsed.is_none());
    }

    #[test]
    fn corrupted_json_falls_back_without_panicking() {
        let path = temp_file("corrupt");
        std::fs::write(&path, b"{\"profiles\": [ this is not json").unwrap();
        let parsed: Option<Vec<String>> = read_json(&path);
        assert!(parsed.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writes_are_atomic_and_can_overwrite_an_existing_file() {
        let path = temp_file("roundtrip");
        let tmp = path.with_extension("json.tmp");

        write_json(&path, &vec!["a".to_string(), "b".to_string()]).unwrap();
        let loaded: Vec<String> = read_json(&path).unwrap();
        assert_eq!(loaded, vec!["a".to_string(), "b".to_string()]);
        assert!(!tmp.exists(), "temporary file must be renamed away");

        write_json(&path, &vec!["c".to_string()]).unwrap();
        let loaded: Vec<String> = read_json(&path).unwrap();
        assert_eq!(loaded, vec!["c".to_string()]);
        assert!(!tmp.exists());

        let _ = std::fs::remove_file(&path);
    }
}
