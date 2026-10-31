# A quiet notice for "you're on the latest version"

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-10-31): when Check for Updates… finds nothing new, a window covering the working
pane is too much. A small notice should slide in at the bottom right, leave by itself after about
5 seconds, and be dismissible.

## Expected Outcome

- Check for Updates… with the latest version installed: a small notice slides in at the bottom right
  of the window, "Headquarter 0.1.21 is the latest version", and slides out after 5 seconds or when
  dismissed. The pane, the keyboard and the session are untouched.
- A failed manual check says why in the same notice, which stays until dismissed.
- With reduced motion, the notice appears and disappears without sliding.

## What Changes

- A notice component in the window's bottom right (D1, D2).
- Manual update check results use it instead of the update window (D3).
- Specs: `desktop-app` (the notice), `app-update` (results of a manual check).

## Non-goals

- Moving other messages to the notice in this change (each can follow when wanted).
- System notifications (unchanged).

## Capabilities

No new project capability. Modified: `desktop-app`, `app-update`.

## Impact

`hq-app` (a `notice` view, `workspace/update.rs`). No host change.
