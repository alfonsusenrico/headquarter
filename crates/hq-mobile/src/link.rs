//! The connection to the computer on screen (add-android-app D4, D6): one iroh stream
//! that carries the host's feed, answers and the open terminal's output, reconnected
//! with backoff while the app is on screen.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use hq_protocol::{ErrorCode, Frame, PROTOCOL_VERSION, Request, Response};
use hq_remote::Relays;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::{Endpoint, EndpointAddr, PublicKey, RelayUrl, SecretKey};
use tokio::sync::{mpsc, oneshot};

use crate::computers::Computer;
use crate::{LinkStatus, Shared, Taken};

/// How long dialling a computer may take before it counts as unreachable.
const DIAL_WAIT: Duration = Duration::from_secs(15);
/// The longest wait between reconnects (D6).
const MAX_DELAY: Duration = Duration::from_secs(15);

pub(crate) enum Command {
    /// A request the host answers. The answer goes back on `answer`.
    Ask {
        request: Request,
        answer: oneshot::Sender<Response>,
    },
    /// A request the host does not answer (`Resize`, `Focus`).
    Tell(Request),
    Input(Vec<u8>),
}

/// How long a stopping link may take to tell the computer it is leaving.
const STOP_WAIT: Duration = Duration::from_secs(2);

/// A running connection to one computer. Dropping it disconnects at once, and [`Link::stop`]
/// says goodbye first.
pub(crate) struct Link {
    pub tx: mpsc::UnboundedSender<Command>,
    pub computer: String,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for Link {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl Link {
    pub fn start(
        runtime: &tokio::runtime::Runtime,
        shared: Arc<Shared>,
        key: SecretKey,
        computer: String,
    ) -> Link {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = runtime.spawn(run(shared, key, computer.clone(), rx));
        Link {
            tx,
            computer,
            task: Some(task),
        }
    }

    /// Ends the link. A connected one closes its connection itself, so the computer hears
    /// the phone leave at once rather than after QUIC's idle timeout. One still dialling or
    /// waiting to retry stops outright.
    pub fn stop(mut self, runtime: &tokio::runtime::Runtime, connected: bool) {
        let Some(task) = self.task.take() else {
            return;
        };
        // Dropping the sender ends the serving loop with `Ending::Stopped`.
        drop(self);
        if !connected {
            task.abort();
            return;
        }
        runtime.spawn(async move {
            let abort = task.abort_handle();
            if tokio::time::timeout(STOP_WAIT, task).await.is_err() {
                abort.abort();
            }
        });
    }
}

/// How one connection ended.
enum Ending {
    /// Dropped or unreachable. Try again.
    Lost { was_connected: bool },
    /// No point retrying until something changes: the phone was removed, or the versions
    /// differ.
    Final(LinkStatus),
    /// The app let the link go.
    Stopped,
}

async fn run(
    shared: Arc<Shared>,
    key: SecretKey,
    id: String,
    mut rx: mpsc::UnboundedReceiver<Command>,
) {
    let mut delay = Duration::from_secs(1);
    let mut attempts = 0u32;
    let mut since: Option<u64> = None;
    loop {
        let Some(computer) = shared.computer(&id) else {
            return;
        };
        shared.set_link(match since {
            None => LinkStatus::Connecting,
            Some(since_ms) => LinkStatus::Reconnecting { since_ms, attempts },
        });
        match connect(&shared, &key, &computer, &mut rx).await {
            Ending::Stopped => return,
            Ending::Final(status) => {
                shared.set_link(status);
                // Requests fail until the app picks another computer or pairs again.
                while let Some(command) = rx.recv().await {
                    drop(command);
                }
                return;
            }
            Ending::Lost { was_connected } => {
                // A link being stopped, or for a computer just forgotten, says nothing more:
                // the status now belongs to whatever replaced it.
                if rx.is_closed() || shared.computer(&id).is_none() {
                    return;
                }
                if was_connected {
                    delay = Duration::from_secs(1);
                    attempts = 0;
                    since = Some(hq_signals::now_ms());
                } else if since.is_none() {
                    since = Some(hq_signals::now_ms());
                }
            }
        }
        attempts += 1;
        shared.set_link(LinkStatus::Reconnecting {
            since_ms: since.unwrap_or_default(),
            attempts,
        });
        let sleep = tokio::time::sleep(delay);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                () = &mut sleep => break,
                command = rx.recv() => match command {
                    // Asked while away: the answer channel drops, so the app hears "not
                    // connected" at once.
                    Some(command) => drop(command),
                    None => return,
                },
            }
        }
        delay = (delay * 2).min(MAX_DELAY);
    }
}

/// The relays the phone binds with to reach `computer`: its home relay, or none.
pub(crate) fn relays_for(relay: Option<&str>) -> Relays {
    if std::env::var("HQ_REMOTE_RELAYS").is_ok_and(|value| value == "none") {
        return Relays::Disabled;
    }
    match relay.and_then(|relay| relay.parse::<RelayUrl>().ok()) {
        Some(url) => Relays::Custom(vec![url]),
        None => Relays::Disabled,
    }
}

