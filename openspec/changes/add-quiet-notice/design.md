# Design: a quiet notice

## D1. Where and how it looks

A card about 320 px wide, 16 px from the window's bottom and right edges, over the terminal pane:
the message, and a close button (×). It uses the raised surface and border tokens, so it follows
the theme. One notice at a time; a new one replaces the old.

## D2. Motion and time

It slides in from the right over 200 ms (ease-out) and out over 160 ms, within the motion budget;
with reduced motion it fades in and out over 120 ms. An informational notice leaves after 5 s; the
pointer over it holds the timer. An error stays until dismissed. It never takes the keyboard, so the
session keeps typing.

## D3. The manual update check

"Headquarter X is the latest version." and "Couldn't check for updates: …" go to the notice. The
update window still opens for an available update, installing, and the host-behind case.

Rejected: a banner across the top (it covers the session header), and a system notification (it
leaves the app and lingers in Notification Center).
