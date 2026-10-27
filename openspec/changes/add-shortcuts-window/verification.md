# Verification

Date: 2026-10-27, branch `feat/shortcuts-window`, the working tree that became the commit after
ee9d12e. `./scripts/check` passed: 523 Rust tests, 0 failed; clippy `-D warnings`; cargo-deny;
the Android checks; OpenSpec strict, 19 items.

Environment: the owner's Mac (macOS, Apple silicon), a debug build, an isolated data and config
folder (`HQ_DATA_DIR`, `HQ_CONFIG_DIR`) with setup skipped and no sessions. Keys sent with System
Events, clicks and scrolling with posted CGEvents.

| Criterion | Evidence | Result |
| --- | --- | --- |
| One table drives the keys, the menus and the window | Unit tests `every_shortcut_is_listed_once_and_no_two_share_a_key`, the menus' binding tests over `SHORTCUTS`, the existing key tests unchanged | passed |
| Rows by area, with each system's keys | Unit test `the_window_lists_every_key_by_area_for_its_system`. Dev app: Sessions, Window, Terminal, Sidebar in order, 19 rows on macOS | passed |
| Typing filters to a flat list with area tags | Unit test `typing_filters_by_name_explanation_or_keys`. Dev app: "focus" (2 rows), "cmd+t" (Shell Panel), "sidebar" (7 rows), "focuszzz" ("No shortcut matches.") | passed |
| Esc clears the filter, then closes | Dev app: the first Esc showed the full list again, the second closed the window | passed |
| ⌘/ and Headquarter ▸ Keyboard Shortcuts… open it, ⌘/ closes it | Unit test `the_window_opens_on_slash_or_question_mark`. Dev app: opened both ways, closed by ⌘/ | passed |
| Opening it from a theme preview puts the theme back | Dev app: Light previewed in the theme switcher, then ⌘/: the window opened in the dark theme | passed |
| Both themes | Dev app, dark and light, unfiltered and filtered | passed |

Screenshots: `research/evidence/shortcuts-window/` (local).

Found and fixed on screen:

- The key column was 190 px wide because Alt-Shift-arrows drew six caps. Shared modifiers now
  show once (⌥ ⇧ ↑ / ↓, unit-tested by `Press::shown`), and the column is 120 px on macOS and
  150 px on Linux.
- Explanations were cut off at 560 px. The window is now 640 × 680 px, and a long explanation
  wraps under itself beside its name.
- Esc in the sidebar read "Close a close or delete prompt." It now reads "Back out of a close or
  delete prompt."

Not verified: Linux (Ctrl+Shift+/ and the Linux key column are unit-tested only), the theme
creator blocking ⌘/ (code inspection only).
