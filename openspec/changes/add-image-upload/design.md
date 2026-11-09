# Design: send images to a session from the phone and from another computer

## Context

- Frames are a 1-byte kind, a 4-byte length and at most 1 MiB of payload (`hq-protocol`,
  `MAX_PAYLOAD`): control (JSON), output and input.
- A paired phone or desktop reaches the host through the gateway (`host/remote.rs` `proxy`), which
  passes input frames as they are and checks each control request against `allowed()`. On the
  host, input on a connection goes to the session it is attached to (`host/mod.rs`), and is
  ignored on a connection that attached to nothing.
- The desktop types a dropped file's path as a bracketed paste, escaped as Terminal.app does
  (`terminal/keys.rs` `dropped_paths`, `paste`), and refuses drops onto a remote host's session
  (`terminal/view.rs` `drop_paths`). GPUI reads images from the clipboard on macOS
  (`vendor/gpui-pre-macos/src/pasteboard.rs`) and Windows (`gpui-pre-windows` `clipboard.rs`).
- The phone's core keeps an `alacritty_terminal` per open terminal, so it knows whether the agent
  asked for bracketed paste.
- Observed 2026-11-06: a bracketed paste of an image's absolute path becomes `[Image #1]` in
  Claude Code 2.1.318 and Codex 0.166.0, with the file outside the project folder.

## Goals

- One way for any client to put an image where the agent can read it.
- Nothing kept longer than the owner chose; nothing kept on the phone.
- Checked at the trust boundary: the host decides what it saves.

## Decisions

### D1. An upload is a request, then the bytes as input frames, on a connection of its own

The client opens a new host connection (a new stream on a link), greets, and sends
`Upload { session, size }`. The host answers `UploadReady` or an error. The client then sends the
image as input frames (up to 1 MiB each); once `size` bytes have arrived the host checks and saves
the file and answers `Uploaded { path }`, or an error. The connection then closes. An upload
connection never attaches, so its input can only ever go to the file.

Rejected: base64 chunks in JSON requests. A third larger, and many requests to order and
reassemble. Rejected: a new frame kind for data. The gateway and the frame reader would need a
change for what input frames already carry. Rejected: raising `MAX_PAYLOAD`. Every connection would
then buffer up to 10 MiB per frame.

### D2. The host saves images in its data folder, one folder per session

`<data folder>/images/<session id>/<unix ms>-<n>.<png|jpg|gif|webp>`, where `n` counts the
images this host process saved, so two in one millisecond differ (the file is created with
`create_new`, never over another). The `images`
folder and each session folder are mode 700, files 600; on Windows the host does not run. The file
is written to a `.part` name first and renamed when complete, so a dropped connection leaves no
half image under a real name (the `.part` is removed when the connection ends).

Rejected: the session's project folder. It would put screenshots in the owner's repositories,
where an agent might commit them. Rejected: the system temp folder. Cleared at the system's pace,
not the owner's, and shared with other users on Linux.

### D3. The host checks size, kind and session before saving

- `size` at most 10 MiB (10,485,760 bytes), refused before any byte is sent.
- The session must be in the index (any state, including not running).
- The kind is read from the file's first bytes, not from the client: PNG, JPEG, GIF or WebP (the
  formats Claude Code and Codex read). Anything else is refused and deleted.
- More bytes than announced, or another request before the last byte, end the upload with an
  error.

Errors are plain: "The image is larger than 10 MB", "Only PNG, JPEG, GIF or WebP images can be
sent", "The session is no longer there".

The format check and the 10 MiB limit live in `hq-protocol` (`image.rs`), so the desktop checks
before sending what the host checks again.

### D4. Images are deleted after 7 days, or when their session leaves the sidebar

- Closing or removing a session deletes its image folder with it.
- At start and every hour the host deletes images older than 7 days (by the time in their name)
  and the folders of sessions no longer in the index.
- An agent that already read an image keeps it in its own conversation; Headquarter does not
  reach into that.

