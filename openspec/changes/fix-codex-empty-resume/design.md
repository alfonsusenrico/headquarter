# Design: an empty Codex session starts fresh

## Decisions

### D1. Codex without a conversation id starts a new conversation

`codex::launch_spec(Conversation::Resume(None))` gives no arguments, as for a new session and as
agy already does. The host's other paths (resume, start with the host) are unchanged: they ask for
`Resume(None)` when the id is unknown.

Rejected: keeping the picker for a resume the owner asks for and starting fresh only on a host
restart. The session is the same empty one either way, and the picker's list is other sessions'
conversations. Rejected: storing the launch options to resume with them. They can carry a prompt.

## Risks

A Codex session whose conversation Headquarter has not learned yet (its id arrives with Codex's
first report or the history scan) would start fresh instead of continuing. The conversation stays
in Codex's history and can be added from Add sessions.
