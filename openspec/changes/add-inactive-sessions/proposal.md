# Proposal

## Why

Quitting an agent frees its memory: 270-720 MB per session on the owner's Mac
(measured 2026-10-24, seven sessions holding 2.9 GB). Today a quit also removes the
session from the sidebar, so using that saving means adding the session back with Add
sessions every time. The owner wants quitting to leave the session in the sidebar,
dimmed as inactive, and resumable from there (2026-10-24). This reverses the 2026-10-21
decision in `revise-session-lifecycle` ("no inactive state; a quit leaves the list").

Today a host start (after a reboot or `hq stop`) leaves every session stopped until the
owner opens it or presses Resume all. The owner wants active sessions running as soon as
the app starts, and inactive ones left alone.

## Expected Outcome

- Quitting an agent from inside it (`/exit`, `/quit`, Ctrl-D, any of the agent's own
  ways), or the agent ending with an error, leaves the session in the sidebar as
  inactive: title and age dimmed, a moon where the status glyph goes. Its shell, if
  open, ends with it.
- Opening an inactive session shows a pane with a "Resume Session" button; the agent
  starts on the same conversation only when the owner presses it.
- The ✕ still stops a session and removes it from the sidebar. On an inactive row it asks
  "Remove “title” from the sidebar?" and removes it.
- When the host starts, every sidebar session that was running when the host last
  stopped starts again on its conversation, whatever stopped the host (reboot, crash,
  `hq stop`): the session the app shows first, then the others one at a time. All of
  them show "starting", breathing, until their agent is ready, so they settle one by one.
  Inactive
  sessions stay inactive. The "N sessions were running" note with Resume all goes.
- A session that cannot start then (folder gone, agent missing) becomes inactive, a
  notification says so, and its pane gives the reason above Resume Session.
- A session marked "ext" whose agent is closed in the other terminal becomes inactive
  the same way, instead of "Closed in the other terminal." with Open here.

## What Changes

- `hq-store`: the existing `interrupted_at` mark now means "stopped with the host; start
  it when the host starts". `hq stop` sets it as a crash or reboot does. Migration
  `0008` converts the sidebar once (design D5).
- `hq-daemon`: a quit (exit code 0) keeps the session in the sidebar as inactive; at
  start-up the host starts marked sessions one at a time (D3) and keeps the reason a
  start failed, in memory only.
- `hq-protocol`: session info gains the start failure reason (defaulted for older hosts);
  the `Dismiss` request goes.
- `hq-app`: the moon glyph and dimmed inactive rows; "starting", breathing, on every
  session the host will start until it is ready; the Resume Session pane; asking before removing an inactive row; resuming the
  shown session first; the start-failure notification; the interrupted note goes.
- `hq-cli`: `hq ls` says "inactive" and "starting"; `hq resume --interrupted` goes
  (`hq resume <name>` stays).

## Non-goals

- Stopping idle sessions automatically to save memory; only the owner or the agent
  quits.
- Changing the ✕ (it still stops and removes) or the agents' own exit commands.
- Releasing a version: the owner asked for no release before the updater work.

## Capabilities affected

`agent-status`, `session-host`, `session-sidebar`, `session-restore`, `notifications`,
`shell-panel`. No new project capability; `database` gains migration `0008` (runs at
host start under the single-instance lock, per the project override).

Approved by the owner on 2026-10-24 ("approve, go ahead").

## Owner decisions (2026-10-24 interview)

- ✕ still removes; only the agent quitting (or failing) makes a session inactive.
- Inactive: moon glyph in the status glyph's place, title and age dimmed.
- Opening an inactive session shows a "Resume Session" button; nothing starts on click.
- An open shell ends when its session becomes inactive.
- At host start, every session that was running starts, even after `hq stop`.
- An agent crashing (non-zero exit) makes its session inactive; no automatic restart.
- Pacing: the session shown first, the rest one at a time.
- The Resume all note is removed.
- A start failure: inactive plus a notification.
- An "ext" session closed in the other terminal becomes inactive.
- ✕ on an inactive row asks before removing.
- On upgrade, only sessions running when the old host stopped start; the rest are
  inactive.
- Found in the on-screen check, decided the same day: a Claude session that never had
  a message starts afresh under its id (option A); an inactive session whose folder is
  gone offers "Remove from Sidebar" instead of Resume Session.

## Impact

`crates/hq-store` (migration, end and stop marks), `crates/hq-daemon` (exit handling,
start-up queue), `crates/hq-protocol`, `crates/hq-app` (sidebar glyph, pane, workspace,
notifications), `crates/hq-cli` (render, resume), host tests.
