# Tasks

Starts after PR #67 merges: rebase this branch on `main` first.

## 1. Session screen

- [x] 1.1 Vector icons and shared parts (key, icon button, tonal button) (D5). Evidence: build, ktlint. Done 2026-10-27: four drawables, `IconAction`, `onAccent`, restyled `KeyButton`.
- [x] 1.2 Slim header for session and shell (D2). Evidence: the debug app on the dev phone, both themes, a long title cut at the end. Done 2026-10-27: session and shell headers on the dev phone, dark and light (`session-keys-dark.png`, `shell-dark.png`, `session-live-light.png`). A long title is cut with an ellipsis by the same one-line rule as before (not shown: the stand-in's title is short).
- [x] 1.3 Key grid with the tall ⏎, folding to one row while the keyboard is open, the shell's keys (D3). Evidence: the dev phone with the keyboard closed and open, session and shell. Each key reaches the agent (the stand-in echoes what it receives). Done 2026-10-27: every key reached the stand-in (1, 2, 3, Esc, Shift-Tab, Up, Down, Tab, Ctrl-C, Enter), the row folds with the keyboard open and returns when it closes, the shell's row with Ctrl-C and Ctrl-D (labels shrink to fit, a fix found here).
- [x] 1.4 Pill composer with Send inside, growing to four lines (D4). Evidence: the dev phone: empty, one line, five lines typed. Send and the keyboard's Send type the text then Enter. Done 2026-10-27: no Send while empty, Send inside the pill with text, four lines then scrolling (`session-typing-dark.png`). The Send button and Gboard's Send key both typed the text then Enter. A hardware Enter adds a line, as before.
- [x] 1.5 Terminal fades and Live (D1). Evidence: the dev phone scrolled back, Live returns to the newest output. Done 2026-10-27: Live shows when scrolled back and returns to the newest output (`session-live-dark.png`).

Expected Result: the session and shell screens look like D3 and behave as before.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` with screenshots (status bar cropped).
- [ ] 2.2 The owner checks it on a phone.
- [ ] 2.3 After the owner accepts: archive, push `feat/phone-session-refresh`, open the PR, report its URL.

Expected Result: one PR with the new session screen.
