# Tasks

## 1. Setup

- [x] 1.1 Supported agents only, with "More agents are on the way." Verify with `installed_hostable_agents_are_ticked`. Done (2026-10-22).
- [x] 1.2 The theme picker step with live preview, the Alacritty look as one of its themes, and the choice kept as the app's theme. Verify with `the_alacritty_look_is_one_theme_among_the_others` and the app tests. Done: 126 app tests pass.
- [x] 1.4 Checklist rows with marks and install links, the marks lighting up while agents are found and while the host starts, and the short Ready step. Verify with `marks_light_up_in_turn_and_dim_again` and the app tests. Done: 127 app tests pass (2026-10-22).
- [x] 1.5 Add sessions shows the animation until the first list, and the host's first list waits for the first history import. Verify with `the_first_list_waits_for_the_first_history_import` (fails without the wait, three runs of three) and the host tests. Done (2026-10-22).
- [x] 1.3 On screen in the installed app (`headquarter-app --setup`): both steps, choosing themes, the Alacritty look, and the app opening in the chosen theme. Done: the owner confirmed it on the test build (`test/combined-build`), 2026-10-23.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; push and open the PR.
