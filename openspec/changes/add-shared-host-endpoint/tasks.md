# Tasks

## 1. Build

- [x] 1.1 `ClientEndpoint` in `hq-remote`: key, runtime, endpoint bound once (D1). Evidence: host test
  `one_endpoint_links_two_hosts_at_once`: two real hosts, one endpoint, both feeds open together,
  closing one link leaves the other working, the closed one dials again.
- [x] 1.2 `Link` on the shared endpoint, closing only its connection (D2). Evidence: the existing
  remote-link host tests pass (`the_client_works_over_a_remote_link_and_dials_again`,
  `a_paired_desktop_works_on_several_streams_within_its_allow_list`).
- [x] 1.3 `hosts.rs` keeps every link; show, pair and forget as D3. Evidence: clippy and
  the app's tests; on screen (switching back without "Connecting…") waits for a free Mac.

Expected Result: every paired host stays connected; switching is instant.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` (639 tests); the Windows check runs on the pull request.
- [ ] 2.2 A host reachable only by relay, idle more than 60 s, used again (the spike's open
  point). Evidence in `research/evidence/several-hosts/`.
- [ ] 2.3 After the owner accepts: archive, push `feat/shared-host-endpoint`, open the PR.
