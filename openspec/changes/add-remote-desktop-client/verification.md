# Verification: add-remote-desktop-client

## Revision tested

Branch `feat/remote-desktop-client`, working tree on top of `0d9bd37` with the fixes found on
screen (committed after this record as listed in the commit log). macOS 27 on the owner's
MacBook Air, Rust toolchain from `rust-toolchain.toml`, debug builds.

## Automated checks

| Command | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed: 596 tests, 0 failed; clippy and fmt clean; cargo-deny ok; OpenSpec 19 of 19 valid |
| `cargo test -p hq-cli --test host` | 77 passed |
| `cargo test -p hq-app` | 218 passed, 1 ignored |
| `cargo test -p hq-remote` | 8 passed |
| `openspec validate add-remote-desktop-client --strict` | valid |

New or extended tests for this change: host tests
`a_paired_desktop_works_on_several_streams_within_its_allow_list` (now also: the desktop's own
feed and list leave out its own name while the computer's clients see it; fails without the fix
with "its own change names no device"), `a_desktop_has_at_most_64_streams`,
`a_desktop_lists_the_hosts_folders_and_a_phone_may_not`,
`the_client_works_over_a_remote_link_and_dials_again`; app unit tests
`a_host_on_another_release_says_which_side_to_update` and
`a_host_that_removed_this_computer_asks_to_pair_again_and_a_drop_retries` (both fail on the old
lookup), the setup decision with a remote host on screen, the hosts file, and the remote
folder source.

## On screen (2026-11-02)

Two sandboxes on this Mac, synthetic data only: host A (`HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR` under `/private/tmp/hqA`) with the dev app, and host B under `/private/tmp/hqB`
as the "server", running a stand-in `claude` (echoes input) in `~/projects/web`. The owner's
real host and sessions were not touched. Captures of the app window only, kept outside the
repository; none shows a pairing code.

| Check | Result |
| --- | --- |
| Switcher on a fresh app | "This Mac ⌄"; list: This Mac (ticked), Connect to a Host… |
| Connect with an empty field | "Paste the code the host shows: …" in amber |
| Connect with `hq remote pair`'s code (direct addresses, relays off) | "Paired with <host>."; sidebar names the server and lists its `claude-1` |
| Open `claude-1`, type a line | The line reaches the agent and its echo comes back |
| New session (⌘N) | Folder browser lists the server's home (`Library`, `projects`, `server-notes`), not this Mac's (`laptop-only`) |
| Find `api` in the browser | `~/projects/api` found on the server; Start session creates `claude-2` there (`hq ls` on B) |
| Remote Control… with the server on screen | Notice: "Remote Control pairs devices with this computer's host. Switch to This Mac to use it." |
| Switch to This Mac and back | Terminals close; each host's list shows |
| ⌘Q with the server on screen | App quits at once, no stop question; B's sessions keep running |
| Reopen with an unreachable server | Pane "Connecting to <host>…" (after the fix; before it said "Starting Headquarter…"); sidebar "Reconnecting to <host>…" |
| Pair again over n0's relay; stop and restart host B with a session open | Sidebar "Reconnecting to <host>…", rows greyed; within 40 s the list and the terminal return without action |
| Close `claude-2` from the client | Confirmation, then the session leaves B (`hq ls`) |
| App log over launch, switch, reconnect | No `Endpoint dropped` and no `Another endpoint connected with the same endpoint id` after the link fix (3 of them before) |
| In-use mark | Before the fix the client marked its own session "Open on <this computer>"; after it, no mark on the client while B names the desktop |

## Keystroke round trip

One key typed into `claude-1` on host B, timed until the terminal echo arrives; 30 samples
each, 50 ms apart, after 3 s to settle. Probe: `research/prototypes/remote-desktop-rtt`
(desktop key from sandbox A, never printed).

| Path | Median | p95 | Max |
| --- | --- | --- | --- |
| Local socket (baseline) | 0.2 ms | 0.3 ms | 0.4 ms |
| Link, direct (path rtt 2.4–2.6 ms) | 1.2–1.3 ms | 1.8–1.9 ms | 2.3 ms |
| Link, relay only (n0 `aps1-1`, path rtt 82–88 ms) | 79.0–80.3 ms | 82.5–103.2 ms | 119.5 ms |

The first default-path run measured 39.7 ms median while hole punching was still settling;
later runs on the selected direct path match the table. Both ends are on one Mac, so "direct"
is a loopback path; a LAN or the internet adds its own latency.

## Not verified

- The removed-device message on screen: the screen locked before the capture. The message and
  the stop of retries are covered by the unit test above, and the host closes the connection
  with `removed` in the host tests.
- Dropping files onto a remote terminal: dragging cannot be driven reliably here; the notice
  path (`DropRefused`) and the blocked file links were checked by reading the code.
- The updater with a remote host on screen: no update was available; the code path skips the
  host check and does not stop a remote host.
- A server on another machine and a Windows client: `add-windows-client` and
  `split-host-and-app-install` cover them.
