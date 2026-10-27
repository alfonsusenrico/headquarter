# Tasks

## 1. Host

- [ ] 1.1 `Resources` request and response, protocol 12, refused for phones (D1, D5). Evidence: protocol round-trip test, `allowed()` test.
- [ ] 1.2 Sampling a session's processes and descendants, CPU per core on the second sample (D1). Evidence: host test with a stub agent that starts a child process and burns CPU: the child is counted, CPU rises above 50% on the second sample, a stopped session has no entry.

## 2. App

- [ ] 2.1 The popover on ⌘U / Ctrl+Shift+U with the top five and Show all (D4). Evidence: unit test of the top-five order, the dev app on screen.
- [ ] 2.2 The Usage window: the Headquarter card, All agents, the sortable table, not running rows, child-process notes, and Window ▸ Usage on macOS (D3, D4). Evidence: unit tests of sorting and totals, the dev app on screen in both themes.
- [ ] 2.3 Measuring only while shown (D2). Evidence: the host log or a counter shows no requests with both closed.

Expected Result: the owner sees what each session and Headquarter itself cost.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check` passes. `verification.md` written.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `feat/usage-window`, open the PR, report its URL.

Expected Result: one PR with the Usage window.
