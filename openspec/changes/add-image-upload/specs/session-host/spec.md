## ADDED Requirements

### Requirement: Images sent to a session
The host SHALL accept an image for a session in its index from any client: at most 10 MiB, and only PNG, JPEG, GIF or WebP as its first bytes show, whatever the client says. It SHALL save it under its data folder in a folder for that session, readable by the owner only, and answer with its absolute path. It SHALL delete a session's images when the session leaves the sidebar, and every image 7 days after it arrived. It SHALL NOT log an image's content, name or path.

#### Scenario: A screenshot
- **WHEN** a client uploads a 2 MB PNG to a Claude Code session
- **THEN** the host saves it under `images/<session id>/` in its data folder with mode 600 and answers with the file's absolute path

#### Scenario: Too large
- **WHEN** a client announces an upload of 12 MB
- **THEN** the host refuses it with "The image is larger than 10 MB" before any byte is sent, and saves nothing

#### Scenario: Not an image
- **WHEN** a client uploads a PDF announced as an image
- **THEN** the host refuses it with "Only PNG, JPEG, GIF or WebP images can be sent" and keeps no part of it

#### Scenario: Cut short
- **WHEN** the connection drops before the last byte of an upload
- **THEN** the host removes the partial file and nothing is left under the session's folder

#### Scenario: The session is closed
- **WHEN** the owner closes a session that was sent two images
- **THEN** both images and the session's image folder are deleted

#### Scenario: A week later
- **WHEN** an image was sent 7 days ago to a session still in the sidebar
- **THEN** the host deletes it within the next hour, or when it next starts

#### Scenario: Logs
- **WHEN** a client uploads an image
- **THEN** the host log records the session, the size and the kind, and not the file's name, path or content
