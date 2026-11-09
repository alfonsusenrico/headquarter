# Verification: fix-remote-polish

Revision: `fix/remote-polish`, on `main` a5d9ee4 (v0.3.0).

| Check | Result |
| --- | --- |
| `hq-cli` `the_qr_code_is_dark_on_light_in_a_terminal` | passed: each line in black on bright white; plain when not coloured |
| `qrcode` 0.14 `Dense1x2` (its own test) | dark modules are the filled blocks, so black on white is the code's own polarity |
| Host test `hq_remote_pairs_lists_and_removes` | passed: the link printed, no escape codes when piped |
| `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest` | passed |
| `./scripts/check` | passed: 635 tests |

Not verified yet: the banner on the phone with a computer turned off; the coloured QR code read
from a dark terminal by the phone app.
