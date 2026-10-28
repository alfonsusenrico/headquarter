# Tasks

## 1. Host and protocol

- [ ] 1.1 `InUse` response and `Device` request, protocol 14 (D1, D2, D5). Evidence: `hq-protocol` round-trip tests, the version test.
- [ ] 1.2 The host sends `InUse` on attach and on every change, for sessions and shells (D1). Evidence: `hq-daemon` unit tests (attach, typing, `Focus {}`, the client in use leaving, a shell), an integration test with two clients.
- [ ] 1.3 The proxy names the phone; `allowed()` refuses `Device` from a phone (D2). Evidence: remote tests with `HQ_REMOTE_RELAYS=none` (named holder, refused self-naming, connection kept).

Expected Result: every client knows who is in use.

## 2. Desktop

- [ ] 2.1 The bar over the terminal pane and the shell panel, clicking it takes over (D4). Evidence: unit test of the bar's text per holder, theme contrast test, on screen with a synthetic host and a test phone connection (bar shown, clicked, gone).

Expected Result: the desktop says when the phone has the session.

## 3. Phone

- [ ] 3.1 The cover with "Use here", typing and keys still take over (D3). Evidence: `hq-mobile` test of the holder state, on the dev phone with the synthetic host (cover shown, "Use here", typing), Android lint and ktlint.

Expected Result: the phone never shows a garbled terminal.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes. `verification.md` written, including the owner's two original sequences.
- [ ] 4.2 The owner checks it on the Mac and the phone.
- [ ] 4.3 After the owner accepts: archive, push `fix/terminal-in-use-elsewhere`, open the PR, report its URL.

Expected Result: one PR with the fix.
