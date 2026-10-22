//! `hq-probe`: proves agent status, quota, and external-session signals before the
//! Headquarter GUI exists. See `openspec/changes/add-status-probe/`.

mod report;
mod run;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hq_signals::event::Agent;

#[derive(Parser)]
#[command(
    name = "hq-probe",
    version,
    about = "Headquarter status and quota probe"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Launch an agent in a pseudo-terminal and record its status changes.
    Run {
        /// claude, codex, or agy
        agent: String,
        /// Working folder for the agent (default: current folder).
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Event log path (default: ~/.local/share/headquarter/probe/runs/).
        #[arg(long)]
        log: Option<PathBuf>,
        /// Replace the agent executable; used by tests with a stub agent.
        #[arg(long, hide = true)]
        program: Option<String>,
        /// Extra arguments passed to the agent after the injected ones.
        #[arg(last = true)]
        extra: Vec<String>,
    },
    /// Hook helper: deliver one payload to a running probe. Always exits 0 silently.
    HookForward {
        #[arg(long)]
        socket: PathBuf,
        #[arg(long)]
        agent: String,
        /// Payload given as an argument; otherwise read from stdin.
        payload: Option<String>,
    },
    /// Claude Code status line wrapper: records rate limits, then runs the owner's
    /// own status line command.
    HookStatusline {
        #[arg(long)]
        chain: Option<String>,
    },
    /// Print each provider's usage windows.
    Quota,
    /// List agent sessions running outside the probe.
    Scan {
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            agent,
            cwd,
            log,
            program,
            extra,
        } => {
            let Some(agent) = Agent::from_id(&agent) else {
                eprintln!("hq-probe: unknown agent `{agent}` (expected claude, codex, or agy)");
                return ExitCode::from(2);
            };
            match run::run(run::RunOptions {
                agent,
                cwd,
                log,
                program,
                extra,
            }) {
                Ok(code) => code,
                Err(error) => {
                    eprintln!("hq-probe: {error:#}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::HookForward {
            socket,
            agent,
            payload,
        } => {
            hq_signals::helpers::hook_forward(&socket, &agent, payload);
            ExitCode::SUCCESS
        }
        Command::HookStatusline { chain } => ExitCode::from(hq_signals::helpers::hook_status_line(
            chain.as_deref(),
            None,
        )),
        Command::Quota => {
            report::quota();
            ExitCode::SUCCESS
        }
        Command::Scan { json } => report::scan(json),
    }
}
