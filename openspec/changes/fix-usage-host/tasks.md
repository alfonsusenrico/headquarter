# Tasks

## 1. Build

- [x] 1.1 The footer's host by D1, as a pure function of the choice, whether this computer runs a
  host, the hosts in pairing order, and the hosts the app holds usage for. Evidence: unit tests for
  each rule, the chosen host away, and no host.
- [ ] 1.2 Rename today's `usage_host()` to `selected_host()` for New Session, groups and the Usage
  window (D4), and give the footer, rail, usage details and Refresh the new `usage_host()`.
  Evidence: `./scripts/check`, plus on screen (2.2) for New Session's default host.
- [x] 1.3 `usage_host` in `app-state.json` (D3): saved on choice, read at start, unknown values
  ignored, other keys kept, dropped on Forget. Evidence: unit tests on a temporary file.
- [ ] 1.4 The "Usage from" row in the usage details (D2): shown with two or more hosts, the shown
  host filled, a host without usage disabled. Evidence: on screen (2.2).

Expected Result: the footer stays on one host whatever is selected, and the owner can switch it.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` (2026-11-10: 655 tests, cargo-deny and OpenSpec strict pass); the Windows check runs on the pull request.
- [ ] 2.2 On screen (Mac free, owner's go), this Mac's host plus an isolated sandbox host paired to
  the app:
  - selecting a sandbox session leaves the footer on this Mac;
  - choosing the sandbox shows its rings named, and the choice survives reopening the app;
  - stopping the sandbox falls back to this Mac, and restarting it brings the sandbox back;
  - forgetting the sandbox clears the choice;
  - New Session still defaults to the selected session's host.
  The owner's server is the real check.
- [ ] 2.3 After the owner accepts: archive, push `fix/usage-host`, open the PR, report its URL.
