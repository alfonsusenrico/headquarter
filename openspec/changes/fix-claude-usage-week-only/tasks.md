# Tasks

## 1. Diagnosis

- [x] 1.1 Why reads fail since 2026-10-31. Evidence: `research/evidence/claude-usage-week-only/findings.md`.

## 2. Build

- [ ] 2.1 Parser accepts week-only screens (D1). Evidence: test with the 2.1.316 screen.
- [ ] 2.2 No standing five-hour window for Claude (D2). Evidence: usage rows test.

Expected Result: Claude usage updates on 2.1.316.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check` passes; the hidden read against the real Claude Code 2.1.316 on a test host succeeds (host log `ok=true`). `verification.md` written.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `fix/claude-usage-week-only`, open the PR, report its URL.
