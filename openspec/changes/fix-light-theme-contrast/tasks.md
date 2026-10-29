# Tasks

## 1. Diagnosis

- [x] 1.1 Headquarter's chrome in the built-in light theme and the owner's own light theme: every pair measured, each failing colour traced (design, Diagnosis 1). Evidence: captures and a contrast table in `research/evidence/fix-light-theme-contrast/findings.md`.
- [x] 1.2 Claude Code, Codex and agy in a light terminal: their colours, what they read (OSC 11, `COLORFGBG`), and any per-launch theme option, with versions (Diagnosis 2 to 4). Evidence: the same findings file.
- [x] 1.3 Update the design with the confirmed causes. Show the owner the light tokens and, if the fix differs from D1 to D4, the new shape before building. Done 2026-10-29: four states compared on screen, the owner chose "apart".

Expected Result: the causes are known and the fix confirmed.

## 2. Fix

- [x] 2.1 Light tokens and a theme test for every chrome pair (D1). Evidence: the test, before and after captures.
- [x] 2.2 Per-launch mode for agents that accept one (D2). Evidence: launch spec tests, on screen with Claude Code.
- [x] 2.3 Minimum contrast in the terminal and its setting, in both modes (D3). Evidence: unit tests of the lightness shift both ways, on screen with Claude Code's dark colours on a light background and a dark colour on a dark background.
- [x] 2.4 Marks in their colours (D4). Evidence: captures in both modes; Claude's orange measured on each light sidebar (2.5 to 2.7:1, recorded).

Expected Result: light themes read as light, dark themes are unchanged.

## 3. Rule

- [x] 3.1 The rule in both places the owner chose: the framework's UI and UX skill (Pakem proposal) and `AGENTS.md` (`docs/theme-contrast-rule`). Evidence: the proposal's pull request and the docs pull request, both merged by the owner (Pakem #102, Headquarter #85, 2026-10-28).

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it on their own desktop with their own theme.
- [ ] 4.3 After the owner accepts: archive, push `fix/light-theme-contrast`, open the PR, report its URL.

Expected Result: one PR with the fix.
