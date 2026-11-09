# Tasks

## 1. The list

- [x] 1.1 `HostId`, `session_hosts`, one feed per host with tagged events; a host's list replaces
  only its sessions; a feed's end greys its sessions (D1, D2). Evidence: clippy and the
  app's tests; the merge itself is the workspace's (GPUI) and is checked on screen (4.2); the
  link layer for two hosts is `one_endpoint_links_two_hosts_at_once` (step 1).
- [x] 1.2 Routing: `host_call`, terminals, shell panels, details, Add sessions by the session's
  host (D4). Evidence: every call names its host (`host_call(host, …)`); on screen (4.2).
- [x] 1.3 Labels (D7). Evidence: `another_computers_rows_carry_its_name_home_and_state`.

## 2. Groups

- [x] 2.1 The merged view and display ids (D3). Evidence: `merged.rs` tests (order, merge by name,
  ids that clash, colour filled in, display ids kept across merges).
- [x] 2.2 Group actions fan out or make the group on the session's host (D3). Evidence: unit tests
  of the action plan; host test with two hosts.

## 3. Hosts, usage, new sessions, notifications

- [x] 3.1 The Hosts menu with Forget confirmed in place (D6). Evidence: on screen.
- [x] 3.2 Usage per host and the footer; the Usage window measures the same host (D5). Evidence:
  on screen.
- [x] 3.3 New Session's host choice and folder browser (D4). Evidence: on screen.
- [x] 3.4 Notifications from every host, named (D2). Evidence: `a_notice_names_another_computer`.

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` (643 tests); the Windows check runs on the pull request.
- [ ] 4.2 On screen (Mac free, owner's go): this Mac plus a sandbox host: one list with labels on
  the sandbox's sessions only, a merged group and a session moved in, a notification from the
  sandbox, the sandbox stopped (greyed) and back, Forget's confirmation, New Session on the sandbox.
- [ ] 4.3 The owner's server and Windows PC.
- [ ] 4.4 After the owner accepts (and step 1 is merged): archive, push `feat/merged-host-list`,
  open the PR.
