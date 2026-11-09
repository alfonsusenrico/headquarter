//! The remote control gateway (add-remote-gateway): off unless the owner turns it on;
//! then an iroh endpoint on its own thread and runtime (D2). A phone pairs once with a
//! code the owner shows (D4); each paired phone's connection is passed to the host's own
//! socket as an ordinary client, with every request checked against what a phone may do
//! (D1, D5). The log records events, never codes, keys, input or output (D11).

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use hq_protocol::{DeviceKind, ErrorCode, Frame, RemoteDevice, Request, Response};
use hq_remote::{ALPN, PAIRING_VALID_SECS, PairingCode, Relays};
use iroh::endpoint::Connection;
use iroh::{Endpoint, SecretKey};

use super::{Host, lock};
use crate::session::Outgoing;

/// Wrong pairing secrets allowed within [`GUESS_WINDOW`] before the code is voided.
const MAX_GUESSES: usize = 5;
const GUESS_WINDOW: Duration = Duration::from_secs(60);
/// How long a connecting phone has to send its first frame.
const FIRST_FRAME_WAIT: Duration = Duration::from_secs(15);
/// How long `RemotePair` waits for the relay, so the code carries its URL.
const RELAY_WAIT: Duration = Duration::from_secs(5);
/// Streams a paired desktop may have open on one connection (add-remote-desktop-client D3).
const MAX_STREAMS: usize = 64;
/// Streams a paired phone may have open on one connection: its own, and one for sending an
/// image while a session stays open (add-image-upload D9).
const PHONE_STREAMS: usize = 2;

/// How long a code works: [`PAIRING_VALID_SECS`], or `HQ_REMOTE_PAIR_SECS` in tests.
fn pairing_valid() -> Duration {
    let secs = std::env::var("HQ_REMOTE_PAIR_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(PAIRING_VALID_SECS);
    Duration::from_secs(secs)
}

/// Remote control's state in the host.
#[derive(Default)]
pub(super) struct Remote {
    state: Mutex<State>,
    /// Held while the gateway is started or stopped to follow the phones, so two such
    /// turns cannot interleave.
    following: Mutex<()>,
}

#[derive(Default)]
struct State {
    running: Option<Running>,
    /// Why the gateway is not running although it was asked to.
    problem: Option<String>,
}

struct Running {
    endpoint: Endpoint,
    runtime: tokio::runtime::Handle,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    pairing: Mutex<Option<Pending>>,
    guesses: Mutex<VecDeque<Instant>>,
    /// Connected phones by endpoint id, for listing and removal.
    connections: Mutex<HashMap<String, Connection>>,
}

/// A shown code waiting for a phone, and the client to tell when it is used.
struct Pending {
    code: PairingCode,
    until: Instant,
    tell: mpsc::Sender<Outgoing>,
    /// The local connection showing the code; the code ends with it.
    connection: u64,
}

impl Pending {
    fn tell(&self, response: &Response) {
        let _ = self.tell.send(Outgoing::Frame(Frame::response(response)));
    }
}

impl Remote {
    pub(super) fn running(&self) -> bool {
        lock(&self.state).running.is_some()
    }
}

/// What a paired device may ask for: a phone (D5) or a desktop app
/// (add-remote-desktop-client D2). Exhaustive on purpose: a new request must be decided
/// here for both.
fn allowed(kind: DeviceKind, request: &Request) -> bool {
    let desktop = kind == DeviceKind::Desktop;
    match request {
        Request::Hello { .. }
        | Request::List {}
        | Request::Subscribe { .. }
        | Request::Attach { .. }
        | Request::AttachShell { .. }
        | Request::Detach {}
        | Request::Resize { .. }
        | Request::Focus {}
        | Request::Resume { .. }
        | Request::Add { .. }
        | Request::Seen { .. }
        | Request::Usage {}
        | Request::FollowDetails { .. }
        | Request::Visible { .. }
        // An image for a session, which any client may send (add-image-upload D1).
        | Request::Upload { .. } => true,
        // What the desktop app does on its own computer, a paired desktop may do too.
        Request::New { .. }
        | Request::Rename { .. }
        | Request::Kill { .. }
        | Request::BringIn { .. }
        | Request::Close { .. }
        | Request::Remove { .. }
        | Request::RemoveConversation { .. }
        | Request::NewGroup { .. }
        | Request::RenameGroup { .. }
        | Request::DeleteGroup { .. }
        | Request::MoveGroup { .. }
        | Request::CollapseGroup { .. }
        | Request::SetGroupColor { .. }
        | Request::SetGroup { .. }
        | Request::CloseShell { .. }
        | Request::RefreshUsage {}
        | Request::Resources {}
        | Request::ListFolder { .. }
        | Request::FindFolders {} => desktop,
        // Never from another computer: a lost laptop cannot stop the host or let more
        // devices in (owner, 2026-11-02).
        Request::Stop {}
        // The gateway names the device itself; a device cannot name its own connection
        // (fix-terminal-in-use-elsewhere D2).
        | Request::Device { .. }
        | Request::Pair { .. }
        | Request::RemoteStatus {}
        | Request::RemoteSet { .. }
        | Request::RemotePair {}
        | Request::RemoteDevices {}
        | Request::RemoteRemove { .. }
        // It starts the host's agents; only `hq` on the host asks (add-agent-update-check D4).
        | Request::CheckAgents {} => false,
        // The gateway answers it itself, for the calling device only (`proxy`).
        Request::Unpair {} => false,
    }
}

/// How a phone's proxied connection ended.
enum Ending {
    Closed,
    /// The phone removed its own pairing (add-android-app D5).
    Unpaired,
}

/// The request's wire name, for the log.
fn kind(request: &Request) -> String {
    serde_json::to_value(request)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|kind| kind.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// The host's iroh key: created once, readable by the owner only (D3).
fn host_key(path: &Path) -> Result<SecretKey, String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| format!("{} is not a remote control key", path.display()))?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let key = SecretKey::generate();
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
            file.write_all(&key.to_bytes())
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            Ok(key)
        }
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

