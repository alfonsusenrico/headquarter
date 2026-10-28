//! The shell panel under a session's agent (add-session-shell-panel): Cmd-T opens the
//! session's shell, which the host runs in the session's folder, and closes it again.
//! The host keeps a session's shell while the owner looks elsewhere; this side keeps
//! only which sessions have their panel open, and attaches to the shown one's shell.

use gpui::{
    AnyElement, AppContext, Context, CursorStyle, DragMoveEvent, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled,
    Subscription, Window, div, px, relative,
};

use super::{Pane, Workspace};
use crate::app_state;
use crate::terminal::session::TerminalSession;
use crate::terminal::view::{TerminalView, hsla};

/// Each part of a split pane keeps at least this many pixels: about six rows.
const MIN_PART: f32 = 110.0;

/// The shown session's shell, attached while its panel is open.
pub(super) struct ShellPanel {
    session: String,
    view: Entity<TerminalView>,
    _ended: Subscription,
    _focus: Subscription,
    _holder: Subscription,
}

/// The divider between the agent and the shell, while it is dragged.
pub struct ShellDividerDrag;

impl Render for ShellDividerDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The shell's share of a pane `height` pixels tall when the divider is `from_bottom`
/// pixels above its bottom, keeping both parts at least `MIN_PART` tall.
pub(super) fn shell_share(from_bottom: f32, height: f32) -> f32 {
    if height <= 2.0 * MIN_PART {
        return app_state::SHELL_SHARE;
    }
    let low = MIN_PART / height;
    (from_bottom / height).clamp(low, 1.0 - low)
}

impl Workspace {
    /// Cmd-T: opens the shown session's shell panel, or closes it and ends its shell.
    pub(super) fn toggle_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Pane::Terminal { id, .. } = &self.pane else {
            return;
        };
        let id = id.clone();
        if self.shell_panels.contains(&id) {
            self.close_shell(&id, true, window, cx);
        } else {
            self.shell_panels.push(id);
            self.save_shell_panels();
            self.sync_shell(window, cx);
            if let Some(shell) = &self.shell {
                window.focus(&shell.view.focus_handle(cx), cx);
            }
        }
        cx.notify();
    }

    /// Attaches to the shown session's shell when its panel is open, and lets go of
    /// any other; a shell let go of keeps running in the host.
    pub(super) fn sync_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = match &self.pane {
            Pane::Terminal { id, .. } if self.shell_panels.contains(id) => Some(id.clone()),
            _ => None,
        };
        if self.shell.as_ref().map(|shell| &shell.session) == wanted.as_ref() {
            return;
        }
        self.shell = None;
        let Some(id) = wanted else {
            return;
        };
        let Some(info) = self.session(&id).cloned() else {
            return;
        };
        let palette = self.theme.palette.clone();
        let terminal = cx.new(|cx| TerminalSession::attach_shell(info, palette, cx));
        let options = self.options.clone();
        let view = cx.new(|cx| TerminalView::new(terminal.clone(), options, window, cx));
        let holder = self.watch_holder(&terminal, &view, window, cx);
        let session = id.clone();
        // The shell exited, or could not start: the panel closes.
        let ended = cx.observe_in(&terminal, window, move |this, terminal, window, cx| {
            let Some(reason) = terminal.read(cx).ended.clone() else {
                return;
            };
            if !reason.starts_with("The shell exited") {
                this.message = Some(format!("Shell: {reason}"));
            }
            let session = id.clone();
            this.close_shell(&session, false, window, cx);
        });
        // Focus in the shell is watching the session too.
        let focus = cx.on_focus(&view.focus_handle(cx), window, |this, window, cx| {
            this.mark_seen_if_watched(window, cx);
            this.tell_host_in_use(window, cx);
        });
        self.shell = Some(ShellPanel {
            session,
            view,
            _ended: ended,
            _focus: focus,
            _holder: holder,
        });
    }

    /// Closes a session's panel; `end` also ends its shell in the host.
    pub(super) fn close_shell(
        &mut self,
        session: &str,
        end: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shell_panels.retain(|id| id != session);
        self.save_shell_panels();
        if self
            .shell
            .as_ref()
            .is_some_and(|shell| shell.session == session)
        {
            self.shell = None;
            if let Pane::Terminal { view, .. } = &self.pane {
                window.focus(&view.focus_handle(cx), cx);
            }
        }
        if end {
            let session = session.to_owned();
            self.host_call(
                window,
                cx,
                move |paths| hq_client::close_shell(paths, &session),
                |_, (), _, _| {},
            );
        }
        cx.notify();
    }

    fn save_shell_panels(&self) {
        if let Some(path) = app_state::path() {
            // A lost write only means a panel is not reopened after a restart.
            let _ = app_state::save_shell_panels(&path, &self.shell_panels);
        }
    }

    /// The shell's terminal view, when the shown session's panel is open.
    pub(super) fn shell_view(&self) -> Option<&Entity<TerminalView>> {
        self.shell.as_ref().map(|shell| &shell.view)
    }

    /// The pane split between the agent above and the shell below, with the divider.
    pub(super) fn render_split(
        &self,
        agent: AnyElement,
        shell: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let share = self.shell_share;
        let line = hsla(self.theme.chrome.sidebar_line);
        div()
            .id("shell-split")
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .h(relative(1.0 - share))
                    .min_h(px(MIN_PART))
                    .child(agent),
            )
            .child(
                div()
                    .id("shell-divider")
                    .w_full()
                    .h(px(7.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .cursor(CursorStyle::ResizeUpDown)
                    .child(div().w_full().h(px(1.)).bg(line))
                    .on_drag(ShellDividerDrag, |_, _, _, cx| cx.new(|_| ShellDividerDrag)),
            )
            .child(div().w_full().flex_1().min_h(px(MIN_PART)).child(shell))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<ShellDividerDrag>, _, cx| {
                    let bounds = event.bounds;
                    let from_bottom = f32::from(bounds.bottom() - event.event.position.y);
                    this.shell_share = shell_share(from_bottom, f32::from(bounds.size.height));
                    cx.notify();
                }),
            )
            .on_drop(cx.listener(|this, _: &ShellDividerDrag, _, _| {
                if let Some(path) = app_state::path() {
                    // A lost write only means the next panel opens at the old size.
                    let _ = app_state::save_shell_share(&path, this.shell_share);
                }
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_divider_keeps_both_parts_usable() {
        assert!((shell_share(300.0, 900.0) - 1.0 / 3.0).abs() < 1e-6);
        assert!(
            (shell_share(5.0, 900.0) - MIN_PART / 900.0).abs() < 1e-6,
            "shell kept"
        );
        assert!(
            (shell_share(895.0, 900.0) - (1.0 - MIN_PART / 900.0)).abs() < 1e-6,
            "agent kept"
        );
        assert_eq!(
            shell_share(100.0, 150.0),
            app_state::SHELL_SHARE,
            "too small to split"
        );
    }
}
