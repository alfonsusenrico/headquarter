# Verification

Working tree on `feat/working-sessions-first` (rebased onto 5304cda), 2026-10-29, macOS.

## Automated

- `./scripts/check`: exit 0. 555 Rust tests passed, 0 failed (1 ignored, as before); rustfmt, clippy
  `-D warnings`, cargo-deny; OpenSpec valid (strict); Android ktlint, Lint (warnings as errors),
  unit tests and builds.
- New tests:
  - `hq-protocol` `needing_you_then_working_then_the_rest_in_each_list`: the owner's case (working
    for 20 minutes above a session finished 17 minutes ago), needs you above working, working by
    recent activity, a session in another terminal among the rest, the Needs you section unchanged;
    the existing order test updated (working first in the ungrouped list).
  - `hq-cli` `working_sessions_come_before_newer_idle_ones`: `hq ls` shows the same order.
  - `hq-app` `held_rows_keep_their_places_and_new_ones_take_theirs`: held rows keep their order with
    their new status word, a new session keeps its computed place, nothing held gives the computed
    order.
  - Android `HeldOrderTest`: the same rule on the phone.

## On screen (desktop)

Synthetic host under `/private/tmp/hq-order` with a stand-in agent that reports working and done over
OSC 7501 while a flag file exists, and two sessions, "alpha (older)" and "beta (newer)".

- Pointer over the pane, alpha starts working: it moves above the newer idle beta at once (`hq ls`
  shows the same).
- Pointer resting on the sidebar, alpha stops and beta starts working: both rows stay in place,
  alpha's showing its age and reply mark and beta's showing working.
- Pointer moves to the pane: beta moves to the top at once.

## Not verified

- The phone holding its rows while touched, on the dev phone (unit test and Lint only so far).
- The icon rail and a drag on screen (the same rows and hold code as the sidebar).
- Linux.
