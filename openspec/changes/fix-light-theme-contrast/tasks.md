# Tasks

## 1. Diagnosis

- [ ] 1.1 Headquarter's chrome in the built-in light theme and the owner's own light theme: every pair measured, each failing colour traced (design, Diagnosis 1). Evidence: captures and a contrast table in `research/evidence/fix-light-theme-contrast/findings.md`.
- [ ] 1.2 Claude Code, Codex and agy in a light terminal: their colours, what they read (OSC 11, `COLORFGBG`), and any per-launch theme option, with versions (Diagnosis 2 to 4). Evidence: the same findings file.
- [ ] 1.3 Update the design with the confirmed causes. Show the owner the light tokens and, if the fix differs from D1 to D4, the new shape before building.

Expected Result: the causes are known and the fix confirmed.

## 2. Fix

- [ ] 2.1 Light tokens and a theme test for every chrome pair (D1). Evidence: the test, before and after captures.
- [ ] 2.2 Per-launch mode for agents that accept one (D2). Evidence: launch spec tests, on screen with Claude Code.
- [ ] 2.3 Minimum contrast in the terminal and its setting (D3). Evidence: unit tests of the lightness shift, on screen with Claude Code's dark colours on a light background.
- [ ] 2.4 Marks in their colours (D4). Evidence: theme test, captures in both modes.

Expected Result: light themes read as light, dark themes are unchanged.

## 3. Rule

- [ ] 3.1 Draft the rule for designing each theme for its own background and place it where the owner chose (a Pakem proposal for the framework's UI and UX skill, and or `AGENTS.md`). Evidence: the owner approves the text, then the proposal or the diff.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it on their own desktop with their own theme.
- [ ] 4.3 After the owner accepts: archive, push `fix/light-theme-contrast`, open the PR, report its URL.

Expected Result: one PR with the fix.
