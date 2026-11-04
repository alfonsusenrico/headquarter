# Tasks

## 1. Diagnosis

- [x] 1.1 Why the share falls back. Evidence: `research/evidence/claude-usage-going-back/findings.md`
  (code path, Claude Code's status-line triggers, a 4-minute snapshot watch).

## 2. Build

- [x] 2.1 The helper does not save a report lower than the saved one for the same window (D1).
  Evidence: `hq-signals` tests: a lower report of the same week is not saved; an equal or higher
  one is, with a new `read_at`; a new week is; without a saved snapshot the report is saved.
- [x] 2.2 The host shows the higher share of the same window when the status line is newer (D2).
  Evidence: `claude_usage()` tests for each spec scenario, including Refresh reading lower.

Expected Result: an idle session's redraw no longer lowers the Claude week; a new week and Refresh
show their own numbers.

## 3. Verification and delivery

- [x] 3.1 `./scripts/check` passes. The real `hq hook-statusline` helper given an older share after
  a newer one keeps the newer share (host test). `verification.md` written.
- [ ] 3.2 The owner checks it in daily use.
- [ ] 3.3 After the owner accepts: archive, push `fix/claude-usage-going-back`, open the PR, report
  its URL.
