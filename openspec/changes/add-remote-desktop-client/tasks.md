# Tasks

## 1. Host

- [x] 1.1 Device kind in `Pair`, store migration, kind in devices lists (D1). Evidence: store migration test; host test pairing a phone without `kind` and a desktop with it.
- [x] 1.2 Desktop allow-list (D2). Evidence: host tests sending every refused request from a desktop and from a phone; allowed ones pass.
- [x] 1.3 Several streams per connection, the 64 cap, removal closing all streams (D3). Evidence: host tests.
- [x] 1.4 `ListFolder` (D7). Evidence: host tests (home, hidden left out, missing folder, refused from a phone).

Expected result: a test client paired as a desktop runs a full app session over the gateway.
Done: host tests `a_paired_desktop_works_on_several_streams_within_its_allow_list`, `a_desktop_has_at_most_64_streams`, `a_desktop_lists_the_hosts_folders_and_a_phone_may_not`; unit tests for the allow-list, device kind and stored kind.

## 2. Client link

- [ ] 2.1 `hq_remote::client::Link` with reconnect (D4). Evidence: tests against a test host with `HQ_REMOTE_RELAYS=none` (open, drop, reconnect, host restart).
- [ ] 2.2 `hq-client` transports (D4). Evidence: existing client tests pass on the local socket; new ones over a link.

Expected result: `hq-client` calls work the same over a local socket and a remote link.

## 3. App

- [ ] 3.1 Client key and `hosts.json` (D5). Evidence: unit tests (mode 600, unknown fields kept).
- [ ] 3.2 Connect to a Host window and the host switcher (D6). Evidence: on screen against a sandboxed host.
- [ ] 3.3 Folder browser through `ListFolder`; setup only for a local host; Usage names hosts (D7). Evidence: unit tests; on screen.
- [ ] 3.4 Local-only quit, update, Remote Control window, file drops (D8). Evidence: on screen.
- [ ] 3.5 Protocol 15 and the mismatch message (D9). Evidence: unit test of the message.

Expected result: on this Mac, the app shows a sandboxed second host as a remote host, starts, types into and closes sessions there, survives a network drop, and quits without stopping it.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes; on screen with a sandboxed host paired as remote; keystroke round trip measured direct and relayed. `verification.md` written.
- [ ] 4.2 The owner checks it.
- [ ] 4.3 After the owner accepts: archive, push `feat/remote-desktop-client`, open the PR, report its URL.
