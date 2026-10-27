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
pub use hq_signals::resources::Footprint;
use hq_signals::status::Status;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 13;
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
    /// Attach to the session's shell, first starting the owner's login shell in the
    /// session's folder when none runs (add-session-shell-panel). Input, resizes and
    /// output then flow as for an attached session.
    AttachShell {
        session: String,
        cols: u16,
        rows: u16,
    },
    /// End the session's shell, if one runs.
    CloseShell {
        session: String,
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
    /// Stop a Claude conversation running in Claude Code's background, then resume it in
    /// Headquarter (fix-claude-conversations D3).
    BringIn {
        session: String,
        #[serde(default)]
        term: Option<String>,
    },
    /// Put sessions in the sidebar: names or id prefixes, all or nothing.
    Add {
        sessions: Vec<String>,
    },
    /// Take sessions out of the sidebar; they stay in the index.
    Remove {
        sessions: Vec<String>,
    },
    /// Remove a past conversation from its agent, the way that agent allows
    /// ([`removal_ways`]); never one that is running.
    RemoveConversation {
        session: String,
        way: RemoveWay,
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
    /// Pick a group's colour by palette name, or `None` for automatic.
    SetGroupColor {
        group: i64,
        #[serde(default)]
        color: Option<String>,
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
    /// The host's own CPU and memory, and each running session's with every process it
    /// started and its shell panel (add-usage-window D1).
    Resources {},
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
    /// The owner brought the attached session to the front on this client: its terminal
    /// takes this client's size (add-remote-gateway D6).
    Focus {},
    /// A phone that is not paired yet presents the pairing code's secret; handled by the
    /// remote gateway, never forwarded to the host.
    Pair {
        secret: String,
        /// The phone's name, such as its model.
        name: String,
    },
    /// Remote control's state; local clients only.
    RemoteStatus {},
    /// Turns remote control on or off; local clients only.
    RemoteSet {
        enabled: bool,
    },
    /// A one-time pairing code; local clients only. The host answers with the code, then
    /// on the same connection when it is used or expires.
    RemotePair {},
    /// The paired phones; local clients only.
    RemoteDevices {},
    /// Removes a paired phone by name or id prefix; local clients only.
    RemoteRemove {
        device: String,
    },
    /// A paired phone removes its own pairing. The remote gateway handles it, and the
    /// local socket refuses it (add-android-app D5).
    Unpair {},
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
    /// The agent could not remove the conversation; the message says which agent.
    RemoveFailed,
    /// A phone asked for something remote control does not allow (add-remote-gateway D5).
    NotAllowed,
    /// Remote control is off, so there is nothing to pair with.
    RemoteOff,
    /// The conversation is running in Claude Code's own background daemon, so resuming it
    /// would fail; the message names its process (fix-claude-conversations).
    RunningInBackground,
}

/// How a past conversation leaves its agent (remove-conversations, owner 2026-10-23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoveWay {
    /// Its files move to the Trash: for an agent with no delete command of its own.
    Trash,
    /// The agent's own archive, which it can undo.
    Archive,
    /// The agent's own permanent delete.
    Delete,
}

/// The ways each agent's conversations can be removed: Claude Code has no delete
/// command, so its transcript goes to the Trash; Codex has `archive` and `delete`; agy
/// deletes only inside its own picker, so Headquarter offers nothing.
pub fn removal_ways(agent: Agent) -> &'static [RemoveWay] {
    match agent {
        Agent::Claude => &[RemoveWay::Trash],
        Agent::Codex => &[RemoveWay::Archive, RemoveWay::Delete],
        Agent::Agy => &[],
    }
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
    /// The phone is paired; the connection goes on as a paired phone's.
    Paired {
        host: String,
    },
    /// The gateway's first frame to a paired phone on every connection: the computer's
    /// name, its home relay, and the host's protocol and version, so the phone follows a
    /// relay change and names the side to update (add-android-app D4).
    RemoteWelcome {
        host: String,
        relay: Option<String>,
        protocol: u32,
        version: String,
    },
    /// The phone's pairing is removed, and the gateway closes the connection.
    Unpaired,
    RemoteStatus {
        enabled: bool,
        /// The gateway runs (it may have failed after being enabled).
        running: bool,
        /// The host's endpoint id while running.
        endpoint: Option<String>,
        /// The relay the host is reachable through, when connected to one.
        relay: Option<String>,
        /// Why the gateway is not running although enabled.
        problem: Option<String>,
    },
    /// A pairing code to show, valid once for `expires_in_secs`.
    RemotePairing {
        code: String,
        expires_in_secs: u64,
    },
    /// The shown code paired the named phone.
    RemotePairUsed {
        device: String,
    },
    /// The shown code expired, or was voided after wrong guesses.
    RemotePairExpired {
        reason: String,
    },
    RemoteDevices {
        devices: Vec<RemoteDevice>,
    },
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
    Resources {
        host: Footprint,
        sessions: Vec<SessionFootprint>,
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

/// A phone paired for remote control (add-remote-gateway).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteDevice {
    /// The phone's endpoint id (its public key).
    pub id: String,
    pub name: String,
    /// Unix milliseconds.
    pub paired_at: i64,
    pub last_seen_at: Option<i64>,
    pub connected: bool,
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

/// What a running session costs: its agent, every process the agent started, and its
/// shell panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionFootprint {
    pub session: String,
    pub footprint: Footprint,
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
    /// Whether the host is starting it with itself (add-inactive-sessions): waiting its
    /// turn, or started and not yet ready.
    #[serde(default)]
    pub starting: bool,
    /// Why the host could not start it with itself, until it starts or leaves the
    /// sidebar.
    #[serde(default)]
    pub start_error: Option<StartError>,
    /// Whether Add sessions offers it: false for a past conversation its agent no longer
    /// keeps, a run a program started, or one never used (offer-continuable-sessions).
    /// Hosts that predate it send nothing, which counts as offered.
    #[serde(default = "offered_by_default")]
    pub offered: bool,
}

fn offered_by_default() -> bool {
    true
}

/// A session the host could not start with itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartError {
    pub reason: String,
    /// Unix milliseconds.
    pub at: u64,
}

