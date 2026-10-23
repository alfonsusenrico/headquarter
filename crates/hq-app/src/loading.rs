//! What shows while Headquarter is still finding agents or waiting for its host: the
//! agents' marks lighting up in turn above a short line (owner's choice, 2026-10-22).
//! Under Reduce motion the marks stay lit.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, IntoElement, ParentElement, SharedString, Styled, div, px,
};
use hq_signals::event::Agent;

use crate::marks::Marks;
use crate::terminal::view::hsla;
use crate::theme::Chrome;

/// One round of all three marks.
const ROUND: Duration = Duration::from_secs(3);
/// How far apart, as a share of the round, the marks light up.
const STAGGER: f32 = 0.17;
const DIM: f32 = 0.15;

/// The opacity of mark `index` at `delta` through the round: dim, rising, lit, falling.
pub fn brightness(delta: f32, index: usize) -> f32 {
    let t = (delta - index as f32 * STAGGER).rem_euclid(1.0);
    let rise = |from: f32, to: f32, at: f32| DIM + (1.0 - DIM) * ((at - from) / (to - from));
    match t {
        t if t < 0.15 => DIM,
        t if t < 0.30 => rise(0.15, 0.30, t),
        t if t < 0.85 => 1.0,
        t => DIM + (1.0 - DIM) * ((1.0 - t) / 0.15),
    }
}

/// The hosted agents' marks, `size` pixels each, lighting up in turn above `label`.
pub fn agents_checking_in(
    key: &'static str,
    marks: &Marks,
    size: f32,
    chrome: Chrome,
    label: &'static str,
) -> AnyElement {
    let still = crate::settings::reduce_motion();
    let row = Agent::ALL.into_iter().enumerate().map(|(index, agent)| {
        let mark = div().child(marks.element(agent, size, chrome.muted));
        if still {
            return mark.into_any_element();
        }
        mark.with_animation(
            SharedString::from(format!("{key}-{index}")),
            Animation::new(ROUND).repeat(),
            move |mark, delta| mark.opacity(brightness(delta, index)),
        )
        .into_any_element()
    });
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(12.))
        .py(px(12.))
        .child(div().flex().gap(px(18.)).children(row))
        .child(
            div()
                .text_size(px(13.))
                .text_color(hsla(chrome.muted))
                .child(label),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_light_up_in_turn_and_dim_again() {
        assert_eq!(brightness(0.0, 0), DIM);
        assert_eq!(brightness(0.5, 0), 1.0);
        assert!(brightness(0.95, 0) < 1.0 && brightness(0.95, 0) > DIM);
        // The second mark is a step behind the first.
        assert_eq!(brightness(0.2, 1), DIM);
        assert_eq!(brightness(0.2 + STAGGER, 1), brightness(0.2, 0));
        for step in 0..100 {
            let level = brightness(step as f32 / 100.0, 2);
            assert!((DIM..=1.0).contains(&level), "{level}");
        }
    }
}
