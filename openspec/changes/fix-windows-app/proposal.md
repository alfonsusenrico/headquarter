# The Windows app: no console windows, its icon, and window buttons

Status: approved by the owner 2026-11-10 ("approve both windows plans"). Decisions: owner, 2026-11-10
(window buttons in the Windows 11 style).

## Why

The owner tested the Windows app (2026-11-09):

- Opening it flashes several console windows, which looks like malware.
- The app has no icon: Explorer and the taskbar show Windows' blank file icon.
- There are no minimize, maximize and close buttons.

Causes, from the code:
- The app is a windowed program, but it starts `curl` for the agents' marks (`marks.rs`, one per
  logo) and for the update check (`update.rs`) without asking Windows for no window, so Windows
  opens a console for each.
- No icon resource is built into `headquarter-app.exe`.
- The window hides the system title bar (`appears_transparent`) for the macOS look, and nothing
  draws the buttons on Windows.

The fourth report, sessions on the server shown as "folder no longer exists", came from 0.3.x
checking a remote session's folder on the PC's own disk. `v0.4.0-alpha.1` already fixed it
(`add-merged-host-list`, found on screen 2026-11-09). This change adds no code for it. The owner
checks it with the alpha.

## Expected Outcome

- Opening the app on Windows shows only its own window.
- `headquarter-app.exe`, the taskbar and Alt+Tab show the Headquarter icon.
- The window has minimize, maximize/restore and close at its top right, in the Windows 11 layout,
  and hovering maximize shows Windows' snap layouts.

## What Changes

- Every helper process the app starts (`curl` today, any later one through the same helper) asks
  Windows for no window (`CREATE_NO_WINDOW`).
- A Windows icon (`.ico`, 16 to 256 px, from the icon's existing renders) is built into the
  executable at build time on Windows only.
- On Windows only, three caption buttons are drawn at the window's top right, marked as the
  window's minimize, maximize and close areas so Windows handles them (snap layouts, keyboard,
  accessibility). The strip above the sidebar's Hosts menu is the window's drag area.

## Non-goals

- The installer: `add-windows-installer`.
- Signing the executable (SmartScreen still asks once).
- macOS and Linux title bars.

## Capabilities

`desktop-app`. No new project capability.

## Impact

`crates/hq-app` (a process helper, a Windows-only title bar element, a `build.rs` with the
`winresource` crate, MIT), `assets/icon/headquarter.ico`. The Windows check (`windows.yml`) builds
it. Ships in 0.4.0.
