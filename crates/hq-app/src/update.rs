//! Updates from this repository's GitHub releases (add-in-app-update): finding a newer
//! release, telling whether this copy came from the installer, and running that
//! release's own installer after checking it against the digest GitHub publishes. Talks
//! to GitHub through `curl`, which the installer needs anyway. Window code lives in
//! `workspace/update.rs`.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use hq_protocol::SessionInfo;
use hq_signals::status::Status;
use sha2::{Digest, Sha256};

const REPO_PAGE: &str = "https://github.com/alfonsusenrico/headquarter";
const REPO_API: &str = "https://api.github.com/repos/alfonsusenrico/headquarter";
/// How often a running app looks for a newer release (owner, 2026-10-24).
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// How often the installer's output is read while it runs.
pub const PROGRESS_POLL: Duration = Duration::from_millis(200);

/// A release version, `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u64, u64, u64);

impl Version {
    /// Reads `0.1.9` or `v0.1.9`; anything else, such as a pre-release suffix, is no
    /// version.
    pub fn parse(text: &str) -> Option<Version> {
        let mut parts = text.trim().trim_start_matches('v').split('.');
        let version = Version(
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
        );
        parts.next().is_none().then_some(version)
    }

    /// This app's own version.
    pub fn running() -> Version {
        Version::parse(env!("CARGO_PKG_VERSION")).expect("the crate version is major.minor.patch")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// The release page, or a local stand-in for tests and dev runs (D9).
fn page_base() -> String {
    std::env::var("HQ_UPDATE_BASE")
        .ok()
        .filter(|base| !base.is_empty())
        .unwrap_or_else(|| REPO_PAGE.to_owned())
}

fn api_base() -> String {
    std::env::var("HQ_UPDATE_BASE")
        .ok()
        .filter(|base| !base.is_empty())
        .map_or_else(|| REPO_API.to_owned(), |base| format!("{base}/api"))
}

/// The page of release `version`, for copies the updater does not install.
pub fn release_page(version: Version) -> String {
    format!("{}/releases/tag/v{version}", page_base())
}

/// The newest release: GitHub answers `/releases/latest` with a redirect to its tag's
/// page, which needs no API call and so no rate limit (D1).
pub fn latest() -> Result<Version> {
    let output = curl(&[
        "-fsSI",
        "-o",
        "/dev/null",
        "-w",
        "%{redirect_url}",
        &format!("{}/releases/latest", page_base()),
    ])?;
    tag_in_redirect(&String::from_utf8_lossy(&output))
        .ok_or_else(|| anyhow!("GitHub did not name the latest release"))
}

/// The version named by a redirect such as `…/releases/tag/v0.1.9`.
fn tag_in_redirect(location: &str) -> Option<Version> {
    let (_, tag) = location.trim().rsplit_once("/releases/tag/")?;
    Version::parse(tag)
}

/// Where the installer put this copy, when it did: the folder holding `hq`, and what to
/// open to start the app again (D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub bin_dir: PathBuf,
    pub app: PathBuf,
}

/// This copy, when the installer's manifest lists it.
pub fn installed_copy() -> Option<Installed> {
    let home = hq_signals::paths::home()?;
    // The installer always writes here, whatever `HQ_DATA_DIR` says.
    let manifest =
        std::fs::read_to_string(home.join(".local/share/headquarter/install-manifest")).ok()?;
    let executable = std::env::current_exe().ok()?;
    installed_from(&manifest, &executable)
}

fn installed_from(manifest: &str, executable: &Path) -> Option<Installed> {
    let listed: Vec<&Path> = manifest.lines().map(Path::new).collect();
    let bin_dir = listed
        .iter()
        .find(|path| path.file_name().is_some_and(|name| name == "hq"))?
        .parent()?
        .to_path_buf();
    // On macOS the bundle is listed; on Linux the executable itself.
    let app = executable
        .ancestors()
        .find(|path| listed.contains(path))?
        .to_path_buf();
    Some(Installed { bin_dir, app })
}

/// A step of an update, as the window lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Download,
    Verify,
    Install,
    Restart,
}

impl Step {
    pub const ALL: [Step; 4] = [Step::Download, Step::Verify, Step::Install, Step::Restart];

    pub fn label(self) -> &'static str {
        match self {
            Step::Download => "Download",
            Step::Verify => "Verify",
            Step::Install => "Install",
            Step::Restart => "Restart",
        }
    }
}

/// The step the installer's output has reached: its last `progress` line (D4). `done`
/// means the install finished, so the restart is next.
pub fn step_in(output: &str) -> Option<Step> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("progress "))
        .next_back()
        .and_then(|step| match step.split_whitespace().next()? {
            "download" => Some(Step::Download),
            "verify" => Some(Step::Verify),
            "install" => Some(Step::Install),
            "done" => Some(Step::Restart),
            _ => None,
        })
}

