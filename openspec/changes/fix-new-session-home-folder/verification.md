# Verification: fix-new-session-home-folder

Branch `fix/new-session-home-folder` at 80ee787, 2026-10-31.

## Checks

- `./scripts/check` passed: 580 tests, lint, Android build, OpenSpec.
- New test `an_empty_field_browses_and_starts_in_the_home_folder` (Open here on the home folder,
  Enter starts there, Tab goes into a subfolder).

## Seen on screen

- ⌘N: the field is empty with "Home folder, or type a path or a folder name"; the list starts
  with `~` "Open here" and its subfolders; the footer reads "Claude Code in ~"
  (`new-session-empty.png`).
- Typing `~/api/` at once: the field reads `~/api/` with nothing to delete first.

## On screen (the owner freed the Mac, 2026-10-31)

One sandbox for all five changes (`/private/tmp/hqo`: synthetic `HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR`, relays off), the app and host built from the five branches merged together in a
scratch worktree, stand-in agents (`research/prototypes/background-status/standin.py`), a stand-in
`claude` answering the hidden `/usage` read with 7 % and 93 % or the trust question, the update
stand-in offering 0.1.21, and the stand-in phone. Captures in
`research/evidence/five-changes-2026-10-31/`. All five branches merge cleanly in pairs and
together; the merged tree lints clean.
