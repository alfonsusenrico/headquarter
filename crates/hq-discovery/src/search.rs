//! Finding agent executables: settings pin, process `PATH`, login-shell `PATH`, then
//! the platform's usual install folders. Every match is reported; the first is used.

use std::collections::HashSet;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::agents::KnownAgent;
use crate::platform::{Os, Platform};

pub const VERSION_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Settings,
    ProcessPath,
    LoginShell,
    KnownFolder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Found {
    pub path: PathBuf,
    /// Where a symlink points, when `path` is one.
    pub target: Option<PathBuf>,
    pub source: Source,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReport {
    pub agent: &'static KnownAgent,
    pub found: Vec<Found>,
    /// Index into `found` of the executable that would be used.
    pub used: Option<usize>,
    pub searched: Vec<PathBuf>,
    pub version: Option<String>,
    /// Set when the settings file pins a path that is not an executable.
    pub pin_problem: Option<String>,
}

impl AgentReport {
    pub fn used_path(&self) -> Option<&Path> {
        self.used.map(|index| self.found[index].path.as_path())
    }
}

/// What the search may look at; built once and shared.
#[derive(Debug, Clone)]
pub struct SearchContext {
    pub platform: Platform,
    pub home: Option<PathBuf>,
    pub process_path: Option<OsString>,
    pub login_path: Option<String>,
    /// Replaces the system-wide folders (Homebrew's, `/usr/local/bin`, Linux's system
    /// folders) when set, from `HQ_SYSTEM_BIN_DIRS`: tests must not find, or run, what
    /// the machine has installed. Empty searches none.
    pub system_folders: Option<Vec<PathBuf>>,
}

impl SearchContext {
    pub fn current(platform: Platform, login_path: Option<String>) -> SearchContext {
        SearchContext {
            platform,
            home: std::env::var_os("HOME")
                .filter(|home| !home.is_empty())
                .map(PathBuf::from),
            process_path: std::env::var_os("PATH"),
            login_path,
            system_folders: std::env::var_os("HQ_SYSTEM_BIN_DIRS").map(|dirs| {
                std::env::split_paths(&dirs)
                    .filter(|dir| !dir.as_os_str().is_empty())
                    .collect()
            }),
        }
    }

    /// The platform's usual install folders, in search order.
    pub fn known_folders(&self) -> Vec<PathBuf> {
        let mut folders = Vec::new();
        if let Some(home) = &self.home {
            for relative in [
                ".local/bin",
                ".cargo/bin",
                ".bun/bin",
                ".volta/bin",
                ".npm-global/bin",
                ".local/share/pnpm",
                ".local/share/mise/shims",
                ".asdf/shims",
            ] {
                folders.push(home.join(relative));
            }
            if self.platform.os == Os::Macos {
                folders.push(home.join("Library/pnpm"));
            }
            if let Some(node) = newest_nvm_bin(&home.join(".nvm/versions/node")) {
                folders.push(node);
            }
        }
        if self.platform.os == Os::Linux
            && let Some(home) = &self.home
        {
            folders.push(home.join(".local/share/flatpak/exports/bin"));
        }
        match &self.system_folders {
            Some(system) => folders.extend(system.iter().cloned()),
            None => folders.extend(self.platform_folders()),
        }
        folders
    }

    /// The system-wide install folders of this platform.
    fn platform_folders(&self) -> Vec<PathBuf> {
        let mut folders = Vec::new();
        if let Some(prefix) = &self.platform.homebrew {
            folders.push(prefix.join("bin"));
        }
        folders.push(PathBuf::from("/usr/local/bin"));
        if self.platform.os == Os::Linux {
            folders.push(PathBuf::from("/usr/bin"));
            folders.push(PathBuf::from("/snap/bin"));
            folders.push(PathBuf::from("/var/lib/flatpak/exports/bin"));
        }
        folders
    }
}

/// Searches for one agent. `pinned` is the settings file's `path`, if any.
pub fn search(
    context: &SearchContext,
    agent: &'static KnownAgent,
    pinned: Option<&Path>,
) -> AgentReport {
    let mut report = AgentReport {
        agent,
        found: Vec::new(),
        used: None,
        searched: Vec::new(),
        version: None,
        pin_problem: None,
    };
    if let Some(pinned) = pinned {
        if is_executable(pinned) {
            report
                .found
                .push(found(pinned.to_owned(), Source::Settings));
        } else {
            report.pin_problem = Some(format!(
                "{} (set in the settings file) is not an executable file",
                pinned.display()
            ));
        }
    }

    let mut folders: Vec<(PathBuf, Source)> = Vec::new();
    if let Some(path) = &context.process_path {
        folders.extend(std::env::split_paths(path).map(|dir| (dir, Source::ProcessPath)));
    }
    if let Some(path) = &context.login_path {
        folders.extend(std::env::split_paths(path).map(|dir| (dir, Source::LoginShell)));
    }
    if let Some(home) = &context.home {
        folders.extend(
            agent
                .home_dirs
                .iter()
                .map(|relative| (home.join(relative), Source::KnownFolder)),
        );
    }
    folders.extend(
        context
            .known_folders()
            .into_iter()
            .map(|dir| (dir, Source::KnownFolder)),
    );

    let mut seen = HashSet::new();
    for (folder, source) in folders {
        if folder.as_os_str().is_empty() || !seen.insert(folder.clone()) {
            continue;
        }
        for name in agent.executables {
            let candidate = folder.join(name);
            if is_executable(&candidate) {
                report.found.push(found(candidate, source));
            }
        }
        report.searched.push(folder);
    }
    // A broken pin must not silently fall back to another install.
    if report.pin_problem.is_none() && !report.found.is_empty() {
        report.used = Some(0);
    }
    report
}

fn found(path: PathBuf, source: Source) -> Found {
    let target = std::fs::symlink_metadata(&path)
        .ok()
        .filter(|metadata| metadata.file_type().is_symlink())
        .and_then(|_| std::fs::canonicalize(&path).ok());
    Found {
        path,
        target,
        source,
    }
}

/// A regular file (or a symlink to one) with an execute bit.
pub fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// The `bin` folder of the newest Node version installed by nvm.
fn newest_nvm_bin(versions: &Path) -> Option<PathBuf> {
    let parse = |name: &str| -> Option<Vec<u64>> {
        name.trim_start_matches('v')
            .split('.')
            .map(|part| part.parse().ok())
            .collect()
    };
    std::fs::read_dir(versions)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            Some((parse(&name)?, entry.path().join("bin")))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, bin)| bin)
}

