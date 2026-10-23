//! Messages between Headquarter clients (`hq`, later the GUI) and the host process.
//!
//! Frames on the Unix socket are a 1-byte kind, a 4-byte big-endian payload length,
//! and the payload (at most [`MAX_PAYLOAD`]). Control frames carry one JSON
//! [`Request`] or [`Response`]; output and input frames carry raw terminal bytes.

pub mod sidebar;
pub mod usage;

use std::collections::HashMap;
use std::io::{Read, Write};

use hq_signals::event::{Agent, NeedsReason};
pub use hq_signals::quota::Account;
use hq_signals::status::Status;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 9;
pub const MAX_PAYLOAD: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Control(Vec<u8>),
    Output(Vec<u8>),
    Input(Vec<u8>),
}

impl Frame {
    fn kind(&self) -> u8 {
        match self {
            Frame::Control(_) => 0,
            Frame::Output(_) => 1,
            Frame::Input(_) => 2,
        }
    }

    fn payload(&self) -> &[u8] {
        match self {
            Frame::Control(bytes) | Frame::Output(bytes) | Frame::Input(bytes) => bytes,
        }
    }

    pub fn request(request: &Request) -> Frame {
        Frame::Control(serde_json::to_vec(request).unwrap_or_default())
    }

    pub fn response(response: &Response) -> Frame {
        Frame::Control(serde_json::to_vec(response).unwrap_or_default())
    }
}

