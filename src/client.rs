use crate::types::*;
use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Deserializer};
use std::time::{Duration, Instant};

fn http_client(timeout_s: u64) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(timeout_s.max(5)))
        .gzip(true)
        .pool_max_idle_per_host(8)
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Ensure host has a scheme and no trailing slash.
pub fn normalize_host(host: &str) -> Result<String> {
    let h = host.trim();
    if h.is_empty() {
        return Err(anyhow!("Host ist leer"));
    }
    let mut s = if h.contains("://") {
        h.to_string()
    } else {
        format!("http://{}", h)
    };
    while s.ends_with('/') {
        s.pop();
    }
    Ok(s)
}

// ---------- Xtream ----------

// Xtream panels are loosely typed: strings may be null, numbers may arrive as
// strings and vice versa. These helpers keep deserialization tolerant.
fn as_string<'de, D>(d: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    })
}

fn as_u32<'de, D>(d: D) -> std::result::Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
        serde_json::Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    })
}

fn as_i64<'de, D>(d: D) -> std::result::Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        serde_json::Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    })
}

fn as_i64_opt<'de, D>(d: D) -> std::result::Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

/// Convert transport failures into user-facing diagnostics without echoing the
/// request URL (which contains provider credentials in its query string).
fn network_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "Zeitüberschreitung. Bitte Verbindung und Timeout-Einstellung prüfen.".to_string()
    } else if error.is_connect() {
        "Server nicht erreichbar. Host, Port, DNS und Internetverbindung prüfen.".to_string()
    } else {
        "Netzwerkfehler beim Anbieter-Aufruf.".to_string()
    }
}

/// Measure one catalogue stage. The report contains no URL, user or password —
/// only a stage name, a row count and a duration — so `DADO_DEBUG=1` logs are
/// safe to attach to a bug report.
async fn timed<T>(future: impl std::future::Future<Output = Result<T>>) -> (Result<T>, u64) {
    let started = Instant::now();
    let result = future.await;
    (
        result,
        started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    )
}

/// Turn a failed catalogue endpoint into a partial result plus a diagnostic.
///
/// This is the fix for the "first login only shows Live-TV" class of bugs: a
/// single slow or malformed VOD/Series endpoint used to abort the whole login
/// via `?`, so the user saw an error (or a half-empty library) and had to log
/// out and back in until all six requests happened to succeed. Every area is
/// now independent; a failed one is reported instead of wiping the others.
fn collect_area<T: Default>(
    (result, elapsed_ms): (Result<T>, u64),
    area: CatalogArea,
    issues: &mut Vec<CatalogIssue>,
) -> T {
    match result {
        Ok(value) => {
            log::debug!("catalog[{}] ok in {}ms", area.label(), elapsed_ms);
            value
        }
        Err(e) => {
            let message = e.to_string();
            log::debug!(
                "catalog[{}] failed in {}ms: {}",
                area.label(),
                elapsed_ms,
                message
            );
            issues.push(CatalogIssue {
                area,
                message,
                elapsed_ms,
            });
            T::default()
        }
    }
}

async fn api_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    stage: &str,
) -> Result<T> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| {
            anyhow!(
                "{} konnte nicht geladen werden: {}",
                stage,
                network_error(&e)
            )
        })?
        .error_for_status()
        .map_err(|e| {
            anyhow!(
                "{} fehlgeschlagen: HTTP {}",
                stage,
                e.status().map(|s| s.as_u16()).unwrap_or(0)
            )
        })?;
    response.json::<T>().await.map_err(|_| {
        anyhow!(
            "{}: Antwort des Anbieters hat ein unerwartetes Format.",
            stage
        )
    })
}

/// Sanitise a container extension coming from a provider.
///
/// Panels send `"mp4"`, `".mkv"`, `"MKV"`, `null` or an empty string. Building
/// `/movie/u/p/42.` (or worse, `/series/.../42.m3u8` for an `.mkv` episode)
/// is the single most common reason a title is listed but cannot be played.
/// Anything that is not a short alphanumeric suffix falls back to `fallback`.
pub fn stream_ext(raw: &str, fallback: &str) -> String {
    let clean = raw.trim().trim_start_matches('.').to_ascii_lowercase();
    let ok =
        !clean.is_empty() && clean.len() <= 8 && clean.bytes().all(|b| b.is_ascii_alphanumeric());
    if ok {
        clean
    } else {
        fallback.trim().trim_start_matches('.').to_ascii_lowercase()
    }
}

/// Build one Xtream live URL. Shared by the library loader and the settings
/// page so changing "Live-Format" takes effect without a new login.
pub fn live_stream_url(stream_base: &str, user: &str, pass: &str, id: u32, ext: &str) -> String {
    format!(
        "{}/live/{}/{}/{}.{}",
        stream_base.trim_end_matches('/'),
        urlencode(user),
        urlencode(pass),
        id,
        stream_ext(ext, "ts")
    )
}

/// Xtream panels usually Base64-encode EPG titles/descriptions.
fn decode_epg(s: &str) -> String {
    let t = s.trim();
    let looks_b64 = !t.is_empty()
        && t.len().is_multiple_of(4)
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=');
    if looks_b64 {
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(t) {
            if let Ok(txt) = String::from_utf8(bytes) {
                return txt;
            }
        }
    }
    s.to_string()
}

