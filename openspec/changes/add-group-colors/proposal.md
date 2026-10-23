# Proposal

## Why

The owner's group headers look too much like session rows (2026-10-23). From four
treatments on a local review page (`research/prototypes/group-contrast/`), the owner
chose D: each group has a colour, shown on its header and along its rows, chosen
automatically unless the owner picks one, from a soft palette.

## Expected Outcome

- Each group header shows a colour dot and its name in strong text; the group's rows
  have a thin line of that colour along their left side, so groups read at a glance.
- A new group gets a colour automatically; the owner can pick another, or go back to
  automatic, from the dot.
- Colours are soft and suit light and dark themes; the name, never the colour alone,
  identifies a group.

## What Changes

- `hq-store`: migration `0007_group_color.sql` adds a nullable `color` to `groups`
  (NULL is automatic).
- `hq-protocol`: `GroupInfo.color` (optional, defaulted for older hosts) and a
  `SetGroupColor { group, color }` request; the palette's names.
- `hq-daemon`: stores the colour and announces the groups.
- `hq-app`: palette tokens per light and dark mode (`theme.rs`); header dot and strong
  title; the colour line along the group's rows; a swatch popover on the dot;
  collapsed-group chips in the icon rail take the colour.

## Non-goals

- Colours for "Needs you" or "Ungrouped"; colours in `hq ls`.
- Custom colours outside the palette.

## Capabilities

### Modified Capabilities
- `session-sidebar`: groups have colours.

No project capability is added; `database` gains one migration under the existing
rules (applied at host start, checksummed).

## Impact

`crates/hq-store` (migration, group row), `crates/hq-protocol` (`GroupInfo`, request),
`crates/hq-daemon/src/host/mod.rs`, `crates/hq-app` (`theme.rs`, `workspace/groups.rs`,
`workspace.rs` rows, `workspace/rail.rs`), tests.

## Approval

Direction approved by the owner on 2026-10-23 ("use D, automate the colors if not
picked specifically by user and use soft palette colors like you show in the
artifact"). Plan approved by the owner on 2026-10-23 ("Approve").
