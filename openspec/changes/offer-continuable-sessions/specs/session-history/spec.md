# Spec Delta

## ADDED Requirements

### Requirement: Conversations the owner started
The system SHALL treat as the owner's own only top-level conversations a person started: SHALL NOT import an agent's sub-agents as conversations, SHALL mark runs a program started (Claude Code's `claude -p` or Agent SDK, Codex's `codex exec`) and agy conversations never used as not offered, SHALL mark a past conversation its agent no longer keeps as not offered, and SHALL keep every such row in its index and in `hq ls --all`.

#### Scenario: Codex sub-agent
- **WHEN** Codex keeps a rollout whose source is a sub-agent
- **THEN** it is not imported as a conversation of its own

#### Scenario: Scripted run
- **WHEN** a Claude Code transcript's entrypoint is `sdk-cli`
- **THEN** its row is listed by `hq ls --all` with `offered` false

#### Scenario: Conversation the agent deleted
- **WHEN** Claude Code deletes an old transcript whose conversation Headquarter had imported
- **THEN** after the next history pass its row is still listed by `hq ls --all`, with `offered` false, and clients are told of the change
