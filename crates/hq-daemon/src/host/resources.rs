//! What the host and each running session cost the machine, measured when a client asks
//! (add-usage-window D1, D2). Nothing is sampled otherwise.

use hq_protocol::{Response, SessionFootprint};

use super::{Host, lock};

impl Host {
    pub(super) fn resources(&self) -> Response {
        // One lock at a time: the agents' pids first, then their shells'.
        let agents: Vec<(String, u32)> = lock(&self.live)
            .values()
            .filter_map(|hosted| Some((hosted.id.clone(), hosted.pid?)))
            .collect();
        let shells: Vec<(String, u32)> = lock(&self.shells)
            .iter()
            .filter_map(|(session, shell)| Some((session.clone(), shell.pid?)))
            .collect();
        let mut sampler = lock(&self.resources);
        sampler.refresh();
        let host = sampler.alone(std::process::id()).unwrap_or_default();
        let sessions = agents
            .into_iter()
            .filter_map(|(session, pid)| {
                let mut roots = vec![pid];
                roots.extend(
                    shells
                        .iter()
                        .filter(|(owner, _)| *owner == session)
                        .map(|(_, shell)| *shell),
                );
                let footprint = sampler.tree(&roots)?;
                Some(SessionFootprint { session, footprint })
            })
            .collect();
        Response::Resources { host, sessions }
    }
}
