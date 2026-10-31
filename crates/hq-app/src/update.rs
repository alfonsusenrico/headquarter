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
/// The phone app's asset in every release (`build-android.yml`).
const PHONE_APP_ASSET: &str = "headquarter-android.apk";
const REPO_API: &str = "https://api.github.com/repos/alfonsusenrico/headquarter";
/// How often a running app looks for a newer release (owner, 2026-10-24).
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// How often the installer's output is read while it runs.
pub const PROGRESS_POLL: Duration = Duration::from_millis(200);

/// Where a phone downloads the newest stable phone app: GitHub sends `latest` to the
/// latest release, which leaves out pre-releases and drafts (add-phone-app-download D1).
pub fn phone_app_link() -> String {
    format!("{REPO_PAGE}/releases/latest/download/{PHONE_APP_ASSET}")
}

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

    /// This app's own version. A pre-release build (0.1.14-rc.1) counts as the release it
    /// leads to, so it runs, and that release is not offered to it as an update.
    pub fn running() -> Version {
        Version::of_build(env!("CARGO_PKG_VERSION"))
            .expect("the crate version starts with major.minor.patch")
    }

    /// The release a build's version leads to: `0.1.14` from `0.1.14` or `0.1.14-rc.1`.
    fn of_build(text: &str) -> Option<Version> {
        Version::parse(text.split_once('-').map_or(text, |(release, _)| release))
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

/// One release's notes, as the What's new page shows them (add-update-notes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseNotes {
    pub version: Version,
    /// When it was published, as `26 Oct 2026`.
    pub date: Option<String>,
    pub changes: Vec<Change>,
}

/// One changelog entry: its first sentence, shown as the headline, and the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub lead: String,
    pub rest: String,
}

/// The notes of every release after `installed` up to `latest`, newest first, from
/// one request for the recent releases.
pub fn notes_since(installed: Version, latest: Version) -> Result<Vec<ReleaseNotes>> {
    let releases = curl(&["-fsSL", &format!("{}/releases?per_page=30", api_base())])
        .context("couldn't read the release notes")?;
    releases_between(&releases, installed, latest)
        .ok_or_else(|| anyhow!("GitHub's answer had no releases"))
}

/// Published releases newer than `installed` and not past `latest`, newest first.
fn releases_between(json: &[u8], installed: Version, latest: Version) -> Option<Vec<ReleaseNotes>> {
    let releases: serde_json::Value = serde_json::from_slice(json).ok()?;
    let mut found: Vec<ReleaseNotes> = releases
        .as_array()?
        .iter()
        .filter(|release| release["draft"] != true && release["prerelease"] != true)
        .filter_map(|release| {
            let version = Version::parse(release["tag_name"].as_str()?)?;
            (installed < version && version <= latest).then(|| ReleaseNotes {
                version,
                date: release["published_at"].as_str().and_then(display_date),
                changes: changes_in(release["body"].as_str().unwrap_or_default()),
            })
        })
        .collect();
    found.sort_by_key(|release| std::cmp::Reverse(release.version));
    Some(found)
}

/// The changelog entries in a release's notes: the "Release Notes" section the
/// release workflow copies from `CHANGELOG.md`, up to the download tables. Entries are
/// hard-wrapped with an indent; their lines are joined again.
fn changes_in(body: &str) -> Vec<Change> {
    let mut entries: Vec<String> = Vec::new();
    let mut inside = false;
    for line in body.lines() {
        if line.trim() == "## Release Notes" {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with("# ") || line.starts_with("## ") {
            break;
        }
        if let Some(entry) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            entries.push(entry.trim().to_owned());
        } else if line.starts_with(' ')
            && !line.trim().is_empty()
            && let Some(last) = entries.last_mut()
        {
            last.push(' ');
            last.push_str(line.trim());
        }
    }
    entries.iter().map(|entry| split_lead(entry)).collect()
}

/// The first sentence, ending at a full stop or colon followed by a space, is the
/// headline; an entry of one sentence is all headline.
fn split_lead(entry: &str) -> Change {
    let end = entry
        .char_indices()
        .find(|&(index, c)| matches!(c, '.' | ':') && entry[index + 1..].starts_with(' '))
        .map(|(index, _)| index + 1);
    match end {
        Some(end) => Change {
            lead: entry[..end].to_owned(),
            rest: entry[end..].trim().to_owned(),
        },
        None => Change {
            lead: entry.to_owned(),
            rest: String::new(),
        },
    }
}

