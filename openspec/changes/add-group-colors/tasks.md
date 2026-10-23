# Tasks

## 1. Data and wire

- [x] 1.1 Migration 0007, `GroupRow.color`, `set_group_color`. Verify with an hq-store test (picked, back to automatic). Done: `groups_order_collapse_and_membership` passes, 2026-10-23.
- [x] 1.2 `GroupInfo.color`, `SetGroupColor`, palette names; host stores and announces. Verify with a protocol round-trip test (missing field reads as automatic) and a host test (set, clear, unknown name refused). Done: `automatic_group_colours_follow_the_id`, the groups round trip, and the host groups test pass (unknown names read as automatic), 2026-10-23.

Expected Result: `cargo test -p hq-store -p hq-protocol -p hq-cli` passes.

## 2. Sidebar

- [x] 2.1 Palette tokens per mode; automatic colour by id. Verify with a theme test (eight colours per mode, dot contrast at least 3:1 on the sidebar) and a model test (automatic by id, picked wins). Done: `group_colours_stand_out_on_the_sidebar`, `group_headers_carry_their_colour` pass, 2026-10-23.
- [ ] 2.2 Header dot and strong title, row line, swatch popover, rail chip outline. Verify on screen in a dev build with the owner's groups (light and dark).

Expected Result: the owner sees coloured groups and can pick and reset a colour.

## 3. Delivery

- [ ] 3.1 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the PR is open.
