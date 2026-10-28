//! Connection to the Headquarter host, starting it on demand. Shared by the `hq`
//! command and the desktop app.

use std::collections::VecDeque;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
pub use hq_daemon::paths::HostPaths;
use hq_protocol::{
    AgentUsage, ErrorCode, Footprint, Frame, GroupColor, GroupInfo, PROTOCOL_VERSION, RemoveWay,
    Request, Response, SessionFootprint, SessionInfo, host_protocol_in_refusal, read_frame,
    write_frame,
};
use hq_signals::details::SessionDetails;
use hq_signals::event::Agent;

/// How long a client waits for a host it just started to accept connections. It covers
/// a host that first waits for a stopping one to exit (fix-host-start-after-stop); the
/// wait ends as soon as the host answers.
const START_TIMEOUT: Duration = Duration::from_secs(12);

pub struct Client {
    pub reader: BufReader<UnixStream>,
    pub writer: UnixStream,
    /// The version the host named in its welcome; empty until it answered.
    pub host_version: String,
}

impl Client {
    pub fn connect_or_start(paths: &HostPaths) -> Result<Client> {
        match Client::greet(paths, PROTOCOL_VERSION) {
            Ok(Greeting::Welcome(client)) => return Ok(client),
            // Starting another host would be refused; say why instead.
            Ok(Greeting::OtherVersion { message, .. }) => bail!(message),
            Err(_) => {}
        }
        start_host()?;
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            match Client::connect(paths) {
                Ok(client) => return Ok(client),
                Err(error) if Instant::now() >= deadline => {
                    return Err(error).context("the Headquarter host did not start");
                }
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }

    /// Connects to a running host without starting one.
    pub fn connect(paths: &HostPaths) -> Result<Client> {
        match Client::greet(paths, PROTOCOL_VERSION)? {
            Greeting::Welcome(client) => Ok(client),
            Greeting::OtherVersion { message, .. } => bail!(message),
        }
    }

    /// Connects and says `Hello` in `protocol`; an error means no host answered.
    pub fn greet(paths: &HostPaths, protocol: u32) -> Result<Greeting> {
        let stream = UnixStream::connect(&paths.control_socket)
            .with_context(|| format!("connect to {}", paths.control_socket.display()))?;
        let writer = stream.try_clone()?;
        let mut client = Client {
            reader: BufReader::new(stream),
            writer,
            host_version: String::new(),
        };
        match client.request(&Request::Hello { protocol })? {
            Response::Welcome { version, .. } => {
                client.host_version = version;
                Ok(Greeting::Welcome(client))
            }
            Response::Error {
                code: ErrorCode::ProtocolMismatch,
                message,
            } => Ok(Greeting::OtherVersion {
                protocol: host_protocol_in_refusal(&message),
                message,
            }),
            Response::Error { message, .. } => bail!(message),
            other => bail!("unexpected greeting from the host: {other:?}"),
        }
    }

    pub fn send(&mut self, request: &Request) -> Result<()> {
        write_frame(&mut self.writer, &Frame::request(request)).map_err(|error| anyhow!(error))
    }

    /// Sends a request and returns the next control response.
    pub fn request(&mut self, request: &Request) -> Result<Response> {
        self.send(request)?;
        self.next_response()
    }

    pub fn next_response(&mut self) -> Result<Response> {
        loop {
            match read_frame(&mut self.reader).map_err(|error| anyhow!(error))? {
                Some(Frame::Control(bytes)) => {
                    return Response::parse(&bytes).map_err(|error| anyhow!(error));
                }
                Some(_) => continue,
                None => bail!("the Headquarter host closed the connection"),
            }
        }
    }
}

/// How a running host answered a client's `Hello`.
pub enum Greeting {
    Welcome(Client),
    /// The host speaks another protocol version; `protocol` is that version when the
    /// refusal names it.
    OtherVersion {
        protocol: Option<u32>,
        message: String,
    },
}

/// Stops a host that speaks protocol `protocol`, with the `Hello` and `Stop` every
/// version understands. Its sessions end with it.
pub fn stop_other_version(paths: &HostPaths, protocol: u32) -> Result<()> {
    let mut client = match Client::greet(paths, protocol)? {
        Greeting::Welcome(client) => client,
        Greeting::OtherVersion { message, .. } => bail!(message),
    };
    match client.request(&Request::Stop {})? {
        Response::Error { message, .. } => bail!(message),
        _ => Ok(()),
    }
}

/// Every session the host knows, with the owner's groups.
pub struct Catalog {
    pub sessions: Vec<SessionInfo>,
    pub groups: Vec<GroupInfo>,
    /// Usage readings, when the feed was opened with usage.
    pub usage: Vec<AgentUsage>,
}

/// A change the host reports to subscribers.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Session(SessionInfo),
    /// Every group, in the owner's order, after a change.
    Groups(Vec<GroupInfo>),
    /// Every agent's usage after a new reading.
    Usage(Vec<AgentUsage>),
    /// The followed session's details.
    Details {
        session: String,
        details: SessionDetails,
    },
}

