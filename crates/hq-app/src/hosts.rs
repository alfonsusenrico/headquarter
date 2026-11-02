//! The hosts this app shows (add-remote-desktop-client D5, D6): this computer's host, and
//! paired hosts on other computers, one on screen at a time. The paired hosts and the one
//! on screen are kept in `<data folder>/hosts.json`, and this computer's key for them in
//! `client.key` beside it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use anyhow::{Context, Result, anyhow};
use hq_client::{Host, HostAddress, HostPaths, Link, LinkError};
use hq_remote::PairingCode;
use serde::{Deserialize, Serialize};

/// A host on another computer this app has paired with. Nothing secret: the pairing
/// code's secret is used once and never kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownHost {
    /// The host's endpoint id (its public key).
    pub id: String,
    /// The host computer's name, as its gateway gives it.
    pub name: String,
    #[serde(default)]
    pub relay: Option<String>,
    #[serde(default)]
    pub addrs: Vec<String>,
    /// Unix milliseconds.
    #[serde(default)]
    pub paired_at: u64,
}

impl KnownHost {
    fn address(&self) -> HostAddress {
        HostAddress {
            id: self.id.clone(),
            relay: self.relay.clone(),
            addrs: self.addrs.clone(),
        }
    }
}

/// Which host is on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    /// This computer's host.
    Local,
    /// A paired host, by id.
    Remote(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    /// The id of the paired host on screen; none for this computer's.
    #[serde(default)]
    shown: Option<String>,
    #[serde(default)]
    hosts: Vec<KnownHost>,
}

struct State {
    file: File,
    /// One link per paired host, made when first needed.
    links: HashMap<String, Link>,
}

fn state() -> MutexGuard<'static, State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE
        .get_or_init(|| {
            let file = hosts_path().map(|path| load(&path)).unwrap_or_default();
            Mutex::new(State {
                file,
                links: HashMap::new(),
            })
        })
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn hosts_path() -> Option<PathBuf> {
    hq_signals::paths::data_dir().map(|dir| dir.join("hosts.json"))
}

fn key_path() -> Option<PathBuf> {
    hq_signals::paths::data_dir().map(|dir| dir.join("client.key"))
}

/// The kept hosts; a missing or unreadable file means none.
fn load(path: &Path) -> File {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save(file: &File) {
    let Some(path) = hosts_path() else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec_pretty(file) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let temporary = path.with_extension("json.tmp");
    if std::fs::write(&temporary, bytes).is_ok() {
        let _ = std::fs::rename(&temporary, &path);
    }
}

/// The host on screen.
pub fn shown() -> Shown {
    match &state().file.shown {
        Some(id) => Shown::Remote(id.clone()),
        None => Shown::Local,
    }
}

/// Whether this computer runs a host the app can show. On Windows it never does: the
/// app there is a client of remote hosts only (add-windows-client D2).
pub fn has_local_host() -> bool {
    cfg!(unix)
}

/// Whether a paired host on another computer is on screen.
pub fn is_remote() -> bool {
    shown() != Shown::Local
}

/// The paired hosts, most recently paired first.
pub fn known() -> Vec<KnownHost> {
    let mut hosts = state().file.hosts.clone();
    hosts.sort_by_key(|host| std::cmp::Reverse(host.paired_at));
    hosts
}

/// The name of the host on screen: "this computer" or the host's own.
pub fn shown_name() -> Option<String> {
    let state = state();
    let id = state.file.shown.as_ref()?;
    state
        .file
        .hosts
        .iter()
        .find(|host| &host.id == id)
        .map(|host| host.name.clone())
}

/// Puts `shown` on screen and remembers it.
pub fn show(shown: Shown) {
    let mut state = state();
    state.file.shown = match shown {
        Shown::Local => None,
        Shown::Remote(id) => Some(id),
    };
    save(&state.file);
}

/// What clients connect to now: the paired host on screen, or this computer's.
pub fn current() -> Result<Host> {
    let mut state = state();
    let Some(id) = state.file.shown.clone() else {
        return local();
    };
    if let Some(link) = state.links.get(&id) {
        return Ok(Host::Remote(link.clone()));
    }
    let host = state
        .file
        .hosts
        .iter()
        .find(|host| host.id == id)
        .cloned()
        .ok_or_else(|| anyhow!("the host on screen is no longer paired"))?;
    let key = hq_remote::client::client_key(&key_path().context("no data folder")?)
        .context("cannot read this computer's key")?;
    let link = Link::new(key, host.address()).context("cannot start the connection")?;
    state.links.insert(id, link.clone());
    Ok(Host::Remote(link))
}

/// This computer's host.
pub fn local() -> Result<Host> {
    HostPaths::standard()
        .map(Host::Local)
        .ok_or_else(|| anyhow!("cannot find the home folder"))
}

/// Remembers where a paired host was last reached, after its gateway named its relay.
pub fn remember_address(link: &Link) {
    let address = link.address();
    let mut state = state();
    let mut changed = false;
    if let Some(host) = state
        .file
        .hosts
        .iter_mut()
        .find(|host| host.id == address.id)
        && host.relay != address.relay
    {
        host.relay = address.relay;
        changed = true;
    }
    if let Some(welcome) = link.welcome()
        && let Some(host) = state
            .file
            .hosts
            .iter_mut()
            .find(|host| host.id == address.id)
        && host.name != welcome.host
    {
        host.name = welcome.host;
        changed = true;
    }
    if changed {
        save(&state.file);
    }
}

/// Why pairing failed, in the owner's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairError {
    /// Not a pairing code at all.
    NotACode(String),
    /// The host refused the code: wrong, used or expired.
    Refused(String),
    /// The host could not be reached.
    Unreachable(String),
    /// The host runs a release with another protocol.
    Mismatch(String),
}

