# Background's mark, and a ring that stays red

Status: approved by the owner 2026-10-31 ("ok approved", with the other four plans of that day).

## Why

The owner (2026-10-31):
- wanted another mark for "background" than the two dots, and chose stacked layers (D on
  `research/prototypes/background-glyph/index.html`);
- saw the week ring at 93% grey instead of red. The rings turn grey while the last reading is old,
  which hid how close the limit was.

## Expected Outcome

- A session doing background work ends its row with "background" in teal and a stacked-layers mark
  in the glyph cell, where review shows its diamond. Nothing moves.
- A ring at or above the near-limit share stays red while its reading is old, paler than a fresh
  one; other rings still grey out.

## What Changes

- Desktop row: the stacked-layers mark replaces the two dots (D1).
- Usage rings: near limit keeps its colour, softened, when the reading is old (D2).
- Specs: `agent-status` (the marks), `provider-quota` (ring colour).

## Non-goals

- The phone (words only, as chosen for add-background-work-status).
- The header and the Usage window (words only).

## Capabilities

No new project capability. Modified: `agent-status`, `provider-quota`.

## Impact

`hq-app` (`sidebar/glyph.rs`, `sidebar/model.rs`, `workspace.rs`). No host or protocol change.
