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
    busy: bool,
    ready: Option<bool>,
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

fn native(previous: bool) -> Result<()> {
    #[cfg(target_os = "ios")]
    {
        let _ = previous;
        extern "C" {
            fn dado_host_mobile_update(install: i32);
        }
        unsafe {
            dado_host_mobile_update(1);
        }
        Ok(())
    }
    #[cfg(target_os = "android")]
    crate::platform::android::jni_bridge::mobile_update(if previous { 2 } else { 1 })
}

async fn stage(github: GitHub, update: Update, previous: bool) -> Result<()> {
    let dir = directory()?;
    std::fs::create_dir_all(&dir)?;
    let temp = dir.join("update.part");
    let file = if cfg!(target_os = "android") && previous {
        "Dado-Player-vorherige-Version.apk"
    } else {
        PACKAGE.3
    };
    let result = async {
        github.download(&update, &temp).await?;
        std::fs::rename(&temp, dir.join(file))?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}

fn hand_off(state: &mut State, previous: bool) {
    #[cfg(target_os = "android")]
    {
        state.attempted_install = true;
    }
    state.message = match native(previous) {
        Err(error) => error.to_string(),
        #[cfg(target_os = "android")]
        Ok(()) if previous => "Android sperrt Downgrades. Du kannst die geprüfte ältere APK speichern.".into(),
        #[cfg(target_os = "android")]
        Ok(()) => "Update geprüft. Installation wird an Android übergeben …".into(),
        #[cfg(target_os = "ios")]
        Ok(()) => "IPA geprüft. Installiere sie als Update mit AltStore Classic und demselben Apple-Konto.".into(),
    };
}

pub fn show(ui: &mut egui::Ui) {
    let mut state = state().lock().unwrap();
    let action = ui.add_enabled_ui(!state.busy, |ui| {
        let check = ui.button("App-Update prüfen und installieren");
        let restore = ui.button("Vorherige Version wiederherstellen");
        if super::focus::clicked(ui, &check) {
            Some(false)
        } else if super::focus::clicked(ui, &restore) {
            Some(true)
        } else {
            None
        }
    });
    if let Some(previous) = action.inner {
        if state.ready == Some(previous) {
            hand_off(&mut state, previous);
        } else {
            state.busy = true;
            state.ready = None;
            state.message = "Version wird gesucht, geladen und geprüft …".into();
            #[cfg(target_os = "android")]
            {
                state.attempted_install = false;
            }
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let result = runtime().and_then(|rt| {
                    rt.block_on(async {
                        let github = GitHub::new()?;
                        let update = github
                            .check(PACKAGE.0, PACKAGE.1, PACKAGE.2, previous)
                            .await?;
                        if let Some(update) = update {
                            self::state().lock().unwrap().message = format!(
                                "Version {} · Build {} wird geladen …",
                                update.version, update.build
                            );
                            ctx.request_repaint();
                            stage(github, update, previous).await?;
                            return Ok(true);
                        }
                        Ok(false)
                    })
                });
                let mut state = self::state().lock().unwrap();
                state.busy = false;
                if matches!(result, Ok(true)) {
                    state.ready = Some(previous);
                    hand_off(&mut state, previous);
                } else {
                    state.message = crate::release_update::status(result, previous);
                }
                ctx.request_repaint();
            });
        }
    }
    if state.busy {
        ui.spinner();
    }
    #[cfg(target_os = "android")]
    if state.attempted_install && !state.busy {
        if let Ok(status) = crate::platform::android::jni_bridge::mobile_update_status() {
            if !status.is_empty() {
                state.message = status;
            }
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(500));
    }
    if !state.message.is_empty() {
        ui.label(&state.message);
    }
}
