# Design: a short offline banner, and `hq remote pair` that scans anywhere

## Decisions

### D1. The banner names the computer and the list's age

Before 10 s of failed tries: "Reconnecting to MacBook… · 1m"; after: "MacBook is offline · 5m"
(owner's choice, 2026-11-09). The age is how long since the list was last current, as before; the
question about remote control goes, since the computers sheet and the state row already say it.

### D2. The QR code is dark on light by its own colours

The `qrcode` crate draws dark modules as filled blocks in the terminal's foreground colour, so a
light-on-dark theme inverts the code. Each line of the code is printed in black on bright white
(SGR 30;107) when standard output is a terminal and `NO_COLOR` is not set; piped output stays plain
text. Under the code: "Get the phone app: <releases/latest/download/headquarter-android.apk>".

Rejected: drawing the code inverted on dark themes. A terminal does not reliably say its theme.
