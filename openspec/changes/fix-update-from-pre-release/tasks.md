# Tasks

## 1. Build

- [x] 1.1 `Build` (D1): read from `0.4.0` or `0.4.0-rc.2`, ordered against a release `Version` as
  semver orders them. Evidence: unit tests: `0.4.0-rc.2` < `0.4.0` < `0.4.1`, `0.4.0-rc.2` >
  `0.3.1`, `0.4.0` == `0.4.0`, odd text is no build.
- [x] 1.2 The check, the notes and the host check use it (D2). Evidence: unit tests: the notes
  for `0.4.0-rc.2` up to `0.4.0` list 0.4.0, and for `0.4.0` up to `0.4.0` nothing; a host on
  `0.4.0-rc.2` is behind an app on `0.4.0`.
- [ ] 1.3 Texts name the build as it is (D3). Evidence: unit test of the "latest version" text for a
  pre-release build; on screen (2.2).

Expected Result: a release candidate is offered its release once published, and nothing else
changes for release builds.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` (2026-11-14: 658 tests, cargo-deny, OpenSpec strict pass); the Windows check runs on the pull request.
- [ ] 2.2 On screen (Mac free, owner's go): a debug build versioned `0.4.0-rc.9` (not committed),
  pointed at the real releases, offers 0.4.0 with its notes. The same build versioned `0.4.1-rc.9`
  says it is on the latest version.
- [ ] 2.3 After the owner accepts: archive, push `fix/update-from-pre-release`, open the PR, report
  its URL.
