# The desktop app on Windows, as a client

Status: plan, waiting for the owner's approval. Builds on `add-remote-desktop-client`.

## Why

The owner (2026-11-02) uses a Windows PC at home and wants to control their sessions from it
without switching to the Mac. Research (`research/notes/2026-11-02-windows-client-research.md`)
found GPUI ships a Windows platform at our pinned 0.3.9 (`gpui-pre-windows`), Zed has been
stable on Windows since 2025-10-15, and the blockers are ours: unguarded `std::os::unix` code
and a client that assumes a host on the same computer. The owner chose a Windows build of this
app over a terminal client, released first as an alpha for them to test.

## Expected Outcome

- A Windows x86-64 zip of the app on the release page (`v0.2.0-alpha.1` first, unsigned). It
  starts at Connect to a Host and, once paired, works like the remote client on a Mac.
- Every pull request is build-checked, linted and tested for Windows on a free GitHub runner,
  since nothing on the owner's Mac can build for Windows.

## What Changes

- Unix-only code behind `cfg(unix)`; `HostPaths` out of `hq-daemon` so `hq-client` no longer
  depends on the host (D1).
- The app on Windows: remote only, its folders, fonts, shortcuts, notifications, update notice
  (D2-D5).
- A Windows check workflow on pull requests (D6); `AGENTS.md`'s checks rule says so.
- `dist` builds the app for `x86_64-pc-windows-msvc` as a zip (D7); `release-install` and
  `desktop-app` specs.
- A spike first, on the Windows runner, before the refactor (D8).

## Non-goals

- A host on Windows (the host needs Unix terminals; WSL is the owner's option).
- Code signing (later; the alpha is unsigned and README says how to run it).
- Installing or updating by itself on Windows (the alpha's update notice links to the release).
- Windows on ARM.

## Capabilities

No new project capability. CI gains a hosted Windows job (a checks-rule change, owner
2026-11-02). Modified: `release-install`, `desktop-app`.

## Impact

`hq-signals`, `hq-discovery`, `hq-config`, `hq-client`, `hq-daemon` (paths move), `hq-app`,
`dist-workspace.toml`, `.github/workflows/`, `AGENTS.md`. No protocol change.

## Not verifiable here

Nothing on the owner's Mac runs Windows. Evidence is the Windows runner's build, lint and tests,
and the owner's test on their PC.
