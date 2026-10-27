# Tasks

## 1. Host

- [ ] 1.1 Read `continued-in` records, taken from branch `fix/claude-continued-conversation` with its tests. Evidence: unit tests (one record, two records, a cycle, a missing transcript).
- [ ] 1.2 Follow the continuation while running and record it, with a fallback at resume by newest write (D1). Evidence: host test with a stub that writes a `continued-in` record mid-session: the index follows, and a restart resumes the new id.
- [ ] 1.3 Find how to end a background Claude session from outside, on a throwaway background session (D3). Evidence: the experiment in `research/evidence/claude-daemon-sessions/`.
- [ ] 1.4 The background check before resume and the new error, protocol 12 (D3). Evidence: host test with a stub `claude agents --json` reporting the conversation as background: the resume answers RunningInBackground and starts nothing.
- [ ] 1.5 Merges keep the sidebar or named row (D2). Evidence: host test of the 2026-10-27 case: the named sidebar row survives, the unnamed history copy goes.
- [ ] 1.6 Logging of Add, Remove, Close and merges (D5). Evidence: host test reading the log lines (ids only).

## 2. App

- [ ] 2.1 The background state and "Bring into Headquarter" (D3). Evidence: the dev app with a stub reporting a background conversation.
- [ ] 2.2 One entry per chain in Add sessions (D4). Evidence: unit test, the dev app with a three-link chain.
- [ ] 2.3 Reproduce the vanishing row with the new logging, and fix its cause if it is in Headquarter. Evidence: the log lines, or a written note of what removed it.

Expected Result: a Claude session always comes back as itself, or says plainly why not.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check` passes. `verification.md` written.
- [ ] 3.2 The owner checks it on the real app.
- [ ] 3.3 After the owner accepts: archive, push `fix/claude-conversations`, open the PR, report its URL. Delete the other agent's branch and worktree with the owner's go.
