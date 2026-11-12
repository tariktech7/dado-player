use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

pub const VERSION: &str = include_str!("../../../windows/VERSION");
const REPO: &str = "https://github.com/tariktech7/dado-player.git";
const PREFIX: &str = "refs/tags/portable-v";

#[derive(Clone, Debug)]
struct Release {
    tag: String,
    commit: String,
}
#[derive(Default, Clone)]
enum State {
    #[default]
    Idle,
    Busy,
    Message(String),
    Available(Release),
    Ready,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn set(value: State) {
    *STATE.get_or_init(Default::default).lock().unwrap() = value;
}

fn git() -> Command {
    let bundled = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("Programs/WorkBuddyAI/resources/vendor/PortableGit/cmd/git.exe"));
    let standard = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .map(|p| p.join("Git/cmd/git.exe"));
    let exe = standard
        .into_iter()
        .chain(bundled)
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("git.exe"));
    let mut c = Command::new(exe);
    c.creation_flags(0x08000000)
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("-c")
        .arg("fetch.fsckObjects=true");
    c
}
fn execute(c: &mut Command) -> anyhow::Result<String> {
    let result = c
        .output()
        .map_err(|_| anyhow::anyhow!("Git für Windows konnte nicht gestartet werden."))?;
    anyhow::ensure!(result.status.success(), "GitHub-Abruf fehlgeschlagen. Internetverbindung und Git-Anmeldung für tariktech7/dado-player prüfen.");
    Ok(String::from_utf8_lossy(&result.stdout).into_owned())
}
fn version(s: &str) -> Option<[u32; 3]> {
    let mut parts = s.split('.');
    let v = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(v)
}
fn check(repo: &str, current: [u32; 3]) -> anyhow::Result<State> {
    let refs = execute(git().args(["ls-remote", "--refs", repo, "refs/tags/portable-v*"]))?;
    let latest = refs
        .lines()
        .filter_map(|line| {
            let (id, name) = line.split_once('\t')?;
            let tag = name.strip_prefix(PREFIX)?;
            let v = version(tag)?;
            (v > current && id.len() == 40 && id.bytes().all(|x| x.is_ascii_hexdigit())).then(
                || {
                    (
                        v,
                        Release {
                            tag: tag.to_owned(),
                            commit: id.to_owned(),
                        },
                    )
                },
            )
        })
        .max_by_key(|(v, _)| *v);
    Ok(latest
        .map(|(_, r)| State::Available(r))
        .unwrap_or_else(|| State::Message("Du verwendest die aktuelle portable Version.".into())))
}
fn download(repo: &str, dir: &Path, release: &Release) -> anyhow::Result<State> {
    let cache = dir.join("repository");
    execute(git().args(["init", "--bare", "--quiet"]).arg(&cache))?;
    execute(git().arg("-C").arg(&cache).args([
        "fetch",
        "--quiet",
        "--no-tags",
        "--depth=1",
        repo,
        &format!("{PREFIX}{}", release.tag),
    ]))?;
    let fetched = execute(
        git()
            .arg("-C")
            .arg(&cache)
            .args(["rev-parse", "FETCH_HEAD"]),
    )?;
    anyhow::ensure!(
        fetched.trim() == release.commit,
        "Das Release wurde geändert. Bitte erneut prüfen."
    );
    let object = "FETCH_HEAD:Dado-Player.exe";
    let size: u64 = execute(git().arg("-C").arg(&cache).args(["cat-file", "-s", object]))?
        .trim()
        .parse()?;
    anyhow::ensure!((64..=104_857_600).contains(&size), "Ungültige Paketgröße.");
    let partial = dir.join("download.partial");
    let status = git()
        .arg("-C")
        .arg(&cache)
        .args(["cat-file", "blob", object])
        .stdout(Stdio::from(std::fs::File::create(&partial)?))
        .status()?;
    anyhow::ensure!(
        status.success() && partial.metadata()?.len() == size,
        "Download unvollständig."
    );
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(&partial)?;
    let mut magic = [0; 8];
    file.read_exact(&mut magic[..2])?;
    anyhow::ensure!(&magic[..2] == b"MZ", "Ungültige Windows-EXE.");
    file.seek(SeekFrom::End(-64))?;
    file.read_exact(&mut magic)?;
    anyhow::ensure!(
        &magic == b"DADOPK01",
        "Das Release enthält kein Dado-Portable-Paket."
    );
    drop(file);
    std::fs::rename(partial, dir.join("verified.exe"))?;
    Ok(State::Ready)
}
fn background(ctx: egui::Context, job: impl FnOnce() -> anyhow::Result<State> + Send + 'static) {
    set(State::Busy);
    std::thread::spawn(move || {
        set(job().unwrap_or_else(|e| State::Message(e.to_string())));
        ctx.request_repaint();
    });
}
pub fn show(ui: &mut egui::Ui) {
    let Some(dir) = std::env::var_os("DADO_UPDATE_DIR").map(PathBuf::from) else {
        return;
    };
    let state = STATE.get_or_init(Default::default).lock().unwrap().clone();
    match state {
        State::Busy => {
            ui.spinner();
            ui.label("GitHub wird abgefragt …");
        }
        State::Available(r) => {
            if ui
                .button(format!("Version {} herunterladen", r.tag))
                .clicked()
            {
                background(ui.ctx().clone(), move || download(REPO, &dir, &r));
            }
        }
        State::Ready => {
            ui.label(
                "Update bereit. Beim Beenden wird die portable EXE ersetzt und neu gestartet.",
            );
            if ui.button("Jetzt neu starten").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        State::Idle | State::Message(_) => {
            if let State::Message(s) = state {
                ui.label(s);
            }
            if ui.button("Auf GitHub-Updates prüfen").clicked() {
                background(ui.ctx().clone(), || {
                    check(REPO, version(VERSION.trim()).unwrap())
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Exercise Git authentication-independent transport, version ordering and the
    // exact-object check using a local release repository.
    #[test]
    fn fetches_the_selected_portable_release() -> anyhow::Result<()> {
        let root =
            std::env::temp_dir().join(format!("dado-git-update-test-{}", std::process::id()));
        std::fs::create_dir(&root)?;
        let result = exercise(&root);
        std::fs::remove_dir_all(&root)?;
        result
    }
    fn exercise(root: &Path) -> anyhow::Result<()> {
        let repo = root.join("source");
        execute(git().arg("init").arg(&repo))?;
        let mut package = vec![0u8; 128];
        package[..2].copy_from_slice(b"MZ");
        package[64..72].copy_from_slice(b"DADOPK01");
        std::fs::write(repo.join("Dado-Player.exe"), &package)?;
        execute(
            git()
                .arg("-C")
                .arg(&repo)
                .args(["add", "-f", "Dado-Player.exe"]),
        )?;
        execute(git().arg("-C").arg(&repo).args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@localhost",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "test",
        ]))?;
        for tag in ["portable-v0.9.0", "portable-v0.10.0"] {
            execute(
                git()
                    .arg("-C")
                    .arg(&repo)
                    .args(["-c", "tag.gpgSign=false", "tag", tag]),
            )?;
        }
        let url = repo.to_str().unwrap();
        let State::Available(r) = check(url, [0, 8, 0])? else {
            anyhow::bail!("missing release")
        };
        assert_eq!(r.tag, "0.10.0");
        assert!(matches!(check(url, [0, 10, 0])?, State::Message(_)));
        let stage = root.join("stage");
        std::fs::create_dir(&stage)?;
        assert!(download(
            url,
            &stage,
            &Release {
                commit: "0".repeat(40),
                ..r.clone()
            }
        )
        .is_err());
        assert!(!stage.join("verified.exe").exists());
        assert!(matches!(download(url, &stage, &r)?, State::Ready));
        assert_eq!(std::fs::read(stage.join("verified.exe"))?, package);
        Ok(())
    }
}
