# The phone's errors read plainly

Status: proposed 2026-10-29, waiting for the owner's approval (the owner approved fixing it on
2026-10-29, after this short plan).

## Why

Found during the in-use fix's phone check (2026-10-28): a failed pairing showed
"reason=Couldn't reach the computer: the computer did not answer".

Cause, from the code: UniFFI generates the Kotlin `MobileException` subclasses with `message` built
from their fields (`"reason=${reason}"`), not from the Rust error's text, and `NotConnected` gets an
empty message (`uniffi/hq_mobile/hq_mobile.kt`). Five places show `error.message` as it is: pairing,
Settings (unpair), Add sessions, and the session screen (opening, resuming).

## Expected Outcome

- Every error the phone shows reads as a sentence: "Couldn't reach the computer: the computer did
  not answer", with no `reason=`.
- "Not connected to the computer" shows where the app used to show nothing.

## What Changes

- Android app: one function turns a `MobileException` into the owner's text, used in the five places.
- `android-app` spec: errors read plainly.

## Non-goals

- What each error says (the wording from the phone core stays).
- Changing the Rust errors or UniFFI's settings (the generated binding stays as generated).

## Capabilities affected

`android-app`. No new project capability.

## Impact

The phone app only. No protocol change, so no desktop release is needed for it, but it ships with
the next release as usual.
