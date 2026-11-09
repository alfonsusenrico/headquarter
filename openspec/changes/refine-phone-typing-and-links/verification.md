# Verification: refine-phone-typing-and-links

Revision: `feat/phone-typing-and-links` at 7ee72ee (code), on `main` 4deda3d.

## Automated (2026-11-09, macOS, Rust from `rust-toolchain.toml`, JDK 17)

| Check | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed: 614 tests, OpenSpec 19/19 |
| `hq-links` (the desktop's link tests, moved) | 6 passed |
| `hq-mobile` core: `a_tapped_cell_gives_its_web_link`, `a_link_is_found_where_it_is_drawn_while_scrolled_back`, `an_edit_erases_then_types_without_enter` | passed |
| Host test (`hq-cli/tests/host.rs`): the phone core types `from-the-coer`, erases 2, types `re`, sends Enter through a real host | the stub read `from-the-core` |
| `./scripts/android-core` | built, binding has `edit` and `linkAt`, no `typeText` |
| `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest` | passed: 15 JVM tests (7 in `LiveTypingTest`), Lint with warnings as errors clean |

Agents: Claude Code 2.1.318 and Codex 0.166.0 take Ctrl+J as a new line
(`research/evidence/phone-typing-images/2026-11-06-agent-input.md`). agy 1.5.1 not checked;
its sessions keep "Enter sends".

## On the phone by adb (2026-11-09)

TECNO CN7c, Android 16, debug build beside the release app, paired with an isolated test host
(`research/evidence/phone-typing-images/2026-11-09-phone-check.md`). Passed: tap to type, live
letters in Claude Code's box, Backspace, Backspace on an empty field (key event), Enter as a new
line in Claude Code and Codex, Enter running a shell command, tapping a link (opened without the
full stop), long press copying it (Android's own "Copied"). Found and fixed: fast edits arrived out
of order ("seocnd") because each ran on its own IO coroutine; 7ee72ee sends them through one
ordered queue, and fast typing then arrived in order. Android checks rerun after the fix: ktlint,
Lint, 15 JVM tests pass.

## Not verified yet

- With Gboard itself (adb injects key events): swipe, voice typing, autocorrect replacing a word,
  Gboard's Backspace on an empty field (task 2.3), a sent two-line Claude message: the owner.
- "No app can open this link" (needs a phone without a browser) and the app's own "Link copied"
  (Android 12 and older).
