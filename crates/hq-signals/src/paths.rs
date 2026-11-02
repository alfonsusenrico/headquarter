//! Filesystem locations Headquarter reads or owns.

use std::path::PathBuf;

/// The user's home folder: `HOME`, or on Windows (which rarely sets it) `USERPROFILE`.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| {
            cfg!(windows)
                .then(|| std::env::var_os("USERPROFILE"))
                .flatten()
                .filter(|value| !value.is_empty())
        })
        .map(PathBuf::from)
}

/// `$XDG_DATA_HOME/headquarter`, falling back to `~/.local/share/headquarter` on
/// both macOS and Linux so the layout is the same everywhere. `HQ_DATA_DIR` replaces
/// the whole folder for development and review runs: agents launched by Headquarter
/// read `XDG_DATA_HOME` too (Claude Code installs its updates under it), so isolating
/// Headquarter through it would move the agents' own files.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("HQ_DATA_DIR").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        return windows_dir(std::env::var_os("APPDATA"));
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".local").join("share")))?;
    Some(base.join("headquarter"))
}

/// Files the host owns inside the Headquarter data folder. Here rather than in the host's
/// crate, so a client builds without the host (add-windows-client D1).
#[derive(Debug, Clone)]
pub struct HostPaths {
    pub dir: PathBuf,
    pub database: PathBuf,
    pub control_socket: PathBuf,
    pub hook_socket: PathBuf,
    pub lock: PathBuf,
    pub log: PathBuf,
}

impl HostPaths {
    pub fn in_dir(dir: PathBuf) -> HostPaths {
        HostPaths {
            database: dir.join("state.db"),
            control_socket: dir.join("agentd.sock"),
            hook_socket: dir.join("hooks.sock"),
            lock: dir.join("agentd.lock"),
            log: dir.join("agentd.log"),
            dir,
        }
    }

    /// In [`data_dir`].
    pub fn standard() -> Option<HostPaths> {
        data_dir().map(HostPaths::in_dir)
    }
}

/// `%APPDATA%\Headquarter`: Windows' roaming application data, where the app keeps its
/// key, paired hosts, state and settings (add-windows-client D2).
pub fn windows_dir(appdata: Option<std::ffi::OsString>) -> Option<PathBuf> {
    appdata
        .filter(|value| !value.is_empty())
        .map(|dir| PathBuf::from(dir).join("Headquarter"))
}

pub fn probe_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("probe"))
}

/// Usage readings, one file per agent.
pub fn quota_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("quota"))
}

/// Claude Code's global configuration file (read-only for Headquarter): beside its
/// configuration directory's contents when `CLAUDE_CONFIG_DIR` is set, else `~/.claude.json`.
pub fn claude_global_config() -> Option<PathBuf> {
    match std::env::var_os("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir).join(".claude.json")),
        None => home().map(|home| home.join(".claude.json")),
    }
}

/// Claude Code's configuration directory (read-only for Headquarter).
pub fn claude_config_dir() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".claude")))
}

/// Codex's home directory (read-only for Headquarter).
pub fn codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".codex")))
}

/// Antigravity CLI's data directory (read-only for Headquarter).
pub fn agy_dir() -> Option<PathBuf> {
    home().map(|home| home.join(".gemini").join("antigravity-cli"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_keeps_the_app_in_its_application_data() {
        let appdata = PathBuf::from("C:")
            .join("Users")
            .join("me")
            .join("AppData")
            .join("Roaming");
        assert_eq!(
            windows_dir(Some(appdata.clone().into_os_string())),
            Some(appdata.join("Headquarter"))
        );
        assert_eq!(windows_dir(Some("".into())), None, "unset");
        assert_eq!(windows_dir(None), None);
    }
}
