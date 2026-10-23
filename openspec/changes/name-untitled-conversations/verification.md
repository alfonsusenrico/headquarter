# Verification

Revision: branch `feat/untitled-conversations` (from offer-continuable-sessions),
working tree with this change, 2026-10-23.

## Evidence

- Owner's machine: 5 Claude Code transcripts with a prompt and a reply but no
  `ai-title` or `agent-name` record (record types counted only).

## Checks

| Check | Result |
| --- | --- |
| `a_past_conversation_without_a_title_is_untitled_with_its_id` (`hq-protocol`) | passed |
| Add sessions tests (`hq-app`) | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 383 tests, 22 OpenSpec items |

## Not verified

- On screen (task 1.2): waits for the owner.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 383 tests, 22 OpenSpec items (at aca8788).
