# Verification: fix-codex-new-unread

Revision: `fix/codex-sessions`, on `main` a5d9ee4 (v0.3.0).

| Check | Result |
| --- | --- |
| Codex 0.166.0 `/new` captured (no prompt sent) | `working`, then `idle` (`research/evidence/codex-new/2026-11-09.md`) |
| `hq-signals` mapping and tracker tests | passed: `done` ends a turn for every agent, `idle` is a stop; a read Codex session stays idle after `/new` |
| Host test `a_codex_turn_ending_reads_codex_usage_again` (stand-in ends turns with `done`) | passed |
| `./scripts/check` | passed: 637 tests, OpenSpec 20/20 |
| A real Codex 0.166.0 turn, "Reply with just OK." (owner's go) | `working`, then `done` 14.6 s later |
