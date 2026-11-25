use super::components::{field, Kb};
use crate::release_update::{runtime, GitHub, Update};
#[cfg(target_os = "android")]
use anyhow::Context;
use anyhow::Result;
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

#[cfg(target_os = "android")]
const PACKAGE: (&str, &str, u32, &str) = (
    "Dado-Player-arm64-debug.apk",
    "android-build-",
    crate::release_info::ANDROID_BUILD,
    "Dado-Player.apk",
);
#[cfg(target_os = "ios")]
const PACKAGE: (&str, &str, u32, &str) = (
    "Dado-Player-unsigned.ipa",
    "ios-build-",
    crate::release_info::IOS_BUILD,
    "Dado-Player.ipa",
);

#[derive(Default)]
struct State {
    token: String,
    visible: bool,
    busy: bool,
    ready: bool,
    available: Option<Update>,
    message: String,
    #[cfg(target_os = "android")]
    attempted_install: bool,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(Default::default)
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
    crate::platform::android::jni_bridge::mobile_update(install)
}

async fn stage(github: GitHub, update: Update) -> Result<()> {
    let dir = directory()?;
    std::fs::create_dir_all(&dir)?;
    let temp = dir.join("update.part");
    let result = async {
        github.download(&update, &temp).await?;
        std::fs::rename(&temp, dir.join(PACKAGE.3))?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}

fn hand_off(state: &mut State) {
    #[cfg(target_os = "android")]
    {
        state.attempted_install = true;
    }
    state.message = match native(true) {
        Err(error) => error.to_string(),
        #[cfg(target_os = "android")]
        Ok(()) => "Update geprüft. Installation wird an Android übergeben …".into(),
        #[cfg(target_os = "ios")]
        Ok(()) => "IPA geprüft. Installiere sie als Update mit AltStore Classic und demselben Apple-Konto.".into(),
    };
}

pub fn show(ui: &mut egui::Ui) {
    let mut s = state().lock().unwrap();
    ui.heading("App-Updates");
    ui.label(format!("Installierter Build {}", PACKAGE.2));
    ui.add_enabled_ui(!s.busy, |ui| {
        ui.collapsing("GitHub-Zugang für private Releases", |ui| {
            ui.label("Fine-grained Token für tariktech7/dado-player · Contents: Read-only");
            let State { token, visible, .. } = &mut *s;
            let r = field(
                ui,
                "Token",
                token,
                "GitHub-Token mit Leserecht",
                Kb::Password,
                visible,
            );
            if r.gained_focus() {
                crate::platform::seed_ime(token);
            }
        });
    });
    if s.busy {
        ui.spinner();
    } else if let Some(update) = s.available.clone() {
        #[cfg(target_os = "android")]
        let title = format!("{} installieren", update.version);
        #[cfg(target_os = "ios")]
        let title = format!("{} für AltStore vorbereiten", update.version);
        let install = ui.button(title);
        if super::focus::clicked(ui, &install) {
            if s.ready {
                hand_off(&mut s);
            } else {
                s.busy = true;
                s.message = "Update wird geladen und geprüft …".into();
                let token = s.token.clone();
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let result = runtime().and_then(|rt| {
                        let github = GitHub::new(token)?;
                        rt.block_on(stage(github, update))
                    });
                    let mut s = state().lock().unwrap();
                    s.busy = false;
                    match result {
                        Ok(()) => {
                            s.token.clear();
                            s.ready = true;
                            hand_off(&mut s);
                        }
                        Err(error) => s.message = error.to_string(),
                    }
                    ctx.request_repaint();
                });
            }
        }
        let reset = ui.button("Erneut auf Updates prüfen");
        if !s.busy && super::focus::clicked(ui, &reset) {
            s.ready = false;
            s.available = None;
            s.message.clear();
            #[cfg(target_os = "android")]
            {
                s.attempted_install = false;
            }
        }
    } else {
        let check = ui.button("Auf GitHub-Updates prüfen");
        if super::focus::clicked(ui, &check) {
            s.busy = true;
            s.message = "GitHub wird abgefragt …".into();
            let token = s.token.clone();
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let result = runtime().and_then(|rt| {
                    let github = GitHub::new(token)?;
                    rt.block_on(github.check(PACKAGE.0, PACKAGE.1, PACKAGE.2))
                });
                let mut s = state().lock().unwrap();
                s.busy = false;
                s.message = match result {
                    Ok(Some(update)) => {
                        let message = format!(
                            "Version {} · Build {} verfügbar.",
                            update.version, update.build
                        );
                        s.available = Some(update);
                        message
                    }
                    Ok(None) => "Du verwendest die aktuelle Version.".into(),
                    Err(error) => error.to_string(),
                };
                ctx.request_repaint();
            });
        }
    }
    #[cfg(target_os = "android")]
    if s.attempted_install && !s.busy {
        if let Ok(status) = crate::platform::android::jni_bridge::mobile_update_status() {
            if !status.is_empty() {
                s.message = status;
            }
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(500));
    }
    if !s.message.is_empty() {
        ui.label(&s.message);
    }
    #[cfg(target_os = "ios")]
    ui.label("AltStore Classic signiert das Update. Wähle die gespeicherte IPA dort unter „My Apps“ über + aus.");
    let releases = ui.button("GitHub-Releases öffnen");
    if !s.busy && super::focus::clicked(ui, &releases) {
        if let Err(error) = native(false) {
            s.message = error.to_string();
        }
    }
}
