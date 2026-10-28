# Tasks

## 1. Window

- [x] 1.1 Counts per folder row (D1). Evidence: unit test of counting by folder (sidebar and past together, outside sessions left out), the dev app on screen. Done 2026-10-28: `folders_count_sidebar_sessions_and_past_conversations`; on screen in both themes, web-app with one sidebar session and two past conversations read "3 sessions" (`research/evidence/folder-sessions/`).
- [x] 1.2 The check on Start and the dialog with Continue, Start new session and Cancel (D2, D3). Evidence: unit test of which sessions are offered (all in the sidebar: none, one of two not added: that one, neither added: both, a not-asked one: none), the dev app on screen in both themes, Continue resuming a past conversation into the sidebar. Done 2026-10-28: `only_sessions_not_in_the_sidebar_are_offered`; on screen: the dialog in both themes, Continue added and resumed "Fix the login redirect loop", Enter started new, Esc and Cancel went back, notes started without a question.
- [x] 1.3 Don't ask again kept in `app-state.json` (D4). Evidence: unit test of the stored ids and their pruning, the dev app after a restart. Done 2026-10-28: `the_not_asked_list_grows_and_forgets_sessions_that_are_gone`, `sessions_not_to_ask_about_are_kept_with_the_rest`; on screen: ticked, then after quitting and reopening the app, web-app started without a question.
- [x] 1.4 The tag's contrast (D5). Evidence: computed contrast in both themes. Done 2026-10-28: `folder_row_labels_meet_wcag_aa` (light and dark, highlighted and hovered rows).

Expected Result: forgotten sessions are found before a new one is started.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-28: 538 tests, 0 failed.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/folder-sessions`, open the PR, report its URL.

Expected Result: one PR with the feature.
