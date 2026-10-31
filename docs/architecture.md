# Architecture

Where each part of Headquarter lives. `AGENTS.md` holds the rules, and `openspec/specs/`
the behaviour.

- `crates/hq-signals`: agent status signals: launch adapters and each agent's close
  keys, OSC scanner, status machine, hook normalization, quota readers, external scan,
  Codex sub-agents (`subagents.rs`) and its shared server (`codex_server.rs`), hook
  helpers, and read-only history readers (`history.rs`).
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
  `host/remote.rs` is remote control's gateway: running only while a phone is paired or a code is shown, an iroh endpoint
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
