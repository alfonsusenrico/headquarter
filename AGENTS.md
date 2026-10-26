# Headquarter

## Purpose and boundaries

Headquarter (`headquarter`, alias `hq`) is the owner's desktop terminal for coding-agent
CLIs (Claude Code, Codex, Antigravity `agy`, and others it discovers). It hosts many
agent sessions, keeps them in a sidebar in groups the owner makes, flags each session's
status (working, needs you, idle not read yet, idle, not running), and shows each provider's
usage quota. Targets macOS and Linux.

- **It owns:** the sessions it launches (PTYs, injected per-launch hooks), its
  session index, groups, unread flags, and quota snapshots.
- **It observes only:** agent sessions started elsewhere. It never attaches to,
  signals, or writes to outside processes.
- **It never:**
  - modifies the agents' own configuration (`~/.claude`, `~/.codex`, `~/.gemini`);
    integration is passed per launch (`--settings`, `-c`, environment);
  - persists prompt text or model output; the index keeps metadata only (ids, paths,
    timestamps, status, quota numbers).
- **One exception to leaving agent data alone (owner, 2026-10-23):** removing a past
  conversation the owner chose in Add sessions: Claude Code's transcript (and the folder
  beside it) goes to the system Trash, Codex only through its own `archive` and
  `delete`; agy is never changed (`host/remove.rs`).

Capabilities: database, remote access, mobile app
Design surface: desktop application (session sidebar, terminal pane, quota footer), and a phone
app (Android: home mirroring the sidebar, session terminal with a key row, pairing, settings)
Language: UI text English; relative ages as `0m`, `1m`, `3h`, `2d`, `5w` (no seconds); times in local 12-hour format.

## Commands

```bash
./scripts/check                 # the project's CI, run before every push: fmt, clippy -D warnings, tests, cargo-deny, OpenSpec
cargo fmt --all                 # apply formatting (rustfmt is authoritative)
cargo run -p hq-cli --bin hq -- ls          # the CLI against your real data folder
HQ_DATA_DIR=/tmp/hq-dev cargo run -p hq-cli --bin hq -- new claude     # isolated data folder
./scripts/android-core          # the phone's Rust core and Kotlin binding, also run by Gradle
(cd mobile/android && ./gradlew assembleDebug)   # the Android app (JDK 17, Android SDK and NDK)
cargo run -p hq-app --bin headquarter-app -- --quit-after 5   # open the app, quit after 5 s
openspec list                   # open changes
```

The Rust toolchain is pinned in `rust-toolchain.toml`. Integration tests start real host
processes with stub agents in `/tmp/hqh-*`; socket paths must stay under about 100 bytes.
Isolate runs with real agents through `HQ_DATA_DIR` and `HQ_CONFIG_DIR`, never `XDG_DATA_HOME`:
launched agents inherit it, and Claude Code then installs itself under it and repoints
`~/.local/bin/claude`. Tests must not depend on, or run, what the test machine has installed: they run with
their own `HOME`, config folder and `SHELL`, and with `HQ_SYSTEM_BIN_DIRS` empty, which replaces the
system-wide folders discovery searches (Homebrew's, `/usr/local/bin`, Linux's); set it to a folder of
stubs to test those. Remote control tests run with `HQ_REMOTE_RELAYS=none` (direct addresses, no
public relays); `HQ_REMOTE_PAIR_SECS` shortens the 2-minute pairing code.

## Architecture map

- `crates/hq-signals`: agent status signals: launch adapters and each agent's close
  keys, OSC scanner, status machine, hook normalization, quota readers, external scan,
  Codex sub-agents (`subagents.rs`) and its shared server (`codex_server.rs`), hook
  helpers, and read-only history readers (`history.rs`).
- Titles read from agent history are display-only: never write them to the index, the
  log, or evidence files (owner decision 2026-10-20).
- `crates/hq-store`: SQLite session index and its migrations
  (`crates/hq-store/migrations/`).
- `crates/hq-protocol`: client and host frames and messages, group labels, and
  `sidebar.rs`: the sidebar's order, the owner's groups and folder labels, shared by
  `hq ls` and the app;
  `usage.rs`: how usage readings are shown, shared by `hq usage` and the app.
- `crates/hq-discovery`: platform detection, login-shell `PATH`, the known-agent table
  (`agents.rs`; adding an agent is a row), and the install-folder search.
- `crates/hq-config`: the optional settings file with strict validation. `schema.rs`
  lists every accepted key and the Alacritty keys reported as unsupported;
  `appearance.rs` reads the look and keys; `setup.rs` writes the file for first-run
  setup and edits only setup's keys on a rerun (`toml_edit`, layout and comments kept).
- `crates/hq-daemon`: the host. It owns PTYs, the index, and `agentd.sock` and
  `hooks.sock` in `~/.local/share/headquarter/`. `host/history.rs` imports agent
  history every 60 s (`HQ_HISTORY_INTERVAL_SECS` shortens it in tests), `host/outside.rs` links outside agents every 5 s,
  `host/close.rs` closes a session with the agent's own keys, and `host/usage.rs`
  reads usage only while a client shows it (`HQ_USAGE_INTERVAL_SECS` shortens the 5-minute Codex interval in tests).
  `host/remote.rs` is remote control's gateway: off unless enabled, an iroh endpoint
  on its own thread that passes each paired phone to `agentd.sock` as an ordinary
  client through an allow-list (add-remote-gateway).
