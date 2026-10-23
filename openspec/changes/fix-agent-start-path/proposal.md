# Proposal

## Why

On the owner's first install (2026-10-22) the installed app showed Codex's usage as
"not available: codex app-server did not answer in time", and setup listed Codex's
version as unknown. The host log showed the cause:
- Codex's usage read ran 40 ms after the host started, 1.3 s before the login-shell
  `PATH` was known.
- Homebrew's `codex` is a Node script (`#!/usr/bin/env node`). An app started from
  Finder has only the system folders on `PATH`, so `node` was not found.

The captured `PATH` was used only to find agents, never to run them. So Codex sessions
started from the installed app would fail the same way, and the read was not tried
again for five minutes.

The owner also asked that usage errors speak only of the provider's own failure, and
say what to do about it.

## Expected Outcome

- Codex's usage, version and sessions work in the installed app, opened from Finder.
- When a provider does not report its usage, the owner sees which provider it was and
  what to try, with no internal detail.

## What Changes

- `hq-discovery`: `shell::agent_path(executable)` gives each agent a `PATH` made of:
  - the agent's own folder first;
  - then the login-shell `PATH`, once known (`remember_login_path`);
  - then the process's own `PATH`.

  Every place that starts an agent uses it: sessions, Claude Code's hidden `/usage`
  read, `claude auth status`, `claude agents`, `codex app-server` (read and proxy), and
  the version probe.
- `hq-daemon`:
  - usage reads wait until the login-shell `PATH` is settled (known, unavailable or
    turned off, at most 3 s);
  - a failed Codex read is tried again after a minute;
  - failures read "Codex didn't report its usage. Open Codex once to check it is signed
    in, then refresh." (and the same for Claude Code), with the reason in the host log;
  - Claude Code's folder-trust question keeps its own message, since the owner can
    answer it.
- `hq-signals` depends on `hq-discovery` for the helper.

Decisions (owner approved, 2026-10-22):
- **The agent's own folder first.** Homebrew and nvm install `node` beside the CLI, so
  this works even before the login `PATH` arrives. Rejected: running agents through the
  login shell (slower, and runs the owner's startup files every time).
- **Provider-facing messages, technical reason in the log.** Rejected: showing the
  internal reason (the owner found it unclear and not actionable).

## Non-goals

- Reading Codex usage another way (its `/status` screen); the app server remains the
  source.

## Capabilities

### Modified Capabilities
- `session-host`: agents start with a `PATH` that finds their runtime.
- `provider-quota`: usage failures name the provider and what to try.

## Impact

`hq-discovery` (`shell.rs`, `search.rs`), `hq-signals` (`quota.rs`, `codex_server.rs`,
`external.rs`, `Cargo.toml`) and `hq-daemon` (`host/mod.rs`, `host/usage.rs`,
`host/claude_usage.rs`).

## Approval

Approved by the owner on 2026-10-22 ("Codex usage").
