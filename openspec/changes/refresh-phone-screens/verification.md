# Verification: the phone's other screens in direction A

Tested: branch `feat/phone-screens-refresh` @ 4267798 from `main` @ 714be1b. 2026-10-28.

## Commands

- `./scripts/check`: passed. Rust 538 tests in 27 suites, 0 failed. ktlint, Android Lint
  (warnings as errors), app unit tests, debug and release APKs. OpenSpec strict 19/19.
- Contrast of the new pairs from the tokens in `ui/Theme.kt` (`contrast.txt`): all at least
  4.5:1 in both themes, lowest 5.16:1 (accent on accent-soft, light).

## On the dev phone

Phone: realme RMX1921, Android 15, debug build (`com.alfonsusenrico.headquarter.debug`), Do Not
Disturb and stay-awake on for the check and off afterwards. Host: a test host with a synthetic
home folder and a stand-in agent that plays a status by folder and answers the usage reads. The
groups were written into the test host's own index. Screenshots, status bar cropped:
`research/evidence/refresh-phone-screens/`.

- Pairing on first launch: three numbered steps, the camera with corner marks, Enter the code
  instead, step 3 reworded while typing. Paired for real with the test host's code.
- Home: the title folds away while scrolling down and returns at the top. The computer row shows
  the dot, the name, "connected" or "reconnecting…" (host stopped, amber dot and banner), and
  opens the sheet. Settings sits beside it.
- Usage tiles: Claude 42%, Codex 18%, Antigravity 81% in red.
- The needs-you card, group headers with chevrons (Client portal open, Experiments folded then
  opened), session rows, "working" with accent dots, "starting" with muted dots after a host
  restart, idle and not running rows.
- Add: shrinks to + while scrolling down, shows "Add" while scrolling up and at the top. Opens
  Add sessions, where adding "Fix the login redirect loop" put it on Home as not running.
- Computers sheet with "Pair another computer" and its icon.
- Settings: sections, the Unpair pill, Pair another computer with its icon, and the theme
  switched to Light from there.
- Dark and light themes for Home, Settings and Pairing. Add sessions in light.

Found and fixed while checking:
- The computer's state word took the space a long name needed ("Alfonsuss-Mac…").
- agy's tile cut its percentage ("week · Gemini · …"). The percentage now sits beside the name.
- Add never shrank: `LazyListState.lastScrolledForward` did not change under the folding
  title's nested scroll. The direction is now read from the scroll itself.

## Not verified

- TalkBack: `uiautomator dump` fails on this phone ("null root node"), so the content
  descriptions (Back, Settings, Add, the computer row's) are checked in code only.
- An unread row on Home (the stand-in's finished turns showed as idle).
- Add sessions in dark, and a computer named longer than the row (cut at the end by code).
- Other phones, screen sizes and font scales.

## Owner check

Pending.
