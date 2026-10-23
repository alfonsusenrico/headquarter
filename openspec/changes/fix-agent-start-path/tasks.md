# Tasks

## 1. Start path and messages

- [x] 1.1 `agent_path` and its use at every agent start; the host remembers the login `PATH` and holds usage reads until it settles. Verify with `an_agent_runs_with_its_own_folder_first_then_the_login_path` and the host tests. Done: workspace tests and lint pass (2026-10-22).
- [x] 1.2 Provider-facing failure messages, the trust message, and the one-minute Codex retry. Verify with the host tests (`a_folder_trust_question_stops_the_usage_read_unanswered`). Done.
- [x] 1.5 One generic line for an agent whose usage cannot be read: "Agy usage not available" (owner, 2026-10-23). Verify with `agy_is_unavailable_with_reason` and the `hq usage` render test. Done (2026-10-23).
- [x] 1.4 Keep tests off the machine's own agents: with the agent's folder on `PATH`, the Claude usage tests started the real Homebrew Codex (its `node` was now reachable) with the test's home, which left `~/.codex` files behind in `/tmp/hqh-*-usagetrust`. `watch_claude_usage` now stubs `codex`. Verify: the usage tests pass and leave no `/tmp/hqh-*` folder. Done (2026-10-22).
- [ ] 1.3 In the installed app opened from Finder: Codex's usage reads at start, setup shows Codex's version, and a Codex session starts.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; push and open the PR.
