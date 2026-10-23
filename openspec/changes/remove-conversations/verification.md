# Verification

Revision: branch `feat/remove-conversations` (name-untitled-conversations and
fix-agent-start-path merged in), working tree with this change, 2026-10-23.

## Research evidence

- Claude Code 2.1: no command deletes one conversation (`claude purge [path]` removes a
  whole project's state; `claude rm` is for background sessions).
- Codex 0.163: `codex archive <id>`, `codex unarchive <id>`, `codex delete --force <id>`
  (no prompt); both exit 1 on an unknown id (checked in a throwaway `CODEX_HOME`).
- agy 1.4.4: deletes only in its `/resume` picker (F4, `item.delete`); print mode
  refuses interactive-only commands; no `delete` subcommand.

## Checks

| Check | Result |
| --- | --- |
| `past_conversations_are_removed_the_way_their_agent_allows` (host): transcript and its folder moved, wrong way refused, `codex archive` and `codex delete --force` run with the ids, row no longer offered | passed |
| `rows_offer_the_removals_their_agent_allows_unless_running` (`hq-app`) | passed |
| The `trash` crate with the file manager method on macOS, one scratch file | landed in `~/.Trash` with no prompt; removed afterwards |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny incl. the new `trash` dependency, OpenSpec) | passed, 386 tests, 24 OpenSpec items |

## Not verified

- Whether Finder offers "Put Back" for items trashed through the file manager call;
  dragging out of the Trash always works.
- On screen (task 2.2): waits for the owner.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 386 tests, 24 OpenSpec items (at d630eda).
