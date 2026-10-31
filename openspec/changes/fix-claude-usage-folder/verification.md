# Verification: fix-claude-usage-folder

Working tree on `fix/claude-usage-folder` (committed with this record), 2026-10-31.

## Checks

- `./scripts/check`: passed. 572 tests, clippy `-D warnings`, rustfmt, cargo-deny, Android build,
  ktlint, Android Lint, OpenSpec (19 items).
- New tests:
  - `hq-signals` `trusted_folders_come_from_claudes_own_record`: trusted, untrusted home, missing
    field, no `projects`, malformed.
  - `hq-daemon` `the_read_skips_folders_claude_does_not_trust`, `a_folder_matches_its_resolved_path`.
  - Host `the_usage_read_skips_a_folder_claude_does_not_trust`: the newest session runs in a folder
    Claude's record marks untrusted (the stub asks to trust it); the reading comes from the older
    trusted folder. Fails with the old choice (checked by putting it back temporarily).
  - Host `a_refresh_after_a_failed_usage_read_reads_again`: a trust failure, then Refresh at once
    reads again and succeeds. Fails without D2 (checked the same way).
- Existing usage host tests (trust question, screen, Codex, agy, read only when someone looks)
  pass unchanged: a test home without `.claude.json` keeps the old folder choice (D1 fallback).

## The owner's machine (read-only)

- `~/.claude.json`: the home folder is recorded untrusted, 17 of 18 other folders trusted, so the
  new choice skips `~` and uses the most recent project folder.

## On screen (the owner freed the Mac, 2026-10-31)

Test host and dev app built from 6f8575e (`HQ_DATA_DIR`/`HQ_CONFIG_DIR` under `/private/tmp/hqv`,
Codex and agy turned off), real Claude Code 2.1.312 and the owner's real `~/.claude.json`: an older
session in the trusted scratch folder, then a newer one in `/Users/enrico` (recorded untrusted; it
sits at Claude's trust question). The hidden read ran in the trusted folder: host log
`usage read agent=claude source=usage-screen ok=true`, and the details show 5h, week and
week · Fable with their resets and no trust message
(`research/evidence/claude-usage-folder/details-home-session.png`). No prompt was sent.

## Not verified

- The installed app itself (after the release).
- Linux.
