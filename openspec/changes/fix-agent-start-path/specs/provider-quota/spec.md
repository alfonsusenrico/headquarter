# Spec Delta

## ADDED Requirements

### Requirement: Usage failures point at the provider
The system SHALL tell the owner, when a provider does not report its usage, which provider it was and what to try, without internal detail, SHALL record the reason in the host log, and SHALL try a failed Codex read again after a minute.

#### Scenario: Codex does not report
- **WHEN** Codex's usage read fails
- **THEN** the details read "Codex didn't report its usage. Open Codex once to check it is signed in, then refresh." and Codex is read again a minute later

#### Scenario: Claude Code asks to trust a folder
- **WHEN** Claude Code's hidden usage read stops at its folder-trust question
- **THEN** the details say Claude Code asked to trust a folder and that opening Claude Code once and answering fixes it
