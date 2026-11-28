use crate::release_update::{runtime, GitHub, Update};
use anyhow::{ensure, Result};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const BUILD: u32 = crate::release_info::WINDOWS_BUILD;

#[derive(Default)]
struct State {
    busy: bool,
    message: String,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

async fn stage(github: &GitHub, dir: &Path, update: &Update) -> Result<()> {
    let partial = dir.join("download.partial");
    let result = async {
        github.download(update, &partial).await?;
        let package = std::fs::read(&partial)?;
        ensure!(package.get(..2) == Some(b"MZ"), "Ungültige Windows-EXE.");
        package_footer::footer_offset(&package).map_err(anyhow::Error::msg)?;
        std::fs::rename(&partial, dir.join("verified.exe"))?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_file(partial);
    }
    result
}

pub fn show(ui: &mut egui::Ui) {
    let Some(dir) = std::env::var_os("DADO_UPDATE_DIR").map(std::path::PathBuf::from) else {
        ui.label("Updates sind in der portablen Dado-Player.exe verfügbar.");
        return;
    };
    let mut state = state().lock().unwrap();
    if let Some(previous) = crate::ui::settings::update_actions(ui, !state.busy) {
        state.busy = true;
        state.message = "Version wird gesucht, geladen und geprüft …".into();
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let result = runtime().and_then(|rt| {
                rt.block_on(async {
                    let github = GitHub::new()?;
                    let update = github
                        .check("Dado-Player.exe", "windows-build-", BUILD, previous)
                        .await?;
                    if let Some(update) = update {
                        self::state().lock().unwrap().message = format!(
                            "Version {} · Build {} wird geladen …",
                            update.version, update.build
                        );
                        ctx.request_repaint();
                        stage(&github, &dir, &update).await?;
                        return Ok(true);
                    }
                    Ok(false)
                })
            });
            if matches!(result, Ok(true)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                let mut state = self::state().lock().unwrap();
                state.busy = false;
                state.message = crate::release_update::status(result, previous);
            }
            ctx.request_repaint();
        });
    }
    if state.busy {
        ui.spinner();
    }
    if !state.message.is_empty() {
        ui.label(&state.message);
    }
}

#[path = "../../../packaging/package_footer.rs"]
mod package_footer;
