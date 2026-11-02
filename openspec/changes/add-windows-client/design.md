# Design: the desktop app on Windows, as a client

## Context

`cargo tree --target x86_64-pc-windows-msvc -p hq-app` resolves; the compile blockers found by
reading the code are `std::os::unix` without a gate in `hq-client`, `hq-signals/src/forward.rs`
(pulled in by `hq-protocol`), `terminal/session.rs`, `workspace/remote.rs`, `quit.rs`,
`workspace/update.rs`, and `PermissionsExt` in `hq-discovery/src/search.rs`. `hq-client` depends
on `hq-daemon` only for `HostPaths`. Paths come from `HOME` and XDG.

## D1. Platform gates

- `HostPaths` moves to `hq-config` (a leaf crate both sides use); `hq-client` drops its
  `hq-daemon` dependency.
- The local-socket transport, the host start, hook forwarding and the local-host parts of quit
  and update compile on `cfg(unix)` only. On Windows `hq-client` has the remote link only.
- Discovery's executable check uses the platform's rule (`cfg(unix)` mode bits; on Windows the
  app does not search for agents).
Rejected: stub modules that compile everywhere and fail at run time (errors found late).

## D2. Remote only

On Windows the host switcher has no "this computer" entry; the first launch opens Connect to a
Host; first-run setup offers only the look. The app's data (client key, hosts, app state,
settings) lives in `%APPDATA%\Headquarter` (Windows' roaming application data folder,
`dirs::config_dir()`), never under `HOME`.

## D3. Look and keys

- Default font: Cascadia Mono, then Consolas (both ship with Windows 11 / 10).
- Shortcuts follow the Linux set (Ctrl+Shift for app keys), which already avoids the agents'
  keys; there is no menu bar, as on Linux, so Remote Control, Usage and Check for Updates stay
  reachable from the sidebar and the Keyboard Shortcuts window.
- Window decorations: the system's.

## D4. Notifications

Through GPUI's Windows platform when it delivers them; otherwise in-app only, recorded in
`verification.md`.

## D5. Updates on Windows

The update check runs as on other platforms; a newer release shows the notice with a link to
the release page instead of Install. The pre-release rule stands: an alpha is never offered as
an update, and an alpha build checks as the release it leads to.

## D6. Windows checks on pull requests

`.github/workflows/windows.yml` on `windows-latest` (free for a public repository) runs `cargo
clippy -p hq-app -p hq-client -p hq-remote -p hq-protocol --target x86_64-pc-windows-msvc -- -D
warnings` and `cargo test` for those crates. Formatting is the same on every platform and stays
in `./scripts/check`. `AGENTS.md` changes "Checks: `./scripts/check` is the project's CI … not a hosted
runner" to name this one hosted job. Rejected: Windows builds only on tags (breakage found at
release).

## D7. Release archive

`dist-workspace.toml` adds `x86_64-pc-windows-msvc`, for `hq-app` only (the host and `hq` are not
built for Windows): `[package.metadata.dist] targets` per package. The archive is a `.zip` with
`headquarter-app.exe`, the licences and theme notices. Unsigned: Windows SmartScreen shows "More
info → Run anyway" once; README says so. The release verify step counts the new asset.

## D8. Spike first

Before the refactor, a throwaway branch builds a GPUI window with a text input and our terminal
view on the Windows runner (build only, plus a headless smoke test if the runner allows a
window). If GPUI's Windows build fails at the pinned version, stop and report with options
(upgrade the gpui-pre crates together, or pause).

## Risks

- IME and keyboard layouts on Windows (Zed has open `platform:windows` issues): the owner's
  test covers their layout; issues become follow-ups.
- Unsigned builds: SmartScreen warnings; signing is a later change.
- A hosted job is now part of the checks: a red Windows job blocks a merge like a red local
  check.
