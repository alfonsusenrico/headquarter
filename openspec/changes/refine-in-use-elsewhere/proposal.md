# The desktop first, the phone second, while both have a session

Status: proposed 2026-10-28, waiting for the owner's approval. A follow-up to
`fix-terminal-in-use-elsewhere` on the same branch and pull request (#86, held by the owner).

## Why

After accepting `fix-terminal-in-use-elsewhere`, the owner held its pull request and asked for
a different design (2026-10-28, options page `research/prototypes/in-use-options/`):

- The desktop should not draw a session while the phone has it. It should show it like an
  inactive session, saying it is open on the phone (option D1).
- The phone is the secondary device. Opening a session there takes it at once, with no ask, and
  only the desktop shows a flag.
- When the desktop takes the session back, the phone leaves the session screen and returns to its
  home, instead of showing a cover.

## Expected Outcome

- While a phone has the session, the desktop's pane shows the inactive-style page "Open on
  <phone>" with "Use here" (or Enter), and no terminal. The sidebar row shows a phone mark in
  place of its age, for any session a phone has.
- Opening a session on the phone works as today: the phone takes it at once.
- When the desktop takes it back, the phone returns to its home at once, with a short note naming
  the session and the computer.

## What Changes

- `hq-app`: the in-use notice over the terminal becomes an inactive-style page in place of the
  terminal (and in place of the shell panel's terminal); the sidebar row's phone mark.
- `hq-protocol`, `hq-daemon`: the session list says which phone has each session
  (`SessionInfo.phone`), so the desktop can mark rows it does not show.
- `hq-mobile` and the Android app: the cover is removed; the session screen closes to home when
  another client takes the session.
- Specs: `terminal-pane`, `android-app`, `session-sidebar`, `session-host`.

## Non-goals

- The phone asking before it takes a session (owner: the phone is secondary).
- Showing the phone's drawing on the desktop (options D2 and D3, declined).
- Taking the session back when the desktop's window becomes active (see design D3).

## Capabilities affected

`terminal-pane`, `android-app`, `session-sidebar`, `session-host`. No new project capability.

## Impact

Desktop app, host and phone app, still protocol 14 (not released yet), so nothing to migrate.
