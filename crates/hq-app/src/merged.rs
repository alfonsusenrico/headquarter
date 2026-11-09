//! Groups merged by name across hosts (add-merged-host-list D3). Each host keeps its own
//! groups, with ids of its own; the sidebar shows one group per name, under a display id
//! that stays the same while the name exists, and maps it back to each host's group.

use std::collections::HashMap;

use hq_protocol::GroupInfo;

use crate::hosts::HostId;

/// Display ids by group name, kept across merges so the sidebar's state for a group (folded,
/// being renamed) follows it.
#[derive(Debug, Default)]
pub struct DisplayIds {
    by_name: HashMap<String, i64>,
    next: i64,
}

impl DisplayIds {
    fn of(&mut self, name: &str) -> i64 {
        let key = key(name);
        if let Some(id) = self.by_name.get(&key) {
            return *id;
        }
        self.next += 1;
        self.by_name.insert(key, self.next);
        self.next
    }
}

/// Names match whatever their case or surrounding space.
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// The merged groups and how each maps to the hosts' own.
#[derive(Debug, Default)]
pub struct MergedGroups {
    /// What the sidebar shows, in order, under display ids.
    pub groups: Vec<GroupInfo>,
    /// Each display id's group on each host that has it.
    members: HashMap<i64, Vec<(HostId, i64)>>,
    to_display: HashMap<(HostId, i64), i64>,
}

impl MergedGroups {
    /// The display id of a host's group.
    pub fn display(&self, host: &HostId, group: Option<i64>) -> Option<i64> {
        group.and_then(|group| self.to_display.get(&(host.clone(), group)).copied())
    }

    /// The host's own group behind a display id, when the host has one of that name.
    pub fn on_host(&self, display: i64, host: &HostId) -> Option<i64> {
        self.members
            .get(&display)?
            .iter()
            .find(|(member, _)| member == host)
            .map(|(_, id)| *id)
    }

    /// Every host's group behind a display id.
    pub fn hosts_of(&self, display: i64) -> Vec<(HostId, i64)> {
        self.members.get(&display).cloned().unwrap_or_default()
    }

    pub fn name(&self, display: i64) -> Option<&str> {
        self.groups
            .iter()
            .find(|group| group.id == display)
            .map(|group| group.name.as_str())
    }
}

/// Merges the hosts' groups by name: the first host's order (this computer's, listed first),
/// then names only later hosts have, in their order. A merged group takes its name's spelling,
/// folded state and colour from the first host that has it, and a colour from a later one when
/// the first has none.
pub fn merge(
    order: &[HostId],
    per_host: &HashMap<HostId, Vec<GroupInfo>>,
    ids: &mut DisplayIds,
) -> MergedGroups {
    let mut merged = MergedGroups::default();
    for host in order {
        for group in per_host.get(host).into_iter().flatten() {
            let display = ids.of(&group.name);
            merged.to_display.insert((host.clone(), group.id), display);
            merged
                .members
                .entry(display)
                .or_default()
                .push((host.clone(), group.id));
            match merged.groups.iter_mut().find(|shown| shown.id == display) {
                Some(shown) => {
                    if shown.color.is_none() {
                        shown.color.clone_from(&group.color);
                    }
                }
                None => merged.groups.push(GroupInfo {
                    id: display,
                    ..group.clone()
                }),
            }
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: i64, name: &str, color: Option<&str>) -> GroupInfo {
        GroupInfo {
            id,
            name: name.into(),
            collapsed: false,
            color: color.map(str::to_owned),
        }
    }

    fn server() -> HostId {
        HostId::Remote("server".into())
    }

    #[test]
    fn same_names_merge_in_this_computers_order_then_the_others() {
        let per_host = HashMap::from([
            (
                HostId::Local,
                vec![group(1, "ops", None), group(2, "Docs", None)],
            ),
            (
                server(),
                vec![
                    group(1, "Health", Some("teal")),
                    group(7, " OPS ", Some("violet")),
                ],
            ),
        ]);
        let mut ids = DisplayIds::default();
        let merged = merge(&[HostId::Local, server()], &per_host, &mut ids);
        let names: Vec<&str> = merged.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["ops", "Docs", "Health"]);
        let ops = merged.groups[0].id;
        assert_eq!(
            merged.groups[0].color.as_deref(),
            Some("violet"),
            "the server's colour fills in"
        );
        assert_eq!(merged.display(&HostId::Local, Some(1)), Some(ops));
        assert_eq!(merged.display(&server(), Some(7)), Some(ops));
        assert_eq!(
            merged.display(&server(), Some(1)),
            Some(merged.groups[2].id),
            "ids clash, names do not"
        );
        assert_eq!(merged.on_host(ops, &server()), Some(7));
        assert_eq!(
            merged.on_host(merged.groups[1].id, &server()),
            None,
            "the server has no Docs"
        );
        assert_eq!(
            merged.hosts_of(ops),
            vec![(HostId::Local, 1), (server(), 7)]
        );
    }

    #[test]
    fn a_name_keeps_its_display_id_across_merges() {
        let mut ids = DisplayIds::default();
        let first = merge(
            &[HostId::Local],
            &HashMap::from([(HostId::Local, vec![group(1, "ops", None)])]),
            &mut ids,
        );
        let second = merge(
            &[HostId::Local, server()],
            &HashMap::from([
                (
                    HostId::Local,
                    vec![group(2, "new", None), group(1, "ops", None)],
                ),
                (server(), vec![group(5, "ops", None)]),
            ]),
            &mut ids,
        );
        let ops =
            |merged: &MergedGroups| merged.groups.iter().find(|g| g.name == "ops").unwrap().id;
        assert_eq!(ops(&first), ops(&second));
    }
}