fn host_name() -> String {
    let uname = rustix::system::uname();
    let name = uname.nodename().to_string_lossy();
    let short = name.split('.').next().unwrap_or_default();
    if short.is_empty() {
        "computer".to_owned()
    } else {
        short.to_owned()
    }
}

/// A device's name as it may be shown: trimmed, at most 64 characters, never empty.
fn device_name(name: &str, kind: DeviceKind) -> String {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect();
    if name.is_empty() {
        match kind {
            DeviceKind::Phone => "Phone".to_owned(),
            DeviceKind::Desktop => "Computer".to_owned(),
        }
    } else {
        name
    }
}

impl Host {
    /// Runs the gateway exactly while a phone is paired or a pairing code is shown, and
    /// stops it otherwise (remote-follows-paired-phones D1). Safe to call at any time
    /// from outside the gateway's own runtime; from inside it, use `follow_phones_later`.
    pub(super) fn follow_phones(self: &Arc<Self>) {
        let _turn = lock(&self.remote.following);
        let pairing = lock(&self.remote.state)
            .running
            .as_ref()
            .is_some_and(|running| lock(&running.shared.pairing).is_some());
        if self.has_phones() || pairing {
            if !self.remote.running()
                && let Err(problem) = self.start_gateway()
            {
                self.log.line(&format!("remote failed problem={problem}"));
                lock(&self.remote.state).problem = Some(problem);
            }
        } else {
            self.stop_gateway();
        }
    }

    /// `follow_phones` on a thread of its own: stopping the gateway waits for its runtime,
    /// so it cannot run on one of that runtime's threads.
    fn follow_phones_later(self: &Arc<Self>) {
        let host = Arc::clone(self);
        thread::spawn(move || host.follow_phones());
    }

    /// Whether at least one phone is paired.
    pub(super) fn has_phones(&self) -> bool {
        lock(&self.store)
            .devices()
            .is_ok_and(|devices| !devices.is_empty())
    }