#[derive(Debug)]
pub enum FrameError {
    Io(std::io::Error),
    UnknownKind(u8),
    TooLarge(usize),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Io(error) => write!(f, "connection error: {error}"),
            FrameError::UnknownKind(kind) => write!(f, "unknown frame kind {kind}"),
            FrameError::TooLarge(length) => {
                write!(f, "frame of {length} bytes exceeds {MAX_PAYLOAD}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

pub fn write_frame(out: &mut impl Write, frame: &Frame) -> Result<(), FrameError> {
    let payload = frame.payload();
    if payload.len() > MAX_PAYLOAD {
        return Err(FrameError::TooLarge(payload.len()));
    }
    let length = u32::try_from(payload.len()).map_err(|_| FrameError::TooLarge(payload.len()))?;
    let mut header = [0u8; 5];
    header[0] = frame.kind();
    header[1..].copy_from_slice(&length.to_be_bytes());
    out.write_all(&header).map_err(FrameError::Io)?;
    out.write_all(payload).map_err(FrameError::Io)?;
    out.flush().map_err(FrameError::Io)
}

/// Reads one frame; `Ok(None)` when the peer closed the connection between frames.
pub fn read_frame(input: &mut impl Read) -> Result<Option<Frame>, FrameError> {
    let mut header = [0u8; 5];
    match input.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(FrameError::Io(error)),
    }
    let length = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
    if length > MAX_PAYLOAD {
        return Err(FrameError::TooLarge(length));
    }
    let mut payload = vec![0u8; length];
    input.read_exact(&mut payload).map_err(FrameError::Io)?;
    match header[0] {
        0 => Ok(Some(Frame::Control(payload))),
        1 => Ok(Some(Frame::Output(payload))),
        2 => Ok(Some(Frame::Input(payload))),
        kind => Err(FrameError::UnknownKind(kind)),
    }
}

/// Field-less requests are empty struct variants because serde enforces
/// `deny_unknown_fields` only on struct variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Request {
    Hello {
        protocol: u32,
    },
    List {},
    New {
        agent: Agent,
        cwd: String,
        #[serde(default)]
        args: Vec<String>,
        /// The client's `TERM`, passed to the agent.
        #[serde(default)]
        term: Option<String>,
        #[serde(default)]
        name: Option<String>,
        /// The owner's group to put the session in; `None` leaves it in no group.
        #[serde(default)]
        group: Option<i64>,
    },
    Attach {
        session: String,
        cols: u16,
        rows: u16,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Detach {},
    Rename {
        session: String,
        name: String,
    },
    /// Hang the agent up at once; the session stays in the sidebar as not running.
    Kill {
        session: String,
    },
    /// Close the session: the agent is stopped with its own keys, falling back to a
    /// hang-up, and the session leaves the sidebar. Answered once the close has started.
    Close {
        session: String,
    },
    Resume {
        session: String,
        #[serde(default)]
        term: Option<String>,
    },
    /// Clear the interrupted mark of these sessions without restarting them; an empty
    /// list clears every mark ("Not now").
    Dismiss {
        sessions: Vec<String>,
    },
    /// Put sessions in the sidebar: names or id prefixes, all or nothing.
    Add {
        sessions: Vec<String>,
    },
    /// Take sessions out of the sidebar; they stay in the index.
    Remove {
        sessions: Vec<String>,
    },
    /// Add a group after the others, named `name` or "New Group #n"; with `session`,
    /// that session moves into it.
    NewGroup {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        session: Option<String>,
    },
    RenameGroup {
        group: i64,
        name: String,
    },
    /// Delete a group; its sessions stay in the sidebar, in no group.
    DeleteGroup {
        group: i64,
    },
    /// Move a group to just before `before`, or to the end.
    MoveGroup {
        group: i64,
        #[serde(default)]
        before: Option<i64>,
    },
    CollapseGroup {
        group: i64,
        collapsed: bool,
    },
    /// Put a sidebar session in a group, or in none.
    SetGroup {
        session: String,
        #[serde(default)]
        group: Option<i64>,
    },
    /// The owner has seen the session's latest state: clears "done, not read yet".
    Seen {
        session: String,
    },
    /// Each agent's usage windows, as last read.
    Usage {},
    /// Read usage again now; subscribers to usage get `UsageChanged` with the result.
    RefreshUsage {},
    /// The client shows this session (or none): it receives `DetailsChanged` with the
    /// session's details at once and whenever they change. Needs `Subscribe` first.
    FollowDetails {
        #[serde(default)]
        session: Option<String>,
    },
    /// Receive `SessionChanged` whenever any session changes, and `GroupsChanged`
    /// whenever the groups change; with `usage`, also `UsageChanged`.
    Subscribe {
        #[serde(default)]
        usage: bool,
    },
    /// Whether the client's window can be seen; a subscriber counts as visible until
    /// it says otherwise. The host scans for outside sessions less often while no
    /// subscriber is visible.
    Visible {
        visible: bool,
    },
    /// End every session and the host.
    Stop {},
}

impl Request {
    pub fn parse(bytes: &[u8]) -> Result<Request, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    ProtocolMismatch,
    BadRequest,
    NotFound,
    Ambiguous,
    AgentNotFound,
    FolderNotFound,
    SpawnFailed,
    NotLive,
    AlreadyLive,
    Storage,
    /// The host is shutting down and starts nothing new.
    Stopping,
    /// The conversation is open in another terminal.
    ExternalLive,
    /// The settings file is invalid; the message names the file and line.
    Settings,
    /// The agent is turned off in the settings file.
    AgentDisabled,
    /// The session cannot join the sidebar: it runs elsewhere with no known conversation.
    NotAddable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Response {
    Welcome {
        protocol: u32,
        version: String,
    },
    Sessions {
        sessions: Vec<SessionInfo>,
        /// The owner's groups, in the owner's order.
        #[serde(default)]
        groups: Vec<GroupInfo>,
    },
    Created {
        session: SessionInfo,
    },
    Attached {
        session: SessionInfo,
    },
    Detached,
    Done,
    SessionChanged {
        session: SessionInfo,
    },
    GroupCreated {
        group: GroupInfo,
    },
    /// Every group, in the owner's order.
    GroupsChanged {
        groups: Vec<GroupInfo>,
    },
    /// The followed session's details, as its agent reports them.
    DetailsChanged {
        session: String,
        details: hq_signals::details::SessionDetails,
    },
    Usage {
        agents: Vec<AgentUsage>,
    },
    UsageChanged {
        agents: Vec<AgentUsage>,
    },
    /// The attached session's agent exited.
    Ended {
        session: String,
        exit_code: Option<u32>,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

impl Response {
    pub fn parse(bytes: &[u8]) -> Result<Response, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }

    pub fn error(code: ErrorCode, message: impl Into<String>) -> Response {
        Response::Error {
            code,
            message: message.into(),
        }
    }
}

/// The host's answer to a client's `Hello`.
pub fn answer_hello(client_protocol: u32) -> Response {
    if client_protocol == PROTOCOL_VERSION {
        Response::Welcome {
            protocol: PROTOCOL_VERSION,
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    } else {
        Response::error(
            ErrorCode::ProtocolMismatch,
            format!(
                "client speaks protocol {client_protocol}, host speaks protocol {PROTOCOL_VERSION}; restart the host with `hq stop`"
            ),
        )
    }
}

/// The host's protocol named in its refusal of a client's `Hello`. `Hello` and `Stop`
/// keep their shape in every protocol version, and so does this refusal, so a client
/// of any version can stop a host of any other (`hq stop` after an upgrade).
pub fn host_protocol_in_refusal(message: &str) -> Option<u32> {
    let (_, rest) = message.split_once("host speaks protocol ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// One usage window as the agent reports it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageWindow {
    /// The window's length; `None` when the agent does not say.
    pub minutes: Option<u64>,
    pub used_percent: f64,
    /// Unix seconds.
    pub resets_at: Option<u64>,
    /// The model a per-model window counts ("Fable"); `None` for all models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// An agent's last usage reading. `problem` explains a missing reading, or why the
/// shown one could not be refreshed; both can be present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentUsage {
    pub agent: Agent,
    pub windows: Vec<UsageWindow>,
    /// Unix seconds; `None` when never read.
    pub read_at: Option<u64>,
    pub problem: Option<String>,
    /// What the limits belong to, when the agent reports it.
    #[serde(default)]
    pub account: Option<Account>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub agent: Agent,
    pub cwd: String,
    pub name: String,
    pub agent_session_id: Option<String>,
    pub status: Status,
    pub needs_reason: Option<NeedsReason>,
    /// Unix milliseconds.
    pub created_at: u64,
    pub last_activity_at: u64,
    pub ended_at: Option<u64>,
    /// Whether the agent is running in the host now.
    pub live: bool,
    pub origin: Origin,
    /// The agent's own title, read from its files for display; never stored.
    pub title: Option<String>,
    /// Set while the conversation's agent runs in another terminal.
    pub external: Option<External>,
    /// Whether the session is in the owner's sidebar.
    #[serde(default)]
    pub in_sidebar: bool,
    /// The owner's group holding the session; `None` when it is in no group.
    #[serde(default)]
    pub group: Option<i64>,
    /// Whether the owner gave `name`; it is then shown instead of the agent's title.
    #[serde(default)]
    pub owner_named: bool,
    /// Whether the session was running when the host stopped without ending it (a
    /// crash, a reboot), until it is restarted or dismissed.
    #[serde(default)]
    pub interrupted: bool,
    /// Whether Add sessions offers it: false for a past conversation its agent no longer
    /// keeps, a run a program started, or one never used (offer-continuable-sessions).
    /// Hosts that predate it send nothing, which counts as offered.
    #[serde(default = "offered_by_default")]
    pub offered: bool,
}

fn offered_by_default() -> bool {
    true
}

/// A group the owner made in the sidebar. Lists of groups are in the owner's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: i64,
    pub name: String,
    pub collapsed: bool,
}

/// Group names are at most this many characters.
pub const GROUP_NAME_MAX: usize = 60;

/// A group name as the owner typed it, trimmed; an error says what is wrong with it.
pub fn group_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("a group name cannot be empty".to_owned());
    }
    if name.chars().count() > GROUP_NAME_MAX {
        return Err(format!(
            "group names are at most {GROUP_NAME_MAX} characters"
        ));
    }
    if name.chars().any(char::is_control) {
        return Err("a group name cannot contain control characters".to_owned());
    }
    Ok(name.to_owned())
}

/// "New Group #n" with the smallest n no existing group uses.
pub fn default_group_name<'a>(existing: impl IntoIterator<Item = &'a str>) -> String {
    let used: std::collections::HashSet<&str> = existing.into_iter().collect();
    (1..)
        .map(|n| format!("New Group #{n}"))
        .find(|name| !used.contains(name.as_str()))
        .expect("an unbounded counter always finds a free name")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// Started by Headquarter.
    Hosted,
    /// Imported from an agent's own history.
    History,
    /// An agent running in another terminal that no known conversation matches;
    /// listed while it runs and never stored.
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct External {
    pub pid: Option<u32>,
    /// `None` when no source reports one.
    pub status: Option<Status>,
}

/// Display labels for group folders: the final folder name, extended with parent
/// names only as far as needed to tell same-named folders apart.
pub fn group_labels<'a>(folders: &[&'a str]) -> HashMap<&'a str, String> {
    let parts: HashMap<&str, Vec<&str>> = folders
        .iter()
        .map(|folder| {
            let components: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
            (*folder, components)
        })
        .collect();
    let label = |folder: &str, depth: usize| -> String {
        let components = &parts[folder];
        if components.is_empty() {
            return "/".to_owned();
        }
        let start = components.len().saturating_sub(depth);
        components[start..].join("/")
    };

