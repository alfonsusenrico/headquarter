# Tasks

## 1. Build

- [x] 1.1 Codex without a conversation id starts without `resume` (D1). Evidence: `hq-signals`
  `resume_uses_the_resume_subcommand_and_an_empty_session_starts_fresh`.
- [x] 1.2 Host test `an_empty_codex_session_starts_fresh_with_the_host`: a stand-in Codex started,
  the host stopped and started, the second launch has no `resume`. It fails with the old launch.

Expected Result: an empty Codex session comes back as a new conversation.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes; `verification.md` written.
- [ ] 2.2 After the owner accepts: archive, push `fix/codex-sessions`, open the PR, report its URL.