    /// Older clients' on/off switch: remote control follows the paired phones now, so it
    /// only answers the status (remote-follows-paired-phones D3).
    pub(super) fn remote_set(&self) -> Response {
        self.remote_status(self.has_phones())
    }

    fn start_gateway(self: &Arc<Self>) -> Result<(), String> {
        let settings = lock(&self.settings)
            .current()
            .map_err(|error| error.to_string())?;
        let relays = Relays::from_settings(&settings.remote.relays)?;
        let secret = host_key(&self.paths.dir.join("remote.key"))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("hq-remote")
            .enable_all()
            .build()
            .map_err(|error| format!("cannot start the gateway's runtime: {error}"))?;
        let endpoint = runtime.block_on(hq_remote::bind(secret, &relays, &[ALPN]))?;
        let shared = Arc::new(Shared::default());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let handle = runtime.handle().clone();
        let thread = {
            let host = Arc::clone(self);
            let endpoint = endpoint.clone();
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("hq-remote".into())
                .spawn(move || {
                    runtime.block_on(accept_loop(host, endpoint, shared, stopped));
                })
                .map_err(|error| format!("cannot start the gateway: {error}"))?
        };
        self.log.line(&format!(
            "remote enabled endpoint={} relays={}",
            endpoint.id().fmt_short(),
            match &relays {
                Relays::Default => "n0".to_owned(),
                Relays::Custom(urls) => format!("own:{}", urls.len()),
                Relays::Disabled => "none".to_owned(),
            }
        ));
        let mut state = lock(&self.remote.state);
        state.problem = None;
        state.running = Some(Running {
            endpoint,
            runtime: handle,
            stop: Some(stop),
            thread: Some(thread),
            shared,
        });
        Ok(())
    }

    pub(super) fn stop_gateway(&self) {
        let running = {
            let mut state = lock(&self.remote.state);
            state.problem = None;
            state.running.take()
        };
        let Some(mut running) = running else {
            return;
        };
        if let Some(pending) = lock(&running.shared.pairing).take() {
            pending.tell(&Response::RemotePairExpired {
                reason: "remote control was turned off".into(),
            });
        }
        if let Some(stop) = running.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = running.thread.take() {
            let _ = thread.join();
        }
        self.log.line("remote disabled");
    }

    pub(super) fn remote_status(&self, enabled: bool) -> Response {
        let state = lock(&self.remote.state);
        let running = state.running.as_ref();
        Response::RemoteStatus {
            enabled,
            running: running.is_some(),
            endpoint: running.map(|running| running.endpoint.id().to_string()),
            relay: running.and_then(|running| {
                running
                    .endpoint
                    .addr()
                    .relay_urls()
                    .next()
                    .map(ToString::to_string)
            }),
            problem: state.problem.clone(),
        }
    }

