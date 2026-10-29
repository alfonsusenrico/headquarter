# Tasks

## 1. Fix

- [ ] 1.1 `MobileException.ownerText()` and its use in the five places (D1). Evidence: an Android unit test for each variant, Android Lint and ktlint, and a failed pairing on the dev phone showing the plain text.

Expected Result: no "reason=" anywhere on the phone.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/phone-error-text`, open the PR, report its URL.

Expected Result: one PR with the fix.
