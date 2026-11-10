# A Windows installer

Status: proposed 2026-11-10, waiting for the owner's approval. Decisions: owner, 2026-11-10 (per
user, an `.exe` setup wizard, installer only).

## Why

The owner tested the Windows app (2026-11-09): the release offers a zip to unpack by hand. Windows
users expect an installer. It should put the app where apps go, add it to the Start menu, and
remove it from Settings ▸ Apps.

## Expected Outcome

- The release's Windows download is `headquarter-setup-x86_64.exe`, a setup wizard with the
  Headquarter icon (Welcome, folder, shortcuts, Install, Finish with "Open Headquarter").
- It installs for the current user into `%LOCALAPPDATA%\Programs\Headquarter` with no
  administrator prompt, adds a Start menu entry and optionally a desktop shortcut, and registers an
  uninstaller in Settings ▸ Apps.
- Running a newer setup over an installed copy updates it in place, closing a running app first.
  The app's data in `%APPDATA%\Headquarter` stays.

## What Changes

- An Inno Setup script (`packaging/windows/headquarter.iss`) and a release job that builds the
  setup from the Windows build and adds it, with its `.sha256`, to the release and `sha256.sum`.
- The Windows zip is no longer offered (D3).
- The app's update notice on Windows links to the release page, as now, where the setup is.
- README's Windows section and `AGENTS.md`'s release kind and verify list name the setup.
- A `-rc.1` goes first, because the release pipeline changes.

## Non-goals

- Code signing. SmartScreen still asks once ("More info ▸ Run anyway") until the app is signed.
- An installer for macOS or Linux (they keep `install.sh`).
- A per-machine (Program Files) install.
- Automatic updates on Windows.

## Capabilities

`release-install`, `desktop-app` (Windows update link only if it changes). No new project
capability. Inno Setup is a build tool on the release runner under its own permissive licence, not
a dependency of the app.

## Impact

- New: `packaging/windows/headquarter.iss` and `.github/workflows/build-windows-installer.yml`.
- Changed: `dist-workspace.toml` and the generated `release.yml`, README, `AGENTS.md` (release
  section).
- Builds on `fix-windows-app` for the icon.
