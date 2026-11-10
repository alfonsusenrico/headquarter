//! The hosts whose sessions the app lists together (add-merged-host-list D1): this computer's
//! host, and paired hosts on other computers. Every paired host keeps its link, all through
//! one endpoint (add-shared-host-endpoint D3). The paired hosts are kept in
//! `<data folder>/hosts.json`, and this computer's key for them in `client.key` beside it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use anyhow::{Context, Result, anyhow};
#[cfg(unix)]
use hq_client::HostPaths;
use hq_client::{Host, HostAddress, Link, LinkError};
use hq_remote::PairingCode;
use hq_remote::client::ClientEndpoint;
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

/// A host whose sessions the app lists.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum HostId {
    /// This computer's host.
    #[default]
    Local,
    /// A paired host, by its endpoint id.
    Remote(String),
}

/// What `hosts.json` keeps. An older file's `shown` (the host once on screen) is ignored.
#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    hosts: Vec<KnownHost>,
}

struct State {
    file: File,
    /// One link per paired host, made when first needed and kept while it is paired.
    links: HashMap<String, Link>,
    /// The endpoint every link dials through, made with this computer's key on first use.
    endpoint: Option<Arc<ClientEndpoint>>,
}

fn state() -> MutexGuard<'static, State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE
        .get_or_init(|| {
            let file = hosts_path().map(|path| load(&path)).unwrap_or_default();
            Mutex::new(State {
                file,
                links: HashMap::new(),
                endpoint: None,
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

/// Whether this computer runs a host the app can show: its `hq` is installed here
/// (split-host-and-app-install D3). On Windows it never is: the app there is a client of
/// remote hosts only (add-windows-client D2). Read once; installing changes it only
/// across a restart, which the updater does.
pub fn has_local_host() -> bool {
    #[cfg(unix)]
    {
        static INSTALLED: OnceLock<bool> = OnceLock::new();
        *INSTALLED.get_or_init(|| hq_client::host_binary().is_some())
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Whether there is no host at all: a computer without a host of its own that has paired
/// with none yet (add-windows-client D2).
pub fn none() -> bool {
    !has_local_host() && state().file.hosts.is_empty()
}

/// Every host whose sessions the list shows: this computer's first when it has one, then the
/// paired hosts in the order they were paired.
pub fn all() -> Vec<HostId> {
    let mut paired = state().file.hosts.clone();
    paired.sort_by_key(|host| host.paired_at);
    has_local_host()
        .then_some(HostId::Local)
        .into_iter()
        .chain(paired.into_iter().map(|host| HostId::Remote(host.id)))
        .collect()
}

/// What this computer is called in the Hosts menu.
pub fn this_computer() -> &'static str {
    if cfg!(target_os = "macos") {
        "This Mac"
    } else {
        "This computer"
    }
}

/// A host's name as the owner reads it.
pub fn name(id: &HostId) -> String {
    match id {
        HostId::Local => this_computer().to_owned(),
        HostId::Remote(id) => state()
            .file
            .hosts
            .iter()
            .find(|host| &host.id == id)
            .map_or_else(|| "A host".to_owned(), |host| host.name.clone()),
    }
}

/// The host name a session's row shows (add-merged-host-list D7): none for this computer's
/// own sessions; on a computer without a host, every host's name.
pub fn label(id: &HostId) -> Option<String> {
    match id {
        HostId::Local => None,
        HostId::Remote(_) => Some(name(id)),
    }
}

/// The paired hosts, most recently paired first.
pub fn known() -> Vec<KnownHost> {
    let mut hosts = state().file.hosts.clone();
    hosts.sort_by_key(|host| std::cmp::Reverse(host.paired_at));
    hosts
}

/// The app's one endpoint for paired hosts, made with this computer's key on first use.
fn shared_endpoint(state: &mut State) -> Result<Arc<ClientEndpoint>> {
    if let Some(endpoint) = &state.endpoint {
        return Ok(Arc::clone(endpoint));
    }
    let key = hq_remote::client::client_key(&key_path().context("no data folder")?)
        .context("cannot read this computer's key")?;
    let endpoint = ClientEndpoint::new(key).context("cannot start the connection")?;
    state.endpoint = Some(Arc::clone(&endpoint));
    Ok(endpoint)
}

/// What clients connect to for host `id`.
pub fn host(id: &HostId) -> Result<Host> {
    let HostId::Remote(id) = id else {
        return local();
    };
    let mut state = state();
    if let Some(link) = state.links.get(id) {
        return Ok(Host::Remote(link.clone()));
    }
    let host = state
        .file
        .hosts
        .iter()
        .find(|host| &host.id == id)
        .cloned()
        .ok_or_else(|| anyhow!("the host is no longer paired"))?;
    let shared = shared_endpoint(&mut state)?;
    let link = Link::new(&shared, host.address());
    state.links.insert(id.clone(), link.clone());
    Ok(Host::Remote(link))
}

/// This computer's host, when it is installed here.
#[cfg(unix)]
pub fn local() -> Result<Host> {
    if !has_local_host() {
        return Err(anyhow!(NO_HOST_YET));
    }
    HostPaths::standard()
        .map(Host::Local)
        .ok_or_else(|| anyhow!("cannot find the home folder"))
}

/// This computer runs no host here: the app shows paired hosts only, and never offers
/// this computer's (add-windows-client D2).
#[cfg(not(unix))]
pub fn local() -> Result<Host> {
    Err(anyhow!(NO_HOST_YET))
}

/// Said where the sessions would be when no host is on screen yet.
pub const NO_HOST_YET: &str = "Connect to a host to see its sessions.";

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

/// Pairs with the host whose code the owner pasted and keeps it; its sessions join the list.
/// Blocks while the host answers; run it off the main thread.
pub fn pair(code: &str) -> Result<KnownHost, PairError> {
    let code = PairingCode::parse(code).map_err(|error| PairError::NotACode(error.to_string()))?;
    // The other hosts stay connected: pairing dials through the same endpoint.
    let shared = shared_endpoint(&mut state())
        .map_err(|error| PairError::Unreachable(format!("{error:#}")))?;
    let (link, welcome) =
        Link::pair(&shared, &code, &computer_name()).map_err(|error| match error {
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

/// Forgets a paired host: it and its sessions leave the list. The host keeps this computer in
/// its device list until the owner removes it there.
pub fn forget(id: &str) {
    let mut state = state();
    state.file.hosts.retain(|host| host.id != id);
    if let Some(link) = state.links.remove(id) {
        link.close();
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
    crate::process::quiet("hostname")
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
            r#"{"shown":"abc","hosts":[{"id":"abc","name":"server","relay":null,"later":1}]}"#,
        )
        .unwrap();
        assert_eq!(file.hosts[0].name, "server");
        assert!(file.hosts[0].addrs.is_empty());
    }

    #[test]
    fn a_refused_code_reads_plainly() {
        assert!(refusal("the pairing code is not valid; show a new one").contains("expired"));
        assert_eq!(refusal("something else"), "something else");
    }
}
