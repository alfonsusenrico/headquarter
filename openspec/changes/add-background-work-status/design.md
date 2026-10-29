# Design: showing background work and reviews in a session's status

## Context

- Statuses: `hq_signals::status::Status` (Working, NeedsYou, DoneUnread, Done, Inactive), set from
  OSC 7501 reports, Claude hooks, and output activity as a fallback.
- Background work today: Claude Code's session record (`~/.claude/sessions/<pid>.json`) says
  `shell` while a background shell command runs after the turn (seen 2.1.310,
  `research/evidence/claude-shell-status/`), and `claude agents --json` reports busy sessions.
  Codex sub-agents are found from their rollouts (`hq_signals::subagents`). Both count as working.
- agy's artifact review: `blocked`, kind permission, message "Approve plan", treated as the end
  of the turn (`adapters::agy::is_artifact_review`,
  `research/evidence/fix-agy-artifact-review/findings.md`).
- Headquarter already samples a session's process tree for the Usage window
  (`hq_signals::resources::Sampler`).

## Research (before the design is fixed)

For each agent, with `hq-probe` and stand-in prompts in a scratch folder, record event types and
fields only, never prompt or output text:

- Claude Code: a Task sub-agent, a background sub-agent, a background shell (`run_in_background`),
  the Monitor tool, and a plain child process. Which show in the session record, `claude agents
  --json`, the hooks (including SubagentStart and SubagentStop where the version has them) and
  the status line input, and when each clears. Why the owner's cases showed idle.
- agy: the artifact review's report and anything that says it was reviewed (approved, rejected,
  opened), and whether agy has background work of its own.
- Codex: background shells or other background work beyond sub-agents.
- Process tree: whether a session's descendants (beyond MCP servers and the agent's own helpers)
  tell background work apart reliably, as a fallback only.

Findings go to `research/evidence/add-background-work-status/findings.md`.

## Options page (before the design is fixed)

`research/prototypes/background-work-status/index.html`, light and dark, with many variants
(the owner asked for a wide choice), over real-shaped rows with made-up titles, each shown on the
sidebar row, the folded group header, the session header, the Usage window row, a phone row and
the phone's session header, for the same set of cases:

- a Claude session on a turn, then at its prompt with two background shells and a sub-agent;
- a Claude session at its prompt with a monitor running;
- an agy session with an artifact to review;
- a Codex session with a sub-agent;
- a session that needs you while background work runs;
- plain idle, unread and not running rows for comparison.

Families of variants, several in each:

- New statuses with their own words and glyphs ("in background", "review ready"), placed in the
  sidebar's order.
- The status word with a detail ("idle · 2 shells, 1 agent", "idle · review ready").
- A second small glyph or badge with a count beside the status glyph.
- A status glyph that changes form (a ring, a dot, a split glyph) while background work runs.
- A compact row by default with the detail on hover (desktop) or in the session header.
- Combinations of the above for desktop and phone.

Each variant states its words, its glyphs, its contrast in both themes, how it reads without
colour, and what the phone shows at its narrow width.

## Decisions

Research done 2026-10-29 with real calls (`research/evidence/add-background-work-status/findings.md`).
The owner chose option E1 on the options page (`research/prototypes/background-status/index.html`),
background work and reviews rising in the sidebar like working, and no notifications (owner,
2026-10-29).

- D1. Needs you keeps first place: a session waiting for the owner always shows it, whatever runs
  in the background.
- D2. Every reader is per-launch and read-only, like today's (no agent configuration changed).
- D3. Unknown or unreadable signals show nothing extra, never a guess.

### D4. Two new row words: "background" and "review" (E1)

A session at its prompt (idle or unread) with background work shows "background" at the row's end,
in place of its age; an agy session with a plan to review shows "review". Working, needs you and
inactive look as today. Both words sort with working, after needs you (the "Order inside each
list" band 1). Neither notifies.

### D5. The detail where there is room

`SessionInfo` gains `background` (running background agents, background commands and monitors, and
the newest agent's title) and `review` (agy has a plan to review). The session header shows
"background" with a chip per kind ("1 agent", "2 shells", "1 monitor") and the agent's title; the
Usage window row says "3 in background" or "plan to review"; a folded group header adds
"2 background · 1 review" to its count; the phone row shows the word and the phone's session header
the detail. The fields are new and optional, so the protocol stays 14 and older clients ignore them.

### D6. Claude: agents from its reports, commands from its processes

- Background agents: the OSC 7501 child records Claude sends per agent
  (`state=working:id=…:title=…`, `state=clear:id=…`), counted while working. The title (base64 in the
  report) is display-only: kept in memory, never written to the index or the log.
- Background commands and monitors: a hook says one started (`PostToolUse` for Bash with
  `run_in_background`, or for Monitor). The host notes the processes that appeared under the agent
  since the matching `PreToolUse` and counts the command until they have all exited. Nothing else
  in the process tree counts, so MCP servers and helpers never show as background work.
- Unknown signals (another version's fields) show nothing extra.

### D7. Codex and agy

- Codex keeps working while its commands run (research), so nothing new; its sub-agents found from
  their rollouts (`hq_signals::subagents`) count as background agents while the main session is at
  its prompt.
- agy: "Approve plan" (`adapters::agy::is_artifact_review`) sets `review` and ends the turn as today
  (idle or unread). It clears on the next report (approve gives `working`, reject `done`) or a new
  prompt.

### D8. Their own colours and marks (owner, 2026-10-29)

As on the options page, and apart from working's accent: "background" in teal with two still dots
after it, "review" in violet with a diamond in the glyph cell. Only working moves; background work
waits on nobody, so its dots stay still and the sidebar stays at rest. The phone row and both
session headers use the same colours, words only.

- Two new chrome colours, `teal` and `violet`, beside `amber`. The built-in themes pick them by
  hand: dark `#67E8C9` and `#C4A7FF`, light `#0E6A5A` and `#6B3FC2`, each 4.5:1 or better on the
  sidebar and the open row. The phone's two themes use the same values.
- Themes made from colours (templates, Alacritty colours, the owner's files) take them from the
  palette's cyan and magenta, stepped toward black or white (`theme::readable`) until they read at
  4.5:1 on the sidebar and the open row. A theme file sets them as `background_work` and `review`
  in `[headquarter.colors]`, and the creator lists them under Status, so a theme now has 32
  colours. Older files without them get the made ones.
- Rejected: reusing the accent (what the first build did). It made "background" look like working,
  which the owner noticed at once. Rejected: animating background's dots. They would keep the
  sidebar redrawing for work that can run for hours, and the word already says it.
- The phone gets an enum (`RowBeside`), not the word, so its colour does not hang on the text.

- Agent formats are unstable external contracts. Each reader tolerates unknown fields and is
  covered by fixtures from the observed versions.
- A command whose processes Claude re-parents or that daemonises itself leaves the count early.
  Accepted: the count can only fall short, never invent work.
