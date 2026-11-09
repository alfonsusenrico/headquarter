# Verification: add-merged-host-list

Revision: `feat/merged-host-list` ca3cfec, rebased onto `main` f6c579d (step 1, #133). On screen ran on the same code before the rebase.

| Check | Result |
| --- | --- |
| Unit tests `merged::tests` (names merge across hosts, display ids kept), `sidebar::model` `another_computers_rows_carry_its_name_home_and_state`, `notifications` `a_notice_names_another_computer` | passed |
| `./scripts/check` at ca3cfec (after the rebase) | passed: 643 tests (1 ignored), OpenSpec 19/19 strict |
| On screen, 2026-11-09: this Mac's sandbox host A plus sandbox host B (own data, config and home; stand-in agents; relays off), the debug app on A | see below |

On screen (debug app, synthetic sessions "fix login", "write docs" on A and "health check",
"read logs" on B; each host has its own "ops" group):

- Hosts menu: "This Mac · connected"; Connect to a Host paired B and the header read "2 hosts".
- One list: B's sessions joined A's, a tag with B's name only on B's rows; both "ops" groups
  showed as one group with 2 sessions.
- B's session attached over the link; the footer named B; switching to A's session was
  immediate.
- B frozen (`kill -STOP`) about 40 s: "Reconnecting to <B>…" in the sidebar, B's rows dimmed, A's
  unchanged. Resumed: B reconnected within about 1 s and its rows came back.
- New Session with B's session selected: "Start on" offered This Mac and B, B chosen, B's folders
  listed; Start session ran the agent in B's home and the new row carried B's tag.
- Moving: B's session dropped onto "ops" went into B's own "ops" (`hq ls` on B). A group made on
  this Mac ("notes"), then B's "read logs" dropped onto it: B made its own "notes" and moved the
  session there; the list showed one "notes" with both.
- Needs you on B (a stand-in reporting a permission request, B's session not shown): the row moved
  to Needs you with B's tag. The macOS banner itself cannot show from the debug app: GPUI turns
  system notifications off outside an app bundle (`vendor/gpui-pre-macos/src/system_notifications.rs`).
  The title "<title> · <host>" is covered by `a_notice_names_another_computer`.
- Forget: the first click asked "Forget <B>?" with "You'll need a new code from it to connect
  again."; Cancel kept B; Forget then Forget removed B's sessions, the header read "1 host" and
  the app's paired hosts were empty.
- Found and fixed on screen (c6e32f8): the host tag hid long titles (the tag now shares the title's
  space and shrinks first); the header showed B's full path instead of `~/logs`; a stopped
  session of another computer would have read "The folder … no longer exists" because its folder
  was checked on this Mac.

Not verified yet:
- The notification banner from another computer (needs the installed, bundled app: 4.3).
- Escape closing Forget's question (Cancel and closing the menu share the same reset).
- The owner's server and Windows PC (4.3).
- Test setup limit: with relays off, a restarted host comes back on new ports that the app cannot
  learn; the app reached B again only after its addresses were edited. Relays (the default)
  cover this.
