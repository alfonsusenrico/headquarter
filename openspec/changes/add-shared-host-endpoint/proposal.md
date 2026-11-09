# One connection point for every paired host

Status: proposed 2026-11-09, waiting for the owner's approval (phase 2, step 1 of
`research/notes/2026-11-09-roadmap-plan.md`).

## Why

The desktop app shows one host at a time, and keeps only that host's link open: each `Link`
binds its own iroh endpoint with this computer's one key, and two endpoints with one key on the
same relay server push each other off (`hq-remote/src/client.rs`; the app closes every other link
when it shows or pairs a host, `hq-app/src/hosts.rs`). So switching hosts dials again and shows
"Connecting to <host>…", and pairing a second host drops the first.

The spike (`research/evidence/several-hosts/2026-11-09-shared-endpoint.md`, iroh 1.4.1): one
endpoint held connections to two hosts at once, closed and reopened each on its own, and reached
a host on a relay outside its relay map; two endpoints with one key on one relay broke. The
merged host list (step 2) needs every host connected at once.

## Expected Outcome

- Switching between hosts no longer dials again: a paired host's connection stays open while
  another is on screen, so its sessions show at once.
- Pairing a new host, or forgetting one, leaves the other hosts connected.
- Nothing else changes on screen; the merged list is step 2.

## What Changes

- `hq-remote`: a client endpoint shared by every link in the app process (D1); a link opens and
  closes only its own connection (D2).
- `hq-app` `hosts.rs`: keeps a link per paired host for the app's life; showing and pairing no
  longer close the others; forgetting closes that host's link (D3).
- `desktop-app` spec: "Host switcher" says the other hosts stay connected.

## Non-goals

- The merged list, labels, groups across hosts (step 2).
- The phone, which talks to one computer at a time and is unchanged.

## Capabilities

No new project capability. Modified: `desktop-app`.

## Impact

`hq-remote` (`client.rs`, `lib.rs`), `hq-client` (`Host::Remote` construction), `hq-app`
(`hosts.rs`). No protocol change; hosts and the phone are unaffected.
