# Design: a Usage window

## Context

The host knows each running session's agent pid (`Hosted::pid`) and its shell panel. `sysinfo`
is already used by `hq-signals` (`external.rs`). The word "usage" already names the providers'
quota meters (`Request::Usage`), so the protocol calls this "resources". The window is titled
Usage, as the owner asked.

## Decisions

### D1. Measured by the host, on request

`Request::Resources {}` returns `Response::Resources { host, sessions }`. `host` is the host
process's CPU % and resident memory. `sessions` has one entry per running session: id, CPU %,
resident memory, and how many processes beyond the agent itself. A session's processes are its
agent and all descendants, and its shell panel and all descendants. The host keeps one `System`
between requests, because CPU % needs two samples: the first answer has no CPU figure ("…"),
and later ones do. CPU % is per core, as Activity Monitor shows it. Memory is resident memory,
summed. Shared pages count once per process, which overstates slightly, the way `top`'s sums do.

Rejected: the app measuring the agents itself. Only the host knows which processes belong to
which session.

### D2. Only while shown

The app asks every 2 s while the popover or the window is open, and not otherwise. The host
measures only when asked. No badge shows the interval.

### D3. Headquarter's own figure

The app adds its own CPU and memory, measured in the app with the same `sysinfo` call, to the
host's figure. One line: Headquarter.

### D4. The popover and the window

- ⌘U (Ctrl+Shift+U on Linux) toggles the popover under the window's top right: the Headquarter
  line, then the five running sessions with the most CPU (name, CPU, memory), then Show all.
  Esc or ⌘U closes it.
- Show all, or Window ▸ Usage on macOS, opens the Usage window in the modal frame: the
  Headquarter card, "All agents" (the sum of the sessions), then a table of every sidebar
  session in sidebar order, sortable by CPU or memory. Rows with child processes say so
  ("+3 child processes"). Sessions that are not running are dimmed and read "not running".
- On Linux, which has no menu bar, the popover's Show all is the way to the window.

### D5. The phone

`allowed()` refuses `Resources`, so a paired phone cannot ask.

## Risks

- A process that changes its parent (a daemon that double-forks) leaves its session's tree
  and is not counted. Claude Code's daemon stays a child of the agent that started it.
- Sampling the process table costs a few milliseconds each time, only while the window is open.
