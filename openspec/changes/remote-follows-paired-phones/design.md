# Design: remote control follows the paired phones

## D1. When the gateway runs

The host runs the gateway when at least one phone is paired, or while a pairing code is shown. At
start it runs if any phone is paired. A pairing request starts it if needed; when the code expires or
is cancelled with no phone paired, it stops. Removing a phone (from the desktop, the CLI or the phone
itself) stops it when none is left. Stopping closes every connection, as turning it off does today.
The host keeps running while the gateway runs, as today.

## D2. The setting

`[remote] enabled` is still accepted, so existing settings files load, and ignored. `relays` keeps
its meaning. The settings reference says the key no longer does anything. Migration: an owner with
phones paired keeps remote control; one with it on and no phone paired loses only an idle endpoint.

## D3. CLI

`hq remote on` and `hq remote off` go: the commands left are status, pair, devices and remove. The
protocol's `RemoteSet` stays (older apps send it) and answers with the status without changing it.

Rejected: keeping `off` as a pause. The owner asked for no toggles; removing the phones is the off.

## D5. The phone hears it was removed

When the last phone is removed while it is connected, the gateway stops right after closing its
connection, so the phone would only find the computer unreachable. The host already closes that
connection with the reason `removed`; the phone core now takes that reason as "removed from this
computer" (`LinkStatus::NotKnown`) instead of retrying. A phone that was offline when it was
removed finds nothing listening once no phone is paired, and shows the computer as unreachable.

## D4. The window

The switch goes. The first step always shows the app's QR code and Copy link; the "For another
phone" line and its Show button go. "How the computer is reached" shows while the gateway runs, and
otherwise "Pair a phone to turn remote control on".
