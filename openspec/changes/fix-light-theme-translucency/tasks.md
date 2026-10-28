# Tasks

## 1. Diagnosis

- [ ] 1.1 Reproduce with the owner's appearance settings in a test folder over a dark desktop picture, light theme. Evidence: window capture matching the owner's screenshot.
- [ ] 1.2 The four experiments of the design's Diagnosis, one change each. Evidence: captures and contrast measured from them, recorded in `research/evidence/fix-light-theme-translucency/findings.md` with which hypothesis held.
- [ ] 1.3 Update the design with the confirmed cause. If the fix differs from D1 and D2, show the owner before building.

Expected Result: the cause is known and the fix confirmed.

## 2. Fix

- [ ] 2.1 Translucency follows the mode, `opacity_light` (D1, D2). Evidence: unit tests of `window_background` for each mode and setting, and of the `opacity_light` range check.
- [ ] 2.2 Colours that do not follow the light theme, if the diagnosis found any (D3). Evidence: a theme test per colour in both modes.
- [ ] 2.3 On screen: the owner's settings, light theme over the dark desktop picture, then a template and a theme made in the creator. Dark theme unchanged. Evidence: captures, contrast measured.

Expected Result: light themes read as light, dark themes are unchanged.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check` passes. `verification.md` written.
- [ ] 3.2 The owner checks it on their own desktop.
- [ ] 3.3 After the owner accepts: archive, push `fix/light-theme-translucency`, open the PR, report its URL.

Expected Result: one PR with the fix.