#[derive(Deserialize)]
struct AuthResp {
    #[serde(default)]
    user_info: Option<AuthUser>,
    #[serde(default)]
    server_info: Option<ServerInfo>,
}
#[derive(Deserialize)]
struct AuthUser {
    #[serde(default, deserialize_with = "as_i64")]
    auth: i64,
}
#[derive(Deserialize)]
struct ServerInfo {
    #[serde(default, deserialize_with = "as_string")]
    url: String,
    #[serde(default, deserialize_with = "as_string")]
    https_port: String,
}

/// Returns the preferred base URL for building stream URLs.
/// Prefers the provider's https_port (many panels reject live streams over HTTP).
pub async fn xtream_login(host: &str, user: &str, pass: &str, timeout_s: u64) -> Result<String> {
    let base = normalize_host(host)?;
    let url = format!(
        "{}/player_api.php?username={}&password={}",
        base,
        urlencode(user),
        urlencode(pass)
    );
    let c = http_client(timeout_s);
    let resp = c
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            anyhow!(
                "Verbindung zum Anbieter fehlgeschlagen: {}",
                network_error(&e)
            )
        })?
        .error_for_status()
        .map_err(|e| {
            anyhow!(
                "Anbieter-Login abgelehnt: HTTP {}",
                e.status().map(|s| s.as_u16()).unwrap_or(0)
            )
        })?;
    let v: AuthResp = resp
        .json()
        .await
        .map_err(|_| anyhow!("Anbieter antwortet nicht im erwarteten Xtream-Format."))?;
    let auth = v.user_info.map(|u| u.auth).unwrap_or(0);
    if auth != 1 {
        return Err(anyhow!("Anmeldung fehlgeschlagen (auth={})", auth));
    }
    let mut stream_base = base.clone();
    if let Some(si) = v.server_info.filter(|_| !cfg!(target_os = "ios")) {
        // Panels usually serve the API on plain HTTP but streams on a second,
        // TLS-only port. `server_protocol` says "http" even then (it describes
        // the API), so it must NOT be used to reject `https_port`. The stream
        // port commonly has a self-signed certificate - libmpv handles that
        // via `tls-verify=no` (see Settings::tls_verify / mpv::apply_tls_policy).
        let host_part = si.url.trim().trim_end_matches('/');
        let host_part = host_part
            .strip_prefix("https://")
            .or_else(|| host_part.strip_prefix("http://"))
            .unwrap_or(host_part);
        let https_port = si.https_port.trim();
        if !host_part.is_empty() && https_port.parse::<u16>().is_ok() && https_port != "0" {
            stream_base = format!("https://{}:{}", host_part, https_port);
        }
    }
    Ok(stream_base)
}

#[derive(Deserialize)]
struct CatX {
    #[serde(default, deserialize_with = "as_string")]
    category_id: String,
    #[serde(default, deserialize_with = "as_string")]
    category_name: String,
}
#[derive(Deserialize)]
struct ChanX {
    #[serde(default, deserialize_with = "as_u32")]
    num: u32,
    #[serde(default, deserialize_with = "as_string")]
    name: String,
    #[serde(default, deserialize_with = "as_u32")]
    stream_id: u32,
    #[serde(default, deserialize_with = "as_string")]
    stream_icon: String,
    #[serde(default, deserialize_with = "as_string")]
    category_id: String,
    #[serde(default, deserialize_with = "as_string")]
    epg_channel_id: String,
    /// A few panels expose the live container here. Live streams normally have
    /// no such field, which is why the URL format is a user setting.
    #[serde(default, deserialize_with = "as_string")]
    container_extension: String,
    #[serde(default, deserialize_with = "as_i64_opt")]
    added: Option<i64>,
}
#[derive(Deserialize)]
struct MovieX {
    #[serde(default, deserialize_with = "as_string")]
    name: String,
    #[serde(default, deserialize_with = "as_u32")]
    stream_id: u32,
    #[serde(default, deserialize_with = "as_string")]
    category_id: String,
    #[serde(default, deserialize_with = "as_string")]
    cover: String,
    #[serde(default, deserialize_with = "as_string")]
    stream_icon: String,
    #[serde(default, deserialize_with = "as_string")]
    rating: String,
    #[serde(default, deserialize_with = "as_string")]
    container_extension: String,
    #[serde(default, deserialize_with = "as_string")]
    plot: String,
    #[serde(default, deserialize_with = "as_i64_opt")]
    added: Option<i64>,
}
#[derive(Deserialize)]
struct SeriesX {
    #[serde(default, deserialize_with = "as_string")]
    name: String,
    #[serde(default, deserialize_with = "as_u32")]
    series_id: u32,
    #[serde(default, deserialize_with = "as_string")]
    category_id: String,
    #[serde(default, deserialize_with = "as_string")]
    cover: String,
    #[serde(default, deserialize_with = "as_string")]
    plot: String,
    #[serde(default, deserialize_with = "as_i64_opt")]
    added: Option<i64>,
}

