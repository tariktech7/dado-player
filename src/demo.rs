//! Temporary local dataset used to check the layout without any provider
//! account. Only active when the env var IPTV_DEMO is set.

use crate::types::*;

const TEST: &str = "https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8";

pub fn library() -> Library {
    let mut lib = Library::default();

    lib.live_categories = vec![
        Category { id: "10".into(), name: "Nachrichten".into() },
        Category { id: "20".into(), name: "Sport".into() },
        Category { id: "30".into(), name: "Doku".into() },
        Category { id: "40".into(), name: "Kinder".into() },
    ];
    let names = [
        "Tagesschau 24", "Welt Nachrichten", "Sport Eins", "Bundesliga HD",
        "Discovery Channel", "National Geographic", "KiKA", "Nickelodeon",
        "Arte Dokumentation", "Phoenix", "ZDF Info", "RTL Aktuell",
    ];
    for (i, n) in names.iter().enumerate() {
        lib.channels.push(Channel {
            id: i as u32 + 1,
            num: i as u32 + 1,
            name: n.to_string(),
            logo: String::new(),
            category_id: lib.live_categories[i % 4].id.clone(),
            epg_channel_id: String::new(),
            stream_url: TEST.to_string(),
        });
    }

    lib.vod_categories = vec![
        Category { id: "100".into(), name: "Action".into() },
        Category { id: "200".into(), name: "Komödie".into() },
        Category { id: "300".into(), name: "Drama".into() },
    ];
    let movies = [
        "Der letzte Zug", "Nacht über Berlin", "Kaltes Wasser", "Roter Horizont",
        "Sieben Stunden", "Die Insel", "Sturmwarnung", "Letzte Chance",
        "Grenzland", "Silent City", "Der Architekt", "Hinter dem Nebel",
    ];
    for (i, n) in movies.iter().enumerate() {
        lib.movies.push(VodMovie {
            id: 1000 + i as u32,
            name: n.to_string(),
            cover: String::new(),
            category_id: lib.vod_categories[i % 3].id.clone(),
            rating: format!("{}.{}", 6 + i % 3, i % 9),
            container_extension: "mkv".into(),
            stream_url: TEST.to_string(),
            plot: String::new(),
        });
    }

    lib.series_categories = vec![
        Category { id: "500".into(), name: "Krimi".into() },
        Category { id: "600".into(), name: "Sci-Fi".into() },
    ];
    let shows = [
        "Tatort Nord", "Die Ermittler", "Space Colony", "Neon Nights",
        "Hafenkante", "Deep Signal", "Aktenzeichen 7", "Orbit",
    ];
    for (i, n) in shows.iter().enumerate() {
        lib.series.push(Series {
            id: 2000 + i as u32,
            name: n.to_string(),
            cover: String::new(),
            category_id: lib.series_categories[i % 2].id.clone(),
            plot: "Eine Demo-Serie, nur zum Prüfen des Layouts.".into(),
        });
    }

    lib.loaded = true;
    lib
}

/// Fake "recently watched" entries so the Continue view can be checked.
pub fn seed_history(lib: &Library) -> Vec<HistEntry> {
    let now = chrono::Utc::now().timestamp();
    let mut v = Vec::new();
    for (i, m) in lib.movies.iter().take(5).enumerate() {
        v.push(HistEntry {
            key: format!("vod:{}", m.id),
            ts: now - i as i64 * 3600,
            title: m.name.clone(),
            sub: "Film".into(),
        });
    }
    for (i, c) in lib.channels.iter().take(3).enumerate() {
        v.push(HistEntry {
            key: format!("live:{}", c.id),
            ts: now - i as i64 * 1800,
            title: c.name.clone(),
            sub: "Live-TV".into(),
        });
    }
    if let Some(s) = lib.series.first() {
        v.push(HistEntry {
            key: "ep:2000:9002".into(),
            ts: now - 600,
            title: s.name.clone(),
            sub: "Staffel 1 · Folge 2".into(),
        });
    }
    v
}
