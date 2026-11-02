//! A desktop app's connection to a host on another computer (add-remote-desktop-client D4):
//! one QUIC connection, a control stream that carries the pairing and the gateway's
//! welcome, and one stream per host client. Streams are read and written with blocking
//! calls, so `hq-client` keeps its blocking code; the link runs its own small runtime.

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use hq_protocol::{DeviceKind, ErrorCode, Frame, PROTOCOL_VERSION, Request, Response};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::{Endpoint, EndpointAddr, PublicKey, RelayUrl, SecretKey};
use tokio::runtime::{Handle, Runtime};

use crate::{ALPN, PairingCode, Relays};

/// How long dialling a host may take before it counts as unreachable.
const DIAL_WAIT: Duration = Duration::from_secs(15);
/// How long the gateway and the host have to answer the handshake.
const ANSWER_WAIT: Duration = Duration::from_secs(15);

/// Where a paired host is dialled: its key, its home relay, and direct addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAddress {
    /// The host's endpoint id (its public key).
    pub id: String,
    pub relay: Option<String>,
    pub addrs: Vec<String>,
}

impl HostAddress {
    pub fn from_code(code: &PairingCode) -> HostAddress {
        HostAddress {
            id: code.host.to_string(),
            relay: code.relay.as_ref().map(ToString::to_string),
            addrs: code.addrs.iter().map(ToString::to_string).collect(),
        }
    }

    fn endpoint_addr(&self) -> Option<EndpointAddr> {
        let host: PublicKey = self.id.parse().ok()?;
        let mut addr = EndpointAddr::new(host);
        if let Some(relay) = self.relay.as_deref().and_then(|relay| relay.parse().ok()) {
            addr = addr.with_relay_url(relay);
        }
        for ip in self.addrs.iter().filter_map(|ip| ip.parse().ok()) {
            addr = addr.with_ip_addr(ip);
        }
        Some(addr)
    }

    /// The relays to bind with to reach this host: its home relay, or none.
    /// `HQ_REMOTE_RELAYS=none` keeps tests on direct addresses.
    fn relays(&self) -> Relays {
        if std::env::var("HQ_REMOTE_RELAYS").is_ok_and(|value| value == "none") {
            return Relays::Disabled;
        }
        match self
            .relay
            .as_deref()
            .and_then(|relay| relay.parse::<RelayUrl>().ok())
        {
            Some(url) => Relays::Custom(vec![url]),
            None => Relays::Disabled,
        }
    }
}

/// What the gateway said about the host when the link connected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Welcome {
    /// The host computer's name.
    pub host: String,
    pub relay: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// The host does not know this computer: never paired, or removed there.
    NotKnown,
    /// The host speaks another protocol; `host_version` names its release.
    Mismatch {
        host_protocol: u32,
        host_version: String,
    },
    /// The pairing code was refused (wrong or expired).
    Refused(String),
    /// The host could not be reached, or the connection broke.
    Unreachable(String),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::NotKnown => f.write_str("the host does not know this computer"),
            LinkError::Mismatch {
                host_protocol,
                host_version,
            } => write!(
                f,
                "the host runs Headquarter {host_version} (protocol {host_protocol}); this app speaks protocol {PROTOCOL_VERSION}"
            ),
            LinkError::Refused(reason) => f.write_str(reason),
            LinkError::Unreachable(reason) => write!(f, "the host is unreachable: {reason}"),
        }
    }
}

impl std::error::Error for LinkError {}

impl From<LinkError> for io::Error {
    fn from(error: LinkError) -> io::Error {
        let kind = match error {
            LinkError::Unreachable(_) => io::ErrorKind::NotConnected,
            _ => io::ErrorKind::PermissionDenied,
        };
        io::Error::new(kind, error)
    }
}

pub use iroh::SecretKey as ClientKey;

/// This computer's key for remote hosts, created once and readable by the owner only
/// (add-remote-desktop-client D5).
pub fn client_key(path: &std::path::Path) -> io::Result<SecretKey> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is not a Headquarter key", path.display()),
                )
            })?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let key = SecretKey::generate();
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            options.open(path)?.write_all(&key.to_bytes())?;
            Ok(key)
        }
        Err(error) => Err(error),
    }
}