    /// A new one-time code; the client hears again on `tell` when a phone uses it or it
    /// expires (D4). A code shown before is replaced.
    /// Starts the gateway first when no phone is paired yet (remote-follows-paired-phones D1).
    pub(super) fn remote_pair(
        self: &Arc<Self>,
        tell: mpsc::Sender<Outgoing>,
        connection: u64,
    ) -> Response {
        if !self.remote.running() {
            let _turn = lock(&self.remote.following);
            if !self.remote.running()
                && let Err(problem) = self.start_gateway()
            {
                self.log.line(&format!("remote failed problem={problem}"));
                lock(&self.remote.state).problem = Some(problem.clone());
                return Response::error(ErrorCode::RemoteOff, problem);
            }
        }
        let (endpoint, runtime, shared) = {
            let state = lock(&self.remote.state);
            let Some(running) = state.running.as_ref() else {
                return Response::error(ErrorCode::RemoteOff, "remote control stopped");
            };
            (
                running.endpoint.clone(),
                running.runtime.clone(),
                Arc::clone(&running.shared),
            )
        };
        // Wait briefly for the relay, so the code works from other networks.
        let addr = runtime.block_on(async {
            let _ = tokio::time::timeout(RELAY_WAIT, endpoint.online()).await;
            endpoint.addr()
        });
        let code = PairingCode::new(
            endpoint.id(),
            addr.relay_urls().next().cloned(),
            addr.ip_addrs().copied().collect(),
        );
        let text = code.format();
        let secret = code.secret.clone();
        if let Some(previous) = lock(&shared.pairing).replace(Pending {
            code,
            until: Instant::now() + pairing_valid(),
            tell,
            connection,
        }) {
            previous.tell(&Response::RemotePairExpired {
                reason: "a newer code was shown".into(),
            });
        }
        lock(&shared.guesses).clear();
        {
            let shared = Arc::clone(&shared);
            let host = Arc::clone(self);
            runtime.spawn(async move {
                tokio::time::sleep(pairing_valid()).await;
                let expired = {
                    let mut pairing = lock(&shared.pairing);
                    if pairing
                        .as_ref()
                        .is_some_and(|pending| pending.code.secret == secret)
                    {
                        pairing.take()
                    } else {
                        None
                    }
                };
                if let Some(expired) = expired {
                    expired.tell(&Response::RemotePairExpired {
                        reason: "the code expired".into(),
                    });
                    host.follow_phones_later();
                }
            });
        }
        self.log.line("remote pairing code shown");
        Response::RemotePairing {
            code: text,
            expires_in_secs: pairing_valid().as_secs(),
        }
    }

    /// The client that showed the pairing code left (its window closed, Cancel, Ctrl-C):
    /// the code stops working (D13).
    pub(super) fn remote_client_left(self: &Arc<Self>, connection: u64) {
        let Some(shared) = lock(&self.remote.state)
            .running
            .as_ref()
            .map(|running| Arc::clone(&running.shared))
        else {
            return;
        };
        let mut pairing = lock(&shared.pairing);
        if pairing
            .as_ref()
            .is_some_and(|pending| pending.connection == connection)
        {
            *pairing = None;
            drop(pairing);
            self.log.line("remote pairing code withdrawn");
            self.follow_phones_later();
        }
    }

    pub(super) fn remote_devices(&self) -> Response {
        let rows = match lock(&self.store).devices() {
            Ok(rows) => rows,
            Err(error) => return Response::error(ErrorCode::Storage, error.to_string()),
        };
        let connected: Vec<String> = lock(&self.remote.state)
            .running
            .as_ref()
            .map(|running| lock(&running.shared.connections).keys().cloned().collect())
            .unwrap_or_default();
        Response::RemoteDevices {
            devices: rows
                .into_iter()
                .map(|row| RemoteDevice {
                    connected: connected.contains(&row.endpoint_id),
                    kind: DeviceKind::from_stored(&row.kind),
                    id: row.endpoint_id,
                    name: row.name,
                    paired_at: row.paired_at,
                    last_seen_at: row.last_seen_at,
                })
                .collect(),
        }
    }

    /// Removes a phone by name (any case) or endpoint id prefix, closing its connection.
    pub(super) fn remote_remove(self: &Arc<Self>, reference: &str) -> Response {
        let rows = match lock(&self.store).devices() {
            Ok(rows) => rows,
            Err(error) => return Response::error(ErrorCode::Storage, error.to_string()),
        };
        let by_name: Vec<_> = rows
            .iter()
            .filter(|row| row.name.eq_ignore_ascii_case(reference.trim()))
            .collect();
        let matches = if by_name.is_empty() && reference.len() >= super::MIN_ID_PREFIX {
            rows.iter()
                .filter(|row| row.endpoint_id.starts_with(reference))
                .collect()
        } else {
            by_name
        };
        let device = match matches.as_slice() {
            [device] => *device,
            [] => {
                return Response::error(
                    ErrorCode::NotFound,
                    format!("no paired device `{reference}`; `hq remote devices` lists them"),
                );
            }
            several => {
                return Response::error(
                    ErrorCode::Ambiguous,
                    format!(
                        "`{reference}` matches {} devices; use its id",
                        several.len()
                    ),
                );
            }
        };
        if let Err(error) = lock(&self.store).remove_device(&device.endpoint_id) {
            return Response::error(ErrorCode::Storage, error.to_string());
        }
        if let Some(running) = lock(&self.remote.state).running.as_ref()
            && let Some(connection) = lock(&running.shared.connections).remove(&device.endpoint_id)
        {
            connection.close(1u32.into(), b"removed");
        }
        self.log
            .line(&format!("remote removed device={}", device.name));
        // The last phone gone, remote control stops (remote-follows-paired-phones D1).
        self.follow_phones();
        Response::Done
    }
}