/// A group the owner made in the sidebar. Lists of groups are in the owner's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: i64,
    pub name: String,
    pub collapsed: bool,
    /// The picked colour's palette name; `None`, or a name this version does not know,
    /// is automatic. Hosts that predate colours send nothing (add-group-colors D6).
    #[serde(default)]
    pub color: Option<String>,
}

impl GroupInfo {
    /// The colour the group shows: the picked one, else its automatic colour.
    pub fn shown_color(&self) -> GroupColor {
        self.color
            .as_deref()
            .and_then(GroupColor::from_name)
            .unwrap_or_else(|| GroupColor::automatic(self.id))
    }
}

/// The group colours, in the order automatic colours take them (add-group-colors D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupColor {
    Mauve,
    Green,
    Teal,
    Peach,
    Blue,
    Pink,
    Yellow,
    Red,
}

impl GroupColor {
    pub const ALL: [GroupColor; 8] = [
        GroupColor::Mauve,
        GroupColor::Green,
        GroupColor::Teal,
        GroupColor::Peach,
        GroupColor::Blue,
        GroupColor::Pink,
        GroupColor::Yellow,
        GroupColor::Red,
    ];

    /// The name stored and sent on the wire.
    pub fn name(self) -> &'static str {
        match self {
            GroupColor::Mauve => "mauve",
            GroupColor::Green => "green",
            GroupColor::Teal => "teal",
            GroupColor::Peach => "peach",
            GroupColor::Blue => "blue",
            GroupColor::Pink => "pink",
            GroupColor::Yellow => "yellow",
            GroupColor::Red => "red",
        }
    }

    /// The name the owner sees.
    pub fn label(self) -> &'static str {
        match self {
            GroupColor::Mauve => "Mauve",
            GroupColor::Green => "Green",
            GroupColor::Teal => "Teal",
            GroupColor::Peach => "Peach",
            GroupColor::Blue => "Blue",
            GroupColor::Pink => "Pink",
            GroupColor::Yellow => "Yellow",
            GroupColor::Red => "Red",
        }
    }

    pub fn from_name(name: &str) -> Option<GroupColor> {
        GroupColor::ALL
            .into_iter()
            .find(|color| color.name() == name)
    }

    /// A group's colour while none is picked: by its id, so consecutive groups differ
    /// and reordering or deleting others never changes it (D2).
    pub fn automatic(group: i64) -> GroupColor {
        let count = GroupColor::ALL.len() as i64;
        let index = (group - 1).rem_euclid(count);
        GroupColor::ALL[usize::try_from(index).unwrap_or(0)]
    }
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
    /// Running in Claude Code's own background daemon rather than a terminal, so it can be
    /// brought into Headquarter (fix-claude-conversations D3).
    #[serde(default)]
    pub background: bool,
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
    fn automatic_group_colours_follow_the_id() {
        assert_eq!(GroupColor::automatic(1), GroupColor::Mauve);
        assert_eq!(GroupColor::automatic(2), GroupColor::Green);
        assert_eq!(
            GroupColor::automatic(9),
            GroupColor::Mauve,
            "the ninth repeats"
        );
        assert_eq!(GroupColor::from_name("green"), Some(GroupColor::Green));
        assert_eq!(
            GroupColor::from_name("Green"),
            None,
            "stored names are lower case"
        );
        let picked = GroupInfo {
            id: 1,
            name: "Backend".into(),
            collapsed: false,
            color: Some("red".into()),
        };
        assert_eq!(picked.shown_color(), GroupColor::Red);
        let unknown = GroupInfo {
            color: Some("ultraviolet".into()),
            ..picked
        };
        assert_eq!(
            unknown.shown_color(),
            GroupColor::Mauve,
            "unknown names are automatic"
        );
    }

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
            Request::RemoveConversation {
                session: "codex-1".into(),
                way: RemoveWay::Archive,
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
                color: Some("green".into()),
            }],
        };
        let bytes = serde_json::to_vec(&groups).unwrap();
        assert_eq!(Response::parse(&bytes).unwrap(), groups);
        let older = Response::parse(
            br#"{"type":"groups-changed","groups":[{"id":3,"name":"Web","collapsed":false}]}"#,
        )
        .unwrap();
        let Response::GroupsChanged { groups } = older else {
            panic!("unexpected {older:?}");
        };
        assert_eq!(groups[0].color, None, "an older host sends no colour");
        assert_eq!(groups[0].shown_color(), GroupColor::Teal, "automatic by id");
        assert_eq!(
            Response::parse(br#"{"type":"sessions","sessions":[]}"#).unwrap(),
            Response::Sessions {
                sessions: vec![],
                groups: vec![]
            }
        );
    }

    #[test]
    fn a_session_from_an_older_host_is_not_starting() {
        let older = br#"{"type":"sessions","sessions":[{"id":"a","agent":"claude","cwd":"/w","name":"claude-1","agent_session_id":null,"status":"inactive","needs_reason":null,"created_at":1,"last_activity_at":2,"ended_at":null,"live":false,"origin":"hosted","title":null,"external":null,"in_sidebar":true,"interrupted":true}]}"#;
        let Response::Sessions { sessions, .. } = Response::parse(older).unwrap() else {
            panic!("not a session list");
        };
        assert!(!sessions[0].starting);
        assert_eq!(sessions[0].start_error, None);

        let mut failed = sessions[0].clone();
        failed.start_error = Some(StartError {
            reason: "the folder no longer exists".into(),
            at: 5,
        });
        let bytes = serde_json::to_vec(&failed).unwrap();
        assert_eq!(
            serde_json::from_slice::<SessionInfo>(&bytes).unwrap(),
            failed
        );
    }

    #[test]
    fn remote_messages_round_trip() {
        for request in [
            Request::Focus {},
            Request::Pair {
                secret: "s3cret".into(),
                name: "Pixel 8".into(),
            },
            Request::RemoteStatus {},
            Request::RemoteSet { enabled: true },
            Request::RemotePair {},
            Request::RemoteDevices {},
            Request::RemoteRemove {
                device: "pixel".into(),
            },
            Request::Unpair {},
        ] {
            let bytes = serde_json::to_vec(&request).unwrap();
            assert_eq!(Request::parse(&bytes).unwrap(), request);
        }
        for response in [
            Response::Paired { host: "mac".into() },
            Response::RemoteWelcome {
                host: "mac".into(),
                relay: Some("https://relay.example.com./".into()),
                protocol: PROTOCOL_VERSION,
                version: "0.1.14".into(),
            },
            Response::Unpaired,
            Response::RemoteStatus {
                enabled: true,
                running: false,
                endpoint: None,
                relay: None,
                problem: Some("bind failed".into()),
            },
            Response::RemotePairing {
                code: "hqp2|x".into(),
                expires_in_secs: 120,
            },
            Response::RemotePairUsed {
                device: "Pixel 8".into(),
            },
            Response::RemotePairExpired {
                reason: "expired".into(),
            },
            Response::RemoteDevices {
                devices: vec![RemoteDevice {
                    id: "abc".into(),
                    name: "Pixel 8".into(),
                    paired_at: 1,
                    last_seen_at: Some(2),
                    connected: true,
                }],
            },
        ] {
            let bytes = serde_json::to_vec(&response).unwrap();
            assert_eq!(Response::parse(&bytes).unwrap(), response);
        }
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
    fn resources_round_trip() {
        let bytes = serde_json::to_vec(&Request::Resources {}).unwrap();
        assert_eq!(Request::parse(&bytes).unwrap(), Request::Resources {});
        let answer = Response::Resources {
            host: Footprint {
                cpu: Some(0.5),
                memory: 40 << 20,
                processes: 1,
            },
            sessions: vec![SessionFootprint {
                session: "claude-1".into(),
                footprint: Footprint {
                    cpu: None,
                    memory: 300 << 20,
                    processes: 4,
                },
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
