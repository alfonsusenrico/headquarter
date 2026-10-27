## ADDED Requirements

### Requirement: Sessions already in a folder
The New session window SHALL show how many sessions each folder holds, sidebar sessions and past conversations together. Starting in a folder that holds sessions not in the sidebar SHALL first list only those, each with its agent, title and when it was last used, to continue one or start a new session, unless the owner chose not to be asked again about them. Starting in a folder whose sessions are all in the sidebar SHALL start at once.

#### Scenario: Browsing
- **WHEN** the owner browses to a folder holding one sidebar session and two past conversations
- **THEN** its row shows "3 sessions"

#### Scenario: All in the sidebar
- **WHEN** the owner starts a session in a folder whose only session is in the sidebar
- **THEN** the new session starts without a question

#### Scenario: One forgotten session
- **WHEN** the owner starts a session in a folder with two sessions, one in the sidebar and one not
- **THEN** a dialog lists only the one not in the sidebar, and Continue adds it to the sidebar and resumes it

#### Scenario: Don't ask again
- **WHEN** the owner ticks "Don't ask again for these" and starts a new session
- **THEN** later starts in that folder do not offer those sessions, and a session forgotten later is still offered
