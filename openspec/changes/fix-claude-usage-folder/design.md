# Design: Claude usage reads in a folder Claude trusts

## Decisions

### D1. Claude's own trust record picks the folder

The read's folder is the most recently active Claude session's folder that exists and that
`~/.claude.json` marks `projects["<folder>"].hasTrustDialogAccepted: true`. Only that field is read,
read-only, each time a read starts (the file is small; Claude rewrites it often). When the file
cannot be read or has no `projects` table, the choice falls back to today's (most recent existing
folder), so a format change degrades to today's behaviour rather than to no reading.

- Rejected: skipping only the home folder. Any folder can be untrusted, and Claude's record is the
  source of truth.
- Rejected: a fixed scratch folder Headquarter owns. Claude would ask to trust it, and answering is
  never Headquarter's to do.

### D2. Refresh after a failure reads again

`refresh_usage` skips a new Claude read within `REFRESH_FLOOR` only when the last read succeeded;
the same for Codex and agy. A failed read is quick, so this cannot pile up reads; the in-progress
guard stays.

### D3. The message

The trust failure says: "Claude Code asked whether to trust a folder before showing its usage.
Open Claude Code once in one of your project folders and answer, then refresh." When no session
folder is trusted at all, the read fails with that message without starting Claude.

## Risks

- `~/.claude.json` is Claude's internal file (an unstable external contract): parsed defensively,
  one field, unknown fields ignored, and the fallback in D1.
