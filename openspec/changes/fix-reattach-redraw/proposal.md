# A correct screen when coming back to a session

Status: approved by the owner on 2026-10-27 ("ok do both").

## Why

The owner often finds a session's screen broken when coming back to it (a sidebar click, an app
restart, switching back to the app), and has to resize the pane to fix it (2026-10-27).
Findings: `research/evidence/fix-reattach-redraw/findings.md`.

Two causes together:

- The app attaches at a placeholder 80 x 24 before the pane is laid out and sends its real size
  a few milliseconds later. The agent draws a frame at 80 x 24, a size nobody shows. The
  owner's screenshot shows that frame left under the real one.
- The host's redraw shrinks the terminal by one row and restores it at once. Claude Code 2.1.311
  repainted 0 times in 5 runs for that. With a pause between the two sizes it repainted every
  time (5 ms to 100 ms, 5 of 5). So whether the screen is right depends on the agent handling
  the 80 x 24 detour on its own, which a busy session may not.

## Expected Outcome

- Coming back to a session shows the agent's current screen at the pane's size, without stray
  text from another size, whether the session is working or idle.
- The agent always gets a size change it notices when a viewer attaches or takes over, so it
  repaints.

## What Changes

- `hq-app`: a terminal attaches only once its pane has a size, at that size (the session and
  the shell panel). If the pane has no size after 500 ms it attaches at 80 x 24 as today.
- `hq-daemon`: the redraw shrinks the terminal by one row, waits until the agent repaints at
  that size (it erases the screen) or 500 ms, then restores the size. It runs off the request
  path, and any newer size change replaces a pending restore. A resize to the size a viewer
  already has is ignored.

## Non-goals

- The catch-up buffer's size and contents.
- The phone, which already attaches at its real size.

## Capabilities affected

`session-host`, `terminal-pane`. No new project capability.

## Impact

`crates/hq-app/src/terminal/session.rs` and `view.rs`, `crates/hq-daemon/src/session.rs`, and
tests in `crates/hq-cli/tests/host.rs`. No protocol change.
