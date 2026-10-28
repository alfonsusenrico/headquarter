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

Made after the research and the owner's choice on the options page, then written here, in the
spec delta and in the tasks before building. Fixed already:

- D1. Needs you keeps first place, and a session waiting for the owner always shows it, whatever
  runs in the background. Whether background work and reviews become statuses of their own or
  details beside today's five is the owner's choice on the options page.
- D2. Every reader is per-launch and read-only, like today's (no agent configuration changed).
- D3. Unknown or unreadable signals show nothing extra, never a guess.

## Risks

- Agent formats are unstable external contracts. Each reader tolerates unknown fields and is
  covered by fixtures from the observed versions.
- A process-tree fallback can mistake MCP servers or language servers for background work. It is
  used only if the research shows no better signal, and only for counts the owner chooses to see.
