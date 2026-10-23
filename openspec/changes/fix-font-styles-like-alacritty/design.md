# Design

## Decisions

### D1. Alacritty's macOS lookup

Alacritty (crossfont) lists the family's faces with Core Text and takes the one whose
style name equals the configured style exactly; when there is none, the bold, italic or
bold-italic font falls back to the regular font. Headquarter does the same for those
three styles: an exact style-name match keeps the style's weight and slant; no match
uses the normal weight, upright. A family Core Text does not know is left to GPUI's
fallback as before. Rejected: keeping the nearest-weight mapping (the reported
difference), and matching names case-insensitively (Alacritty does not).

### D2. Lookup in the app, decision testable without fonts

`hq-config` only parses; the app owns the platform lookup. The decision takes a lookup
function, so tests use a fake font list; one macOS test uses Menlo, which ships with
macOS.

### D3. Quiet, like Alacritty

A fallback is written to the app's log only, naming every style that falls back, for
example: `font.bold "Heavy" and font.bold_italic "Heavy Italic" not in JetBrainsMono
Nerd Font; that text uses the normal face, as in Alacritty`. Alacritty, too, only logs
it. Rejected: a settings note in the sidebar (shown on screen 2026-10-23; the owner
chose silence).

### D4. Line height from the font

Alacritty takes the line height from the font: ascent, descent and leading, each
rounded (18 pt for JetBrains Mono at 14). Headquarter used 1.2 times the size (17 pt),
so its lines were tighter than Alacritty's on the owner's screen (2026-10-23). The pane
now uses the font's ascent plus descent, each rounded, with 1.2 times the size only when
the font reports no metrics. GPUI does not expose the line gap (leading); terminal fonts
commonly set it to 0, as JetBrains Mono, Menlo and SF Mono do. Rejected: a line-spacing
setting (Alacritty has `font.offset`, still reported as unsupported; not asked for).
