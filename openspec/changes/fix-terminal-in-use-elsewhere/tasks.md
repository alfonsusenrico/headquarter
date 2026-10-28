# Tasks

## 1. Host and protocol

- [x] 1.1 `InUse` response and `Device` request, protocol 14 (D1, D2, D5). Evidence: `hq-protocol` round-trip test `remote_messages_round_trip`.
- [x] 1.2 The host sends `InUse` on attach and on every change, for sessions and shells (D1). Evidence: `hq-cli` host test `the_terminal_is_sized_for_the_device_in_use` (attach, `Focus {}`, typing, the client in use leaving, each client's own side); shells on screen (verification.md).
- [x] 1.3 The proxy names the phone; `allowed()` refuses `Device` from a phone (D2). Evidence: `a_paired_phone_follows_types_and_is_held_to_the_allow_list` (named holder, refused self-naming, connection kept, name unchanged), `the_allow_list_follows_the_owner`.

Expected Result: every client knows who is in use.

## 2. Desktop

- [x] 2.1 The notice over the terminal pane and the shell panel, clicking it takes over (D4). Evidence: `the_pane_names_the_client_in_use_elsewhere`, on screen with a synthetic host and a stand-in phone on the real phone core (verification.md).

Expected Result: the desktop says when the phone has the session.

## 3. Phone

- [x] 3.1 The core keeps the holder and never takes `InUse` for an answer (D3). Evidence: `the_open_terminal_says_where_it_is_in_use`, and the stand-in phone end to end (verification.md).
- [x] 3.2 The cover with "Use here" on the Android app; typing and keys still take over (D3). Evidence: Android Lint, ktlint and builds pass; on the dev phone with the debug app (verification.md).

Expected Result: the phone never shows a garbled terminal.

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes. `verification.md` written, including the owner's two original sequences.
- [ ] 4.2 The owner checks it on the Mac and the phone.
- [ ] 4.3 After the owner accepts: archive, push `fix/terminal-in-use-elsewhere`, open the PR, report its URL.

Expected Result: one PR with the fix.
