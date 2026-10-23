# Tasks

## 1. Copy

- [x] 1.1 Keep OSC 52 copies in the model and write them to the clipboard. Verify with a model test (a copy is kept and decoded; a read request gets no reply). Done: `agent_copies_are_kept_and_reads_are_refused` passes; hq-app 140 tests, clippy clean (2026-10-23).

Expected Result: `cargo test -p hq-app` passes.

## 2. On screen and delivery

- [ ] 2.1 On screen: select text with the mouse in a real Claude Code session in a dev build, then paste elsewhere. Evidence: the owner's confirmation.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the owner can copy from Claude Code, and the PR is open.
