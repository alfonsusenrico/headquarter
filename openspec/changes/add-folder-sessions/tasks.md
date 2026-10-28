# Tasks

## 1. Window

- [ ] 1.1 Counts per folder row (D1). Evidence: unit test of counting by folder (sidebar and past together, outside sessions left out), the dev app on screen. Code 2026-10-28: `folders_count_sidebar_sessions_and_past_conversations`.
- [ ] 1.2 The check on Start and the dialog with Continue, Start new session and Cancel (D2, D3). Evidence: unit test of which sessions are offered (all in the sidebar: none, one of two not added: that one, neither added: both, a not-asked one: none), the dev app on screen in both themes, Continue resuming a past conversation into the sidebar.
- [ ] 1.3 Don't ask again kept in `app-state.json` (D4). Evidence: unit test of the stored ids and their pruning, the dev app after a restart.
- [x] 1.4 The tag's contrast (D5). Evidence: computed contrast in both themes. Done 2026-10-28: `folder_row_labels_meet_wcag_aa` (light and dark, highlighted and hovered rows).

Expected Result: forgotten sessions are found before a new one is started.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/folder-sessions`, open the PR, report its URL.

Expected Result: one PR with the feature.
