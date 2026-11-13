# Design: usage from one host

## Context

`Workspace::usage_host()` (`crates/hq-app/src/workspace/several.rs`) returns the selected session's
host, else the first host. The footer, the rail's ring, the usage details and their Refresh read
`hosts.usage` for that host. The same function also picks New Session's default host, the host of a
group made empty, and the host the Usage window (CPU and memory) measures. Every host's feed carries
its usage, so the app holds a reading for every connected host.

## Goals and non-goals

Goals: a footer that stays on one host, the owner's choice of that host, and the same rule on a
client without its own host. Non-goals are in the proposal.

## Decisions

### D1. Which host the footer shows

In order:

1. the host chosen in the usage details, while the app holds its usage;
2. this computer, when it runs a host;
3. the first paired host, oldest pairing first, that the app holds usage for.

"Holds usage" means `hosts.usage` has the host and the host is neither being reached again nor given
up on. A chosen host that is away therefore falls back to the next rule, and comes back by itself
when it lists again. The choice is kept meanwhile. The footer names the host when it is not this computer, as now.

Rejected:
- Following the selection (today): the rings flip as the owner clicks, and one host's failure looks
  like another's.
- Merging hosts per agent: a reading names only the plan, so different logins would mix in one ring.
- Showing nothing while the chosen host is away: a blank footer says less than another host's named
  rings.

### D2. Where the owner chooses

The usage details (opened from a ring, the "more" chip or the rail) start with a "Usage from" row
when more than one host is known: one small button per host, this computer first and then the
paired hosts oldest first, named as in the Hosts menu, with the shown host as the primary button, as in New Session's "Start on". A host the app
holds no current usage for is disabled. Choosing one keeps the details open and shows that host's
agents at once.

Rejected:
- The Hosts menu: it manages connections, and the choice belongs beside what it changes.
- Settings: too far from the rings.

### D3. Keeping the choice

`app-state.json` gets `usage_host`: `"local"` or the paired host's id. A missing, unknown or
unreadable value means no choice. Forget drops the choice when it names the forgotten host. Other
keys in the file are kept, as `app_state.rs` does for every key.

### D4. What still follows the selection

New Session's default host, the host of a group made empty, and the Usage window (CPU and memory)
keep following the selected session, because they concern the computer the session runs on, not a
login's limits. Today's `usage_host()` is renamed `selected_host()` for them, and `usage_host()`
becomes D1's rule.

### D5. Hosts keep reading usage

Every connected host keeps its usage reads while the app is open, as now. Choosing another host
then shows a current reading at once, with no protocol change.

## Failure behaviour

- The chosen host fails or is away: the footer shows the next host by D1, and the details' "Usage
  from" row shows the chosen host disabled.
- No host holds usage: the footer is empty, as now.

## Compatibility and rollback

No protocol, host or index change. `app-state.json` gains one key, which older apps ignore.
Rollback is the previous release.

## Risks

- The owner may expect the footer to show the selected server session's limits. The rings are named
  when they are not this computer's, and the choice is one click away.