/// `2026-10-26T01:16:17Z` as `26 Oct 2026`.
fn display_date(published: &str) -> Option<String> {
    let date = chrono::NaiveDate::parse_from_str(published.get(..10)?, "%Y-%m-%d").ok()?;
    Some(date.format("%-d %b %Y").to_string())
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

/// The step to show while the installer runs: its own latest report, and Download until
/// it reports one. The app's fetch and check of the installer belong to Download too;
/// showing Verify for that check made the steps jump back to Download as soon as the
/// installer began its own downloads (fix-update-progress).
pub fn shown_step(output: &str) -> Step {
    step_in(output).unwrap_or(Step::Download)
}

/// A line of the update's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub text: String,
    /// Something finished, such as a file that matched its checksum.
    pub done: bool,
}

impl LogLine {
    pub fn doing(text: impl Into<String>) -> LogLine {
        LogLine {
            text: text.into(),
            done: false,
        }
    }

    pub fn done(text: impl Into<String>) -> LogLine {
        LogLine {
            text: text.into(),
            done: true,
        }
    }
}

/// The installer's notes for the log (`note [ok] <text>`), in order; its other output,
/// written for someone at a terminal, is left out.
pub fn notes_in(output: &str) -> Vec<LogLine> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("note "))
        .map(|note| match note.strip_prefix("ok ") {
            Some(text) => LogLine::done(text),
            None => LogLine::doing(note),
        })
        .collect()
}

