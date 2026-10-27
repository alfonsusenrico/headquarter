# Tasks

## 1. App

- [ ] 1.1 Attach on the first layout at the pane's size, 80 x 24 after 500 ms without one, for the session and the shell panel (D1). Evidence: the dev app attaching to a stub agent that logs every size it sees: no 80 x 24 among them. Built 2026-10-27 (`TerminalSession::start`, `set_size`). The on-screen part waits for task 3.1.

## 2. Host

- [x] 2.1 Redraw: shrink, restore after the agent's repaint (erase screen) or 500 ms, off the request path, a newer size change replacing a pending restore (D2). Evidence: host tests with a stub agent that records the sizes it sees (SIGWINCH): one row fewer, then the full size, as two separate changes, a silent agent restored after 500 ms, two quick attaches ending at the latest size. Done 2026-10-27: `attaching_at_the_agents_own_size_still_makes_it_repaint` (the stub logs `29 100` then `30 100`, twice), `a_silent_agent_gets_its_size_back_after_a_redraw`, `quick_attaches_end_at_the_latest_size`. The first two fail on the old host.
- [x] 2.2 The harness against Claude Code: attach at the real size gets a full repaint. Evidence: `research/prototypes/attach-repro` run, repaint counts. Done 2026-10-27: Claude Code 2.1.311, 5 of 5 attaches at its own size repainted twice (at 11 ms and 28 ms), against 0 of 5 before (`research/evidence/fix-reattach-redraw/claude-attach-after-fix.log`).

## 3. On screen

- [ ] 3.1 The dev app with Claude Code sessions: switch between a working and an idle session, restart the app, switch back to the app, with and without the shell panel. Evidence: screenshots of the pane after each, no text from another size.

Expected Result: coming back to a session shows a correct screen without resizing.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it.
- [ ] 4.3 After the owner accepts: archive, push `fix/reattach-redraw`, open the PR, report its URL.

Expected Result: one PR with the fix.
