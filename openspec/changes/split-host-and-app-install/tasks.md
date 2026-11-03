# Tasks

## 1. Installer

- [x] 1.1 `--host-only`, `--app-only`, `--full` (added while building), parts in the install record, rerun and uninstall by parts (D2). Evidence: installer tests from local archives in sandboxed homes (each mode, mode change, uninstall).
- [x] 1.2 App archive and macOS bundle without `hq` (D1). Evidence: archive listing; installer test.
- [x] 1.3 Host service: systemd user unit, linger request and fallback, macOS LaunchAgent (D4). Evidence: `systemd-analyze verify`; launchd agent loaded and unloaded in a sandboxed home on this Mac; fallback message in the container.

## 2. App

- [x] 2.1 Local host lookup through the install record; no host started when none is found (D3). Evidence: unit tests; on screen with an app-only sandbox.
- [x] 2.2 Updater by parts (D5). Evidence: unit tests of full and app-only records (the installer keeps the recorded parts on a rerun, covered by the installer test).

## 3. Docs and release

- [x] 3.1 README install section (D6). Evidence: the owner reads it.
- [x] 3.2 `AGENTS.md` Release section (D7).

## 4. Linux host

- [x] 4.1 Host-only install and a remote session in a Linux container (D8). Evidence: `research/evidence/split-host-and-app-install/linux-container.md`.

Expected result: a full upgrade from 0.1.24 keeps working; a host-only Linux host serves the Mac app as a remote host.
Done: installer test (56+ checks, macOS and Debian) incl. an install from before parts; Linux host in a container served a desktop client on the app's client library (`linux-container.md`); a real LaunchAgent in a sandbox home started, restarted after `hq stop`, and unloaded on uninstall. Found while testing: `hq stop` reported a failure when a service started the host again at once; the host now names itself in its lock file (host test `stop_reports_a_host_its_service_started_again`).

## 5. Verification and delivery

- [x] 5.1 `./scripts/check` passes; `verification.md` written.
- [x] 5.2 The owner checks it.
- [ ] 5.3 After the owner accepts: archive, push `feat/split-install`, open the PR, report its URL. Then the `chore/release-0.2.0-alpha.1` pull request (pre-release).
