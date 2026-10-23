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

### D3. Said, not silent

Each fallback adds a settings note, for example: `font.bold style "Heavy" is not in
JetBrainsMono Nerd Font; bold text uses the normal face, as in Alacritty`.
