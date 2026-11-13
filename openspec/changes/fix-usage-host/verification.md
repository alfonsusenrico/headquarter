# Verification: fix-usage-host

Revision: `fix/usage-host` 687205c (6aaf251 plus `fix/rustix-advisory` 480594d), on `main` 09b9865.

| Check | Result |
| --- | --- |
| `./scripts/check` | passed: 655 tests (1 ignored), cargo-deny, OpenSpec 19/19 |
| Unit tests | the footer's host by each rule, a chosen host that is away, an unknown choice, no host (`several.rs`, 3); the kept choice saved, cleared and odd values ignored with other keys kept (`app_state.rs`, 1) |

cargo-deny failed on 2026-11-13 for RUSTSEC-2026-0486 (`rustix` 1.1.5, published after
`v0.4.0-rc.2`), on `main` as well. `fix/rustix-advisory` changes only `rustix`'s version and
checksum in `Cargo.lock` (`cargo update -p rustix` also re-resolved unrelated `windows-sys` edges;
1.1.6 keeps 1.1.5's `windows-sys` range, and `cargo metadata --locked` accepts the two-line lock).

On screen, 2026-11-13 (debug app on sandbox host A with stand-in Claude and Codex; sandbox host B
with stand-in Codex only, paired through Connect to a Host, default public relays). Evidence:
`research/evidence/fix-usage-host/`.

- One host: the usage details have no "Usage from" row (u5).
- B's session selected: the footer stays on this computer, with no host name (u10).
- "Usage from" lists This Mac and B; choosing B moves the footer to B's rings named
  "Alfonsuss-MacBook-Air" and writes `usage_host` to `app-state.json` (u11, u12, u24, u26).
- A's session selected afterwards: the footer stays on B (u13). New Session still starts on the
  selected session's host, "This Mac" (u14).
- The app reopened: the footer is still on B (u15).
- B stopped: B is "Reconnecting", the footer falls back to this computer, and B is disabled in
  "Usage from" (u16, u17). B restarted: the footer returns to B by itself (u19).
- Forget B: 1 host, the footer on this computer, `usage_host` cleared (u22).
- The first build marked the shown host with a faint fill; it now uses New Session's "Start on"
  style, the shown host as the primary button (6aaf251, u24, u26).

Not verified:
- A client without its own host (Windows): unit test only.
- The owner's Mac and server.
