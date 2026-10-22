# Proposal

## Why

In Add sessions, past conversations showed their generated names (`claude-<id>`)
instead of their titles, while sessions running in other terminals showed titles
(owner report, 2026-10-22). The host knew the titles: `hq ls --all --json` reported a
title for such a conversation, and for 138 of 203 history rows.

Cause: titles live only in the host's memory and are filled by each import pass. The
app listed the sessions before the first pass after the host started, so it got rows
without titles. That pass then told clients only about rows whose activity moved, so
titles learned for unchanged rows never reached the app. Rows in other terminals
showed titles because their status changes kept sending them again. The same gap
hides a title Claude Code writes after a conversation's first message when no new
activity comes with it.

## Expected Outcome

- Past conversations show their titles in the sidebar and in Add sessions, whether
  the app listed them before or after the host read the titles.
- A title the agent adds or changes later shows within the import interval.

## What Changes

- Each import pass tells clients about every row whose title appeared, changed, or
  went away, as well as rows whose activity moved.
- The host's history interval becomes a host option; `HQ_HISTORY_INTERVAL_SECS`
  shortens it in tests, like `HQ_USAGE_INTERVAL_SECS`. The default stays 60 s.

Decisions:
- **Broadcast title changes from the import pass (host).** Titles change only there,
  so the pass owns telling clients. Rejected: having the app list again after the
  first pass (every client would need it, and later title changes would still be
  lost). Rejected: relying on the first list waiting for the first pass
  (refine-first-run-setup): that covers only the first list, and it gives up after
  10 s.

## Non-goals

- Titles stay in memory only; nothing about storing them changes.

## Capabilities affected

- session-history: titles reach clients when they change.

## Impact

- `crates/hq-daemon/src/host/history.rs`, `host/mod.rs` (option), `crates/hq-cli/src/lib.rs`
  (environment variable), `AGENTS.md`. No protocol or index change. Adds no project
  capability.
