# Tasks

## 1. Host

- [x] 1.1 Read `continued-in` records, taken from branch `fix/claude-continued-conversation` with its tests. Evidence: unit tests (one record, two records, a cycle, a missing transcript). Done 2026-10-27: returns every continuation, `a_claude_conversation_resumes_where_it_was_carried_on`, `of_two_continuations_the_one_written_last_wins`, `continuations_never_loop_or_leave_the_projects_folder`.
- [x] 1.2 Follow the continuation while running and record it, with a fallback at resume by newest write (D1). Evidence: host test with a stub that writes a `continued-in` record mid-session: the index follows, and a restart resumes the new id. Done 2026-10-27: adopted on the first report from the continued id (checked at most every 5 s), fallback at resume. `a_resume_follows_the_conversation_claude_continued_in`, and the adoption inside `merges_keep_the_owners_rows_and_drop_only_unnamed_copies`.
- [x] 1.3 Find how to end a background Claude session from outside, on a throwaway background session (D3). Evidence: the experiment in `research/evidence/claude-daemon-sessions/`. Done 2026-10-27: `claude stop <background id>` works, the conversation id does not (`research/evidence/claude-daemon-sessions/findings.md`).
- [x] 1.4 The background check before resume and the new error, protocol 12 (D3). Evidence: host test with a stub `claude agents --json` reporting the conversation as background: the resume answers RunningInBackground and starts nothing. Done 2026-10-27: `a_conversation_running_in_claudes_background_is_not_resumed`, and BringIn: `a_background_conversation_is_brought_into_headquarter` (stopped by `bg42`, then resumed).
- [x] 1.5 Merges keep the sidebar or named row (D2). Evidence: host test of the 2026-10-27 case: the named sidebar row survives, the unnamed history copy goes. Done 2026-10-27: `record_conversation` replaces `drop_imported_duplicate` for every id change, `merges_keep_the_owners_rows_and_drop_only_unnamed_copies`.
- [x] 1.6 Logging of Add, Remove, Close and merges (D5). Evidence: host test reading the log lines (ids only). Done 2026-10-27: `sidebar added/removed ids=…`, merge lines, `conversation still running in background id=…`, checked in the merges test.

## 2. App

- [ ] 2.1 The background state and "Bring into Headquarter" (D3). Evidence: the dev app with a stub reporting a background conversation. Built 2026-10-27: the pane's background state and Bring into Headquarter, the Add sessions tag. On screen with the owner's real case (3.2).
- [x] 2.2 One entry per chain in Add sessions (D4). Evidence: unit test, the dev app with a three-link chain. Done 2026-10-27: a conversation continued under a saved id is no longer offered, `a_continued_conversation_is_offered_as_its_newest_only`. On screen with 3.2.
- [ ] 2.3 Reproduce the vanishing row with the new logging, and fix its cause if it is in Headquarter. Evidence: the log lines, or a written note of what removed it. Logging in place 2026-10-27. The removal was not reproduced: the owner's next real use shows the lines if it happens again.

Expected Result: a Claude session always comes back as itself, or says plainly why not.

## 3. Verification and delivery

- [x] 3.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-27.
- [ ] 3.2 The owner checks it on the real app.
- [ ] 3.3 After the owner accepts: archive, push `fix/claude-conversations`, open the PR, report its URL. Delete the other agent's branch and worktree with the owner's go.
