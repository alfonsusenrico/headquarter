# Keep Claude Code sessions on their own conversation

Status: approved by the owner on 2026-10-27 ("approve, go ahead").

## Why

On 2026-10-27, after the app restart at 15:47, the owner's Headquarter dev session came back
as a conversation two hours old, could not be continued, lost its sidebar row twice, and showed
three copies in Add sessions. Findings: `research/evidence/claude-daemon-sessions/findings.md`
and the other agent's `research/evidence/claude-continued-conversation/findings.md` (branch
`fix/claude-continued-conversation`, not merged). Four defects together, all from Claude Code
2.1.311 behaviour the host does not follow:

1. **Continued conversations.** Claude Code sometimes carries a resumed conversation on under a
   new id. It writes a `continued-in` record in the old transcript and sends no hook. The index
   kept the old id, so the next start resumed the old conversation, which Claude Code then
   forked again.
2. **Background sessions.** Claude Code can keep a conversation running in its own daemon
   (`claude agents --json` reports it as `kind: background`) after the host's agent ends. A
   resume then fails at once (exit 1), and the session shows "In another terminal" with no way
   forward in the app. The CLI's `/stop` ends it, and `claude stop <id>` did not for the owner.
3. **Deleted rows.** The urgent fix merged "duplicate" rows by deleting the one imported from
   history, which was the owner's named sidebar row.
4. **Copies in Add sessions,** and a sidebar row that left the sidebar three times with nothing
   in the host log.

## Expected Outcome

- A Claude session always resumes its newest conversation, so a restart never brings back an
  older one, and the index follows the new id as soon as Claude Code continues it.
- When a session's conversation is still running in Claude Code's background, the app says so
  and offers to bring it back into Headquarter, instead of failing silently.
- A row in the sidebar or named by the owner is never deleted by any merge.
- Add sessions shows one entry per conversation chain.
- Adding, removing and closing sessions are logged with ids, so a vanishing row can be traced.

## What Changes

- `hq-signals::history`: read a transcript's `continued-in` records (taken from the other
  agent's branch, with its tests).
- `hq-daemon`: follow a running session's continuation as soon as it appears, and record it.
  Resume follows the record from the stored id only as a fallback. Merges keep the sidebar or
  owner-named row and delete only unnamed history rows outside the sidebar. A resume of a
  conversation Claude Code reports running in the background returns a clear error the app
  can act on. Add, Remove and Close are logged (ids only).
- `hq-app`: the "running in Claude Code's background" state on the session pane, with "Bring
  into Headquarter". Add sessions shows one entry per chain.

## Non-goals

- Changing Claude Code, or editing its transcripts.
- Codex and agy.
- The three approved features, which follow this fix.

## Capabilities affected

`session-host`, `desktop-app`. No new project capability.

## Impact

Host and app. No index schema change. Protocol 12: the `RunningInBackground` error, the `BringIn`
request and the `background` flag on outside sessions. The other agent's branch is folded in, then deleted with the owner's go.
