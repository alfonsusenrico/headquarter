# Verification: add-windows-client

## Revision tested

Branch `feat/windows-client` at `4361a1e` (on `main` after #110). Windows: GitHub's
`windows-latest` runner, toolchain 1.99.1 from `rust-toolchain.toml`, through the throwaway
branch `spike/windows-gpui` (the same steps as `windows.yml`, which runs only on pull
requests). macOS: the owner's MacBook Air.

## Spike (D8)

Run 41457873032: GPUI 0.3.9, gpui-component 0.8.0 and alacritty_terminal 0.26.0 at the app's
pins built, opened a window and quit; Cascadia Mono and Consolas present; the terminal grid
rendered (`research/evidence/windows-client/2026-11-02-spike.md`).

## Automated checks

| Check | Result |
| --- | --- |
| `./scripts/check` on macOS (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed at 4361a1e: 599 tests, 0 failed; clippy and fmt clean; cargo-deny ok; OpenSpec 19 of 19 valid |
| Windows clippy `-p hq-app -p hq-client -p hq-remote -p hq-protocol --all-targets -D warnings` (run 41469164644, at 5fe7e74; 4361a1e after it changes one label) | clean |
| Windows tests, same crates (run 41469164644) | hq-app 212 passed, 1 ignored; hq-protocol 38; hq-remote 8 |
| Release build `--profile dist --no-default-features` on Windows, run twice | built; both runs exit 0 |
| `dist plan` | `hq-app-x86_64-pc-windows-msvc.zip` with `headquarter-app.exe`; no Windows host archive |
| `openspec validate add-windows-client --strict` | valid |

Five app tests that browse this computer's folders (local source) are Unix-only: on Windows
the browser reads only a remote host. New tests: host paths keep their slashes on Windows
(compares strings, since Windows compares paths by component), the Windows data folder,
the Windows default font, the setup decision.

## On the Windows runner (screenshots)

`research/evidence/windows-client/`, the runner's own screen (no personal data), release
build with `HQ_CONFIG_DIR` and `HQ_DATA_DIR` in a scratch folder:

- First launch: setup asks only for the look ("step 1 of 2"), no agents step.
- With a settings file: "No host yet" in the switcher, Connect to a Host open, the sidebar
  offering Connect to a Host…, the field's chip `Ctrl+Shift+O`.

Found and fixed through these runs: a console window opened beside the app (now a Windows
GUI program); setup never showed on Windows; the empty sidebar offered Add sessions with no
host; the chip read `Ctrl⇧O` and setup said `⌘G`; a remote host's folders would have been
joined with `\` and refused by the host; `curl -o /dev/null` for the update check (now `NUL`
on Windows).

## Not verified

- Pairing with a real host, sessions, typing, IME and keyboard layouts, toast notifications,
  the update notice and SmartScreen on a real Windows PC: the owner's test after
  `v0.2.0-alpha.1` (task 5.2).
- The `windows.yml` run on this branch's pull request: it runs once the PR exists (task 4.1).
- The macOS app on screen after the refactor: needs the Mac free (task 5.1). The Unix code
  paths changed only in where they find this computer's host (`hosts::local()` and
  `connect()` checks instead of raw socket probes), covered by the host and app tests.