// The six catalogue areas are assembled step by step so that each failed
// endpoint can be recorded independently; a single struct literal would force
// all-or-nothing construction again.
#[allow(clippy::field_reassign_with_default)]
pub async fn xtream_load_library(
    host: &str,
    stream_base: &str,
    user: &str,
    pass: &str,
    timeout_s: u64,
    live_ext: &str,
) -> Result<Library> {
    let base = normalize_host(host)?;
    let sb = if stream_base.trim().is_empty() {
        base.clone()
    } else {
        stream_base.trim().to_string()
    };
    let c = http_client(timeout_s);
    let api = |action: &str| {
        format!(
            "{}/player_api.php?username={}&password={}&action={}",
            base,
            urlencode(user),
            urlencode(pass),
            action
        )
    };

    // Two waves of three concurrent requests. Six fully parallel calls can
    // trip the connection limit of cheap Xtream panels; six strictly serial
    // calls can burn six timeouts on a slow provider and make the very first
    // login look like it "only loaded Live-TV". Three at a time keeps the
    // first login short without hammering the panel.
    // Bind the URLs: `api()` returns an owned String, and a temporary inside
    // `join!` would be dropped before the future is polled.
    let url_live_cat = api("get_live_categories");
    let url_vod_cat = api("get_vod_categories");
    let url_ser_cat = api("get_series_categories");
    let url_live_streams = api("get_live_streams");
    let url_vod_streams = api("get_vod_streams");
    let url_series = api("get_series");

    let (live_cat_r, vod_cat_r, ser_cat_r) = tokio::join!(
        timed(api_json::<Vec<CatX>>(&c, &url_live_cat, "Live-Kategorien")),
        timed(api_json::<Vec<CatX>>(&c, &url_vod_cat, "Film-Kategorien")),
        timed(api_json::<Vec<CatX>>(&c, &url_ser_cat, "Serien-Kategorien")),
    );
    let (channels_r, movies_r, series_r) = tokio::join!(
        timed(api_json::<Vec<ChanX>>(&c, &url_live_streams, "Senderliste")),
        timed(api_json::<Vec<MovieX>>(&c, &url_vod_streams, "Filmliste")),
        timed(api_json::<Vec<SeriesX>>(&c, &url_series, "Serienliste")),
    );

    // Every endpoint is independent. A timeout or malformed reply used to
    // abort the whole login with `?`, so the user had to log out and back in
    // until all six happened to succeed. Now the working areas are kept and
    // the failed ones are reported in the UI with a one-tap refresh.
    let mut issues = Vec::new();
    let live_cat = collect_area(live_cat_r, CatalogArea::LiveCategories, &mut issues);
    let vod_cat = collect_area(vod_cat_r, CatalogArea::MovieCategories, &mut issues);
    let ser_cat = collect_area(ser_cat_r, CatalogArea::SeriesCategories, &mut issues);
    let channels_x = collect_area(channels_r, CatalogArea::LiveStreams, &mut issues);
    let movies_x = collect_area(movies_r, CatalogArea::Movies, &mut issues);
    let series_x = collect_area(series_r, CatalogArea::Series, &mut issues);

    let mut lib = Library::default();
    lib.live_categories = live_cat
        .into_iter()
        .map(|c| Category {
            id: c.category_id,
            name: c.category_name,
        })
        .collect();
    lib.vod_categories = vod_cat
        .into_iter()
        .map(|c| Category {
            id: c.category_id,
            name: c.category_name,
        })
        .collect();
    lib.series_categories = ser_cat
        .into_iter()
        .map(|c| Category {
            id: c.category_id,
            name: c.category_name,
        })
        .collect();

    lib.channels = channels_x
        .into_iter()
        .map(|x| {
            // A panel that explicitly says "m3u8"/"ts" wins over the global
            // setting; otherwise use the configured live format. Both are real
            // Xtream endpoints, but only one of them is enabled on many panels.
            let ext = stream_ext(&x.container_extension, live_ext);
            Channel {
                id: x.stream_id,
                num: x.num,
                added: x.added,
                name: x.name,
                logo: x.stream_icon,
                category_id: x.category_id,
                epg_channel_id: x.epg_channel_id,
                stream_url: format!(
                    "{}/live/{}/{}/{}.{}",
                    sb,
                    urlencode(user),
                    urlencode(pass),
                    x.stream_id,
                    ext
                ),
                // Xtream's live list has no radio flag; M3U is the only source.
                radio: false,
            }
        })
        .collect();

    lib.movies = movies_x
        .into_iter()
        .map(|x| VodMovie {
            id: x.stream_id,
            name: x.name,
            cover: if !x.cover.is_empty() {
                x.cover
            } else {
                x.stream_icon
            },
            category_id: x.category_id,
            rating: x.rating,
            added: x.added,
            container_extension: stream_ext(&x.container_extension, "mp4"),
            stream_url: format!(
                "{}/movie/{}/{}/{}.{}",
                sb,
                urlencode(user),
                urlencode(pass),
                x.stream_id,
                stream_ext(&x.container_extension, "mp4")
            ),
            plot: x.plot,
        })
        .collect();

    lib.series = series_x
        .into_iter()
        .map(|x| Series {
            id: x.series_id,
            name: x.name,
            added: x.added,
            cover: x.cover,
            category_id: x.category_id,
            plot: x.plot,
        })
        .collect();

    lib.stream_base = sb;
    lib.catalog_issues = issues;
    // Authentication already succeeded in `xtream_login` before this call. Even
    // if every endpoint failed, show the (empty) library with an explicit
    // error + refresh instead of trapping the user on the login screen.
    lib.loaded = true;
    log::debug!(
        "catalog ready: live={} movies={} series={} failed_areas={}",
        lib.channels.len(),
        lib.movies.len(),
        lib.series.len(),
        lib.catalog_issues.len()
    );
    Ok(lib)
}

