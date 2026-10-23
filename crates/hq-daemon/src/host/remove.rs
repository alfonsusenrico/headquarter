//! Removing a past conversation from its agent, on the owner's request from Add sessions
//! (remove-conversations, owner 2026-10-23). The one place Headquarter changes an
//! agent's own data: Claude Code's transcript goes to the Trash, since Claude Code has
//! no delete command; Codex is changed only through its own `archive` and `delete`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hq_protocol::{ErrorCode, RemoveWay, removal_ways};
use hq_signals::event::Agent;
use hq_signals::paths;

use super::{Host, OpResult, fail, lock};

/// `codex archive` and `codex delete` answered in about a second on 2026-10-23.
const CODEX_TIMEOUT: Duration = Duration::from_secs(30);

impl Host {
    pub(super) fn remove_conversation(&self, reference: &str, way: RemoveWay) -> OpResult<()> {
        let row = self.resolve(reference)?;
        if lock(&self.live).contains_key(&row.id) {
            return Err(fail(
                ErrorCode::AlreadyLive,
                format!("session `{}` is running; close it first", row.name),
            ));
        }
        if lock(&self.outside).linked.contains_key(&row.id) {
            return Err(fail(
                ErrorCode::ExternalLive,
                format!("session `{}` is running in another terminal", row.name),
            ));
        }
        if !removal_ways(row.agent).contains(&way) {
            return Err(fail(
                ErrorCode::BadRequest,
                format!("{} conversations cannot be removed that way", row.agent),
            ));
        }
        let Some(conversation) = row.agent_session_id.clone() else {
            return Err(fail(
                ErrorCode::BadRequest,
                format!("session `{}` has no conversation yet", row.name),
            ));
        };
        let removed = match (row.agent, way) {
            (Agent::Claude, RemoveWay::Trash) => self.trash_claude(&conversation),
            (Agent::Codex, RemoveWay::Archive) => self.run_codex(&["archive", &conversation]),
            (Agent::Codex, RemoveWay::Delete) => {
                self.run_codex(&["delete", "--force", &conversation])
            }
            _ => Err("not a way this agent allows".to_owned()),
        };
        if let Err(reason) = removed {
            self.log.line(&format!(
                "conversation not removed agent={} way={way:?} reason={reason}",
                row.agent
            ));
            return Err(fail(
                ErrorCode::RemoveFailed,
                format!("{} could not remove the conversation", row.agent),
            ));
        }
        self.log.line(&format!(
            "conversation removed agent={} way={way:?}",
            row.agent
        ));
        // Offered again only if the next history pass still finds it.
        lock(&self.offered).remove(&(row.agent, conversation));
        self.broadcast(&row.id);
        Ok(())
    }

    /// Moves the transcript and the folder Claude Code keeps beside it (sub-agents,
    /// tool output) to the Trash.
    fn trash_claude(&self, conversation: &str) -> Result<(), String> {
        let projects = paths::claude_config_dir()
            .ok_or("the home folder is unknown")?
            .join("projects");
        let transcript = hq_signals::history::claude_transcript(&projects, conversation)
            .ok_or("no transcript for this conversation")?;
        let mut items = vec![transcript.clone()];
        let beside = transcript.with_extension("");
        if beside.is_dir() {
            items.push(beside);
        }
        match &self.trash_dir {
            Some(dir) => move_into(dir, &items),
            None => move_to_system_trash(&items),
        }
    }

    fn run_codex(&self, args: &[&str]) -> Result<(), String> {
        let codex = self
            .resolve_agent(Agent::Codex)
            .map_err(|error| super::usage::problem_text(*error))?;
        let mut child = Command::new(&codex)
            .args(args)
            .env("PATH", hq_discovery::shell::agent_path(&codex))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("cannot start codex: {error}"))?;
        let deadline = Instant::now() + CODEX_TIMEOUT;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => return Err(format!("codex {} ended with {status}", args[0])),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("codex {} did not finish in time", args[0]));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(error) => return Err(error.to_string()),
            }
        }
    }
}

/// The system Trash, through the file manager's own call on macOS, which needs no
/// permission to control Finder.
fn move_to_system_trash(items: &[PathBuf]) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut context = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }
    context.delete_all(items).map_err(|error| error.to_string())
}

/// Tests move items into a folder of their own instead of the machine's Trash.
fn move_into(dir: &Path, items: &[PathBuf]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    for item in items {
        let name = item.file_name().ok_or("an item has no name")?;
        std::fs::rename(item, dir.join(name)).map_err(|error| error.to_string())?;
    }
    Ok(())
}
