# Proposal

## Why

Updating Headquarter means copying the install line from the README, and the installer
leaves the running host on its old program until the owner runs `hq stop`, which ended
every session. The owner wants Headquarter to notice new releases itself and update in
one click (request 2026-10-24; research `research/notes/2026-10-24-auto-update-plan.md`).
Since `add-inactive-sessions`, a host restart no longer costs the sessions: those that
were running start again on their conversations, so the hot restart the research
considered is not needed.

## Expected Outcome

- Headquarter checks for a newer release at start, every 6 hours while open, and from
  Headquarter > Check for Updates… on macOS. A setting turns the automatic checks off.
- A newer release shows as a quiet "Update 0.1.9" pill at the bottom of the sidebar, which
  stays until the owner updates. It opens a modal: "Headquarter 0.1.9 is available", with
  Update Now and Later.
- Update Now shows the steps as they happen: download, verify, install, restart. A
  failure says why and leaves the installed version as it was.
- When sessions are working or waiting for the owner, the modal names them first and
  offers Restart Now (interrupting them) or Wait. With Wait the new version installs at
  once and the restart happens by itself as soon as none is busy.
- The restart stops the host, which marks the running sessions, and the app reopens on
  the new version. The new host starts those sessions again, the shown one first.

## What Changes

- `hq-app`: the check (one HTTPS request through `curl`, which the installer already
  needs), the pill, the modal with its steps and the busy warning, the restart, and the
  macOS menu item.
- `scripts/install.sh`: `HQ_PROGRESS=1` prints one machine-readable line per step.
- `hq-config`: `[updates] check`, true by default.

## Non-goals

- Restarting the host without restarting its agents (hot restart); downgrades;
  pre-releases; updates without the owner's click; Windows.
- Updating copies of Headquarter not installed by the installer (dev builds, `cargo
  install`): they are told a release exists and get the release page.
- A release: the owner releases when ready.

## Capabilities affected

New capability `app-update`; `settings` gains the update setting; `release-install`
gains the installer's progress lines and the macOS menu item. No project capability
changes.

Approved by the owner on 2026-10-24 ("ok").

## Owner decisions (2026-10-24 interview)

- Host: restart once idle; sessions come back through starting with the host.
- Checks: at start, every 6 hours, and from a menu item.
- Notice: a sidebar pill and the menu item, no system notification.
- Busy sessions: ask first, naming them; Restart Now or Wait.
- Wait: install now, restart by itself once nothing is busy.
- Modal: the version and Update Now / Later (no release notes).
- Later: only closes the modal; the pill stays until the update.
- Setting: `[updates] check = true` by default.

## Impact

`crates/hq-app` (new `update.rs`, workspace, menus), `crates/hq-config` (schema, read),
`scripts/install.sh`, `scripts/test-install.sh` if it covers the new mode, README.
