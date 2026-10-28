# Tasks

## 1. Host

- [x] 1.1 `SessionInfo.phone`, sent again whenever the phone in use changes (D2). Evidence: `a_paired_phone_follows_types_and_is_held_to_the_allow_list` (a subscriber sees "Test Phone" come and go; the list names it too).

Expected Result: every client knows which sessions a phone has.

## 2. Desktop

- [x] 2.1 The inactive-style page in place of the terminal and the shell panel's terminal; "Use here" and Enter take the session back; other keys are dropped; window activation does not take it back (D1, D3). Evidence: `the_page_names_the_client_in_use_elsewhere`, on screen (verification.md).
- [x] 2.2 The sidebar's phone mark and tooltip (D2). Evidence: `a_session_a_phone_has_names_the_phone`, on screen.
- [x] 2.3 At start, the app does not reopen a session a phone has (D3 risk). Evidence: on screen, relaunching the app while the stand-in phone has the session.

Expected Result: the desktop flags the session and never draws it half-sized.

## 3. Phone

- [x] 3.1 Remove the cover; leave for home with the note when another client takes the open session (D4). Evidence: Android Lint and ktlint, on the dev phone.

Expected Result: the phone gives way to the desktop.

## 4. Verification and delivery

- [x] 4.1 `./scripts/check` passes. `verification.md` written.
- [ ] 4.2 The owner checks it on the Mac and the phone.
- [ ] 4.3 After the owner accepts: archive, push to `fix/terminal-in-use-elsewhere`, update PR #86, report its URL.

Expected Result: PR #86 carries both changes.
