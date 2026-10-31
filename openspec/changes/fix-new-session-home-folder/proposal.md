# An empty folder field means the home folder

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-10-31): the New session window's folder field starts filled with `~/`, which has to
be deleted before typing a folder. An empty field should already mean the home folder.

## Expected Outcome

- The folder field opens empty, with a hint saying an empty field starts in the home folder.
- With the field empty, the list shows the home folder ("Open here") and its subfolders, as `~/`
  does today, and Start starts there.
- Typing a path or a name behaves as before.

## What Changes

- `NewSession`: no default value; an empty field browses and starts in the home folder.
- `session-sidebar` spec: New session window.

## Non-goals

- The Open sessions window's search field (it already starts empty).

## Capabilities

No new project capability. Modified: `session-sidebar`.

## Impact

`hq-app` (`modal/new_session.rs`, the folder browser's text handling). No host change.
