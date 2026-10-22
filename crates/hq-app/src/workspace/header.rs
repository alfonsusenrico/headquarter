//! The session header above the terminal pane
//! (`openspec/changes/add-session-header/design.md`, D1 to D4): title, folder and
//! branch, then the model, the context meter, what is left and the cost, each only
//! when the session's agent reports it.

use std::time::{Duration, Instant};

use gpui::{
    AnyElement, ClickEvent, ClipboardItem, Context, InteractiveElement, IntoElement, MouseButton,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::tooltip::Tooltip;
use hq_protocol::SessionInfo;
use hq_signals::details::SessionDetails;

use super::Workspace;
use crate::header::{self, HeaderLine};
use crate::terminal::view::hsla;

const HEADER_HEIGHT: f32 = 28.0;
/// How long "Copied" shows after the folder is copied.
const COPIED_FOR: Duration = Duration::from_secs(1);
const LEFT_NOTE: &str = "Context left in the model's window. Automatic compaction runs near the end of it unless its threshold was changed.";

impl Workspace {
    /// Tells the host which session the pane shows, when that changed.
    pub(super) fn follow_shown(&mut self) {
        let shown = self.shown_id().map(str::to_owned);
        if self.followed.as_ref() == Some(&shown) {
            return;
        }
        let Some(control) = self.feed_control.as_mut() else {
            return;
        };
        if control.follow(shown.as_deref()).is_ok() {
            self.details_open = false;
            self.followed = Some(shown);
        }
    }

    /// The pane: the session's header over its terminal or its details.
    pub(super) fn render_pane(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let body = self.render_pane_body(cx);
        let Some(session) = self.shown_id().and_then(|id| self.session(id)).cloned() else {
            return body;
        };
        let pane_width = f32::from(window.viewport_size().width) - self.sidebar_width();
        let header = self.render_session_header(&session, pane_width, cx);
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .child(header)
            .child(div().flex_1().min_h(px(0.)).child(body))
            .children(
                self.details_open
                    .then(|| self.render_details_popover(&session, cx)),
            )
            .into_any_element()
    }

    fn render_session_header(
        &self,
        session: &SessionInfo,
        width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let background = super::translucent(self.theme.palette.background, self.appearance.opacity);
        let details = self.details.get(&session.id).cloned().unwrap_or_default();
        let title = hq_protocol::sidebar::display_title(session);
        let full_folder = hq_protocol::sidebar::home_path(&session.cwd, self.home.as_deref());
        let line = HeaderLine::new(&title, &full_folder, &details).fitted(width);
        let separator = || div().flex_none().text_color(hsla(chrome.track)).child("·");
        let muted = |text: String| div().flex_none().text_color(hsla(chrome.muted)).child(text);
        let copied = self.copied_at.is_some_and(|at| at.elapsed() < COPIED_FOR);
        let copy_path = session.cwd.clone();

        let mut left = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(7.))
            .child(self.marks.element(session.agent, 18., chrome.muted))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(hsla(chrome.text))
                    .child(line.title.clone()),
            )
            .child(separator())
            .child(
                div()
                    .id("header-folder")
                    .flex_none()
                    .cursor_pointer()
                    .text_color(hsla(chrome.muted))
                    .hover(|style| style.underline())
                    .child(if copied {
                        "Copied".to_owned()
                    } else {
                        line.folder.clone()
                    })
                    .tooltip(move |window, cx| {
                        Tooltip::new(format!("{full_folder} (click to copy)")).build(window, cx)
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(copy_path.clone()));
                        this.copied_at = Some(Instant::now());
                        cx.notify();
                        // Redraw once more when "Copied" should go.
                        cx.spawn_in(window, async move |this, cx| {
                            cx.background_executor().timer(COPIED_FOR).await;
                            let _ = this.update(cx, |_, cx| cx.notify());
                        })
                        .detach();
                    })),
            );
        if let Some(branch) = &line.branch {
            left = left.child(separator()).child(muted(format!("⎇ {branch}")));
        }

        let mut right = div().flex_none().flex().items_center().gap(px(7.));
        let mut first = true;
        let mut push = |right: gpui::Div, element: AnyElement| {
            let right = if first {
                right
            } else {
                right.child(separator())
            };
            first = false;
            right.child(element)
        };
        if let Some(model) = &line.model {
            right = push(right, muted(model.clone()).into_any_element());
        }
        if let Some(used) = line.used {
            right = push(right, self.render_meter(used, cx));
        }
        if let Some(left_words) = &line.left {
            right = push(
                right,
                div()
                    .id("header-left")
                    .flex_none()
                    .text_color(hsla(chrome.muted))
                    .child(left_words.clone())
                    .tooltip(|window, cx| Tooltip::new(LEFT_NOTE).build(window, cx))
                    .into_any_element(),
            );
        }
        if let Some(cost) = &line.cost {
            right = push(right, muted(cost.clone()).into_any_element());
        }

        div()
            .h(px(HEADER_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(10.))
            .text_size(px(12.5))
            .bg(background)
            .border_b_1()
            .border_color(hsla(chrome.sidebar_line))
            .child(left)
            .child(right)
            .into_any_element()
    }

    /// A 40 px bar and "62% context"; amber from 80% used. Clicking opens the details.
    fn render_meter(&self, used: f64, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let color = if used >= 80.0 {
            chrome.amber
        } else {
            chrome.text
        };
        let filled = (used / 100.0).clamp(0.0, 1.0) as f32 * 40.0;
        div()
            .id("header-meter")
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(4.))
            .rounded(px(5.))
            .cursor_pointer()
            .hover(|style| style.bg(gpui::transparent_white()))
            .child(
                div()
                    .w(px(40.))
                    .h(px(5.))
                    .rounded(px(3.))
                    .bg(hsla(chrome.track))
                    .child(div().w(px(filled)).h_full().rounded(px(3.)).bg(hsla(color))),
            )
            .child(
                div()
                    .text_color(hsla(chrome.muted))
                    .child(header::context_words(used)),
            )
            .tooltip(|window, cx| Tooltip::new("Token details").build(window, cx))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.details_open = !this.details_open;
                cx.notify();
            }))
            .into_any_element()
    }

    /// The token details under the header's right end (D4).
    fn render_details_popover(&self, session: &SessionInfo, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let details: SessionDetails = self.details.get(&session.id).cloned().unwrap_or_default();
        let row = |label: &str, value: String| {
            div()
                .flex()
                .justify_between()
                .gap(px(16.))
                .child(div().text_color(hsla(chrome.muted)).child(label.to_owned()))
                .child(div().text_color(hsla(chrome.text)).child(value))
        };
        let mut panel = div()
            .id("details-popover")
            .absolute()
            .top(px(HEADER_HEIGHT + 4.))
            .right(px(10.))
            .w(px(280.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .p(px(12.))
            .rounded(px(10.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.sidebar))
            .shadow_lg()
            .text_size(px(12.5))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.details_open = false;
                cx.notify();
            }));
        if let Some(tokens) = &details.tokens {
            for (label, value) in [
                ("Input tokens", tokens.input),
                ("Output tokens", tokens.output),
                ("Cache read", tokens.cache_read),
                ("Cache write", tokens.cache_write),
            ] {
                if let Some(value) = value {
                    panel = panel.child(row(label, header::tokens(value)));
                }
            }
        }
        if let Some(window_size) = details.context_window {
            panel = panel.child(row("Context window", header::tokens(window_size)));
        }
        let started =
            chrono::DateTime::from_timestamp_millis(i64::try_from(session.created_at).unwrap_or(0))
                .map(|at| {
                    at.with_timezone(&chrono::Local)
                        .format("%b %-d, %-I:%M %p")
                        .to_string()
                });
        if let Some(started) = started {
            panel = panel.child(row("Started", started));
        }
        if let Some(conversation) = session.agent_session_id.clone() {
            let shown = header::middle(&conversation, 18);
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_color(hsla(chrome.muted)).child("Session id"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().text_color(hsla(chrome.text)).child(shown))
                            .child(
                                Button::new("copy-session-id")
                                    .ghost()
                                    .xsmall()
                                    .label("Copy")
                                    .on_click(move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            conversation.clone(),
                                        ));
                                    }),
                            ),
                    ),
            );
        }
        panel
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .into_any_element()
    }
}
