## MODIFIED Requirements

### Requirement: Files stay on their computer
With a remote host's session on screen, dropping images onto its terminal, or pasting an image copied to the clipboard, SHALL send each image to that host and paste the path it answers, as a dropped file's path is pasted on this computer. Dropping any other file SHALL show a notice that only images can be sent to another computer and type nothing. File links SHALL open only when the host is this computer.

#### Scenario: Dropping a file onto a remote session
- **WHEN** the owner drops a PDF from the PC onto the server's session
- **THEN** a notice says only images can be sent to another computer, and nothing is typed into the session

#### Scenario: Dropping a screenshot onto a remote session
- **WHEN** the owner drops a PNG from the PC onto a Claude Code session on the server
- **THEN** "Sending image…" shows, the image is saved on the server, and Claude Code's box shows `[Image #1]`

#### Scenario: Pasting a screenshot into a remote session
- **WHEN** the owner takes a screenshot to the clipboard on Windows and presses the terminal's Paste key in a Codex session on the server
- **THEN** the image is sent as a PNG and Codex's box shows `[Image #1]`

#### Scenario: Pasting text is unchanged
- **WHEN** the clipboard holds text and the owner pastes into a remote session
- **THEN** the text is pasted as before and nothing is uploaded
