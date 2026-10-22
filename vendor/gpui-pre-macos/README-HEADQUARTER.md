# gpui-pre-macos 0.3.9, patched

A copy of [`gpui-pre-macos` 0.3.9](https://crates.io/crates/gpui-pre-macos) (Apache-2.0,
`LICENSE-APACHE`), used through `[patch.crates-io]` in the workspace manifest.

Changed by Headquarter (2026-10-21, change `fix-idle-display-link`): a window whose
display link ticks for one second without any frame demand stops the link, and the
platform's `frame_waker` (GPUI's existing hook for demand-driven platforms) starts it
again. Unpatched, a visible window wakes the app on every display refresh even when
nothing changes, about 100 times a second. Changes are marked "Headquarter patch" in
`src/window.rs`; `src/display_link.rs` gains `WindowFrameSource::is_running`; `Cargo.toml` allows the
`deprecated` lint, which a registry build caps anyway.

Upgrade it together with the other `gpui-pre` crates: copy the new version's crate,
reapply the marked changes, and drop this copy once upstream stops idle display links.
