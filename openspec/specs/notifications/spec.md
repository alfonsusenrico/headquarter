# notifications Specification

## Purpose
How Headquarter tells the owner, through the system's notifications, that a session in the sidebar needs them or has replied while they look elsewhere, and how one click takes them to it.

## Requirements

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

### Requirement: Open from a notification
The system SHALL bring the app forward with the notified session shown when the owner clicks its notification.

#### Scenario: Click
- **WHEN** the owner clicks the "needs you" notification for a session
- **THEN** the app comes forward showing that session in the terminal pane

### Requirement: One notification per session
The system SHALL replace a session's earlier notification with its newer one.

#### Scenario: Replied after asking
- **WHEN** a session needed the owner, was answered, and then finished its turn while the owner was away
- **THEN** only the "replied" notification remains for it

### Requirement: Unavailable notifications are explained
The system SHALL tell the owner once when notifications cannot be shown, such as when the app runs outside an app bundle on macOS.

#### Scenario: Bare binary on macOS
- **WHEN** the app runs from `cargo run` on macOS
- **THEN** the sidebar notes once that notifications need the installed app
