# Tasks

## 1. Row layouts

- [x] 1.1 Row layouts kept while unchanged, the wide-character fallback, the exact advance (D1-D3). Evidence: unit test of the row text (padding to columns, styles per span, a wide row falls back), the debug app on the dev phone in both themes with a coloured screen, box drawing, a CJK row and the cursor. Done 2026-10-27: `TerminalRowTest` (3 tests), `m-dark.png` and `m-light.png` in the evidence folder.
- [x] 1.2 Measure against `main` on the dev phone (scroll and spinner loads, `measure.sh`). Evidence: `research/evidence/cache-terminal-row-layouts/findings.md` updated with the final build. Done 2026-10-27: scroll 111 → 17 ms a frame, spinner 25 → 14-15 ms.
- [ ] 1.3 Scrollback drag and pinch still work. Evidence: the debug app on the dev phone. Scrollback done 2026-10-27 (`m-scroll.png`). Pinch is left to the owner's check (2.2): adb sends one pointer only.

Expected Result: fast output scrolls smoothly and the terminal looks the same.

## 2. Verification and delivery

- [ ] 2.1 Remove the temporary timing. `./scripts/check` passes (ktlint, Android Lint, unit tests, APK). `verification.md` written.
- [ ] 2.2 The owner checks it on a phone.
- [ ] 2.3 After the owner accepts: archive, push `perf/terminal-row-cache`, open the PR, report its URL.

Expected Result: one PR with the faster terminal.
