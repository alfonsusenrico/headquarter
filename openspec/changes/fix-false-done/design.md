# Design: replies that are not the end

## Context

`notifications::notice` posts "replied" on Working -> DoneUnread. The host already sends each
session's background work (`SessionInfo.background`, add-background-work-status), and
`hq_protocol::sidebar::beside` says when a session shows "background" or "review". The updater's
`busy` counts Working and NeedsYou only, and `restart_if_idle` restarts on the first update with
nothing busy. Owner decisions 2026-10-29: hold "replied" until the session settles; no settle time
for the update restart; a plan to review holds the restart.

## Decisions

### D1. "replied" waits for a quiet session

A session that becomes unread is put on a pending list instead of being notified. It is notified
once it has been unread, showing no background work, for 10 seconds; it leaves the list without a
notification if it works again, needs the owner (that notifies as today), is read, or leaves the
sidebar. Background work that ends while it is unread starts the 10 seconds then.

- 10 seconds: of the owner's 484 self-started turns, 68 came within 10 s (54 within 5 s); 30 s
  would catch 116 but delays every real reply by 30 s. Background work Headquarter can see covers
  the longer waits.
- The watched rule is checked when it is posted, so a session the owner is looking at by then is
  not notified.
- Rejected: holding it in the host (the host's status and unread mark stay as they are; only the
  notice moves). Rejected: a "still working in the background" wording (the owner chose to wait).

### D2. One timer

The pending list keeps each session's quiet-since time. A single timer runs while the list is
non-empty and checks it once a second; session updates also re-check. Nothing is persisted: an
app restart drops pending notices, as the first listing never notifies today.

### D3. The phone's top section

Home's top section lists unread replies only when they show no background work; one that does
stays in its group with "background" and rises once the work ends. No timer: the phone posts no
system notifications.

### D4. Busy for the update

`update::busy` adds live sidebar sessions showing "background" or "review"
(`sidebar::beside`). The window names them as it names working ones. The restart still runs on the
first update with nothing busy.

## Risks

- A reply notice comes 10 s later than today. Accepted by the owner's choice to wait.
- Background work Headquarter cannot see still ends the wait early; the quiet time only covers
  quick returns.
