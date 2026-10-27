# Tasks

## 1. Host

- [x] 1.1 `Hosted` keeps the Claude conversation id. Hook and status-line payloads from another conversation are ignored, and `SessionStart` `clear` or an unknown id is adopted (D1, D2). Evidence: host integration tests with a stub agent sending hooks under one `HQ_SESSION` with its own and a foreign conversation id. The foreign "waiting" event leaves the status working, a `clear` switches the id, the foreign status line does not change the details. Done 2026-10-27: `a_report_from_another_claude_conversation_leaves_the_session_alone` and `a_claude_session_follows_the_conversation_its_agent_starts` (resume with a new id, `clear`, the old id then ignored). Both fail on the unfixed host.
- [x] 1.2 The rate-limited log line (D3). Evidence: the foreign-report test counts one log line for two foreign reports. Done 2026-10-27.
- [x] 1.3 On the owner's machine: run the dev host with a real Claude Code session and confirm its own reports still count. Evidence: the log line count and the status over a turn. Done 2026-10-27: Claude Code 2.1.311 on a dev host, one prompt, then kill and resume (same id kept): status followed, 0 foreign lines, id unchanged. The daemon's foreign session is only on the owner's running host, so its case rests on the tests.

Expected Result: a hidden Claude Code session can no longer mark another session as needing the owner.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-27.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/hook-session-match`, open the PR, report its URL.

Expected Result: one PR with the fix.
