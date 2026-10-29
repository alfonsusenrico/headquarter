# Tasks

## 1. Diagnosis

- [x] 1.1 Why the drag does nothing: Claude Code 2.1.312's modes and the phone's scroll path. Evidence: `research/evidence/phone-scroll/findings.md`.

Expected Result: the cause is known.

## 2. Build

- [ ] 2.1 Core: `Terminal::scroll` with the three paths and both mouse encodings (D1). Evidence: `hq-mobile` tests per mode, cap and direction.
- [ ] 2.2 Android: the drag's cell passed to the core (D2). Evidence: ktlint, Android Lint, build.

Expected Result: the drag reaches full-screen agents.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check` passes. A stand-in phone scroll against a real Claude Code session on a test host sends wheel reports; on the dev phone with the owner's go. `verification.md` written.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `fix/phone-scroll`, open the PR, report its URL.

Expected Result: one PR with the fix.
