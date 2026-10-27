# Tasks

## 1. App

- [ ] 1.1 Attach on the first layout at the pane's size, 80 x 24 after 500 ms without one, for the session and the shell panel (D1). Evidence: unit test that no attach starts before a size and that the first size is used, the dev app attaching (host log shows no 80 x 24 resize).

## 2. Host

- [ ] 2.1 Redraw: shrink, restore after the agent's next output or 500 ms, off the request path, a newer redraw replacing a pending one (D2). Evidence: host tests with a stub agent that records the sizes it sees (SIGWINCH): one row fewer, then the full size, as two separate changes, a silent agent restored after 500 ms, two quick attaches ending at the latest size.
- [ ] 2.2 The harness against Claude Code: attach at the real size gets a full repaint. Evidence: `research/prototypes/attach-repro` run, repaint counts.

## 3. On screen

- [ ] 3.1 The dev app with Claude Code sessions: switch between a working and an idle session, restart the app, switch back to the app, with and without the shell panel. Evidence: screenshots of the pane after each, no text from another size.

Expected Result: coming back to a session shows a correct screen without resizing.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it.
- [ ] 4.3 After the owner accepts: archive, push `fix/reattach-redraw`, open the PR, report its URL.

Expected Result: one PR with the fix.
