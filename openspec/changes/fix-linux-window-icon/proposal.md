# Proposal

## Why

The owner wants to share Headquarter with other users (2026-10-23) and asked to confirm
the icon is installed properly. On macOS it is. On Linux the installer puts the icons
and `headquarter.desktop` in place, but the app's window carries no app id, so GNOME,
KDE and other desktops cannot match the running window to its entry and show a
generic icon in the dock, taskbar and window switcher.

## Expected Outcome

- On Linux, the running Headquarter window shows the Headquarter icon in the dock,
  taskbar and window switcher, on Wayland and X11.

## What Changes

- `hq-app` (`main.rs`): the window is opened with the app id `headquarter`, which GPUI
  sets as the Wayland app id and the X11 `WM_CLASS`.
- `scripts/install.sh`: the desktop entry adds `StartupWMClass=headquarter`.
- `scripts/test-install.sh`: checks that line.

## Non-goals

- No change on macOS (GPUI ignores the app id there) or to the icon files.
- Linux notification icons: the app posts no notifications on Linux.

## Capabilities

### Modified Capabilities
- `release-install`: the Linux install ties the running window to its menu entry.

No project capability is added.

## Impact

`crates/hq-app/src/main.rs`, `scripts/install.sh`, `scripts/test-install.sh`. No
agent CLI involved.

## Approval

Approved by the owner on 2026-10-23.
