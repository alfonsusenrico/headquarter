# Tasks

## 1. The list

- [ ] 1.1 `HostId`, `session_hosts`, one feed per host with tagged events; a host's list replaces
  only its sessions; a feed's end greys its sessions (D1, D2). Evidence: unit tests of the merge
  (two hosts' lists, one host's update, one host reconnecting).
- [ ] 1.2 Routing: `host_call`, terminals, shell panels, details, Add sessions by the session's
  host (D4). Evidence: host test with two sandbox hosts through `hq-client` (rename and attach on
  each).
- [ ] 1.3 Labels (D7). Evidence: sidebar model tests (with and without a host here).

## 2. Groups

- [ ] 2.1 The merged view and display ids (D3). Evidence: unit tests (order, merge by name, a
  session's group mapped).
- [ ] 2.2 Group actions fan out or make the group on the session's host (D3). Evidence: unit tests
  of the action plan; host test with two hosts.

## 3. Hosts, usage, new sessions, notifications

- [ ] 3.1 The Hosts menu with Forget confirmed in place (D6). Evidence: on screen.
- [ ] 3.2 Usage per host and the footer (D5). Evidence: unit tests; on screen.
- [ ] 3.3 New Session's host choice and folder browser (D4). Evidence: on screen.
- [ ] 3.4 Notifications from every host, named (D2). Evidence: unit tests of the notice text.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check`; the Windows check.
- [ ] 4.2 On screen (Mac free, owner's go): this Mac plus a sandbox host: one list with labels on
  the sandbox's sessions only, a merged group and a session moved in, a notification from the
  sandbox, the sandbox stopped (greyed) and back, Forget's confirmation, New Session on the sandbox.
- [ ] 4.3 The owner's server and Windows PC.
- [ ] 4.4 After the owner accepts (and step 1 is merged): archive, push `feat/merged-host-list`,
  open the PR.
