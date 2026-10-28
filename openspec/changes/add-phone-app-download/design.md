# Design: getting the phone app from Remote Control

## Context

The Remote Control window (`hq-app` `workspace/remote.rs`) has a header, a status card with the
switch, "PAIR A PHONE" (only while remote control is on) and "PAIRED PHONES". It already draws
the pairing code as a QR code with the `qrcode` crate (`qr_code`). The README links the APK at
GitHub's latest-download address. The update check names the repository in `update.rs`
(`REPO_PAGE`).

## Goals and non-goals

Goals: from the window, the owner gets the app onto the phone with one scan, or one copied link,
and always the newest stable build. Non-goals: the terminal's `hq remote pair`, Google Play, a
combined download-and-pair code.

## Decisions

### D1. The link

`https://github.com/alfonsusenrico/headquarter/releases/latest/download/headquarter-android.apk`.
GitHub redirects it to the asset of the latest release, which excludes pre-releases and drafts,
so the app never needs a new link and a release candidate is never offered. The release already
publishes the APK under that exact name (`build-android.yml`). Checked 2026-10-28: 302 to
`releases/download/v0.1.16/headquarter-android.apk`.

It is built from the repository address `update.rs` already keeps, made shared rather than
written twice.

Rejected: the release page (the owner asked for the file itself), the GitHub API at run time (a
network call for an address that never changes), a URL shortener (a third party in the path).

### D2. The step, first in the window

A "GET THE PHONE APP" section above "PAIR A PHONE", shown whether remote control is on or off,
because getting the app comes before turning anything on. Its card holds:

- the QR code of the link, 132 px, drawn by `qr_code` with the same quiet zone as the pairing
  code (version 4 or 5 at medium error correction, about 33 to 37 modules);
- "Scan with your phone's camera" (semibold), then "Headquarter for Android, Android 10 or
  later." and "Your phone asks once to let your browser install apps." (muted);
- Copy link, which writes the link to the clipboard and reads "Copied" for 2 s, then returns.

The QR code is encoded once when the window opens, not on every frame.

### D3. Folding once a phone is paired

With one or more paired phones the step is a single row: "Get the phone app" with "For another
phone" in muted text and a Show button. Show opens the full step until the window closes. With
no paired phone it is always open.

Rejected: always open (it pushes the paired phones down for someone who already has the app),
an info icon (the owner chose a visible step: it is easy to miss when it matters).

### D4. Text and contrast

No semicolons in the window's text. The muted lines use the existing muted token on the card,
which the theme test already holds to AA. The QR code is drawn dark on white in both themes, as
the pairing code is, since phone cameras read it reliably that way.

## Risks

- Android developer verification: from 30 September 2026 in Indonesia, Brazil, Singapore and
  Thailand, apps from unverified developers cannot be installed through Google Play and six
  other large stores. A browser download, as here, is not covered yet. Google plans to cover
  every install source worldwide in 2027. Then the APK would need the owner registered as a
  verified developer, or users would go through Android's extra warning flow. To be decided
  before 2027, outside this change.
- GitHub can change its redirect. The README depends on the same address, and a check that the
  link still answers 302 with an APK belongs to the release's verify step.
