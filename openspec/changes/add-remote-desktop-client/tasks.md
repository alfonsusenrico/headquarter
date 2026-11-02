# Tasks

## 1. Host

- [x] 1.1 Device kind in `Pair`, store migration, kind in devices lists (D1). Evidence: store migration test; host test pairing a phone without `kind` and a desktop with it.
- [x] 1.2 Desktop allow-list (D2). Evidence: host tests sending every refused request from a desktop and from a phone; allowed ones pass.
- [x] 1.3 Several streams per connection, the 64 cap, removal closing all streams (D3). Evidence: host tests.
- [x] 1.4 `ListFolder` (D7). Evidence: host tests (home, hidden left out, missing folder, refused from a phone).

Expected result: a test client paired as a desktop runs a full app session over the gateway.
Done: host tests `a_paired_desktop_works_on_several_streams_within_its_allow_list`, `a_desktop_has_at_most_64_streams`, `a_desktop_lists_the_hosts_folders_and_a_phone_may_not`; unit tests for the allow-list, device kind and stored kind.

## 2. Client link

- [x] 2.1 `hq_remote::client::Link` with reconnect (D4). Evidence: tests against a test host with `HQ_REMOTE_RELAYS=none` (open, drop, reconnect, host restart).
- [x] 2.2 `hq-client` transports (D4). Evidence: existing client tests pass on the local socket; new ones over a link.

Expected result: `hq-client` calls work the same over a local socket and a remote link.
Done: host test `the_client_works_over_a_remote_link_and_dials_again` (pair, feed, create, rename, attach and type, dial again after a drop, removal); `hq-remote` unit tests for the key and the address; every existing client test passes on the local socket.

## 3. App

- [x] 3.1 Client key and `hosts.json` (D5). Evidence: unit tests (mode 600, unknown fields kept).
- [x] 3.2 Connect to a Host window and the host switcher (D6). Evidence: on screen against a sandboxed host.
- [x] 3.3 Folder browser through `ListFolder`; setup only for a local host; Usage names hosts (D7). Evidence: unit tests; on screen.
- [x] 3.4 Local-only quit, update, Remote Control window, file drops (D8). Evidence: on screen (quit, Remote Control); update and file drops by code path only, see `verification.md`.
- [x] 3.5 Protocol 15 and the mismatch message (D9). Evidence: unit test of the message.

Expected result: on this Mac, the app shows a sandboxed second host as a remote host, starts, types into and closes sessions there, survives a network drop, and quits without stopping it.
Done: on screen 2026-11-02 against sandboxed hosts (`verification.md`). Found and fixed there: a desktop saw its own session as in use elsewhere (host test extended); the pane said "Starting Headquarter…" for a remote host; each dial bound a new endpoint with the same key (relay conflict) and dropped failed ones uncleanly; final link errors were never recognised inside `io::Error`, so a removed computer retried forever (unit tests); first-run setup could start with a remote host on screen (unit test).

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes; on screen with a sandboxed host paired as remote; keystroke round trip measured direct and relayed. `verification.md` written.
- [ ] 4.2 The owner checks it.
- [ ] 4.3 After the owner accepts: archive, push `feat/remote-desktop-client`, open the PR, report its URL.
