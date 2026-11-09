# Tasks

## 1. Shared link finder

- [x] 1.1 Move `hq-app/src/terminal/links.rs` to a new crate `hq-links`; `hq-app` uses it (D5).
  Evidence: the moved link tests pass in `hq-links`; `hq-app`'s terminal tests pass unchanged;
  `cargo deny` passes.
- [x] 1.2 `Phone::link_at(row, col)` in `hq-mobile` (D4). Evidence: core tests: a plain address the
  terminal wrapped over two rows, a cell beside it (none), a marked-up `file:` link (none), a
  marked-up web link found where it is drawn while scrolled back; the moved `hq-links` tests
  cover marked-up links wrapped over rows.

Expected Result: the phone core answers which web link is at a cell, with the desktop's rules.

## 2. Typing

- [x] 2.1 `Phone::edit(erase, text)` replaces `type_text` and `ENTER_DELAY` (D6). Evidence: core
  test: the input bytes are the Backspaces then the text, `\n` kept, no `\r`. Host test: the
  phone types `from-the-coer`, erases 2 and types `re`, sends Enter; the stub reads
  `from-the-core`.
- [x] 2.2 The mirror's diff as a pure Kotlin function (D1): appended text, a replaced last word, a
  deletion, an emoji erased as one character, a newline. Evidence: JVM unit tests.
- [ ] 2.3 Before choosing D2's empty-field Backspace: what Gboard sends on an empty field, on the
  owner's phone with the owner's go. Evidence: note in `research/evidence/phone-typing-images/`.
- [x] 2.4 The hidden field, tap to focus, cursor pinned at the end, emptied by the key row's keys
  and on reconnect, Backspace on empty as a key event (D1, D2; the placeholder stays the fallback
  if 2.3 shows Gboard sends no key event). `Composer` and its Send icon removed. Evidence:
  ktlint and Android Lint clean; `LiveTypingTest` (the draft starts afresh after a key).
- [x] 2.5 Enter: new line for Claude Code and Codex, Enter for a shell and agy (D3). Evidence:
  unit test of the per-agent rule.

Expected Result: the session screen has no text box; typing lands in the agent's own box.

## 3. Links

- [x] 3.1 Tap opens a web link, long press copies (the app's "Link copied" before Android 13, the
  system's confirmation from 13), no browser shows "No app can open this link", any other tap
  opens the keyboard (D4). Evidence: core link tests (1.2); Lint clean; on the phone in 4.2.

Expected Result: links open and copy from the phone; other taps type.

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes; `./scripts/android-core` and `./gradlew assembleDebug lint
  test` pass.
- [ ] 4.2 On the owner's phone, with the owner's go: each spec scenario with Claude Code and a
  shell (typing, a corrected word, swipe, voice, recalled text erased, two-line message, link open
  and copy). Codex: a two-line message. Screenshots without notifications. `verification.md`
  written.
- [ ] 4.3 After the owner accepts: archive, push `feat/phone-typing-and-links`, open the PR, report
  its URL.
