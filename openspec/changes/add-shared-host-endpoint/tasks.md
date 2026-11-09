# Tasks

## 1. Build

- [ ] 1.1 `ClientEndpoint` in `hq-remote`: key, runtime, endpoint bound once (D1). Evidence: tests
  with two hosts and relays off: two links on one endpoint exchange frames at once; closing one
  leaves the other working.
- [ ] 1.2 `Link` on the shared endpoint, closing only its connection (D2). Evidence: the existing
  remote-link host tests pass unchanged.
- [ ] 1.3 `hosts.rs` keeps every link; show, pair and forget as D3. Evidence: unit tests of the
  link book; on screen with this Mac and a sandbox host: switching back shows no "Connecting…".

Expected Result: every paired host stays connected; switching is instant.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check`; the Windows check builds the client crates.
- [ ] 2.2 A host reachable only by relay, idle more than 60 s, used again (the spike's open
  point). Evidence in `research/evidence/several-hosts/`.
- [ ] 2.3 After the owner accepts: archive, push `feat/shared-host-endpoint`, open the PR.