/// First line of `<executable> --version`, or `None` if it fails or takes longer than
/// `timeout`.
pub fn probe_version(executable: &Path, timeout: Duration) -> Option<String> {
    let mut child = Command::new(executable)
        .arg("--version")
        .env("PATH", crate::shell::agent_path(executable))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut output = String::new();
        let _ = std::io::Read::read_to_string(&mut stdout, &mut output);
        output
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let output = reader.join().ok()?;
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    Some(line.chars().take(80).collect())
}

/// Fills in each report's version, probing all agents at once.
pub fn add_versions(reports: &mut [AgentReport], timeout: Duration) {
    std::thread::scope(|scope| {
        for report in reports.iter_mut() {
            if let Some(path) = report.used_path().map(Path::to_owned) {
                scope.spawn(move || {
                    report.version = probe_version(&path, timeout);
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::known;
    use crate::platform::Platform;

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let root =
                std::env::temp_dir().join(format!("hq-search-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Fixture { root }
        }

        fn executable(&self, relative: &str, script: &str) -> PathBuf {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }

        fn context(&self, process_path: &str, login_path: Option<&str>) -> SearchContext {
            SearchContext {
                platform: Platform {
                    os: Os::Linux,
                    arch: "x86_64".into(),
                    distro: None,
                    wsl: false,
                    homebrew: None,
                },
                home: Some(self.root.join("home")),
                process_path: Some(process_path.into()),
                login_path: login_path.map(str::to_owned),
                system_folders: Some(vec![self.root.join("system")]),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn agent(id: &str) -> &'static KnownAgent {
        known(id).unwrap()
    }

    #[test]
    fn agent_only_in_its_installer_folder() {
        let fixture = Fixture::new("installer");
        let opencode = fixture.executable("home/.opencode/bin/opencode", "echo 1.0");
        let report = search(
            &fixture.context("/nonexistent", None),
            agent("opencode"),
            None,
        );
        assert_eq!(report.used_path(), Some(opencode.as_path()));
        assert_eq!(report.found[0].source, Source::KnownFolder);
    }

    #[test]
    fn agent_only_on_the_login_shell_path() {
        let fixture = Fixture::new("login");
        let bin = fixture.root.join("brew/bin");
        let codex = fixture.executable("brew/bin/codex", "echo codex");
        let report = search(
            &fixture.context("/nonexistent", Some(&bin.to_string_lossy())),
            agent("codex"),
            None,
        );
        assert_eq!(report.used_path(), Some(codex.as_path()));
        assert_eq!(report.found[0].source, Source::LoginShell);
    }

    #[test]
    fn two_installs_are_both_reported_and_path_order_wins() {
        let fixture = Fixture::new("two");
        let first = fixture.executable("a/claude", "echo a");
        fixture.executable("home/.local/bin/claude", "echo b");
        let report = search(
            &fixture.context(&fixture.root.join("a").to_string_lossy(), None),
            agent("claude"),
            None,
        );
        assert_eq!(report.found.len(), 2);
        assert_eq!(report.used_path(), Some(first.as_path()));
    }

    #[test]
    fn non_executables_are_ignored_and_absence_lists_the_search() {
        let fixture = Fixture::new("absent");
        let plain = fixture.root.join("home/.local/bin/agy");
        std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
        std::fs::write(&plain, "not executable").unwrap();
        let report = search(&fixture.context("/nonexistent", None), agent("agy"), None);
        assert!(report.found.is_empty());
        assert_eq!(report.used, None);
        assert!(
            report
                .searched
                .contains(&fixture.root.join("home/.local/bin"))
        );
        assert!(report.searched.contains(&fixture.root.join("system")));
    }

    #[test]
    fn linux_system_folders_are_searched_unless_replaced() {
        let fixture = Fixture::new("linux");
        let context = SearchContext {
            system_folders: None,
            ..fixture.context("/nonexistent", None)
        };
        let folders = context.known_folders();
        for system in [
            "/usr/local/bin",
            "/usr/bin",
            "/snap/bin",
            "/var/lib/flatpak/exports/bin",
        ] {
            assert!(folders.contains(&PathBuf::from(system)), "{system}");
        }
        assert!(folders.contains(&fixture.root.join("home/.local/share/flatpak/exports/bin")));
    }

    #[test]
    fn symlinks_report_their_target() {
        let fixture = Fixture::new("symlink");
        let real = fixture.executable("store/claude-2.1", "echo 2.1");
        let link = fixture.root.join("home/.local/bin/claude");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let report = search(
            &fixture.context("/nonexistent", None),
            agent("claude"),
            None,
        );
        assert_eq!(report.found[0].path, link);
        assert_eq!(
            report.found[0].target.as_deref(),
            Some(real.canonicalize().unwrap().as_path())
        );
    }

    #[test]
    fn a_pin_wins_and_a_broken_pin_does_not_fall_back() {
        let fixture = Fixture::new("pin");
        fixture.executable("home/.local/bin/agy", "echo found");
        let pinned = fixture.executable("custom/agy", "echo pinned");
        let report = search(
            &fixture.context("/nonexistent", None),
            agent("agy"),
            Some(&pinned),
        );
        assert_eq!(report.used_path(), Some(pinned.as_path()));
        assert_eq!(report.found[0].source, Source::Settings);

        let broken = search(
            &fixture.context("/nonexistent", None),
            agent("agy"),
            Some(Path::new("/no/such/agy")),
        );
        assert_eq!(broken.used, None);
        assert!(broken.pin_problem.unwrap().contains("/no/such/agy"));
    }

    #[test]
    fn newest_nvm_version_is_searched() {
        let fixture = Fixture::new("nvm");
        for version in ["v18.20.0", "v22.1.0", "v20.11.1"] {
            std::fs::create_dir_all(
                fixture
                    .root
                    .join(format!("home/.nvm/versions/node/{version}/bin")),
            )
            .unwrap();
        }
        let folders = fixture.context("/x", None).known_folders();
        assert!(folders.contains(&fixture.root.join("home/.nvm/versions/node/v22.1.0/bin")));
        assert!(!folders.contains(&fixture.root.join("home/.nvm/versions/node/v18.20.0/bin")));
    }

    #[test]
    fn system_folders_can_be_replaced_and_home_folders_stay() {
        let fixture = Fixture::new("system");
        let gemini = fixture.executable("system/gemini", "echo 0.1");
        let context = fixture.context("/nonexistent", None);
        let folders = context.known_folders();
        assert!(folders.contains(&fixture.root.join("home/.local/bin")));
        assert!(
            !folders.contains(&PathBuf::from("/usr/local/bin")),
            "the platform's folders are replaced"
        );
        let report = search(&context, agent("gemini"), None);
        assert_eq!(report.used_path(), Some(gemini.as_path()));

        let none = SearchContext {
            system_folders: Some(Vec::new()),
            ..context
        };
        assert!(
            none.known_folders()
                .iter()
                .all(|dir| dir.starts_with(&fixture.root))
        );
    }

    #[test]
    fn versions_are_probed_in_parallel_with_a_timeout() {
        let fixture = Fixture::new("versions");
        fixture.executable("bin/claude", "echo '2.1.304 (Claude Code)'");
        fixture.executable("bin/codex", "sleep 10");
        let context = fixture.context(&fixture.root.join("bin").to_string_lossy(), None);
        let mut reports = vec![
            search(&context, agent("claude"), None),
            search(&context, agent("codex"), None),
        ];
        // macOS scans a new executable on its first run, which can take most of a
        // second, so the bound is generous; the hanging probe still must not add up.
        let started = Instant::now();
        add_versions(&mut reports, Duration::from_secs(2));
        assert!(started.elapsed() < Duration::from_millis(4_500));
        assert_eq!(reports[0].version.as_deref(), Some("2.1.304 (Claude Code)"));
        assert_eq!(reports[1].version, None);
    }
}