impl std::fmt::Display for PairError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PairError::NotACode(reason)
            | PairError::Refused(reason)
            | PairError::Unreachable(reason)
            | PairError::Mismatch(reason) => f.write_str(reason),
        }
    }
}

/// Pairs with the host whose code the owner pasted, keeps it, and puts it on screen.
/// Blocks while the host answers; run it off the main thread.
pub fn pair(code: &str) -> Result<KnownHost, PairError> {
    let code = PairingCode::parse(code).map_err(|error| PairError::NotACode(error.to_string()))?;
    let key_path = key_path().ok_or_else(|| PairError::Unreachable("no data folder".into()))?;
    let key = hq_remote::client::client_key(&key_path).map_err(|error| {
        PairError::Unreachable(format!("cannot read this computer's key: {error}"))
    })?;
    let (link, welcome) =
        Link::pair(key, &code, &computer_name()).map_err(|error| match error {
            LinkError::Refused(reason) => PairError::Refused(refusal(&reason)),
            LinkError::NotKnown => PairError::Refused(
                "The code was not accepted. Show a new one on the host and try again.".into(),
            ),
            LinkError::Mismatch { .. } => PairError::Mismatch(error.to_string()),
            LinkError::Unreachable(reason) => PairError::Unreachable(format!(
                "Couldn't reach the host: {reason}. Check that it is on and online."
            )),
        })?;
    let address = link.address();
    let host = KnownHost {
        id: address.id.clone(),
        name: welcome.host,
        relay: address.relay,
        addrs: address.addrs,
        paired_at: hq_signals::now_ms(),
    };
    let mut state = state();
    state.file.hosts.retain(|known| known.id != host.id);
    state.file.hosts.push(host.clone());
    state.file.shown = Some(host.id.clone());
    state.links.insert(host.id.clone(), link);
    save(&state.file);
    Ok(host)
}

/// The host's refusal of a code, in the owner's words.
fn refusal(reason: &str) -> String {
    if reason.contains("not valid") || reason.contains("not paired") {
        "The code is wrong or has expired. Show a new one on the host and try again.".into()
    } else {
        reason.to_owned()
    }
}

/// Forgets a paired host: it leaves the list, and this computer shows its own host when
/// that one was on screen. The host keeps this computer in its device list until the owner
/// removes it there.
pub fn forget(id: &str) {
    let mut state = state();
    state.file.hosts.retain(|host| host.id != id);
    if let Some(link) = state.links.remove(id) {
        link.close();
    }
    if state.file.shown.as_deref() == Some(id) {
        state.file.shown = None;
    }
    save(&state.file);
}

/// This computer's name, as the host lists it among its devices.
fn computer_name() -> String {
    if let Ok(name) = std::env::var("COMPUTERNAME")
        && !name.trim().is_empty()
    {
        return name.trim().to_owned();
    }
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|name| name.trim().split('.').next().map(str::to_owned))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Computer".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hosts_file_keeps_unknown_hosts_fields_out_and_reads_old_ones() {
        let file: File = serde_json::from_str(
            r#"{"hosts":[{"id":"abc","name":"server","relay":null,"later":1}]}"#,
        )
        .unwrap();
        assert_eq!(file.shown, None);
        assert_eq!(file.hosts[0].name, "server");
        assert!(file.hosts[0].addrs.is_empty());
    }

    #[test]
    fn a_refused_code_reads_plainly() {
        assert!(refusal("the pairing code is not valid; show a new one").contains("expired"));
        assert_eq!(refusal("something else"), "something else");
    }
}
