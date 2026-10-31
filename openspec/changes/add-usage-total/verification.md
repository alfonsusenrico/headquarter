# Verification: add-usage-total

Branch `feat/usage-total` at 6049c7d, 2026-10-31.

## Checks

- `./scripts/check` passed: 580 tests, lint, Android build, OpenSpec.
- New test `the_total_adds_headquarter_and_every_running_session` (sums, a stopped session adds
  nothing, labels).

## Seen on screen

- ⌘U: the five busiest, then Headquarter 8.5 % 139 MB, All 4 sessions 0.1 % 35 MB, Total 8.7 %
  174 MB (`usage-popover.png`). Each figure is rounded on its own, so the shown CPU sum can differ
  by 0.1.
- Show all: the table, then the HEADQUARTER, ALL SESSIONS and TOTAL card (`usage-window.png`).

## On screen (the owner freed the Mac, 2026-10-31)

One sandbox for all five changes (`/private/tmp/hqo`: synthetic `HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR`, relays off), the app and host built from the five branches merged together in a
scratch worktree, stand-in agents (`research/prototypes/background-status/standin.py`), a stand-in
`claude` answering the hidden `/usage` read with 7 % and 93 % or the trust question, the update
stand-in offering 0.1.21, and the stand-in phone. Captures in
`research/evidence/five-changes-2026-10-31/`. All five branches merge cleanly in pairs and
together; the merged tree lints clean.
