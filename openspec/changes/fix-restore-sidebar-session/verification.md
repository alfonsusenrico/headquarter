# Verification

Revision: branch `fix/restore-sidebar-session` from `main` c858c9d, working tree with
this change, 2026-10-22.

## Diagnosis evidence

- `app-state.json` kept that session as `shown_session`; the host reported its row
  with `in_sidebar: false`, running in another terminal, and no session in the
  sidebar, matching the owner's screenshot.

## Checks

| Check | Result |
| --- | --- |
| `a_kept_session_is_shown_only_while_in_the_sidebar` (`hq-app`) | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 373 tests, 15 OpenSpec items |

## Not verified

- On screen (task 1.2): waits for the owner's test build. The unit test covers the
  new rule; the change in `show_kept_session` is the one call that uses it.
