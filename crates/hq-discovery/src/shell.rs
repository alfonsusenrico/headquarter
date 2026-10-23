//! The owner's login-shell `PATH`, which a process started from a desktop launcher
//! does not inherit.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(3);

/// The login-shell `PATH`, once a process has learnt it (`remember_login_path`).
static LOGIN_PATH: OnceLock<String> = OnceLock::new();

/// Keeps the login-shell `PATH` for [`agent_path`]; only the first value counts.
pub fn remember_login_path(path: &str) {
    let _ = LOGIN_PATH.set(path.to_owned());
}

/// The `PATH` an agent runs with. A process started from a desktop launcher has only
/// the system folders, so a CLI written for Node (`#!/usr/bin/env node`) cannot find
/// `node`. First the agent's own folder (Homebrew and nvm put `node` beside it), then
/// the login-shell `PATH` once known, then this process's own; repeats removed.
pub fn agent_path(executable: &Path) -> OsString {
    let login = LOGIN_PATH.get().map(OsString::from);
    path_for(executable, login, std::env::var_os("PATH"))
}

fn path_for(executable: &Path, login: Option<OsString>, process: Option<OsString>) -> OsString {
    let mut folders: Vec<PathBuf> = Vec::new();
    let own = executable
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .map(Path::to_path_buf);
    let rest = [login, process]
        .into_iter()
        .flatten()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>());
    for folder in own.into_iter().chain(rest) {
        if !folder.as_os_str().is_empty() && !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    std::env::join_paths(folders).unwrap_or_default()
}

const START: &str = "__HQ_PATH__";
const END: &str = "__HQ_END__";

/// Runs `$SHELL` as an interactive login shell and returns its `PATH`, or `None` when
/// there is no shell, it fails, or it takes longer than `timeout`.
pub fn login_shell_path(timeout: Duration) -> Option<String> {
    let shell = std::env::var_os("SHELL")?;
    path_from_shell(Path::new(&shell), timeout)
}

/// Interactive, because many people set `PATH` only in `.zshrc` or `.bashrc`. The
/// markers separate the value from anything the startup files print.
pub fn path_from_shell(shell: &Path, timeout: Duration) -> Option<String> {
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c"])
        .arg(format!(r#"printf '\n{START}%s{END}\n' "$PATH""#))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout.read_to_end(&mut output);
        output
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
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
    let output = String::from_utf8_lossy(&reader.join().ok()?).into_owned();
    let start = output.rfind(START)? + START.len();
    let end = start + output[start..].find(END)?;
    let path = output[start..end].trim();
    (!path.is_empty()).then(|| path.to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_agent_runs_with_its_own_folder_first_then_the_login_path() {
        let path = super::path_for(
            std::path::Path::new("/opt/homebrew/bin/codex"),
            Some("/Users/me/.nvm/bin:/opt/homebrew/bin".into()),
            Some("/usr/bin:/bin".into()),
        );
        assert_eq!(
            path,
            std::ffi::OsString::from("/opt/homebrew/bin:/Users/me/.nvm/bin:/usr/bin:/bin")
        );
        let before_login = super::path_for(
            std::path::Path::new("/opt/homebrew/bin/codex"),
            None,
            Some("/usr/bin:/bin".into()),
        );
        assert_eq!(
            before_login,
            std::ffi::OsString::from("/opt/homebrew/bin:/usr/bin:/bin")
        );
    }

    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// A fake shell: runs `body` and ignores the arguments a real shell would read.
    fn fake_shell(name: &str, body: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("hq-shell-{}-{name}", std::process::id()));
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn markers_survive_startup_noise() {
        let shell = fake_shell(
            "noisy",
            r"echo 'Welcome back!'; printf '\n__HQ_PATH__/opt/x/bin:/usr/bin__HQ_END__\n'; echo bye",
        );
        assert_eq!(
            path_from_shell(&shell, LOGIN_SHELL_TIMEOUT).as_deref(),
            Some("/opt/x/bin:/usr/bin")
        );
        let _ = std::fs::remove_file(shell);
    }

    #[test]
    fn slow_shell_times_out() {
        let shell = fake_shell("slow", "sleep 10");
        let started = Instant::now();
        assert_eq!(path_from_shell(&shell, Duration::from_millis(500)), None);
        assert!(started.elapsed() < Duration::from_secs(3));
        let _ = std::fs::remove_file(shell);
    }

    #[test]
    fn failing_or_missing_shell_gives_nothing() {
        let shell = fake_shell("failing", "exit 3");
        assert_eq!(path_from_shell(&shell, LOGIN_SHELL_TIMEOUT), None);
        let _ = std::fs::remove_file(shell);
        assert_eq!(
            path_from_shell(Path::new("/no/such/shell"), LOGIN_SHELL_TIMEOUT),
            None
        );
    }

    #[test]
    fn real_shell_reports_a_path() {
        // /bin/sh exists on every supported system and needs no startup files.
        let path = path_from_shell(Path::new("/bin/sh"), LOGIN_SHELL_TIMEOUT);
        assert!(path.is_some_and(|path| path.contains('/')));
    }
}