#[derive(Deserialize)]
struct SeriesInfo {
    #[serde(default)]
    episodes: std::collections::HashMap<String, Vec<EpInfo>>,
}
#[derive(Deserialize)]
struct EpInfo {
    #[serde(default, deserialize_with = "as_u32")]
    id: u32,
    #[serde(default, deserialize_with = "as_string")]
    title: String,
    /// Some panels repeat the season here. The `episodes` map key is the
    /// authoritative value; this is only a fallback.
    #[serde(default, deserialize_with = "as_string")]
    season: String,
    /// Xtream calls the episode number `episode_num`, not `episode`. Reading
    /// only `episode` made every episode "Folge 0" and sorted them all into
    /// one bucket on real panels.
    #[serde(default, deserialize_with = "as_string")]
    episode_num: String,
    /// A few panels use the shorter name.
    #[serde(default, deserialize_with = "as_string")]
    episode: String,
    #[serde(default, deserialize_with = "as_string")]
    container_extension: String,
}

/// First integer anywhere in a string, used only as a last resort for the
/// episode number when a panel omits both numeric fields.
fn first_number(s: &str) -> u32 {
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if !digits.is_empty() {
            break;
        }
    }
    digits.parse().unwrap_or(0)
}

pub async fn xtream_series_info(
    host: &str,
    stream_base: &str,
    user: &str,
    pass: &str,
    series_id: u32,
    timeout_s: u64,
) -> Result<Vec<Episode>> {
    let base = normalize_host(host)?;
    let sb = if stream_base.trim().is_empty() {
        base.clone()
    } else {
        stream_base.trim().to_string()
    };
    let url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series_info&series_id={}",
        base,
        urlencode(user),
        urlencode(pass),
        series_id
    );
    let c = http_client(timeout_s);
    // Same credential-safe error path as the library loader: a bare `?` here
    // used to bubble a reqwest error whose Display contains the full URL,
    // including username and password, into the UI.
    let info: SeriesInfo = api_json(&c, &url, "Serien-Episoden").await?;
    Ok(episodes_from_info(&info, series_id, &sb, user, pass))
}

/// Pure conversion so the Xtream `get_series_info` shape can be tested without
/// a network call. Kept separate from the async loader for exactly that reason.
fn episodes_from_info(
    info: &SeriesInfo,
    series_id: u32,
    sb: &str,
    user: &str,
    pass: &str,
) -> Vec<Episode> {
    let mut out = Vec::new();
    for (season_key, eps) in &info.episodes {
        let key_season = season_key.trim().parse::<u32>().ok();
        for e in eps {
            let season = key_season
                .or_else(|| e.season.trim().parse::<u32>().ok())
                .unwrap_or(0);
            let episode = e
                .episode_num
                .trim()
                .parse::<u32>()
                .ok()
                .or_else(|| e.episode.trim().parse::<u32>().ok())
                .unwrap_or_else(|| first_number(&e.title));
            // Episodes are files, not HLS playlists. Hardcoding `.m3u8` here
            // was the reason "Filme gehen, Serien nicht" — the same stream
            // base works, but the extension did not exist on the panel.
            let ext = stream_ext(&e.container_extension, "mp4");
            out.push(Episode {
                id: e.id,
                series_id,
                season,
                episode,
                title: e.title.clone(),
                container_extension: ext.clone(),
                stream_url: format!(
                    "{}/series/{}/{}/{}.{}",
                    sb.trim_end_matches('/'),
                    urlencode(user),
                    urlencode(pass),
                    e.id,
                    ext
                ),
            });
        }
    }
    out.sort_by_key(|a| (a.season, a.episode, a.id));
    out
}

#[derive(Deserialize)]
struct EpgResp {
    #[serde(default)]
    epg_listings: Vec<EpgX>,
}
#[derive(Deserialize)]
struct EpgX {
    #[serde(default, deserialize_with = "as_string")]
    title: String,
    #[serde(default, deserialize_with = "as_string")]
    description: String,
    #[serde(default, deserialize_with = "as_string")]
    start_timestamp: String,
    #[serde(default, deserialize_with = "as_string")]
    stop_timestamp: String,
}

