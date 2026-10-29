# Verification

Working tree on `feat/background-work-status` (rebased onto 9d6ca92), 2026-10-29, macOS.

## Research

`research/evidence/add-background-work-status/findings.md` (real calls on Claude Code 2.1.312,
codex-cli 0.164.0 and agy 1.4.9) and the options page the owner chose from
(`research/prototypes/background-status/index.html`, option E1).

## Automated

- `./scripts/check`: exit 0. 565 Rust tests passed, 0 failed (1 ignored, as before); rustfmt, clippy
  `-D warnings`, cargo-deny; OpenSpec valid (strict); Android ktlint, Lint, unit tests and builds.
- New tests:
  - `hq-signals` `work`: background agents follow their child records (with the recorded
    report and title), a cleared root clears every agent, a background command is its processes (an
    MCP-like process already there never counts), foreground calls and other tools are not followed,
    an agy review lasts until the next report; `resources` descendants and running.
  - `hq-cli` host tests: `a_background_command_counts_while_its_process_runs` (through the real
    hook helper) and `a_background_agent_counts_with_its_title_until_cleared` (the title never in
    the host's log); `claude_records_spare_starting_claude_each_scan` (a record saying "shell" no
    longer shows working).
  - `hq-protocol`: background and review sort with working, needs you wins, the header detail's
    wording.
  - `hq-app`: the row word ends the row, a folded group adds "1 background · 1 review".
  - `hq-mobile`: the phone row's word and detail, rising like working.

## On screen (desktop)

Synthetic host under `/private/tmp/hq-bg` with stand-in agents
(`research/prototypes/background-status/standin.py`) that report like Claude Code and agy, the
Claude one calling the hook command Headquarter passes in `--settings`.

- A Claude stand-in with a background agent ("Phase 4a capture") and two background shells: the row
  says "background"; the header "background · 1 agent, 2 shells · Phase 4a capture", the title
  keeping its width on a narrow pane while the detail shortens.
- The shells' processes end: the header says "1 agent · Phase 4a capture". The agent clears: the row
  shows its age again and the header no detail.
- An agy stand-in with "Approve plan": the row says "review", the header "review · plan to review
  (/artifact)"; it reports done: the row shows its age.
- The Usage window: "1 in background", "plan to review".
- A shell tracked before an app restart stays counted (the host keeps it).

## Own colours and marks (D8), 2026-10-29

Revision 2486806. `./scripts/check` passed: 567 tests, Android build, ktlint, Android Lint, OpenSpec.
New tests: teal and violet at 4.5:1 or better on the sidebar and the open row in the built-in light
and dark themes and all eight templates (Nord's cyan kept as is); a theme file's `background_work`
round trip; the review row keeps its glyph cell.

On screen (the owner freed the Mac), the same stand-ins with one session working, one with a
background agent and shell, one with a plan to review; captures in
`research/evidence/add-background-work-status/colours-{dark,light,nord}.png`:

- Dark: "background" teal with two still dots, "review" violet with a diamond in the glyph cell,
  "working" blue with moving dots; the header word in the same colour as the row's.
- Light: the same in the light values, readable on the sidebar and the open row.
- Nord: "background" in Nord's cyan, "review" in its magenta, "working" in its blue. Nord's cyan and
  blue are close; the words and the still versus moving dots tell them apart.

## Not verified

- The phone's row word, colours and session header on the dev phone (unit test and Lint only).
- A folded group's note and the order on screen (unit tests; same layout code as before).
- Real agents inside the app (the stand-ins follow the recorded formats; the research ran the real
  CLIs).
- Linux.
