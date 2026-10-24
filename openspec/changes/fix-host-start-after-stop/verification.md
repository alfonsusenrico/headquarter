# Verification

Date: 2026-10-24, branch `fix/host-start-after-stop`. `./scripts/check` passed: 435
tests, clippy `-D warnings`, cargo-deny, OpenSpec strict.

| Criterion | Evidence | Result |
| --- | --- | --- |
| A host started while another is stopping waits, then runs | `a_new_host_waits_for_a_stopping_one` (holds the lock with no socket, as a stopping host does); fails with the wait disabled | passed |
| A running host still refuses a second at once | `single_instance_with_private_files` | passed |
| A command right after `hq stop` reaches a fresh host | `cli_starts_the_host_and_manages_sessions` | passed |

Not verified: the owner's real update restart, which is first possible from 0.1.10 (the
host that starts must carry this fix).
