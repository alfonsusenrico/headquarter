//! Every host's sessions in one list (add-merged-host-list): one feed per host, each event
//! tagged with its host, merged into the workspace's list; groups merged by name; each
//! session's host kept for routing and its row.

use std::collections::HashMap;

use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use gpui::{Context, Task, Window};
use hq_client::FeedControl;
use hq_protocol::{AgentUsage, GroupInfo, SessionInfo};

use super::{Pane, Workspace, watch_sessions};
use crate::hosts::{self, HostId};
use crate::merged::{self, DisplayIds, MergedGroups};
use crate::sidebar::model::RowHost;
use crate::trouble::HostTrouble;

/// What the workspace knows per host.
#[derive(Default)]
pub(super) struct Hosts {
    /// Each listed session's host (session ids are host-made UUIDs: unique across hosts, D1).
    pub session_hosts: HashMap<String, HostId>,
    /// Each session's group on its own host, before the merged view's display id.
    real_groups: HashMap<String, Option<i64>>,
    groups: HashMap<HostId, Vec<GroupInfo>>,
    display_ids: DisplayIds,
    pub merged: MergedGroups,
    /// Each host's home folder, labelled `~` in its sessions' paths.
    pub homes: HashMap<HostId, String>,
    pub usage: HashMap<HostId, Vec<AgentUsage>>,
    /// The handle telling each host what the window shows.
    pub controls: HashMap<HostId, FeedControl>,
    /// Hosts being reached again, and why.
    pub away: HashMap<HostId, String>,
    /// Hosts the owner has to act on, and why: removed there, versions differ, not readable.
    pub failed: HashMap<HostId, HostTrouble>,
    /// Hosts that answered with their list at least once.
    pub listed: Vec<HostId>,
    feeds: HashMap<HostId, Task<()>>,
    /// The paired host whose Forget waits for a second click (D6).
    pub forget_asked: Option<String>,
    /// The host New Session starts a session on (D4).
    pub new_on: HostId,
    /// The host the owner chose for the usage footer, kept in `app-state.json`
    /// (fix-usage-host D3).
    pub usage_choice: Option<HostId>,
}

impl Hosts {
    /// No host followed yet, with the usage host choice kept from the last run.
    pub(super) fn kept() -> Hosts {
        Hosts {
            usage_choice: crate::app_state::path()
                .and_then(|path| crate::app_state::load_usage_host(&path))
                .map(|key| usage_host_from_key(&key)),
            ..Hosts::default()
        }
    }
}

/// The footer's host (fix-usage-host D1): the chosen host while the app holds its usage,
/// else this computer when it runs a host (`hosts` then starts with it), else the first host
/// in pairing order the app holds usage for.
pub(super) fn footer_host(
    chosen: Option<&HostId>,
    hosts: &[HostId],
    holds: impl Fn(&HostId) -> bool,
) -> Option<HostId> {
    if let Some(chosen) = chosen.filter(|chosen| holds(chosen)) {
        return Some(chosen.clone());
    }
    if hosts.contains(&HostId::Local) {
        return Some(HostId::Local);
    }
    hosts.iter().find(|host| holds(host)).cloned()
}

/// How `app-state.json` names a host: `"local"` or the paired host's id.
pub(super) fn usage_host_key(host: &HostId) -> String {
    match host {
        HostId::Local => LOCAL_KEY.to_owned(),
        HostId::Remote(id) => id.clone(),
    }
}

fn usage_host_from_key(key: &str) -> HostId {
    if key == LOCAL_KEY {
        HostId::Local
    } else {
        HostId::Remote(key.to_owned())
    }
}

const LOCAL_KEY: &str = "local";

impl Workspace {
    /// Starts following every host the app lists.
    pub(super) fn follow_every_host(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if hosts::none() {
            self.listed = true;
            self.message = Some(hosts::NO_HOST_YET.to_owned());
            return;
        }
        for host in hosts::all() {
            self.follow_host_feed(host, window, cx);
        }
    }

