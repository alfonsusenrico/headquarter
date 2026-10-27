# Design: faster terminal drawing on the phone

## Context

`TerminalView` draws the core's `Screen` (rows of styled spans) on a Compose `Canvas`, one
`drawText` per span through a `TextMeasurer` with a 512-entry cache. Findings:
`research/evidence/cache-terminal-row-layouts/findings.md`.

## Goals and non-goals

Goals: fewer layouts and paint calls per frame, the same look. Non-goals: core or protocol
changes, partial invalidation.

## Decisions

### D1. One layout per row, kept while the row is unchanged

Each row becomes one `AnnotatedString`: spans in order, spaces up to each span's column, a
`SpanStyle` per span (colour, dim, bold, italic, underline). It is measured once without
constraints (`softWrap = false`, one line) and kept in an LRU keyed by the row's spans
(`List<Span>`, compared by value), 400 rows, so a few screens of scrollback are kept too. The
row is drawn at `(padding, padding + row × cell height)`. Backgrounds and the cursor are drawn
as today.

Rejected:
- Keep one `drawText` per span with a fixed `size` (experiment A): fixes the cache key, but
  about 500 paint calls a frame remain (31 ms against 16 ms).
- Record each row as a picture (`RenderNode`): fewer paint costs still, but more code and
  memory, and not needed for the measured goal.

### D2. Rows with wide characters fall back to spans

A span whose code point count differs from its cell count (wide or combining characters) would
drift if laid out in one text. Such a row is drawn span by span, each at its column, with each
span's layout kept in its own LRU (400 spans) keyed by the span.

### D3. The exact advance as the cell width

The grid and positions use the monospace font's advance measured over 100 "M"s, a fraction of a
pixel, instead of one "M" rounded to whole pixels. Positions within a row then match the grid
across its full width, and the columns that fit are computed from the same width.

## Failure behaviour

A layout that fails to build cannot happen for plain text. Memory is bounded by the two LRUs
(800 small layouts). Both are dropped when the text size, density or theme changes.

## Risks

- A font fallback glyph that is not one cell wide in a row the check considers narrow (a symbol
  missing from JetBrains Mono): the rest of that row could shift. Checked on screen with box
  drawing, arrows and a CJK row.
