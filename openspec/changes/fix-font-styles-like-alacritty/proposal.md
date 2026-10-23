# Proposal

## Why

The owner compared the same Claude Code session in Headquarter and in Alacritty, with
settings copied from Alacritty (2026-10-23): bold text was far heavier in Headquarter.
Their config asks for `bold = "Heavy"` and `bold_italic = "Heavy Italic"`, which
JetBrainsMono Nerd Font does not have. On macOS, Alacritty looks for a face with exactly
that style name and, finding none, draws those styles with the normal face. Headquarter
turned "Heavy" into weight 900 and macOS took the nearest face, ExtraBold.

## Expected Outcome

- With a config copied from Alacritty, bold and italic text in Headquarter look as they
  do in Alacritty on macOS, including when a named style is missing from the font.
- The settings note says when a named style is not in the font and which face is used.

## What Changes

- `hq-config` (`appearance.rs`): keeps the style names of the bold, italic and bold
  italic faces as written, besides their weights.
- `hq-app` (`settings.rs`, `terminal/view.rs`): on macOS, a style name that names no face
  of the font's family makes that style use the normal face (normal weight, upright),
  as Alacritty does, and adds a settings note. Faces are listed with Core Text
  (`core-text`, MIT or Apache-2.0, already in the lockfile through GPUI).

## Non-goals

- Linux: Alacritty asks fontconfig for the nearest match there, which is what
  Headquarter already does; unchanged.
- Separate families per style (still reported, not used).

## Capabilities

### Modified Capabilities
- `settings`: font styles are looked up as Alacritty does on macOS.

No project capability is added.

## Impact

`crates/hq-config/src/appearance.rs`, `crates/hq-app/src/settings.rs`,
`crates/hq-app/src/terminal/view.rs`, `crates/hq-app/Cargo.toml` (macOS-only
dependency). Compared against Alacritty with the owner's config.

## Approval

Approved by the owner on 2026-10-23 ("Both": settings now, and this code change).
