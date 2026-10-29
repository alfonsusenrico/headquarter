# Tasks

## 1. Order

- [x] 1.1 `layout` orders each group and the ungrouped list by band (needs you, working, the rest), recent activity inside each (D1). Evidence: `hq-protocol` unit tests (working above a newer idle session, needs you above working, ext in the last band, the Needs you section unchanged, groups in the owner's order).
- [x] 1.2 `hq ls` shows the same order. Evidence: `hq-cli` render test.

Expected Result: every client computes the new order.

## 2. Holding still

- [x] 2.1 The app holds the shown order while the pointer is over the sidebar or rail, or a drag is under way (D2). Evidence: unit tests of the hold function (held rows keep places, a new session and a moved one take computed places, release applies the new order), on screen with a synthetic host and stand-in agents: a session starts working with the pointer on the sidebar and off it.
- [ ] 2.2 The phone holds its rows while a finger is on the list (D3). Evidence so far: `HeldOrderTest`, Android Lint and ktlint. Pending: on the dev phone with the synthetic host, a session starts working during a scroll and after it.

Expected Result: rows never move under the owner's pointer or finger.

## 3. Verification and delivery

- [x] 3.1 `./scripts/check` passes. Android lint and ktlint pass. `verification.md` written.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `feat/working-sessions-first`, open the PR, report its URL.

Expected Result: one PR with the change.
