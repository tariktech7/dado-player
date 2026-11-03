use crate::types::{Profile, Settings};
use anyhow::Result;
use directories::ProjectDirs;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn app_dir() -> PathBuf {
    let dir = ProjectDirs::from("com", "Tarik", "Dado-Player")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    migrate_legacy(&dir);
    dir
}

/// The app was renamed twice. Copy any existing data forward once so saved
/// profiles, favorites and resume points are not lost.
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
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

fn write_json<T: serde::Serialize + ?Sized>(path: &PathBuf, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, data)?;
    Ok(())
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

pub fn load_settings() -> Settings {
    let mut dir = app_dir();
    dir.push("settings.json");
    let mut s: Settings = read_json(&dir).unwrap_or_default();
    // Older versions always stored an explicit "Dark". The theme now follows
    // Windows by default, so migrate once (the user can still pick Dunkel/Hell).
    if s.settings_version < 1 {
        s.theme = crate::types::Theme::System;
        s.settings_version = 1;
    }
    s
}

pub fn save_settings(s: &Settings) -> Result<()> {
    let mut dir = app_dir();
    dir.push("settings.json");
    write_json(&dir, s)
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
