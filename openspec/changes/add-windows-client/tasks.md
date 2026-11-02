# Tasks

## 1. Spike

- [x] 1.1 Throwaway branch: GPUI window, text input and terminal view built on `windows-latest` (D8). Evidence: the run's log; stop and report if it fails.

Expected result: GPUI 0.3.9 builds for Windows with our view code.
Done: run 41457873032 (2026-11-02): built, window opened and quit, Cascadia Mono and Consolas present, alacritty grid rendered (`research/evidence/windows-client/2026-11-02-spike.md`).

## 2. Platform gates

- [x] 2.1 `HostPaths` to `hq-signals::paths` (design D1, changed while building); `hq-client` without `hq-daemon` (D1). Evidence: `./scripts/check`.
- [x] 2.2 `cfg(unix)` gates in `hq-signals`, `hq-discovery`, `hq-client`, `hq-app` (D1). Evidence: Windows clippy clean on the runner.

## 3. The app on Windows

- [x] 3.1 Remote only, `%APPDATA%\Headquarter`, first launch at Connect to a Host (D2). Evidence: unit tests for the paths; runner tests.
- [x] 3.2 Fonts, shortcuts, decorations (D3); notifications (D4); update notice with a link (D5). Evidence: unit tests; recorded as unverified on Windows until the owner's test.

## 4. CI and release

- [ ] 4.1 `windows.yml` on pull requests; `AGENTS.md` checks rule (D6). Evidence: a green run on this branch's PR (the same steps passed on the throwaway branch, run 41469164644).
- [x] 4.2 `dist` Windows target for `hq-app`, zip contents (D7). Evidence: `dist plan` lists the zip; a dry-run build on the runner.

Expected result: every pull request shows a Windows check, and a tag would publish the zip.
Done: `dist plan` lists `hq-app-x86_64-pc-windows-msvc.zip` (`headquarter-app.exe`) and no Windows host; the release-profile build (`--profile dist --no-default-features`) built and ran twice on `windows-latest` (run 41469164644), exit 0 both times.

## 5. Verification and delivery

- [ ] 5.1 `./scripts/check` passes; the Windows job passes; macOS app unchanged on screen. `verification.md` written, naming what only the owner's PC can verify. Open: the macOS on-screen check (needs the Mac free).
- [ ] 5.2 The owner checks it (after the alpha, on the PC).
- [ ] 5.3 After the owner accepts: archive, push `feat/windows-client`, open the PR, report its URL.
