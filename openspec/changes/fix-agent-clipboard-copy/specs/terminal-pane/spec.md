# Spec Delta

## MODIFIED Requirements

### Requirement: Clipboard
The system SHALL copy selected text to the system clipboard, SHALL put text the agent asks the terminal to copy (OSC 52) on the system clipboard without ever letting the agent read the clipboard, and SHALL paste clipboard text to the agent, wrapped for bracketed paste when the agent enabled it.

#### Scenario: Paste into an agent prompt
- **WHEN** the owner pastes several lines while the agent has bracketed paste enabled
- **THEN** the agent receives the text between bracketed-paste markers

#### Scenario: Copied by the agent
- **WHEN** the owner selects text with the mouse inside Claude Code, which copies it with OSC 52
- **THEN** the text is on the system clipboard, as in Alacritty

#### Scenario: Agent asks to read the clipboard
- **WHEN** an agent sends an OSC 52 request to read the clipboard
- **THEN** nothing is sent back and the clipboard is unchanged
