# Verification

Revision: branch `feat/add-sessions-running-elsewhere`, rebased on
`docs/split-provider-quota-requirements`, 2026-10-23.

## Checks

| Check | Result |
| --- | --- |
| `rows_show_title_folder_and_whether_they_can_be_added` (`hq-app`) | passed |
| `closing_the_other_terminal_is_told_apart` (`hq-app`) | passed |
| `clients_hear_unlinked_agents_appear_change_and_go` (`hq-daemon`) | passed |
| `clients_hear_outside_agents_come_and_go` (`hq-cli` host test) | passed; fails without the host change (timed out waiting for the change) |
| `./scripts/check` | see the final run recorded in `PROJECT_STATE.md` |

## On screen

Dev build with its own data folder (`HQ_DATA_DIR=/tmp/hqo`) and a copy of the owner's
settings, real agents only, 2026-10-23:

- The owner's first look found stale greyed "claude pid N" rows beside the same
  sessions linked, a new session missing, and misaligned rows. Fixed in this change
  (design D4, D5); the owner then confirmed no stale rows and asked for the tag column
  to line up, fixed with a fixed-width age column, and accepted ("ok good").
- A real Claude Code 2.1.309 session in Alacritty was added while running ("ext",
  pid 98706), closed there, and continued in Headquarter (`resume=true`).

## Not verified

- The "Closed in the other terminal." text was not captured on screen.
- Linux: not run on a Linux desktop.
