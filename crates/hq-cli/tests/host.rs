//! End-to-end tests of the host and `hq` with stub agents. Each test gets its own
//! data folder, `HOME`, and a `PATH` holding only stub agents and system tools, so no
//! real agent runs and no owner file is read.

use std::io::{BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use hq_protocol::{
    ErrorCode, Frame, PROTOCOL_VERSION, Request, Response, SessionInfo, read_frame, write_frame,
};
use hq_signals::event::Agent;
use hq_signals::forward::ForwardedPayload;
use hq_signals::status::Status;

const HQ: &str = env!("CARGO_BIN_EXE_hq");
const WAIT: Duration = Duration::from_secs(10);

/// Waits for the OSC 7501 permission prompt to be answered, then reports work and
/// completion, then waits for a second line.
const INTERACTIVE_STUB: &str = r#"printf 'stub-ready\n'
printf '\033]7501;state=blocked:kind=permission\007'
read line
printf 'got:%s\n' "$line"
printf '\033]7501;state=working\007'
sleep 0.3
printf '\033]7501;state=done\007'
read second
printf 'second:%s\n' "$second"
sleep 30
"#;

/// Records its arguments, one per line, then idles. Answers the host's outside-session
/// query (`claude agents --json`) with no sessions.
const RECORDING_STUB: &str = r#"if [ "$1" = "agents" ]; then echo '[]'; exit 0; fi
printf '%s\n' "$@" >> "$STUB_ARGS"
printf 'recording-stub\n'
sleep 30
"#;

struct TestEnv {
    root: PathBuf,
    daemons: Vec<Child>,
}

impl TestEnv {
    fn new(name: &str) -> TestEnv {
        let root = PathBuf::from(format!("/tmp/hqh-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["data", "home", "stubs", "work"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        TestEnv {
            root,
            daemons: Vec::new(),
        }
    }

    fn work(&self) -> PathBuf {
        self.root.join("work").canonicalize().unwrap()
    }

    fn host_dir(&self) -> PathBuf {
        self.root.join("data").join("headquarter")
    }

    fn stub(&self, agent: &str, script: &str) {
        let path = self.root.join("stubs").join(agent);
        std::fs::write(&path, format!("#!/bin/sh\n{script}")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(HQ);
        command
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("HOME", self.root.join("home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.root.join("stubs").display()),
            )
            .env("STUB_ARGS", self.root.join("stub-args.txt"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            // Never run the owner's real login shell from tests.
            .env("SHELL", "/nonexistent/shell")
            .env("HQ_IDLE_EXIT_SECS", "60")
            .env_remove("HQ_SESSION");
        command
    }

    fn hq(&self, args: &[&str]) -> Output {
        self.command()
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn start_daemon(&mut self) {
        let child = self
            .command()
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        self.daemons.push(child);
        let socket = self.host_dir().join("agentd.sock");
        wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    }

    fn connect(&self) -> Conn {
        let mut conn = Conn::open(&self.host_dir().join("agentd.sock"));
        match conn.request(&Request::Hello {
            protocol: PROTOCOL_VERSION,
        }) {
            Response::Welcome { .. } => conn,
            other => panic!("unexpected greeting {other:?}"),
        }
    }

    /// Sessions in this test's index. The outside scan is machine-wide, so agents
    /// running elsewhere on the test machine show up as external entries; they are
    /// left out here and checked only through `all_sessions`.
    fn sessions(&self) -> Vec<SessionInfo> {
        self.all_sessions()
            .into_iter()
            .filter(|session| session.origin != hq_protocol::Origin::External)
            .collect()
    }

    fn all_sessions(&self) -> Vec<SessionInfo> {
        match self.connect().request(&Request::List {}) {
            Response::Sessions { sessions, .. } => sessions,
            other => panic!("unexpected list answer {other:?}"),
        }
    }

    fn session(&self, id: &str) -> SessionInfo {
        self.sessions()
            .into_iter()
            .find(|session| session.id == id)
            .expect("session listed")
    }

    fn wait_status(&self, id: &str, status: Status) {
        wait_until(&format!("status {status:?}"), || {
            self.session(id).status == status
        });
    }

    fn create(&self, agent: Agent) -> SessionInfo {
        match self.connect().request(&Request::New {
            agent,
            cwd: self.work().to_string_lossy().into_owned(),
            args: vec![],
            term: None,
            name: None,
            group: None,
        }) {
            Response::Created { session } => session,
            other => panic!("unexpected create answer {other:?}"),
        }
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.host_dir().join("agentd.log")).unwrap_or_default()
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if let Ok(stream) = UnixStream::connect(self.host_dir().join("agentd.sock")) {
            let mut conn = Conn::from_stream(stream);
            conn.send(&Request::Hello {
                protocol: PROTOCOL_VERSION,
            });
            conn.send(&Request::Stop {});
            let _ = conn.frames.recv_timeout(Duration::from_secs(2));
        }
        for daemon in &mut self.daemons {
            let _ = daemon.kill();
            let _ = daemon.wait();
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A client connection whose frames arrive on a channel, so reads can time out
/// without breaking framing.
struct Conn {
    writer: UnixStream,
    frames: Receiver<Frame>,
}

impl Conn {
    fn open(socket: &Path) -> Conn {
        Conn::from_stream(UnixStream::connect(socket).unwrap())
    }

    fn from_stream(stream: UnixStream) -> Conn {
        let writer = stream.try_clone().unwrap();
        let (tx, frames) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            while let Ok(Some(frame)) = read_frame(&mut reader) {
                if tx.send(frame).is_err() {
                    return;
                }
            }
        });
        Conn { writer, frames }
    }

    /// Ends the connection for the host now; dropping `Conn` alone leaves the socket
    /// open until its reader thread notices.
    fn close(self) {
        let _ = self.writer.shutdown(std::net::Shutdown::Both);
    }

    fn send(&mut self, request: &Request) {
        let _ = write_frame(&mut self.writer, &Frame::request(request));
    }

    fn send_frame(&mut self, frame: &Frame) {
        write_frame(&mut self.writer, frame).unwrap();
    }

    fn next_response(&mut self) -> Response {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.frames.recv_timeout(left).expect("a response in time") {
                Frame::Control(bytes) => return Response::parse(&bytes).unwrap(),
                _ => continue,
            }
        }
    }

    fn request(&mut self, request: &Request) -> Response {
        self.send(request);
        self.next_response()
    }

    /// Sends a request on a subscribed connection and returns its answer, past any
    /// changes the host sends first.
    fn answer(&mut self, request: &Request) -> Response {
        self.send(request);
        loop {
            match self.next_response() {
                Response::SessionChanged { .. }
                | Response::GroupsChanged { .. }
                | Response::DetailsChanged { .. }
                | Response::UsageChanged { .. } => continue,
                other => return other,
            }
        }
    }

    /// Collects output until it contains `needle`; returns everything read.
    fn output_until(&mut self, needle: &str) -> String {
        let deadline = Instant::now() + WAIT;
        let mut seen = String::new();
        while !seen.contains(needle) {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.frames.recv_timeout(left) {
                Ok(Frame::Output(bytes)) => seen.push_str(&String::from_utf8_lossy(&bytes)),
                Ok(_) => {}
                Err(_) => panic!("`{needle}` not seen; got {seen:?}"),
            }
        }
        seen
    }
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn single_instance_with_private_files() {
    let mut env = TestEnv::new("single");
    env.start_daemon();
    let second = env.hq(&["daemon"]);
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("already running"));
    assert!(env.sessions().is_empty(), "first host still serves");

    assert_eq!(mode(&env.host_dir()), 0o700);
    assert_eq!(mode(&env.host_dir().join("agentd.sock")), 0o600);
    assert_eq!(mode(&env.host_dir().join("hooks.sock")), 0o600);
}

#[test]
fn a_new_host_waits_for_a_stopping_one() {
    let env = TestEnv::new("stopping");
    std::fs::create_dir_all(env.host_dir()).unwrap();
    // A stopping host: its socket is gone, its lock still held while agents end.
    let held = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(env.host_dir().join("agentd.lock"))
        .unwrap();
    held.lock().unwrap();
    let mut waiting = env
        .command()
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(800));
    assert!(
        waiting.try_wait().unwrap().is_none(),
        "it waits instead of giving up"
    );
    held.unlock().unwrap();
    let socket = env.host_dir().join("agentd.sock");
    wait_until("the new host serves", || {
        UnixStream::connect(&socket).is_ok()
    });
    assert!(env.sessions().is_empty());
    let mut env = env;
    env.daemons.push(waiting);
}

#[test]
fn hosted_session_survives_detach_and_reports_status() {
    let mut env = TestEnv::new("attach");
    env.stub("agy", INTERACTIVE_STUB);
    env.start_daemon();
    let session = env.create(Agent::Agy);
    assert!(session.live);
    assert_eq!(session.name, "agy-1");
    env.wait_status(&session.id, Status::NeedsYou);

    let mut first = env.connect();
    assert!(matches!(
        first.request(&Request::Attach {
            session: session.id.clone(),
            cols: 100,
            rows: 30
        }),
        Response::Attached { .. }
    ));
    first.output_until("stub-ready");
    first.send_frame(&Frame::Input(b"yes\n".to_vec()));
    first.output_until("got:yes");
    env.wait_status(&session.id, Status::DoneUnread);
    assert!(matches!(
        first.request(&Request::Detach {}),
        Response::Detached
    ));
    drop(first);

    // The agent kept running; a new client catches up on earlier output.
    let mut second = env.connect();
    assert!(matches!(
        second.request(&Request::Attach {
            session: "agy-1".into(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    second.output_until("got:yes");
    second.send_frame(&Frame::Input(b"two\n".to_vec()));
    second.output_until("second:two");

    assert!(matches!(
        env.connect().request(&Request::Kill {
            session: session.id.clone()
        }),
        Response::Done
    ));
    let ended = loop {
        match second.next_response() {
            Response::Ended { session: id, .. } => break id,
            _ => continue,
        }
    };
    assert_eq!(ended, session.id);
    env.wait_status(&session.id, Status::Inactive);
    assert!(!env.session(&session.id).live);
}

/// A Claude Code stub whose `agents --json` reports the hosted stub busy or idle, as
/// the file `$STUB_ARGS.state` says.
const CLAUDE_BACKGROUND_STUB: &str = r#"if [ "$1" = "agents" ]; then
  printf '[{"pid":%s,"cwd":"/w","kind":"interactive","name":"x","sessionId":"s","startedAt":1,"status":"%s"}]\n' "$(cat "$STUB_ARGS.pid")" "$(cat "$STUB_ARGS.state")"
  exit 0
fi
echo $$ > "$STUB_ARGS.pid"
printf 'stub-ready\n'
sleep 60
"#;

#[test]
fn claude_background_work_shows_as_working() {
    let mut env = TestEnv::new("background");
    env.stub("claude", CLAUDE_BACKGROUND_STUB);
    let state = env.root.join("stub-args.txt.state");
    std::fs::write(&state, "idle").unwrap();
    env.start_daemon();
    let session = env.create(Agent::Claude);
    env.wait_status(&session.id, Status::Done);
    std::fs::write(&state, "busy").unwrap();
    env.wait_status(&session.id, Status::Working);
    std::fs::write(&state, "idle").unwrap();
    env.wait_status(&session.id, Status::Done);
}

/// Keeps Claude Code's session record for itself, with the status in
/// `$STUB_ARGS.state`, and counts how often `claude agents` is started.
const CLAUDE_RECORD_STUB: &str = r#"if [ "$1" = "agents" ]; then
  echo run >> "$STUB_ARGS.agents"
  echo '[]'
  exit 0
fi
records="$HOME/.claude/sessions"
printf 'stub-ready\n'
while :; do
  printf '{"pid":%s,"cwd":"/w","kind":"interactive","sessionId":"s","startedAt":1,"status":"%s"}' $$ "$(cat "$STUB_ARGS.state")" > "$records/$$.tmp"
  mv "$records/$$.tmp" "$records/$$.json"
  sleep 0.2
done
"#;

#[test]
fn claude_status_comes_from_its_records_without_starting_claude_each_scan() {
    let mut env = TestEnv::new("records");
    env.stub("claude", CLAUDE_RECORD_STUB);
    let state = env.root.join("stub-args.txt.state");
    std::fs::write(&state, "idle").unwrap();
    std::fs::create_dir_all(env.root.join("home/.claude/sessions")).unwrap();
    env.start_daemon();
    let session = env.create(Agent::Claude);
    env.wait_status(&session.id, Status::Done);
    std::fs::write(&state, "busy").unwrap();
    env.wait_status(&session.id, Status::Working);
    std::fs::write(&state, "idle").unwrap();
    env.wait_status(&session.id, Status::Done);
    // Claude Code 2.1.310 records "shell" while background commands run after a turn.
    std::fs::write(&state, "shell").unwrap();
    env.wait_status(&session.id, Status::Working);
    std::fs::write(&state, "idle").unwrap();
    env.wait_status(&session.id, Status::Done);

    let runs = std::fs::read_to_string(env.root.join("stub-args.txt.agents"))
        .unwrap_or_default()
        .lines()
        .count();
    assert!(runs <= 1, "claude agents started {runs} times");
}

/// Counts `claude agents` runs, which the host starts on every outside scan while
/// Claude Code keeps no session records.
const CLAUDE_SCAN_COUNTER: &str = r#"if [ "$1" = "agents" ]; then
  echo run >> "$STUB_ARGS.agents"
  echo '[]'
  exit 0
fi
sleep 30
"#;

#[test]
fn the_outside_scan_slows_while_nobody_looks() {
    let mut env = TestEnv::new("pace");
    env.stub("claude", CLAUDE_SCAN_COUNTER);
    env.start_daemon();
    let runs = env.root.join("stub-args.txt.agents");
    let scans = || {
        std::fs::read_to_string(&runs)
            .unwrap_or_default()
            .lines()
            .count()
    };
    wait_until("first scan", || scans() >= 1);
    std::thread::sleep(Duration::from_secs(7));
    assert_eq!(scans(), 1, "scanned while no client was connected");

    let mut app = env.connect();
    assert!(matches!(
        app.request(&Request::Subscribe { usage: false }),
        Response::Done
    ));
    wait_until("a scan as the client subscribes", || scans() >= 2);
    wait_until("the next scan 5 s later", || scans() >= 3);

    assert!(matches!(
        app.answer(&Request::Visible { visible: false }),
        Response::Done
    ));
    std::thread::sleep(Duration::from_secs(1));
    let hidden = scans();
    std::thread::sleep(Duration::from_secs(7));
    assert_eq!(scans(), hidden, "scanned while the window was hidden");

    assert!(env.hq(&["ls"]).status.success());
    wait_until("a scan after listing", || scans() > hidden);
}

/// Prints the terminal size whenever it changes.
const SIZE_STUB: &str = r#"trap 'printf "size:%s\n" "$(stty size)"' WINCH
printf 'stub-ready\n'
while :; do sleep 0.1; done
"#;

#[test]
fn a_viewer_leaving_returns_the_session_to_the_size_still_shown() {
    let mut env = TestEnv::new("size");
    env.stub("agy", SIZE_STUB);
    env.start_daemon();
    let session = env.create(Agent::Agy);
    let attach = |conn: &mut Conn, cols, rows| {
        assert!(matches!(
            conn.request(&Request::Attach {
                session: session.id.clone(),
                cols,
                rows
            }),
            Response::Attached { .. }
        ));
    };
    let mut app = env.connect();
    attach(&mut app, 100, 30);
    app.output_until("stub-ready");
    app.send(&Request::Resize {
        cols: 120,
        rows: 50,
    });
    app.output_until("size:50 120");
    let mut terminal = env.connect();
    attach(&mut terminal, 110, 40);
    app.output_until("size:40 110");
    assert!(matches!(
        terminal.request(&Request::Detach {}),
        Response::Detached
    ));
    app.output_until("size:50 120");
}

#[test]
fn hooks_route_to_their_own_session_and_stay_out_of_the_log() {
    let mut env = TestEnv::new("hooks");
    env.stub("claude", RECORDING_STUB);
    env.start_daemon();
    let first = env.create(Agent::Claude);
    let second = env.create(Agent::Claude);
    assert_eq!(first.agent_session_id.as_deref(), Some(first.id.as_str()));

    let payload = ForwardedPayload {
        agent: "claude".into(),
        raw: format!(
            r#"{{"session_id":"{}","hook_event_name":"UserPromptSubmit","prompt":"SECRET-HOOK-TEXT"}}"#,
            first.id
        ),
        session: Some(first.id.clone()),
    };
    let mut hooks = UnixStream::connect(env.host_dir().join("hooks.sock")).unwrap();
    writeln!(hooks, "{}", serde_json::to_string(&payload).unwrap()).unwrap();
    drop(hooks);

    env.wait_status(&first.id, Status::Working);
    assert_eq!(env.session(&second.id).status, Status::Done);

    let log = env.log();
    assert!(log.contains("session started"));
    assert!(!log.contains("SECRET-HOOK-TEXT"));
    assert!(!log.contains("recording-stub"));
}

/// Reports a turn with OSC 7501, then waits.
const PROGRAM_STATUS_STUB: &str = r#"printf '\033]7501;state=working\007'
sleep 1
printf '\033]7501;state=done\007'
sleep 30
"#;

/// Opening a stopped session is not something its agent did: an agent that reports a
/// turn while loading, before the owner has typed, leaves the row's age alone.
#[test]
fn resuming_keeps_the_age_through_the_agents_startup_report() {
    let mut env = TestEnv::new("resumeage");
    env.stub("claude", PROGRAM_STATUS_STUB);
    env.start_daemon();
    let session = env.create(Agent::Claude);
    env.wait_status(&session.id, Status::Done);
    assert!(matches!(
        env.connect().request(&Request::Kill {
            session: session.id.clone()
        }),
        Response::Done
    ));
    wait_until("ended", || !env.session(&session.id).live);
    let stopped_at = env.session(&session.id).last_activity_at;
    std::thread::sleep(Duration::from_millis(50));
    match env.connect().request(&Request::Resume {
        session: session.id.clone(),
        term: None,
    }) {
        Response::Created { .. } => {}
        other => panic!("unexpected {other:?}"),
    }
    // The app marks the opened session seen at once; that is not asking for work.
    assert!(matches!(
        env.connect().request(&Request::Seen {
            session: session.id.clone()
        }),
        Response::Done
    ));
    env.wait_status(&session.id, Status::Working);
    env.wait_status(&session.id, Status::Done);
    assert_eq!(env.session(&session.id).last_activity_at, stopped_at);
}

/// Claude Code can deliver a tool hook after its turn has ended; the agent's own status
/// report wins, so the session does not become working again.
#[test]
fn a_late_tool_hook_does_not_restart_a_finished_turn() {
    let mut env = TestEnv::new("latehook");
    env.stub("claude", PROGRAM_STATUS_STUB);
    env.start_daemon();
    let session = env.create(Agent::Claude);
    env.wait_status(&session.id, Status::Working);
    env.wait_status(&session.id, Status::Done);
    let payload = ForwardedPayload {
        agent: "claude".into(),
        raw: format!(
            r#"{{"session_id":"{}","hook_event_name":"PostToolUse","tool_name":"Bash"}}"#,
            session.id
        ),
        session: Some(session.id.clone()),
    };
    let mut hooks = UnixStream::connect(env.host_dir().join("hooks.sock")).unwrap();
    writeln!(hooks, "{}", serde_json::to_string(&payload).unwrap()).unwrap();
    drop(hooks);
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(env.session(&session.id).status, Status::Done);
}

/// Details a status line reports reach the client following that session, with the
/// branch read from the folder's `.git`, and never reach the index or the log.
#[test]
fn details_reach_followers_and_stay_in_memory() {
    let mut env = TestEnv::new("details");
    env.stub("claude", RECORDING_STUB);
    std::fs::create_dir_all(env.work().join(".git")).unwrap();
    std::fs::write(env.work().join(".git/HEAD"), "ref: refs/heads/feature-x\n").unwrap();
    env.start_daemon();
    let session = env.create(Agent::Claude);

    let mut follower = env.connect();
    assert!(matches!(
        follower.request(&Request::Subscribe { usage: false }),
        Response::Done
    ));
    follower.send(&Request::FollowDetails {
        session: Some(session.id.clone()),
    });
    let first = loop {
        match follower.next_response() {
            Response::DetailsChanged {
                session: id,
                details,
            } if id == session.id => {
                break details;
            }
            _ => continue,
        }
    };
    assert_eq!(first.branch.as_deref(), Some("feature-x"));
    assert_eq!(first.model, None, "nothing reported yet");

    let payload = ForwardedPayload {
        agent: "details".into(),
        raw: r#"{"model":"MODEL-MARK-77","context_used_percent":62.0,"context_left_percent":38.0,"cost_usd":0.42,"context_window":999999}"#.into(),
        session: Some(session.id.clone()),
    };
    let mut hooks = UnixStream::connect(env.host_dir().join("hooks.sock")).unwrap();
    writeln!(hooks, "{}", serde_json::to_string(&payload).unwrap()).unwrap();
    drop(hooks);
    let reported = loop {
        match follower.next_response() {
            Response::DetailsChanged { details, .. } if details.model.is_some() => break details,
            _ => continue,
        }
    };
    assert_eq!(reported.model.as_deref(), Some("MODEL-MARK-77"));
    assert_eq!(reported.context_used_percent, Some(62.0));
    assert_eq!(reported.cost_usd, Some(0.42));
    assert_eq!(reported.branch.as_deref(), Some("feature-x"));

    assert!(!contains(&env.index_bytes(), "MODEL-MARK-77"));
    assert!(!contains(&env.index_bytes(), "999999"));
    assert!(!env.log().contains("MODEL-MARK-77"));
}

#[test]
fn launched_agents_do_not_inherit_a_parent_agent_session() {
    let mut env = TestEnv::new("markers");
    env.stub(
        "claude",
        r#"if [ "$1" = "agents" ]; then echo '[]'; exit 0; fi
printf 'child=%s code=%s bedrock=%s\n' "${CLAUDE_CODE_CHILD_SESSION-unset}" "${CLAUDECODE-unset}" "${CLAUDE_CODE_USE_BEDROCK-unset}" >> "$STUB_ARGS"
sleep 30
"#,
    );
    // The host was started from inside another agent's session.
    let child = env
        .command()
        .arg("daemon")
        .env("CLAUDE_CODE_CHILD_SESSION", "1")
        .env("CLAUDECODE", "1")
        .env("CLAUDE_CODE_USE_BEDROCK", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    env.daemons.push(child);
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());

    env.create(Agent::Claude);
    wait_until("stub environment", || {
        std::fs::read_to_string(env.root.join("stub-args.txt"))
            .is_ok_and(|seen| seen.contains("child=unset code=unset bedrock=1"))
    });
}

#[test]
fn a_host_crash_restarts_what_ran_with_the_agent_resume() {
    let mut env = TestEnv::new("restart");
    env.stub("claude", RECORDING_STUB);
    env.start_daemon();
    let session = env.create(Agent::Claude);
    let unsaved = env.create(Agent::Claude);
    wait_until("stubs started", || {
        std::fs::read_to_string(env.root.join("stub-args.txt"))
            .is_ok_and(|args| args.matches("--session-id").count() == 2)
    });
    // Claude Code saved the first conversation; the second never had a message.
    env.claude_transcript(&session.id, "Saved");

    // The host dies without cleaning up.
    let mut crashed = env.daemons.pop().unwrap();
    crashed.kill().unwrap();
    crashed.wait().unwrap();
    std::fs::write(env.root.join("stub-args.txt"), "").unwrap();

    env.start_daemon();
    wait_until("started with the host", || {
        env.session(&session.id).live && env.session(&unsaved.id).live
    });
    assert_eq!(env.session(&session.id).name, "claude-1");
    wait_until("start arguments", || {
        std::fs::read_to_string(env.root.join("stub-args.txt")).is_ok_and(|args| {
            args.contains(&format!("--resume\n{}\n", session.id))
                && args.contains(&format!("--session-id\n{}\n", unsaved.id))
        })
    });
    assert!(matches!(
        env.connect().request(&Request::Resume {
            session: session.id.clone(),
            term: None
        }),
        Response::Error {
            code: ErrorCode::AlreadyLive,
            ..
        }
    ));
}

#[test]
fn failures_are_specific_and_record_nothing() {
    let mut env = TestEnv::new("failures");
    env.start_daemon();
    let mut conn = env.connect();
    if let Some(missing) = absent_agent(&[Agent::Agy]) {
        match conn.request(&Request::New {
            agent: missing,
            cwd: env.work().to_string_lossy().into_owned(),
            args: vec![],
            term: None,
            name: None,
            group: None,
        }) {
            Response::Error { code, message } => {
                assert_eq!(code, ErrorCode::AgentNotFound);
                assert!(message.contains(missing.binary()));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    env.stub("agy", INTERACTIVE_STUB);
    match conn.request(&Request::New {
        agent: Agent::Agy,
        cwd: "/no/such/folder".into(),
        args: vec![],
        term: None,
        name: None,
        group: None,
    }) {
        Response::Error { code, message } => {
            assert_eq!(code, ErrorCode::FolderNotFound);
            assert!(message.contains("/no/such/folder"));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(env.sessions().is_empty());

    // Garbage closes only that connection.
    conn.send_frame(&Frame::Control(b"{not json".to_vec()));
    assert!(matches!(
        conn.next_response(),
        Response::Error {
            code: ErrorCode::BadRequest,
            ..
        }
    ));
    assert!(env.sessions().is_empty());

    let mut stranger = Conn::open(&env.host_dir().join("agentd.sock"));
    assert!(matches!(
        stranger.request(&Request::Hello {
            protocol: PROTOCOL_VERSION + 1
        }),
        Response::Error {
            code: ErrorCode::ProtocolMismatch,
            ..
        }
    ));
}

/// The calls the desktop app's session list makes: list and follow changes, start a
/// session, attach to it, and report host errors.
#[test]
fn app_client_lists_starts_attaches_and_reports_errors() {
    let mut env = TestEnv::new("appclient");
    env.stub("agy", INTERACTIVE_STUB);
    env.start_daemon();
    let paths = hq_client::HostPaths::in_dir(env.host_dir());

    let (mut feed, catalog) = hq_client::SessionFeed::open(&paths, false).unwrap();
    assert!(
        catalog
            .sessions
            .iter()
            .all(|session| session.origin == hq_protocol::Origin::External)
    );
    let (changes, changed) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(Some(change)) = feed.next_change() {
            if let hq_client::Change::Session(session) = change
                && changes.send(session).is_err()
            {
                return;
            }
        }
    });

    let created = hq_client::create_session(&paths, Agent::Agy, &env.work(), None).unwrap();
    assert_eq!(created.agent, Agent::Agy);
    let deadline = Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let session = changed
            .recv_timeout(left)
            .expect("the new session is reported");
        if session.id == created.id {
            break;
        }
    }

    let client = hq_client::attach(&paths, &created.id, 80, 24).unwrap();
    let (output, seen) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = client.reader;
        let mut text = String::new();
        while let Ok(Some(frame)) = read_frame(&mut reader) {
            if let Frame::Output(bytes) = frame {
                text.push_str(&String::from_utf8_lossy(&bytes));
                if text.contains("stub-ready") {
                    let _ = output.send(());
                    return;
                }
            }
        }
    });
    seen.recv_timeout(WAIT).expect("the agent's output arrives");

    let missing_folder =
        hq_client::create_session(&paths, Agent::Agy, Path::new("/no/such/folder"), None)
            .unwrap_err();
    assert!(
        format!("{missing_folder:#}").contains("/no/such/folder"),
        "{missing_folder:#}"
    );
    let Err(unknown) = hq_client::attach(&paths, "no-such-session", 80, 24) else {
        panic!("attaching to an unknown session must fail");
    };
    assert!(
        format!("{unknown:#}").contains("no-such-session"),
        "{unknown:#}"
    );

    // A hang-up, then the pane's Resume.
    assert!(matches!(
        env.connect().request(&Request::Kill {
            session: created.id.clone()
        }),
        Response::Done
    ));
    env.wait_status(&created.id, Status::Inactive);
    hq_client::remove_from_sidebar(&paths, std::slice::from_ref(&created.id)).unwrap();
    let resumed = hq_client::resume(&paths, &created.id).unwrap();
    assert!(resumed.live);
    assert!(
        resumed.in_sidebar,
        "resuming here puts the session back in the sidebar"
    );
    hq_client::rename(&paths, &created.id, "renamed-agy").unwrap();
    assert_eq!(env.session(&created.id).name, "renamed-agy");
    assert!(env.session(&created.id).owner_named);
    hq_client::rename(&paths, &created.id, "  ").unwrap();
    let cleared = env.session(&created.id);
    assert!(
        cleared.name.starts_with("agy-") && !cleared.owner_named,
        "a blank name gives back a generated one: {}",
        cleared.name
    );
}

#[test]
fn cli_starts_the_host_and_manages_sessions() {
    let env = TestEnv::new("cli");
    env.stub("agy", INTERACTIVE_STUB);
    let work = env.work();
    let work = work.to_string_lossy();

    let started = env.hq(&["new", "agy", "--detach", "--cwd", &work]);
    assert!(started.status.success(), "{started:?}");
    assert!(String::from_utf8_lossy(&started.stdout).contains("agy-1 (agy) running in"));

    let listed = env.hq(&["ls"]);
    let text = String::from_utf8_lossy(&listed.stdout);
    let line = text
        .lines()
        .find(|line| line.contains("agy-1"))
        .expect(&text);
    assert!(line.ends_with("work"), "the folder is on the row: {text}");
    assert!(
        !text.contains("Ungrouped"),
        "no heading without groups: {text}"
    );

    assert!(
        env.hq(&["rename", "agy-1", "landing-hero"])
            .status
            .success()
    );
    let duplicate = env.hq(&["new", "agy", "-d", "--name", "landing-hero", "--cwd", &work]);
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already used"));
    let json = env.hq(&["ls", "--json"]);
    let sessions: Vec<SessionInfo> = serde_json::from_slice::<Vec<SessionInfo>>(&json.stdout)
        .unwrap()
        .into_iter()
        .filter(|session| session.origin != hq_protocol::Origin::External)
        .collect();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "landing-hero");

    let refused = env.hq(&["stop"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--yes"));

    assert!(env.hq(&["kill", "landing-hero"]).status.success());
    env.wait_status(&sessions[0].id, Status::Inactive);

    // A command right after `stop` reaches a fresh host, never the stopping one.
    assert!(
        env.hq(&["new", "agy", "--detach", "--cwd", &work])
            .status
            .success()
    );
    let stopped = env.hq(&["stop", "--yes"]);
    assert!(stopped.status.success(), "{stopped:?}");
    let restarted = env.hq(&["new", "agy", "--detach", "--cwd", &work]);
    assert!(restarted.status.success(), "{restarted:?}");
    // The fresh host runs the new session and starts again the one the stop ended.
    let live_count = || {
        let listed: Vec<SessionInfo> =
            serde_json::from_slice(&env.hq(&["ls", "--json"]).stdout).unwrap();
        listed.iter().filter(|session| session.live).count()
    };
    wait_until("both running on the fresh host", || live_count() == 2);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(live_count(), 2);
    assert!(env.hq(&["stop", "--yes"]).status.success());
    wait_until("host gone", || {
        UnixStream::connect(env.host_dir().join("agentd.sock")).is_err()
    });
}

#[test]
fn stop_ends_running_sessions_and_the_next_host_starts_them() {
    let mut env = TestEnv::new("stop");
    env.stub("agy", INTERACTIVE_STUB);
    env.start_daemon();
    let session = env.create(Agent::Agy);
    assert!(matches!(
        env.connect().request(&Request::Stop {}),
        Response::Done
    ));
    let mut host = env.daemons.pop().unwrap();
    let status = host.wait().unwrap();
    assert!(status.success());

    env.start_daemon();
    let listed = env.session(&session.id);
    assert!(listed.in_sidebar, "a host stop removes nothing");
    assert!(listed.starting || listed.live, "{listed:?}");
    wait_until("started again", || {
        let session = env.session(&session.id);
        session.live && !session.starting
    });
}

/// Records the folder it starts in, reports itself idle with OSC 7501 shortly after, and
/// exits 0 on `/quit`.
const STARTING_STUB: &str = r#"if [ "$1" = "agents" ]; then echo '[]'; exit 0; fi
printf '%s\n' "$PWD" >> "$STUB_ARGS"
printf 'stub-ready\n'
sleep 0.3
printf '\033]7501;state=done\007'
while read line; do
  case "$line" in
    /quit) exit 0 ;;
  esac
done
"#;

#[test]
fn sessions_start_with_the_host_in_sidebar_order_and_failures_stay_inactive() {
    let mut env = TestEnv::new("starts");
    env.stub("claude", STARTING_STUB);
    env.start_daemon();
    let mut created = Vec::new();
    for folder in ["alpha", "beta", "gone", "quit"] {
        let cwd = env.work().join(folder);
        std::fs::create_dir_all(&cwd).unwrap();
        let session = match env.connect().request(&Request::New {
            agent: Agent::Claude,
            cwd: cwd.to_string_lossy().into_owned(),
            args: vec![],
            term: None,
            name: None,
            group: None,
        }) {
            Response::Created { session } => session,
            other => panic!("unexpected create answer {other:?}"),
        };
        // One group each, in this order, fixes the sidebar's order.
        assert!(matches!(
            env.connect().request(&Request::NewGroup {
                name: Some(folder.to_owned()),
                session: Some(session.id.clone()),
            }),
            Response::GroupCreated { .. }
        ));
        created.push(session);
    }
    let [alpha, beta, gone, quit] = <[SessionInfo; 4]>::try_from(created).unwrap();
    let mut viewer = env.connect();
    assert!(matches!(
        viewer.request(&Request::Attach {
            session: quit.id.clone(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    viewer.output_until("stub-ready");
    viewer.send_frame(&Frame::Input(b"/quit\n".to_vec()));
    wait_until("quit", || !env.session(&quit.id).live);
    let quit_row = env.session(&quit.id);
    assert!(quit_row.in_sidebar, "a quit keeps the session");
    assert_eq!(quit_row.status, Status::Inactive);

    assert!(matches!(
        env.connect().request(&Request::Stop {}),
        Response::Done
    ));
    env.daemons.pop().unwrap().wait().unwrap();
    std::fs::remove_dir_all(env.work().join("gone")).unwrap();
    std::fs::write(env.root.join("stub-args.txt"), "").unwrap();

    env.start_daemon();
    for session in [&alpha, &beta, &gone] {
        let listed = env.session(&session.id);
        assert!(listed.starting || listed.live, "{listed:?}");
    }
    assert!(!env.session(&quit.id).starting, "inactive stays inactive");
    wait_until("all settled", || {
        [&alpha, &beta, &gone]
            .iter()
            .all(|session| !env.session(&session.id).starting)
    });
    assert!(env.session(&alpha.id).live);
    assert!(env.session(&beta.id).live);
    let failed = env.session(&gone.id);
    assert!(!failed.live);
    assert_eq!(failed.status, Status::Inactive);
    let reason = failed.start_error.expect("a start failure").reason;
    assert!(reason.contains("does not exist"), "{reason}");
    assert!(!env.session(&quit.id).live);

    let started = std::fs::read_to_string(env.root.join("stub-args.txt")).unwrap();
    let alpha_at = started.find("/alpha").expect("alpha started");
    let beta_at = started.find("/beta").expect("beta started");
    assert!(alpha_at < beta_at, "sidebar order: {started}");
    assert!(!started.contains("/quit"), "{started}");
}

/// Never reports itself, so each start waits the full readiness time.
const SILENT_STUB: &str = r#"if [ "$1" = "agents" ]; then echo '[]'; exit 0; fi
printf '%s\n' "$PWD" >> "$STUB_ARGS"
sleep 60
"#;

#[test]
fn a_session_asked_for_starts_ahead_of_its_turn() {
    let mut env = TestEnv::new("ahead");
    env.stub("claude", SILENT_STUB);
    env.start_daemon();
    let mut created = Vec::new();
    for folder in ["first", "second", "third"] {
        let cwd = env.work().join(folder);
        std::fs::create_dir_all(&cwd).unwrap();
        let session = match env.connect().request(&Request::New {
            agent: Agent::Claude,
            cwd: cwd.to_string_lossy().into_owned(),
            args: vec![],
            term: None,
            name: None,
            group: None,
        }) {
            Response::Created { session } => session,
            other => panic!("unexpected create answer {other:?}"),
        };
        assert!(matches!(
            env.connect().request(&Request::NewGroup {
                name: Some(folder.to_owned()),
                session: Some(session.id.clone()),
            }),
            Response::GroupCreated { .. }
        ));
        created.push(session);
    }
    assert!(matches!(
        env.connect().request(&Request::Stop {}),
        Response::Done
    ));
    env.daemons.pop().unwrap().wait().unwrap();

    env.start_daemon();
    let third = &created[2];
    assert!(env.session(&third.id).starting);
    match env.connect().request(&Request::Resume {
        session: third.id.clone(),
        term: None,
    }) {
        Response::Created { session } => assert!(session.live),
        other => panic!("unexpected resume answer {other:?}"),
    }
    assert!(
        !env.session(&created[1].id).live,
        "the second waits while the first gets ready"
    );
    assert!(
        env.session(&third.id).starting,
        "starting until it is ready"
    );
}

/// Reads lines like a prompt: exactly `/quit` exits 0, `crash` exits 3. Ctrl-U clears
/// the line in the terminal's line editing, as agy's Ctrl-U clears its prompt, so a
/// draft left in front of `/quit` keeps the stub running.
const EXITING_STUB: &str = r#"printf 'stub-ready\n'
while read line; do
  case "$line" in
    /quit) exit 0 ;;
    crash) exit 3 ;;
  esac
done
"#;

/// Ignores its exit command and being hung up.
const STUBBORN_STUB: &str = r#"trap '' HUP
printf 'stub-ready\n'
while :; do sleep 1; done
"#;

#[test]
fn closing_quits_with_the_agents_keys_and_leaves_the_sidebar() {
    let mut env = TestEnv::new("close");
    env.stub("agy", EXITING_STUB);
    env.start_daemon();
    let session = env.create(Agent::Agy);
    let mut viewer = env.connect();
    assert!(matches!(
        viewer.request(&Request::Attach {
            session: session.id.clone(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    viewer.output_until("stub-ready");
    viewer.send_frame(&Frame::Input(b"half-typed".to_vec()));
    assert!(matches!(
        env.connect().request(&Request::Close {
            session: session.id.clone()
        }),
        Response::Done
    ));
    wait_until("closed", || !env.session(&session.id).live);
    let closed = env.session(&session.id);
    assert!(!closed.in_sidebar);
    assert!(env.log().contains("exit_code=0"), "{}", env.log());
    assert!(!env.log().contains("close fell back"), "{}", env.log());
}

#[test]
fn only_sessions_running_at_a_crash_start_again() {
    let mut env = TestEnv::new("interrupt");
    env.stub("agy", EXITING_STUB);
    env.start_daemon();
    let running = env.create(Agent::Agy);
    let exited = env.create(Agent::Agy);
    let closed = env.create(Agent::Agy);
    let mut viewer = env.connect();
    assert!(matches!(
        viewer.request(&Request::Attach {
            session: exited.id.clone(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    viewer.output_until("stub-ready");
    viewer.send_frame(&Frame::Input(b"/quit\r".to_vec()));
    wait_until("agent exited", || !env.session(&exited.id).live);
    assert!(matches!(
        env.connect().request(&Request::Close {
            session: closed.id.clone()
        }),
        Response::Done
    ));
    wait_until("closed", || !env.session(&closed.id).live);

    let mut crashed = env.daemons.pop().unwrap();
    crashed.kill().unwrap();
    crashed.wait().unwrap();
    env.start_daemon();
    wait_until("running one started again", || {
        env.session(&running.id).live
    });
    let quit = env.session(&exited.id);
    assert!(!quit.live && !quit.starting, "the agent ended itself");
    assert!(quit.in_sidebar);
    let gone = env.session(&closed.id);
    assert!(!gone.live && !gone.in_sidebar, "closed on purpose");
}

#[test]
fn hq_close_waits_for_the_agent_and_reports() {
    let mut env = TestEnv::new("hqclose");
    env.stub("agy", EXITING_STUB);
    env.start_daemon();
    let work = env.work().to_string_lossy().into_owned();
    assert!(
        env.hq(&["new", "agy", "--detach", "--cwd", &work])
            .status
            .success()
    );
    let closed = env.hq(&["close", "agy-1"]);
    assert!(closed.status.success(), "{closed:?}");
    assert_eq!(String::from_utf8_lossy(&closed.stdout), "Closed agy-1\n");
    let listed = String::from_utf8_lossy(&env.hq(&["ls"]).stdout).into_owned();
    assert!(!listed.contains("agy-1"), "{listed}");
}

#[test]
fn quitting_inside_or_a_crash_keeps_the_session_inactive() {
    let mut env = TestEnv::new("quit");
    env.stub("agy", EXITING_STUB);
    env.start_daemon();
    for line in ["/quit\n", "crash\n"] {
        let session = env.create(Agent::Agy);
        let mut viewer = env.connect();
        assert!(matches!(
            viewer.request(&Request::Attach {
                session: session.id.clone(),
                cols: 80,
                rows: 24
            }),
            Response::Attached { .. }
        ));
        viewer.output_until("stub-ready");
        viewer.send_frame(&Frame::Input(line.as_bytes().to_vec()));
        wait_until("ended", || !env.session(&session.id).live);
        let ended = env.session(&session.id);
        assert!(ended.in_sidebar, "{line:?}");
        assert!(!ended.starting, "{line:?}");
        assert_eq!(ended.status, Status::Inactive);
    }
}

#[test]
fn an_agent_that_ignores_closing_is_hung_up_then_killed() {
    let mut env = TestEnv::new("stubborn");
    env.stub("agy", STUBBORN_STUB);
    env.start_daemon();
    let session = env.create(Agent::Agy);
    let mut viewer = env.connect();
    assert!(matches!(
        viewer.request(&Request::Attach {
            session: session.id.clone(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    viewer.output_until("stub-ready");
    assert!(matches!(
        env.connect().request(&Request::Close {
            session: session.id.clone()
        }),
        Response::Done
    ));
    let closing = env.session(&session.id);
    assert!(
        closing.live && !closing.in_sidebar,
        "the session leaves the sidebar before its agent has stopped"
    );
    let deadline = Instant::now() + Duration::from_secs(25);
    while env.session(&session.id).live {
        assert!(Instant::now() < deadline, "still running: {}", env.log());
        std::thread::sleep(Duration::from_millis(200));
    }
    let log = env.log();
    assert!(log.contains("close fell back to hangup agent=agy"), "{log}");
    assert!(log.contains("close killed agent=agy ok=true"), "{log}");
    assert!(!env.session(&session.id).in_sidebar);
}

/// A host left running by an older version (as after an upgrade) refuses this
/// version's `Hello`; `hq stop` still stops it, through the `Hello` and `Stop` every
/// version understands, and other commands say to run `hq stop` instead of failing to
/// start a second host.
#[test]
fn stop_reaches_a_host_of_another_version() {
    let env = TestEnv::new("oldhost");
    std::fs::create_dir_all(env.host_dir()).unwrap();
    let listener =
        std::os::unix::net::UnixListener::bind(env.host_dir().join("agentd.sock")).unwrap();
    let old = PROTOCOL_VERSION - 1;
    let (stopped_tx, stopped) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            let reply = |writer: &mut UnixStream, json: String| {
                write_frame(writer, &Frame::Control(json.into_bytes())).unwrap();
            };
            let Ok(Some(Frame::Control(hello))) = read_frame(&mut reader) else {
                continue;
            };
            let hello: serde_json::Value = serde_json::from_slice(&hello).unwrap();
            let protocol = hello["protocol"].as_u64().unwrap();
            if protocol != u64::from(old) {
                // Worded as hosts of protocol 1 to 5 word it.
                reply(
                    &mut writer,
                    format!(
                        r#"{{"type":"error","code":"protocol-mismatch","message":"client speaks protocol {protocol}, host speaks protocol {old}; restart the host with `hq stop`"}}"#
                    ),
                );
                continue;
            }
            reply(
                &mut writer,
                format!(r#"{{"type":"welcome","protocol":{old},"version":"0.0.0"}}"#),
            );
            if let Ok(Some(Frame::Control(request))) = read_frame(&mut reader) {
                let request: serde_json::Value = serde_json::from_slice(&request).unwrap();
                stopped_tx
                    .send(request["type"].as_str().unwrap().to_owned())
                    .unwrap();
                reply(&mut writer, r#"{"type":"done"}"#.to_owned());
            }
        }
    });

    let listed = env.hq(&["ls"]);
    assert!(!listed.status.success());
    let said = String::from_utf8_lossy(&listed.stderr).into_owned();
    assert!(said.contains("restart the host with `hq stop`"), "{said}");

    let refused = env.hq(&["stop"]);
    assert!(
        !refused.status.success(),
        "no terminal to ask, and no --yes"
    );
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--yes"));
    assert!(stopped.try_recv().is_err(), "nothing stopped without --yes");

    let stop = env.hq(&["stop", "--yes"]);
    assert!(stop.status.success(), "{stop:?}");
    assert_eq!(stopped.recv_timeout(WAIT).unwrap(), "stop");
    let said = String::from_utf8_lossy(&stop.stdout).into_owned();
    assert!(said.contains(&format!("protocol {old}")), "{said}");
}

/// A stub `claude` that also answers `claude agents --json` with outside sessions:
/// conversation `conv-outside` (linked) and `conv-unknown` (no history), both as pid 1,
/// which is never a descendant of the host.
const CLAUDE_WITH_OUTSIDE: &str = r#"if [ "$1" = "agents" ]; then
  printf '[{"pid":1,"cwd":"%s","kind":"interactive","sessionId":"conv-outside","startedAt":1000,"status":"busy"},' "$WORK"
  printf '{"pid":1,"cwd":"%s","kind":"interactive","sessionId":"conv-unknown","startedAt":1000,"status":"waiting"}]\n' "$WORK"
  exit 0
fi
printf '%s\n' "$@" >> "$STUB_ARGS"
sleep 30
"#;

/// Lists one Claude session with no known conversation while `$STUB_ARGS.outside`
/// exists, and none otherwise.
const CLAUDE_OUTSIDE_COMING_AND_GOING: &str = r#"if [ "$1" = "agents" ]; then
  if [ -e "$STUB_ARGS.outside" ]; then
    printf '[{"pid":1,"cwd":"%s","kind":"interactive","sessionId":"conv-unknown","startedAt":1000,"status":"waiting"}]\n' "$WORK"
  else
    echo '[]'
  fi
  exit 0
fi
sleep 30
"#;

const TITLE_SECRET: &str = "TITLE-SECRET-91";

/// Answers `account/rateLimits/read` like `codex app-server`, recording each start;
/// fails while `<STUB_ARGS>.fail` exists.
const CODEX_USAGE_STUB: &str = r#"[ "$1" = app-server ] || { sleep 30; exit 0; }
echo started >> "$STUB_ARGS"
[ -e "$STUB_ARGS.fail" ] && exit 1
while read line; do
  case "$line" in
    *rateLimits*) echo '{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":4102444800}}}}' ;;
    *account/read*) echo '{"id":3,"result":{"account":{"type":"chatgpt","email":"someone@example.com","planType":"plus"}}}' ;;
  esac
done
"#;

impl TestEnv {
    fn claude_transcript(&self, id: &str, title: &str) -> PathBuf {
        let path = self
            .root
            .join("home/.claude/projects/-work")
            .join(format!("{id}.jsonl"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                "{{\"type\":\"user\",\"cwd\":\"{}\",\"message\":{{\"content\":\"PROMPT-TEXT\"}}}}\n{{\"type\":\"ai-title\",\"aiTitle\":\"{title}\",\"sessionId\":\"{id}\"}}\n",
                self.work().display()
            ),
        )
        .unwrap();
        path
    }

    fn age_file(path: &Path, days: u64) {
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(std::time::SystemTime::now() - Duration::from_secs(days * 86_400))
            .unwrap();
    }

    fn index_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for suffix in ["", "-wal"] {
            let path = format!("{}{suffix}", self.host_dir().join("state.db").display());
            bytes.extend(std::fs::read(path).unwrap_or_default());
        }
        bytes
    }
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// Removing past conversations: Claude Code's transcript and the folder beside it go
/// to the Trash, Codex runs its own archive and delete, agy is refused, and a removed
/// conversation is no longer offered.
#[test]
fn past_conversations_are_removed_the_way_their_agent_allows() {
    let mut env = TestEnv::new("remove");
    let transcript = env.claude_transcript("conv-gone", "Old work");
    std::fs::create_dir_all(transcript.with_extension("").join("subagents")).unwrap();
    let rollout = |id: &str| {
        let path = env.root.join(format!(
            "home/.codex/sessions/2026/10/20/rollout-{id}.jsonl"
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"cwd\":\"{}\",\"source\":\"cli\"}}}}\n{{\"type\":\"response_item\",\"payload\":{{}}}}\n",
                env.work().display()
            ),
        )
        .unwrap();
    };
    rollout("11111111-1111-1111-1111-111111111111");
    rollout("22222222-2222-2222-2222-222222222222");
    env.stub(
        "codex",
        r#"echo "$@" >> "$STUB_ARGS"; [ "$1" = archive ] || [ "$1" = delete ]"#,
    );
    let trash = env.root.join("trash");
    let mut daemon = env.command();
    daemon.env("HQ_TRASH_DIR", &trash);
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    let row = |env: &TestEnv, conversation: &str| {
        env.sessions()
            .into_iter()
            .find(|session| session.agent_session_id.as_deref() == Some(conversation))
    };
    wait_until("history imported", || {
        row(&env, "conv-gone").is_some()
            && row(&env, "22222222-2222-2222-2222-222222222222").is_some()
    });
    let mut conn = env.connect();
    let mut remove = |session: String, way: hq_protocol::RemoveWay| {
        conn.request(&Request::RemoveConversation { session, way })
    };

    let claude = row(&env, "conv-gone").unwrap();
    assert!(matches!(
        remove(claude.id.clone(), hq_protocol::RemoveWay::Delete),
        Response::Error {
            code: ErrorCode::BadRequest,
            ..
        }
    ));
    assert!(matches!(
        remove(claude.id.clone(), hq_protocol::RemoveWay::Trash),
        Response::Done
    ));
    assert!(!transcript.exists());
    assert!(trash.join("conv-gone.jsonl").is_file());
    assert!(
        trash.join("conv-gone/subagents").is_dir(),
        "the folder beside it too"
    );
    assert_eq!(
        row(&env, "conv-gone").map(|session| session.offered),
        Some(false)
    );

    let archived = row(&env, "11111111-1111-1111-1111-111111111111").unwrap();
    assert!(matches!(
        remove(archived.id, hq_protocol::RemoveWay::Archive),
        Response::Done
    ));
    let deleted = row(&env, "22222222-2222-2222-2222-222222222222").unwrap();
    assert!(matches!(
        remove(deleted.id, hq_protocol::RemoveWay::Delete),
        Response::Done
    ));
    let ran = std::fs::read_to_string(env.root.join("stub-args.txt")).unwrap();
    assert!(
        ran.contains("archive 11111111-1111-1111-1111-111111111111"),
        "{ran}"
    );
    assert!(
        ran.contains("delete --force 22222222-2222-2222-2222-222222222222"),
        "{ran}"
    );
}

/// Add sessions offers a past conversation only while its agent keeps it and when the
/// owner started it: a scripted run is not offered, and one the agent deleted stops
/// being offered without the row going away.
#[test]
fn past_conversations_are_offered_only_while_kept_and_owner_started() {
    let mut env = TestEnv::new("offered");
    let typed = env.claude_transcript("conv-typed", "Typed work");
    let scripted = env.claude_transcript("conv-scripted", "Scripted work");
    std::fs::write(
        &scripted,
        format!(
            "{{\"type\":\"user\",\"cwd\":\"{}\",\"entrypoint\":\"sdk-cli\"}}\n",
            env.work().display()
        ),
    )
    .unwrap();
    let mut daemon = env.command();
    daemon.env("HQ_HISTORY_INTERVAL_SECS", "1");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    let offered = |env: &TestEnv, conversation: &str| {
        env.sessions()
            .into_iter()
            .find(|session| session.agent_session_id.as_deref() == Some(conversation))
            .map(|session| session.offered)
    };
    wait_until("both imported", || {
        offered(&env, "conv-typed").is_some() && offered(&env, "conv-scripted").is_some()
    });
    assert_eq!(offered(&env, "conv-typed"), Some(true));
    assert_eq!(offered(&env, "conv-scripted"), Some(false));

    std::fs::remove_file(&typed).unwrap();
    wait_until("the deleted conversation is no longer offered", || {
        offered(&env, "conv-typed") == Some(false)
    });
}

/// Claude Code writes a conversation's title after its first message, so the title
/// often arrives in a later import pass than the conversation, with no new activity.
#[test]
fn a_title_found_later_reaches_subscribers() {
    let mut env = TestEnv::new("retitle");
    let path = env.claude_transcript("conv-late", "placeholder");
    std::fs::write(
        &path,
        format!(
            "{{\"type\":\"user\",\"cwd\":\"{}\",\"message\":{{\"content\":\"PROMPT-TEXT\"}}}}\n",
            env.work().display()
        ),
    )
    .unwrap();
    TestEnv::age_file(&path, 1);
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();

    let mut daemon = env.command();
    daemon.env("HQ_HISTORY_INTERVAL_SECS", "1");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    wait_until("the conversation imported", || {
        env.sessions()
            .iter()
            .any(|session| session.agent_session_id.as_deref() == Some("conv-late"))
    });

    let paths = hq_client::HostPaths::in_dir(env.host_dir());
    let (mut feed, _) = hq_client::SessionFeed::open(&paths, false).unwrap();
    let (changes, changed) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(Some(change)) = feed.next_change() {
            if let hq_client::Change::Session(session) = change
                && changes.send(session).is_err()
            {
                return;
            }
        }
    });

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    writeln!(
        file,
        "{{\"type\":\"ai-title\",\"aiTitle\":\"Late title\",\"sessionId\":\"conv-late\"}}"
    )
    .unwrap();
    // Same activity as before: only the title changed.
    file.set_modified(modified).unwrap();
    drop(file);

    let deadline = Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let session = changed
            .recv_timeout(left)
            .expect("the new title is reported");
        if session.agent_session_id.as_deref() == Some("conv-late") {
            assert_eq!(session.title.as_deref(), Some("Late title"));
            break;
        }
    }
}

/// The first list after the host starts already holds past conversations, so Add
/// sessions never opens empty while the import runs.
#[test]
fn the_first_list_waits_for_the_first_history_import() {
    let mut env = TestEnv::new("firstlist");
    // Enough history that the import is still running when the first list comes in.
    for index in 0..1500 {
        env.claude_transcript(&format!("conv-{index}"), "Earlier work");
    }
    env.claude_transcript("conv-first", "First work");
    env.start_daemon();
    let sessions = env.sessions();
    assert!(
        sessions
            .iter()
            .any(|session| session.agent_session_id.as_deref() == Some("conv-first")),
        "{sessions:?}"
    );
}

#[test]
fn history_is_imported_with_titles_never_stored() {
    let mut env = TestEnv::new("history");
    env.claude_transcript("conv-recent", TITLE_SECRET);
    let old = env.claude_transcript("conv-old", "Old work");
    TestEnv::age_file(&old, 40);
    let rollout = env
        .root
        .join("home/.codex/sessions/2026/10/20/rollout-x.jsonl");
    std::fs::create_dir_all(rollout.parent().unwrap()).unwrap();
    std::fs::write(
        &rollout,
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"session_id\":\"thread-1\",\"cwd\":\"{}\"}}}}\n",
            env.work().display()
        ),
    )
    .unwrap();
    std::fs::write(
        env.root.join("home/.codex/session_index.jsonl"),
        r#"{"id":"thread-1","thread_name":"Codex thread","updated_at":"x"}"#,
    )
    .unwrap();
    env.start_daemon();

    wait_until("history imported", || env.sessions().len() == 3);
    let sessions = env.sessions();
    let recent = sessions
        .iter()
        .find(|session| session.agent_session_id.as_deref() == Some("conv-recent"))
        .unwrap();
    assert_eq!(recent.title.as_deref(), Some(TITLE_SECRET));
    assert_eq!(recent.status, Status::Inactive);
    assert_eq!(recent.origin, hq_protocol::Origin::History);
    assert_eq!(recent.name, "claude-convrece");
    assert!(
        sessions
            .iter()
            .any(|session| session.title.as_deref() == Some("Codex thread"))
    );

    // Imported history waits in `hq ls --all` until the owner adds it.
    let listed = String::from_utf8_lossy(&env.hq(&["ls"]).stdout).into_owned();
    assert!(listed.contains("The sidebar is empty"), "{listed}");
    assert!(env.hq(&["add", "claude-convrece"]).status.success());
    let listed = String::from_utf8_lossy(&env.hq(&["ls"]).stdout).into_owned();
    assert!(listed.contains(TITLE_SECRET), "{listed}");
    assert!(!listed.contains("Old work"), "{listed}");
    let everything = String::from_utf8_lossy(&env.hq(&["ls", "--all"]).stdout).into_owned();
    assert!(everything.contains("Old work"), "{everything}");

    assert!(
        !contains(&env.index_bytes(), TITLE_SECRET),
        "title stored in the index"
    );
    assert!(!contains(&env.index_bytes(), "PROMPT-TEXT"));
    assert!(!env.log().contains(TITLE_SECRET));
    assert!(env.log().contains("history import conversations=3 added=3"));
}

#[test]
fn hosted_conversation_is_not_imported_twice() {
    let mut env = TestEnv::new("dedupe");
    env.stub("claude", RECORDING_STUB);
    env.start_daemon();
    let hosted = env.create(Agent::Claude);
    // Claude writes its transcript under the id Headquarter chose.
    env.claude_transcript(&hosted.id, "Hosted title");
    assert!(matches!(
        env.connect().request(&Request::Stop {}),
        Response::Done
    ));
    env.daemons.pop().unwrap().wait().unwrap();

    env.start_daemon();
    wait_until("import ran", || env.log().contains("history import"));
    let sessions = env.sessions();
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    assert_eq!(sessions[0].id, hosted.id);
    assert_eq!(sessions[0].origin, hq_protocol::Origin::Hosted);
    assert_eq!(sessions[0].title.as_deref(), Some("Hosted title"));
    assert!(
        sessions[0].in_sidebar,
        "the hosted row keeps its sidebar place"
    );
}

#[test]
fn outside_sessions_are_linked_and_block_resume() {
    let mut env = TestEnv::new("outside");
    env.stub("claude", CLAUDE_WITH_OUTSIDE);
    env.claude_transcript("conv-outside", "Open elsewhere");
    let mut daemon = env.command();
    daemon.env("WORK", env.work());
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());

    wait_until("outside sessions seen", || {
        let sessions = env.all_sessions();
        sessions.iter().any(|session| {
            session.agent_session_id.as_deref() == Some("conv-outside")
                && session.external.is_some()
        }) && sessions
            .iter()
            .any(|session| session.agent_session_id.as_deref() == Some("conv-unknown"))
    });
    let sessions = env.all_sessions();
    let linked = sessions
        .iter()
        .find(|session| session.agent_session_id.as_deref() == Some("conv-outside"))
        .unwrap();
    let external = linked.external.as_ref().unwrap();
    assert_eq!(external.pid, Some(1));
    assert_eq!(external.status, Some(Status::Working));
    let unlinked = sessions
        .iter()
        .find(|session| {
            session.origin == hq_protocol::Origin::External
                && session.agent_session_id.as_deref() == Some("conv-unknown")
        })
        .unwrap();
    assert_eq!(
        unlinked.external.as_ref().unwrap().status,
        Some(Status::NeedsYou)
    );

    match env.connect().request(&Request::Resume {
        session: linked.id.clone(),
        term: None,
    }) {
        Response::Error { code, message } => {
            assert_eq!(code, ErrorCode::ExternalLive);
            assert!(message.contains("pid 1"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    let everything = String::from_utf8_lossy(&env.hq(&["ls", "--all"]).stdout).into_owned();
    assert!(everything.contains("working · external 1"), "{everything}");
    assert!(
        everything.contains("needs you · external 1"),
        "its own line keeps the outside status: {everything}"
    );
    assert!(
        !everything.contains("1 needs you"),
        "it is not counted as needing the owner here: {everything}"
    );

    // Only an outside session with a known conversation can join the sidebar.
    let refused = env.hq(&["add", &unlinked.id]);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("no known conversation"),
        "{refused:?}"
    );
    assert!(env.hq(&["add", &linked.id]).status.success());
    let listed = String::from_utf8_lossy(&env.hq(&["ls"]).stdout).into_owned();
    assert!(listed.contains("working · external 1"), "{listed}");
    assert!(!listed.contains("needs you"), "{listed}");
}

#[test]
fn clients_hear_outside_agents_come_and_go() {
    let mut env = TestEnv::new("outgo");
    env.stub("claude", CLAUDE_OUTSIDE_COMING_AND_GOING);
    let mut daemon = env.command();
    daemon.env("WORK", env.work());
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    let mut app = env.connect();
    assert!(matches!(
        app.request(&Request::Subscribe { usage: false }),
        Response::Done
    ));
    let flag = env.root.join("stub-args.txt.outside");
    let entry = |session: &SessionInfo| {
        session.origin == hq_protocol::Origin::External
            && session.agent_session_id.as_deref() == Some("conv-unknown")
    };

    // Started after the app listed the sessions: reported without a new list.
    std::fs::write(&flag, "").unwrap();
    let appeared = loop {
        match app.next_response() {
            Response::SessionChanged { session } if entry(&session) => break session,
            _ => continue,
        }
    };
    assert!(appeared.offered);

    // Exited: reported once more as no longer offered.
    std::fs::remove_file(&flag).unwrap();
    let gone = loop {
        match app.next_response() {
            Response::SessionChanged { session } if entry(&session) => break session,
            _ => continue,
        }
    };
    assert_eq!(gone.id, appeared.id);
    assert!(!gone.offered);
}

#[test]
fn a_sessions_shell_runs_beside_it_and_ends_with_it() {
    let mut env = TestEnv::new("shell");
    env.stub("claude", INTERACTIVE_STUB);
    let mut daemon = env.command();
    env.daemons.push(
        daemon
            .env("SHELL", "/bin/sh")
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    let session = env.create(Agent::Claude);
    let attach_shell = |env: &TestEnv| {
        let mut shell = env.connect();
        match shell.request(&Request::AttachShell {
            session: session.id.clone(),
            cols: 80,
            rows: 24,
        }) {
            Response::Attached { session: info } => assert_eq!(info.id, session.id),
            other => panic!("unexpected attach-shell answer {other:?}"),
        }
        shell
    };
    let ended = |shell: &mut Conn| loop {
        if let Response::Ended { .. } = shell.next_response() {
            break;
        }
    };

    // A shell in the session's folder, beside its agent.
    let mut shell = attach_shell(&env);
    shell.send_frame(&Frame::Input(b"echo shell-says-$((6*7)); pwd\n".to_vec()));
    let seen = shell.output_until("shell-says-42");
    let folder = env
        .work()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    if !seen.contains(&folder) {
        shell.output_until(&folder);
    }
    assert_eq!(
        env.session(&session.id).status,
        Status::Done,
        "the shell does not change the session's status"
    );

    // Attaching again reaches the same shell.
    shell.send_frame(&Frame::Input(b"export MARK=kept\n".to_vec()));
    drop(shell);
    let mut again = attach_shell(&env);
    again.send_frame(&Frame::Input(b"echo mark-$MARK\n".to_vec()));
    again.output_until("mark-kept");

    // Closing the shell ends it.
    assert!(matches!(
        env.connect().request(&Request::CloseShell {
            session: session.id.clone()
        }),
        Response::Done
    ));
    ended(&mut again);

    // Taking the session out of the sidebar ends its shell too.
    let mut fresh = attach_shell(&env);
    fresh.send_frame(&Frame::Input(b"echo mark-${MARK:-none}\n".to_vec()));
    fresh.output_until("mark-none");
    assert!(matches!(
        env.connect().request(&Request::Remove {
            sessions: vec![session.id.clone()]
        }),
        Response::Done
    ));
    ended(&mut fresh);
}

impl TestEnv {
    fn settings(&self, text: &str) {
        let path = self.root.join("config/headquarter/config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// Starts the host with only system folders on its `PATH`, as a desktop launcher
    /// would.
    fn start_daemon_without_path(&mut self) {
        let child = self
            .command()
            .env("PATH", "/usr/bin:/bin")
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        self.daemons.push(child);
        let socket = self.host_dir().join("agentd.sock");
        wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    }

    fn try_create(&self, agent: Agent) -> Response {
        self.connect().request(&Request::New {
            agent,
            cwd: self.work().to_string_lossy().into_owned(),
            args: vec![],
            term: None,
            name: None,
            group: None,
        })
    }

    fn home_executable(&self, relative: &str, script: &str) {
        let path = self.root.join("home").join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("#!/bin/sh\n{script}")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn agents_are_found_without_the_shell_path() {
    let mut env = TestEnv::new("discover");
    env.home_executable(".local/bin/agy", INTERACTIVE_STUB);
    env.start_daemon_without_path();
    assert!(matches!(
        env.try_create(Agent::Agy),
        Response::Created { .. }
    ));

    if let Some(missing) = absent_agent(&[Agent::Agy]) {
        match env.try_create(missing) {
            Response::Error { code, message } => {
                assert_eq!(code, ErrorCode::AgentNotFound);
                assert!(message.contains("Searched:"), "{message}");
                assert!(message.contains("/usr/bin"), "{message}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[test]
fn settings_turn_agents_off_and_errors_keep_listing() {
    let mut env = TestEnv::new("settings");
    env.stub("agy", INTERACTIVE_STUB);
    env.settings("[agents.agy]\nenabled = false\n");
    env.start_daemon();
    match env.try_create(Agent::Agy) {
        Response::Error { code, message } => {
            assert_eq!(code, ErrorCode::AgentDisabled);
            assert!(message.contains("turned off"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }

    env.settings("[discovery]\nlogin_shell = 0\n");
    match env.try_create(Agent::Agy) {
        Response::Error { code, message } => {
            assert_eq!(code, ErrorCode::Settings);
            assert!(message.contains("config.toml:2:"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(env.sessions().is_empty(), "listing still works");
    let listed = env.hq(&["ls"]);
    assert!(listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stderr).contains("settings error"));

    // Fixed while the host runs.
    env.settings("[discovery]\nlogin_shell = false\n");
    assert!(matches!(
        env.try_create(Agent::Agy),
        Response::Created { .. }
    ));
}

#[test]
fn agents_report_and_retired_window_setting() {
    let mut env = TestEnv::new("report");
    env.home_executable(
        ".local/bin/claude",
        "if [ \"$1\" = --version ]; then echo '9.9.9 (Claude Code)'; fi\n",
    );
    let output = env
        .command()
        .env("PATH", "/usr/bin:/bin")
        .args(["agents", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let agent = |id: &str| {
        report["agents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|agent| agent["id"] == id)
            .unwrap()
            .clone()
    };
    let claude = agent("claude");
    assert_eq!(claude["installed"], true);
    assert_eq!(claude["hostable"], true);
    assert_eq!(claude["version"], "9.9.9 (Claude Code)");
    assert!(
        claude["used"]
            .as_str()
            .unwrap()
            .ends_with("home/.local/bin/claude")
    );
    assert_eq!(agent("opencode")["installed"], false);
    assert_eq!(report["login_shell_path"], "unavailable");
    assert!(report["platform"]["os"].is_string());

    // The old display window loads with a warning and no longer hides anything.
    env.settings("[sidebar]\nhistory_days = 7\n");
    let transcript = env.claude_transcript("conv-tenday", "Ten days ago");
    TestEnv::age_file(&transcript, 10);
    env.start_daemon();
    wait_until("history imported", || env.sessions().len() == 1);
    let listed = env.hq(&["ls", "--all"]);
    assert!(listed.status.success(), "{listed:?}");
    assert!(String::from_utf8_lossy(&listed.stdout).contains("Ten days ago"));
    let warning = String::from_utf8_lossy(&listed.stderr).into_owned();
    assert!(warning.contains("config.toml:2:"), "{warning}");
    assert!(warning.contains("no longer has an effect"), "{warning}");
}

/// Sidebar membership and the owner's groups: kept by the host across restarts,
/// validated, and announced to subscribers.
#[test]
fn sidebar_membership_and_groups() {
    let mut env = TestEnv::new("sidebar");
    env.stub("agy", INTERACTIVE_STUB);
    env.claude_transcript("conv-past", "Past work");
    env.start_daemon();
    wait_until("history imported", || env.sessions().len() == 1);
    let past = env.sessions().remove(0);
    assert!(!past.in_sidebar, "history waits to be added");

    let hosted = env.create(Agent::Agy);
    assert!(hosted.in_sidebar, "sessions started here join on their own");

    let mut subscriber = env.connect();
    assert!(matches!(
        subscriber.request(&Request::Subscribe { usage: false }),
        Response::Done
    ));

    // One unknown reference: nothing changes, and the error names it.
    let refused = env.hq(&["add", "claude-convpast", "no-such-session"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("no-such-session"));
    assert!(!env.session(&past.id).in_sidebar);

    // Several at once, including one already in the sidebar.
    assert!(
        env.hq(&["add", "claude-convpast", "agy-1"])
            .status
            .success()
    );
    assert!(env.session(&past.id).in_sidebar);
    let changed = loop {
        match subscriber.next_response() {
            Response::SessionChanged { session } if session.id == past.id => break session,
            _ => continue,
        }
    };
    assert!(changed.in_sidebar);

    // Groups: default names, validation, order, collapse, membership, announcements.
    let first = match env.connect().request(&Request::NewGroup {
        name: None,
        session: Some(past.id.clone()),
    }) {
        Response::GroupCreated { group } => group,
        other => panic!("unexpected new group answer {other:?}"),
    };
    assert_eq!(first.name, "New Group #1");
    assert_eq!(env.session(&past.id).group, Some(first.id));
    let announced = loop {
        match subscriber.next_response() {
            Response::GroupsChanged { groups } => break groups,
            _ => continue,
        }
    };
    assert_eq!(announced, std::slice::from_ref(&first));
    match env.connect().request(&Request::NewGroup {
        name: Some("x".repeat(61)),
        session: None,
    }) {
        Response::Error { code, message } => {
            assert_eq!(code, ErrorCode::BadRequest);
            assert!(message.contains("at most 60"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    let docs = match env.connect().request(&Request::NewGroup {
        name: None,
        session: None,
    }) {
        Response::GroupCreated { group } => group,
        other => panic!("unexpected new group answer {other:?}"),
    };
    assert_eq!(docs.name, "New Group #2");
    for request in [
        Request::RenameGroup {
            group: docs.id,
            name: "  Docs ".into(),
        },
        Request::MoveGroup {
            group: docs.id,
            before: Some(first.id),
        },
        Request::CollapseGroup {
            group: first.id,
            collapsed: true,
        },
        Request::SetGroupColor {
            group: docs.id,
            color: Some("green".into()),
        },
        Request::SetGroupColor {
            group: first.id,
            color: None,
        },
        Request::SetGroup {
            session: hosted.id.clone(),
            group: Some(docs.id),
        },
    ] {
        assert!(
            matches!(env.connect().request(&request), Response::Done),
            "{request:?}"
        );
    }
    let known = env.all_sessions().len();
    for request in [
        Request::RenameGroup {
            group: 999,
            name: "Gone".into(),
        },
        Request::SetGroup {
            session: hosted.id.clone(),
            group: Some(999),
        },
        Request::New {
            agent: Agent::Agy,
            cwd: env.work().to_string_lossy().into_owned(),
            args: vec![],
            term: None,
            name: None,
            group: Some(999),
        },
    ] {
        match env.connect().request(&request) {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::NotFound, "{request:?}"),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(
        env.all_sessions().len(),
        known,
        "the refused start recorded nothing"
    );
    match env.connect().request(&Request::SetGroupColor {
        group: docs.id,
        color: Some("ultraviolet".into()),
    }) {
        Response::Error { code, message } => {
            assert_eq!(code, ErrorCode::BadRequest);
            assert!(message.contains("ultraviolet"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    let listed = match env.connect().request(&Request::List {}) {
        Response::Sessions { groups, .. } => groups,
        other => panic!("unexpected {other:?}"),
    };
    let colors: Vec<(i64, Option<String>)> = listed
        .iter()
        .map(|group| (group.id, group.color.clone()))
        .collect();
    assert!(
        colors.contains(&(docs.id, Some("green".into()))) && colors.contains(&(first.id, None)),
        "picked and automatic colours are kept: {colors:?}"
    );

    // Seen clears "done, not read yet" and nothing else.
    let mut viewer = env.connect();
    assert!(matches!(
        viewer.request(&Request::Attach {
            session: hosted.id.clone(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    viewer.output_until("stub-ready");
    env.wait_status(&hosted.id, Status::NeedsYou);
    assert!(matches!(
        env.connect().request(&Request::Seen {
            session: hosted.id.clone()
        }),
        Response::Done
    ));
    assert_eq!(env.session(&hosted.id).status, Status::NeedsYou);
    viewer.send_frame(&Frame::Input(b"yes\n".to_vec()));
    env.wait_status(&hosted.id, Status::DoneUnread);
    let finished_at = env.session(&hosted.id).last_activity_at;
    std::thread::sleep(Duration::from_millis(20));
    assert!(matches!(
        env.connect().request(&Request::Seen {
            session: hosted.id.clone()
        }),
        Response::Done
    ));
    env.wait_status(&hosted.id, Status::Done);
    assert_eq!(
        env.session(&hosted.id).last_activity_at,
        finished_at,
        "reading a finished session keeps its age"
    );

    // Membership and folders survive a restart; removing keeps the session.
    drop(viewer);
    assert!(env.hq(&["stop", "--yes"]).status.success());
    wait_until("host gone", || {
        UnixStream::connect(env.host_dir().join("agentd.sock")).is_err()
    });
    env.start_daemon();
    assert!(env.session(&past.id).in_sidebar);
    assert!(env.session(&hosted.id).in_sidebar);
    match env.connect().request(&Request::List {}) {
        Response::Sessions { groups, .. } => {
            let kept: Vec<(&str, bool)> = groups
                .iter()
                .map(|group| (group.name.as_str(), group.collapsed))
                .collect();
            assert_eq!(kept, [("Docs", false), ("New Group #1", true)]);
        }
        other => panic!("unexpected list answer {other:?}"),
    }
    assert_eq!(env.session(&hosted.id).group, Some(docs.id));
    let listed = String::from_utf8_lossy(&env.hq(&["ls"]).stdout).into_owned();
    let docs_at = listed.find("Docs  1").expect(&listed);
    let first_at = listed.find("New Group #1  1  (collapsed)").expect(&listed);
    assert!(docs_at < first_at, "{listed}");

    assert!(matches!(
        env.connect()
            .request(&Request::DeleteGroup { group: docs.id }),
        Response::Done
    ));
    let ungrouped = env.session(&hosted.id);
    assert_eq!(
        ungrouped.group, None,
        "deleting a group ungroups its sessions"
    );
    assert!(ungrouped.in_sidebar);
    assert!(env.hq(&["remove", "claude-convpast"]).status.success());
    assert!(!env.session(&past.id).in_sidebar);
    assert_eq!(
        env.session(&past.id).group,
        None,
        "leaving the sidebar leaves the group"
    );
    let everything = String::from_utf8_lossy(&env.hq(&["ls", "--all"]).stdout).into_owned();
    assert!(everything.contains("  claude-convpast"), "{everything}");
    assert!(everything.contains("* agy-1"), "{everything}");
}

/// A hostable agent not installed in any system-wide folder discovery searches, so a
/// "not found" check never starts a real agent. Folders under the owner's home do not
/// matter: tests run with their own `HOME`. `None` (a machine with every agent
/// installed system-wide) skips the check.
fn absent_agent(exclude: &[Agent]) -> Option<Agent> {
    const SYSTEM_FOLDERS: &[&str] = &[
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/opt/homebrew/bin",
        "/home/linuxbrew/.linuxbrew/bin",
        "/snap/bin",
        "/var/lib/flatpak/exports/bin",
    ];
    let absent = [Agent::Codex, Agent::Claude, Agent::Agy]
        .into_iter()
        .filter(|agent| !exclude.contains(agent))
        .find(|agent| {
            SYSTEM_FOLDERS
                .iter()
                .all(|folder| !Path::new(folder).join(agent.binary()).exists())
        });
    if absent.is_none() {
        eprintln!("every agent is installed system-wide here; skipping the not-found check");
    }
    absent
}

/// Codex usage is read on request and while a client shows usage, never otherwise;
/// a failed read keeps the last numbers with the reason.
/// Claude Code that answers `/usage` with a screen like 2.1.306's, and asks the folder
/// trust question instead while `$STUB_ARGS.trust` exists.
const CLAUDE_USAGE_SCREEN_STUB: &str = r#"case "$1 $2" in
  "auth status") echo '{"loggedIn":true,"authMethod":"claude.ai","subscriptionType":"max"}'; exit 0 ;;
  "agents "*) echo '[]'; exit 0 ;;
esac
printf '%s\n' "$@" >> "$STUB_ARGS"
if [ -e "$STUB_ARGS.trust" ]; then
  printf 'Quick safety check\r\nYes, I trust this folder\r\n'
  sleep 30
  exit 0
fi
printf '\033]7501;state=idle\007'
while read line; do
  case "$line" in
    */usage*)
      printf 'Current session\r\n\342\226\210\342\226\210 15%%used\r\nResets 11:30am (Asia/Jakarta)\r\n'
      printf 'Current week (all models)\r\n\342\226\210\342\226\210 26%%used\r\nResets Oct 24 at 3pm (Asia/Jakarta)\r\n'
      printf 'Current week (Fable)\r\n\342\226\210 3%%used\r\n'
      sleep 30 ;;
  esac
done
"#;

/// Starts a host with a short usage interval and one Claude session, so a folder Claude
/// was used in is known, then watches usage until Claude's changes as `done` says.
fn watch_claude_usage(
    env: &mut TestEnv,
    done: impl Fn(&hq_protocol::AgentUsage) -> bool,
) -> hq_protocol::AgentUsage {
    // Codex and agy usage are read too: without stubs the host would find and start
    // the machine's own, with this test's home.
    env.stub("codex", "exit 1");
    env.stub("agy", "exit 1");
    let mut daemon = env.command();
    daemon.env("HQ_USAGE_INTERVAL_SECS", "1");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    env.create(Agent::Claude);
    let mut watcher = env.connect();
    assert!(matches!(
        watcher.request(&Request::Subscribe { usage: true }),
        Response::Done
    ));
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(
            Instant::now() < deadline,
            "no such Claude usage: {}",
            env.log()
        );
        if let Response::UsageChanged { agents } = watcher.next_response()
            && let Some(claude) = agents.iter().find(|usage| usage.agent == Agent::Claude)
            && done(claude)
        {
            return claude.clone();
        }
    }
}

#[test]
fn claude_usage_comes_from_its_own_usage_screen() {
    let mut env = TestEnv::new("usagescreen");
    env.stub("claude", CLAUDE_USAGE_SCREEN_STUB);
    let claude = watch_claude_usage(&mut env, |usage| !usage.windows.is_empty());
    let used: Vec<(Option<u64>, f64, Option<&str>)> = claude
        .windows
        .iter()
        .map(|window| (window.minutes, window.used_percent, window.model.as_deref()))
        .collect();
    assert_eq!(
        used,
        [
            (Some(300), 15.0, None),
            (Some(10_080), 26.0, None),
            (Some(10_080), 3.0, Some("Fable"))
        ]
    );
    assert_eq!(claude.problem, None);
    let args = std::fs::read_to_string(env.root.join("stub-args.txt")).unwrap();
    assert!(
        args.contains("--setting-sources\nproject,local\n"),
        "the owner's user settings stay out: {args}"
    );
    assert!(env.log().contains("source=usage-screen ok=true"));
}

#[test]
fn a_folder_trust_question_stops_the_usage_read_unanswered() {
    let mut env = TestEnv::new("usagetrust");
    env.stub("claude", CLAUDE_USAGE_SCREEN_STUB);
    std::fs::write(env.root.join("stub-args.txt.trust"), "").unwrap();
    let claude = watch_claude_usage(&mut env, |usage| {
        usage
            .problem
            .as_deref()
            .is_some_and(|problem| problem.contains("trust"))
    });
    assert!(claude.windows.is_empty());
    assert!(env.log().contains("source=usage-screen ok=false"));
}

#[test]
fn a_codex_turn_ending_reads_codex_usage_again() {
    let mut env = TestEnv::new("codexturn");
    env.stub("agy", "exit 1");
    env.stub(
        "codex",
        &CODEX_USAGE_STUB.replace(
            r#"[ "$1" = app-server ] || { sleep 30; exit 0; }"#,
            r#"if [ "$1" != app-server ]; then
  printf 'stub-ready\n'
  read line
  printf '\033]7501;state=working\007'
  sleep 0.3
  printf '\033]7501;state=idle\007'
  sleep 30
  exit 0
fi"#,
        ),
    );
    let starts = |env: &TestEnv| {
        std::fs::read_to_string(env.root.join("stub-args.txt")).map_or(0, |text| {
            text.lines().filter(|line| *line == "started").count()
        })
    };
    // A long interval, so only the turn's end can start the second read.
    let mut daemon = env.command();
    daemon.env("HQ_USAGE_INTERVAL_SECS", "600");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    let session = env.create(Agent::Codex);
    let mut watcher = env.connect();
    assert!(matches!(
        watcher.request(&Request::Subscribe { usage: true }),
        Response::Done
    ));
    wait_until("first read", || starts(&env) == 1);
    let mut viewer = env.connect();
    assert!(matches!(
        viewer.request(&Request::Attach {
            session: session.id.clone(),
            cols: 80,
            rows: 24
        }),
        Response::Attached { .. }
    ));
    viewer.output_until("stub-ready");
    viewer.send_frame(&Frame::Input(b"go\n".to_vec()));
    let deadline = Instant::now() + Duration::from_secs(20);
    while starts(&env) < 2 {
        assert!(
            Instant::now() < deadline,
            "no read after the turn: {}",
            env.log()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// agy's own headless `/usage`, read on refresh in a folder of Headquarter's, gives a
/// window per pool of models.
#[test]
fn agy_usage_comes_from_its_own_usage_command() {
    let mut env = TestEnv::new("agyusage");
    env.stub(
        "agy",
        r#"[ "$1 $2 $3 $4" = "-p /usage --output-format json" ] || { sleep 30; exit 0; }
pwd >> "$STUB_ARGS"
echo '{"conversation_id":"","status":"SUCCESS","num_turns":0,"command":{"name":"usage","data":{"groups":[{"name":"Gemini Models","buckets":[{"id":"gemini-5h","window":"5h","remaining_fraction":0.58,"reset_time":"2100-01-01T00:00:00Z"},{"id":"gemini-weekly","window":"weekly","remaining_fraction":0.9,"reset_time":"2100-01-02T00:00:00Z"}]}]}}}'
"#,
    );
    env.stub("codex", "exit 1");
    env.stub("claude", "exit 1");
    let mut daemon = env.command();
    daemon.env("HQ_USAGE_INTERVAL_SECS", "1");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());

    let mut conn = env.connect();
    assert!(matches!(
        conn.request(&Request::RefreshUsage {}),
        Response::Done
    ));
    let agy = match conn.request(&Request::Usage {}) {
        Response::Usage { agents } => agents
            .into_iter()
            .find(|usage| usage.agent == Agent::Agy)
            .expect("agy is listed"),
        other => panic!("unexpected answer {other:?}"),
    };
    let windows: Vec<(Option<u64>, i64, Option<&str>)> = agy
        .windows
        .iter()
        .map(|window| {
            (
                window.minutes,
                window.used_percent.round() as i64,
                window.model.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        windows,
        vec![
            (Some(300), 42, Some("Gemini")),
            (Some(10_080), 10, Some("Gemini"))
        ]
    );
    assert_eq!(agy.problem, None);
    let ran_in = std::fs::read_to_string(env.root.join("stub-args.txt")).unwrap();
    assert!(
        ran_in.trim_end().ends_with("quota/agy-workspace"),
        "agy runs in Headquarter's own folder: {ran_in}"
    );
}

/// Refresh answers once the readings are taken, so the details can show "Refreshing"
/// until then and the new numbers right after.
#[test]
fn refresh_answers_after_the_new_reading() {
    let mut env = TestEnv::new("refresh");
    env.stub("agy", "exit 1");
    // Codex takes a second to answer, with the percentage the test sets.
    env.stub(
        "codex",
        &CODEX_USAGE_STUB.replace(
            r#"*rateLimits*) echo '{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":42,"#,
            r#"*rateLimits*) sleep 1; echo '{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":'"$(cat "$STUB_ARGS.percent" 2>/dev/null || echo 42)"',"#,
        ),
    );
    env.stub("claude", "exit 1");
    let mut daemon = env.command();
    daemon.env("HQ_USAGE_INTERVAL_SECS", "1");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());
    let codex_percent = |conn: &mut Conn| match conn.request(&Request::Usage {}) {
        Response::Usage { agents } => agents
            .iter()
            .find(|usage| usage.agent == Agent::Codex)
            .and_then(|usage| usage.windows.first())
            .map(|window| window.used_percent),
        other => panic!("unexpected answer {other:?}"),
    };

    let mut conn = env.connect();
    assert_eq!(codex_percent(&mut conn), Some(42.0));
    std::fs::write(env.root.join("stub-args.txt.percent"), "77").unwrap();
    // Past the refresh floor, which follows the one-second interval here.
    std::thread::sleep(Duration::from_millis(1_100));
    assert!(matches!(
        conn.request(&Request::RefreshUsage {}),
        Response::Done
    ));
    assert_eq!(codex_percent(&mut conn), Some(77.0));
}

#[test]
fn usage_is_read_only_when_someone_looks() {
    let mut env = TestEnv::new("usage");
    env.stub("codex", CODEX_USAGE_STUB);
    env.stub("agy", "exit 1");
    env.stub(
        "claude",
        r#"case "$1 $2" in
  "auth status") echo '{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","email":"someone@example.com","subscriptionType":"max"}' ;;
  "agents "*) echo '[]' ;;
esac
"#,
    );
    let starts = |env: &TestEnv| {
        std::fs::read_to_string(env.root.join("stub-args.txt")).map_or(0, |text| {
            text.lines().filter(|line| *line == "started").count()
        })
    };
    let mut daemon = env.command();
    daemon.env("HQ_USAGE_INTERVAL_SECS", "1");
    env.daemons.push(
        daemon
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = env.host_dir().join("agentd.sock");
    wait_until("host socket", || UnixStream::connect(&socket).is_ok());

    // A subscriber that did not ask for usage starts nothing.
    let mut plain = env.connect();
    assert!(matches!(
        plain.request(&Request::Subscribe { usage: false }),
        Response::Done
    ));
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(starts(&env), 0, "nobody looks at usage");

    let listed = env.hq(&["usage"]);
    assert!(listed.status.success(), "{listed:?}");
    let text = String::from_utf8_lossy(&listed.stdout).into_owned();
    assert!(text.contains("codex   5h     42%"), "{text}");
    assert!(text.contains("claude  not available"), "{text}");
    assert!(text.contains("        Plus plan\n"), "{text}");
    assert!(!text.contains("as of"), "no reading age: {text}");
    assert!(text.contains("        Max plan\n"), "{text}");
    assert!(!text.contains("someone@example.com"), "{text}");
    assert!(text.contains("agy     not available"), "{text}");
    assert_eq!(starts(&env), 1);

    let mut watcher = env.connect();
    assert!(matches!(
        watcher.request(&Request::Subscribe { usage: true }),
        Response::Done
    ));
    let codex = |agents: &[hq_protocol::AgentUsage]| {
        agents
            .iter()
            .find(|usage| usage.agent == Agent::Codex)
            .cloned()
            .unwrap()
    };
    // Claude's hidden read also announces usage; wait for Codex's second read.
    let read = loop {
        if let Response::UsageChanged { agents } = watcher.next_response()
            && starts(&env) >= 2
        {
            break codex(&agents);
        }
    };
    assert_eq!(read.windows[0].used_percent, 42.0);
    assert_eq!(
        read.account,
        Some(hq_protocol::Account::Subscription {
            plan: Some("plus".into())
        })
    );
    assert!(starts(&env) >= 2, "read again while watched");

    std::fs::write(env.root.join("stub-args.txt.fail"), "").unwrap();
    let failed = loop {
        if let Response::UsageChanged { agents } = watcher.next_response() {
            let usage = codex(&agents);
            if usage.problem.is_some() {
                break usage;
            }
        }
    };
    assert_eq!(
        failed.windows[0].used_percent, 42.0,
        "the last numbers stay"
    );
    assert!(failed.read_at.is_some());

    watcher.close();
    // At most a read already under way finishes after the client leaves.
    std::thread::sleep(Duration::from_millis(2_500));
    let settled = starts(&env);
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(starts(&env), settled, "no reads once nobody watches");
    let mut plain_saw_usage = false;
    while let Ok(frame) = plain.frames.try_recv() {
        if let Frame::Control(bytes) = frame {
            plain_saw_usage |= matches!(Response::parse(&bytes), Ok(Response::UsageChanged { .. }));
        }
    }
    assert!(!plain_saw_usage, "usage goes only to those who asked");
}
