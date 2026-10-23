# Verification

Revision: branch `fix/feed-subscribe-first` from `main` c858c9d, working tree with this
change, 2026-10-22.

## Diagnosis evidence

- Installed v0.1.0 host log: host started at 1792659383373; the Codex usage reading
  finished at 1792659386313, so the app's `Usage` request, sent between its list and
  its subscription, waited about 3 s.
- Claude Code's records listed four sessions: two busy, two idle. The busy two were
  shown as in another terminal at first; the idle two were not. `hq ls --all --json`
  later showed all four linked, and resuming one was refused as running in another
  terminal, so the host had the link and the app did not.

## Checks

| Check | Result |
| --- | --- |
| `cargo test -p hq-client --lib` with the old order (list, usage, subscribe) | failed: "expected the link, got None" |
| same test with the fix | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 373 tests, 15 OpenSpec items |

## Not verified

- On screen with real sessions in other terminals (task 1.2): waits for the owner's
  test build.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 373 tests, 15 OpenSpec items (at 694a634).
