# Verification: keep Claude Code sessions on their own conversation

Tested: branch `fix/claude-conversations` from `main` @ 33f0a08, working tree with the change.
2026-10-27.

## Commands

- `./scripts/check`: passed. Rust 517 tests, 0 failed, app and Android checks, OpenSpec 19 items,
  0 failed. Log: `research/evidence/claude-daemon-sessions/check.log`.
- New host tests: `a_resume_follows_the_conversation_claude_continued_in`,
  `merges_keep_the_owners_rows_and_drop_only_unnamed_copies`,
  `a_conversation_running_in_claudes_background_is_not_resumed`,
  `a_background_conversation_is_brought_into_headquarter`. The first three fail on the old code.
- New unit tests in `hq-signals`: two continuations (the one written last wins), a continued
  conversation offered once, the background flag and id from `claude agents --json`.

## Real agent

Claude Code 2.1.311, a throwaway `claude --bg` session: `claude stop` with the conversation id
answers "No job matching". With the background id (`id` field) it stops the session, and the
conversation stays (`research/evidence/claude-daemon-sessions/findings.md`).

## Not verified yet

- On screen: the background state and Bring into Headquarter, the Add sessions tag, one entry
  per chain. Checked with the owner on the real app (task 3.2).
- What removed the owner's sidebar row three times (task 2.3): not reproduced. The new
  `sidebar removed ids=…` lines show it if it happens again.
