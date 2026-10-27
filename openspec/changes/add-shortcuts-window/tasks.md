# Tasks

## 1. Window

- [x] 1.1 The shortcut table, with `shortcut()` and the menus reading from it (D1). Evidence: unit tests (every shortcut listed once, no shared keys, the existing key tests unchanged). Done 2026-10-27: `SHORTCUTS` drives `shortcut()` and the menus' bindings, `every_shortcut_is_listed_once_and_no_two_share_a_key`, `the_window_lists_every_key_by_area_for_its_system`.
- [x] 1.2 The window, its filter and Esc behaviour (D2). Evidence: unit test of the filter, the dev app on screen in both themes, unfiltered and filtered. Done 2026-10-27: `typing_filters_by_name_explanation_or_keys`, on screen in dark and light, "focus", "cmd+t", "sidebar" and a no-match query, Esc clearing then closing.
- [x] 1.3 ⌘/ and the menu item (D3). Evidence: unit test of the key, the dev app opening it both ways. Done 2026-10-27: `the_window_opens_on_slash_or_question_mark`, opened by ⌘/ and by Headquarter ▸ Keyboard Shortcuts…, closed by ⌘/, opened from a theme preview with the theme put back.

Expected Result: every shortcut can be found in one window.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/shortcuts-window`, open the PR, report its URL.

Expected Result: one PR with the window.
