# Verification

Date: 2026-10-24, branch `feat/in-app-update` (built on `feat/inactive-sessions`, PR #40).
`./scripts/check` passed: 434 tests, clippy `-D warnings`, cargo-deny, OpenSpec strict.
`scripts/test-install.sh` passed on macOS.

## Acceptance

| Criterion | Evidence | Result |
| --- | --- | --- |
| A newer release is found from the latest-release redirect; older, equal and pre-releases are not | `versions_compare_by_number`, `the_latest_release_is_read_from_its_redirect`; real repository answered 302 to `v0.1.8` (2026-10-24) | passed |
| The pill shows, stays after Later, and opens the window | owner on screen (sandbox) | passed |
| The installer is checked against GitHub's digest before it runs | `the_installer_digest_is_read_from_the_release`; real repository's digest matched `install.sh` (2026-10-24); sandbox run | passed |
| Steps download, verify, install, restart show as they happen | `progress_and_failures_are_read_from_the_installer_output`; installer test "progress lines in order"; owner on screen | passed |
| Busy sessions are named first; Wait installs and restarts once none is busy | `working_or_asking_sessions_are_busy`; owner on screen with a working stub session; host log: stop only after it went idle | passed |
| The restart stops the host, reopens the app, and running sessions start again | sandbox host log: stop, new host 1.2 s later, `claude-1` started again; owner on screen | passed |
| Only an installer-installed copy updates in place | `only_a_copy_the_installer_listed_is_updated_in_place` | passed |
| `[updates] check = false` stops automatic checks | `update_checks_are_on_unless_turned_off` | passed |
| Check for Updates… reports the latest version | owner on screen | passed |
| The download folder is removed after the install | sandbox: no `headquarter-update-*` left | passed |

## Environment

A scratch `HOME` (`/tmp/hqup-home`) with the installer's own install of this branch's
build, a stub Claude agent, and a local server playing GitHub (`HQ_UPDATE_BASE`,
`HQ_INSTALL_FROM`), so the owner's installed copy was never touched. Removed afterwards
with the installer's `--uninstall`.

## Not verified

- A real update between two published releases: the first is possible once a release
  with this change exists. The sandbox reinstalled the same build under a fake 0.1.9.
- Relaunching through `open` (the sandbox ran the bundle's program directly, D9).
- "Restart to finish update" with an older host on screen (unit-level logic only).
- A failed install on screen (checksum mismatch, installer error): covered by unit tests
  of the parsing only.
- Linux.
