# Tasks

## 1. Window and entry

- [ ] 1.1 Window opened with app id `headquarter`; desktop entry with `StartupWMClass=headquarter`. Verify with `cargo build -p hq-app` and `scripts/test-install.sh` on macOS and in `debian:12-slim` (new check "menu entry names the window class").

Expected Result: the installer test passes on macOS and Debian with the new check.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; `verification.md` records that the icon on a running Linux desktop was not seen on screen (no Linux desktop here).
- [ ] 2.2 After the owner accepts: sync and archive, push the branch, open the PR for the owner to merge, and report its URL.

Expected Result: the PR is open with the verification record.
