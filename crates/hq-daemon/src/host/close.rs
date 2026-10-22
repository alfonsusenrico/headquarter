//! Closing a hosted session with the agent's own keys (revise-session-lifecycle D4):
//! Codex sub-agents are interrupted through the shared server, a working turn is
//! interrupted, unsent text is cleared, and the agent's exit commands are typed. An
//! agent that does not exit is hung up, then killed. The session leaves the sidebar when
//! its agent has exited (`on_exit`).

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use hq_signals::event::Agent;
use hq_signals::status::Status;
use hq_signals::{adapters, codex_server, external, paths, subagents};

use super::usage::problem_text;
use super::{Host, OpResult, lock};
use crate::session::Hosted;

const INTERRUPT_WAIT: Duration = Duration::from_secs(3);
const EXIT_WAIT: Duration = Duration::from_secs(10);
const HANGUP_WAIT: Duration = Duration::from_secs(3);
const SUBAGENT_TIMEOUT: Duration = Duration::from_secs(5);
/// Agents read typed commands key by key; a burst can be taken for a paste.
const KEY_GAP: Duration = Duration::from_millis(15);
/// Time for the agent to act on a key or command before the next one.
const SETTLE: Duration = Duration::from_millis(400);

impl Host {
    /// Closes a session: it leaves the sidebar at once, and its agent is stopped in the
    /// background, so the owner never waits for the agent's own exit (D4). One not
    /// running here (ended, or running in another terminal) is not signalled.
    pub(super) fn close(self: &Arc<Self>, reference: &str) -> OpResult<()> {
        let row = self.resolve(reference)?;
        let hosted = lock(&self.live).get(&row.id).cloned();
        self.remove_from_sidebar(std::slice::from_ref(&row.id))?;
        let Some(hosted) = hosted else {
            return Ok(());
        };
        if hosted.begin_close() {
            let host = Arc::clone(self);
            thread::spawn(move || host.run_close(&hosted));
        }
        Ok(())
    }

    fn run_close(&self, hosted: &Arc<Hosted>) {
        let keys = adapters::close_keys(hosted.agent);
        if hosted.agent == Agent::Codex {
            self.stop_subagents(hosted);
        }
        if hosted.status() == Status::Working {
            hosted.write_to_agent(keys.interrupt);
            wait_for(INTERRUPT_WAIT, || hosted.status() != Status::Working);
        }
        for keys in [keys.dismiss, keys.clear] {
            if !keys.is_empty() {
                hosted.write_to_agent(keys);
                thread::sleep(SETTLE);
            }
        }
        for command in keys.exit {
            for key in command.bytes() {
                hosted.write_to_agent(&[key]);
                thread::sleep(KEY_GAP);
            }
            thread::sleep(SETTLE);
            hosted.write_to_agent(b"\r");
            if wait_for(SETTLE * 3, || !self.is_live(&hosted.id)) {
                return;
            }
        }
        if wait_for(EXIT_WAIT, || !self.is_live(&hosted.id)) {
            return;
        }
        self.log.line(&format!(
            "close fell back to hangup agent={}",
            hosted.agent.id()
        ));
        hosted.kill();
        if wait_for(HANGUP_WAIT, || !self.is_live(&hosted.id)) {
            return;
        }
        if let Some(pid) = hosted.pid {
            let killed = external::force_kill(pid);
            self.log.line(&format!(
                "close killed agent={} ok={killed}",
                hosted.agent.id()
            ));
        }
    }

    /// Interrupts the Codex sub-agents still running for the session, looked up now so
    /// one started since the last scan is included.
    fn stop_subagents(&self, hosted: &Hosted) {
        let thread_id = lock(&self.store)
            .get(&hosted.id)
            .ok()
            .flatten()
            .and_then(|row| row.agent_session_id);
        let (Some(thread_id), Some(home)) = (thread_id, paths::codex_home()) else {
            return;
        };
        let running = subagents::running_subagents(&home.join("sessions"), &[thread_id])
            .into_values()
            .flatten()
            .collect::<Vec<_>>();
        if running.is_empty() {
            return;
        }
        let outcome = self
            .resolve_agent(Agent::Codex)
            .map_err(|error| problem_text(*error))
            .and_then(|codex| codex_server::interrupt_turns(&codex, &running, SUBAGENT_TIMEOUT));
        match outcome {
            Ok(stopped) => self.log.line(&format!(
                "close stopped codex subagents count={stopped} of={}",
                running.len()
            )),
            Err(error) => self.log.line(&format!(
                "close could not stop codex subagents count={} error={error}",
                running.len()
            )),
        }
    }

    fn is_live(&self, id: &str) -> bool {
        lock(&self.live).contains_key(id)
    }
}

/// Polls `done` until it holds or `limit` passes; returns whether it held.
fn wait_for(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
