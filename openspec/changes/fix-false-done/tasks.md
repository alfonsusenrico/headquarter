# Tasks

## 1. Diagnosis

- [x] 1.1 Reproduce from the owner's data: what started the next Claude turn after each turn end, and the 2026-10-28 update restart. Evidence: `research/evidence/false-done/findings.md` (shape only, no text).

Expected Result: the cause is known and recorded.

## 2. Build

- [ ] 2.1 Desktop: pending "replied" notices, posted after 10 quiet seconds without background work, dropped on work, needs you, read or leaving the sidebar (D1, D2). Evidence: unit tests with a fake clock for each case.
- [ ] 2.2 Phone home: unread replies with background work stay in their group (D3). Evidence: `hq-mobile` mirror test.
- [ ] 2.3 Updater: background work and plans to review are busy (D4). Evidence: `update::busy` test.

Expected Result: no "replied" while work goes on, and Wait waits for it.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check` passes. On screen with the stand-ins: a background agent holds the notification and the restart, then both follow when it ends (with the owner's go for the Mac). `verification.md` written.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `fix/false-done`, open the PR, report its URL.

Expected Result: one PR with the fix.
