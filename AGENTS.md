# Headquarter

<!-- pakem:profile name=personal version=1.0.1 sha256=1c6beb9c1bcc7e2717cc2783f1aa19692ba35b99ba577fdb9e0b548ddbd4bf22 -->
Personal project rules (Pakem profile `personal`):

- Prefer free services and tiers, weighed against quality (`default:free-first`). When
  a paid option is clearly better, propose it with the free alternative, a monthly
  cost estimate, and the quality gained. Record an approved one as an `## Overrides`
  bullet naming the service and its budget.
- Call models through the owner's AI inference gateway. A project with only a simple
  AI call may call a provider directly and says so in `AGENTS.md`.
- Repositories live on GitHub, private unless the owner makes one public. Checks run on
  GitHub-hosted runners in public repositories and on the owner's self-hosted runner in
  private ones.
- Hosts and environments are chosen per project (`deploy/release.toml`), and services
  release with `pakem release`. Runtime settings and secrets live in env files on the
  host, never in the repository.
- Code, docs, and commits are in English; each project states the language of its UI
  and generated content.
- Commit with the owner's own name and personal email, set repo-locally before the
  first commit. The owner reviews and merges.
<!-- /pakem:profile -->

## Purpose and boundaries

Headquarter (`headquarter`, alias `hq`) is the owner's desktop terminal for coding-agent
CLIs (Claude Code, Codex, Antigravity `agy`, and others it discovers). It hosts many
agent sessions, keeps them in a sidebar in groups the owner makes, flags each session's
status (working, needs you, idle not read yet, idle, not running), and shows each provider's
usage quota. Targets macOS and Linux; the desktop app also runs on Windows as a client of
remote hosts (no host there).

- **It owns:** the sessions it launches (PTYs, injected per-launch hooks), its
  session index, groups, unread flags, and quota snapshots.
- **It observes only:** agent sessions started elsewhere. It never attaches to,
  signals, or writes to outside processes.
- **It never:**
  - modifies the agents' own configuration (`.claude`, `.codex`, `.gemini` in the home
    folder); integration is passed per launch (`--settings`, `-c`, environment);
  - persists prompt text or model output; the index keeps metadata only (ids, paths,
    timestamps, status, quota numbers).
- **One exception to leaving agent data alone (owner, 2026-10-23):** removing a past
  conversation the owner chose in Add sessions: Claude Code's transcript (and the folder
  beside it) goes to the system Trash, Codex only through its own `archive` and
  `delete`; agy is never changed (`host/remove.rs`).
- **Images the owner sends (2026-11-06):** kept owner-only in the data folder's
  `images/<session id>/`, deleted after 7 days or with the session (`host/images.rs`).

Capabilities: database, deployed
Design surface: desktop application (session sidebar, terminal pane, quota footer), and a phone
app (Android: home mirroring the sidebar, session terminal with a key row, pairing, settings)
Delivery: us
Language: UI text English; relative ages as `0m`, `1m`, `3h`, `2d`, `5w` (no seconds); times in local 12-hour format.

## Commands

```bash
./scripts/check     # the project's CI, before every push: fmt, clippy -D warnings, tests, cargo-deny, OpenSpec
HQ_DATA_DIR=/tmp/hq-dev cargo run -p hq-cli --bin hq -- new claude   # the CLI, isolated data folder
./scripts/android-core                            # the phone's Rust core and Kotlin binding
(cd mobile/android && ./gradlew assembleDebug)    # the Android app (JDK 17, Android SDK and NDK)
cargo run -p hq-app --bin headquarter-app -- --quit-after 5   # open the app, quit after 5 s
```

Integration tests start real hosts with stub agents in `/tmp/hqh-*` (socket paths under about
100 bytes). Isolate runs with real agents through `HQ_DATA_DIR` and `HQ_CONFIG_DIR`, never
`XDG_DATA_HOME`: agents inherit it, and Claude Code then reinstalls itself under it and repoints
its launcher. Tests never use what the machine has installed: their own `HOME`, config folder
and `SHELL`, and `HQ_SYSTEM_BIN_DIRS` empty (it replaces the system-wide folders discovery
searches; a folder of stubs tests them). Remote control tests run with `HQ_REMOTE_RELAYS=none`;
`HQ_REMOTE_PAIR_SECS` shortens the 2-minute pairing code.

## Architecture map

Each crate and folder: [`docs/architecture.md`](docs/architecture.md).

- `openspec/` is the source of truth for behaviour. `openspec/changes/archive/`, `research/`
  and the `docs/ref *.jpg` references stay local, never committed (`.gitignore`).

## Project rules

- Titles read from agent history are display-only: never write them to the index, the
  log, or evidence files (owner decision 2026-10-20).
- Status comes first from OSC 7501, which Claude Code, Codex, and agy all emit once the
  terminal answers its query. Hooks supplement it for Claude (session id, tools,
  reasons). Output activity is a fallback until the first structured report.
