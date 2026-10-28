# Verification

Working tree on `fix/terminal-in-use-elsewhere` (base 3ed195c), 2026-10-28, macOS.

## Automated

- `./scripts/check`: exit 0. 544 Rust tests passed, 0 failed (1 ignored, as before); rustfmt,
  clippy `-D warnings`, cargo-deny; OpenSpec 19 of 19 valid (strict); Android ktlint, Lint
  (warnings as errors), unit tests, debug and unsigned release builds.
- New or extended tests:
  - `hq-protocol` `remote_messages_round_trip`: `Device`, `InUse` with each holder.
  - `hq-cli` `the_terminal_is_sized_for_the_device_in_use`: each client is told it is in use,
    or that another client on the computer is, on attach, `Focus {}`, typing and leaving.
  - `hq-cli` `a_paired_phone_follows_types_and_is_held_to_the_allow_list`: through the remote
    gateway the desktop hears "Test Phone" (the paired name); a phone's own `Device` is refused
    with `not-allowed`, the connection carries on, and the name stays the paired one.
  - `hq-daemon` `the_allow_list_follows_the_owner`: `Device` is refused from phones.
  - `hq-app` `the_pane_names_the_client_in_use_elsewhere`: the notice text per holder.
  - `hq-mobile` `the_open_terminal_says_where_it_is_in_use`: `InUse` is a change, never an
    answer; the computer's or the other phone's name; forgotten on close.

## On screen (desktop)

Synthetic host under `/private/tmp/hq-inuse` (own `HOME`, `HQ_DATA_DIR`, `HQ_CONFIG_DIR`,
`HQ_REMOTE_RELAYS=none`), a stand-in `claude` that wraps a paragraph to the terminal width as
Claude Code does, the dev app, and a stand-in phone: a small program on the real `hq-mobile`
core, paired through remote control as "Test Phone" (scratch, not committed).

- Desktop first, then the phone opens the session (the owner's first case): the desktop shows
  "In use on Test Phone · Click to use here" at the top right over the 54-column drawing.
- Phone first, then the desktop opens it (the owner's second case): the phone core reports
  "in use elsewhere: Alfonsuss-MacBook-Air" over the same garbled wrapping as the owner's
  photo, which the app now covers.
- "Use here" on the phone (`focus`): the agent redraws at 54 columns, nothing covers it.
- Clicking the notice: it goes away, the agent redraws at 69 columns, the phone is covered.
- Typing on the phone: the text reaches the agent and the phone takes over; typing on the
  desktop takes it back.
- The phone closes the session while in use: the desktop takes over without a click.
- Shell panel: the phone opens the session's shell; the panel shows the notice, the agent pane
  above it does not.
- The notice uses the terminal's text colour as its background and the terminal's background as
  its text, so its contrast equals the terminal text's in every theme (checked in the dark
  default; light themes follow by construction).

## Not verified yet

- The Android cover and "Use here" on a real phone: no emulator here, and the attached phone is
  not the dev phone. Waiting for the owner.
- Linux.
- In a narrow pane the notice overlaps the end of the first drawn line (seen at 846 pt wide).
  In a full-size window it sits clear of a phone-width drawing.
