# Design: Headquarter checks an agent after it updates

## D1. Noticing a new version

Discovery already probes each agent's version. The host keeps the last checked version per agent in
`agent-checks.json` in its data folder. At start and with each discovery refresh, an agent whose
version differs (or was never checked) is checked once, after the first check of the day only.

## D2. The free checks

- Status at start: the agent runs in a hidden terminal in a folder it already trusts (Claude's from
  its own record, Codex's and agy's last session folder), Headquarter answers its terminal queries,
  and the check passes when a root OSC 7501 report arrives within 20 s. It is hung up after.
- Usage: the agent's own usage read (Claude's hidden `/usage`, Codex's app-server, agy's `/usage`),
  passing when it gives at least one window.
Results are logged as `agent check agent=… version=… status=ok|failed usage=ok|failed`, never with
screen text.

## D3. Telling the owner

A failed check sends the result with the agent list (optional field), and the app shows an error
notice that stays until dismissed: "<Agent> <version>: Headquarter can't read its usage" or "…: its
status reports are missing; status falls back to output activity". It shows once per version.

## D4. The repository script

`scripts/agent-check [--turn]` runs the same free checks against the installed agents through an
isolated test host, and with `--turn` one real turn per agent ("Reply with just OK.") in scratch
folders, checking working then done. It writes `research/evidence/agent-checks/<date>.md` with the
versions and results, no conversation text. Real turns run only on the owner's go.

## Risks

- A check that starts an agent can trip its own update prompt: launches pass the same per-launch
  options as sessions (for Codex, no update check).
- A first-run screen (login, trust) fails the status check; the notice says what failed, and the
  owner opens the agent once.
