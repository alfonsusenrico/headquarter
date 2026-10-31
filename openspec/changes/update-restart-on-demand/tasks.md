# Tasks

## 1. Build

- [x] 1.1 `Installed` phase, no automatic restart, window text (D1). Evidence: unit tests for the phase rules.
- [ ] 1.2 Sidebar "Update Now" with the confirmation and Later (D2). Evidence: on screen with the update stand-in (`STANDIN_SUCCEED=1`) and stand-in agents: Wait installs and does not restart when the sessions go idle; Update Now asks while busy; Later closes.

Expected Result: the restart happens only when the owner presses it.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes; on screen (owner's go for the Mac). `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/update-restart-on-demand`, open the PR, report its URL.

Expected Result: one PR.
