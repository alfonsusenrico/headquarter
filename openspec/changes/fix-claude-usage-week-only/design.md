# Design: Claude usage without a five-hour window

## D1. One all-models window is enough

`parse_usage_screen` accepts a screen with at least one all-models window (week or five hours)
and fails only when it has none. Blocks keep their meaning; a five-hour block, when shown, is read
as before. The 2.1.316 screen is added as a test fixture.

## D2. No standing five-hour row

Claude's standing windows become the week only. When a reading has no five-hour window, the details
show no five-hour row, and the ring is the week alone (one ring, as for agents with one window).

Rejected: keep the five-hour row as "reset before this reading" (it would show for good on
2.1.316 and suggest a limit that is not reported). Rejected: a "not reported" row (a row that says
nothing useful).
