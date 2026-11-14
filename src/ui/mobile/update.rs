use super::components::{field, Kb};
use anyhow::{anyhow, ensure, Context, Result};
use serde::Deserialize;
use std::{
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::Duration,
};

const API: &str = "https://api.github.com/repos/tariktech7/dado-player/releases";
#[cfg(target_os = "android")]
const PACKAGE: (&str, &str, u32, &str) = (
    "Dado-Player-arm64-debug.apk",
    "android-build-",
    8,
    "Dado-Player.apk",
);
#[cfg(target_os = "ios")]
const PACKAGE: (&str, &str, u32, &str) = (
    "Dado-Player-unsigned.ipa",
    "ios-build-",
    5,
    "Dado-Player.ipa",
);

#[derive(Default)]
struct State {
    token: String,
    visible: bool,
    busy: bool,
    ready: bool,
    message: String,
}
fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}
#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    label: Option<String>,
    id: u64,
    size: u64,
    digest: Option<String>,
}

fn directory() -> Result<PathBuf> {
    #[cfg(target_os = "android")]
    return Ok(PathBuf::from(
        crate::platform::android::jni_bridge::storage_dir().context("App-Speicher fehlt")?,
    )
    .join("updates"));
    #[cfg(target_os = "ios")]
    return Ok(std::env::temp_dir());
}
fn native(install: bool) -> Result<()> {
    #[cfg(target_os = "ios")]
    {
        extern "C" {
            fn dado_host_mobile_update(install: i32);
        }
        unsafe {
            dado_host_mobile_update(install as i32);
        }
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        crate::platform::android::jni_bridge::mobile_update(install)
    }
}
async fn download(token: String) -> Result<bool> {
    let client = reqwest::Client::builder()
        .user_agent("DadoPlayer-Mobile")
        .timeout(Duration::from_secs(180))
        .build()?;
    let get = |url: String| {
        let r = client.get(url);
        if token.trim().is_empty() {
            r
        } else {
            r.bearer_auth(token.trim())
        }
    };
    let res = get(format!("{API}/latest")).send().await?;
    ensure!(!matches!(res.status().as_u16(),401|403|404),"Privates Repository: GitHub-Lesezugriff erforderlich. Token prüfen oder Releases im Browser öffnen.");
    let release: Release = res.error_for_status()?.json().await?;
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name == PACKAGE.0)
        .context("Kein Paket für diese Plattform im neuesten Release")?;
    let build: u32 = asset
        .label
        .as_deref()
        .and_then(|s| s.strip_prefix(PACKAGE.1))
        .context("Buildnummer im Release fehlt")?
        .parse()?;
    if build <= PACKAGE.2 {
        return Ok(false);
    }
    ensure!(
        asset.size > 0 && asset.size <= 128 * 1024 * 1024,
        "Ungültige Paketgröße"
    );
    let expected = asset
        .digest
        .as_deref()
        .and_then(|s| s.strip_prefix("sha256:"))
        .context("GitHub-Prüfsumme fehlt")?;
    let mut response = get(format!("{API}/assets/{}", asset.id))
        .header("Accept", "application/octet-stream")
        .send()
        .await?
        .error_for_status()?;
    let dir = directory()?;
    std::fs::create_dir_all(&dir)?;
    let temp = dir.join("update.part");
    let mut file = std::fs::File::create(&temp)?;
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
        .map(|b| format!("{b:02x}"))
        .collect();
    ensure!(
        size == asset.size && expected == actual,
        "Download unvollständig oder Prüfsumme falsch"
    );
    file.sync_all()?;
    drop(file);
    std::fs::rename(temp, dir.join(PACKAGE.3))?;
    Ok(true)
}

pub fn show(ui: &mut egui::Ui) {
    let mut s = state().lock().unwrap();
    ui.heading("App-Updates");
    ui.label(format!("Installierter Build {}", PACKAGE.2));
    if ui.button("GitHub-Releases öffnen").clicked() {
        if let Err(e) = native(false) {
            s.message = e.to_string();
        }
    }
    ui.collapsing("GitHub-Zugang für private Releases", |ui| {
        ui.label("Ein Repository-Token mit Leserecht für Contents gilt nur für diese Prüfung.");
        let State { token, visible, .. } = &mut *s;
        let r = field(ui, "Token", token, "GitHub-Token", Kb::Password, visible);
        if r.gained_focus() {
            crate::platform::seed_ime(token);
        }
    });
    if ui
        .add_enabled(!s.busy, egui::Button::new("Update prüfen und laden"))
        .clicked()
    {
        s.busy = true;
        s.ready = false;
        s.message = "GitHub-Release wird geprüft …".into();
        let token = std::mem::take(&mut s.token);
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| anyhow!(e))
                .and_then(|rt| rt.block_on(download(token)));
            let mut s = state().lock().unwrap();
            s.busy = false;
            s.ready = matches!(result, Ok(true));
            s.message = match result {
                Ok(true) => "Update geladen und geprüft.".into(),
                Ok(false) => "Du verwendest die aktuelle Version.".into(),
                Err(e) => e.to_string(),
            };
            ctx.request_repaint();
        });
    }
    if !s.message.is_empty() {
        ui.label(&s.message);
    }
    #[cfg(target_os = "ios")]
    let action = "IPA teilen / in Dateien sichern";
    #[cfg(target_os = "android")]
    let action = "Android-Installer öffnen";
    if s.ready && ui.button(action).clicked() {
        if let Err(e) = native(true) {
            s.message = e.to_string();
        }
    }
    #[cfg(target_os = "ios")]
    ui.label("Die neue IPA in AltStore Classic unter My Apps über + installieren.");
}
