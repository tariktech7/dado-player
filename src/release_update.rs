use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const API: &str = "https://api.github.com/repos/tariktech7/dado-player/releases";
const MAX_PACKAGE: u64 = 128 * 1024 * 1024;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    assets: Vec<Asset>,
}

#[derive(Clone, Deserialize)]
pub struct Asset {
    name: String,
    label: Option<String>,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Clone)]
pub struct Update {
    pub version: String,
    pub build: u32,
    asset: Asset,
}

pub struct GitHub {
    client: reqwest::Client,
}

impl GitHub {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("DadoPlayer-Update")
                .timeout(Duration::from_secs(180))
                .redirect(reqwest::redirect::Policy::limited(4))
                .https_only(true)
                .build()?,
        })
    }

    pub async fn check(
        &self,
        file: &str,
        label: &str,
        current_build: u32,
        previous: bool,
    ) -> Result<Option<Update>> {
        let response = self
            .client
            .get(format!("{API}?per_page=30"))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        ensure!(
            !matches!(response.status().as_u16(), 403 | 429),
            "GitHub begrenzt die Update-Abfragen. Versuche es später erneut."
        );
        let releases = response.error_for_status()?.json::<Vec<Release>>().await?;
        Ok(select(
            releases,
            file,
            label,
            version(crate::release_info::VERSION).expect("packaged version"),
            current_build,
            previous,
        ))
    }

    pub async fn download(&self, update: &Update, path: &Path) -> Result<()> {
        let asset = &update.asset;
        ensure!(
            (1..=MAX_PACKAGE).contains(&asset.size),
            "Ungültige Paketgröße"
        );
        let expected = asset
            .digest
            .as_deref()
            .and_then(|value| value.strip_prefix("sha256:"))
            .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .context("GitHub-Prüfsumme fehlt")?;
        let url = reqwest::Url::parse(&asset.browser_download_url)?;
        ensure!(
            url.scheme() == "https"
                && url.host_str() == Some("github.com")
                && url
                    .path()
                    .starts_with("/tariktech7/dado-player/releases/download/"),
            "Ungültige Download-Adresse"
        );
        let mut response = self.client.get(url).send().await?.error_for_status()?;
        let mut file = std::fs::File::create(path)?;
        let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
        let mut size = 0u64;
        while let Some(chunk) = response.chunk().await? {
            size += chunk.len() as u64;
            ensure!(size <= asset.size, "Download zu groß");
            hash.update(&chunk);
            file.write_all(&chunk)?;
        }
        let actual: String = hash
            .finish()
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        ensure!(
            size == asset.size && expected.eq_ignore_ascii_case(&actual),
            "Download unvollständig oder Prüfsumme falsch"
        );
        file.sync_all()?;
        Ok(())
    }
}

pub fn status(result: Result<bool>, previous: bool) -> String {
    match result {
        Ok(false) if previous => "Keine frühere Version verfügbar.".into(),
        Ok(false) => "Du verwendest die aktuelle Version.".into(),
        Ok(true) => "Paket geprüft.".into(),
        Err(error) => error.to_string(),
    }
}

pub fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

fn version(value: &str) -> Option<[u32; 4]> {
    let mut parts = value.split('.');
    let mut parsed = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        0,
    ];
    if let Some(revision) = parts.next() {
        let [digit @ b'1'..=b'9'] = revision.as_bytes() else {
            return None;
        };
        parsed[3] = u32::from(*digit - b'0');
    }
    parts.next().is_none().then_some(parsed)
}

fn release_version(tag: &str) -> Option<(&str, [u32; 4])> {
    let tag = tag.strip_prefix('v')?;
    let number = if let Some((number, suffix)) = tag.split_once('-') {
        if suffix != "beta" {
            let beta = suffix.strip_prefix("beta.")?;
            if beta.starts_with('0') || !beta.bytes().all(|b| b.is_ascii_digit()) || beta.is_empty()
            {
                return None;
            }
        }
        number
    } else {
        tag
    };
    let parsed = version(number)?;
    if parsed[3] != 0 && number != tag {
        return None;
    }
    Some((number, parsed))
}

