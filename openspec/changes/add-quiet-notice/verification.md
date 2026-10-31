# Verification: add-quiet-notice

Branch `feat/quiet-notice` at 9b297df, 2026-10-31.

## Checks

- `./scripts/check` passed: 581 tests, lint, Android build, OpenSpec.
- New tests: an informational notice leaves after 5 s unless the pointer is on it; a failure stays.

## Seen on screen

- Check for Updates… with 0.1.21 offered: "Headquarter 0.1.21 is the latest version." at the bottom
  right with a close button, the pane uncovered (`notice-latest.png`); gone 6.3 s later
  (`notice-gone-after-6s.png`).
- With the server down: "Couldn't check for updates: curl: (7) …" with an amber border, still there
  after 8 s (`notice-error-stays.png`); the close button removed it.

## Not verified

- The slide itself frame by frame (200 ms; the first capture at 0.12 s showed it nearly in place),
  and reduced motion.

## On screen (the owner freed the Mac, 2026-10-31)

One sandbox for all five changes (`/private/tmp/hqo`: synthetic `HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR`, relays off), the app and host built from the five branches merged together in a
scratch worktree, stand-in agents (`research/prototypes/background-status/standin.py`), a stand-in
`claude` answering the hidden `/usage` read with 7 % and 93 % or the trust question, the update
stand-in offering 0.1.21, and the stand-in phone. Captures in
`research/evidence/five-changes-2026-10-31/`. All five branches merge cleanly in pairs and
together; the merged tree lints clean.
