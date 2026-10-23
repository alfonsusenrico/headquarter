//! `hq` (also installed as `headquarter`): start, list, attach to, and resume
//! coding-agent sessions hosted by the Headquarter host. Both executables call
//! [`main`].

mod agents;
mod attach;
mod render;

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use hq_daemon::paths::HostPaths;
use hq_protocol::{GroupInfo, Request, Response, SessionInfo};
use hq_signals::event::Agent;

use crate::attach::Outcome;
use hq_client::{Client, Greeting, expect_ok};

/// Idle time before the host exits with no sessions and no clients.
const DEFAULT_IDLE_EXIT: Duration = Duration::from_secs(10 * 60);
/// How often Codex usage is read while the app shows usage (design D1).
const DEFAULT_USAGE_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// Spec: activity outside Headquarter shows within a minute.
const DEFAULT_HISTORY_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Parser)]
#[command(
    name = "hq",
    version,
    about = "Headquarter: one home for your coding-agent sessions"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a new agent session (claude, codex, or agy) and attach to it.
    New {
        agent: String,
        /// Working folder (default: the current folder).
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        name: Option<String>,
        /// Start without attaching.
        #[arg(long, short)]
        detach: bool,
        /// Extra arguments for the agent, after `--`.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// List the sidebar's sessions: those needing you first, then by folder.
    Ls {
        #[arg(long)]
        json: bool,
        /// List every session the host knows, marking those in the sidebar with `*`.
        #[arg(long)]
        all: bool,
    },
    /// Put sessions in the sidebar (names or id prefixes, as `hq ls --all` shows).
    Add {
        #[arg(required = true)]
        sessions: Vec<String>,
    },
    /// Take sessions out of the sidebar; the agents' history is untouched.
    Remove {
        #[arg(required = true)]
        sessions: Vec<String>,
    },
    /// Show each agent's usage windows: share used, reset time, and reading age.
    Usage {
        #[arg(long)]
        json: bool,
    },
    /// Show this machine's platform and the agent CLIs installed on it.
    Agents {
        #[arg(long)]
        json: bool,
    },
    /// Attach to a running session (detach with Ctrl-\ then d).
    Attach { session: String },
    /// Restart a stopped session's conversation and attach to it; with
    /// `--interrupted`, restart every sidebar session that was running when Headquarter
    /// stopped, without attaching.
    Resume {
        #[arg(required_unless_present = "interrupted")]
        session: Option<String>,
        #[arg(long, short)]
        detach: bool,
        #[arg(long, conflicts_with_all = ["session", "detach"])]
        interrupted: bool,
    },
    /// Rename a session.
    Rename { session: String, name: String },
    /// Close a session: stop its agent with the agent's own exit command and take it
    /// out of the sidebar.
    Close { session: String },
    /// Hang up a running session's agent at once; it stays in the sidebar.
    Kill { session: String },
    /// End every session and stop the host.
    Stop {
        /// Do not ask for confirmation.
        #[arg(long)]
        yes: bool,
    },
    /// Run the host in the foreground (other commands start it automatically).
    Daemon,
    #[command(hide = true)]
    HookForward {
        #[arg(long)]
        socket: PathBuf,
        #[arg(long)]
        agent: String,
        payload: Option<String>,
    },
    #[command(hide = true)]
    HookStatusline {
        #[arg(long)]
        chain: Option<String>,
        /// The host's hooks socket, for the session's details.
        #[arg(long)]
        socket: Option<PathBuf>,
    },
}

