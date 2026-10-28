# Showing background work and reviews in a session's status

Status: proposed 2026-10-28, waiting for the owner's approval. The owner asked for plans only:
no implementation until approved, after a few days of using 0.1.17. The design is chosen from an
options page first (owner: "of course in design variance").

## Why

The owner's findings (2026-10-28):

- A Claude Code session shows idle while it still runs work it started: sub-agents, background
  shell commands, monitors and other processes.
- An agy session shows idle (since fix-agy-artifact-review, unread then idle) while an artifact
  waits that the owner can open, approve or reject. The review is optional, but worth flagging.
- Codex, and other agents, may have similar cases not seen yet.

Today's statuses are five words (working, needs you, idle not read yet, idle, not running). The
spec already counts Claude background shells and Codex sub-agents as working (Background work
counts as working), but the owner sees idle in cases it misses, and "working" alone does not say
that the prompt is free while something runs behind it.

## Expected Outcome

- A session whose agent is waiting at its prompt while work it started still runs says so,
  distinctly from a session that is working on a turn: what kind of work (sub-agents, shells,
  monitors) and how many where known.
- A session with something to review says so (agy's artifacts first), without counting as needs
  you.
- The same signals show wherever the status shows: the sidebar row, the session header, the
  phone's rows and header. Whether they notify is decided with the owner (open question).
- Each signal comes from what the agent itself reports or records, read defensively, and the
  observed versions are recorded in `research/evidence/`.

## What Changes

Final shape decided after research and the options page:

- `hq-signals`: readers for each agent's background work and reviews (Claude Code's session
  record and hooks, Codex rollouts, agy's reports, and a process-tree fallback if needed).
- `hq-protocol`: the session's background work and review on `SessionInfo` (a protocol bump).
- `hq-app` sidebar, header and usage of the status glyphs, `hq-mobile` and the phone app's rows.
- `agent-status` spec: the requirements below, made final.

## Non-goals

- Changing the five statuses' meaning or the needs-you rules.
- Acting on the background work (stopping a shell or approving an artifact from Headquarter).
- Agents beyond Claude Code, Codex and agy (new agents come with add-more-providers).

## Capabilities affected

`agent-status`, `session-sidebar`, `android-app`. No new project capability.

## Impact

Desktop app, host and phone app, with a protocol bump. Verified against the installed agents
when built (Claude Code 2.1.31x, Codex 0.164.x, agy 1.4.x at the time of writing).

## Open questions for the owner

- Should finishing background work notify, like a finished turn does?
- Should the agy review flag stay until the artifact is reviewed, or clear once the owner opens
  the session?
