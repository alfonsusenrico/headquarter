# Verification

Revision: branch `fix/hide-gone-folders` (from mark-external-sessions-unaddable),
working tree with this change, 2026-10-23.

## Diagnosis evidence

- `hq ls --all --json` on the owner's machine: 98 conversations whose folder no longer
  exists (69 in temporary `af-eval-*` folders, 16 in development folders under `/tmp`,
  13 others), none in the sidebar.

## Checks

| Check | Result |
| --- | --- |
| `conversations_whose_folder_is_gone_are_not_offered` (`hq-app`) | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 376 tests, 17 OpenSpec items |

## Not verified

- On screen (task 1.2): waits for the owner.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 376 tests, 17 OpenSpec items (at 95840c0).