async fn xtream_epg_action(
    host: &str,
    user: &str,
    pass: &str,
    action: &str,
    stream_id: u32,
    timeout_s: u64,
) -> Result<Vec<EpgEntry>> {
    let base = normalize_host(host)?;
    let url = format!(
        "{}/player_api.php?username={}&password={}&action={}&stream_id={}",
        base,
        urlencode(user),
        urlencode(pass),
        action,
        stream_id
    );
    // Never propagate `reqwest::Error` directly here: its Display includes the
    // full request URL, which carries `username=` and `password=` in the query.
    // The same rule as `api_json` applies — diagnose by kind, never echo the URL.
    let response = http_client(timeout_s)
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("EPG konnte nicht geladen werden: {}", network_error(&e)))?
        .error_for_status()
        .map_err(|e| {
            anyhow!(
                "EPG fehlgeschlagen: HTTP {}",
                e.status().map(|s| s.as_u16()).unwrap_or(0)
            )
        })?;
    let resp: EpgResp = response
        .json()
        .await
        .map_err(|_| anyhow!("EPG: Antwort des Anbieters hat ein unerwartetes Format."))?;
    Ok(resp
        .epg_listings
        .into_iter()
        .filter_map(|e| {
            let start = e.start_timestamp.parse::<i64>().ok()?;
            let end = e.stop_timestamp.parse::<i64>().ok()?;
            (start > 0 && end > start).then(|| EpgEntry {
                title: decode_epg(&e.title),
                description: decode_epg(&e.description),
                start,
                end,
            })
        })
        .collect())
}

pub async fn xtream_short_epg(
    host: &str,
    user: &str,
    pass: &str,
    stream_id: u32,
    timeout_s: u64,
) -> Result<Vec<EpgEntry>> {
    xtream_epg_action(host, user, pass, "get_short_epg", stream_id, timeout_s).await
}

/// Full schedule where the Xtream portal implements the commonly available
/// get_simple_data_table action. Portals without it fall back to short EPG.
pub async fn xtream_full_epg(
    host: &str,
    user: &str,
    pass: &str,
    stream_id: u32,
    timeout_s: u64,
) -> Result<Vec<EpgEntry>> {
    xtream_epg_action(
        host,
        user,
        pass,
        "get_simple_data_table",
        stream_id,
        timeout_s,
    )
    .await
}

// ---------- M3U ----------

pub async fn m3u_load(url: &str, file: &str, timeout_s: u64) -> Result<Library> {
    let content = if !file.is_empty() {
        std::fs::read_to_string(file)?
    } else {
        // Users paste M3U links without a scheme ("provider.eu/get.php?…").
        // Without this, reqwest fails with "relative URL without a base" and
        // the login screen just shows an error — it looks like the link is
        // broken. Also surface HTTP 404/401 pages instead of quietly parsing
        // their HTML into an empty library.
        let url = normalize_host(url)?;
        let c = http_client(timeout_s);
        let response = c
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("M3U-Playlist nicht erreichbar: {}", network_error(&e)))?
            .error_for_status()
            .map_err(|e| {
                anyhow!(
                    "M3U-Server antwortet mit HTTP {}.",
                    e.status().map(|s| s.as_u16()).unwrap_or(0)
                )
            })?;
        response
            .text()
            .await
            .map_err(|_| anyhow!("M3U-Antwort konnte nicht gelesen werden."))?
    };
    let lib = parse_m3u(&content);
    if lib.channels.is_empty() {
        return Err(anyhow!(
            "Playlist enthält keine Sender. Bitte Link oder Datei prüfen."
        ));
    }
    Ok(lib)
}

pub fn parse_m3u(content: &str) -> Library {
    let mut lib = Library::default();
    let mut name: Option<String> = None;
    let mut logo: Option<String> = None;
    let mut group: Option<String> = None;
    let mut radio = false;
    let mut expecting_url = false;
    let mut id = 0u32;

    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("#EXTINF") {
            let (n, l, g, r) = parse_extinf(line);
            name = Some(n);
            logo = l;
            group = g;
            radio = r;
            expecting_url = true;
        } else if line.starts_with('#') {
            // comment / directive -> reset expectation if it was a different tag
            expecting_url = false;
        } else if !line.is_empty() {
            // Two legal M3U shapes: "#EXTINF …" followed by the stream URL, and
            // a bare list of stream URLs (a plain .m3u with no metadata at all).
            // The old parser silently dropped the second shape, so pasting such
            // a link looked like "nothing happens": the app logged in with an
            // empty library and no error. Only accept something URL-shaped when
            // there was no EXTINF, so an HTML error page is not turned into a
            // list of junk channels.
            if !expecting_url && !looks_like_stream_url(line) {
                continue;
            }
            let g = group.take().unwrap_or_else(|| "Channels".to_string());
            id += 1;
            if !lib.live_categories.iter().any(|c| c.id == g) {
                lib.live_categories.push(Category {
                    id: g.clone(),
                    name: g.clone(),
                });
            }
            let display = name.take().unwrap_or_default();
            lib.channels.push(Channel {
                id,
                num: id,
                added: None,
                name: if display.is_empty() {
                    bare_channel_name(line)
                } else {
                    display
                },
                logo: logo.take().unwrap_or_default(),
                category_id: g,
                epg_channel_id: String::new(),
                stream_url: line.to_string(),
                radio,
            });
            radio = false;
            expecting_url = false;
        }
    }
    lib.loaded = true;
    lib
}

/// A line without `#EXTINF` is only a channel if it actually names a source.
/// Covers http(s), rtmp, udp, mms, file and the like.
fn looks_like_stream_url(s: &str) -> bool {
    s.contains("://")
}

