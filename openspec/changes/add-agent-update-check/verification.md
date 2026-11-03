# Verification: add-agent-update-check

## Revision tested

Branch `feat/agent-update-check` at `7ec5be4` (on `main` after `v0.2.0-alpha.1`), macOS on the
owner's MacBook Air. Installed agents: claude 2.1.317, codex 0.166.0, agy 1.4.12.

## Automated checks

| Check | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed: 611 tests, 0 failed |

New tests:
- `hq-daemon` `agent_check`: a root report at start passes; a report while asking about trust
  passes; a child report alone with a trust question is "not checked" and ends the wait early;
  an agent that exits silent fails; a rewritten executable changes the fingerprint; the version
  number is taken from `--version` answers.
- Host test `a_new_agent_version_is_checked_once_and_a_failure_is_reported` (stand-in agy):
  1.0.0 checked and passing; 2.0.0 with an unreadable `/usage` reaches usage subscribers as a
  failed check; after a restart 2.0.0 is not checked again and the failure is still in `Usage`;
  `hq agents --check` prints it and exits 1; nothing the agent printed is logged.
- `hq-protocol`: `CheckAgents`, `AgentChecks` and `AgentUsage.check` round trip; no `check` is
  sent while every check passes. `hq-daemon` remote: `CheckAgents` refused to phones and
  desktops. `hq-cli`: the printed results. `hq-app`: the notice text.

## The agents' start, probed first

`research/evidence/add-agent-update-check/start-probe.md`: all three report OSC 7501 as they
start without a prompt (Codex only in a folder it trusts); none wrote a conversation; agy leaves
a crash log when killed, so checks hang up.

## Against the installed agents

`research/evidence/agent-checks/2026-11-03.md` (`scripts/agent-check --folder cash-flow-tracker
--turn`, owner's go 2026-11-03):

- Free checks: claude, codex and agy all `status at start: ok`, `usage: ok`, in about 25 s; no
  new Claude transcript or Codex session file.
- One real turn each: Claude Code working, then idle after 2 s; Codex after 5 s. agy's turn did
  not run: agy trusts no existing folder, and Headquarter never answers its trust question.

## On screen

The app (debug build) with a sandbox host (own home, data and settings) and a stand-in agy 9.9.9
whose `/usage` answer is unreadable, `HQ_AGENT_CHECK_SECS=2`:

- The host logged `agent check agent=agy version=9.9.9 status=ok usage=failed` about 4 s after
  starting; the app showed "Antigravity CLI 9.9.9: Headquarter can't read its usage." in the
  notice, with the error border, staying (`notice-failed-usage.png`).
- After the app restarted (the host kept running), no notice (`notice-not-again.png`); the app's
  state remembers `agy 9.9.9`, and the host did not check 9.9.9 again.

The sandbox was removed afterwards.

## Not verified

- agy's real turn (needs a folder agy trusts).
- A real agent update on this Mac triggering the automatic check: the trigger is covered by the
  host test with a rewritten stand-in.