fn select(
    releases: Vec<Release>,
    file: &str,
    label: &str,
    current_version: [u32; 4],
    current_build: u32,
    previous: bool,
) -> Option<Update> {
    releases
        .into_iter()
        .filter_map(|release| {
            if release.draft {
                return None;
            }
            let (tag, version) = release_version(&release.tag_name)?;
            let asset = release
                .assets
                .into_iter()
                .find(|asset| asset.name == file)?;
            let published_build = asset
                .label
                .as_deref()
                .and_then(|value| value.strip_prefix(label))
                .and_then(|value| value.parse::<u32>().ok())?;
            let eligible = if previous {
                version <= current_version && published_build < current_build
            } else {
                version >= current_version && published_build > current_build
            };
            eligible.then_some((
                (version, published_build),
                Update {
                    version: tag.into(),
                    build: published_build,
                    asset,
                },
            ))
        })
        .max_by_key(|(key, _)| *key)
        .map(|(_, update)| update)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_newest_compatible_beta_with_an_exact_canonical_asset() {
        let releases = serde_json::from_value(serde_json::json!([
            {"tag_name":"v1.0.0","draft":true,"assets":[{"name":"file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/1","size":42,"label":"build-50"}]},
            {"tag_name":"v0.6.0","draft":false,"assets":[{"name":"file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/2","size":42,"label":"build-60"}]},
            {"tag_name":"v0.7.9-beta","draft":false,"assets":[{"name":"file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/3","size":42,"label":"build-31"}]},
            {"tag_name":"v0.7.10-beta","draft":false,"assets":[{"name":"incoming-file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/4","size":42,"label":"build-32"}]},
            {"tag_name":"v0.7.9-beta.2","draft":false,"assets":[{"name":"file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/6","size":42,"label":"build-32"}]},
            {"tag_name":"v0.7.9-beta.0","draft":false,"assets":[{"name":"file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/7","size":42,"label":"build-99"}]},
            {"tag_name":"v0.7.9","draft":false,"assets":[{"name":"file","browser_download_url":"https://github.com/tariktech7/dado-player/releases/download/test/5","size":42,"label":"build-30"}]}
        ])).unwrap();
        let update = select(releases, "file", "build-", [0, 7, 9, 0], 30, false).unwrap();
        assert_eq!(update.version, "0.7.9");
        assert_eq!(update.build, 32);
        assert!(update.asset.browser_download_url.ends_with("/6"));
        let older = select(
            vec![Release {
                tag_name: "v0.7.9-beta.2".into(),
                draft: false,
                assets: vec![update.asset],
            }],
            "file",
            "build-",
            [0, 8, 0, 0],
            33,
            true,
        )
        .unwrap();
        assert_eq!(older.build, 32);
    }

    #[test]
    fn stable_revisions_upgrade_and_roll_back_in_order() {
        let releases = || {
            [
                ("v0.8.5", 23),
                ("v0.8.5.1", 24),
                ("v0.8.5.9", 25),
                ("v0.8.5.10", 30),
                ("v0.8.5.01", 31),
                ("v0.8.5.0", 32),
                ("v0.8.5.2-beta", 33),
            ]
            .into_iter()
            .map(|(tag, build)| Release {
                tag_name: tag.into(),
                draft: false,
                assets: vec![Asset {
                    name: "file".into(),
                    label: Some(format!("build-{build}")),
                    browser_download_url: String::new(),
                    size: 42,
                    digest: None,
                }],
            })
            .collect()
        };
        let next = select(releases(), "file", "build-", [0, 8, 5, 0], 23, false).unwrap();
        assert_eq!((next.version.as_str(), next.build), ("0.8.5.9", 25));
        let previous = select(releases(), "file", "build-", [0, 8, 5, 9], 25, true).unwrap();
        assert_eq!((previous.version.as_str(), previous.build), ("0.8.5.1", 24));
        assert!(select(releases(), "file", "build-", [0, 8, 5, 9], 25, false).is_none());
        assert!(version("0.8.5.9").unwrap() < version("0.8.6").unwrap());
    }
}
