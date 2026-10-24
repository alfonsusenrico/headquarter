# Tasks

## 1. Installer and settings

- [x] 1.1 `HQ_PROGRESS=1` lines in `scripts/install.sh` (D4). Verify with `scripts/test-install.sh` or a local run with `HQ_INSTALL_FROM`, with and without the variable.
- [x] 1.2 `[updates] check` in `hq-config` (schema, read, live reload). Verify with config unit tests.

Done 2026-10-24: `scripts/test-install.sh` passed on macOS (progress lines in order when asked, none otherwise); `update_checks_are_on_unless_turned_off` passes.

Expected Result: config tests pass; the installer prints progress lines only when asked.

## 2. App

- [x] 2.1 The check (D1): redirect parse, version compare, schedule, manual check, setting. Verify with unit tests (tag parse, newer/older/equal, malformed).
- [x] 2.2 Installed-by-installer detection (D2) and the install (D3): download, digest check, run, progress file. Verify with unit tests (manifest match, digest parse, progress lines) and a local run against a fake release served from a local folder.
- [x] 2.3 Pill, modal, busy warning and Wait (D5), restart (D6), finishing an update made elsewhere (D7), macOS menu item (D8). Verify with unit tests (busy list, pill state) and on screen.

Done 2026-10-24: unit tests in `update.rs` (versions, redirect, installed copy, digest, progress, busy) pass. On screen in a sandbox home with a fake release (D9): the pill, Later, Update Now with its steps, the restart onto the installed copy with the host restarted, the manual check; then with a working stub session: the busy question, Wait, the restart once idle, and the session starting again (owner, "ok working"; host log confirms). The download folder is removed after the install (fix made during the check).

Expected Result: the owner checks on screen in a dev window: a fake newer release shows the pill; Later keeps it; Update Now runs the steps; with a working session the window asks first; the restart reopens the app with sessions starting.

## 3. Delivery

- [x] 3.1 `./scripts/check` passes; README; `verification.md`; archive; push and open the PR once #40 is merged (this branch builds on it). No release (the owner releases). Done: `./scripts/check` passed, 434 tests (2026-10-24); PR after #40.

Expected Result: the PR is open.