/// A connection to one host, made on first use and again after it breaks. Cloning shares
/// it.
#[derive(Clone)]
pub struct Link {
    inner: Arc<Inner>,
}

struct Inner {
    runtime: Runtime,
    key: SecretKey,
    address: Mutex<HostAddress>,
    /// One endpoint per link, kept across redials: a second endpoint with the same key
    /// would take the relay's place from the first.
    endpoint: Mutex<Option<Bound>>,
    live: Mutex<Option<Live>>,
    welcome: Mutex<Option<Welcome>>,
}

struct Bound {
    endpoint: Endpoint,
    relays: Relays,
}

struct Live {
    connection: Connection,
    /// The control stream's sending half, held so the gateway keeps the stream.
    _control: SendStream,
}

impl Inner {
    /// Closes the connection and the endpoint, so this computer's key is free again.
    fn shut(&self) {
        if let Some(live) = lock(&self.live).take() {
            live.connection.close(0u32.into(), b"bye");
        }
        if let Some(bound) = lock(&self.endpoint).take() {
            self.runtime.block_on(async {
                let _ = tokio::time::timeout(Duration::from_secs(1), bound.endpoint.close()).await;
            });
        }
    }

    /// The link's endpoint, bound with `relays` when there is none yet or the host's
    /// relay changed.
    fn endpoint(&self, relays: Relays) -> Result<Endpoint, LinkError> {
        let mut bound = lock(&self.endpoint);
        if let Some(current) = bound.as_ref()
            && current.relays == relays
            && !current.endpoint.is_closed()
        {
            return Ok(current.endpoint.clone());
        }
        if let Some(old) = bound.take() {
            self.runtime.block_on(async {
                let _ = tokio::time::timeout(Duration::from_secs(1), old.endpoint.close()).await;
            });
        }
        let endpoint = self
            .runtime
            .block_on(crate::bind(self.key.clone(), &relays, &[]))
            .map_err(LinkError::Unreachable)?;
        *bound = Some(Bound {
            endpoint: endpoint.clone(),
            relays,
        });
        Ok(endpoint)
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.shut();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn runtime() -> io::Result<Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("hq-link")
        .enable_all()
        .build()
}

impl Link {
    /// A link to a paired host; nothing is dialled until the first stream is opened.
    pub fn new(key: SecretKey, address: HostAddress) -> io::Result<Link> {
        Ok(Link {
            inner: Arc::new(Inner {
                runtime: runtime()?,
                key,
                address: Mutex::new(address),
                endpoint: Mutex::new(None),
                live: Mutex::new(None),
                welcome: Mutex::new(None),
            }),
        })
    }

    /// Pairs with the host that showed `code`, as a desktop named `name`, and keeps the
    /// connection.
    pub fn pair(
        key: SecretKey,
        code: &PairingCode,
        name: &str,
    ) -> Result<(Link, Welcome), LinkError> {
        let link = Link::new(key, HostAddress::from_code(code))
            .map_err(|error| LinkError::Unreachable(error.to_string()))?;
        let first = Request::Pair {
            secret: code.secret.clone(),
            name: name.to_owned(),
            kind: DeviceKind::Desktop,
        };
        let welcome = link.connect(first)?;
        Ok((link, welcome))
    }

    /// Where the host is dialled; the gateway's welcome updates its relay.
    pub fn address(&self) -> HostAddress {
        lock(&self.inner.address).clone()
    }

    /// The gateway's last welcome, once connected.
    pub fn welcome(&self) -> Option<Welcome> {
        lock(&self.inner.welcome).clone()
    }

    /// Whether a connection is up now.
    pub fn connected(&self) -> bool {
        lock(&self.inner.live)
            .as_ref()
            .is_some_and(|live| live.connection.close_reason().is_none())
    }

