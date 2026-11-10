# Show usage from one host, chosen by the owner

Status: approved by the owner 2026-11-10 ("ok, approved, go ahead"). Option B chosen by the owner 2026-11-10 ("same logins, go
with B").

## Why

Testing `v0.4.0-rc.2` (2026-11-10), the owner saw the Claude usage ring on this Mac sometimes say
Claude usage is not available. This Mac reads it fine: every Claude usage read in its host log since
4 November succeeded. The footer follows the selected session's host (`add-merged-host-list` D5), so
selecting a server session switches the rings to the server's usage, where Claude's read fails. The
server's name is added to the label under the rings, but it is easy to miss, and the rings seem to
flicker between good and not available.

The owner uses the same Claude and Codex logins on both computers, so switching hosts shows the same
limits twice at best. At worst it shows one host's failure as if it were this computer's.

## Expected Outcome

- The footer shows one host's usage, whichever session is selected: this computer's when it runs a
  host, else the first paired host.
- In the usage details, the owner can choose another host. The footer then shows that host's rings
  with its name, and the app remembers the choice.
- A client without its own host (Windows) shows its first paired host's usage, named.

## What Changes

- The footer's host no longer follows the selection. It is the chosen host while the app holds its
  usage, else this computer when it runs a host, else the first paired host the app holds usage for.
- The usage details get a "Usage from" row, one button per host, when the app knows more than one host.
- The choice is kept in `app-state.json` and dropped when that host is forgotten.
- New Session's default host, a new group's host, and the Usage window (CPU and memory) still follow
  the selected session, as now.

## Non-goals

- Merging usage across hosts. A reading names only the plan, not the login, so two hosts cannot be
  matched.
- Why the server's Claude usage read fails. That is a separate fix once the owner sends the
  server's `hq usage` line.
- Hosts reading usage only while their usage is shown. Every connected host keeps reading it as
  now, so switching hosts is instant.
- The phone app, which shows one host.

## Capabilities

`desktop-app` only. No new project capability.

## Impact

`crates/hq-app`: `workspace/several.rs` (choosing the host), `workspace.rs` (footer and usage
details), `app_state.rs` (the kept choice), `workspace/hosts.rs` (Forget), and the callers of today's
`usage_host()`. No protocol, host or phone change. Ships in 0.4.0.
