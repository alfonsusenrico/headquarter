# Verification: fix-update-from-pre-release

Revision: `fix/update-from-pre-release` ffc27db, on `main` 38f50fb.

| Check | Result |
| --- | --- |
| `./scripts/check` | passed: 658 tests (1 ignored), cargo-deny, OpenSpec 19/19 strict |
| Unit tests | `0.4.0-rc.2` older than `0.4.0` and `0.4.1`, newer than `0.3.1`; `0.4.0` not older than itself; candidates not ordered among themselves; odd text is no build; a host on `0.4.0-rc.2` behind an app on `0.4.0`; notes for `0.1.12-rc.2` up to `0.1.12` list 0.1.12, for `0.1.12` none, for `0.1.11-rc.1` both; the "latest version" text names `0.4.0-rc.2` |

On screen, 2026-11-14 (debug app against the real GitHub releases, latest `v0.4.0`; sandbox host
with a stand-in Claude; the app's version set in `crates/hq-app/Cargo.toml` for the run only and
reverted). Evidence: `research/evidence/fix-update-from-pre-release/`.

- Built as `0.4.0-rc.9`: the sidebar offered "Update Headquarter v0.4.0" right after opening (p2).
  What's new read "v0.4.0-rc.9 installed → v0.4.0" and listed 0.4.0's notes only (p3). The debug
  copy offers the release page, so nothing was installed.
- Built as `0.4.1-rc.9`: Check for Updates… said "Headquarter 0.4.1-rc.9 is the latest version."
  (p5). The sidebar showed "Restart to finish update", since the sandbox host ran `0.4.0`, older than
  that build: the host check now reads the host's version against the build.

Not verified:
- A host on a release candidate behind an app on the release, on screen: unit test only.
- The owner's installed `0.4.0-rc.2` keeps the old rule; it is offered 0.4.1 once released.