    /// Follows one host's sessions on a thread of its own (D2).
    pub(super) fn follow_host_feed(
        &mut self,
        host: HostId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (tx, mut rx) = unbounded();
        let watched = host.clone();
        std::thread::spawn(move || watch_sessions(&watched, &tx));
        let tagged = host.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            while let Some(event) = rx.next().await {
                let received = this.update_in(cx, |workspace, window, cx| {
                    workspace.receive(tagged.clone(), event, window, cx);
                });
                if received.is_err() {
                    break;
                }
            }
        });
        self.message = None;
        self.hosts.feeds.insert(host, task);
    }

    /// A host just paired: its sessions join the list.
    pub(super) fn add_host(&mut self, host: HostId, window: &mut Window, cx: &mut Context<Self>) {
        self.follow_host_feed(host, window, cx);
        cx.notify();
    }

    /// A host forgotten: its feed ends and its sessions leave the list.
    pub(super) fn drop_host(&mut self, host: &HostId, window: &mut Window, cx: &mut Context<Self>) {
        self.hosts.feeds.remove(host);
        self.hosts.failed.remove(host);
        self.forget_sessions_of(host, window, cx);
        if hosts::none() {
            self.message = Some(hosts::NO_HOST_YET.to_owned());
        }
    }

    /// Takes a host's sessions, groups, usage and state out of the workspace.
    pub(super) fn forget_sessions_of(
        &mut self,
        host: &HostId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hosts.controls.remove(host);
        self.hosts.usage.remove(host);
        self.hosts.homes.remove(host);
        self.hosts.away.remove(host);
        self.hosts.groups.remove(host);
        self.hosts.listed.retain(|listed| listed != host);
        let gone: Vec<String> = self
            .hosts
            .session_hosts
            .iter()
            .filter(|(_, owner)| *owner == host)
            .map(|(id, _)| id.clone())
            .collect();
        if self
            .shown_id()
            .is_some_and(|id| gone.iter().any(|gone| gone == id))
        {
            self.pane = Pane::Empty;
            self.selected = None;
            self.sync_shell(window, cx);
        }
        self.sessions.retain(|session| !gone.contains(&session.id));
        for id in &gone {
            self.hosts.session_hosts.remove(id);
            self.hosts.real_groups.remove(id);
        }
        self.remerge();
        cx.notify();
    }

    /// The host a session runs on; this computer's when it is not known.
    pub(super) fn host_of(&self, id: &str) -> HostId {
        self.hosts
            .session_hosts
            .get(id)
            .cloned()
            .unwrap_or_else(|| hosts::all().into_iter().next().unwrap_or(HostId::Local))
    }

    /// A host's home folder, labelled `~` in its sessions' paths.
    pub(super) fn home_of(&self, host: &HostId) -> Option<&str> {
        match host {
            HostId::Local => self.home.as_deref(),
            remote => self.hosts.homes.get(remote).map(String::as_str),
        }
    }

    /// The selected session's host, else the first host: where New Session starts, a group
    /// made empty goes, and the Usage window measures (fix-usage-host D4).
    pub(super) fn selected_host(&self) -> HostId {
        if let Some(id) = self.selected.as_deref().or_else(|| self.shown_id()) {
            return self.host_of(id);
        }
        hosts::all().into_iter().next().unwrap_or(HostId::Local)
    }

    /// The host whose usage the footer shows (fix-usage-host D1); `None` before any host
    /// reports usage.
    pub(super) fn usage_host(&self) -> Option<HostId> {
        footer_host(self.hosts.usage_choice.as_ref(), &hosts::all(), |host| {
            self.holds_usage(host)
        })
    }

    /// Whether the app has a current usage reading from `host`: it reported one and is
    /// neither being reached again nor given up on.
    pub(super) fn holds_usage(&self, host: &HostId) -> bool {
        self.hosts.usage.contains_key(host)
            && !self.hosts.away.contains_key(host)
            && !self.hosts.failed.contains_key(host)
    }

    /// Shows `host`'s usage in the footer from now on and keeps the choice (D3).
    pub(super) fn choose_usage_host(&mut self, host: HostId, cx: &mut Context<Self>) {
        self.keep_usage_choice(Some(host));
        // Details open on one agent stay open on the new host's first agent when it lacks
        // that one.
        if let Some(open) = self.usage_open {
            let shown = self.shown_usage();
            if !shown.iter().any(|usage| usage.agent == open) {
                self.usage_open = shown.first().map(|usage| usage.agent);
            }
        }
        cx.notify();
    }

    /// Sets the usage host choice and writes it to `app-state.json`.
    pub(super) fn keep_usage_choice(&mut self, choice: Option<HostId>) {
        let key = choice.as_ref().map(usage_host_key);
        self.hosts.usage_choice = choice;
        if let Some(path) = crate::app_state::path() {
            // The choice still holds for this run when the file cannot be written.
            let _ = crate::app_state::save_usage_host(&path, key.as_deref());
        }
    }

    /// This computer's sessions: the only ones quitting or updating may stop.
    pub(super) fn local_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .filter(|session| self.host_of(&session.id) == HostId::Local)
            .cloned()
            .collect()
    }

    /// The usage the footer shows.
    pub(super) fn shown_usage(&self) -> &[AgentUsage] {
        self.usage_host()
            .and_then(|host| self.hosts.usage.get(&host))
            .map_or(&[], Vec::as_slice)
    }

    /// Takes in a session from `host`: notes its host and its own group, and gives it the
    /// merged group's display id.
    pub(super) fn adopt(&mut self, host: &HostId, mut session: SessionInfo) -> SessionInfo {
        self.hosts
            .session_hosts
            .insert(session.id.clone(), host.clone());
        self.hosts
            .real_groups
            .insert(session.id.clone(), session.group);
        session.group = self.hosts.merged.display(host, session.group);
        session
    }

    /// A host's whole list: it replaces that host's sessions only (D2).
    pub(super) fn take_host_list(
        &mut self,
        host: &HostId,
        sessions: Vec<SessionInfo>,
        groups: Vec<GroupInfo>,
    ) {
        let theirs: Vec<String> = self
            .hosts
            .session_hosts
            .iter()
            .filter(|(_, owner)| *owner == host)
            .map(|(id, _)| id.clone())
            .collect();
        self.sessions
            .retain(|session| !theirs.contains(&session.id));
        for id in &theirs {
            self.hosts.session_hosts.remove(id);
            self.hosts.real_groups.remove(id);
        }
        self.hosts.groups.insert(host.clone(), groups);
        self.remerge();
        for session in sessions {
            let session = self.adopt(host, session);
            self.sessions.push(session);
        }
        if !self.hosts.listed.contains(host) {
            self.hosts.listed.push(host.clone());
        }
    }

    pub(super) fn take_host_groups(&mut self, host: &HostId, groups: Vec<GroupInfo>) {
        self.hosts.groups.insert(host.clone(), groups);
        self.remerge();
    }

    /// Merges the hosts' groups again and gives every session its group's display id.
    pub(super) fn remerge(&mut self) {
        self.hosts.merged = merged::merge(
            &hosts::all(),
            &self.hosts.groups,
            &mut self.hosts.display_ids,
        );
        self.groups = self.hosts.merged.groups.clone();
        for session in &mut self.sessions {
            let Some(host) = self.hosts.session_hosts.get(&session.id) else {
                continue;
            };
            let real = self.hosts.real_groups.get(&session.id).copied().flatten();
            session.group = self.hosts.merged.display(host, real);
        }
    }

    /// What the sidebar says about each paired host the owner has to act on, in name order
    /// (fix-hosts-menu-reason D2).
    pub(super) fn trouble_lines(&self) -> Vec<String> {
        trouble_lines(
            self.hosts
                .failed
                .iter()
                .filter(|(host, _)| **host != HostId::Local)
                .map(|(host, trouble)| (hosts::name(host), trouble)),
        )
    }

    /// How each session's host shows in its row (D7).
    pub(super) fn row_hosts(&self) -> HashMap<String, RowHost> {
        let mut labels: HashMap<HostId, RowHost> = HashMap::new();
        self.hosts
            .session_hosts
            .iter()
            .map(|(id, host)| {
                let shown = labels
                    .entry(host.clone())
                    .or_insert_with(|| RowHost {
                        label: hosts::label(host),
                        home: self.hosts.homes.get(host).cloned(),
                        away: self.hosts.away.contains_key(host),
                    })
                    .clone();
                (id.clone(), shown)
            })
            .collect()
    }
}

