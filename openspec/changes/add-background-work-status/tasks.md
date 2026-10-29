# Tasks

## 1. Research

- [x] 1.1 Claude Code background work: sub-agents, background shells, Monitor, child processes, and why the owner's cases showed idle (design, Research). Evidence: `research/evidence/add-background-work-status/findings.md`, probe event logs (types only).
- [x] 1.2 agy reviews and Codex background work (design, Research). Evidence: the same findings file.
- [x] 1.3 Process tree as a fallback: what a session's descendants look like at rest and with background work. Evidence: findings, counts per kind. Done 2026-10-29 with real calls (owner's go): background shells, monitors and the commands of quiet subagents show only in the process tree once the turn ends.

Expected Result: each agent's signals are known, with versions.

## 2. Design with the owner

- [x] 2.1 Options page with many variants across the families in the design, every case on every surface (sidebar row, folded group, header, Usage row, phone row and header), light and dark. Evidence: `research/prototypes/background-status/index.html` (8 variants in 6 families, light and dark, each with its words, contrast and no-colour notes).
- [x] 2.2 The owner chooses, and answers the open questions (notifications, whether background work rises in the sidebar order; when the review flag clears is answered by the research: on approve, reject or a new prompt). Record the decisions in the design, the spec delta and these tasks, then ask for approval of the build. Done 2026-10-29: E1, rise like working, no notifications.

Expected Result: an approved design.

## 3. Build

- [x] 3.1 `SessionInfo.background` and `review`, optional (protocol 14) (D5). Evidence: protocol round-trip and older-message tests.
- [x] 3.2 Claude background agents from OSC 7501 child records, with titles kept in memory only (D6). Evidence: scanner and tracker tests from the recorded reports.
- [x] 3.3 Claude background commands and monitors from the hooks and the processes they started (D6). Evidence: host test with a stand-in that reports the hooks and starts and ends a process; MCP-like helpers not counted.
- [x] 3.4 Codex sub-agents and agy reviews (D7). Evidence: unit tests; agy fixture from the recorded reports.
- [x] 3.5 Order: background and review in band 1 (shared layout). Evidence: `hq-protocol` test.
- [x] 3.6 Desktop: row words, header chips and title, Usage row, folded group count (D4, D5). Evidence: model tests, on screen with stand-ins (with the owner's go for the Mac).
- [x] 3.8 Their own colours and marks (D8, owner 2026-10-29): `teal` and `violet` chrome colours, made from the palette in other themes, settable in theme files and the creator; "background" with still dots, "review" with a diamond; the header and the phone in the same colours. Evidence: theme contrast tests for every shipped theme, theme file round trip, `./scripts/check`, on screen with stand-ins in light and dark.
- [ ] 3.7 Phone: row word and session header detail. Evidence so far: `hq-mobile` mirror test, Android Lint and ktlint. Pending: on the dev phone with the owner's go.

Expected Result: background work and reviews show as chosen, and nothing extra shows without a signal.

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it.
- [ ] 4.3 After the owner accepts: archive, push `feat/background-work-status`, open the PR, report its URL.

Expected Result: one PR with the feature.
