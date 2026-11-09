# Tasks

## 1. Build

- [x] 1.1 `Key::Left`, `Key::Right` by cursor mode (D1). Evidence: `left_and_right_follow_the_cursor_mode`.
- [x] 1.2 `Terminal::cursor_steps` and `Phone::move_cursor_to` (D2). Evidence:
  `a_tap_on_the_cursors_row_counts_the_arrows_to_it` (to a letter, already there, past the end,
  another row, after moving, a hidden cursor), `a_wide_character_is_one_press`.
- [x] 1.3 Key rows (six and four a row), tap to move with the field started afresh (D1, D2).
  Evidence: ktlint, Android Lint, 17 JVM tests.

Expected Result: the arrows and a tap move the agent's cursor.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check`; `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest`.
- [x] 2.2 On the owner's phone by adb with a test host: the grid, "abcdef" then a tap on "c" and
  "X" gives "abXcdef", Backspace there, ← twice then "Y". Evidence:
  `research/evidence/phone-typing-images/2026-11-09-cursor.md`.
- [ ] 2.3 The owner tries it; then archive with `add-image-upload`, push `feat/image-upload`, open
  the one pull request, report its URL.
