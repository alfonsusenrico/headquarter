//! Files the host owns inside the Headquarter data folder.

use std::path::PathBuf;

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

    /// `$XDG_DATA_HOME/headquarter`, or `~/.local/share/headquarter`.
    pub fn standard() -> Option<HostPaths> {
        hq_signals::paths::data_dir().map(HostPaths::in_dir)
    }
}