/// A digest as its first eight and last four characters.
pub fn short_digest(digest: &str) -> String {
    match (
        digest.get(..8),
        digest.get(digest.len().saturating_sub(4)..),
    ) {
        (Some(head), Some(tail)) if digest.len() > 12 => format!("{head}…{tail}"),
        _ => digest.to_owned(),
    }
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
    /// The installer's SHA-256 as GitHub publishes it, which it matched.
    pub digest: String,
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
    Ok(Run {
        log,
        child,
        digest: expected,
    })
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

/// The sidebar sessions a restart would interrupt: live and working, waiting for the
/// owner, running background work or holding a plan to review (D5, fix-false-done D4).
pub fn busy(sessions: &[SessionInfo]) -> Vec<&SessionInfo> {
    sessions
        .iter()
        .filter(|session| {
            session.in_sidebar
                && session.live
                && (matches!(session.status, Status::Working | Status::NeedsYou)
                    || hq_protocol::sidebar::beside(session).is_some())
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

    const RELEASES: &str = r###"[
      {"tag_name": "v0.1.12", "draft": false, "prerelease": false, "published_at": "2026-10-26T03:00:00Z",
       "body": "## Release Notes\n\n- Links open with a click. Hover a web address and it\n  underlines; click and it opens.\n- Working in a session's shell panel counts as watching it: no unread\n  flag.\n\n# hq-app 0.1.12\n\n## Download hq-app 0.1.12\n\n- not a change"},
      {"tag_name": "v0.1.13", "draft": true, "prerelease": false, "body": ""},
      {"tag_name": "v0.1.11", "draft": false, "prerelease": false, "published_at": "2026-10-26T01:16:17Z",
       "body": "## Release Notes\n\n- Shift+Tab now reaches your agent\n"},
      {"tag_name": "v0.2.0-rc1", "draft": false, "prerelease": true, "body": ""},
      {"tag_name": "v0.1.10", "draft": false, "prerelease": false, "body": "## Release Notes\n\n- Old.\n"}
    ]"###;

    #[test]
    fn notes_list_the_releases_after_the_installed_one_newest_first() {
        let installed = Version::parse("0.1.10").unwrap();
        let latest = Version::parse("0.1.12").unwrap();
        let notes = releases_between(RELEASES.as_bytes(), installed, latest).unwrap();
        let versions: Vec<String> = notes.iter().map(|n| n.version.to_string()).collect();
        assert_eq!(
            versions,
            ["0.1.12", "0.1.11"],
            "drafts, pre-releases and older left out"
        );
        assert_eq!(notes[0].date.as_deref(), Some("26 Oct 2026"));
        assert_eq!(
            notes[0].changes,
            [
                Change {
                    lead: "Links open with a click.".into(),
                    rest: "Hover a web address and it underlines; click and it opens.".into(),
                },
                Change {
                    lead: "Working in a session's shell panel counts as watching it:".into(),
                    rest: "no unread flag.".into(),
                },
            ],
            "wrapped lines joined; the download tables left out"
        );
        assert_eq!(
            notes[1].changes,
            [Change {
                lead: "Shift+Tab now reaches your agent".into(),
                rest: String::new(),
            }],
            "one sentence is all headline"
        );
    }

    #[test]
    fn notes_without_the_section_have_no_changes() {
        assert!(changes_in("Some text\n- a bullet").is_empty());
        assert_eq!(
            releases_between(b"{}", Version(0, 1, 0), Version(0, 2, 0)),
            None
        );
    }

    #[test]
    fn a_version_number_does_not_end_the_headline() {
        let change = split_lead("Restart onto 0.1.12 to finish. Then carry on.");
        assert_eq!(change.lead, "Restart onto 0.1.12 to finish.");
        assert_eq!(change.rest, "Then carry on.");
    }

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
        // A build's own version may be a pre-release: it counts as the release it leads to.
        assert_eq!(Version::of_build("0.1.14-rc.1"), Some(Version(0, 1, 14)));
        assert_eq!(Version::of_build("0.1.14"), Some(Version(0, 1, 14)));
        assert_eq!(Version::of_build("rc.1"), None);
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
    fn the_steps_only_move_forward_from_download() {
        // As the installer's output grows: before its first line, then line by line.
        let output = "progress download\nnote Downloading hq-cli-x.tar.gz\nprogress verify\nprogress install\nprogress done 0.1.12\n";
        let mut seen = vec![shown_step("")];
        for end in output.match_indices('\n').map(|(index, _)| index + 1) {
            seen.push(shown_step(&output[..end]));
        }
        assert_eq!(
            seen[0],
            Step::Download,
            "the app's own check of the installer"
        );
        let order = |step: &Step| Step::ALL.iter().position(|each| each == step);
        assert!(
            seen.windows(2)
                .all(|pair| order(&pair[0]) <= order(&pair[1])),
            "{seen:?}"
        );
        assert_eq!(seen.last(), Some(&Step::Restart));
    }

    #[test]
    fn notes_are_the_log_and_terminal_advice_is_not() {
        let output = "progress download\nnote Downloading hq-cli-x.tar.gz\nnote ok hq-cli-x.tar.gz matches its checksum\nInstalled Headquarter 0.1.12.\nIf an older Headquarter is still running, quit it and run: hq stop\n";
        assert_eq!(
            notes_in(output),
            [
                LogLine::doing("Downloading hq-cli-x.tar.gz"),
                LogLine::done("hq-cli-x.tar.gz matches its checksum"),
            ]
        );
        assert_eq!(
            short_digest("03064c5d51b86a83b9e0cceab055ef17b5b192bc73d84ff079ade6705509a91a"),
            "03064c5d…a91a"
        );
        assert_eq!(short_digest("abc"), "abc");
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
            phone: None,
            background: Default::default(),
            review: false,
        }
    }

    #[test]
    fn working_asking_background_and_review_sessions_are_busy() {
        let mut background = session("background", Status::DoneUnread, true, true);
        background.background.commands = 1;
        let mut review = session("review", Status::Done, true, true);
        review.review = true;
        let mut stopped_background = session("stopped", Status::Inactive, false, true);
        stopped_background.background.commands = 1;
        let sessions = [
            session("working", Status::Working, true, true),
            session("asking", Status::NeedsYou, true, true),
            session("idle", Status::Done, true, true),
            session("inactive", Status::Inactive, false, true),
            session("elsewhere", Status::Working, true, false),
            background,
            review,
            stopped_background,
        ];
        let busy: Vec<&str> = busy(&sessions)
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert_eq!(busy, ["working", "asking", "background", "review"]);
    }

    #[test]
    fn the_phone_app_link_names_the_latest_stable_apk() {
        assert_eq!(
            phone_app_link(),
            "https://github.com/alfonsusenrico/headquarter/releases/latest/download/headquarter-android.apk"
        );
    }
}
