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
    id: u64,
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
    token: String,
}

impl GitHub {
    pub fn new(token: String) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("DadoPlayer-Update")
                .timeout(Duration::from_secs(180))
                .redirect(reqwest::redirect::Policy::limited(4))
                .https_only(true)
                .build()?,
            token,
        })
    }

    fn get(&self, url: String) -> reqwest::RequestBuilder {
        let request = self.client.get(url);
        if self.token.trim().is_empty() {
            request
        } else {
            request.bearer_auth(self.token.trim())
        }
    }

    pub async fn check(
        &self,
        file: &str,
        label: &str,
        current_build: u32,
    ) -> Result<Option<Update>> {
        let response = self
            .get(format!("{API}?per_page=30"))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        ensure!(
            !matches!(response.status().as_u16(), 401 | 403 | 404),
            "GitHub-Lesezugriff fehlt oder das Limit ist erreicht. Prüfe den Zugang zum privaten Repository."
        );
        let releases = response.error_for_status()?.json::<Vec<Release>>().await?;
        Ok(select(
            releases,
            file,
            label,
            version(crate::release_info::VERSION).expect("packaged version"),
            current_build,
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
        let mut response = self
            .get(format!("{API}/assets/{}", asset.id))
            .header("Accept", "application/octet-stream")
            .send()
            .await?
            .error_for_status()?;
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

pub fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

fn version(value: &str) -> Option<[u32; 3]> {
    let mut parts = value.split('.');
    let parsed = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(parsed)
}

fn release_version(tag: &str) -> Option<(&str, [u32; 3])> {
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
    Some((number, version(number)?))
}

fn select(
    releases: Vec<Release>,
    file: &str,
    label: &str,
    current_version: [u32; 3],
    current_build: u32,
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
            (version >= current_version && published_build > current_build).then_some((
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
            {"tag_name":"v1.0.0","draft":true,"assets":[{"name":"file","id":1,"size":42,"label":"build-50"}]},
            {"tag_name":"v0.6.0","draft":false,"assets":[{"name":"file","id":2,"size":42,"label":"build-60"}]},
            {"tag_name":"v0.7.9-beta","draft":false,"assets":[{"name":"file","id":3,"size":42,"label":"build-31"}]},
            {"tag_name":"v0.7.10-beta","draft":false,"assets":[{"name":"incoming-file","id":4,"size":42,"label":"build-32"}]},
            {"tag_name":"v0.7.9-beta.2","draft":false,"assets":[{"name":"file","id":6,"size":42,"label":"build-32"}]},
            {"tag_name":"v0.7.9-beta.0","draft":false,"assets":[{"name":"file","id":7,"size":42,"label":"build-99"}]},
            {"tag_name":"v0.7.9","draft":false,"assets":[{"name":"file","id":5,"size":42,"label":"build-30"}]}
        ])).unwrap();
        let update = select(releases, "file", "build-", [0, 7, 9], 30).unwrap();
        assert_eq!(update.version, "0.7.9");
        assert_eq!(update.build, 32);
        assert_eq!(update.asset.id, 6);
    }
}
