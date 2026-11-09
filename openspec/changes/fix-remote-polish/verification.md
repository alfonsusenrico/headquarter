# Verification: fix-remote-polish

Revision: `fix/remote-polish`, on `main` a5d9ee4 (v0.3.0).

| Check | Result |
| --- | --- |
| `hq-cli` `the_qr_code_is_dark_on_light_in_a_terminal` | passed: each line in black on bright white; plain when not coloured |
| `qrcode` 0.14 `Dense1x2` (its own test) | dark modules are the filled blocks, so black on white is the code's own polarity |
| Host test `hq_remote_pairs_lists_and_removes` | passed: the link printed, no escape codes when piped |
| `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest` | passed |
| `./scripts/check` | passed: 635 tests |

| The real `hq remote pair` output from a scratch host, drawn as a dark terminal shows it, decoded with zxing-cpp | new black on white: decodes with and without inverted reading; old terminal colours: only with inverted reading; the app link printed |
| Phone (TECNO CN7c, Android 16), debug build, paired computer unreachable | about 15 s: "Reconnecting to Alfonsuss-MacBook-Air…"; after 2 minutes: "Alfonsuss-MacBook-Air is offline" (no age: no list since the app started) |

Not verified: the age after a list was current (unit of `ageText`, unchanged).