Rejected: deleting when the agent has read the image. There is no reliable signal for that, and an
agent may look again later in the conversation (owner's choice of 7 days, 2026-11-06).

### D5. The client pastes the path

The host answers with the absolute path. The client pastes it as it pastes text: escaped as a
dropped path is (a backslash before shell characters), followed by a space, and wrapped in
bracketed paste when the agent asked for it. The phone's typing starts afresh after the paste, as
after a key (the agent's box changed).

Rejected: the host typing the path into the session. The client already knows which terminal is
open (the agent or its shell) and how it pastes; the host would have to guess both.

### D6. The phone: an image key and the photo picker

- The session key grid gets an image key in its free last place; while the keyboard is open it
  is pinned beside ⏎, outside the row that scrolls sideways (the owner found it scrolled out of
  sight, 2026-11-09). The shell's keys do not have it.
- The image key opens Android's photo picker (one image; no storage permission needed).
- The typing field moved to Compose's state-based field (`TextFieldState`): its input
  transformation sees each edit's text before and after, in order, so the mirror needs no copy
  of its own and a key tapped right after typing cannot drop letters; the cursor is placed at the
  end there.
- Rejected after building: images the keyboard offers (Gboard's clipboard and screenshot
  suggestion), through Compose's content receiver. Only the state-based field takes them
  (`research/evidence/image-upload/2026-11-09-content-receiver.md`), the API is still marked
  experimental, and the owner never uses them (2026-11-09); removed rather than shipped untested.
- An image that is not PNG, JPEG, GIF or WebP (HEIC from a camera), or that is larger than 10 MiB,
  is converted on the phone to JPEG at quality 90, scaled down so its longer side is at most 4096
  pixels, and smaller again until it fits. Agents scale images down further themselves.
- While it sends: "Sending image… 40%" in the session's banner. On failure the banner says why;
  nothing is pasted. The image is held in memory only, never written to the phone's storage.

### D7. The desktop uploads image drops and pastes on a remote host's session

- Dropping files onto a session on another computer: each image (by its first bytes, at most 10
  MiB) is uploaded and its path pasted; other files are refused with the existing notice, now
  "Only images can be sent to another computer". A local session is unchanged.
- The terminal's Paste key (as listed in the shortcuts window) with an image and no text on the
  clipboard, in a remote session: a PNG, JPEG, GIF or WebP is uploaded as it is, anything else the
  `image` crate reads (a Windows bitmap, a macOS TIFF) is converted to PNG first, and its path
  pasted.
  With text on the clipboard, paste is unchanged. A local session is unchanged.
- While it sends, the pane's notice says "Sending image…"; errors show there.

### D8. Logs carry the event only

`image saved session=<id> bytes=<n> kind=<png|jpeg|gif|webp>`, `image refused session=<id>
reason=<size|kind|session|cut short>`, `images deleted count=<n>`. Never a file name the owner
chose, a path or a byte of the image.

### D9. A phone may open a second stream for an upload

The phone's one stream stays attached to the open session, and an upload needs a connection
attached to nothing (D1). The gateway lets a phone open one more stream on its connection
(`PHONE_STREAMS` = 2; a desktop keeps 64), proxied like a desktop's further streams. The phone's
core opens it on its existing connection, greets, uploads and closes it.

Rejected: uploading on the attached stream. Input frames would then mean the session's keys or
the image's bytes depending on timing, and typing during an upload would land in the file.
Rejected: a second QUIC connection. The gateway treats a device's newest connection as the one
in use.

## Data, API and failure behaviour

- Protocol 16. `Request::Upload { session: String, size: u64 }`; `Response::UploadReady {}`,
  `Response::Uploaded { path: String }`; errors through the existing `Response::Error` codes
  (`BadRequest`, `NotFound`, `TooLarge` new).
- `allowed()`: `Upload` for phones and desktops. A phone may have 2 streams (D9).
- A dropped connection mid-upload: the `.part` file is removed and nothing is pasted; the client
  says "The image didn't arrive".

## Compatibility and rollback

Protocol 16: clients and hosts update together, as with every protocol bump (the app and the
phone already tell the owner when versions differ). Rolling back to 15 leaves the `images` folder;
the older host ignores it, and the next 16 host's sweep deletes it by age.

## Risks

- A paired device can fill the host's disk 10 MiB at a time. A paired device can already type
  anything into a session, so it is trusted; the 7-day sweep bounds what stays.
- `AGENTS.md` says the index keeps metadata only; images are owner content on disk, so the change
  adds a stated exception with its deletion rule, as for removing past conversations.

## Open questions

- agy's behaviour with a pasted image path (once agy trusts a folder).
