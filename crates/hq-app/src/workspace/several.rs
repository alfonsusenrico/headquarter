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
    /// Hosts whose feed gave up, and why: removed there, versions differ, not installed.
    pub failed: HashMap<HostId, String>,
    /// Hosts that answered with their list at least once.
    pub listed: Vec<HostId>,
    feeds: HashMap<HostId, Task<()>>,
    /// The paired host whose Forget waits for a second click (D6).
    pub forget_asked: Option<String>,
    /// The host New Session starts a session on (D4).
    pub new_on: HostId,
}

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

    /// The host whose usage the footer shows: the selected session's, else this computer's,
    /// else the first host (D5).
    pub(super) fn usage_host(&self) -> HostId {
        if let Some(id) = self.selected.as_deref().or_else(|| self.shown_id()) {
            return self.host_of(id);
        }
        hosts::all().into_iter().next().unwrap_or(HostId::Local)
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
        self.hosts
            .usage
            .get(&self.usage_host())
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