/// Tells the host what this feed's client shows: which session's details, and whether
/// its window can be seen. Writes on the feed's connection, so the answers arrive
/// through the feed.
pub struct FeedControl {
    writer: UnixStream,
}

impl FeedControl {
    pub fn follow(&mut self, session: Option<&str>) -> Result<()> {
        self.send(&Request::FollowDetails {
            session: session.map(str::to_owned),
        })
    }

    pub fn set_visible(&mut self, visible: bool) -> Result<()> {
        self.send(&Request::Visible { visible })
    }

    fn send(&mut self, request: &Request) -> Result<()> {
        write_frame(&mut self.writer, &Frame::request(request)).map_err(|error| anyhow!(error))
    }
}

/// Every session, then each change as the host reports it.
pub struct SessionFeed {
    client: Client,
    /// Changes that arrived before the opening answers, reported first and in order.
    early: VecDeque<Change>,
}

impl SessionFeed {
    /// Connects (starting the host if needed), subscribes, and lists every session;
    /// with `usage`, also reads usage and follows its changes.
    pub fn open(paths: &HostPaths, usage: bool) -> Result<(SessionFeed, Catalog)> {
        SessionFeed::start(Client::connect_or_start(paths)?, usage)
    }

    /// Subscribes before listing: the host tells only subscribers about a change, so
    /// one made between a list and a later subscription would never reach this client.
    fn start(client: Client, usage: bool) -> Result<(SessionFeed, Catalog)> {
        let mut feed = SessionFeed {
            client,
            early: VecDeque::new(),
        };
        match feed.answer(&Request::Subscribe { usage })? {
            Response::Done => {}
            other => bail!("unexpected answer to subscribe: {other:?}"),
        }
        let (sessions, groups) = match feed.answer(&Request::List {})? {
            Response::Sessions { sessions, groups } => (sessions, groups),
            other => bail!("unexpected answer to list: {other:?}"),
        };
        let usage_now = if usage {
            match feed.answer(&Request::Usage {})? {
                Response::Usage { agents } => agents,
                other => bail!("unexpected answer to usage: {other:?}"),
            }
        } else {
            Vec::new()
        };
        let catalog = Catalog {
            sessions,
            groups,
            usage: usage_now,
        };
        Ok((feed, catalog))
    }

    /// Sends `request` and returns its answer, keeping the changes that arrive first.
    fn answer(&mut self, request: &Request) -> Result<Response> {
        self.client.send(request)?;
        loop {
            match incoming(self.client.next_response()?) {
                Incoming::Change(change) => self.early.push_back(change),
                Incoming::Answer(response) => return expect_ok(response),
            }
        }
    }

    /// A handle for telling the host what this feed's client shows.
    pub fn control(&self) -> Result<FeedControl> {
        Ok(FeedControl {
            writer: self.client.writer.try_clone()?,
        })
    }

    /// The next change, or `None` when the host closes the connection.
    pub fn next_change(&mut self) -> Result<Option<Change>> {
        if let Some(change) = self.early.pop_front() {
            return Ok(Some(change));
        }
        loop {
            let frame = read_frame(&mut self.client.reader).map_err(|error| anyhow!(error))?;
            match frame {
                Some(Frame::Control(bytes)) => {
                    if let Ok(Incoming::Change(change)) = Response::parse(&bytes).map(incoming) {
                        return Ok(Some(change));
                    }
                }
                Some(_) => {}
                None => return Ok(None),
            }
        }
    }
}

/// What the host sent a feed: a change, or the answer to the feed's request.
enum Incoming {
    Change(Change),
    Answer(Response),
}

