# Tasks

## 1. Shared parts

- [ ] 1.1 Icons and shared parts: header, card, pill button, section label (D1). Evidence: build, ktlint, Android Lint.

Expected Result: every screen draws its header, cards and buttons from the new parts.

## 2. Home

- [ ] 2.1 The folding title and the computer row with Settings (D2). Evidence: the dev phone scrolled and at the top, both themes, each link state shown in the row (connected, reconnecting with the host stopped).
- [ ] 2.2 Usage tiles (D3). Evidence: the dev phone with the test host's usage, both themes.
- [ ] 2.3 Cards, group headers, rows and the working dots (D4). Evidence: the dev phone with a needs-you, an unread, a working, an idle and an inactive stand-in session, a folded and an open group.
- [ ] 2.4 The floating Add button (D5) and the computers sheet (D6). Evidence: the dev phone scrolling down and up, the sheet open.

Expected Result: Home looks like direction A and behaves as before.

## 3. Pairing, Settings, Add sessions

- [ ] 3.1 Pairing (D7). Evidence: the dev phone: the steps and camera, then the code field, and a real pairing with the test host.
- [ ] 3.2 Settings and Add sessions (D8). Evidence: the dev phone in both themes, adding a past conversation from Add sessions.

Expected Result: the other screens share Home's look.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes. Contrast of the new pairs in both themes. `verification.md` with screenshots (status bar cropped, no notifications).
- [ ] 4.2 The owner checks it on a phone.
- [ ] 4.3 After the owner accepts: archive, push `feat/phone-screens-refresh`, open the PR, report its URL.

Expected Result: one PR with the four screens in direction A.