/// Why the installer failed: its last error line, else its last line.
pub fn failure_in(output: &str) -> String {
    let lines: Vec<&str> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    lines
        .iter()
        .rev()
        .find_map(|line| line.strip_prefix("headquarter install: "))
        .or_else(|| lines.last().copied())
        .unwrap_or("the installer stopped without saying why")
        .to_owned()
}

/// A running installer: its output file and process.
pub struct Run {
    pub log: PathBuf,
    pub child: std::process::Child,
}

/// Downloads release `version`'s installer, checks it against the digest GitHub
/// publishes for it, and starts it in a process group of its own writing to a file, so
/// quitting the app does not cut it off (D3).
pub fn start_install(version: Version, installed: &Installed) -> Result<Run> {
    let folder = std::env::temp_dir().join(format!(
        "headquarter-update-{}-{version}",
        std::process::id()
    ));
    std::fs::create_dir_all(&folder).context("cannot make a folder for the download")?;
    let started = run_installer(&folder, version, installed);
    if started.is_err() {
        // Nothing ran; the download is of no further use.
        let _ = std::fs::remove_dir_all(&folder);
    }
    started
}

fn run_installer(folder: &Path, version: Version, installed: &Installed) -> Result<Run> {
    let script = folder.join("install.sh");
    curl(&[
        "-fsSL",
        "--retry",
        "2",
        "-o",
        &script.to_string_lossy(),
        &format!("{}/releases/download/v{version}/install.sh", page_base()),
    ])
    .context("couldn't download the installer")?;
    let release = curl(&["-fsSL", &format!("{}/releases/tags/v{version}", api_base())])
        .context("couldn't read the release's checksums")?;
    let expected = installer_digest(&release)
        .ok_or_else(|| anyhow!("the release publishes no checksum for its installer"))?;
    let bytes = std::fs::read(&script).context("cannot read the downloaded installer")?;
    if sha256_hex(&bytes) != expected {
        bail!("the installer did not match its published checksum; nothing was installed");
    }
    let log = folder.join("install.log");
    let output = std::fs::File::create(&log).context("cannot write the installer's log")?;
    let errors = output.try_clone()?;
    let mut command = Command::new("sh");
    command
        .arg(&script)
        .env("HQ_VERSION", version.to_string())
        .env("HQ_INSTALL_DIR", &installed.bin_dir)
        .env("HQ_PROGRESS", "1")
        .stdin(Stdio::null())
        .stdout(output)
        .stderr(errors);
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let child = command.spawn().context("cannot start the installer")?;
    Ok(Run { log, child })
}

/// The SHA-256 GitHub publishes for the release's `install.sh`, from the release API's
/// asset list (`"digest": "sha256:…"`).
fn installer_digest(release_json: &[u8]) -> Option<String> {
    let release: serde_json::Value = serde_json::from_slice(release_json).ok()?;
    release["assets"]
        .as_array()?
        .iter()
        .find(|asset| asset["name"] == "install.sh")?["digest"]
        .as_str()?
        .strip_prefix("sha256:")
        .map(str::to_ascii_lowercase)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl Run {
    /// Removes the downloaded installer and its log once it has exited.
    pub fn clean_up(&self) {
        if let Some(folder) = self.log.parent() {
            // A folder left behind is in the system's temporary folder, which it clears.
            let _ = std::fs::remove_dir_all(folder);
        }
    }
}

/// Reads what the installer has written so far.
pub fn read_log(path: &Path) -> String {
    let mut text = String::new();
    if let Ok(mut file) = std::fs::File::open(path) {
        // A partial read only delays the next step's display.
        let _ = file.read_to_string(&mut text);
    }
    text
}

fn curl(args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("curl")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("curl is needed to reach GitHub")?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(if message.is_empty() {
            format!("curl exited with {}", output.status)
        } else {
            message
        });
    }
    Ok(output.stdout)
}

/// The sidebar sessions a restart would interrupt: live and working (background work
/// included) or waiting for the owner (D5).
pub fn busy(sessions: &[SessionInfo]) -> Vec<&SessionInfo> {
    sessions
        .iter()
        .filter(|session| {
            session.in_sidebar
                && session.live
                && matches!(session.status, Status::Working | Status::NeedsYou)
        })
        .collect()
}

/// Opens the new copy once this process has ended: a detached `sh` that waits for this
/// process id to go, then opens the bundle (macOS) or runs the app (Linux) (D6).
pub fn relaunch_after_exit(app: &Path) -> Result<()> {
    let open = if cfg!(target_os = "macos") {
        "open \"$1\"".to_owned()
    } else {
        "\"$1\" >/dev/null 2>&1 &".to_owned()
    };
    let script = format!(
        "while kill -0 {} 2>/dev/null; do sleep 0.2; done; {open}",
        std::process::id()
    );
    let mut command = Command::new("sh");
    command
        .args(["-c", &script, "relaunch"])
        .arg(app)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command
        .spawn()
        .map(drop)
        .context("cannot schedule the app's relaunch")
}

