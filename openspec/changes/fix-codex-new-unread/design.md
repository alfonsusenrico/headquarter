# Design: Codex `/new` no longer marks a session unread

## Decisions

### D1. Only `done` ends a turn, for every agent

`terminal_event` maps OSC 7501 `idle` to `Interrupted` for Codex too; `done` stays
`TurnComplete`. The usage re-read at a Codex turn's end follows `TurnComplete`, so it still runs
after a real turn and no longer after `/new`.

Rejected: telling `/new` apart by timing or by what the owner typed. A short real turn and `/new`
look alike, and reading typed input for status is out of bounds.

## Risks

- An installed Codex 0.163.0 or older would leave finished turns unmarked. Headquarter's agent
  update check runs on each new version; the owner's Codex is 0.166.0.
- If a current Codex ended some turn with `idle` alone, that turn would show idle rather than
  unread. One real turn on 0.166.0 confirms `done` before release.