pub(crate) fn address(computer: &Computer) -> Option<EndpointAddr> {
    let host: PublicKey = computer.id.parse().ok()?;
    let mut addr = EndpointAddr::new(host);
    if let Some(relay) = computer
        .relay
        .as_deref()
        .and_then(|relay| relay.parse().ok())
    {
        addr = addr.with_relay_url(relay);
    }
    for ip in computer.addrs.iter().filter_map(|ip| ip.parse().ok()) {
        addr = addr.with_ip_addr(ip);
    }
    Some(addr)
}

/// Binds and dials, then opens the one stream the gateway serves.
pub(crate) async fn dial(
    key: &SecretKey,
    relays: &Relays,
    addr: EndpointAddr,
) -> Result<(Endpoint, Connection, SendStream, RecvStream), String> {
    let endpoint = hq_remote::bind(key.clone(), relays, &[]).await?;
    let connection = tokio::time::timeout(DIAL_WAIT, endpoint.connect(addr, hq_remote::ALPN))
        .await
        .map_err(|_| "the computer did not answer".to_owned())?
        .map_err(|error| error.to_string())?;
    let (send, recv) = connection
        .open_bi()
        .await
        .map_err(|error| error.to_string())?;
    Ok((endpoint, connection, send, recv))
}

/// What a version difference means for the owner (D4).
pub(crate) fn mismatch(host_protocol: u32, host_version: String) -> Option<LinkStatus> {
    match host_protocol.cmp(&PROTOCOL_VERSION) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Less => Some(LinkStatus::ComputerTooOld {
            version: host_version,
        }),
        std::cmp::Ordering::Greater => Some(LinkStatus::AppTooOld {
            version: host_version,
        }),
    }
}

async fn read_response(recv: &mut RecvStream) -> Option<Response> {
    loop {
        match hq_remote::read_frame(recv).await {
            Ok(Some(Frame::Control(bytes))) => return Response::parse(&bytes).ok(),
            Ok(Some(_)) => continue,
            _ => return None,
        }
    }
}

async fn connect(
    shared: &Arc<Shared>,
    key: &SecretKey,
    computer: &Computer,
    rx: &mut mpsc::UnboundedReceiver<Command>,
) -> Ending {
    let lost = Ending::Lost {
        was_connected: false,
    };
    let Some(addr) = address(computer) else {
        return Ending::Final(LinkStatus::NotKnown);
    };
    let relays = relays_for(computer.relay.as_deref());
    let Ok((endpoint, connection, mut send, mut recv)) = dial(key, &relays, addr).await else {
        return lost;
    };
    let hello = Request::Hello {
        protocol: PROTOCOL_VERSION,
    };
    if hq_remote::write_frame(&mut send, &Frame::request(&hello))
        .await
        .is_err()
    {
        return lost;
    }
    // The gateway's welcome comes first. A refusal instead means the computer no longer
    // knows this phone.
    match read_response(&mut recv).await {
        Some(Response::RemoteWelcome {
            host,
            relay,
            protocol,
            version,
        }) => {
            shared.welcomed(&computer.id, host, relay);
            if let Some(status) = mismatch(protocol, version) {
                connection.close(0u32.into(), b"versions differ");
                return Ending::Final(status);
            }
        }
        Some(Response::Error {
            code: ErrorCode::NotAllowed,
            ..
        }) => return Ending::Final(LinkStatus::NotKnown),
        _ => return lost,
    }
    if !matches!(
        read_response(&mut recv).await,
        Some(Response::Welcome { .. })
    ) {
        return lost;
    }
    let mut session = Session {
        shared,
        send,
        recv,
        pending: VecDeque::new(),
    };
    // Subscribe before listing, as the desktop does: a change between the two is then
    // not missed.
    for request in [
        Request::Subscribe { usage: true },
        Request::List {},
        Request::Usage {},
    ] {
        match session.ask_now(request).await {
            Some(Response::Sessions { sessions, groups }) => shared.listed(sessions, groups),
            Some(Response::Usage { agents }) => shared.usage(agents),
            Some(Response::Done) => {}
            _ => return lost,
        }
    }
    shared.connected(connection.clone());
    if let Some(open) = shared.reopen() {
        let request = match open.shell {
            true => Request::AttachShell {
                session: open.session,
                cols: open.cols,
                rows: open.rows,
            },
            false => Request::Attach {
                session: open.session,
                cols: open.cols,
                rows: open.rows,
            },
        };
        let (answer, _ignored) = oneshot::channel();
        if session.write(&Frame::request(&request)).await.is_err() {
            return Ending::Lost {
                was_connected: true,
            };
        }
        session.pending.push_back(answer);
    }
    let ending = match session.serve(rx).await {
        // The computer closed the link because it removed this phone: it will not answer
        // again (remote control stops with its last phone), so say so instead of retrying.
        Ending::Lost { .. } if removed_by_computer(&connection) => {
            Ending::Final(LinkStatus::NotKnown)
        }
        ending => ending,
    };
    shared.disconnected(&connection);
    connection.close(0u32.into(), b"bye");
    endpoint.close().await;
    ending
}

