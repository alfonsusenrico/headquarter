# Tasks

## 1. Shared parts

- [x] 1.1 Icons and shared parts: header, card, pill button, section label (D1). Evidence: build, ktlint, Android Lint. Done 2026-10-28: five Material icons, `Crumb` with the back icon, 16 dp `Card`, pill `HqButton`, `RowPill`, `WorkingDots`; ktlint, Android Lint (warnings as errors) and the debug build pass.

Expected Result: every screen draws its header, cards and buttons from the new parts.

## 2. Home

- [x] 2.1 The folding title and the computer row with Settings (D2). Evidence: the dev phone scrolled and at the top, both themes, each link state shown in the row (connected, reconnecting with the host stopped). Done 2026-10-28: the title folds away and returns at the top (`home-dark.png`, `home-scrolled-dark.png`), "connected" and "reconnecting…" with the test host stopped (`home-reconnecting-light.png`). Found and fixed: the state word took the space a long computer name needed.
- [x] 2.2 Usage tiles (D3). Evidence: the dev phone with the test host's usage, both themes. Done 2026-10-28: Claude 42%, Codex 18%, Antigravity 81% (red) in both themes. Found and fixed: agy's "week · Gemini · 81%" was cut, so the percentage moved beside the name.
- [x] 2.3 Cards, group headers, rows and the working dots (D4). Evidence: the dev phone with a needs-you, an unread, a working, an idle and an inactive stand-in session, a folded and an open group. Done 2026-10-28: needs-you card, working (accent dots) and starting (muted dots), idle and inactive rows, "Client portal" open and "Experiments" folded then opened (`home-group-open-light.png`). No unread row: the stand-in's finished turns showed as idle.
- [x] 2.4 The floating Add button (D5) and the computers sheet (D6). Evidence: the dev phone scrolling down and up, the sheet open. Done 2026-10-28: Add shrinks to + while scrolling down and shows "Add" scrolling up and at the top (`home-scrolling-up-dark.png`), the sheet (`computers-sheet-dark.png`). Found and fixed: `LazyListState.lastScrolledForward` did not change under the folding title's nested scroll, so the direction is read from the scroll itself.

Expected Result: Home looks like direction A and behaves as before.

## 3. Pairing, Settings, Add sessions

- [x] 3.1 Pairing (D7). Evidence: the dev phone: the steps and camera, then the code field, and a real pairing with the test host. Done 2026-10-28: steps over the camera with corner marks (`pair-camera-dark.png`, `pair-camera-light.png`), Enter the code instead, step 3 reworded, a real pairing with `hq remote pair`'s code. The capture with the code typed was deleted.
- [x] 3.2 Settings and Add sessions (D8). Evidence: the dev phone in both themes, adding a past conversation from Add sessions. Done 2026-10-28: Settings in both themes, Add sessions in light; "Fix the login redirect loop" added and shown on Home as not running.

Expected Result: the other screens share Home's look.

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes. Contrast of the new pairs in both themes. `verification.md` with screenshots (status bar cropped, no notifications). Done 2026-10-28: 538 tests, Android checks, OpenSpec 19/19; contrast lowest 5.16:1 (`contrast.txt`).
- [ ] 4.2 The owner checks it on a phone.
- [ ] 4.3 After the owner accepts: archive, push `feat/phone-screens-refresh`, open the PR, report its URL.

Expected Result: one PR with the four screens in direction A.
