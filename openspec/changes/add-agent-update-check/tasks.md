# Tasks

## 1. Build

- [ ] 1.1 Version memory and the trigger (D1). Evidence: unit tests (new version, same version, first run).
- [ ] 1.2 Status-at-start and usage checks with stand-ins (D2). Evidence: host tests with stand-ins that do and do not report OSC 7501.
- [ ] 1.3 Notice for a failed check, once per version (D3). Evidence: on screen with a stand-in whose usage fails.
- [ ] 1.4 `scripts/agent-check` (D4). Evidence: a run against the installed agents (free checks); `--turn` on the owner's go.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes; on screen. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/agent-update-check`, open the PR, report its URL.