async fn accept_loop(
    host: Arc<Host>,
    endpoint: Endpoint,
    shared: Arc<Shared>,
    mut stopped: tokio::sync::oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut stopped => break,
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else { break };
                let host = Arc::clone(&host);
                let shared = Arc::clone(&shared);
                let endpoint = endpoint.clone();
                tokio::spawn(async move {
                    if let Ok(connection) = incoming.await {
                        serve_device(host, shared, &endpoint, connection).await;
                    }
                });
            }
        }
    }
    for (_, connection) in lock(&shared.connections).drain() {
        connection.close(0u32.into(), b"remote control stopped");
    }
    endpoint.close().await;
}

/// One device's connection: pairing if needed, the welcome, then the proxy. A desktop
/// opens more streams on the same connection, each proxied the same way, and keeps the
/// connection until it closes it (add-remote-desktop-client D3).
async fn serve_device(
    host: Arc<Host>,
    shared: Arc<Shared>,
    endpoint: &Endpoint,
    connection: Connection,
) {
    let device = connection.remote_id().to_string();
    let Ok(Ok((mut send, mut recv))) =
        tokio::time::timeout(FIRST_FRAME_WAIT, connection.accept_bi()).await
    else {
        return;
    };
    let first = match tokio::time::timeout(FIRST_FRAME_WAIT, hq_remote::read_frame(&mut recv)).await
    {
        Ok(Ok(Some(frame))) => frame,
        _ => return,
    };
    let known = lock(&host.store).device(&device).ok().flatten();
    // A device this computer already knows may pair again with a shown code (it lost its
    // list, or scanned the code from its switcher): it keeps its record, renamed.
    let pairing = matches!(
        &first,
        Frame::Control(bytes) if matches!(Request::parse(bytes), Ok(Request::Pair { .. }))
    );
    let (name, kind, first) = match known {
        Some(row) if !pairing => (row.name, DeviceKind::from_stored(&row.kind), Some(first)),
        _ => match pair(&host, &shared, &device, &first) {
            Ok((name, kind)) => {
                let paired = Response::Paired { host: host_name() };
                if hq_remote::write_frame(&mut send, &Frame::response(&paired))
                    .await
                    .is_err()
                {
                    return;
                }
                (name, kind, None)
            }
            Err(refusal) => {
                let _ = hq_remote::write_frame(&mut send, &Frame::response(&refusal)).await;
                let _ = send.finish();
                // Closing drops what the device has not received yet: let the refusal
                // arrive first, briefly.
                let _ = tokio::time::timeout(FIRST_FRAME_WAIT / 5, send.stopped()).await;
                connection.close(2u32.into(), b"not paired");
                return;
            }
        },
    };
    let welcome = Response::RemoteWelcome {
        host: host_name(),
        relay: endpoint.addr().relay_urls().next().map(ToString::to_string),
        protocol: hq_protocol::PROTOCOL_VERSION,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    if hq_remote::write_frame(&mut send, &Frame::response(&welcome))
        .await
        .is_err()
    {
        return;
    }
    lock(&shared.connections).insert(device.clone(), connection.clone());
    let _ = lock(&host.store).touch_device(&device, hq_signals::now_ms());
    let path = path_kind(&connection);
    host.log.line(&format!(
        "remote connected device={name} kind={} path={path}",
        kind.as_str()
    ));
    let streams = Arc::new(AtomicUsize::new(1));
    let limit = match kind {
        DeviceKind::Desktop => MAX_STREAMS,
        DeviceKind::Phone => PHONE_STREAMS,
    };
    let more = tokio::spawn(more_streams(
        Arc::clone(&host),
        connection.clone(),
        Stream {
            device: device.clone(),
            name: name.clone(),
            kind,
        },
        Arc::clone(&streams),
        limit,
    ));
    let stream = Stream {
        device: device.clone(),
        name: name.clone(),
        kind,
    };
    let ending = proxy(&host, &stream, first, send, recv).await;
    streams.fetch_sub(1, Ordering::SeqCst);
    if let Ending::Unpaired = ending {
        more.abort();
        lock(&shared.connections).remove(&device);
        unpaired(&host, &connection, &name).await;
        return;
    }
    if kind == DeviceKind::Desktop {
        // The desktop's first stream ended; its other streams go on until it closes the
        // connection, or the device is removed or unpaired.
        connection.closed().await;
    }
    more.abort();
    let still_ours = {
        let mut connections = lock(&shared.connections);
        let ours = connections
            .get(&device)
            .is_some_and(|known| known.stable_id() == connection.stable_id());
        if ours {
            connections.remove(&device);
        }
        ours
    };
    if still_ours {
        let _ = lock(&host.store).touch_device(&device, hq_signals::now_ms());
    }
    // Why QUIC ended it (idle timeout, closed by the device, lost path), when it did; the
    // reason carries no frames or keys.
    let reason = match connection.close_reason() {
        Some(error) => error.to_string(),
        None => "host".to_owned(),
    };
    connection.close(0u32.into(), b"bye");
    host.log.line(&format!(
        "remote disconnected device={name} reason={reason}"
    ));
}

/// The paired device behind a proxied stream.
#[derive(Clone)]
struct Stream {
    /// Its endpoint id.
    device: String,
    name: String,
    kind: DeviceKind,
}

/// A device's further streams, each proxied to its own host connection, at most `limit`
/// at once: [`MAX_STREAMS`] for a desktop, [`PHONE_STREAMS`] for a phone.
async fn more_streams(
    host: Arc<Host>,
    connection: Connection,
    stream: Stream,
    streams: Arc<AtomicUsize>,
    limit: usize,
) {
    while let Ok((mut send, recv)) = connection.accept_bi().await {
        if streams.load(Ordering::SeqCst) >= limit {
            host.log.line(&format!(
                "remote refused stream device={} streams={limit}",
                stream.name
            ));
            let refusal = Response::error(
                ErrorCode::NotAllowed,
                format!("at most {limit} streams on one connection"),
            );
            let _ = hq_remote::write_frame(&mut send, &Frame::response(&refusal)).await;
            let _ = send.finish();
            continue;
        }
        streams.fetch_add(1, Ordering::SeqCst);
        let host = Arc::clone(&host);
        let connection = connection.clone();
        let stream = stream.clone();
        let streams = Arc::clone(&streams);
        tokio::spawn(async move {
            let ending = proxy(&host, &stream, None, send, recv).await;
            streams.fetch_sub(1, Ordering::SeqCst);
            if let Ending::Unpaired = ending {
                if let Some(running) = lock(&host.remote.state).running.as_ref() {
                    lock(&running.shared.connections).remove(&stream.device);
                }
                unpaired(&host, &connection, &stream.name).await;
            }
        });
    }
}

/// The device removed its own pairing: let the answer reach it, then close.
async fn unpaired(host: &Arc<Host>, connection: &Connection, name: &str) {
    // Closing at once would drop the answer on its way to the device (D4 of
    // add-remote-gateway). The device closes once it has it.
    let _ = tokio::time::timeout(FIRST_FRAME_WAIT / 5, connection.closed()).await;
    connection.close(0u32.into(), b"unpaired");
    host.log.line(&format!("remote unpaired device={name}"));
    host.follow_phones_later();
}

/// Checks a device's first frame against the shown code; records the device and its kind
/// when it matches (D4).
fn pair(
    host: &Arc<Host>,
    shared: &Shared,
    device: &str,
    first: &Frame,
) -> Result<(String, DeviceKind), Box<Response>> {
    let refused = |message: &str| Box::new(Response::error(ErrorCode::NotAllowed, message));
    let Frame::Control(bytes) = first else {
        return Err(refused("this device is not paired"));
    };
    let Ok(Request::Pair { secret, name, kind }) = Request::parse(bytes) else {
        return Err(refused("this device is not paired"));
    };
    let mut pairing = lock(&shared.pairing);
    let valid = pairing.as_ref().is_some_and(|pending| {
        Instant::now() < pending.until && hq_remote::same_secret(&pending.code.secret, &secret)
    });
    if !valid {
        drop(pairing);
        guessed_wrong(host, shared);
        host.log.line("remote pairing failed");
        return Err(refused("the pairing code is not valid; show a new one"));
    }
    let name = device_name(&name, kind);
    if let Err(error) =
        lock(&host.store).add_device(device, &name, kind.as_str(), hq_signals::now_ms())
    {
        return Err(Box::new(Response::error(
            ErrorCode::Storage,
            error.to_string(),
        )));
    }
    if let Some(used) = pairing.take() {
        used.tell(&Response::RemotePairUsed {
            device: name.clone(),
            kind,
        });
    }
    host.log.line(&format!(
        "remote paired device={name} kind={}",
        kind.as_str()
    ));
    Ok((name, kind))
}

/// Five wrong codes within a minute void the code being shown.
fn guessed_wrong(host: &Arc<Host>, shared: &Shared) {
    let now = Instant::now();
    let mut guesses = lock(&shared.guesses);
    guesses.push_back(now);
    while guesses
        .front()
        .is_some_and(|at| now.duration_since(*at) > GUESS_WINDOW)
    {
        guesses.pop_front();
    }
    if guesses.len() >= MAX_GUESSES {
        guesses.clear();
        if let Some(voided) = lock(&shared.pairing).take() {
            voided.tell(&Response::RemotePairExpired {
                reason: "too many wrong codes; show a new one".into(),
            });
            host.log
                .line("remote pairing code voided after wrong codes");
            host.follow_phones_later();
        }
    }
}

fn path_kind(connection: &Connection) -> &'static str {
    let paths = connection.paths();
    match paths.iter().find(|path| path.is_selected()) {
        Some(path) if path.is_relay() => "relayed",
        Some(_) => "direct",
        None => "unknown",
    }
}

