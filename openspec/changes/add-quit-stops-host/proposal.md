# Quitting Headquarter stops its host

Status: approved by the owner on 2026-10-27 ("approve, go ahead").

## Why

Quitting the app (⌘Q, the menu, the window's close button) leaves the background host and every
agent running, so a real restart needs `hq stop` as well (owner, 2026-10-27: "closing hq
completely should also close the host, should be one"). The host was kept separate so agents
outlive the window (add-session-daemon), which the owner still wants as a choice, not the
default.

## Expected Outcome

- Quitting asks every time: "Quit Headquarter?" with how many sessions are running, and three
  buttons: Quit, Keep Running, Cancel.
- **Quit** stops the host and its agents, then the app. Sessions that were running resume the
  next time Headquarter opens, as after `hq stop` today.
- **Keep Running** quits the app only. The host and agents go on, as today.
- **Cancel** returns to the app.
- ⌘Q, Headquarter ▸ Quit Headquarter, and the window's close button all ask the same. On Linux,
  Ctrl+Shift+Q and the window's close button.
- With remote control on, the dialog says a paired phone loses access until Headquarter opens
  again (only for Quit).

## What Changes

- `hq-app`: a quit dialog in the window, the quit shortcut and menu action routed to it, the
  window's close request intercepted (`on_window_should_close`), and Quit stopping the host the
  way the update's restart already does (`hq_client::stop_host`, then wait for the socket to
  close, at most 10 s).

## Non-goals

- `hq stop` and the CLI.
- The update's restart, which already stops the host.
- Logging out or shutting down the Mac.

## Capabilities affected

`desktop-app`. No new project capability.

## Impact

App only. No host or protocol change.