pub fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("hq: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::HookForward {
            socket,
            agent,
            payload,
        } => {
            hq_signals::helpers::hook_forward(&socket, &agent, payload);
            return Ok(ExitCode::SUCCESS);
        }
        Command::HookStatusline { chain, socket } => {
            return Ok(ExitCode::from(hq_signals::helpers::hook_status_line(
                chain.as_deref(),
                socket.as_deref(),
            )));
        }
        Command::Daemon => return daemon(),
        Command::Agents { json } => {
            let discovered = agents::discover();
            if json {
                println!("{}", agents::render_json(&discovered));
            } else {
                print!("{}", agents::render_text(&discovered));
            }
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    let paths = HostPaths::standard().context("cannot find the home folder")?;
    match command {
        Command::New {
            agent,
            cwd,
            name,
            detach,
            args,
        } => {
            let agent = Agent::from_id(&agent).with_context(|| {
                format!("unknown agent `{agent}` (expected claude, codex, or agy)")
            })?;
            let cwd = match cwd {
                Some(cwd) => cwd,
                None => std::env::current_dir()?,
            };
            let mut client = Client::connect_or_start(&paths)?;
            let response = expect_ok(client.request(&Request::New {
                agent,
                cwd: cwd.to_string_lossy().into_owned(),
                args,
                term: std::env::var("TERM").ok(),
                name,
                group: None,
            })?)?;
            let Response::Created { session } = response else {
                bail!("unexpected answer from the host");
            };
            open(client, &session, detach)
        }
        Command::Ls { json, all } => {
            warn_about_settings();
            let mut client = Client::connect_or_start(&paths)?;
            print_sessions(&mut client, json, all)
        }
        Command::Usage { json } => {
            let agents = hq_client::usage(&paths)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&agents)?);
            } else {
                print!("{}", render::usage_list(&agents, &chrono::Local::now()));
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Add { sessions } => {
            hq_client::add_to_sidebar(&paths, &sessions)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Remove { sessions } => {
            hq_client::remove_from_sidebar(&paths, &sessions)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Attach { session } => {
            let mut client = Client::connect_or_start(&paths)?;
            let info = find(&mut client, &session)?;
            open(client, &info, false)
        }
        Command::Resume {
            interrupted: true, ..
        } => resume_interrupted(&paths),
        Command::Resume {
            session, detach, ..
        } => {
            let Some(session) = session else {
                bail!("name a session to resume, or pass --interrupted");
            };
            let mut client = Client::connect_or_start(&paths)?;
            let response = expect_ok(client.request(&Request::Resume {
                session,
                term: std::env::var("TERM").ok(),
            })?)?;
            let Response::Created { session } = response else {
                bail!("unexpected answer from the host");
            };
            open(client, &session, detach)
        }
        Command::Rename { session, name } => {
            let mut client = Client::connect_or_start(&paths)?;
            expect_ok(client.request(&Request::Rename { session, name })?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Close { session } => close(&paths, &session),
        Command::Kill { session } => {
            let mut client = Client::connect_or_start(&paths)?;
            expect_ok(client.request(&Request::Kill { session })?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Stop { yes } => stop(&paths, yes),
        Command::Daemon
        | Command::Agents { .. }
        | Command::HookForward { .. }
        | Command::HookStatusline { .. } => {
            unreachable!("handled above")
        }
    }
}

/// Reports a settings file problem on stderr; listing keeps working while the owner
/// fixes it.
fn warn_about_settings() {
    let Some(path) = hq_config::settings_path() else {
        return;
    };
    let ids: Vec<&str> = hq_discovery::KNOWN_AGENTS
        .iter()
        .map(|agent| agent.id)
        .collect();
    match hq_config::load(&path, &ids) {
        Ok(settings) => {
            for warning in &settings.warnings {
                eprintln!("hq: settings warning: {warning}");
            }
        }
        Err(error) => eprintln!("hq: settings error: {error}"),
    }
}

fn daemon() -> Result<ExitCode> {
    let paths = HostPaths::standard().context("cannot find the home folder")?;
    let seconds = |name: &str, default: Duration| {
        std::env::var(name)
            .ok()
            .and_then(|secs| secs.parse().ok())
            .map_or(default, Duration::from_secs)
    };
    let options = hq_daemon::HostOptions {
        paths,
        helper_exe: std::env::current_exe().context("locate the hq executable")?,
        idle_exit: seconds("HQ_IDLE_EXIT_SECS", DEFAULT_IDLE_EXIT),
        usage_interval: seconds("HQ_USAGE_INTERVAL_SECS", DEFAULT_USAGE_INTERVAL),
        history_interval: seconds("HQ_HISTORY_INTERVAL_SECS", DEFAULT_HISTORY_INTERVAL),
    };
    match hq_daemon::run(options) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(hq_daemon::HostError::AlreadyRunning) => {
            eprintln!("hq: the Headquarter host is already running");
            Ok(ExitCode::FAILURE)
        }
        Err(error) => Err(error.into()),
    }
}

/// Attaches to a new or resumed session, or reports it when not attaching.
fn open(client: Client, session: &SessionInfo, detach: bool) -> Result<ExitCode> {
    if detach || !std::io::stdin().is_terminal() {
        println!(
            "{} ({}) running in {}",
            session.name, session.agent, session.cwd
        );
        return Ok(ExitCode::SUCCESS);
    }
    match attach::attach(client, session)? {
        Outcome::Detached => {
            eprintln!("[detached from {}; it keeps running]", session.name);
        }
        Outcome::Ended(code) => {
            let code = code.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
            eprintln!("[{} exited, code {code}]", session.name);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn sessions(client: &mut Client) -> Result<Vec<SessionInfo>> {
    Ok(sessions_and_groups(client)?.0)
}

fn sessions_and_groups(client: &mut Client) -> Result<(Vec<SessionInfo>, Vec<GroupInfo>)> {
    match expect_ok(client.request(&Request::List {})?)? {
        Response::Sessions { sessions, groups } => Ok((sessions, groups)),
        _ => bail!("unexpected answer from the host"),
    }
}

/// Restarts every interrupted sidebar session without attaching and says how each went
/// (session-restore D5).
fn resume_interrupted(paths: &HostPaths) -> Result<ExitCode> {
    let mut client = Client::connect_or_start(paths)?;
    let sessions = sessions(&mut client)?;
    let interrupted: Vec<SessionInfo> = sessions
        .into_iter()
        .filter(|session| session.in_sidebar && session.interrupted)
        .collect();
    if interrupted.is_empty() {
        println!("No session was interrupted.");
        return Ok(ExitCode::SUCCESS);
    }
    let mut failed = false;
    for session in interrupted {
        let answer = client.request(&Request::Resume {
            session: session.id.clone(),
            term: std::env::var("TERM").ok(),
        })?;
        match expect_ok(answer) {
            Ok(_) => println!("{}: restarted", session.name),
            Err(error) => {
                failed = true;
                println!("{}: {error:#}", session.name);
            }
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Closing waits for the agent's exit command, then a hang-up, then a kill.
const CLOSE_WAIT: Duration = Duration::from_secs(20);

fn close(paths: &HostPaths, reference: &str) -> Result<ExitCode> {
    let mut client = Client::connect_or_start(paths)?;
    let session = find(&mut client, reference)?;
    expect_ok(client.request(&Request::Close {
        session: session.id.clone(),
    })?)?;
    let deadline = Instant::now() + CLOSE_WAIT;
    while find(&mut client, &session.id)?.live {
        if Instant::now() >= deadline {
            eprintln!("{} is still closing", session.name);
            return Ok(ExitCode::FAILURE);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    println!("Closed {}", session.name);
    Ok(ExitCode::SUCCESS)
}

/// Finds a session by name, id, or id prefix, the same way the host does.
fn find(client: &mut Client, reference: &str) -> Result<SessionInfo> {
    let all = sessions(client)?;
    let matches: Vec<&SessionInfo> = all
        .iter()
        .filter(|session| {
            session.id == reference
                || session.name == reference
                || (reference.len() >= 4 && session.id.starts_with(reference))
        })
        .collect();
    match matches.as_slice() {
        [only] => Ok((*only).clone()),
        [] => bail!("no session matches `{reference}`"),
        _ => bail!("`{reference}` matches several sessions; use more of the id"),
    }
}

fn print_sessions(client: &mut Client, json: bool, all: bool) -> Result<ExitCode> {
    let now = hq_signals::now_ms();
    let (mut listed, groups) = sessions_and_groups(client)?;
    if !all {
        listed.retain(|session| session.in_sidebar);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&listed)?);
    } else if all {
        print!("{}", render::catalog_list(&listed, &groups, now));
    } else {
        print!("{}", render::sidebar_list(&listed, &groups, now));
    }
    Ok(ExitCode::SUCCESS)
}

/// Whether a host process still holds the single-instance lock.
fn host_holds_lock(paths: &HostPaths) -> bool {
    let Ok(file) = std::fs::OpenOptions::new().write(true).open(&paths.lock) else {
        return false;
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(_) => true,
    }
}

fn stop(paths: &HostPaths, yes: bool) -> Result<ExitCode> {
    let mut client = match Client::greet(paths, hq_protocol::PROTOCOL_VERSION) {
        Ok(Greeting::Welcome(client)) => client,
        Ok(Greeting::OtherVersion {
            protocol: Some(protocol),
            ..
        }) => return stop_other_version(paths, protocol, yes),
        Ok(Greeting::OtherVersion {
            protocol: None,
            message,
        }) => bail!(message),
        Err(_) => {
            println!("The Headquarter host is not running.");
            return Ok(ExitCode::SUCCESS);
        }
    };
    let running = sessions(&mut client)?
        .iter()
        .filter(|session| session.live)
        .count();
    if !yes && running > 0 {
        if !std::io::stdin().is_terminal() {
            bail!("{running} sessions are running; pass --yes to stop them without asking");
        }
        print!("Stop the host and end {running} running sessions? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            println!("Nothing stopped.");
            return Ok(ExitCode::SUCCESS);
        }
    }
    expect_ok(client.request(&Request::Stop {})?)?;
    wait_until_stopped(paths)?;
    println!("Stopped the host and {running} sessions.");
    Ok(ExitCode::SUCCESS)
}

/// Stops a host left running by another Headquarter version, as after an upgrade. Its
/// sessions cannot be counted in its protocol, so the question says they all end.
fn stop_other_version(paths: &HostPaths, protocol: u32, yes: bool) -> Result<ExitCode> {
    if !yes {
        if !std::io::stdin().is_terminal() {
            bail!(
                "a host from another Headquarter version (protocol {protocol}) is running; pass --yes to stop it and end its sessions"
            );
        }
        print!(
            "A host from another Headquarter version (protocol {protocol}) is running. Stop it and end its sessions? [y/N] "
        );
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            println!("Nothing stopped.");
            return Ok(ExitCode::SUCCESS);
        }
    }
    hq_client::stop_other_version(paths, protocol)?;
    wait_until_stopped(paths)?;
    println!("Stopped the host from the other version (protocol {protocol}) and its sessions.");
    Ok(ExitCode::SUCCESS)
}

/// The host answers before its agents have exited; report only once it is gone.
fn wait_until_stopped(paths: &HostPaths) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while host_holds_lock(paths) {
        if std::time::Instant::now() >= deadline {
            bail!("the host did not stop within 10 seconds");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}
