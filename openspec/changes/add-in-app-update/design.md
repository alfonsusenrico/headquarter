# Design

## Context

Releases are built by `dist` on a `v*` tag; `scripts/install.sh` (published with each
release) verifies the archives against their `.sha256` files, installs `hq` and
`headquarter` into `~/.local/bin`, assembles and signs `~/Applications/Headquarter.app`
on macOS, and records what it installed in `~/.local/share/headquarter/install-manifest`.
The host keeps running its old program after an install. Since `add-inactive-sessions`,
stopping the host marks its running sessions, and the next host starts them again on
their conversations. The host's welcome already carries its version.

Checked 2026-10-24 against the repository: `HEAD /releases/latest` answers 302 to
`/releases/tag/v0.1.8`; the release API lists `digest: sha256:20ce…` for `install.sh`,
which matches the file.

## Decisions

### D1. The check: one redirect, through curl

`curl -fsSI -o /dev/null -w '%{redirect_url}' https://github.com/<repo>/releases/latest`
gives the newest tag without the API's 60-an-hour limit; pre-releases are never
"latest". The tag is compared as `major.minor.patch` with the app's own version; only a
newer one counts. At start (after the first list), every 6 hours while open, and on
Check for Updates…. An automatic check that fails stays silent; a manual one says why.
Rejected: a Rust HTTP client (a TLS stack and new dependencies for one request; the
installer already needs curl), the releases API for the check (rate-limited).

### D2. Installed by the installer, or not

The update installs in place only when the installer's manifest lists where the running
app is (the bundle holding the executable on macOS, `headquarter-app` on Linux). The
installer is then run with `HQ_INSTALL_DIR` set to the folder of the recorded `hq`, so it
replaces the same copy. Otherwise (a dev build, another install) the modal says a newer
release exists and offers the release page.

### D3. Install: the release's own installer, verified

The app downloads that tag's `install.sh` into a temporary folder and reads the file's
SHA-256 digest from the release API (one request per update), refusing to run it when
the digest is missing or differs. It runs `sh install.sh` with `HQ_VERSION=<tag>`,
`HQ_INSTALL_DIR` and `HQ_PROGRESS=1`, in a session of its own, with its output going to
a file the app reads every 200 ms; so quitting the app mid-install does not cut the
installer off. The installer verifies the archives and installs only after all of them
verified, as today; a non-zero exit shows its last error line, and nothing was changed.
Rejected: reimplementing the installer in Rust (two copies of the signing logic);
adding `install.sh` to `sha256.sum` (needs a release-job change; GitHub's digest gives the
same assurance today).

### D4. Installer progress

With `HQ_PROGRESS=1` the installer also prints `progress <step>` lines: `download`,
`verify`, `install`, then `progress done <version>`. The modal shows Download, Verify,
Install and Restart, each pending, running or done. Without the variable its output is
unchanged.

### D5. Busy sessions and waiting

Busy means a live sidebar session that is working (background work included) or needs
the owner: restarting either loses the turn in progress. With any busy, Update Now first
shows "2 sessions are busy: Fix login, Release notes. Restarting interrupts them.", with
Restart Now and Wait. Wait installs at once and keeps the restart pending; the pill reads
"Restart to update" and the restart runs as soon as no session is busy.

### D6. The restart

1. The app sends `Stop`; the host marks its running sessions and ends them.
2. The app waits until the host's socket is gone (at most 10 s).
3. It starts a detached helper (`sh -c`, own session) that waits for the app's process to
   end, then opens the new copy (`open <bundle>` on macOS, `<bin>/headquarter-app` on
   Linux).
4. The app quits. The new app starts the new host, which starts the marked sessions; the
   session the window showed starts first.

### D7. Finishing an update made elsewhere

At connect, when the host's welcome names an older version than the app's, the app shows
the pill "Restart to finish update" and offers the same restart (D5, D6). This also covers
an update installed with the install line, and an app quit while a restart waited.

### D8. Setting and menu

`[updates] check = true` by default; false stops the automatic checks; applied without a
restart like the notification settings. Check for Updates… sits in the Headquarter menu
on macOS. Linux has no menu bar: it gets the automatic checks and the pill.

### D9. Testing without touching the installed copy

`HQ_UPDATE_BASE` (tests and dev only, like `HQ_HISTORY_INTERVAL_SECS`) replaces
`https://github.com/<repo>` and the API base, so a local server can play a release. The
installer's own `HQ_INSTALL_FROM` passes through the environment, so the archives come
from a local folder. The on-screen check runs the dev app with `HOME` set to a scratch
folder and stub agents, so the installer writes only there, never to the owner's
`~/Applications` or `~/.local/bin`.

## Failure behaviour

- No network or GitHub unreachable: automatic checks stay silent; Update Now says
  "Couldn't download the installer: …" and nothing changes.
- Digest mismatch or missing: "The installer did not match its published checksum", and
  it is not run.
- The installer fails: its last error line is shown; the installed copy is unchanged (it
  installs only after verifying everything).
- The host does not stop within 10 s: the app says so and does not relaunch; the new
  version is installed and D7 offers the restart again next time.

## Risks

- `curl` missing: the check says so (it is on macOS and on the Linux systems the
  installer supports, which need it).
- Network access: the check sends one request to github.com every 6 hours; nothing about
  the owner or their sessions is sent.
- The first version with the updater can only update to releases whose installer knows
  `HQ_PROGRESS`; without its lines the modal shows "Installing…" until the exit.
