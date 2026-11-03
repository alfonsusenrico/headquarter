# Tasks

## 1. Build

- [x] 1.1 Version memory and the trigger (D1). Evidence: unit tests (fingerprint, version number); host test `a_new_agent_version_is_checked_once_and_a_failure_is_reported` (first run, new version, same version after a restart).
- [x] 1.2 Status-at-start and usage checks with stand-ins (D2). Evidence: unit tests with stand-ins that report, report while asking about trust, send only a child report, or exit silent; the host test; the free checks against the installed agents.
- [ ] 1.3 Notice for a failed check, once per version (D3). Evidence: on screen with a stand-in whose usage fails.
- [ ] 1.4 `hq agents --check` and `scripts/agent-check` (D4). Evidence: a run against the installed agents (free checks, done 2026-11-03: all ok); `--turn` on the owner's go.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes; on screen. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/agent-update-check`, open the PR, report its URL.