/// Passes frames between a device's stream and the host's socket, refusing what that kind
/// of device may not ask for (D1, D5; add-remote-desktop-client D2).
async fn proxy(
    host: &Host,
    stream: &Stream,
    first: Option<Frame>,
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
) -> Ending {
    let Ok(local) = tokio::net::UnixStream::connect(&host.paths.control_socket).await else {
        return Ending::Closed;
    };
    let (mut from_host, mut to_host) = local.into_split();
    let (to_device, mut for_device) = tokio::sync::mpsc::channel::<Frame>(64);
    let writer = tokio::spawn(async move {
        while let Some(frame) = for_device.recv().await {
            if hq_remote::write_frame(&mut send, &frame).await.is_err() {
                break;
            }
        }
        let _ = send.finish();
    });
    let from_host_task = {
        let to_device = to_device.clone();
        tokio::spawn(async move {
            while let Ok(Some(frame)) = hq_remote::read_frame(&mut from_host).await {
                if to_device.send(frame).await.is_err() {
                    break;
                }
            }
        })
    };
    let mut pending = first;
    let mut ending = Ending::Closed;
    loop {
        let frame = match pending.take() {
            Some(frame) => frame,
            None => match hq_remote::read_frame(&mut recv).await {
                Ok(Some(frame)) => frame,
                _ => break,
            },
        };
        let greeting = matches!(
            &frame,
            Frame::Control(bytes) if matches!(Request::parse(bytes), Ok(Request::Hello { .. }))
        );
        let pass = match &frame {
            Frame::Input(_) => true,
            Frame::Output(_) => break,
            Frame::Control(bytes) => match Request::parse(bytes) {
                Ok(Request::Unpair {}) => {
                    let answer = match lock(&host.store).remove_device(&stream.device) {
                        Ok(_) => {
                            ending = Ending::Unpaired;
                            Response::Unpaired
                        }
                        Err(error) => Response::error(ErrorCode::Storage, error.to_string()),
                    };
                    let _ = to_device.send(Frame::response(&answer)).await;
                    if let Ending::Unpaired = ending {
                        break;
                    }
                    false
                }
                Ok(request) if allowed(stream.kind, &request) => true,
                Ok(request) => {
                    let kind = kind(&request);
                    host.log.line(&format!("remote refused request={kind}"));
                    let refusal = Response::error(
                        ErrorCode::NotAllowed,
                        format!("`{kind}` is not allowed from a {}", stream.kind.as_str()),
                    );
                    if to_device.send(Frame::response(&refusal)).await.is_err() {
                        break;
                    }
                    false
                }
                // The host answers malformed requests as for any client.
                Err(_) => true,
            },
        };
        if pass && hq_remote::write_frame(&mut to_host, &frame).await.is_err() {
            break;
        }
        if greeting {
            let named = Frame::request(&Request::Device {
                name: stream.name.clone(),
            });
            if hq_remote::write_frame(&mut to_host, &named).await.is_err() {
                break;
            }
        }
    }
    drop(to_host);
    from_host_task.abort();
    drop(to_device);
    let _ = writer.await;
    ending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allow_list_follows_the_owner() {
        let both = [
            Request::List {},
            Request::Focus {},
            Request::Resume {
                session: "s".into(),
                term: None,
            },
            Request::Upload {
                session: "s".into(),
                size: 1,
            },
        ];
        for request in &both {
            assert!(allowed(DeviceKind::Phone, request), "{request:?}");
            assert!(allowed(DeviceKind::Desktop, request), "{request:?}");
        }
        let desktop_only = [
            Request::CloseShell {
                session: "s".into(),
            },
            Request::RefreshUsage {},
            Request::Resources {},
            Request::Rename {
                session: "s".into(),
                name: "n".into(),
            },
            Request::ListFolder {
                path: String::new(),
            },
            Request::FindFolders {},
        ];
        for request in &desktop_only {
            assert!(!allowed(DeviceKind::Phone, request), "{request:?}");
            assert!(allowed(DeviceKind::Desktop, request), "{request:?}");
        }
        let neither = [
            Request::Stop {},
            Request::CheckAgents {},
            Request::RemotePair {},
            Request::RemoteDevices {},
            Request::RemoteRemove { device: "d".into() },
            Request::RemoteSet { enabled: false },
            Request::RemoteStatus {},
            Request::Device {
                name: "MacBook".into(),
            },
            Request::Unpair {},
        ];
        for request in &neither {
            assert!(!allowed(DeviceKind::Phone, request), "{request:?}");
            assert!(!allowed(DeviceKind::Desktop, request), "{request:?}");
        }
        assert_eq!(kind(&Request::Stop {}), "stop");
    }

    #[test]
    fn names_are_tidy() {
        assert_eq!(device_name("  Pixel 8\n", DeviceKind::Phone), "Pixel 8");
        assert_eq!(device_name("", DeviceKind::Phone), "Phone");
        assert_eq!(device_name("", DeviceKind::Desktop), "Computer");
        assert_eq!(device_name(&"x".repeat(100), DeviceKind::Phone).len(), 64);
    }

    #[test]
    fn the_key_is_private_and_kept() {
        let dir = std::env::temp_dir().join(format!("hq-remote-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("remote.key");
        let first = host_key(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(
            std::os::unix::fs::PermissionsExt::mode(&mode) & 0o777,
            0o600
        );
        assert_eq!(host_key(&path).unwrap().public(), first.public(), "reused");
        std::fs::write(&path, b"short").unwrap();
        assert!(host_key(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