fn incoming(response: Response) -> Incoming {
    match response {
        Response::SessionChanged { session } => Incoming::Change(Change::Session(session)),
        Response::GroupsChanged { groups } => Incoming::Change(Change::Groups(groups)),
        Response::UsageChanged { agents } => Incoming::Change(Change::Usage(agents)),
        Response::DetailsChanged { session, details } => {
            Incoming::Change(Change::Details { session, details })
        }
        other => Incoming::Answer(other),
    }
}

/// Sends a request whose success answer is `Done`.
fn request_done(paths: &HostPaths, request: &Request) -> Result<()> {
    let mut client = Client::connect_or_start(paths)?;
    match expect_ok(client.request(request)?)? {
        Response::Done => Ok(()),
        other => bail!("unexpected answer: {other:?}"),
    }
}

/// Puts sessions (names or id prefixes) in the sidebar; all or nothing.
pub fn add_to_sidebar(paths: &HostPaths, sessions: &[String]) -> Result<()> {
    request_done(
        paths,
        &Request::Add {
            sessions: sessions.to_vec(),
        },
    )
}

/// Takes sessions out of the sidebar; their history is untouched.
pub fn remove_from_sidebar(paths: &HostPaths, sessions: &[String]) -> Result<()> {
    request_done(
        paths,
        &Request::Remove {
            sessions: sessions.to_vec(),
        },
    )
}

/// Adds a group after the others, named `name` or "New Group #n"; with `session`, that
/// sidebar session moves into it.
pub fn new_group(
    paths: &HostPaths,
    name: Option<&str>,
    session: Option<&str>,
) -> Result<GroupInfo> {
    let mut client = Client::connect_or_start(paths)?;
    let response = client.request(&Request::NewGroup {
        name: name.map(str::to_owned),
        session: session.map(str::to_owned),
    })?;
    match expect_ok(response)? {
        Response::GroupCreated { group } => Ok(group),
        other => bail!("unexpected answer to new group: {other:?}"),
    }
}

pub fn rename_group(paths: &HostPaths, group: i64, name: &str) -> Result<()> {
    request_done(
        paths,
        &Request::RenameGroup {
            group,
            name: name.to_owned(),
        },
    )
}

/// Deletes a group; its sessions stay in the sidebar, in no group.
pub fn delete_group(paths: &HostPaths, group: i64) -> Result<()> {
    request_done(paths, &Request::DeleteGroup { group })
}

/// Moves a group to just before `before`, or to the end.
pub fn move_group(paths: &HostPaths, group: i64, before: Option<i64>) -> Result<()> {
    request_done(paths, &Request::MoveGroup { group, before })
}

pub fn collapse_group(paths: &HostPaths, group: i64, collapsed: bool) -> Result<()> {
    request_done(paths, &Request::CollapseGroup { group, collapsed })
}

/// Picks a group's colour by palette name, or `None` for automatic.
pub fn set_group_color(paths: &HostPaths, group: i64, color: Option<GroupColor>) -> Result<()> {
    request_done(
        paths,
        &Request::SetGroupColor {
            group,
            color: color.map(|color| color.name().to_owned()),
        },
    )
}

/// Puts a sidebar session in a group, or in none.
pub fn set_group(paths: &HostPaths, session: &str, group: Option<i64>) -> Result<()> {
    request_done(
        paths,
        &Request::SetGroup {
            session: session.to_owned(),
            group,
        },
    )
}

/// A remote control request (status, on or off, devices, removal); errors become `Err`.
pub fn remote(paths: &HostPaths, request: &Request) -> Result<Response> {
    let mut client = Client::connect_or_start(paths)?;
    expect_ok(client.request(request)?)
}

/// Asks the host for a pairing code and its lifetime in seconds. The returned client hears
/// when the code is used or expires; closing it withdraws the code (add-remote-gateway D13).
pub fn begin_pairing(paths: &HostPaths) -> Result<(Client, String, u64)> {
    let mut client = Client::connect_or_start(paths)?;
    match expect_ok(client.request(&Request::RemotePair {})?)? {
        Response::RemotePairing {
            code,
            expires_in_secs,
        } => Ok((client, code, expires_in_secs)),
        other => bail!("unexpected answer from the host: {other:?}"),
    }
}

/// Tells the host the owner has seen the session's latest state.
pub fn mark_seen(paths: &HostPaths, session: &str) -> Result<()> {
    request_done(
        paths,
        &Request::Seen {
            session: session.to_owned(),
        },
    )
}

