# Tasks

## 1. Diagnosis

- [x] 1.1 Why Claude's reads failed and Refresh did nothing. Evidence: `research/evidence/claude-usage-folder/findings.md`.

Expected Result: the cause is known.

## 2. Build

- [x] 2.1 Trusted folder from `~/.claude.json`, with the fallback (D1). Evidence: unit tests with fixture files (trusted, untrusted home, missing, malformed).
- [x] 2.2 Refresh after a failure reads again (D2). Evidence: host test with a stub Claude that asks to trust its folder.
- [x] 2.3 Message (D3). Evidence: test.

Expected Result: usage updates while the owner works in the home folder.

## 3. Verification and delivery

- [x] 3.1 `./scripts/check` passes. A test host with a stub Claude: a home-folder session does not break the reading. `verification.md` written.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `fix/claude-usage-folder`, open the PR, report its URL.

Expected Result: one PR with the fix.
