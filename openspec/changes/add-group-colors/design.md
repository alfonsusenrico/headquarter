# Design

## Decisions

### D1. A fixed soft palette per mode

Eight colours, named for the palette they come from (Catppuccin, MIT; colours only):
mauve, green, teal, peach, blue, pink, yellow, red. Dark mode uses the Mocha values,
light mode the Latte values, whatever theme is chosen, as on the review page. Rejected:
the theme's ANSI colours (soft in Catppuccin, harsh in others), and any colour (no
palette to keep soft).

### D2. Automatic colours stay put

A group with no picked colour shows `palette[(id - 1) % 8]`: consecutive new groups get
consecutive colours, and a colour never changes when groups are reordered, renamed or
others are deleted. Picking stores the colour's name; "Automatic" stores NULL. Rejected:
colouring by position (reordering would recolour groups), and storing the automatic
choice (it would then count as picked).

### D3. The dot is the picker

The header starts with a 9 px rounded-square dot in the group's colour. Clicking it
opens a popover with the eight swatches, each named in its tooltip, and "Automatic";
the current one is ticked. Escape or a click outside closes it. The header keeps its
other clicks: the name renames, the rest collapses. Like renaming and reordering, it
is a mouse action.

### D4. Header and rows

The title is drawn in the strong text colour at 13.5 px semibold instead of the grey
group colour. Rows in a group are indented and drawn beside a 2 px line in the group's
colour, as on the review page. Rows in "Needs you" and "Ungrouped" have no line. A
collapsed group keeps its dot; its waiting dot and amber edge (needs you) are unchanged.

### D5. Icon rail

A collapsed group's chip in the icon rail takes its colour as its outline.

### D6. Wire compatibility

`GroupInfo.color` is optional and defaults to automatic, so an app reading an older
host shows automatic colours; `SetGroupColor` sent to an older host fails with its
usual error until the host restarts on the new version. `PROTOCOL_VERSION` stays 9, as
for earlier additive requests, so updating never forces sessions to end.

## Data

`0007_group_color.sql`: `ALTER TABLE groups ADD COLUMN color TEXT;` NULL is automatic;
an unknown name read back is treated as automatic. Like `0002`, SQLite cannot guard the
column with `IF NOT EXISTS`; the recorded-migration check keeps it from running twice.
Rollback: an older host ignores the column.

## Risks

- Eight colours repeat from the ninth group; the name still tells them apart.
