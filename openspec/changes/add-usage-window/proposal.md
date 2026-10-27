# A Usage window: CPU and memory per session

Status: awaiting the owner's approval.

## Why

The owner wants to see what each session costs the machine (2026-10-27): "a hidden menu Usage,
through cmd/ctrl + U, a complete list of added sessions active or inactive with memory and CPU
usage, inactive shown as inactive, and at the top how many resources the Headquarter app uses
alone (client and host combined)". Options page:
`research/prototypes/folder-and-usage-options/index.html`, section 2. Owner's choices: the
shortcut shows a small top-5 popover (E), whose Show all opens the full table (A), and the menu
bar opens the table directly. No refresh badge. Linux uses Ctrl+Shift+U. Only sessions in the
sidebar, a session's figures include the processes it started with a note saying so, and the
phone does not get this.

## Expected Outcome

- ⌘U (Ctrl+Shift+U on Linux) opens a small popover at the window's top right: Headquarter's own
  CPU and memory (app and host together), then the five sessions using the most CPU, and Show
  all.
- Show all, or Window ▸ Usage on macOS, opens the Usage window: the Headquarter card, a total
  for all agents, then every sidebar session with its agent, status, CPU % (100% is one core)
  and memory, sortable by either. A session's figures cover its agent and every process it
  started (MCP servers, tool shells, Claude Code's own background daemon), plus its shell
  panel, with a note such as "+3 child processes".
- Sessions that are not running read "not running", with no figures.
- Figures refresh every 2 s while the popover or window is open, and nothing is measured
  otherwise.

## What Changes

- `hq-protocol`: a `Resources` request and its response (protocol 12): the host's own CPU and
  memory, and per running session its CPU, memory and process count.
- `hq-daemon`: answers it by sampling the process table (`sysinfo`, already a dependency of
  `hq-signals`) for each running session's agent and shell panel and their descendants. The
  phone's allow-list refuses it.
- `hq-app`: the popover, the Usage window, ⌘U, the menu item, and its own CPU and memory added
  to the host's for the Headquarter figure.

## Non-goals

- Sessions started elsewhere (outside sessions), history, graphs, alerts, killing processes.
- The phone.
- Process names or arguments on screen. Arguments can hold tokens.

## Capabilities affected

`desktop-app`, `session-host`, `remote-control` (one more refused request). No new project
capability.

## Impact

Protocol 11 to 12: the app and host update together, as for every protocol change.
