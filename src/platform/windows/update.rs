use crate::release_update::{runtime, GitHub, Update};
use anyhow::{ensure, Result};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

const BUILD: u32 = crate::release_info::WINDOWS_BUILD;

#[derive(Default)]
struct State {
    token: String,
    busy: bool,
    available: Option<Update>,
    message: String,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

fn github(token: String) -> Result<GitHub> {
    if !token.trim().is_empty() {
        return GitHub::new(token);
    }
    let token = std::env::var("DADO_GITHUB_TOKEN")
        .ok()
        .or_else(git_credential)
        .unwrap_or_default();
    GitHub::new(token)
}

fn git_credential() -> Option<String> {
    let bundled = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Programs/WorkBuddyAI/resources/vendor/PortableGit/cmd/git.exe"));
    let exe = bundled
        .filter(|p| p.is_file())
        .unwrap_or_else(|| "git.exe".into());
    let mut child = Command::new(exe)
        .creation_flags(0x08000000)
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["-c", "credential.interactive=never", "credential", "fill"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"protocol=https\nhost=github.com\npath=tariktech7/dado-player.git\n\n")
            .ok()?;
    }
    let result = child.wait_with_output().ok()?;
    if !result.status.success() {
        return None;
    }
    String::from_utf8(result.stdout)
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("password=").map(str::to_owned))
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
    ui.add_enabled_ui(!state.busy, |ui| {
        let access = ui.collapsing("Zugang zum privaten GitHub-Release", |ui| {
            ui.label("Fine-grained Token für tariktech7/dado-player · Contents: Read-only");
            ui.add(crate::ui::text_field(
                &mut state.token,
                "GitHub-Token mit Leserecht",
                None,
                |edit| edit.password(true),
            ));
        });
        crate::ui::settings::focus_option(ui, &access.header_response);
    });
    if state.busy {
        ui.spinner();
    } else if let Some(update) = state.available.clone() {
        if ui
            .button(format!("{} installieren und neu starten", update.version))
            .clicked()
        {
            state.busy = true;
            state.message = "Update wird geladen und geprüft …".into();
            let token = state.token.clone();
            let ctx = ui.ctx().clone();
            drop(state);
            std::thread::spawn(move || {
                let result = runtime().and_then(|rt| {
                    let github = github(token)?;
                    rt.block_on(stage(&github, &dir, &update))
                });
                match result {
                    Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    Err(error) => {
                        let mut state = self::state().lock().unwrap();
                        state.message = error.to_string();
                        state.busy = false;
                    }
                }
                ctx.request_repaint();
            });
            return;
        }
        if ui.button("Erneut auf Updates prüfen").clicked() {
            state.available = None;
        }
    } else if ui.button("Auf GitHub-Updates prüfen").clicked() {
        state.busy = true;
        state.message = "GitHub wird abgefragt …".into();
        let token = state.token.clone();
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let result = runtime().and_then(|rt| {
                let github = github(token)?;
                rt.block_on(github.check("Dado-Player.exe", "windows-build-", BUILD))
            });
            let mut state = self::state().lock().unwrap();
            state.busy = false;
            state.message = match result {
                Ok(Some(update)) => {
                    let message = format!(
                        "Version {} · Build {} verfügbar.",
                        update.version, update.build
                    );
                    state.available = Some(update);
                    message
                }
                Ok(None) => "Du verwendest die aktuelle portable Version.".into(),
                Err(error) => error.to_string(),
            };
            ctx.request_repaint();
        });
    }
    if !state.message.is_empty() {
        ui.label(&state.message);
    }
}

#[path = "../../../packaging/package_footer.rs"]
mod package_footer;
