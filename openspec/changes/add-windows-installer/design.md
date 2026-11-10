# Design: a Windows installer

## Context

`dist` 0.32 builds the release. The Windows target produces
`hq-app-x86_64-pc-windows-msvc.zip`. Custom jobs already run in the same release:
`build-android.yml` builds and signs the APK as a global artifacts job, and the release is
published only when every job succeeds. GitHub's Windows runners are free for this public
repository.

## Decisions

### D1. Inno Setup, per user

The owner chose an `.exe` wizard and a per-user install (2026-11-10). Inno Setup 6 settings:
- `PrivilegesRequired=lowest`, `DefaultDirName={localappdata}\Programs\Headquarter`, a fixed
  `AppId`, and `AppVersion` from the tag.
- `SetupIconFile` and the uninstaller's icon are the app's `.ico` (`fix-windows-app`).
- The Start menu entry, an optional desktop shortcut (a task, off by default), and "Open
  Headquarter" on the last page.
- `CloseApplications=yes`, so a running app is closed before files are replaced.
- The uninstaller removes the program files and shortcuts and leaves `%APPDATA%\Headquarter`.

Rejected: an `.msi` from `dist` (WiX). Its dialog is plainer, and per-user MSI installs are awkward
(the owner chose the wizard). NSIS: equally capable, but its scripts are harder to read and keep.

### D2. A release job builds the app and the setup

Windows leaves `dist`'s targets (the workspace's and the app's), so `dist` builds and publishes
no Windows zip. `build-windows-installer.yml` runs as one more global artifacts job, like the
APK's, on `windows-2022`:
1. It builds `headquarter-app.exe` as `dist` did: `--profile dist`, no default features, locked.
2. It checks that the app's version is the release's tag.
3. It runs `ISCC` from the runner image, which lists Inno Setup 6.7.1 (2026-11-10). It stops if
   Inno Setup is missing rather than install an unpinned copy.
4. It writes `headquarter-setup-x86_64.exe` and its `.sha256` in `sha256sum`'s format.

`dist` writes `sha256.sum` before custom jobs run, so the setup is checked by its own
`.sha256`, as the APK is. The release waits for the job (`dist generate` adds it to `host`'s
needs).

Rejected: packing the zip that `dist` built. Every published file comes from the run's
artifacts, so the zip would still be on the release page. Removing it afterwards would leave
`sha256.sum` naming a missing file.

### D3. Installer only

The owner chose a single Windows download (2026-11-10). D2 gives it: the release carries the
setup and no zip. `dist plan` for a tag lists no Windows artifact.

### D4. Verify list

`AGENTS.md`'s verify list gets the setup and its checksum, and 4 app archives instead of 5. On
the owner's PC: install, Start menu, open, update over an older setup, uninstall from Settings ▸
Apps, data kept.

## Risks

- SmartScreen warns on an unsigned setup more insistently than on an unsigned zip. Signing is a
  later, paid decision.
- An antivirus may flag a fresh unsigned setup. Reported by the owner's test.
