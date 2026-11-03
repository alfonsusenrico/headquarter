# Design: Headquarter checks an agent after it updates

## D1. Noticing a new version

The host has no discovery refresh of its own, so it watches: every minute, starting two minutes
after it starts and once the login-shell `PATH` is known, it resolves each agent turned on in the
settings and compares the executable (resolved path, modified time, size) with what it saw last.
Only on a difference does it ask the agent's version (`--version`, as discovery does). A version
other than the one recorded in `agent-checks.json` in the data folder, or none recorded, is
checked once. The record keeps each agent's checked version and its results, so a version is
checked once across restarts and a failure is still reported after one.

`HQ_AGENT_CHECK_SECS` sets both the first wait and the interval (tests use 1).

## D2. The free checks

Probe on 2026-11-03 (`research/prototypes/agent-start-probe`, no prompt sent): claude 2.1.317
reported `idle` 0.3 s after starting; codex 0.166.0 reported `idle` at 0.9 s in a folder it
trusts and, in one it does not, showed its trust question and reported nothing; agy 1.4.12
reported `blocked` (sign-in, then folder trust) at 0.1 s. None wrote a conversation. agy leaves
a crash log when killed, so the check hangs up (SIGHUP), as a closed terminal does.

- **Status at start:** the agent starts in a terminal no one sees, as the usage reads start it,
  Headquarter answers its terminal queries, and the check passes when a root OSC 7501 report
  (any state) arrives within 20 s. Folders: Claude's is the usage read's (a session folder
  Claude Code trusts); Codex's is its most recent session folder; agy's is the host's own empty
  `quota/agy-workspace`.
- **Usage:** the agent's own usage read, as the usage readers do it (Claude's hidden `/usage`,
  Codex's app-server, agy's `/usage`); it passes with at least one window.
- A check that meets a folder-trust question, or has no folder to run in, is "not checked": it
  says nothing about the new version, so it is logged and not reported.

Results are logged as `agent check agent=… version=… status=ok|failed|not-checked
usage=…`, never with screen text.

## D3. Telling the owner

`AgentUsage` gets an optional `check` (version and each result), present only while the
installed version has a failed check. The app shows a notice that stays until dismissed:
"<Agent> <version>: Headquarter can't read its usage.", "…: its status reports are missing, so
status falls back to output activity.", or both. It shows once per agent and version, remembered
in the app's state. Older apps ignore the field.

## D4. Checking on demand

`hq agents --check` asks the host to check every agent turned on now (request `CheckAgents`,
which the remote gateway refuses to phones and other computers), prints each result, and records
it as an automatic check would. `scripts/agent-check` runs it and writes
`research/evidence/agent-checks/<date>.md` with the versions and results, no conversation text.
With `--turn` it also starts each agent through an isolated test host (its own data folder) in
the folder the check uses, with "Reply with just OK.", checks that the session goes working, then
idle, and closes it. Real turns run only on the owner's go.

## Risks

- Codex's start can show an update prompt; right after an update there is nothing newer, and a
  missing report is then a real failure to look at.
- Codex's most recent folder may no longer be trusted: the check is "not checked", not a false
  alarm.