    /// A new stream to the host, as one host client, dialling first when needed.
    pub fn open(&self) -> Result<(Reader, Writer), LinkError> {
        let connection = self.connection()?;
        let handle = self.inner.runtime.handle().clone();
        match handle.block_on(connection.open_bi()) {
            Ok((send, recv)) => Ok(stream(&handle, send, recv)),
            Err(_) => {
                // The connection broke since it was checked: dial once more.
                if let Some(old) = lock(&self.inner.live).take() {
                    old.connection.close(0u32.into(), b"bye");
                }
                let connection = self.connection()?;
                let (send, recv) = handle
                    .block_on(connection.open_bi())
                    .map_err(|error| LinkError::Unreachable(error.to_string()))?;
                Ok(stream(&handle, send, recv))
            }
        }
    }

    /// Ends the connection and lets go of the endpoint; the next `open` dials again.
    pub fn close(&self) {
        self.inner.shut();
    }

    /// The live connection, or a new one after the `Hello` handshake.
    fn connection(&self) -> Result<Connection, LinkError> {
        if let Some(live) = lock(&self.inner.live).as_ref() {
            if live.connection.close_reason().is_none() {
                return Ok(live.connection.clone());
            }
            if removed(&live.connection) {
                return Err(LinkError::NotKnown);
            }
        }
        self.connect(Request::Hello {
            protocol: PROTOCOL_VERSION,
        })?;
        lock(&self.inner.live)
            .as_ref()
            .map(|live| live.connection.clone())
            .ok_or_else(|| LinkError::Unreachable("the connection closed".into()))
    }

    /// Dials, opens the control stream with `first` (a `Hello` or a `Pair`), and reads the
    /// gateway's welcome and the host's.
    fn connect(&self, first: Request) -> Result<Welcome, LinkError> {
        let address = self.address();
        let addr = address
            .endpoint_addr()
            .ok_or_else(|| LinkError::Unreachable("the host's key is not valid".into()))?;
        let endpoint = self.inner.endpoint(address.relays())?;
        let (connection, control, welcome) =
            self.inner.runtime.block_on(greet(&endpoint, addr, first))?;
        let live = Live {
            connection,
            _control: control,
        };
        if let Some(relay) = &welcome.relay {
            lock(&self.inner.address).relay = Some(relay.clone());
        }
        *lock(&self.inner.welcome) = Some(welcome.clone());
        if let Some(old) = lock(&self.inner.live).replace(live) {
            old.connection.close(0u32.into(), b"bye");
        }
        Ok(welcome)
    }
}

/// Whether the host closed `connection` because it removed this device.
fn removed(connection: &Connection) -> bool {
    matches!(
        connection.close_reason(),
        Some(iroh::endpoint::ConnectionError::ApplicationClosed(close))
            if close.reason.as_ref() == b"removed"
    )
}

async fn read_response(recv: &mut RecvStream) -> Result<Response, LinkError> {
    loop {
        let frame = tokio::time::timeout(ANSWER_WAIT, crate::read_frame(recv))
            .await
            .map_err(|_| LinkError::Unreachable("the host did not answer".into()))?
            .map_err(|error| LinkError::Unreachable(error.to_string()))?;
        match frame {
            Some(Frame::Control(bytes)) => {
                return Response::parse(&bytes)
                    .map_err(|error| LinkError::Unreachable(error.to_string()));
            }
            Some(_) => continue,
            None => {
                return Err(LinkError::Unreachable(
                    "the host closed the connection".into(),
                ));
            }
        }
    }
}

/// Dials the host and greets it; the connection and its control stream once it answers.
async fn greet(
    endpoint: &Endpoint,
    addr: EndpointAddr,
    first: Request,
) -> Result<(Connection, SendStream, Welcome), LinkError> {
    let connection = tokio::time::timeout(DIAL_WAIT, endpoint.connect(addr, ALPN))
        .await
        .map_err(|_| LinkError::Unreachable("the host did not answer".into()))?
        .map_err(|error| LinkError::Unreachable(error.to_string()))?;
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|error| LinkError::Unreachable(error.to_string()))?;
    let pairing = matches!(first, Request::Pair { .. });
    crate::write_frame(&mut send, &Frame::request(&first))
        .await
        .map_err(|error| LinkError::Unreachable(error.to_string()))?;
    if pairing {
        match read_response(&mut recv).await? {
            Response::Paired { .. } => {}
            Response::Error { message, .. } => return Err(LinkError::Refused(message)),
            other => {
                return Err(LinkError::Unreachable(format!(
                    "unexpected answer {other:?}"
                )));
            }
        }
    }
    let welcome = match read_response(&mut recv).await? {
        Response::RemoteWelcome {
            host,
            relay,
            protocol,
            version,
        } => {
            if protocol != PROTOCOL_VERSION {
                connection.close(0u32.into(), b"versions differ");
                return Err(LinkError::Mismatch {
                    host_protocol: protocol,
                    host_version: version,
                });
            }
            Welcome {
                host,
                relay,
                version,
            }
        }
        Response::Error {
            code: ErrorCode::NotAllowed,
            ..
        } => return Err(LinkError::NotKnown),
        other => {
            return Err(LinkError::Unreachable(format!(
                "unexpected answer {other:?}"
            )));
        }
    };
    if pairing {
        // The paired stream goes on as a host client: greet the host on it too.
        crate::write_frame(
            &mut send,
            &Frame::request(&Request::Hello {
                protocol: PROTOCOL_VERSION,
            }),
        )
        .await
        .map_err(|error| LinkError::Unreachable(error.to_string()))?;
    }
    match read_response(&mut recv).await? {
        Response::Welcome { .. } => {}
        Response::Error { message, .. } => return Err(LinkError::Refused(message)),
        other => {
            return Err(LinkError::Unreachable(format!(
                "unexpected answer {other:?}"
            )));
        }
    }
    // The control stream stays open while the link lives; whatever the host sends on it
    // is not needed.
    tokio::spawn(async move {
        let mut sink = [0u8; 1024];
        while let Ok(Some(_)) = recv.read(&mut sink).await {}
    });
    Ok((connection, send, welcome))
}

