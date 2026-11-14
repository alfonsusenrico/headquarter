# Design: offer the final release to its release candidate

## Context

`update::Version` is `major.minor.patch` only. `Version::parse` rejects a pre-release suffix, so the
tags from `/releases/latest` and the release list are always releases. `Version::running()` strips
this build's suffix, so `0.4.0-rc.2` runs as `0.4.0`. The check offers `latest` when
`latest > running`, the notes list releases with `running < v <= latest`, and the host check marks
the host behind when `host < running` (a host version with a suffix fails to parse, so the check
gives up).

## Decisions

### D1. A build version that knows it is a pre-release

`Version::running()` and the host's version become a `Build`: the release it leads to plus whether
it is a pre-release. `Build` orders against a release `Version` as semver does: a pre-release is
older than its own release and newer than every earlier one. Releases (`Version`) stay as they are,
so `/releases/latest` and the release list are read as now.

Rejected:
- A full semver crate: two comparisons and no ordering between pre-releases do not need one.
- Treating `0.4.0-rc.2` as `0.3.x`: it would be offered an older 0.3 release as an update.

### D2. Where the order applies

- The check offers `latest` when the running build is older than it.
- The notes list releases the running build is older than, up to `latest`, so `0.4.0-rc.2` gets
  0.4.0's notes.
- The host check marks the host behind when the host's build is older than the app's release. A
  host on `0.4.0-rc.2` is behind an app on `0.4.0`. A host on the same pre-release as the app is
  not.

### D3. Naming the running build

The installed chip, the "is the latest version" notice and window text name the build as it is
(`0.4.0-rc.2`). The release it leads to stays an internal value.

## Compatibility and rollback

App-only. Copies already installed keep their old rule until they update. Rollback is the previous
release.

## Risks

- None for release builds: a build without a suffix orders exactly as before.
