# Tasks

## 1. Build

- [x] 1.1 Codex `idle` is a stop; only `done` ends a turn (D1). Evidence: `hq-signals`
  `only_done_ends_a_turn_and_idle_is_a_stop_for_every_agent`,
  `codex_new_after_a_read_turn_leaves_nothing_unread`; host test
  `a_codex_turn_ending_reads_codex_usage_again` with the stand-in ending turns with `done`.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes; `verification.md` written.
- [x] 2.2 One real Codex 0.166.0 turn ("Reply with just OK.") ends with `done`, with the owner's go.
  Evidence: `research/evidence/codex-new/2026-11-09.md` (working, done 14.6 s later).
- [ ] 2.3 After the owner accepts: archive, push `fix/codex-sessions`, open the PR, report its URL.
