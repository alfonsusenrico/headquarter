# A short offline banner on the phone, and `hq remote pair` that scans anywhere

Status: approved by the owner 2026-11-09 ("do it all"; the banner text chosen: "MacBook is
offline · 5m"). Phase 1 of `research/notes/2026-11-09-roadmap-plan.md`.

## Why

- The phone's banner when the computer cannot be reached is a sentence and a half: "Can't reach
  MacBook. Is Headquarter running there with remote control on? The list shows what it knew 5m
  ago."
- `hq remote pair` draws its QR code with the terminal's own colours, so on a dark theme the code
  shows light on dark (inverted), which many camera apps cannot read; and a server's terminal gives
  no way to get the phone app, which the desktop's Remote Control window offers.

## Expected Outcome

- The banner reads "MacBook is offline · 5m" (after 10 s of trying) or "Reconnecting to MacBook… ·
  5m" before that; the age is how old the list is.
- `hq remote pair` draws the QR code dark on light in any terminal theme, and prints "Get the phone
  app:" with the latest APK's address under the code.

## What Changes

- Android `HomeScreen.kt` `ConnectionBanner` (D1).
- `hq-cli` `remote.rs`: the QR code coloured black on bright white when printing to a terminal (not
  when piped, or with `NO_COLOR`), and the phone app link (D2).
- Specs: `android-app` "Connected only while on screen", `remote-control` "Pairing from a terminal".

## Non-goals

- A second QR code for the app in the terminal (one code to scan avoids scanning the wrong one).

## Capabilities

No new project capability.

## Impact

Android `ui/HomeScreen.kt`; `hq-cli` `src/remote.rs`.
