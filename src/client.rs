use crate::types::*;
use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Deserializer};
use std::time::Duration;

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

/// Xtream panels usually Base64-encode EPG titles/descriptions.
fn decode_epg(s: &str) -> String {
    let t = s.trim();
    let looks_b64 = !t.is_empty()
        && t.len() % 4 == 0
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
    let resp = c.get(&url).send().await?.error_for_status()?;
    let v: AuthResp = resp.json().await?;
    let auth = v.user_info.map(|u| u.auth).unwrap_or(0);
    if auth != 1 {
        return Err(anyhow!("Anmeldung fehlgeschlagen (auth={})", auth));
    }
    let mut stream_base = base.clone();
    if let Some(si) = v.server_info {
        let host_part = si.url.trim();
        let https_port = si.https_port.trim();
        if !host_part.is_empty() && !https_port.is_empty() && https_port != "0" {
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
}

pub async fn xtream_load_library(
    host: &str,
    stream_base: &str,
    user: &str,
    pass: &str,
    timeout_s: u64,
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

    let live_cat: Vec<CatX> = c.get(api("get_live_categories")).send().await?.json().await?;
    let vod_cat: Vec<CatX> = c.get(api("get_vod_categories")).send().await?.json().await?;
    let ser_cat: Vec<CatX> = c.get(api("get_series_categories")).send().await?.json().await?;

    let channels_x: Vec<ChanX> = c.get(api("get_live_streams")).send().await?.json().await?;
    let movies_x: Vec<MovieX> = c.get(api("get_vod_streams")).send().await?.json().await?;
    let series_x: Vec<SeriesX> = c.get(api("get_series")).send().await?.json().await?;

    let mut lib = Library::default();
    lib.live_categories = live_cat
        .into_iter()
        .map(|c| Category { id: c.category_id, name: c.category_name })
        .collect();
    lib.vod_categories = vod_cat
        .into_iter()
        .map(|c| Category { id: c.category_id, name: c.category_name })
        .collect();
    lib.series_categories = ser_cat
        .into_iter()
        .map(|c| Category { id: c.category_id, name: c.category_name })
        .collect();

    lib.channels = channels_x
        .into_iter()
        .map(|x| {
            Channel {
                id: x.stream_id,
                num: x.num,
                name: x.name,
                logo: x.stream_icon,
                category_id: x.category_id,
                epg_channel_id: x.epg_channel_id,
                stream_url: format!("{}/live/{}/{}/{}.m3u8", sb, user, pass, x.stream_id),
            }
        })
        .collect();

    lib.movies = movies_x
        .into_iter()
        .map(|x| VodMovie {
            id: x.stream_id,
            name: x.name,
            cover: if !x.cover.is_empty() { x.cover } else { x.stream_icon },
            category_id: x.category_id,
            rating: x.rating,
            container_extension: x.container_extension.clone(),
            stream_url: format!(
                "{}/movie/{}/{}/{}.{}",
                sb, user, pass, x.stream_id, x.container_extension
            ),
            plot: x.plot,
        })
        .collect();

    lib.series = series_x
        .into_iter()
        .map(|x| Series {
            id: x.series_id,
            name: x.name,
            cover: x.cover,
            category_id: x.category_id,
            plot: x.plot,
        })
        .collect();

    lib.stream_base = sb;
    lib.loaded = true;
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
    #[serde(default, deserialize_with = "as_string")]
    season: String,
    #[serde(default, deserialize_with = "as_string")]
    episode: String,
    #[serde(default, deserialize_with = "as_string")]
    container_extension: String,
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
    let info: SeriesInfo = c.get(&url).send().await?.json().await?;
    let mut out = Vec::new();
    for (_season_key, eps) in info.episodes {
        for e in eps {
            let season = e.season.parse::<u32>().unwrap_or(0);
            let episode = e.episode.parse::<u32>().unwrap_or(0);
            out.push(Episode {
                id: e.id,
                series_id,
                season,
                episode,
                title: e.title,
                container_extension: e.container_extension,
                stream_url: format!("{}/series/{}/{}/{}.m3u8", sb, user, pass, e.id),
            });
        }
    }
    out.sort_by(|a, b| (a.season, a.episode).cmp(&(b.season, b.episode)));
    Ok(out)
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

pub async fn xtream_short_epg(
    host: &str,
    user: &str,
    pass: &str,
    stream_id: u32,
    timeout_s: u64,
) -> Result<Vec<EpgEntry>> {
    let base = normalize_host(host)?;
    let url = format!(
        "{}/player_api.php?username={}&password={}&action=get_short_epg&stream_id={}",
        base,
        urlencode(user),
        urlencode(pass),
        stream_id
    );
    let c = http_client(timeout_s);
    let resp: EpgResp = c.get(&url).send().await?.json().await?;
    Ok(resp
        .epg_listings
        .into_iter()
        .map(|e| EpgEntry {
            title: decode_epg(&e.title),
            description: decode_epg(&e.description),
            start: e.start_timestamp.parse::<i64>().unwrap_or(0),
            end: e.stop_timestamp.parse::<i64>().unwrap_or(0),
        })
        .collect())
}

// ---------- M3U ----------

pub async fn m3u_load(url: &str, file: &str, timeout_s: u64) -> Result<Library> {
    let content = if !file.is_empty() {
        std::fs::read_to_string(file)?
    } else {
        let c = http_client(timeout_s);
        c.get(url).send().await?.text().await?
    };
    Ok(parse_m3u(&content))
}

pub fn parse_m3u(content: &str) -> Library {
    let mut lib = Library::default();
    let mut name: Option<String> = None;
    let mut logo: Option<String> = None;
    let mut group: Option<String> = None;
    let mut expecting_url = false;
    let mut id = 0u32;

    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("#EXTINF") {
            let (n, l, g) = parse_extinf(line);
            name = Some(n);
            logo = l;
            group = g;
            expecting_url = true;
        } else if line.starts_with('#') {
            // comment / directive -> reset expectation if it was a different tag
            expecting_url = false;
        } else if !line.is_empty() {
            if expecting_url {
                let g = group.take().unwrap_or_else(|| "Channels".to_string());
                id += 1;
                if !lib.live_categories.iter().any(|c| c.id == g) {
                    lib.live_categories.push(Category {
                        id: g.clone(),
                        name: g.clone(),
                    });
                }
                lib.channels.push(Channel {
                    id,
                    num: id,
                    name: name.take().unwrap_or_default(),
                    logo: logo.take().unwrap_or_default(),
                    category_id: g,
                    epg_channel_id: String::new(),
                    stream_url: line.to_string(),
                });
                expecting_url = false;
            }
        }
    }
    lib.loaded = true;
    lib
}

fn parse_extinf(line: &str) -> (String, Option<String>, Option<String>) {
    let mut logo = None;
    let mut group = None;
    // attribute scan
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b' ' || bytes[i] == b'\t' {
            i += 1;
            continue;
        }
        // find key=
        let start = i;
        while i < bytes.len() && bytes[i] != b'=' && bytes[i] != b' ' && bytes[i] != b'\t' {
            i += 1;
        }
        let key = &line[start..i];
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            let quote = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                bytes[i]
            } else {
                b'\0'
            };
            let val_start = i;
            if quote != b'\0' {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                let val = &line[val_start..i];
                i += 1; // skip closing quote
                match key {
                    "tvg-logo" => logo = Some(val.to_string()),
                    "group-title" => group = Some(val.to_string()),
                    _ => {}
                }
            }
        } else {
            break;
        }
    }
    // name after last comma
    let name = line
        .rfind(',')
        .map(|p| line[p + 1..].trim().to_string())
        .unwrap_or_default();
    (name, logo, group)
}

/// Validate credentials / availability and load the full library for a profile.
pub async fn load_profile(p: &Profile, timeout_s: u64) -> Result<Library> {
    match p.kind {
        ProfileKind::Xtream => {
            let stream_base =
                xtream_login(&p.host, &p.username, &p.password, timeout_s).await?;
            xtream_load_library(
                &p.host,
                &stream_base,
                &p.username,
                &p.password,
                timeout_s,
            )
            .await
        }
        ProfileKind::M3u => m3u_load(&p.m3u_url, &p.m3u_file, timeout_s).await,
    }
}

fn urlencode(s: &str) -> String {
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
