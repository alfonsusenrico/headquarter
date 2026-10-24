# Tasks

## 1. Host and protocol

- [x] 1.1 `AttachShell`/`CloseShell` (no `SessionInfo` field: the app remembers open panels), host-owned shell PTY in memory, ended with its session; excluded from status and from the outside scan's hosted list. Verify with host tests: open, attach, input/output, close, session removal ends it. Done: `a_sessions_shell_runs_beside_it_and_ends_with_it` passes (2026-10-24). An agent inside a shell is outside by construction: shells never enter the hosted list the scan excludes.

Expected Result: `cargo test -p hq-cli -p hq-protocol` passes.

## 2. App

- [ ] 2.1 Split pane with divider, Cmd-T / Ctrl-Shift-T toggle, focus moves, open panels kept per session in `app-state.json`, menu item. Verify with unit tests (shortcut mapping, split sizes, app-state round trip) and on screen.
- [ ] 2.2 Resource check: idle memory and CPU with five sessions each with an open shell, versus none. Evidence in `research/evidence/session-shell-panel/`.

Expected Result: the owner uses the panel on screen; resource numbers recorded.

## 3. Delivery

- [ ] 3.1 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the PR is open.
