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

### D2. A release job builds the setup from the Windows build

`build-windows-installer.yml` runs as one more global artifacts job on `windows-2022`:
1. It takes the Windows build's zip from the run's artifacts. If `dist` gives global jobs no
   access to the local builds, it builds `headquarter-app.exe` itself with the same profile.
2. It runs `ISCC`. Inno Setup is installed with Chocolatey when the runner image lacks it, pinned
   to one version.
3. It writes `headquarter-setup-x86_64.exe` and its `.sha256`.

Both files join the release, and the setup's checksum joins `sha256.sum`. Which of the two ways
step 1 can work is checked first, on the `-rc.1`.

### D3. Installer only

The owner chose a single Windows download (2026-11-10). If `dist` can keep the zip as an internal
artifact without publishing it, it is dropped from the release. Otherwise the zip stays on the
release page, unlinked from every document and from the app, and this is recorded as debt.
Checked on `-rc.1`.

### D4. Verify list

`AGENTS.md`'s verify list gets the setup and its checksum, and loses the zip if D3 drops it. On
the owner's PC: install, Start menu, open, update over an older setup, uninstall from Settings ▸
Apps, data kept.

## Risks

- SmartScreen warns on an unsigned setup more insistently than on an unsigned zip. Signing is a
  later, paid decision.
- An antivirus may flag a fresh unsigned setup. Reported by the owner's test.
