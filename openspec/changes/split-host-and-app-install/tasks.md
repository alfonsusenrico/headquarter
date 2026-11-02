# Tasks

## 1. Installer

- [ ] 1.1 `--host-only`, `--app-only`, parts in the install record, rerun and uninstall by parts (D2). Evidence: installer tests from local archives in sandboxed homes (each mode, mode change, uninstall).
- [ ] 1.2 App archive and macOS bundle without `hq` (D1). Evidence: archive listing; installer test.
- [ ] 1.3 Host service: systemd user unit, linger request and fallback, macOS LaunchAgent (D4). Evidence: `systemd-analyze verify`; launchd agent loaded and unloaded in a sandboxed home on this Mac; fallback message in the container.

## 2. App

- [ ] 2.1 Local host lookup through the install record; no host started when none is found (D3). Evidence: unit tests; on screen with an app-only sandbox.
- [ ] 2.2 Updater by parts (D5). Evidence: update stand-in with full and app-only records.

## 3. Docs and release

- [ ] 3.1 README install section (D6). Evidence: the owner reads it.
- [ ] 3.2 `AGENTS.md` Release section (D7).

## 4. Linux host

- [ ] 4.1 Host-only install and a remote session in a Linux container (D8). Evidence: `research/evidence/split-host-and-app-install/linux-container.md`.

Expected result: a full upgrade from 0.1.24 keeps working; a host-only Linux host serves the Mac app as a remote host.

## 5. Verification and delivery

- [ ] 5.1 `./scripts/check` passes; `verification.md` written.
- [ ] 5.2 The owner checks it.
- [ ] 5.3 After the owner accepts: archive, push `feat/split-install`, open the PR, report its URL. Then the `chore/release-0.2.0-alpha.1` pull request (pre-release).
