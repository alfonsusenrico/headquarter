# Getting the phone app from Remote Control

Status: proposed 2026-10-28, waiting for the owner's approval.

## Why

Pairing needs Headquarter for Android on the phone first, and today the only way to it is the
README. The owner wants the Remote Control window to make the download easy (2026-10-28), with a
QR code that leads the phone straight to the latest stable APK, not to the repository or the
release page. Decisions from the owner on 2026-10-28: a visible first step rather than an info
icon, with a Copy link button.

## Expected Outcome

- Headquarter ▸ Remote Control… shows "Get the phone app" as the first step, above the switch
  and "Pair a phone": a QR code, "Scan with your phone's camera", the Android version it needs,
  a line on what Android asks during the install, and Copy link.
- Scanning the QR code with the phone's camera opens the download of the newest stable
  `headquarter-android.apk`. A new release needs no change in the app: the link always names
  the latest one, and pre-releases are skipped.
- Copy link puts the same address on the clipboard and says "Copied" for a moment.
- Once a phone is paired, the step folds to one line with a Show button, so it no longer takes
  the room the paired phones need.
- The step shows whether remote control is on or off.

## What Changes

- `hq-app` `workspace/remote.rs`: the step, its QR code (drawn with the window's existing
  `qrcode` renderer), Copy link and its folding.
- `remote-control` spec: a requirement for the step.

## Non-goals

- The terminal's `hq remote pair`.
- Google Play, an app page of our own, or one QR code that both downloads and pairs.
- The phone app itself and the release pipeline (the APK and its name are unchanged).
- Android developer verification (see Risks in the design).

## Capabilities affected

`remote-control`. No new project capability.

## Impact

Desktop app only. No agent CLI is involved. The link was checked on 2026-10-28: it redirects to
the v0.1.16 APK.
