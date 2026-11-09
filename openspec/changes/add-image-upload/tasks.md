# Tasks

## 1. Protocol and host

- [x] 1.1 Protocol 16: `Upload`, `UploadReady`, `Uploaded`, `ErrorCode::TooLarge` (D1). Evidence:
  `hq-protocol` round-trip tests; the version check refuses 15.
- [x] 1.2 `host/images.rs`: size, session and first-bytes checks, `.part` then rename, modes 700 and
  600, the log lines without names or paths (D2, D3, D8). Evidence: unit tests for each format's
  first bytes, a PDF, 10 MiB plus one byte, a missing session, mode bits.
- [x] 1.3 The connection loop: `Upload`, input into the file, the answer, the cut-short cleanup
  (D1). Evidence: host tests through the real socket: a PNG saved and its path answered; more bytes
  than announced; another request mid-upload; the connection dropped mid-upload leaves nothing.
- [x] 1.4 Deletion with the session (close, remove) and the sweep at start and hourly (D4).
  Evidence: host tests: closing deletes the folder; an image named 8 days old is deleted at start;
  a folder of a session no longer in the sidebar is deleted; unit test of the sweep (stale
  partial files, a file not Headquarter's left alone).
- [x] 1.5 `allowed()`: `Upload` for phones and desktops. Evidence: the gateway's allow-list tests.

Expected Result: any client can save an image for a session on the host, and only what the owner
chose stays, for as long as the owner chose.

## 2. Clients

- [x] 2.1 `hq-client`: upload over this computer's socket or a link, returning the path (D1).
  Evidence: host tests: raw socket uploads (`images_are_checked_saved_privately_...`); through the
  gateway over a desktop link with progress in steps (`the_client_works_over_a_remote_link_...`).
- [x] 2.2 `hq-mobile`: `send_image(bytes) -> path` with progress, then the path pasted into the open
  terminal, bracketed when asked (D5), on a second stream (D9). Evidence: host test: the phone
  core sends a PNG while its session stays open; the host saves it and the stub reads its path.
- [x] 2.3 Desktop: image drops and image pastes on a remote session upload and paste the path;
  other files keep the notice, worded "Only images can be sent to another computer"; local
  sessions unchanged (D7). Evidence: `terminal/images.rs` tests (a PNG as it is, a bitmap and a
  TIFF converted to PNG, an SVG refused; a dropped file judged by its bytes, over 10 MiB, a
  folder); on screen in 4.3.

Expected Result: the phone core and the desktop put an image's path in the agent's box.

## 3. Phone app

- [x] 3.1 Check first: Compose's content receiver with the `TextFieldValue` field (D6). Evidence:
  `research/evidence/image-upload/2026-11-09-content-receiver.md` (foundation 1.12.1: only the
  state-based field takes keyboard content); the field moved to `TextFieldState`; the mirror's
  `LiveTypingTest` cases pass.
- [x] 3.2 The image key in the session grid and folded row, the photo picker, keyboard images,
  progress and failure in the banner, the draft starting afresh after the paste (D6). Evidence:
  ktlint and Android Lint clean.
- [x] 3.3 Conversion: not PNG, JPEG, GIF or WebP, or over 10 MiB, to JPEG 90 with the longer side at
  most 4096 px, smaller until it fits; in memory only (D6). Evidence: JVM tests of the decision
  (format by first bytes, size); on the phone, a HEIC photo.

Expected Result: the phone sends a picked or offered image and the agent attaches it.

## 4. Records, verification and delivery

- [x] 4.1 `AGENTS.md`: the host keeps images the owner sends, with the deletion rule;
  `docs/architecture.md`: `host/images.rs`. Evidence: diff.
- [x] 4.2 `./scripts/check` passes; `./scripts/android-core` and `./gradlew ktlintCheck assembleDebug
  lint testDebugUnitTest` pass.
- [ ] 4.3 On the owner's phone with a test host, with the owner's go: a screenshot to Claude Code
  and to Codex (`[Image #1]` in the box, nothing sent to the agent), a HEIC photo, a keyboard
  screenshot, the link dropped mid-send. An isolated app instance on this Mac (own
  `HQ_DATA_DIR`, once the owner frees the Mac) paired with the test host: a PNG dropped and a
  screenshot pasted. The images folder after closing the session.
  `verification.md` written.
- [ ] 4.4 After the owner accepts: archive, push `feat/image-upload`, open the PR, report its URL.
