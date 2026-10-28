# The update log reads from the top and follows the update

Status: proposed 2026-10-28, waiting for the owner's approval. The owner asked for plans only:
no implementation until approved, after a few days of using 0.1.17.

## Why

The owner's finding while updating to 0.1.17 (2026-10-28, screenshot): the log under the steps
starts at the bottom of an empty box, so the first lines sit low with blank space above them.
The owner wants it to start at the top and scroll by itself as the update goes on.

Cause, from the code (`render_log` in `hq-app` `workspace/update.rs`): the box is a fixed 132 px
tall, aligned to the bottom (`justify_end`), clipped (`overflow_hidden`) and shows only the last
6 lines (`LOG_LINES`). Earlier lines cannot be read back.

## Expected Outcome

- The first line shows at the top of the box, and lines follow downwards.
- Once the lines fill the box, it scrolls by itself to keep the newest line in view.
- The owner can scroll back through every line of this update. While scrolled back, new lines do
  not pull the view down. Scrolling to the bottom again resumes following.

## What Changes

- `hq-app` `workspace/update.rs`: the log becomes a scrolling list with a scrollbar, top-aligned,
  holding every line of the current update, following the end.
- `app-update` spec: the log requirement says where lines start and how it scrolls.

## Non-goals

- What the log says (its lines and their wording).
- The What's new page and the steps above the log.

## Capabilities affected

`app-update`. No new project capability.

## Impact

Desktop app only. No agent CLI is involved.
