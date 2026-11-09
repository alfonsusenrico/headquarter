# Tasks

## 1. Shared link finder

- [ ] 1.1 Move `hq-app/src/terminal/links.rs` to a new crate `hq-links`; `hq-app` uses it (D5).
  Evidence: the moved link tests pass in `hq-links`; `hq-app`'s terminal tests pass unchanged;
  `cargo deny` passes.
- [ ] 1.2 `Phone::link_at(row, col)` in `hq-mobile` (D4). Evidence: core tests: a plain address, a
  marked-up link wrapped over two rows, a `file:` link (none), a cell beside a link (none), while
  scrolled back.

Expected Result: the phone core answers which web link is at a cell, with the desktop's rules.

## 2. Typing

- [ ] 2.1 `Phone::edit(erase, text)` replaces `type_text` and `ENTER_DELAY` (D6). Evidence: core
  test: the input bytes are the Backspaces then the text, `\n` kept, no `\r`.
- [ ] 2.2 The mirror's diff as a pure Kotlin function (D1): appended text, a replaced last word, a
  deletion, an emoji erased as one character, a newline. Evidence: JVM unit tests.
- [ ] 2.3 Before choosing D2's empty-field Backspace: what Gboard sends on an empty field, on the
  owner's phone with the owner's go. Evidence: note in `research/evidence/phone-typing-images/`.
- [ ] 2.4 The hidden field, tap to focus, cursor pinned at the end, emptied by the key row's keys
  and on reconnect, Backspace on empty (D1, D2). `Composer` removed. Evidence: Android Lint and
  ktlint clean; unit tests for the emptying rules.
- [ ] 2.5 Enter: new line for Claude Code and Codex, Enter for a shell and agy (D3). Evidence:
  unit test of the per-agent rule.

Expected Result: the session screen has no text box; typing lands in the agent's own box.

## 3. Links

- [ ] 3.1 Tap opens a web link, long press copies with "Link copied", no browser shows "No app can
  open this link", any other tap opens the keyboard (D4). Evidence: unit test of the tap decision;
  Lint clean.

Expected Result: links open and copy from the phone; other taps type.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes; `./scripts/android-core` and `./gradlew assembleDebug lint
  test` pass.
- [ ] 4.2 On the owner's phone, with the owner's go: each spec scenario with Claude Code and a
  shell (typing, a corrected word, swipe, voice, recalled text erased, two-line message, link open
  and copy). Codex: a two-line message. Screenshots without notifications. `verification.md`
  written.
- [ ] 4.3 After the owner accepts: archive, push `feat/phone-typing-and-links`, open the PR, report
  its URL.