fn stream(handle: &Handle, send: SendStream, recv: RecvStream) -> (Reader, Writer) {
    (
        Reader {
            handle: handle.clone(),
            recv,
        },
        Writer {
            handle: handle.clone(),
            send,
        },
    )
}

/// A stream's receiving half, read with blocking calls.
pub struct Reader {
    handle: Handle,
    recv: RecvStream,
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.handle.block_on(self.recv.read(buf)) {
            Ok(Some(read)) => Ok(read),
            Ok(None) => Ok(0),
            Err(error) => Err(io::Error::new(io::ErrorKind::ConnectionReset, error)),
        }
    }
}

/// A stream's sending half, written with blocking calls.
pub struct Writer {
    handle: Handle,
    send: SendStream,
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.handle
            .block_on(self.send.write(buf))
            .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Writer {
    /// Ends the stream: the host's client on it goes away, and the reader then ends.
    pub fn finish(&mut self) {
        let _ = self.send.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_client_key_is_private_and_kept() {
        let dir = std::env::temp_dir().join(format!("hq-client-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("client.key");
        let first = client_key(&path).unwrap();
        #[cfg(unix)]
        {
            let mode = std::fs::metadata(&path).unwrap().permissions();
            assert_eq!(
                std::os::unix::fs::PermissionsExt::mode(&mode) & 0o777,
                0o600
            );
        }
        assert_eq!(client_key(&path).unwrap().public(), first.public(), "kept");
        std::fs::write(&path, b"short").unwrap();
        assert!(client_key(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_host_address_round_trips_through_a_code() {
        let key = SecretKey::generate();
        let code = PairingCode::new(key.public(), None, vec!["127.0.0.1:4000".parse().unwrap()]);
        let address = HostAddress::from_code(&code);
        assert_eq!(address.id, key.public().to_string());
        let addr = address.endpoint_addr().unwrap();
        assert_eq!(addr.id, key.public());
    }
}
