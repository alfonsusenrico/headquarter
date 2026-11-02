# Host and app install separately

Status: plan, waiting for the owner's approval. Builds on `add-remote-desktop-client` and
`add-windows-client`; the `v0.2.0-alpha.1` pre-release follows all three.

## Why

The owner (2026-11-02) will run the host on a headless server and use the app as a client on
every device they carry. Today the install line always installs the command line, the host and
the app together, and `Headquarter.app` carries its own copy of `hq`. The owner wants the app
downloads to be client-only, per platform, and two install lines in the README: full (host and
app) and host only, so a server install is one copied line.

## Expected Outcome

- The release page offers the host (`hq` with the host; macOS and Linux) and the app (client
  only; macOS, Linux, Windows) as separate downloads.
- README shows two one-liners: full install, and host only with `--host-only`; the app alone
  is a download (Windows) or `--app-only` (macOS, Linux).
- A host-only install keeps the host running as a user service, so a remote client always finds
  it, and `hq remote pair` pairs a client from an SSH session.
- The app connects to this computer's host when one is installed, and otherwise opens Connect
  to a Host. It never starts a host it did not find.
- Updates keep the same parts: the app's updater updates what the full or app-only install put
  there; a server reruns its one-liner.

## What Changes

- Release archives: the app archive without `hq` (D1).
- `install.sh`: `--host-only`, `--app-only`, parts recorded and kept on rerun and uninstall (D2).
- The app finds the local host through the install record, not its bundle (D3).
- Host-only installs run the host as a user service (systemd on Linux, launchd on macOS) (D4).
- Updates per part (D5); README install section rewritten (D6); `AGENTS.md` release section
  (D7).
- A first real run of the host on Linux, in a container on the owner's Mac (D8).
- Specs: `release-install`, `desktop-app`, `session-host`.

## Non-goals

- Moving the owner's existing sessions and data to the server (a later change: Headquarter's
  own data and host key; projects are Pakem's).
- A Windows installer script (the alpha is a zip).
- Renaming the archives' crate names (D1).

## Capabilities

No new project capability. Modified: `release-install`, `desktop-app`, `session-host`.

## Impact

`scripts/install.sh`, `crates/hq-app` (local host lookup, updater), `crates/hq-client` (start
path), `dist-workspace.toml`, `README.md`, `AGENTS.md` (Release), a service file template.
