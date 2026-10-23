# Verification

Revision: branch `fix/agy-link-by-activity` (from fix-title-broadcast), working tree
with this change, 2026-10-23.

## Diagnosis evidence

- One agy process (started 08:32:48 local) and one top-level agy conversation in its
  folder, last written 08:53:55 local; Headquarter listed both an unlinked entry for the
  process and the conversation as addable, then linked them later.
- agy's index (`conversation_summaries`) has `not_fully_idle`, `parent_conversation_id`
  and `nesting_depth`; sub-agents (nesting 1 or more) were already left out of the list.

## Checks

| Check | Result |
| --- | --- |
| `agy_names_its_conversation_on_the_command_line` (`hq-signals`) | passed |
| `agy_links_to_the_conversation_its_folder_wrote_since_it_started` (`hq-daemon`) | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 377 tests, 17 OpenSpec items |

## Not verified

- Whether agy clears `not_fully_idle` while waiting for approval was not observed
  directly; the new rule does not depend on it.
- On screen (task 1.2): waits for the owner.