pub fn rename(paths: &HostPaths, session: &str, name: &str) -> Result<()> {
    request_done(
        paths,
        &Request::Rename {
            session: session.to_owned(),
            name: name.to_owned(),
        },
    )
}

/// Closes a session: its agent is stopped with its own keys and the session leaves the
/// sidebar once the agent has exited.
pub fn close(paths: &HostPaths, session: &str) -> Result<()> {
    request_done(
        paths,
        &Request::Close {
            session: session.to_owned(),
        },
    )
}

/// Stops the host and every session it runs; those running start again with the next
/// host (add-inactive-sessions).
pub fn stop_host(paths: &HostPaths) -> Result<()> {
    let mut client = Client::connect(paths)?;
    match expect_ok(client.request(&Request::Stop {})?)? {
        Response::Done => Ok(()),
        other => bail!("unexpected answer: {other:?}"),
    }
}

/// The version of the running host, without starting one.
pub fn host_version(paths: &HostPaths) -> Result<String> {
    Ok(Client::connect(paths)?.host_version)
}

/// Every enabled agent's usage, read now when due.
pub fn usage(paths: &HostPaths) -> Result<Vec<AgentUsage>> {
    let mut client = Client::connect_or_start(paths)?;
    match expect_ok(client.request(&Request::Usage {})?)? {
        Response::Usage { agents } => Ok(agents),
        other => bail!("unexpected answer to usage: {other:?}"),
    }
}

/// Asks for a new reading; subscribers to usage receive it.
pub fn refresh_usage(paths: &HostPaths) -> Result<()> {
    request_done(paths, &Request::RefreshUsage {})
}

/// The host's own CPU and memory, and each running session's (add-usage-window D1).
pub fn resources(paths: &HostPaths) -> Result<(Footprint, Vec<SessionFootprint>)> {
    let mut client = Client::connect_or_start(paths)?;
    match expect_ok(client.request(&Request::Resources {})?)? {
        Response::Resources { host, sessions } => Ok((host, sessions)),
        other => bail!("unexpected answer to resources: {other:?}"),
    }
}

/// Removes a past conversation from its agent, the way that agent allows.
pub fn remove_conversation(paths: &HostPaths, session: &str, way: RemoveWay) -> Result<()> {
    request_done(
        paths,
        &Request::RemoveConversation {
            session: session.to_owned(),
            way,
        },
    )
}

/// Continues an ended session's conversation in Headquarter.
pub fn resume(paths: &HostPaths, session: &str) -> Result<SessionInfo> {
    let mut client = Client::connect_or_start(paths)?;
    let response = client.request(&Request::Resume {
        session: session.to_owned(),
        term: Some("xterm-256color".to_owned()),
    })?;
    match expect_ok(response)? {
        Response::Created { session } => Ok(session),
        other => bail!("unexpected answer to resume: {other:?}"),
    }
}

/// Stops a conversation running in Claude Code's background and resumes it in Headquarter.
pub fn bring_in(paths: &HostPaths, session: &str) -> Result<SessionInfo> {
    let mut client = Client::connect_or_start(paths)?;
    let response = client.request(&Request::BringIn {
        session: session.to_owned(),
        term: Some("xterm-256color".to_owned()),
    })?;
    match expect_ok(response)? {
        Response::Created { session } => Ok(session),
        other => bail!("unexpected answer to bring in: {other:?}"),
    }
}

/// Starts `agent` in `folder` as a hosted session, in `group` when given.
pub fn create_session(
    paths: &HostPaths,
    agent: Agent,
    folder: &Path,
    group: Option<i64>,
) -> Result<SessionInfo> {
    let mut client = Client::connect_or_start(paths)?;
    let response = client.request(&Request::New {
        agent,
        cwd: folder.to_string_lossy().into_owned(),
        args: Vec::new(),
        term: Some("xterm-256color".to_owned()),
        name: None,
        group,
    })?;
    match expect_ok(response)? {
        Response::Created { session } => Ok(session),
        other => bail!("unexpected answer to new: {other:?}"),
    }
}

/// Attaches to a session; the returned client then carries its output and input.
pub fn attach(paths: &HostPaths, session: &str, cols: u16, rows: u16) -> Result<Client> {
    let mut client = Client::connect_or_start(paths)?;
    let response = client.request(&Request::Attach {
        session: session.to_owned(),
        cols,
        rows,
    })?;
    match expect_ok(response)? {
        Response::Attached { .. } => Ok(client),
        other => bail!("unexpected answer to attach: {other:?}"),
    }
}

