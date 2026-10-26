//! The terminal pane: model, rendering and input for one attached session.

pub mod copy;
pub mod frame;
pub mod keys;
pub mod links;
pub mod model;
pub mod session;
pub mod view;

use gpui::{App, KeyBinding, actions};

actions!(terminal, [SendTab, SendShiftTab]);

/// The key context the terminal pane sets; its bindings outrank the window's.
pub const CONTEXT: &str = "Terminal";

/// Binds the keys the window root takes for moving focus (Tab, Shift-Tab) to the
/// terminal too: GPUI runs bindings before key handlers, so without these the pane never
/// sees Shift-Tab, which agents use to cycle modes. The deeper context wins.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", SendTab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", SendShiftTab, Some(CONTEXT)),
    ]);
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::{
        AppContext, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
        Styled, TestAppContext, Window, div, px,
    };

    use super::*;

    /// The terminal pane's key context and Shift-Tab action beside a focusable sidebar row,
    /// inside the window root the app uses.
    struct Pane {
        terminal: FocusHandle,
        row: FocusHandle,
        sent: Rc<Cell<usize>>,
    }

    impl Render for Pane {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let sent = self.sent.clone();
            div()
                .size_full()
                .child(
                    div()
                        .id("row")
                        .track_focus(&self.row)
                        .tab_index(0)
                        .size(px(10.)),
                )
                .child(
                    div()
                        .id("terminal")
                        .track_focus(&self.terminal)
                        .key_context(CONTEXT)
                        .on_action(
                            cx.listener(move |_, _: &SendShiftTab, _, _| sent.set(sent.get() + 1)),
                        )
                        .size(px(10.)),
                )
        }
    }

    #[gpui::test]
    fn shift_tab_reaches_the_terminal_instead_of_moving_focus(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            bind_keys(cx);
        });
        let (terminal, row) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle().tab_stop(true)));
        let sent = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let terminal = terminal.clone();
            let row = row.clone();
            let sent = sent.clone();
            move |window, cx| {
                let pane = cx.new(|_| Pane {
                    terminal,
                    row,
                    sent,
                });
                gpui_component::Root::new(pane, window, cx)
            }
        });
        cx.update(|window, cx| {
            window.activate_window();
            terminal.focus(window, cx);
            window.draw(cx).clear(cx);
        });

        cx.simulate_keystrokes("shift-tab");

        assert_eq!(sent.get(), 1);
        cx.update(|window, cx| assert_eq!(window.focused(cx), Some(terminal.clone())));
    }
}