#[cfg(test)]
mod tests {
    use hq_protocol::Origin;
    use hq_signals::event::Agent;

    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert_eq!(Version::parse("v0.1.9"), Some(Version(0, 1, 9)));
        assert!(Version::parse("0.1.10") > Version::parse("0.1.9"));
        assert!(Version::parse("1.0.0") > Version::parse("0.9.99"));
        assert_eq!(
            Version::parse("0.2.0-beta.1"),
            None,
            "pre-releases are skipped"
        );
        assert_eq!(Version::parse("0.2"), None);
        assert_eq!(Version::parse("0.1.9.1"), None);
        assert_eq!(Version(0, 1, 9).to_string(), "0.1.9");
        assert!(Version::parse(&Version::running().to_string()).is_some());
    }

    #[test]
    fn the_latest_release_is_read_from_its_redirect() {
        assert_eq!(
            tag_in_redirect("https://github.com/alfonsusenrico/headquarter/releases/tag/v0.1.9\n"),
            Some(Version(0, 1, 9))
        );
        assert_eq!(tag_in_redirect(""), None, "no redirect");
        assert_eq!(
            tag_in_redirect("https://github.com/alfonsusenrico/headquarter/releases"),
            None
        );
    }

    #[test]
    fn only_a_copy_the_installer_listed_is_updated_in_place() {
        let manifest = "/u/.local/bin/hq\n/u/.local/bin/headquarter\n/u/.local/share/headquarter/signing.keychain-db\n/u/Applications/Headquarter.app\n";
        assert_eq!(
            installed_from(
                manifest,
                Path::new("/u/Applications/Headquarter.app/Contents/MacOS/headquarter-app")
            ),
            Some(Installed {
                bin_dir: PathBuf::from("/u/.local/bin"),
                app: PathBuf::from("/u/Applications/Headquarter.app"),
            })
        );
        let linux = "/u/bin/hq\n/u/bin/headquarter\n/u/bin/headquarter-app\n";
        assert_eq!(
            installed_from(linux, Path::new("/u/bin/headquarter-app")),
            Some(Installed {
                bin_dir: PathBuf::from("/u/bin"),
                app: PathBuf::from("/u/bin/headquarter-app"),
            })
        );
        assert_eq!(
            installed_from(manifest, Path::new("/src/target/debug/headquarter-app")),
            None,
            "a dev build"
        );
    }

    #[test]
    fn the_installer_digest_is_read_from_the_release() {
        let release = br#"{"tag_name":"v0.1.9","assets":[{"name":"sha256.sum","digest":"sha256:01"},{"name":"install.sh","digest":"sha256:20CE73"}]}"#;
        assert_eq!(installer_digest(release).as_deref(), Some("20ce73"));
        assert_eq!(
            installer_digest(br#"{"assets":[{"name":"install.sh"}]}"#),
            None,
            "no digest published"
        );
        assert_eq!(installer_digest(b"not json"), None);
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn progress_and_failures_are_read_from_the_installer_output() {
        assert_eq!(step_in(""), None);
        assert_eq!(step_in("progress download\n"), Some(Step::Download));
        assert_eq!(
            step_in("progress download\nprogress verify\nprogress install\n"),
            Some(Step::Install)
        );
        assert_eq!(
            step_in("progress install\nprogress done 0.1.9\nInstalled Headquarter 0.1.9.\n"),
            Some(Step::Restart)
        );
        assert_eq!(
            failure_in("progress download\nheadquarter install: cannot download x\n"),
            "cannot download x"
        );
        assert_eq!(failure_in("something odd\n"), "something odd");
        assert_eq!(failure_in(""), "the installer stopped without saying why");
    }

    fn session(id: &str, status: Status, live: bool, in_sidebar: bool) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: "/w".to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status,
            needs_reason: None,
            created_at: 0,
            last_activity_at: 0,
            ended_at: None,
            live,
            origin: Origin::Hosted,
            title: None,
            external: None,
            in_sidebar,
            group: None,
            owner_named: true,
            starting: false,
            start_error: None,
            offered: true,
        }
    }

    #[test]
    fn working_or_asking_sessions_are_busy() {
        let sessions = [
            session("working", Status::Working, true, true),
            session("asking", Status::NeedsYou, true, true),
            session("idle", Status::Done, true, true),
            session("inactive", Status::Inactive, false, true),
            session("elsewhere", Status::Working, true, false),
        ];
        let busy: Vec<&str> = busy(&sessions)
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert_eq!(busy, ["working", "asking"]);
    }
}
