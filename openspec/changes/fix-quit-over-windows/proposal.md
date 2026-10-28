# Quit's question over every window

Status: requested by the owner on 2026-10-28 ("fix the quit dialog under Remote Control"), which
approves this fix.

## Why

Found while checking add-phone-app-download (2026-10-28): with Headquarter ▸ Remote Control…
open, ⌘Q or Quit Headquarter asked its question underneath the Remote Control window, where it
could not be seen. Focus went to the hidden question, so Esc cancelled it instead of closing the
window, and the window then no longer answered Esc. The same happens over the update window and
the Usage window.

## Expected Outcome

- Quit's question shows on top of every window the app draws: Remote Control, Check for
  Updates, Usage, the session windows and the confirmations.
- Cancel (or Esc) closes the question and puts focus back where it was, so the window under it
  answers Esc again.

## What Changes

- `hq-app` `workspace.rs`: the question is drawn last.
- `hq-app` `workspace/quit.rs`: the question keeps the focus it took and gives it back on Cancel.
- `desktop-app` spec: a scenario for quitting with a window open.

## Non-goals

- What Quit, Keep Running and Cancel do.
- The order of the other windows.

## Capabilities affected

`desktop-app`. No new project capability.

## Impact

Desktop app only. No agent CLI is involved.