/// Attaches to a session's shell, starting it in the session's folder if needed.
pub fn attach_shell(paths: &HostPaths, session: &str, cols: u16, rows: u16) -> Result<Client> {
    let mut client = Client::connect_or_start(paths)?;
    let response = client.request(&Request::AttachShell {
        session: session.to_owned(),
        cols,
        rows,
    })?;
    match expect_ok(response)? {
        Response::Attached { .. } => Ok(client),
        other => bail!("unexpected answer to attach shell: {other:?}"),
    }
}

/// Ends a session's shell, if one runs.
pub fn close_shell(paths: &HostPaths, session: &str) -> Result<()> {
    request_done(
        paths,
        &Request::CloseShell {
            session: session.to_owned(),
        },
    )
}

/// Turns an error response into an error for the caller.
pub fn expect_ok(response: Response) -> Result<Response> {
    match response {
        Response::Error { code, message, .. } => Err(Refused { code, message }.into()),
        other => Ok(other),
    }
}

/// The host refused a request: why, in its words, and its code, so a client can act on a
/// particular refusal (`anyhow::Error::downcast_ref`).
#[derive(Debug)]
pub struct Refused {
    pub code: ErrorCode,
    pub message: String,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refused {}

/// Starts the host as `<hq> daemon`, where `<hq>` is this executable when it is `hq`
/// (or `headquarter`), or the `hq` beside it otherwise (the desktop app).
fn start_host() -> Result<()> {
    let exe = std::env::current_exe().context("locate this executable")?;
    let name = exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let hq = if name == "hq" || name == "headquarter" {
        exe.clone()
    } else {
        exe.with_file_name("hq")
    };
    Command::new(hq)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("start the Headquarter host")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hq_protocol::{External, Origin};
    use hq_signals::status::Status;

    fn session(external: Option<External>) -> SessionInfo {
        SessionInfo {
            id: "row-1".to_owned(),
            agent: Agent::Claude,
            cwd: "/w".to_owned(),
            name: "claude-1".to_owned(),
            agent_session_id: Some("conv-1".to_owned()),
            status: Status::Inactive,
            needs_reason: None,
            created_at: 0,
            last_activity_at: 0,
            ended_at: None,
            live: false,
            origin: Origin::History,
            title: None,
            external,
            in_sidebar: false,
            group: None,
            owner_named: false,
            starting: false,
            start_error: None,
            offered: true,
            phone: None,
        }
    }

    /// Answers like the host, with the outside scan linking the row to a process in
    /// another terminal right after the list is answered. Like the host, it tells only
    /// subscribed connections about the change.
    fn host_linking_after_the_list(stream: UnixStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut send = |response: &Response| {
            write_frame(&mut writer, &Frame::response(response)).unwrap();
        };
        let mut subscribed = false;
        while let Ok(Some(Frame::Control(bytes))) = read_frame(&mut reader) {
            match Request::parse(&bytes).unwrap() {
                Request::List {} => {
                    send(&Response::Sessions {
                        sessions: vec![session(None)],
                        groups: Vec::new(),
                    });
                    if subscribed {
                        send(&Response::SessionChanged {
                            session: session(Some(External {
                                pid: Some(42),
                                status: Some(Status::Done),
                                background: false,
                            })),
                        });
                    }
                }
                Request::Usage {} => send(&Response::Usage { agents: Vec::new() }),
                Request::Subscribe { .. } => {
                    subscribed = true;
                    send(&Response::Done);
                }
                other => panic!("unexpected request {other:?}"),
            }
        }
    }

    #[test]
    fn a_change_while_the_feed_opens_is_not_lost() {
        let (near, far) = UnixStream::pair().unwrap();
        let host = std::thread::spawn(move || host_linking_after_the_list(far));
        let client = Client {
            reader: BufReader::new(near.try_clone().unwrap()),
            writer: near.try_clone().unwrap(),
            host_version: String::new(),
        };
        let (mut feed, catalog) = SessionFeed::start(client, true).unwrap();
        assert!(catalog.sessions[0].external.is_none());
        near.shutdown(std::net::Shutdown::Write).unwrap();
        host.join().unwrap();
        match feed.next_change().unwrap() {
            Some(Change::Session(session)) => assert!(session.external.is_some()),
            other => panic!("expected the link, got {other:?}"),
        }
    }
}
