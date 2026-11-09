# Tasks

## 1. Build

- [ ] 1.1 `HostTrouble { kind, message }` from the feed thread: `Update` (with both versions and
  which side is older), `PairAgain`, `Other` (D1). Evidence: unit tests on the wording for each
  kind, the older-side choice, and no semicolons in the text.
- [ ] 1.2 Retry every 30 seconds on `Update` while the window listens; stop on `PairAgain` and
  `Other` (D4). Evidence: unit test of the watch loop's decision per kind; on screen (2.2).
- [ ] 1.3 Sidebar lines and Hosts menu state words and reasons (D2, D3). Evidence: unit test of the
  lines in host-name order; on screen (2.2).

Expected Result: a host the app gave up on is named with what to do, and one that was updated comes
back by itself.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check`; the Windows check runs on the pull request.
- [ ] 2.2 On screen (Mac free, owner's go): a sandbox host running the released
  `v0.2.0-alpha.2` `hq` (protocol 15): its sidebar line and menu row; the same sandbox restarted on
  this branch's `hq` (protocol 16) comes back within 30 seconds without reopening the app; Forget
  clears the line. The owner's server is the real check.
- [ ] 2.3 After the owner accepts: archive, push `fix/hosts-menu-reason`, open the PR, report its
  URL.