- Agent CLI formats (transcripts, rollout files, hook payloads) are unstable external
  contracts: parse defensively, tolerate unknown fields, and record observed versions
  in `research/evidence/`.
- Licence hygiene: do not copy code from AGPL or GPL projects (Warp, Zed's `terminal`
  crate, cmux). Ideas and wire formats only; dependencies must be MIT, Apache-2.0, or
  similarly permissive.
- Remote access: a paired phone or desktop reaches only what `allowed()` in `host/remote.rs`
  lists for its kind (an exhaustive match: each new request is decided there). The host key
  (`remote.key`, mode 600), pairing secrets, typed input and terminal output never reach logs.
  Relays default to n0's public ones; a personal relay is a setting, never the default (owner,
  2026-10-26).
- Each theme is designed for its own background (UI and UX rule): the chrome in `theme.rs`,
  themes made in the creator, the agents' marks, the phone's two themes, and what agents draw
  in the terminal (owner, 2026-10-28).
- The phone app keeps no session content: only its key (Keystore-wrapped, backups off), the
  paired computers' names, keys and relays, and its settings. Its logs carry connection
  events only. Kotlin formats with ktlint, and Android Lint runs with warnings as errors.

- GPUI comes from the `gpui-pre` snapshot crates, pinned with `=`; upgrade them together and
  re-run the app. `gpui-pre-macos` is patched in `vendor/` (see its `README-HEADQUARTER.md`).
  Shaders build at runtime (`runtime-shaders`), so macOS builds need no Xcode.

## Capability standards

Where a standard does not fit, and what this project does instead:

- `database`: SQLite (`rusqlite`, bundled), not PostgreSQL: a single-user app with a local
  index. Tests run real SQLite (add-session-daemon D2). Migrations run at host startup under
  the single-instance lock; applied ones are checksummed and a changed one stops the host. The
  recorded-migration check alone keeps `0002` from running twice (add-session-history).
- `deployed`: a desktop app serves no requests, so no request ids or health checks. Logs
  carry events only. Releases roll back by fixing forward (`## Release`).
- Checks: `./scripts/check` is the project's CI, run locally before every push (owner,
  2026-10-21). One hosted check: `windows.yml` builds, lints and tests the client crates on
  Windows for every pull request (add-windows-client D6); a red run blocks the merge.
  `release.yml` from `dist` runs on version tags. Not covered: a Linux build, and a secret
  scan of the history (review diffs for secrets).
- Dependencies: `deny.toml` checks `unmaintained` for workspace crates' direct
  dependencies only, because GPUI's deep tree carries unmaintained crates we cannot
  replace (add-terminal-pane, 2026-10-20).

## Release

- Who: the agent on the owner's go
- Kind: installer. Host archives `hq-cli-<target>.tar.gz` (macOS and Linux), app archives
  `hq-app-<target>` (macOS and Linux `.tar.gz`, Windows x86-64 `.zip`, unsigned), `install.sh`
  (full, `--host-only`, `--app-only`), and the signed Android APK
- Trigger: pushing a `v*` tag on `main` runs `release.yml` (`dist`). Its `custom-build-android`
  job (`build-android.yml`, environment `android-release`) builds and signs the APK
- Versions: `vMAJOR.MINOR.PATCH`, one version for every crate and the APK (desktop and phone
  release together). A `-rc.N` pre-release goes first only when the release pipeline changed;
  an `-alpha.N` pre-release only on the owner's request (first: `v0.2.0-alpha.1`, 2026-11-02)
- Notes: a `## X.Y.Z (YYYY-MM-DD)` section at the top of `CHANGELOG.md`, user-facing, which
  `dist` uses as the GitHub release's notes
- Before: a `chore/release-X.Y.Z` pull request sets `version` and every `hq-*` path dependency in
  `crates/*/Cargo.toml`, `Cargo.lock` (`cargo update -w`) and the changelog. `./scripts/check`
  passes on it, then the owner merges it
- Steps: from `main` equal to `origin/main` at the merge commit, `git tag vX.Y.Z <merge commit>`,
  `git push origin vX.Y.Z`, then watch the `release.yml` run until every job succeeds
- Staging and soak: none
- Verify: every job succeeded; `releases/latest` is the new tag (not for a pre-release); the
  assets (4 host and 5 app archives, `install.sh`, `source.tar.gz`, `sha256.sum`, the APK and
  its `.sha256`) match their checksums, and no app archive holds `hq`. `apksigner verify
  --print-certs` shows `CN=Headquarter`, SHA-256
  `7c64a1f2b6ba540fda5e93dc663b23dc6fb3e5bd19811b09dae1293e25ed5f64`, and the version name
  and code. The released `hq --version` prints the version
- Rollback: a published tag is never moved or reused. Fix forward with the next patch release.
  Marking the broken GitHub release as a pre-release (so the app's update check skips it) needs
  the owner's instruction

## References

- Claude Code hooks: https://code.claude.com/docs/en/hooks
- Codex app-server: https://learn.chatgpt.com/docs/app-server
- Antigravity CLI hooks: https://antigravity.google/docs/hooks
