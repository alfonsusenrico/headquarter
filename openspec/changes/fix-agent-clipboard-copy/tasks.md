# Tasks

## 1. Copy

- [x] 1.1 Keep OSC 52 copies in the model and write them to the clipboard. Verify with a model test (a copy is kept and decoded; a read request gets no reply). Done: `agent_copies_are_kept_and_reads_are_refused` passes; hq-app 140 tests, clippy clean (2026-10-23).

- [x] 1.2 Headquarter owns drags: select, copy on release; clicks without a drag go to an agent that uses the mouse. Verify with `drags_select_and_copy_while_clicks_reach_the_agent`. Done: passes; hq-app 141 tests, clippy clean (2026-10-23).

- [x] 1.3 Copied note, tidy copies, selection text colour, drag tracking outside the pane and edge scrolling. Verify with `margins_trailing_spaces_and_blank_edges_go`, `a_selection_started_mid_row_keeps_the_others_margin_right`, `the_note_counts_lines`, `dragging_past_an_edge_scrolls_faster_further_out`, `selection_paints_its_background` (theme text colour) and `selection_text_colour_from_a_theme`. Done: pass; hq-app 146 tests, clippy clean (2026-10-23).

- [x] 1.4 Selection edges by the half of the cell under the pointer; drags in full-screen mouse agents go to the agent (movement included), with the note on its copies. Verify with `a_selection_made_leftwards_takes_the_first_cell` and `only_full_screen_mouse_agents_take_drags`. Done: pass; hq-app 148 tests, clippy clean (2026-10-23).

Expected Result: `cargo test -p hq-app` passes.

## 2. On screen and delivery

- [ ] 2.1 On screen: drag over text in real Claude Code and Codex sessions in a dev build, release, then paste elsewhere; click a Claude Code control and check it still responds. Evidence: the owner's confirmation.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the owner can copy from Claude Code, and the PR is open.