/// Fallback label for a bare URL: the last path segment, without its query.
fn bare_channel_name(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/').next().unwrap_or(url).to_string()
}

fn parse_extinf(line: &str) -> (String, Option<String>, Option<String>, bool) {
    let mut logo = None;
    let mut group = None;
    let mut radio = false;

    // Drop the "#EXTINF:<duration>" header first. The old scanner started at
    // byte 0, read that header as the first "key", hit the following space and
    // broke out immediately — so tvg-logo and group-title were never parsed.
    // A header with no attributes at all ("#EXTINF:0,Sender") has no space
    // before the comma, in which case there is nothing to scan.
    let body = line.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b' ' || bytes[i] == b'\t' {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b'=' && bytes[i] != b' ' && bytes[i] != b'\t' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            break;
        }
        let key = &body[start..i];
        i += 1; // '='
        if i >= bytes.len() {
            break;
        }
        // Quoted values may contain spaces, so split_whitespace is not usable.
        // The slice must start AFTER the opening quote — the old code started at
        // the quote itself and produced logos like `"http://l/a.png`.
        let (val, next) = if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            let vs = i + 1;
            let mut j = vs;
            while j < bytes.len() && bytes[j] != quote {
                j += 1;
            }
            (&body[vs..j], if j < bytes.len() { j + 1 } else { j })
        } else {
            let vs = i;
            let mut j = vs;
            while j < bytes.len() && bytes[j] != b' ' && bytes[j] != b'\t' {
                j += 1;
            }
            (&body[vs..j], j)
        };
        i = next;
        match key {
            "tvg-logo" => logo = Some(val.to_string()),
            "group-title" => group = Some(val.to_string()),
            // IPTVnator parity: radio="true" marks audio-only stations.
            "radio" => radio = val.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    // Channel name is everything after the LAST comma (attribute values may
    // legitimately contain commas, e.g. group-title="News, Sport").
    let name = line
        .rfind(',')
        .map(|p| line[p + 1..].trim().to_string())
        .unwrap_or_default();
    (name, logo, group, radio)
}

/// Validate credentials / availability and load the full library for a profile.
pub async fn load_profile(p: &Profile, timeout_s: u64, live_ext: &str) -> Result<Library> {
    match p.kind {
        ProfileKind::Xtream => {
            let stream_base = xtream_login(&p.host, &p.username, &p.password, timeout_s).await?;
            xtream_load_library(
                &p.host,
                &stream_base,
                &p.username,
                &p.password,
                timeout_s,
                live_ext,
            )
            .await
        }
        ProfileKind::M3u => m3u_load(&p.m3u_url, &p.m3u_file, timeout_s).await,
    }
}

