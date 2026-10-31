//! A quiet notice at the window's bottom right (add-quiet-notice): it slides in, leaves by
//! itself after a few seconds unless the pointer is on it, can be dismissed, and never takes
//! the keyboard. A notice reporting a failure stays until dismissed.

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    Animation, AnimationExt, AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::{Icon, IconName, Sizable, button::Button, button::ButtonVariants};

use super::Workspace;
use crate::terminal::view::hsla;

/// How long an informational notice stays (D2).
const STAY: Duration = Duration::from_secs(5);
const SLIDE_IN: Duration = Duration::from_millis(200);
const SLIDE_OUT: Duration = Duration::from_millis(160);
/// With reduced motion it only fades.
const FADE: Duration = Duration::from_millis(120);
/// How far it slides, in pixels.
const TRAVEL: f32 = 40.0;
const WIDTH: f32 = 320.0;
const EDGE: f32 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Notice {
    text: String,
    /// A failure stays until dismissed.
    error: bool,
    /// Tells this notice's timers and animations from an earlier one's.
    id: u64,
    hovered: bool,
    leaving: bool,
}

/// Whether a notice shown for `shown` should leave now (D2).
fn leaves_by_itself(error: bool, hovered: bool, shown: Duration) -> bool {
    !error && !hovered && shown >= STAY
}

impl Workspace {
    /// Shows `text` in the notice, replacing any notice shown before.
    pub(super) fn show_notice(
        &mut self,
        text: String,
        error: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notice_count += 1;
        let id = self.notice_count;
        self.notice = Some(Notice {
            text,
            error,
            id,
            hovered: false,
            leaving: false,
        });
        cx.notify();
        if error {
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            let mut shown = Duration::ZERO;
            loop {
                let step = if shown < STAY {
                    STAY - shown
                } else {
                    Duration::from_millis(500)
                };
                cx.background_executor().timer(step).await;
                shown += step;
                let leave = this.update(cx, |this, _| {
                    let notice = this.notice.as_ref().filter(|notice| notice.id == id)?;
                    Some(leaves_by_itself(notice.error, notice.hovered, shown))
                });
                match leave {
                    Ok(Some(true)) => break,
                    Ok(Some(false)) => continue,
                    // Replaced, dismissed, or the workspace is gone.
                    _ => return,
                }
            }
            let _ = this.update_in(cx, |this, window, cx| this.dismiss_notice(id, window, cx));
        })
        .detach();
    }

    /// Slides notice `id` out, then removes it.
    fn dismiss_notice(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(notice) = self.notice.as_mut().filter(|notice| notice.id == id) else {
            return;
        };
        notice.leaving = true;
        cx.notify();
        let out = if crate::settings::reduce_motion() {
            FADE
        } else {
            SLIDE_OUT
        };
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(out).await;
            let _ = this.update(cx, |this, cx| {
                if this.notice.as_ref().is_some_and(|notice| notice.id == id) {
                    this.notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn render_notice(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let notice = self.notice.as_ref()?;
        let chrome = self.theme.chrome;
        let id = notice.id;
        let card = div()
            .id(SharedString::from(format!("notice-{id}")))
            .occlude()
            .absolute()
            .bottom(px(EDGE))
            .right(px(EDGE))
            .w(px(WIDTH))
            .flex()
            .items_start()
            .gap(px(10.))
            .py(px(10.))
            .pl(px(14.))
            .pr(px(8.))
            .rounded(px(10.))
            .border_1()
            .border_color(hsla(if notice.error {
                chrome.amber
            } else {
                chrome.sidebar_line
            }))
            .bg(hsla(chrome.selected))
            .shadow_lg()
            .text_size(px(13.))
            .text_color(hsla(chrome.text))
            .on_hover(cx.listener(move |this, over: &bool, _, _| {
                if let Some(notice) = this.notice.as_mut().filter(|notice| notice.id == id) {
                    notice.hovered = *over;
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .pt(px(3.))
                    .child(notice.text.clone()),
            )
            .child(
                Button::new(SharedString::from(format!("notice-close-{id}")))
                    .ghost()
                    .small()
                    .child(Icon::new(IconName::Close).small())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.dismiss_notice(id, window, cx);
                    })),
            );
        let still = crate::settings::reduce_motion();
        let leaving = notice.leaving;
        let (length, phase) = match (still, leaving) {
            (true, _) => (FADE, if leaving { "fade-out" } else { "fade-in" }),
            (false, false) => (SLIDE_IN, "in"),
            (false, true) => (SLIDE_OUT, "out"),
        };
        let shown = move |card: gpui::Stateful<gpui::Div>, delta: f32| {
            let amount = if leaving { 1.0 - delta } else { delta };
            card.opacity(amount).when(!still, |card| {
                card.right(px(EDGE - TRAVEL * (1.0 - amount)))
            })
        };
        Some(
            card.with_animation(
                SharedString::from(format!("notice-{id}-{phase}")),
                Animation::new(length).with_easing(|t: f32| 1.0 - (1.0 - t).powi(3)),
                shown,
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_informational_notice_leaves_after_five_seconds_unless_pointed_at() {
        assert!(!leaves_by_itself(false, false, Duration::from_secs(4)));
        assert!(leaves_by_itself(false, false, STAY));
        assert!(
            !leaves_by_itself(false, true, Duration::from_secs(9)),
            "held"
        );
        assert!(
            leaves_by_itself(false, false, Duration::from_secs(9)),
            "let go"
        );
    }

    #[test]
    fn a_failure_stays_until_dismissed() {
        assert!(!leaves_by_itself(true, false, Duration::from_secs(60)));
    }
}
