# Proposal

## Why

The owner's first install (2026-10-22) showed two things to change in setup:
- **Step 1 lists every agent Headquarter knows,** including nine it cannot host yet,
  each "not installed" or "found, not yet supported".
- **Step 2 offers only "Headquarter" or "Use my Alacritty look",** while the app has a
  full theme switcher (⌘G).

## Expected Outcome

After the first preview the owner asked for more (2026-10-22, choices from five options
each in a local preview):
- **Step 1 is a checklist with the agents' marks.** Each row shows the version and a
  tick, or "How to install" linking to the vendor's instructions when the agent is not
  installed.
- **While agents are being found,** the agents' marks light up in turn above "Looking
  for your agents…". The app's first seconds, before the host answers, show the same
  thing, larger, with "Starting Headquarter…" in the main pane, where there is room and
  the eye goes; the sidebar stays blank until the list arrives (owner, 2026-10-23,
  replacing the first placement in the sidebar).
- **Add sessions opened during those first seconds shows the same animation** ("Loading
  your sessions…"), not "No sessions match". The host's first list now waits, at most
  10 s, for its first history import, so the list never arrives without the owner's
  past conversations (found on screen, 2026-10-22).
- **The Ready step drops the settings file's path and backup.** It shows "Ready" and
  the three shortcuts worth knowing (New session, Add sessions, Next one that needs
  you).

- Step 1 shows only the agents Headquarter hosts (Claude Code, Codex, Antigravity),
  then "More agents are on the way."
- Step 2 is a theme picker:
  - it lists the switcher's themes (System, Light, Dark, the templates, the owner's
    themes) and "Your Alacritty look" when an Alacritty file exists;
  - the setup screen takes the theme on as each one is chosen;
  - the choice becomes the app's theme.

## What Changes

- `hq-app` (`setup.rs`):
  - agent rows only for hostable agents, and the closing line;
  - the look step built from `theme_switcher::rows`, with its Alacritty row standing for
    the whole Alacritty look (window, padding, font, cursor, colours, keys), written as
    before;
  - the chosen theme is saved as the app's theme (`app-state.json`);
  - Up and Down move through the themes;
  - "Change it any time with Themes (⌘G)."

- `hq-discovery`: each hosted agent's install page (`install_url`) in the known-agent
  table.
- `hq-daemon`: the first list request after start waits for the first history import
  (`history::FIRST_IMPORT_WAIT`, 10 s).
- `hq-app`:
  - `loading.rs`: the marks lighting up in turn (still under Reduce motion), used by
    setup and by the workspace until the host's first list;
  - the Ready step shows the shortcuts; failures still show their message.

Decisions (owner, 2026-10-22):
- **The checklist with marks** (E of five). **The marks lighting up in turn** (B of
  five). **"Ready" with three shortcuts** (E of five). Rejected options are in the local
  preview `research/prototypes/setup-refresh/`.
- **Supported agents and one line.** Rejected: naming the unsupported agents found on
  this Mac.
- **The theme picker inside setup.** Rejected: dropping the look step and opening the
  switcher after setup.

## Non-goals

- Creating or editing themes in setup (the switcher's Add, Duplicate, Edit and Delete).

## Capabilities

### Modified Capabilities
- `first-run-setup`: supported agents only; the look step is a theme picker.

## Impact

`crates/hq-app/src/setup.rs`, `loading.rs`, `workspace.rs`, and
`crates/hq-discovery/src/agents.rs`.

## Approval

Approved by the owner on 2026-10-22 ("Supported + one line", "Theme picker inside setup").
