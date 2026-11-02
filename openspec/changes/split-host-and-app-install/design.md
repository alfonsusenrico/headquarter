# Design: host and app install separately

## Context

`dist` builds two packages per target: `hq-cli` (`hq`, `headquarter`; the host is `hq daemon`)
and `hq-app` (`headquarter-app`). `install.sh` installs both, puts `hq` in `~/.local/bin` and also
copies it into `Headquarter.app/Contents/MacOS/`, and the app starts the host from "the `hq`
beside it" (`hq-client::start_host`). Nothing keeps a host running without a client starting it.

## D1. Two downloads

- Host: the `hq-cli-<target>.tar.gz` archives (macOS, Linux).
- App: the `hq-app-<target>` archives (macOS and Linux `.tar.gz`; Windows `.zip` from
  `add-windows-client`). The app archive and the macOS bundle no longer carry `hq`.
- The README's download table names them "Host" and "App" per platform.
Rejected: renaming the crates so the archives read `headquarter-host-…` (touches every crate and
path dependency for a label; the README table gives the clarity). Rejected: one archive with
everything and options to drop parts (ships a host to Windows that cannot run there).

## D2. Install modes

- `install.sh` (no option): full, host and app.
- `--host-only`: `hq` and `headquarter`, and the host service (D4). No app, no desktop entry, no
  macOS signing certificate.
- `--app-only`: the app only (macOS bundle or Linux binary and menu entry).
- `--full`: both again after one of the above (added while building, 2026-11-02: without it a
  rerun keeps the recorded parts, so nothing could return to both).
- The install record (`install-manifest`) gains `parts=host,app`. A rerun without an option keeps
  the recorded parts; an option changes them and removes the part no longer wanted: whatever the
  old record lists and the new one does not is removed (a service stopped first, the host
  stopped before its files go). `--uninstall` removes what is recorded.

## D3. Finding this computer's host

The app looks for the host in this order: `HQ_HOST_BIN` (tests), the `hq` beside the app
(development builds in `target/`; added while building so a dev app never starts the installed
host), the `hq` named in the install record, `~/.local/bin/hq`. Found, "this computer" is in the host switcher and the app starts it
when it is not running (as today). Not found, the app never starts a host; the first launch
opens Connect to a Host. `hq` itself still starts the host from its own executable.

## D4. The host as a service (host only)

- Linux: a systemd user unit `headquarter-host.service` (`ExecStart=%h/.local/bin/hq daemon`,
  `Restart=always`), enabled and started by the installer. To run with nobody logged in, the
  installer asks for lingering (`loginctl enable-linger`); if the system refuses, it prints the
  one line for the owner: `sudo loginctl enable-linger <user>`. Without systemd (containers,
  WSL without it), it prints how to start `hq daemon` instead.
- macOS: a LaunchAgent `io.github.alfonsusenrico.headquarter.host` (`RunAtLoad`, `KeepAlive`),
  which runs while the owner is logged in.
- Both carry `HOME`, and `HQ_DATA_DIR` and `HQ_CONFIG_DIR` when the installer ran with them, so
  the service uses the folders the install did. `HQ_SERVICE=files` writes the file without
  starting it, for the installer test, which must never reach the machine's own launchd or
  systemd.
- `hq stop` with `Restart=always` restarts the host on the new binary, which is how a host-only
  update takes effect; stopping for good is `systemctl --user stop headquarter-host` (README).
- The host already refuses a second instance (single-instance lock), so a client starting `hq`
  next to the service is harmless.
Rejected: a system-wide service as root (agents must run as the owner, with their logins).

## D5. Updates per part

- The app's updater runs the installer with the recorded parts. With a full install it updates
  host and app and restarts as today; with an app-only install it updates and restarts the app.
- A host-only install updates by rerunning its one-liner; the installer then says how many
  sessions are running and that `hq stop` switches the host to the new version (sessions resume).
- A client and host of different protocols: the app says which side is older and shows the line
  that updates it (from `add-remote-desktop-client` D9).

## D6. README

The Install section becomes: what to install where (a table: this computer / a server / a
laptop or Windows PC); the full one-liner; the host-only one-liner; the app downloads per
platform with the Windows SmartScreen note; pairing a client with a host (`hq remote pair` on a
server, Connect to a Host in the app); updates and uninstall per mode. The alpha's own section
says how to install a pre-release (`HQ_VERSION=0.2.0-alpha.1`) until 0.2.0 is out.

## D7. Release section

`AGENTS.md` `## Release`: Kind names the host archives, the app archives incl. the Windows zip,
and the APK; Verify counts the assets anew and checks the macOS app archive has no `hq`.

## D8. The host on Linux, first run

A Linux container on the owner's Mac (Docker, arm64 Ubuntu) installs from local archives with
`--host-only` (`HQ_INSTALL_FROM`), runs `hq daemon` (no systemd in the container: the fallback
message is the evidence for D4's branch), and the Mac app pairs with it as a remote host and runs
a stand-in session. The systemd path is checked with `systemd-analyze verify` and, for real, on
the owner's server.

## Risks

- Existing full installs: the rerun keeps both parts, the bundle loses its copy of `hq`, and the
  app finds the host through the record. Tested by upgrading a sandboxed 0.1.24 install.
- Lingering may need the owner's sudo once on the server.
- The alpha APK has version code 200 like 0.2.0: installing 0.2.0 over the alpha is a reinstall,
  which Android allows.
