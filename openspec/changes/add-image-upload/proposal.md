# Send images to a session from the phone and from another computer

Status: approved by the owner 2026-11-09 ("approve, build it").

## Why

The owner (2026-11-06) often gives agents screenshots, and cannot from the phone: the agent runs on
the computer and cannot read the phone's files. The desktop app has the same gap with a host on
another computer: dropping a file onto a server's session is refused, because the path would name
a file on the PC (`desktop-app` "Files stay on their computer"). From the Windows client every
session is on another computer.

Claude Code 2.1.318 and Codex 0.166.0 both turn a pasted image path into an attachment (`[Image
#1]`), as a drag onto a local session does today
(`research/evidence/phone-typing-images/2026-11-06-agent-input.md`). So the image only needs to
reach the computer the agent runs on, and its path the agent's input box.

Decisions: `research/notes/2026-11-06-phone-typing-images-links.md` (images kept 7 days or until
the session leaves the sidebar) and `research/notes/2026-11-04-several-hosts.md` (desktop drops onto
a remote session upload too, 2026-11-09).

## Expected Outcome

- On the phone, an image key in a session's key row opens the photo picker. The chosen image is
  sent to the computer, and its path appears in the agent's box, where Claude Code and Codex show
  `[Image #1]`. A screenshot offered by the keyboard can be sent the same way.
- In the desktop app, dropping an image onto a session on another computer, or pasting an image
  copied to the clipboard (a Windows or macOS screenshot), does the same.
- The computer keeps each image 7 days, or until its session leaves the sidebar if that is sooner.
  Nothing else on the computer, and nothing on the phone, keeps a copy.

## What Changes

- Protocol 16: an `Upload` request on its own connection, the image as input frames, answered
  with the saved path (D1). Allowed from a paired phone and a paired desktop.
- The host checks and saves images under its data folder, one folder per session, and deletes
  them by age and with their session (D2-D4).
- Clients paste the returned path as they paste text (D5).
- The phone's image key, photo picker, keyboard images, and a conversion to JPEG for images the
  agents cannot read or that are too large (D6).
- The desktop uploads image drops and image pastes on a remote host's session (D7).
- `AGENTS.md`: the host keeps the images the owner sends, as a stated exception to keeping
  metadata only.
- Specs: `remote-control` (what a phone may do), `session-host` (images sent to a session),
  `android-app` (sending an image, no copy on the phone), `desktop-app` (files stay on their
  computer, images go).

## Non-goals

- Other files (PDFs, archives, folders): a non-image drop onto a remote session stays refused.
- Video.
- Several images in one pick on the phone (one at a time; the desktop sends each dropped image).
- Pasting clipboard images into a session on this computer: unchanged (Claude Code reads the
  clipboard itself with its own Ctrl+V).
- Several hosts in one list (the merged-list change).

## Capabilities

No new project capability: images are files in the host's data folder, not in the index. The
`deployed` standard is unchanged (logs carry events only). Verified against Claude Code 2.1.318
and Codex 0.166.0; agy 1.5.1 receives the path as text, its attachment behaviour unchecked.

## Impact

- `hq-protocol` (version 16: `Upload`, `UploadReady`, `Uploaded`), `hq-daemon` (`host/images.rs`,
  the connection loop, close and remove, the hourly sweep, `allowed()`), `hq-client` (upload over
  a host connection or a link), `hq-mobile` (`send_image`), `hq-app` (drop and paste on a remote
  session).
- Android: the image key, the photo picker, keyboard images, conversion, progress.
- Every client and host must update together (protocol 16), as for every protocol bump.
