# Verification

Working tree on `fix/phone-error-text` (rebased onto 5a047fd), 2026-10-29, macOS.

- `./scripts/check`: exit 0. 555 Rust tests passed, 0 failed (1 ignored, as before); OpenSpec valid
  (strict); Android ktlint, Lint (warnings as errors), unit tests and builds.
- New Android test `ErrorTextTest`: `Failed` and `Refused` show their reason ("Couldn't reach the
  computer: the computer did not answer"), `NotConnected` shows "Not connected to the computer.".
- `error.message` no longer appears in the app's own code (only in the generated binding).
- Reproduced before the fix on the dev phone (2026-10-28, in-use fix check): "reason=Couldn't reach
  the computer: the computer did not answer" under the pairing field.

Not verified: the fixed text on the dev phone (needs the owner's go for the Mac and the phone).
