# Proposal

## Why

The owner's first install (2026-10-22) showed two things to change in setup:
- **Step 1 lists every agent Headquarter knows,** including nine it cannot host yet,
  each "not installed" or "found, not yet supported".
- **Step 2 offers only "Headquarter" or "Use my Alacritty look",** while the app has a
  full theme switcher (⌘G).

## Expected Outcome

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

Decisions (owner, 2026-10-22):
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

`crates/hq-app/src/setup.rs` and its tests.

## Approval

Approved by the owner on 2026-10-22 ("Supported + one line", "Theme picker inside setup").
