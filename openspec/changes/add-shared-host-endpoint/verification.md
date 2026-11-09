# Verification: add-shared-host-endpoint

Revision: `feat/shared-host-endpoint`, on `main` e4c4ae6 (v0.3.1).

| Check | Result |
| --- | --- |
| Spike, iroh 1.4.1 (`research/evidence/several-hosts/2026-11-09-shared-endpoint.md`) | one endpoint holds two hosts; one key on two endpoints on one relay breaks |
| Host test `one_endpoint_links_two_hosts_at_once` (two real hosts, relays off) | passed: both feeds at once, each lists only its sessions; closing one link leaves the other answering; the closed one dials again |
| Host tests `the_client_works_over_a_remote_link_and_dials_again`, `a_paired_desktop_works_on_several_streams_within_its_allow_list` | passed on the shared endpoint |
| `./scripts/check` | passed: 639 tests, OpenSpec 19/19 |

Not verified yet:
- On screen: switching back to a paired host without "Connecting…" (needs a free Mac).
- A host reached only through a relay outside n0's map, idle more than 60 s (iroh closes unused
  non-home relay connections then): the owner's server is the natural check.
