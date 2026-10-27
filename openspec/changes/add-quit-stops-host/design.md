# Design: quitting Headquarter stops its host

## Context

The app quits with `cx.quit()` from the Quit shortcut (`workspace.rs`) and the menu's Quit action
(`menus.rs`). The window's close button closes the only window, which ends the app. The host
keeps running either way. The update's restart (`workspace/update.rs`) stops the host with
`hq_client::stop_host` and waits for `agentd.sock` to close.

## Decisions

### D1. One dialog for every way out

The Quit shortcut, the menu's Quit, and the window's close request (`window.on_window_should_close`
returns false and opens the dialog) all open one dialog in the window's modal layer. It asks every
time, also with no session running (owner, 2026-10-27).

Title "Quit Headquarter?". Body: "3 sessions are running. Quit stops them, and they resume when
you open Headquarter again." (or "No sessions are running."). With remote control on, a second
line: "Your paired phone can't reach this computer until you open Headquarter again." Buttons,
equal width: Cancel, Keep Running, Quit (primary). Enter is Quit, Esc is Cancel.

### D2. Quit

The dialog shows "Stopping sessions…", the app saves its window state, stops the host
(`hq_client::stop_host`), waits until `agentd.sock` stops answering (at most 10 s), then quits.
If the host cannot be stopped, the dialog says why and offers Quit Anyway (the app quits, the
host stays) and Cancel.

### D3. Keep Running

The app saves its window state and quits, leaving the host as today.

### D4. Already quitting

A quit while the dialog is open, or while it stops the host, does nothing more. The update's
restart is unchanged, since it stops the host itself before quitting.

## Risks

- macOS logout sends a quit the app now answers with a dialog, which can hold the logout until
  answered. Accepted, as other apps with unsaved work do.
