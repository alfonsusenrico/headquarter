# Verification

Revision: branch `feat/usage-both-windows` as pushed, 2026-10-23.

## Checks

| Check | Result |
| --- | --- |
| `rings_show_the_short_window_inside_and_the_week_outside`, `label_names_one_agent_near_its_limit_or_counts_several`, `one_agent_is_named_by_its_nearest_window_the_shorter_on_a_tie` (`hq-protocol`) | passed |
| `usage_rings_meet_non_text_contrast` (`hq-app`): red fill at 3:1 in light, dark and Catppuccin | passed |
| `./scripts/check` | passed, 374 tests, 15 OpenSpec items (at cc1f274) |

## On screen

The owner confirmed both rings and the tooltip (2026-10-23).

## Not verified

- Red at 80% on screen (task 1.4): no window was at 80% during the test; the tests
  above cover it.
