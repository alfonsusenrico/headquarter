# Verification: update-restart-on-demand

Branch `feat/update-restart-on-demand`, 2026-10-31.

## Checks

- `./scripts/check` passed after the wording fix: 581 tests, lint, Android build, OpenSpec.
- New test `the_pill_offers_update_now_once_installed`; the Escape test covers `Installed`.

## On screen (the owner freed the Mac, 2026-10-31)

Sandbox `/private/tmp/hqo` (synthetic `HOME` whose install manifest lists the dev build; stand-in
agents; the update stand-in serving v0.1.99 with `STANDIN_SUCCEED=1`, so its installer reports
success and changes nothing). Sessions: "api migration" (background agent), "release plan" (plan
to review), "notes" (idle). Captures in `research/evidence/update-restart-on-demand/`:

- Update Now: "2 sessions are busy: api migration, release plan", with Wait and Restart Now and
  the text that Wait installs now (`confirm-before-install.png`).
- Wait: the stand-in install finished; "Update installed … Restart with Update Now in the sidebar
  when you're ready"; the sidebar button reads "Update Now". Both sessions then went idle and
  nothing restarted for 15 s (no "host stopping" in the log) (`installed-idle-no-restart.png`).
- With the background agent running again, the sidebar's Update Now: "1 session is busy: api
  migration" with Later and Restart Now (`confirm-from-update-now.png`). Later: back to the sidebar
  with "Update Now" (`later-keeps-update-now.png`).
- Found on screen and fixed: one busy session read "Restarting interrupts them"; it now reads
  "interrupts it" (and Wait's text says "installs the update now").
- A first click went to the owner's own Headquarter coming to the front; driving stopped until the
  owner freed the Mac again. Nothing was sent to the owner's app.

## Not verified

- Restart Now and the sidebar's Update Now with nothing busy (they restart onto the installed
  copy, which would reopen the app outside the sandbox). Restarting is unchanged code
  (`restart_for_update`).
