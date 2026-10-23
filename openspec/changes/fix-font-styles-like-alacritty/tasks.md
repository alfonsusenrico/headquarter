# Tasks

## 1. Lookup

- [x] 1.1 Keep style names in `FontSettings`; resolve each style with a lookup, falling back to the normal face; notes. Verify with unit tests using a fake font list, and a macOS test with Menlo ("Bold" found, "Heavy" not). Done: `styles_the_font_lacks_use_the_normal_face_like_alacritty`, `styles_the_font_has_keep_their_face`, `core_text_lists_the_faces_of_a_family` pass; hq-app 135, hq-config 37 tests (2026-10-23).

Expected Result: `cargo test -p hq-config -p hq-app` passes.

## 2. On screen and delivery

- [ ] 2.1 On screen: with the owner's original config (bold "Heavy"), bold text in Headquarter matches Alacritty, and the note names "Heavy". Evidence: window captures side by side.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the owner sees the same bold text in both apps, and the PR is open.