/// The lines for `(host name, trouble)` pairs, sorted by host name.
fn trouble_lines<'a>(troubles: impl Iterator<Item = (String, &'a HostTrouble)>) -> Vec<String> {
    let mut named: Vec<(String, &HostTrouble)> = troubles.collect();
    named.sort_by_key(|(name, _)| name.to_lowercase());
    named
        .into_iter()
        .map(|(name, trouble)| trouble.line(&name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(id: &str) -> HostId {
        HostId::Remote(id.to_owned())
    }

    #[test]
    fn the_footer_stays_on_one_host() {
        let mac_and_server = [HostId::Local, remote("server")];
        let all = |_: &HostId| true;
        assert_eq!(
            footer_host(None, &mac_and_server, all),
            Some(HostId::Local),
            "this computer when it runs a host"
        );
        assert_eq!(
            footer_host(Some(&remote("server")), &mac_and_server, all),
            Some(remote("server")),
            "the chosen host"
        );
        let only_local = |host: &HostId| *host == HostId::Local;
        assert_eq!(
            footer_host(Some(&remote("server")), &mac_and_server, only_local),
            Some(HostId::Local),
            "a chosen host that is away falls back"
        );
        assert_eq!(
            footer_host(Some(&remote("forgotten")), &mac_and_server, |host| {
                *host != remote("forgotten")
            }),
            Some(HostId::Local),
            "an unknown choice is no choice"
        );
    }

    #[test]
    fn a_client_without_a_host_shows_its_first_paired_host() {
        let two_servers = [remote("first"), remote("second")];
        assert_eq!(
            footer_host(None, &two_servers, |_| true),
            Some(remote("first"))
        );
        assert_eq!(
            footer_host(None, &two_servers, |host| *host == remote("second")),
            Some(remote("second")),
            "the first one the app holds usage for"
        );
        assert_eq!(
            footer_host(Some(&remote("second")), &two_servers, |_| true),
            Some(remote("second"))
        );
        assert_eq!(
            footer_host(None, &two_servers, |_| false),
            None,
            "no host yet"
        );
    }

    #[test]
    fn usage_host_keys_round_trip() {
        for host in [HostId::Local, remote("b2f0c1")] {
            assert_eq!(usage_host_from_key(&usage_host_key(&host)), host);
        }
        assert_eq!(usage_host_key(&HostId::Local), "local");
    }

    #[test]
    fn trouble_lines_follow_the_host_names() {
        let update = HostTrouble::update(1, "0.1.0".into());
        let removed = HostTrouble::pair_again();
        let lines = trouble_lines(
            [
                ("server".to_owned(), &update),
                ("Laptop".to_owned(), &removed),
            ]
            .into_iter(),
        );
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("Laptop no longer knows"), "{lines:?}");
        assert!(lines[1].starts_with("server needs an update"), "{lines:?}");
    }
}
