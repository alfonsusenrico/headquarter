## MODIFIED Requirements

### Requirement: The phone keeps no content
The app SHALL store only the phone's key (encrypted by the Android Keystore and excluded from backups), the paired computers' names, keys and relays, and its settings. It SHALL NOT store or log prompts, terminal output, session titles, pairing codes, keys or images it sends.

#### Scenario: After use
- **WHEN** the owner has used sessions on the phone and closes the app
- **THEN** the app's storage holds no terminal content, titles or codes, and its log only connection events

#### Scenario: After sending an image
- **WHEN** the owner has sent a photo, converted to JPEG, to a session
- **THEN** neither the photo nor the JPEG is in the app's storage or cache

## ADDED Requirements

### Requirement: Sending an image
A session's screen SHALL have an image key that opens the system's photo picker, and SHALL take an image the keyboard offers. The app SHALL send the chosen image to the computer for that session and paste the path the computer answers into the agent's box, followed by a space. It SHALL show the sending's progress and say why when it fails, pasting nothing. A shell's screen SHALL have no image key.

#### Scenario: A screenshot to Claude Code
- **WHEN** the owner taps the image key in a Claude Code session and picks a screenshot
- **THEN** "Sending image…" shows with its progress, and then Claude Code's box shows `[Image #1]`

#### Scenario: From the keyboard
- **WHEN** Gboard offers the screenshot just taken and the owner taps it while typing to Codex
- **THEN** the screenshot is sent and Codex's box shows `[Image #1]` after the typed text

#### Scenario: The connection drops
- **WHEN** the link to the computer drops while an image is sending
- **THEN** the banner says "The image didn't arrive" and nothing is pasted

### Requirement: Images the agents can read
Before sending, the app SHALL convert an image that is not PNG, JPEG, GIF or WebP, or that is larger than 10 MiB, to JPEG, scaled down until it is at most 10 MiB.

#### Scenario: A camera photo in HEIC
- **WHEN** the owner picks a 12 MB HEIC photo
- **THEN** the app sends it as a JPEG under 10 MiB and Claude Code attaches it
