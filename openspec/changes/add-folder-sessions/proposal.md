# Sessions already in a folder

Status: awaiting the owner's approval.

## Why

The owner wants to find forgotten sessions when starting a new one (2026-10-27): show how many
sessions a folder has while browsing, and when starting in a folder that holds sessions not in
the sidebar, ask whether to continue one of them instead. Options page:
`research/prototypes/folder-and-usage-options/index.html`, section 1. The owner chose A (a
dialog on Start) with the counts while browsing, and a "Don't ask again" checkbox.

## Expected Outcome

- In the New session window's folder browser, each folder row shows its total sessions
  ("3 sessions"), sidebar sessions and past conversations together.
- Start in a folder where every session is already in the sidebar: the session starts at once.
- Start in a folder with sessions not in the sidebar (past conversations): a dialog lists only
  those, each with its agent, title and "last used 2h ago", and Continue. Continue adds it to
  the sidebar and resumes it. "Start new session" starts a new one. "Don't ask again for these"
  stops these sessions being offered again, while a newly forgotten one still is.
- The "Open here" tag meets AA contrast in both themes.

## What Changes

- `hq-app` `modal/new_session.rs` and `folders.rs`: the counts per folder row, the check on
  Start, the dialog.
- `hq-app` `app_state.rs`: the conversations the owner chose not to be asked about again.
- `hq-app` `modal/mod.rs`: the tag's contrast.

## Non-goals

- Sessions running in another terminal (outside sessions): they are not counted or offered.
- Changing Add sessions (⌘O) or the sidebar.
- The phone.

## Capabilities affected

`desktop-app`. No new project capability.

## Impact

App only. The host already sends every session with its folder and whether it is in the
sidebar.
