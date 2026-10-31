//! Quitting Headquarter (add-quit-stops-host): one question for ⌘Q, the menu's Quit and the
//! window's close button. Quit stops the host and its agents, Keep Running leaves them as
//! before, Cancel stays.

use std::time::{Duration, Instant};

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement,
    Styled, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use hq_client::HostPaths;

use super::{Workspace, overlay};
use crate::terminal::view::hsla;

/// How long Quit waits for the stopped host to let go of its socket.
const HOST_STOP_WAIT: Duration = Duration::from_secs(10);
/// The dialog's buttons share one width (equal-width button groups).
const BUTTON_WIDTH: f32 = 112.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum QuitState {
    Asking,
    Stopping,
    /// The host could not be stopped: why.
    Failed(String),
}

/// What the dialog says about the sessions Quit would stop.
pub(super) fn running_line(running: usize) -> String {
    match running {
        0 => "No sessions are running.".to_owned(),
        1 => "1 session is running. Quit stops it, and it resumes when you open Headquarter again."
            .to_owned(),
        n => format!(
            "{n} sessions are running. Quit stops them, and they resume when you open Headquarter again."
        ),
    }
}

/// Said only when remote control is on: Quit takes the phone's way in away too.
pub(super) const REMOTE_LINE: &str =
    "Your paired phone can't reach this computer until you open Headquarter again.";

impl Workspace {
    /// Asks how to quit, unless already asking or stopping.
    pub(super) fn request_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.quit.is_some() {
            return;
        }
        self.quit = Some(QuitState::Asking);
        self.quit_return = window.focused(cx);
        window.focus(&self.quit_focus, cx);
        // Whether a phone can reach the computer: remote control runs while one is paired
        // (remote-follows-paired-phones D1), so the host knows, not the settings file.
        self.remote_on = false;
        let status = cx.background_executor().spawn(async move {
            let paths = HostPaths::standard()?;
            std::os::unix::net::UnixStream::connect(&paths.control_socket).ok()?;
            match hq_client::remote(&paths, &hq_protocol::Request::RemoteStatus {}) {
                Ok(hq_protocol::Response::RemoteStatus { running, .. }) => Some(running),
                _ => None,
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let running = status.await.unwrap_or(false);
            let _ = this.update(cx, |this, cx| {
                this.remote_on = running;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Closes the dialog and gives focus back to what had it, so a window under the dialog
    /// answers its keys again (fix-quit-over-windows D2).
    fn cancel_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.quit != Some(QuitState::Stopping) {
            self.quit = None;
            if let Some(focus) = self.quit_return.take() {
                window.focus(&focus, cx);
            }
            cx.notify();
        }
    }

    /// The app quits, the host and its agents go on.
    fn keep_running(window: &mut Window, cx: &mut Context<Self>) {
        Self::save_window_now(window);
        cx.quit();
    }

    /// Stops the host, waits until it has let go of its socket, then quits.
    fn quit_and_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.quit == Some(QuitState::Stopping) {
            return;
        }
        self.quit = Some(QuitState::Stopping);
        cx.notify();
        let task = cx.background_executor().spawn(async move {
            let paths = HostPaths::standard().ok_or("cannot find the home folder")?;
            if std::os::unix::net::UnixStream::connect(&paths.control_socket).is_err() {
                return Ok(());
            }
            hq_client::stop_host(&paths).map_err(|error| format!("{error:#}"))?;
            let deadline = Instant::now() + HOST_STOP_WAIT;
            while std::os::unix::net::UnixStream::connect(&paths.control_socket).is_ok() {
                if Instant::now() >= deadline {
                    return Err("the host did not stop within 10 seconds".to_owned());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(())
        });
        cx.spawn_in(window, async move |this, cx| {
            let result: Result<(), String> = task.await;
            let _ = this.update_in(cx, |this, window, cx| match result {
                Ok(()) => {
                    Self::save_window_now(window);
                    cx.quit();
                }
                Err(reason) => {
                    this.quit = Some(QuitState::Failed(reason));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn render_quit(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = self.quit.clone()?;
        let chrome = self.theme.chrome;
        let running = self.sessions.iter().filter(|session| session.live).count();
        let remote = self.remote_on;
        let button = |id: &'static str, label: &'static str| {
            Button::new(id).label(label).w(px(BUTTON_WIDTH))
        };
        let (title, lines, buttons): (&str, Vec<String>, Vec<AnyElement>) = match &state {
            QuitState::Asking => {
                let mut lines = vec![running_line(running)];
                if remote {
                    lines.push(REMOTE_LINE.to_owned());
                }
                (
                    "Quit Headquarter?",
                    lines,
                    vec![
                        button("quit-cancel", "Cancel")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.cancel_quit(window, cx)),
                            )
                            .into_any_element(),
                        button("quit-keep", "Keep Running")
                            .on_click(
                                cx.listener(|_, _, window, cx| Self::keep_running(window, cx)),
                            )
                            .into_any_element(),
                        button("quit-stop", "Quit")
                            .primary()
                            .on_click(
                                cx.listener(|this, _, window, cx| this.quit_and_stop(window, cx)),
                            )
                            .into_any_element(),
                    ],
                )
            }
            QuitState::Stopping => (
                "Quitting…",
                vec!["Stopping sessions…".to_owned()],
                Vec::new(),
            ),
            QuitState::Failed(reason) => (
                "Couldn't stop the sessions",
                vec![
                    reason.clone(),
                    "Quit Anyway leaves them running in the background.".to_owned(),
                ],
                vec![
                    button("quit-cancel", "Cancel")
                        .on_click(cx.listener(|this, _, window, cx| this.cancel_quit(window, cx)))
                        .into_any_element(),
                    button("quit-anyway", "Quit Anyway")
                        .primary()
                        .on_click(cx.listener(|_, _, window, cx| Self::keep_running(window, cx)))
                        .into_any_element(),
                ],
            ),
        };
        Some(
            div()
                .id("quit-backdrop")
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(overlay(chrome))
                .track_focus(&self.quit_focus)
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => this.cancel_quit(window, cx),
                        "enter" if this.quit == Some(QuitState::Asking) => {
                            this.quit_and_stop(window, cx);
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .child(
                    div()
                        .id("quit-dialog")
                        .w(px(420.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .p(px(20.))
                        .rounded(px(14.))
                        .border_1()
                        .border_color(hsla(chrome.sidebar_line))
                        .bg(hsla(chrome.sidebar))
                        .shadow_lg()
                        .text_size(px(14.))
                        .text_color(hsla(chrome.text))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .children(
                            lines
                                .into_iter()
                                .map(|line| div().text_color(hsla(chrome.muted)).child(line)),
                        )
                        .child(div().flex().justify_end().gap(px(8.)).children(buttons)),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dialog_says_what_quit_stops() {
        assert_eq!(running_line(0), "No sessions are running.");
        assert!(running_line(1).starts_with("1 session is running. Quit stops it,"));
        assert!(running_line(3).starts_with("3 sessions are running. Quit stops them,"));
        assert!(!REMOTE_LINE.contains(';') && !running_line(3).contains(';'));
    }
}
