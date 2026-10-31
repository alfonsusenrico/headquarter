# Design: the update restarts only when the owner says

## D1. Installed, waiting for the owner

The `Waiting` phase becomes `Installed`: the update is installed and the restart waits for the
owner. `restart_if_idle` and its call on every session change go. An install the owner started
with nothing busy still restarts at once (`restart_now`); one started with Wait never does. The
window after Wait: "Update installed", the steps and log, and "Restart with Update Now in the
sidebar when you're ready", with Close and Restart Now.

## D2. The sidebar's Update Now

While `Installed`, the sidebar button reads "Update Now". Pressing it restarts at once when no
session is busy; otherwise it opens the update window on the confirmation, which names the busy
sessions and says restarting interrupts them, with Later and Restart Now. Before an install the
confirmation keeps Wait and Restart Now, and says Wait installs now and leaves the restart to the
owner.

Rejected: keeping an optional "restart when idle" (the owner asked for no automatic restart).
