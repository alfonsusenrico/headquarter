# Tasks

## 1. Rows

- [x] 1.1 One name cell for bar and reset rows. Verify: drawing code, no unit test can see wrapping; both rows now share the helper, and the owner checks it on screen. Done: clippy and `cargo test -p hq-app` pass (2026-10-24).

Expected Result: names stay on one line in every row.

## 2. On screen and delivery

- [ ] 2.1 On screen: agy's usage details with a reset per-model window. Evidence: the owner's confirmation.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the owner sees one-line names, and the PR is open.
