# Remote control follows the paired phones

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-10-31): the Remote Control window needs no on/off switch and no folded "For
another phone" step. Remote control should run only while at least one phone is paired, stop when
the last one is removed (to save resources), and the app's QR code should simply be there to scan.

## Expected Outcome

- No switch, in the window or the settings file: remote control runs while at least one phone is
  paired, and while a pairing code is shown; otherwise the host opens no network endpoint.
- Removing the last phone stops it at once.
- The Remote Control window always shows the phone app's QR code and Copy link in full.

## What Changes

- Host: the gateway starts when a phone is paired or a code is shown, and stops when neither holds
  (D1); `[remote] enabled` is accepted but ignored (D2).
- CLI: `hq remote on` and `off` are removed; `status`, `pair`, `devices`, `remove` stay (D3).
- Window: no switch; the app step always open (D4).
- Specs: `remote-control`, `settings`.

## Non-goals

- Pairing, the phone's rights, relays and logging (unchanged).
- The phone app (it already works whenever the computer answers).

## Capabilities

No new project capability. Modified: `remote-control`, `settings`.

## Impact

`hq-daemon` (`host/remote.rs`), `hq-cli` (`remote` commands), `hq-config` (the key kept, ignored),
`hq-app` (`workspace/remote.rs`). Protocol: `RemoteSet` stays for older clients and only reports
the status.
