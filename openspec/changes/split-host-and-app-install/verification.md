# Verification: split-host-and-app-install

## Revision tested

Branch `feat/split-install` at `1a6b11d` (on `main` after #111), macOS on the owner's MacBook
Air, Docker 29.9.2 (arm64) for Linux. The on-screen check found one bug, fixed in `9c6722e`;
`./scripts/check` passed again there (601 tests, 0 failed).

## Automated checks

| Check | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed: 601 tests, 0 failed; OpenSpec 19 of 19 valid |
| `sh scripts/test-install.sh` on macOS | all installer checks passed (aarch64-apple-darwin) |
| The same in `debian:12-slim` | all installer checks passed (aarch64-unknown-linux-gnu) |

The installer test, from stand-in archives in throwaway homes, covers: the full install
(`parts=host,app`, no service, the bundle without `hq`), an install from before parts (stays
full; the old bundle's `hq` removed), host only (no app, no signing keychain, a service file
running `hq daemon` with `HOME`, recorded), a rerun keeping host only and saying how to switch
the running host, a change to app only (host stopped first, `hq` and the service removed, the
version read from the app), `--full` again, an unknown option, uninstall, and a checksum
mismatch. Service files are written with `HQ_SERVICE=files`, never started, so the test never
reaches the machine's launchd or systemd.

New unit and host tests: the host lookup order (`HQ_HOST_BIN`, beside the app, the install
record, `~/.local/bin/hq`, none), the updater with full and app-only records, and
`stop_reports_a_host_its_service_started_again` (fails on the old `hq stop` with "did not stop
within 10 seconds").

## The host as a service

- **macOS, for real, in a sandbox home** (`/private/tmp/hqS`, its own data and settings): the
  host-only install loaded the LaunchAgent and the host ran on the sandbox's data; `hq stop`
  ended it and launchd started it again ("Its service started it again, on the installed
  version"); a rerun said the running host keeps its version until `hq stop`; uninstall
  unloaded the agent and stopped the host. The sandbox was removed afterwards.
- **Linux:** the unit passes `systemd-analyze verify` in user and system mode; without systemd
  the installer prints the `nohup … hq daemon` line (container).

## Linux host (D8)

`research/evidence/split-host-and-app-install/linux-container.md`: `hq` built for arm64 Linux,
installed host-only in a fresh Debian container, started as the installer said, and a
desktop client on the Mac (the app's `hq-client`) paired through n0's relay in 440 ms, listed
the server's session and folders, attached and typed.

## The app without a host, on screen

The debug app copied alone into a sandbox (its own `HOME`, data and settings folders, no `hq`
beside it, none recorded or in `~/.local/bin`), captures in
`research/evidence/split-host-and-app-install/`:

- Setup showed the look only ("step 1 of 2"). After Skip, Connect to a Host opened over Add
  sessions, which setup opens to fill the sidebar. Fixed in `9c6722e`: Add sessions opens
  after setup only when a host is on screen (this was also wrong on Windows). Afterwards only
  Connect to a Host showed, over "No host yet" (`app-only-after-setup.png`).
- No host started: the sandbox's data folder held only the app's own files.
- Paired with the Linux container host through n0's relay with a code from `hq remote pair`
  (through a file and the clipboard, both cleared): "Paired with e94b4384a2be.", its session
  listed, opened and echoing typed text (`app-only-paired.png`, `app-only-linux-session.png`,
  `app-only-linux-typed.png`).
- ⌘Q quit at once, without the stop question.

The sandbox and the container were removed afterwards.

## Not verified

- systemd running the unit with lingering on a real server: the owner's server.
- The first `hq stop` against a host started before this change (it does not name itself in
  its lock) still reports "did not stop within 10 seconds" when a service restarts it at once;
  it has stopped and restarted. Only an install that adds the service while an older host runs
  meets it once.