- `crates/hq-remote`: remote control's connection, shared by the host's gateway and the
  phone's core: binding the iroh endpoint (relays), pairing codes, and `hq-protocol`
  frames over QUIC.
- `crates/hq-mobile`: the phone's core for the Android app, exposed to Kotlin with UniFFI:
  pairing, the computers it knows, the connection to the computer on screen, home as
  `hq_protocol::sidebar` arranges it, and open terminals (`alacritty_terminal`). Calls block,
  the app waits for changes with `wait_change()`, and nothing calls back into Kotlin (add-android-app).
- `mobile/android/`: the Android app (Kotlin, Jetpack Compose, minSdk 29, application id
  `com.alfonsusenrico.headquarter`). `Core.kt` holds the core and the lifecycle (connected
  only while on screen), `PhoneKey.kt` keeps the phone's key under the Android Keystore,
  `ui/` holds the screens and `ui/Theme.kt` the desktop's tokens. The core's libraries and
  binding are generated (`scripts/android-core`) and not committed.
- `crates/hq-client`: the host connection shared by the CLI and the app; it starts the
  host (`hq` beside the running executable) when none is running.
- `crates/hq-cli`: the `hq` and `headquarter` executables. They are thin entry points
  over `hq_cli::main`.
- `crates/hq-app`: the desktop app `headquarter-app`, a GPUI client of the same
  protocol. `terminal/` holds the model (`alacritty_terminal`), frame building, key
  encoding, the session connection and the painted view; `theme.rs` is the tokens
  file; `settings.rs` applies the settings file and reloads it; `sidebar/` holds the
  sidebar model and the painted status glyphs; `modal/` holds the Open sessions (⌘O)
  and New session (⌘N) windows in one frame and the folder browser (`folders.rs`);
  `workspace/groups.rs` draws the owner's groups and handles dragging sessions into them;
  `shortcuts.rs` keeps app shortcuts off the keys agents use; `menus.rs` (macOS menus),
  `notifications.rs` (when to notify); `setup.rs` is the first-run screen and
  `shell.rs` the root view that shows it before the workspace; `app_state.rs` keeps
  the window, shown session, theme and sidebar in `app-state.json`.
- `crates/hq-probe`: the Phase 0 evidence tool.
- `openspec/`: proposals, designs, and specs; the source of truth for behaviour.
  Archived changes (`openspec/changes/archive/`) stay local, as do `research/` (notes,
  prototypes, evidence) and the `docs/ref *.jpg` references: development material,
  never committed (`.gitignore`). The repository carries the product only.

## Project rules

- Status comes first from OSC 7501, which Claude Code, Codex, and agy all emit once the
  terminal answers its query. Hooks supplement it for Claude (session id, tools,
  reasons). Output activity is a fallback until the first structured report.
- Agent CLI formats (transcripts, rollout files, hook payloads) are unstable external
  contracts: parse defensively, tolerate unknown fields, and record observed versions
  in `research/evidence/`.
- Licence hygiene: do not copy code from AGPL or GPL projects (Warp, Zed's `terminal`
  crate, cmux). Ideas and wire formats only; dependencies must be MIT, Apache-2.0, or
  similarly permissive.
- Remote access: a phone reaches only what `allowed()` in `host/remote.rs` lists, and
  the host enforces it; a new request is decided there (the match is exhaustive). The
  host key (`remote.key`, mode 600), pairing secrets, typed input and terminal output
  never reach logs. Relays default to n0's public ones; a personal relay is a setting,
  never a product default (owner, 2026-10-26).
- The phone app keeps no session content: only its key (Keystore-wrapped, backups off), the
  paired computers' names, keys and relays, and its settings. Its logs carry connection
  events only. Kotlin formats with ktlint, and Android Lint runs with warnings as errors.

- GPUI comes from the `gpui-pre` snapshot crates, pinned with `=`; upgrade them
  together and re-run the app. `gpui-pre-macos` is patched in `vendor/`
  (idle display link); see its `README-HEADQUARTER.md` on upgrade. The app builds shaders at runtime by default
  (`runtime-shaders`), so local macOS builds need no Xcode. Expect about 4 GB in
  `target/`.

## Overrides

- `database` (PostgreSQL by default): Headquarter uses SQLite (`rusqlite`, bundled)
  because it is a single-user desktop app with a local index (add-session-daemon D2,
  2026-10-20).
- `database` (migrations as a separate step before app start): migrations run at host
  startup, under the single-instance lock, so exactly one process can apply them.
  Applied migrations are checksummed and a changed one stops the host
  (add-session-daemon D2, 2026-10-20). `0002` adds a column, which SQLite cannot guard
  with `IF NOT EXISTS`; the recorded-migration check alone keeps it from running twice
  (add-session-history, 2026-10-20).

- CI (hosted CI from the first commit): `./scripts/check` is the project's CI, run
  locally before every push (owner, 2026-10-21). The only hosted workflow, `release.yml`
  from `dist`, runs on version tags (add-release-installer D5). Not covered locally: a
  Linux build, and a secret scan of the history (review diffs for secrets).

- Dependency policy (crates.io only, no unmaintained crates): `deny.toml` checks
  `unmaintained` for workspace crates' direct dependencies only, because GPUI's deep
  tree carries unmaintained crates we cannot replace (add-terminal-pane, 2026-10-20).

## References

- Claude Code hooks: https://code.claude.com/docs/en/hooks
- Codex app-server: https://learn.chatgpt.com/docs/app-server
- Antigravity CLI hooks: https://antigravity.google/docs/hooks
