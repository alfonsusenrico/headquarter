# Tasks

## 1. Research

- [ ] 1.1 Claude Code background work: sub-agents, background shells, Monitor, child processes, and why the owner's cases showed idle (design, Research). Evidence: `research/evidence/add-background-work-status/findings.md`, probe event logs (types only).
- [ ] 1.2 agy reviews and Codex background work (design, Research). Evidence: the same findings file.
- [ ] 1.3 Process tree as a fallback: what a session's descendants look like at rest and with background work. Evidence: findings, counts per kind.

Expected Result: each agent's signals are known, with versions.

## 2. Design with the owner

- [ ] 2.1 Options page, two to four variants, light and dark. Evidence: `research/prototypes/background-work-status/index.html`, contrast table.
- [ ] 2.2 The owner chooses, and answers the open questions (notifications, when the review flag clears). Record the decisions in the design, the spec delta and these tasks, then ask for approval of the build.

Expected Result: an approved design.

## 3. Build (detailed after 2.2)

- [ ] 3.1 Readers per agent with fixtures. Evidence: unit tests from the recorded versions.
- [ ] 3.2 Protocol and host. Evidence: host tests with stub agents that report each kind.
- [ ] 3.3 Desktop sidebar and header, phone rows and header. Evidence: on screen in both themes, the dev phone.

Expected Result: background work and reviews show beside the status.

## 4. Verification and delivery

- [ ] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it.
- [ ] 4.3 After the owner accepts: archive, push `feat/background-work-status`, open the PR, report its URL.

Expected Result: one PR with the feature.