    let mut depth: HashMap<&str, usize> = folders.iter().map(|folder| (*folder, 1)).collect();
    loop {
        let mut by_label: HashMap<String, Vec<&str>> = HashMap::new();
        for folder in folders {
            by_label
                .entry(label(folder, depth[folder]))
                .or_default()
                .push(folder);
        }
        let mut grew = false;
        for clashing in by_label.values().filter(|group| group.len() > 1) {
            for folder in clashing {
                let deepest = parts[folder].len().max(1);
                let current = depth.get_mut(folder).expect("every folder has a depth");
                if *current < deepest {
                    *current += 1;
                    grew = true;
                }
            }
        }
        if !grew {
            return folders
                .iter()
                .map(|folder| (*folder, label(folder, depth[folder])))
                .collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusals_name_the_host_protocol() {
        let Response::Error { code, message } = answer_hello(PROTOCOL_VERSION + 1) else {
            panic!("a different version is refused");
        };
        assert_eq!(code, ErrorCode::ProtocolMismatch);
        assert_eq!(host_protocol_in_refusal(&message), Some(PROTOCOL_VERSION));
        assert_eq!(
            host_protocol_in_refusal(
                "client speaks protocol 6, host speaks protocol 5; restart the host with `hq stop`"
            ),
            Some(5),
            "the wording every host version sends"
        );
        assert_eq!(host_protocol_in_refusal("expected hello first"), None);
    }

    #[test]
    fn group_names_are_trimmed_and_checked() {
        assert_eq!(group_name("  Backend ").unwrap(), "Backend");
        assert!(group_name("   ").is_err());
        assert!(group_name(&"x".repeat(60)).is_ok());
        assert_eq!(
            group_name(&"x".repeat(61)).unwrap_err(),
            "group names are at most 60 characters"
        );
        assert!(group_name("a\u{7}b").is_err());
    }

    #[test]
    fn default_group_names_reuse_gaps() {
        assert_eq!(default_group_name([]), "New Group #1");
        assert_eq!(
            default_group_name(["New Group #1", "New Group #3", "Docs"]),
            "New Group #2"
        );
    }

    #[test]
    fn frames_round_trip() {
        let frames = [
            Frame::request(&Request::List {}),
            Frame::Output(b"\x1b[31mred\x1b[0m".to_vec()),
            Frame::Input(vec![]),
        ];
        let mut buffer = Vec::new();
        for frame in &frames {
            write_frame(&mut buffer, frame).unwrap();
        }
        let mut reader = buffer.as_slice();
        for frame in &frames {
            assert_eq!(read_frame(&mut reader).unwrap().as_ref(), Some(frame));
        }
        assert!(read_frame(&mut reader).unwrap().is_none());
    }

    #[test]
    fn oversized_frames_are_rejected_both_ways() {
        let big = Frame::Output(vec![0; MAX_PAYLOAD + 1]);
        assert!(matches!(
            write_frame(&mut Vec::new(), &big),
            Err(FrameError::TooLarge(_))
        ));
        let mut header = vec![1u8];
        header.extend_from_slice(&u32::try_from(MAX_PAYLOAD + 1).unwrap().to_be_bytes());
        assert!(matches!(
            read_frame(&mut header.as_slice()),
            Err(FrameError::TooLarge(_))
        ));
    }

    #[test]
    fn malformed_frames_are_errors() {
        let mut unknown: &[u8] = &[9, 0, 0, 0, 0];
        assert!(matches!(
            read_frame(&mut unknown),
            Err(FrameError::UnknownKind(9))
        ));
        let mut truncated: &[u8] = &[0, 0, 0, 0, 5, b'{'];
        assert!(matches!(read_frame(&mut truncated), Err(FrameError::Io(_))));
    }

    #[test]
    fn requests_round_trip_and_reject_unknown_input() {
        let request = Request::New {
            agent: Agent::Codex,
            cwd: "/w".into(),
            args: vec![],
            term: None,
            name: None,
            group: Some(3),
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        assert_eq!(Request::parse(&bytes).unwrap(), request);
        assert_eq!(
            Request::parse(br#"{"type":"new","agent":"claude","cwd":"/w"}"#).unwrap(),
            Request::New {
                agent: Agent::Claude,
                cwd: "/w".into(),
                args: vec![],
                term: None,
                name: None,
                group: None
            }
        );
        assert!(Request::parse(br#"{"type":"new","agent":"gpt","cwd":"/w"}"#).is_err());
        assert!(Request::parse(br#"{"type":"list","extra":1}"#).is_err());
        assert!(Request::parse(br#"{"type":"kill","session":"a","signal":9}"#).is_err());
        assert!(Request::parse(br#"{"type":"format-disk"}"#).is_err());
        assert!(Request::parse(b"not json").is_err());
    }

    #[test]
    fn sidebar_requests_round_trip() {
        for request in [
            Request::Add {
                sessions: vec!["claude-1".into(), "3f2a".into()],
            },
            Request::Remove {
                sessions: vec!["claude-1".into()],
            },
            Request::NewGroup {
                name: None,
                session: Some("claude-1".into()),
            },
            Request::RenameGroup {
                group: 1,
                name: "Backend".into(),
            },
            Request::DeleteGroup { group: 1 },
            Request::MoveGroup {
                group: 2,
                before: Some(1),
            },
            Request::CollapseGroup {
                group: 1,
                collapsed: true,
            },
            Request::SetGroup {
                session: "claude-1".into(),
                group: None,
            },
            Request::Seen {
                session: "claude-1".into(),
            },
            Request::Close {
                session: "claude-1".into(),
            },
            Request::Dismiss { sessions: vec![] },
            Request::Dismiss {
                sessions: vec!["claude-1".into()],
            },
        ] {
            let bytes = serde_json::to_vec(&request).unwrap();
            assert_eq!(Request::parse(&bytes).unwrap(), request);
        }
        assert!(Request::parse(br#"{"type":"add"}"#).is_err());
        assert_eq!(
            Request::parse(br#"{"type":"new-group"}"#).unwrap(),
            Request::NewGroup {
                name: None,
                session: None
            }
        );
        let groups = Response::GroupsChanged {
            groups: vec![GroupInfo {
                id: 1,
                name: "Backend".into(),
                collapsed: false,
            }],
        };
        let bytes = serde_json::to_vec(&groups).unwrap();
        assert_eq!(Response::parse(&bytes).unwrap(), groups);
        assert_eq!(
            Response::parse(br#"{"type":"sessions","sessions":[]}"#).unwrap(),
            Response::Sessions {
                sessions: vec![],
                groups: vec![]
            }
        );
    }

    #[test]
    fn a_session_from_before_the_interrupted_mark_reads_as_not_interrupted() {
        let older = br#"{"type":"sessions","sessions":[{"id":"a","agent":"claude","cwd":"/w","name":"claude-1","agent_session_id":null,"status":"inactive","needs_reason":null,"created_at":1,"last_activity_at":2,"ended_at":null,"live":false,"origin":"hosted","title":null,"external":null,"in_sidebar":true}]}"#;
        let Response::Sessions { sessions, .. } = Response::parse(older).unwrap() else {
            panic!("not a session list");
        };
        assert!(!sessions[0].interrupted);

        let mut marked = sessions[0].clone();
        marked.interrupted = true;
        let bytes = serde_json::to_vec(&marked).unwrap();
        assert_eq!(
            serde_json::from_slice::<SessionInfo>(&bytes).unwrap(),
            marked
        );
    }

    #[test]
    fn usage_messages_round_trip() {
        for request in [
            Request::Usage {},
            Request::RefreshUsage {},
            Request::Subscribe { usage: true },
            Request::Visible { visible: false },
        ] {
            let bytes = serde_json::to_vec(&request).unwrap();
            assert_eq!(Request::parse(&bytes).unwrap(), request);
        }
        assert_eq!(
            Request::parse(br#"{"type":"subscribe"}"#).unwrap(),
            Request::Subscribe { usage: false },
            "older clients subscribe without usage"
        );
        let answer = Response::Usage {
            agents: vec![AgentUsage {
                agent: Agent::Codex,
                windows: vec![UsageWindow {
                    minutes: Some(300),
                    used_percent: 82.5,
                    resets_at: Some(2_000),
                    model: None,
                }],
                read_at: Some(1_000),
                problem: None,
                account: Some(Account::Subscription {
                    plan: Some("plus".into()),
                }),
            }],
        };
        let bytes = serde_json::to_vec(&answer).unwrap();
        assert_eq!(Response::parse(&bytes).unwrap(), answer);
    }

    #[test]
    fn hello_version_check() {
        assert!(matches!(
            answer_hello(PROTOCOL_VERSION),
            Response::Welcome { .. }
        ));
        match answer_hello(PROTOCOL_VERSION + 1) {
            Response::Error { code, message } => {
                assert_eq!(code, ErrorCode::ProtocolMismatch);
                assert!(message.contains(&(PROTOCOL_VERSION + 1).to_string()));
                assert!(message.contains(&PROTOCOL_VERSION.to_string()));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn group_labels_disambiguate_same_names() {
        let folders = ["/u/work/app", "/u/side/app", "/u/notes", "/"];
        let labels = group_labels(&folders);
        assert_eq!(labels["/u/work/app"], "work/app");
        assert_eq!(labels["/u/side/app"], "side/app");
        assert_eq!(labels["/u/notes"], "notes");
        assert_eq!(labels["/"], "/");
    }

    #[test]
    fn group_labels_with_deeper_clashes() {
        let folders = ["/a/x/app", "/b/x/app"];
        let labels = group_labels(&folders);
        assert_eq!(labels["/a/x/app"], "a/x/app");
        assert_eq!(labels["/b/x/app"], "b/x/app");
    }
}
