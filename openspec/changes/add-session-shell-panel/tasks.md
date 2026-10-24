# Tasks

## 1. Host and protocol

- [x] 1.1 `AttachShell`/`CloseShell` (no `SessionInfo` field: the app remembers open panels), host-owned shell PTY in memory, ended with its session; excluded from status and from the outside scan's hosted list. Verify with host tests: open, attach, input/output, close, session removal ends it. Done: `a_sessions_shell_runs_beside_it_and_ends_with_it` passes (2026-10-24). An agent inside a shell is outside by construction: shells never enter the hosted list the scan excludes.

Expected Result: `cargo test -p hq-cli -p hq-protocol` passes.

## 2. App

- [x] 2.1 Split pane with divider, Cmd-T / Ctrl-Shift-T toggle, focus moves, open panels kept per session in `app-state.json`, menu item. Verify with unit tests (shortcut mapping, split sizes, app-state round trip) and on screen. Done (unit part): `the_shell_panel_takes_cmd_t_or_ctrl_shift_t_only`, `the_divider_keeps_both_parts_usable`, `shell_panels_and_their_share_are_kept_with_the_rest` pass; hq-app 153 tests, clippy clean (2026-10-24). On screen: the owner confirmed it in a dev window ("ok working", 2026-10-24).
- [x] 2.2 Resource check: idle memory and CPU with five sessions each with an open shell, versus none. Evidence in `research/evidence/session-shell-panel/`. Done: each shell 7.5-8.1 MB in its own process with no idle CPU; host memory unchanged (2026-10-24).

Expected Result: the owner uses the panel on screen; resource numbers recorded.

## 3. Delivery

- [x] 3.1 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge. Done: `./scripts/check` passed, 422 tests (2026-10-24).

Expected Result: the PR is open.
