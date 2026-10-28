# Saying where a session is in use

Status: proposed 2026-10-28, waiting for the owner's approval (owner: plan now, build after
approval).

## Why

The owner's finding on 0.1.17 (2026-10-28, two screenshots, "cash flow tracker dev" open on the
desktop and the phone):

- Opened on the desktop, then on the phone: the desktop showed Claude drawn as a narrow column
  until the owner clicked in the terminal.
- Opened on the phone, then on the desktop: the phone showed a garbled screen until the owner
  switched to another session on the desktop.

Cause, from the code: this is the size rule of `add-remote-gateway` D6 working as designed. A
session has one terminal, so one size, and the device used last sets it (`Hosted::attach`,
`focus`, `detach` in `hq-daemon` `session.rs`). The desktop takes the session back only when its
terminal gains focus, gets typing or gets a click (`tell_host_in_use` in `workspace.rs`,
`mouse_down` in `terminal/view.rs`); its window was already active, so only the click did. The
phone keeps its own narrow grid while the agent draws for the desktop's width, so lines wrap and
land in the wrong places.

The defect is what the device not in use shows: a broken or shrunken screen with no reason given
and no obvious way back. Two screens of different sizes cannot both show one terminal program
correctly, because the agent draws for one size (tmux, zellij and SSH share this limit). Drawing
the conversation separately for the phone was discussed and declined for now (owner,
2026-10-28).

## Expected Outcome

- The phone, when another device is in use, covers the terminal with "In use on <computer>" and a
  "Use here" button. Tapping it, or typing, makes the phone the device in use, and the agent
  redraws at the phone's width.
- The desktop, when a phone is in use, keeps showing the terminal under a slim bar "In use on
  <phone> · Click to use here". Clicking the bar or the terminal, or typing, takes it back, as
  today.
- With one device, nothing changes on screen.

## What Changes

- `hq-protocol`: a new response telling a viewer who is in use, and a request naming the phone
  behind a remote connection. Protocol 13 becomes 14.
- `hq-daemon`: each viewer is told who is in use when it attaches and whenever that changes. The
  remote proxy names the phone; a phone cannot name itself (`allowed()` refuses it).
- `hq-app`: the bar over the terminal pane and the shell panel.
- `hq-mobile` and the Android app: the cover and "Use here".
- Specs: `session-host`, `remote-control`, `terminal-pane`, `android-app`.

## Non-goals

- Showing both devices at their own sizes at once (not possible for one terminal program).
- A phone view of the conversation drawn from the agent's transcript (declined for now).
- Changing who takes the size (D6 stays: the device used last).

## Capabilities affected

`session-host`, `remote-control`, `terminal-pane`, `android-app`. No new project capability.

## Impact

Desktop app, host and phone app. The protocol version changes, so the phone and the desktop must
be updated together. Both release together, and the phone already says which side to update.
