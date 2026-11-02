# Tasks

## 1. Spike

- [ ] 1.1 Throwaway branch: GPUI window, text input and terminal view built on `windows-latest` (D8). Evidence: the run's log; stop and report if it fails.

Expected result: GPUI 0.3.9 builds for Windows with our view code.

## 2. Platform gates

- [ ] 2.1 `HostPaths` to `hq-config`; `hq-client` without `hq-daemon` (D1). Evidence: `./scripts/check`.
- [ ] 2.2 `cfg(unix)` gates in `hq-signals`, `hq-discovery`, `hq-client`, `hq-app` (D1). Evidence: Windows clippy clean on the runner.

## 3. The app on Windows

- [ ] 3.1 Remote only, `%APPDATA%\Headquarter`, first launch at Connect to a Host (D2). Evidence: unit tests for the paths; runner tests.
- [ ] 3.2 Fonts, shortcuts, decorations (D3); notifications (D4); update notice with a link (D5). Evidence: unit tests; recorded as unverified on Windows until the owner's test.

## 4. CI and release

- [ ] 4.1 `windows.yml` on pull requests; `AGENTS.md` checks rule (D6). Evidence: a green run on this branch's PR.
- [ ] 4.2 `dist` Windows target for `hq-app`, zip contents (D7). Evidence: `dist plan` lists the zip; a dry-run build on the runner.

Expected result: every pull request shows a Windows check, and a tag would publish the zip.

## 5. Verification and delivery

- [ ] 5.1 `./scripts/check` passes; the Windows job passes; macOS app unchanged on screen. `verification.md` written, naming what only the owner's PC can verify.
- [ ] 5.2 The owner checks it (after the alpha, on the PC).
- [ ] 5.3 After the owner accepts: archive, push `feat/windows-client`, open the PR, report its URL.