pub(crate) fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        bare_channel_name, episodes_from_info, first_number, live_stream_url, normalize_host,
        parse_m3u, stream_ext, xtream_load_library, SeriesInfo,
    };
    use crate::types::CatalogArea;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// Route by `action=` instead of by arrival order. The loader now issues
    /// three requests concurrently, so a sequential "first request gets the
    /// first canned body" mock would be non-deterministic.
    fn mock_xtream_server(
        routes: Vec<(&'static str, u16, &'static str)>,
        expected_requests: usize,
    ) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let mut workers = Vec::new();
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let routes = routes.clone();
                let requests = Arc::clone(&requests);
                workers.push(thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    let size = stream.read(&mut buf).unwrap();
                    let line = String::from_utf8_lossy(&buf[..size])
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_string();
                    requests.lock().unwrap().push(line.clone());
                    let action = line
                        .split("action=")
                        .nth(1)
                        .and_then(|s| s.split(['&', ' ']).next())
                        .unwrap_or("");
                    let (status, body) = routes
                        .iter()
                        .find(|(a, _, _)| *a == action)
                        .map(|(_, s, b)| (*s, *b))
                        .unwrap_or((404, "[]"));
                    let reason = match status {
                        200 => "OK",
                        404 => "Not Found",
                        _ => "Internal Server Error",
                    };
                    write!(stream, "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, reason, body.len(), body).unwrap();
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
            let guard = requests.lock().unwrap();
            guard.clone()
        });
        (format!("http://{}", address), handle)
    }

    const LIVE_CATS: (&str, u16, &str) = (
        "get_live_categories",
        200,
        r#"[{"category_id":"lc","category_name":"Live"}]"#,
    );
    const VOD_CATS: (&str, u16, &str) = (
        "get_vod_categories",
        200,
        r#"[{"category_id":"vc","category_name":"Movies"}]"#,
    );
    const SER_CATS: (&str, u16, &str) = (
        "get_series_categories",
        200,
        r#"[{"category_id":"sc","category_name":"Shows"}]"#,
    );
    const LIVE_STREAMS: (&str, u16, &str) = (
        "get_live_streams",
        200,
        r#"[{"stream_id":11,"num":1,"name":"News","category_id":"lc"}]"#,
    );
    const VOD_STREAMS: (&str, u16, &str) = (
        "get_vod_streams",
        200,
        r#"[{"stream_id":22,"name":"Film","category_id":"vc","container_extension":"mkv"}]"#,
    );
    const SER_STREAMS: (&str, u16, &str) = (
        "get_series",
        200,
        r#"[{"series_id":33,"name":"Series","category_id":"sc"}]"#,
    );

    fn all_ok_routes() -> Vec<(&'static str, u16, &'static str)> {
        vec![
            LIVE_CATS,
            VOD_CATS,
            SER_CATS,
            LIVE_STREAMS,
            VOD_STREAMS,
            SER_STREAMS,
        ]
    }

    #[tokio::test]
    async fn xtream_library_loads_all_six_endpoint_groups() {
        let (host, server) = mock_xtream_server(all_ok_routes(), 6);

        let library = xtream_load_library(&host, "", "user", "pass", 5, "ts")
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert!(library.loaded);
        assert_eq!(library.stream_base, host);
        assert_eq!(library.live_categories.len(), 1);
        assert_eq!(library.vod_categories.len(), 1);
        assert_eq!(library.series_categories.len(), 1);
        assert_eq!(library.channels.len(), 1);
        assert_eq!(library.channels[0].name, "News");
        assert_eq!(library.movies.len(), 1);
        assert_eq!(
            library.movies[0].stream_url,
            format!("{}/movie/user/pass/22.mkv", host)
        );
        assert_eq!(library.series.len(), 1);
        assert_eq!(library.series[0].name, "Series");
        assert!(
            library.catalog_issues.is_empty(),
            "{:?}",
            library.catalog_issues
        );
        assert_eq!(requests.len(), 6);
        for action in [
            "get_live_categories",
            "get_vod_categories",
            "get_series_categories",
            "get_live_streams",
            "get_vod_streams",
            "get_series",
        ] {
            assert!(
                requests
                    .iter()
                    .any(|request| request.contains(&format!("action={}", action))),
                "missing request for {}",
                action
            );
        }
    }

    /// Regression for the reported "first login shows only Live-TV until you
    /// log in ten times" bug: a failing VOD/Series endpoint must NOT abort the
    /// entire login or discard the parts that did arrive.
    #[tokio::test]
    async fn xtream_single_endpoint_failure_keeps_remaining_library() {
        let routes = vec![
            LIVE_CATS,
            VOD_CATS,
            ("get_series_categories", 500, "{}"),
            LIVE_STREAMS,
            VOD_STREAMS,
            ("get_series", 200, "not-json"),
        ];
        let (host, server) = mock_xtream_server(routes, 6);

        let library = xtream_load_library(&host, "", "user", "pass", 5, "ts")
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert!(library.loaded, "login must succeed after authentication");
        assert_eq!(requests.len(), 6, "all six areas must be attempted");
        assert_eq!(library.channels.len(), 1, "live TV must still be usable");
        assert_eq!(library.movies.len(), 1, "films must still be usable");
        assert_eq!(library.series.len(), 0);
        assert_eq!(library.catalog_issues.len(), 2);
        assert!(library
            .catalog_issues
            .iter()
            .any(|i| i.area == CatalogArea::SeriesCategories));
        assert!(library
            .catalog_issues
            .iter()
            .any(|i| i.area == CatalogArea::Series));
        assert!(library.catalog_issue_summary().contains("Serienliste"));
        // Diagnostics must never contain provider credentials.
        let dump = format!("{:?}", library.catalog_issues);
        assert!(!dump.contains("password="));
        assert!(!dump.contains("username="));
    }

    #[tokio::test]
    async fn legitimately_empty_vod_catalog_is_not_reported_as_an_error() {
        let routes = vec![
            LIVE_CATS,
            ("get_vod_categories", 200, "[]"),
            SER_CATS,
            LIVE_STREAMS,
            ("get_vod_streams", 200, "[]"),
            ("get_series", 200, "[]"),
        ];
        let (host, server) = mock_xtream_server(routes, 6);

        let library = xtream_load_library(&host, "", "user", "pass", 5, "ts")
            .await
            .unwrap();
        let _ = server.join().unwrap();

        assert!(library.loaded);
        assert!(library.movies.is_empty());
        assert!(library.series.is_empty());
        assert!(
            library.catalog_issues.is_empty(),
            "empty lists are valid data"
        );
        assert!(library.catalog_issue_summary().is_empty());
    }

    #[test]
    fn scheme_less_m3u_links_get_http() {
        assert_eq!(
            normalize_host("provider.eu/get.php?username=a&password=b").unwrap(),
            "http://provider.eu/get.php?username=a&password=b"
        );
        assert_eq!(
            normalize_host("  http://10.0.2.2:8899/liste.m3u/  ").unwrap(),
            "http://10.0.2.2:8899/liste.m3u"
        );
        assert_eq!(
            normalize_host("https://example.com/list.m3u").unwrap(),
            "https://example.com/list.m3u"
        );
        assert!(normalize_host("   ").is_err());
    }

    /// Regression: the plain "one URL per line" playlist used to parse into an
    /// empty library, so a perfectly valid M3U link looked broken on login.
    #[test]
    fn bare_urls_are_channels() {
        let lib = parse_m3u(
            "#EXTM3U\n\
             http://host/live/ard.ts\n\
             https://host2/live/zdf.m3u8?token=abc\n\
             udp://@239.0.0.1:1234\n",
        );
        assert_eq!(lib.channels.len(), 3);
        assert_eq!(lib.channels[0].stream_url, "http://host/live/ard.ts");
        assert_eq!(lib.channels[0].name, "ard.ts");
        assert_eq!(lib.channels[1].name, "zdf.m3u8");
        assert_eq!(lib.channels[1].category_id, "Channels");
        assert_eq!(lib.live_categories.len(), 1);
        assert!(lib.loaded);
    }

    #[test]
    fn extinf_metadata_is_still_used() {
        let lib = parse_m3u(
            "#EXTM3U\n\
             #EXTINF:-1 tvg-logo=\"http://l/a.png\" group-title=\"Sport\",ARD HD\n\
             http://host/1.ts\n\
             #EXTINF:-1,ZDF HD\n\
             http://host/2.ts\n",
        );
        assert_eq!(lib.channels.len(), 2);
        assert_eq!(lib.channels[0].name, "ARD HD");
        assert_eq!(lib.channels[0].logo, "http://l/a.png");
        assert_eq!(lib.channels[0].category_id, "Sport");
        assert_eq!(lib.channels[1].name, "ZDF HD");
    }

    #[test]
    fn radio_attribute_marks_audio_only_channels() {
        let lib = parse_m3u(
            "#EXTM3U\n\
             #EXTINF:-1 radio=\"true\" group-title=\"Radio\",Jazz FM\n\
             https://radio.example/stream\n\
             #EXTINF:-1 radio=\"false\",Music TV\n\
             https://tv.example/live.m3u8\n",
        );
        assert_eq!(lib.channels.len(), 2);
        assert!(lib.channels[0].radio);
        assert!(!lib.channels[1].radio);
        assert_eq!(lib.channels[0].name, "Jazz FM");
        assert_eq!(lib.channels[0].category_id, "Radio");
    }

    /// An HTML error page must not be turned into a list of junk channels.
    #[test]
    fn html_is_not_a_playlist() {
        let lib = parse_m3u(
            "<!doctype html>\n<html><body>404 Not Found</body></html>\nhttps://host/a.ts\n",
        );
        // Only the genuinely URL-shaped line is accepted; the tags are skipped.
        assert_eq!(lib.channels.len(), 1);
        assert_eq!(lib.channels[0].stream_url, "https://host/a.ts");
        assert_eq!(bare_channel_name("https://h/x.ts?k=1#f"), "x.ts");
    }

    #[test]
    fn container_extensions_are_normalised() {
        assert_eq!(stream_ext("mp4", "ts"), "mp4");
        assert_eq!(stream_ext(".MKV", "ts"), "mkv");
        assert_eq!(stream_ext("", "ts"), "ts");
        assert_eq!(stream_ext("  ", "m3u8"), "m3u8");
        // A panel returning HTML or a URL in this field must not end up in the
        // stream path.
        assert_eq!(stream_ext("text/html", "mp4"), "mp4");
        assert_eq!(stream_ext("../../../etc/passwd", "mp4"), "mp4");
    }

    #[test]
    fn live_urls_use_the_selected_format() {
        assert_eq!(
            live_stream_url("http://tv:8080/", "u@x", "p w", 42, "ts"),
            "http://tv:8080/live/u%40x/p%20w/42.ts"
        );
        assert_eq!(
            live_stream_url("http://tv:8080", "u", "p", 42, "m3u8"),
            "http://tv:8080/live/u/p/42.m3u8"
        );
    }

    #[test]
    fn series_episodes_use_real_container_and_episode_num() {
        // Shape of a real Xtream `get_series_info` response: season is the map
        // key, the number is `episode_num`, and the file is an .mkv.
        let json = r#"{
            "info": {"name": "Demo"},
            "episodes": {
                "2": [
                    {"id": "902", "episode_num": 3, "title": "Folge drei",
                     "container_extension": "mkv", "season": "2"},
                    {"id": "901", "episode_num": 1, "title": "Pilot",
                     "container_extension": ".MP4"}
                ],
                "1": [
                    {"id": "900", "episode_num": "2", "title": "Auflösung",
                     "container_extension": ""}
                ]
            }
        }"#;
        let info: SeriesInfo = serde_json::from_str(json).unwrap();
        let eps = episodes_from_info(&info, 77, "http://tv:8080", "u", "p");
        assert_eq!(eps.len(), 3);
        // Sorted by season, then episode.
        assert_eq!((eps[0].season, eps[0].episode), (1, 2));
        assert_eq!((eps[1].season, eps[1].episode), (2, 1));
        assert_eq!((eps[2].season, eps[2].episode), (2, 3));
        assert_eq!(eps[0].stream_url, "http://tv:8080/series/u/p/900.mp4");
        assert_eq!(eps[1].stream_url, "http://tv:8080/series/u/p/901.mp4");
        assert_eq!(eps[2].stream_url, "http://tv:8080/series/u/p/902.mkv");
    }

    #[test]
    fn episode_number_falls_back_to_legacy_field_then_title() {
        assert_eq!(first_number("Folge 12: Der Anfang"), 12);
        assert_eq!(first_number("Keine Nummer"), 0);
    }
}
