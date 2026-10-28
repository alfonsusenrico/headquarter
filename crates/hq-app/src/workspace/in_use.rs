//! A session another client has: the pane shows a page in its terminal's place, laid out
//! as for an inactive session, until the owner takes it back (refine-in-use-elsewhere
//! D1, D3). The terminal stays attached underneath, so it is current when it returns.

use gpui::{
    AnyElement, Context, Entity, Focusable, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, SharedString, StyleRefinement, Styled, Subscription, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};

use super::{PANE_TEXT_WIDTH, Workspace, translucent};
use crate::terminal::session::{TerminalSession, in_use_page};
use crate::terminal::view::{TerminalView, hsla};
use crate::themes::Token;

impl Workspace {
    /// Redraws the pane when another client takes the session or gives it back, and moves
    /// the keyboard with it: to the page, so Enter takes it back, and to the terminal
    /// again. Output alone does not redraw the workspace.
    pub(super) fn watch_holder(
        &self,
        session: &Entity<TerminalSession>,
        view: &Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        let view = view.clone();
        let mut elsewhere = false;
        cx.observe_in(session, window, move |this, session, window, cx| {
            let session = session.read(cx);
            let now = in_use_page(session.holder.as_ref(), session.shell).is_some();
            if now == elsewhere {
                return;
            }
            elsewhere = now;
            if now {
                if view.focus_handle(cx).is_focused(window) {
                    window.focus(&this.pane_focus, cx);
                }
            } else if this.pane_focus.is_focused(window) {
                window.focus(&view.focus_handle(cx), cx);
            }
            cx.notify();
        })
    }

    /// The terminal, or the page in its place while another client has its session.
    pub(super) fn terminal_part(
        &self,
        view: &Entity<TerminalView>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let session = view.read(cx).session().read(cx);
        let Some((heading, note)) = in_use_page(session.holder.as_ref(), session.shell) else {
            // Cached, so a sidebar frame (a breathing word) does not repaint the terminal.
            return view
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element();
        };
        let agent = session.info.agent;
        let chrome = self.theme.chrome;
        let background = translucent(
            Token::Background.get(&self.theme),
            self.look_appearance.opacity,
        );
        let take_back = {
            let view = view.clone();
            move |_: &mut Workspace, cx: &mut Context<Workspace>| {
                view.read(cx).session().read(cx).focus();
            }
        };
        let on_click = take_back.clone();
        div()
            .id(SharedString::from(format!("in-use-{}", view.entity_id())))
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .px(px(24.))
            .bg(background)
            .track_focus(&self.pane_focus)
            // Enter takes the session back; other keys never reach the agent from here.
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" {
                    cx.stop_propagation();
                    take_back(this, cx);
                }
            }))
            .child(self.marks.element(agent, 28., chrome.muted))
            .child(
                div()
                    .w_full()
                    .max_w(px(PANE_TEXT_WIDTH))
                    .text_center()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(hsla(chrome.text))
                    .child(heading),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(PANE_TEXT_WIDTH))
                    .text_center()
                    .text_size(px(14.))
                    .text_color(hsla(chrome.muted))
                    .child(note),
            )
            .child(
                Button::new(SharedString::from(format!("use-here-{}", view.entity_id())))
                    .primary()
                    .label("Use here")
                    .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx))),
            )
            .into_any_element()
    }
}
