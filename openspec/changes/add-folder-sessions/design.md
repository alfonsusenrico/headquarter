# Design: sessions already in a folder

## Context

The app's session list (`SessionInfo`) holds sidebar sessions and imported past conversations
(`origin: history`), each with `cwd`, `title`, `last_activity_at` and `in_sidebar`. The New
session window (`modal/new_session.rs`) browses folders (`folders.rs`) and starts on "Open here".

## Decisions

### D1. Counts while browsing

Sessions are counted by folder (`cwd`, exact match after `~` expansion), leaving out outside
sessions. A folder row with any shows "N sessions" (1: "1 session") in muted text, right
aligned. No split between sidebar and past. Past conversations count only when Add sessions
offers them (`offered`): one its agent no longer keeps, or a run a program started, cannot be
continued, so counting it would promise a session the dialog never lists.

### D2. The check on Start

On Start, the app takes the chosen folder's counted sessions (D1) that are not in the sidebar
and not in the owner's "don't ask again" list. None: start at once, as today. Any: the dialog.

### D3. The dialog

Over the New session window, titled "Not in your sidebar" with the folder. One row per offered
session, most recent first: agent mark, title (or the agent's name and short id when untitled),
"last used 2h ago" in the app's relative ages, and Continue. Below: a "Don't ask again for
these" checkbox, then Cancel and Start new session (equal width, Start new session primary).
Continue adds the session to the sidebar (`Add`), resumes it (`Resume`), shows it and closes
both windows. Enter is Start new session, Esc is Cancel (back to the New session window).

### D4. Don't ask again

Ticked, Start new session or Continue stores the listed sessions' ids in `app-state.json`
(`not_asked`). Those are not offered again. Another forgotten session in the folder still is.
Ids of sessions that no longer exist are dropped when the state is saved.

### D5. The tag

On the highlighted folder row, "Open here", the count and the › use the group colour, as the
options page drew them, and muted text elsewhere. Muted on the highlighted row's accent tint
was 4.10:1 in dark. A theme test checks AA for each on its row, highlighted or hovered, in both
themes.

Rejected: accent text on an accent-soft fill. It would be the only filled tag in the app.

## Risks

- Folders compared by exact path: a session started through a symlinked path counts under that
  path only.