/// Whether the computer closed `connection` because it removed this phone (the host closes
/// with the reason `removed`).
fn removed_by_computer(connection: &Connection) -> bool {
    matches!(
        connection.close_reason(),
        Some(iroh::endpoint::ConnectionError::ApplicationClosed(close))
            if close.reason.as_ref() == b"removed"
    )
}

struct Session<'a> {
    shared: &'a Arc<Shared>,
    send: SendStream,
    recv: RecvStream,
    /// Answers owed, oldest first: the host answers in order.
    pending: VecDeque<oneshot::Sender<Response>>,
}

impl Session<'_> {
    async fn write(&mut self, frame: &Frame) -> Result<(), ()> {
        hq_remote::write_frame(&mut self.send, frame)
            .await
            .map_err(|_| ())
    }

    /// Sends a request during setup and waits for its answer, taking in the changes that
    /// arrive first.
    async fn ask_now(&mut self, request: Request) -> Option<Response> {
        self.write(&Frame::request(&request)).await.ok()?;
        loop {
            match hq_remote::read_frame(&mut self.recv).await {
                Ok(Some(Frame::Control(bytes))) => {
                    let response = Response::parse(&bytes).ok()?;
                    if let Taken::Answer(answer) = self.shared.take_in(response) {
                        return Some(*answer);
                    }
                }
                Ok(Some(_)) => continue,
                _ => return None,
            }
        }
    }

    async fn serve(&mut self, rx: &mut mpsc::UnboundedReceiver<Command>) -> Ending {
        let lost = Ending::Lost {
            was_connected: true,
        };
        loop {
            tokio::select! {
                frame = hq_remote::read_frame(&mut self.recv) => match frame {
                    Ok(Some(Frame::Control(bytes))) => {
                        let Ok(response) = Response::parse(&bytes) else { continue };
                        match self.shared.take_in(response) {
                            Taken::Change => {}
                            Taken::Answer(answer) => {
                                if let Some(waiting) = self.pending.pop_front() {
                                    let _ = waiting.send(*answer);
                                }
                            }
                            Taken::Read(session) => {
                                let (answer, _ignored) = oneshot::channel();
                                self.pending.push_back(answer);
                                if self.write(&Frame::request(&Request::Seen { session })).await.is_err() {
                                    return lost;
                                }
                            }
                        }
                    }
                    Ok(Some(Frame::Output(bytes))) => {
                        let replies = self.shared.output(&bytes);
                        if !replies.is_empty() && self.write(&Frame::Input(replies)).await.is_err() {
                            return lost;
                        }
                    }
                    Ok(Some(Frame::Input(_))) => {}
                    _ => return lost,
                },
                command = rx.recv() => {
                    let written = match command {
                        None => return Ending::Stopped,
                        Some(Command::Ask { request, answer }) => {
                            self.pending.push_back(answer);
                            self.write(&Frame::request(&request)).await
                        }
                        Some(Command::Tell(request)) => self.write(&Frame::request(&request)).await,
                        Some(Command::Input(bytes)) => self.write(&Frame::Input(bytes)).await,
                    };
                    if written.is_err() {
                        return lost;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_difference_names_the_side_to_update() {
        assert_eq!(mismatch(PROTOCOL_VERSION, "0.1.14".into()), None);
        assert_eq!(
            mismatch(PROTOCOL_VERSION - 1, "0.1.13".into()),
            Some(LinkStatus::ComputerTooOld {
                version: "0.1.13".into()
            })
        );
        assert_eq!(
            mismatch(PROTOCOL_VERSION + 1, "0.2.0".into()),
            Some(LinkStatus::AppTooOld {
                version: "0.2.0".into()
            })
        );
    }

    #[test]
    fn a_computer_is_dialled_through_its_relay_and_addresses() {
        let host = SecretKey::generate().public();
        let computer = Computer {
            id: host.to_string(),
            name: "MacBook".into(),
            relay: Some("https://relay.example.com./".into()),
            addrs: vec!["192.168.1.5:41000".into()],
            paired_at: 1,
            last_seen_at: None,
        };
        let addr = address(&computer).unwrap();
        assert_eq!(addr.id, host);
        assert_eq!(addr.relay_urls().count(), 1);
        assert_eq!(addr.ip_addrs().count(), 1);
        let unknown = Computer {
            id: "not a key".into(),
            ..computer
        };
        assert!(address(&unknown).is_none());
    }
}
