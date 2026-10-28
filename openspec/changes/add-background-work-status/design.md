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

`research/prototypes/background-work-status/index.html`, light and dark, two to four variants
over real-shaped rows (made-up titles), for the sidebar row, the header and a phone row, for:

- an idle Claude session with two background shells and a sub-agent;
- an agy session with an artifact to review;
- a working session that also has background work.

Variants to draw from, among others: a second glyph or badge beside the status glyph; a status
word with a count ("idle · 2 shells"); a distinct status ("in background") with its own glyph;
hover or tap detail listing the work. Each with its contrast and how it reads without colour.

## Decisions

Made after the research and the owner's choice on the options page, then written here, in the
spec delta and in the tasks before building. Fixed already:

- D1. The five statuses keep their meaning. Background work and reviews are extra facts beside
  the status, so needs-you ordering and unread rules do not change.
- D2. Every reader is per-launch and read-only, like today's (no agent configuration changed).
- D3. Unknown or unreadable signals show nothing extra, never a guess.

## Risks

- Agent formats are unstable external contracts. Each reader tolerates unknown fields and is
  covered by fixtures from the observed versions.
- A process-tree fallback can mistake MCP servers or language servers for background work. It is
  used only if the research shows no better signal, and only for counts the owner chooses to see.
