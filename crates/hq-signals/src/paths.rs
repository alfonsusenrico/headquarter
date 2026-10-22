//! Filesystem locations Headquarter reads or owns.

use std::path::PathBuf;

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
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
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".local").join("share")))?;
    Some(base.join("headquarter"))
}

pub fn probe_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("probe"))
}

/// Usage readings, one file per agent.
pub fn quota_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("quota"))
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
