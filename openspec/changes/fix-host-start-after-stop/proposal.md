# Proposal

## Why

The owner installed 0.1.9 and pressed "Restart to finish update" (2026-10-24). The 0.1.8
host stopped, but Headquarter reopened with "cannot reach the Headquarter host: the
Headquarter host did not start". Evidence: the host log shows the stop from 19:57:08.898
to 19:57:09.810 (seven agents and a shell to end); the reopened app started at 19:57:09;
no new host logged a start. A stopping host removes its control socket first but keeps
the single-instance lock until it exits; the updater waited only for the socket, so the
new app started a host that found the lock taken and exited at once, and the client
gave up after 3 s. A host started by hand afterwards ran normally.

Approved by the owner on 2026-10-24 ("go ahead").

## Expected Outcome

A host started while another is stopping waits for it to exit, then runs. A host
started while another is running still exits at once with "already running".

## What Changes

- `hq-daemon`: when the lock is held and the control socket is gone, wait up to 10 s
  for the lock instead of exiting.
- `hq-client`: wait up to 12 s for a host it started (the 10 s wait plus start-up); it
  connects as soon as the host answers.

## Impact

`crates/hq-daemon/src/host/mod.rs`, `crates/hq-client/src/lib.rs`, a host test. Needs a
release (0.1.10): every in-app update restarts a host that may still be stopping.
