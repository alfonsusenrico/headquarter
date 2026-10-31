# Verification: remote-follows-paired-phones

Branch `feat/remote-follows-phones` at 5cf3570, 2026-10-31.

## Checks

- `./scripts/check` passed: 576 tests, lint, Android build, OpenSpec.
- Host tests: no phone, no endpoint; a shown code starts it and an unused code stops it; the last
  phone removed stops it and the phone is told it was removed; a host with a paired phone starts the
  gateway, an older `enabled = true` alone does not; `hq remote on` is gone and the settings file
  is left alone; the allow-list test keeps a second phone paired to test refusal; an idle host
  exits once the last phone is removed.

## Seen on screen

- Found and fixed on screen: with remote control off the window had no Pair a Phone button (the
  pairing section was shown only while running). Fixed in 5cf3570; the check above includes it.
- No phone: "Remote control is off · Pair a phone to turn it on", the app's QR code in full, Pair a
  Phone, no switch (`remote-off.png`).
- Pair a Phone: remote control on with the code; Cancel: "remote disabled" in the host log and the
  window back to off (that capture showed a code and was not kept).
- The stand-in phone paired: on, "Test Phone · Connected now", the app QR still in full
  (`remote-on-paired.png`). Remove: "remote removed", "remote disconnected", "remote disabled";
  the window back to off (`remote-off-after-remove.png`).
- `hq remote status` with no phone: "Remote control is off: no phone is paired."

## Not verified

- A real phone: the "removed from the computer" message on the device.

## On screen (the owner freed the Mac, 2026-10-31)

One sandbox for all five changes (`/private/tmp/hqo`: synthetic `HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR`, relays off), the app and host built from the five branches merged together in a
scratch worktree, stand-in agents (`research/prototypes/background-status/standin.py`), a stand-in
`claude` answering the hidden `/usage` read with 7 % and 93 % or the trust question, the update
stand-in offering 0.1.21, and the stand-in phone. Captures in
`research/evidence/five-changes-2026-10-31/`. All five branches merge cleanly in pairs and
together; the merged tree lints clean.
