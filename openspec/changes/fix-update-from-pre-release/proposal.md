# Offer the final release to its release candidate

Status: approved by the owner 2026-11-14 ("build the fix").

## Why

After `v0.4.0` was released (2026-11-13), the owner's app, `0.4.0-rc.2`, said it was on the latest
version. The update check counts a pre-release build as the release it leads to
(`Version::running()` in `crates/hq-app/src/update.rs`), as the "Pre-release builds" requirement
says. So whoever installed a release candidate is never offered that release, and gets an update
only at the next one. The owner had to run the install line by hand.

## Expected Outcome

- An app on `0.4.0-rc.2` offers `0.4.0` once it is the latest release, with 0.4.0's notes on the
  What's new page.
- It still never offers a pre-release, and a release candidate whose release is not out yet still
  says it is on the latest version.
- An app on `0.4.0` finds a host on `0.4.0-rc.2` behind it and offers the restart that finishes the
  update.

## What Changes

- Versions compare as semver orders them: a pre-release comes just before its release
  (`0.4.0-rc.2` < `0.4.0`), and after every earlier release.
- The update check, the What's new notes and the host version check use that order.
- The update window and the "latest version" notice name the running build as it is, for example
  "v0.4.0-rc.2 installed".

## Non-goals

- Offering pre-releases to anyone. `releases/latest` never names one.
- Ordering pre-releases among themselves (`rc.1` against `rc.2`). Nothing compares two of them.
- Apps already on a release candidate: their copy keeps the old rule, so `0.4.0-rc.2` is offered
  0.4.1, not 0.4.0. Only release candidates built after this change benefit.

## Capabilities

`app-update` only. No new project capability.

## Impact

`crates/hq-app/src/update.rs` (the version and its order, notes between versions) and
`crates/hq-app/src/workspace/update.rs` (the check, the host check, the texts). No host, protocol or
phone change. Ships in 0.4.1.
