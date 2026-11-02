# Verification: fix-claude-usage-week-only

Branch `fix/claude-usage-week-only`, 2026-11-02.

## Checks

- `./scripts/check` passed: 582 tests, lint, Android build, OpenSpec. A first run had 73 host tests
  time out waiting for their hosts' sockets while the Mac was loaded (load average 12.8, the
  owner's sessions); the failing test passed alone and the rerun passed in full.
- New `a_screen_with_only_the_weeks_reads`: Claude Code 2.1.316's Usage tab text (no five-hour
  block) gives the week at 6 % and Fable's week at 0 % with their resets; the old rule rejects it.
- `missing_blocks_or_garbage_are_errors`: one all-models window is enough; a screen with only a
  per-model window, or nothing, is still an error.
- `a_window_claude_left_out_is_shown_as_reset`: no five-hour row when Claude reports none; the
  week still stands.

## Against the real Claude Code 2.1.316

Isolated test host (`HQ_DATA_DIR`/`HQ_CONFIG_DIR` under `/private/tmp/hqw`, Codex and agy off), one
Claude session in a folder Claude trusts, the dev app opened to start the hidden read (no keys or
clicks): host log `usage read agent=claude source=usage-screen ok=true`; `hq usage` shows
`week 7% resets Sat 2:59 PM`, `week · Fable 0% resets Sat 3:00 PM`, Max plan, no five-hour row.

## Not verified

- The installed app after the release.
