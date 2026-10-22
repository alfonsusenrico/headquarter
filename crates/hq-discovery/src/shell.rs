//! The owner's login-shell `PATH`, which a process started from a desktop launcher
//! does not inherit.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(3);

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
