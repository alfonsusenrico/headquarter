# Spec Delta

## MODIFIED Requirements

### Requirement: Notify when needed
The system SHALL show a system notification when a session in the sidebar starts needing the owner or finishes a turn, SHALL NOT notify about sessions outside the sidebar or about a session running in another terminal, and SHALL NOT notify about the session the owner is watching in the focused app.

#### Scenario: Permission prompt while away
- **WHEN** the sidebar session "Fix login" asks for permission while the owner works in another app
- **THEN** a notification titled "Fix login" appears, reading "Claude needs your permission", with no folder

#### Scenario: A reply while away
- **WHEN** the Codex session "Release notes" finishes a turn while the owner works in another app
- **THEN** a notification titled "Release notes" appears, reading "Codex replied"

#### Scenario: Watching it
- **WHEN** the session finishes a turn while shown in the focused pane of the active window
- **THEN** no notification appears

#### Scenario: Not in the sidebar
- **WHEN** a session outside the sidebar needs the owner
- **THEN** no notification appears

#### Scenario: App starts
- **WHEN** the app opens while sessions already need the owner
- **THEN** no notification appears for them until their status changes again

#### Scenario: Asking in another terminal
- **WHEN** a sidebar session running in another terminal asks for permission there
- **THEN** no notification is shown

#### Scenario: Session never renamed
- **WHEN** a session the owner never renamed, which the sidebar shows as "Fixing Unresponsive Docker Daemon", finishes a turn while the owner works elsewhere
- **THEN** the notification is titled "Fixing Unresponsive Docker Daemon", not the session's id
