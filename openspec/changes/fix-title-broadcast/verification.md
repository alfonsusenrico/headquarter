# Verification

Revision: branch `fix/broadcast-title-changes` from `main` c858c9d, working tree with
this change, 2026-10-22.

## Diagnosis evidence

- Claude Code transcripts still carry `ai-title` and `agent-name` records (types only
  were counted, in the 40 newest transcripts); a conversation shown by its generated name has one.
- The running v0.1.0 host reported a title for that conversation and for 138 of 203 history
  rows, while the app showed its generated name.

## Checks

| Check | Result |
| --- | --- |
| `a_title_found_later_reaches_subscribers` without the title broadcast | failed: "the new title is reported" after 10 s |
| same test with the fix | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 373 tests, 15 OpenSpec items |

## Not verified

- On screen with the owner's history (task 1.2): waits for the owner's test build.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 376 tests, 16 OpenSpec items (at 2c25807).
