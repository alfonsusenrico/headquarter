# Verification: refine-background-mark-and-ring

Branch `feat/background-mark-and-ring` at 36b0188, 2026-10-31.

## Checks

- `./scripts/check` passed: 579 tests, lint, Android build, OpenSpec.
- Contrast test: the old near-limit ring keeps 3:1 against the sidebar and the track in the light,
  dark and Catppuccin themes (the light one needed a step darker, made by `readable`).
- Model test: background rows keep the glyph cell for the mark.

## Seen on screen

- Dark: "background" in teal with the stacked layers in the glyph cell, beside working (moving
  dots) and review (diamond) (`background-mark.png`); light (`background-mark-light.png`).
- The week at 93 % fresh: full red (`ring-fresh.png`). After a failed read ("Couldn't update since
  1:05 PM"): a pale red, not grey (`ring-old-reading.png`); light: a muted brick red
  (`ring-old-reading-light.png`).

## On screen (the owner freed the Mac, 2026-10-31)

One sandbox for all five changes (`/private/tmp/hqo`: synthetic `HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR`, relays off), the app and host built from the five branches merged together in a
scratch worktree, stand-in agents (`research/prototypes/background-status/standin.py`), a stand-in
`claude` answering the hidden `/usage` read with 7 % and 93 % or the trust question, the update
stand-in offering 0.1.21, and the stand-in phone. Captures in
`research/evidence/five-changes-2026-10-31/`. All five branches merge cleanly in pairs and
together; the merged tree lints clean.
