//! The Remote Control window (add-remote-gateway D12): Headquarter ▸ Remote Control…
//! opens it over the session. Getting the phone app (add-phone-app-download), a switch for
//! remote control, how the computer is reached, Pair a Phone with the code as an image QR
//! code, and the paired phones with Remove.

use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Bounds, ClipboardItem, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    Task, Window, canvas, div, fill, point, px, size,
};
use gpui_component::button::{Button, ButtonVariants};
use hq_client::HostPaths;
use hq_protocol::{RemoteDevice, Request, Response};

use super::{Pane, Workspace, overlay};
use crate::terminal::view::hsla;

/// Buttons shown together share this width (owner, 2026-10-26).
const BUTTON_WIDTH: f32 = 132.0;
/// How often the window reads the status and the phones while open.
const REFRESH: Duration = Duration::from_secs(5);
/// The QR code's drawn size, quiet zone included.
const QR_SIDE: f32 = 224.0;
/// The phone app's QR code is smaller: a phone's camera reads a link from further away than
/// it needs to for a pairing code (add-phone-app-download D2).
const APP_QR_SIDE: f32 = 132.0;
/// How long Copy link reads "Copied".
const COPIED_FOR: Duration = Duration::from_secs(2);

pub(super) struct RemoteWindow {
    focus: FocusHandle,
    status: Option<Status>,
    devices: Vec<RemoteDevice>,
    pairing: Pairing,
    problem: Option<String>,
    /// The phone app link's QR code: its modules, row by row, and its width.
    app_qr: Option<(Vec<bool>, usize)>,
    /// While set, Copy link reads "Copied"; it clears itself.
    copied: Option<Task<()>>,
    _refresh: Task<()>,
}

/// A QR code's modules, row by row, true for dark, and its width.
fn qr_modules(text: &str) -> Option<(Vec<bool>, usize)> {
    let qr = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let width = qr.width();
    let modules = qr
        .to_colors()
        .into_iter()
        .map(|color| color == qrcode::Color::Dark)
        .collect();
    Some((modules, width))
}

struct Status {
    running: bool,
    relay: Option<String>,
    problem: Option<String>,
}

enum Pairing {
    Idle,
    Asking,
    Showing {
        code: String,
        /// The QR code's modules, row by row, true for dark.
        modules: Vec<bool>,
        width: usize,
        until: Instant,
        /// The connection that holds the code; closing it withdraws the code.
        holder: UnixStream,
        _countdown: Task<()>,
    },
    Paired(String),
    Ended(String),
}

impl Pairing {
    fn withdraw(&self) {
        if let Pairing::Showing { holder, .. } = self {
            let _ = holder.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl Workspace {
    pub(super) fn open_remote_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.remote_window.is_some() {
            return;
        }
        let refresh = cx.spawn_in(window, async move |this, cx| {
            loop {
                let open = this
                    .update_in(cx, |this, window, cx| {
                        let open = this.remote_window.is_some();
                        if open {
                            this.refresh_remote(window, cx);
                        }
                        open
                    })
                    .unwrap_or(false);
                if !open {
                    break;
                }
                cx.background_executor().timer(REFRESH).await;
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.remote_window = Some(RemoteWindow {
            focus,
            status: None,
            devices: Vec::new(),
            pairing: Pairing::Idle,
            problem: None,
            app_qr: qr_modules(&crate::update::phone_app_link()),
            copied: None,
            _refresh: refresh,
        });
        cx.notify();
    }

    fn close_remote_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(remote) = self.remote_window.take() {
            remote.pairing.withdraw();
        }
        match &self.pane {
            Pane::Terminal { view, .. } => window.focus(&view.focus_handle(cx), cx),
            Pane::Details(_) => window.focus(&self.pane_focus, cx),
            Pane::Empty => window.focus(&self.sidebar_focus, cx),
        }
        cx.notify();
    }

    /// Runs `call` against the host off the main thread, then `done` with its result; a
    /// failure is shown in the window rather than under the sidebar it covers.
    fn remote_call<R: Send + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        call: impl FnOnce(&HostPaths) -> anyhow::Result<R> + Send + 'static,
        done: impl FnOnce(&mut Self, R, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let task = cx.background_executor().spawn(async move {
            let paths = HostPaths::standard()
                .ok_or_else(|| anyhow::anyhow!("cannot find the home folder"))?;
            call(&paths)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(answer) => done(this, answer, window, cx),
                    Err(error) => {
                        if let Some(remote) = this.remote_window.as_mut() {
                            remote.problem = Some(format!("{error:#}"));
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn refresh_remote(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.remote_call(
            window,
            cx,
            |paths| {
                let status = hq_client::remote(paths, &Request::RemoteStatus {})?;
                let devices = hq_client::remote(paths, &Request::RemoteDevices {})?;
                Ok((status, devices))
            },
            |this, (status, devices), _, cx| {
                let Some(remote) = this.remote_window.as_mut() else {
                    return;
                };
                if let Response::RemoteStatus {
                    running,
                    relay,
                    problem,
                    ..
                } = status
                {
                    remote.status = Some(Status {
                        running,
                        relay,
                        problem,
                    });
                }
                if let Response::RemoteDevices { devices } = devices {
                    remote.devices = devices;
                }
                cx.notify();
            },
        );
    }

    /// The switch: the settings file first, so it holds after a restart, then the host.
    fn pair_phone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(remote) = self.remote_window.as_mut() else {
            return;
        };
        remote.pairing = Pairing::Asking;
        cx.notify();
        let task = cx.background_executor().spawn(async move {
            let paths = HostPaths::standard()
                .ok_or_else(|| anyhow::anyhow!("cannot find the home folder"))?;
            hq_client::begin_pairing(&paths)
        });
        cx.spawn_in(window, async move |this, cx| {
            let begun = task.await;
            let _ = this.update_in(cx, |this, window, cx| match begun {
                Ok((client, code, secs)) => this.show_code(client, code, secs, window, cx),
                Err(error) => {
                    if let Some(remote) = this.remote_window.as_mut() {
                        remote.pairing = Pairing::Ended(format!("{error:#}"));
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn show_code(
        &mut self,
        mut client: hq_client::Client,
        code: String,
        secs: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(remote) = self.remote_window.as_mut() else {
            // Closed meanwhile: dropping the client withdraws the code.
            return;
        };
        let Some((modules, width)) = qr_modules(&code) else {
            remote.pairing = Pairing::Ended("cannot draw the QR code".into());
            return;
        };
        let Ok(holder) = client.writer.try_clone() else {
            remote.pairing = Pairing::Ended("lost the connection to the host".into());
            return;
        };
        // Hears whether a phone used the code, or why it ended.
        let outcome = cx.background_executor().spawn(async move {
            loop {
                match client.next_response() {
                    Ok(Response::RemotePairUsed { device }) => return Ok(device),
                    Ok(Response::RemotePairExpired { reason }) => return Err(reason),
                    Ok(_) => continue,
                    Err(_) => return Err(String::new()),
                }
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let ended = outcome.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(remote) = this.remote_window.as_mut() else {
                    return;
                };
                if !matches!(remote.pairing, Pairing::Showing { .. }) {
                    return;
                }
                remote.pairing = match ended {
                    Ok(device) => Pairing::Paired(device),
                    // Withdrawn by Cancel or Close: nothing to say.
                    Err(reason) if reason.is_empty() => Pairing::Idle,
                    Err(reason) => Pairing::Ended(reason),
                };
                this.refresh_remote(window, cx);
                cx.notify();
            });
        })
        .detach();
        let countdown = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let showing = this
                    .update(cx, |this, cx| {
                        cx.notify();
                        this.remote_window
                            .as_ref()
                            .is_some_and(|remote| matches!(remote.pairing, Pairing::Showing { .. }))
                    })
                    .unwrap_or(false);
                if !showing {
                    break;
                }
            }
        });
        remote.pairing = Pairing::Showing {
            code,
            modules,
            width,
            until: Instant::now() + Duration::from_secs(secs),
            holder,
            _countdown: countdown,
        };
        cx.notify();
    }

    fn cancel_pairing(&mut self, cx: &mut Context<Self>) {
        if let Some(remote) = self.remote_window.as_mut() {
            remote.pairing.withdraw();
            remote.pairing = Pairing::Idle;
        }
        cx.notify();
    }

    fn remove_phone(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.remote_call(
            window,
            cx,
            move |paths| hq_client::remote(paths, &Request::RemoteRemove { device: id }),
            |this, _, window, cx| this.refresh_remote(window, cx),
        );
    }

    /// Puts the phone app link on the clipboard; the button reads "Copied" for a moment.
    fn copy_phone_app_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(crate::update::phone_app_link()));
        let Some(remote) = self.remote_window.as_mut() else {
            return;
        };
        remote.copied = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(remote) = this.remote_window.as_mut() {
                    remote.copied = None;
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    pub(super) fn render_remote_window(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let remote = self.remote_window.as_ref()?;
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let card = || {
            div()
                .flex()
                .flex_col()
                .rounded(px(12.))
                .border_1()
                .border_color(hsla(chrome.sidebar_line))
                .bg(hsla(chrome.selected))
        };
        let item = || {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(14.))
                .py(px(12.))
        };
        let heading = |text: &'static str| {
            div()
                .text_size(px(12.))
                .text_color(muted)
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(text)
        };
        let lines = |title: String, detail: String| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().font_weight(gpui::FontWeight::MEDIUM).child(title))
                .child(div().text_size(px(12.5)).text_color(muted).child(detail))
        };
        let dot = |on: bool| {
            div()
                .flex_none()
                .size(px(8.))
                .rounded(px(999.))
                .bg(hsla(if on { chrome.accent } else { chrome.track }))
        };

        let running = remote.status.as_ref().is_some_and(|status| status.running);
        let (title, detail) = match &remote.status {
            None => ("Remote control".to_owned(), "Reading…".to_owned()),
            Some(status) if status.running => (
                "Remote control is on".to_owned(),
                match &status.relay {
                    Some(relay) => format!("Reachable through {relay}"),
                    None => {
                        "Connecting to a relay; phones on this network can already pair.".to_owned()
                    }
                },
            ),
            // Remote control follows the paired phones, with no switch
            // (remote-follows-paired-phones D4).
            Some(status) => (
                "Remote control is off".to_owned(),
                status.problem.clone().unwrap_or_else(|| {
                    "Pair a phone to turn it on. Nothing is reachable from outside until then."
                        .to_owned()
                }),
            ),
        };
        let status_card = card().child(item().child(dot(running)).child(lines(title, detail)));

        let pairing = match &remote.pairing {
            Pairing::Idle | Pairing::Ended(_) => {
                let detail = match &remote.pairing {
                    Pairing::Ended(reason) => format!("No phone paired: {reason}"),
                    _ => "Shows a QR code for Headquarter on your phone; it works once, for 2 minutes."
                        .to_owned(),
                };
                card().child(
                    item().child(lines("Pair a phone".into(), detail)).child(
                        Button::new("remote-pair")
                            .primary()
                            .w(px(BUTTON_WIDTH))
                            .label("Pair a Phone")
                            .on_click(cx.listener(|this, _, window, cx| this.pair_phone(window, cx))),
                    ),
                )
            }
            Pairing::Asking => card().child(item().child(lines(
                "Pair a phone".into(),
                "Asking the host for a code…".into(),
            ))),
            Pairing::Showing {
                code,
                modules,
                width,
                until,
                ..
            } => {
                let left = until.saturating_duration_since(Instant::now()).as_secs();
                card().child(
                    div()
                        .flex()
                        .gap(px(20.))
                        .p(px(16.))
                        .child(qr_code(modules.clone(), *width, QR_SIDE))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Scan with Headquarter on your phone"),
                                )
                                .child(
                                    div()
                                        .text_color(muted)
                                        .child("The code works once. It expires in"),
                                )
                                .child(
                                    div()
                                        .text_size(px(26.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(format!("{}:{:02}", left / 60, left % 60)),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(muted)
                                        .font_family(self.options.font_family.clone())
                                        .child(code.clone()),
                                )
                                .child(
                                    div().child(
                                        Button::new("remote-cancel")
                                            .w(px(BUTTON_WIDTH))
                                            .label("Cancel")
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.cancel_pairing(cx)),
                                            ),
                                    ),
                                ),
                        ),
                )
            }
            Pairing::Paired(device) => card().child(
                item()
                    .child(lines(
                        format!("✓ Paired {device}"),
                        "It can open your sessions and type into them, but not start, rename or close them."
                            .into(),
                    ))
                    .child(
                        Button::new("remote-paired")
                            .w(px(BUTTON_WIDTH))
                            .label("Done")
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_pairing(cx))),
                    ),
            ),
        };

        // Always open, so another phone can get the app at any time
        // (remote-follows-paired-phones D4).
        let app_step =
            {
                let copied = remote.copied.is_some();
                card().child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(20.))
                        .p(px(16.))
                        .children(
                            remote
                                .app_qr
                                .clone()
                                .map(|(modules, width)| qr_code(modules, width, APP_QR_SIDE)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Scan with your phone's camera"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .text_color(muted)
                                        .child("Headquarter for Android, Android 10 or later."),
                                )
                                .child(div().text_size(px(12.5)).text_color(muted).child(
                                    "Your phone asks once to let your browser install apps.",
                                ))
                                .child(
                                    div().pt(px(6.)).child(
                                        Button::new("remote-app-copy")
                                            .w(px(BUTTON_WIDTH))
                                            .label(if copied { "Copied" } else { "Copy link" })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.copy_phone_app_link(window, cx)
                                            })),
                                    ),
                                ),
                        ),
                )
            };

        let now = hq_signals::now_ms();
        let mut phones = card();
        if remote.devices.is_empty() {
            phones =
                phones.child(item().child(div().text_color(muted).child("No phones paired yet.")));
        }
        for (index, device) in remote.devices.iter().enumerate() {
            let seen = if device.connected {
                "Connected now".to_owned()
            } else {
                match device.last_seen_at {
                    Some(at) => format!(
                        "Seen {} ago",
                        hq_protocol::sidebar::age(now.saturating_sub(at.max(0) as u64))
                    ),
                    None => "Never connected".to_owned(),
                }
            };
            let id = device.id.clone();
            phones = phones.child(
                item()
                    .when(index > 0, |row| {
                        row.border_t_1().border_color(hsla(chrome.sidebar_line))
                    })
                    .child(dot(device.connected))
                    .child(lines(device.name.clone(), seen))
                    .child(
                        Button::new(gpui::SharedString::from(format!("remote-remove-{index}")))
                            .danger()
                            .w(px(BUTTON_WIDTH))
                            .label("Remove")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_phone(id.clone(), window, cx)
                            })),
                    ),
            );
        }

        let body = div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Remote Control"),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(muted)
                            .child("Control Headquarter from your phone"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(heading("GET THE PHONE APP"))
                    .child(app_step),
            )
            .child(status_card)
            .when(running, |body| {
                body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(heading("PAIR A PHONE"))
                        .child(pairing),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(heading("PAIRED PHONES"))
                    .child(phones),
            )
            .children(
                self.remote_window
                    .as_ref()
                    .and_then(|remote| remote.problem.clone())
                    .map(|problem| div().text_color(hsla(chrome.amber)).child(problem)),
            )
            .child(
                div().flex().justify_end().child(
                    Button::new("remote-close")
                        .w(px(BUTTON_WIDTH))
                        .label("Close")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.close_remote_window(window, cx)),
                        ),
                ),
            );

        Some(
            div()
                .id("remote-backdrop")
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(overlay(chrome))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| this.close_remote_window(window, cx)),
                )
                .child(
                    div()
                        .id("remote-window")
                        .track_focus(&remote.focus)
                        .w(px(580.))
                        .max_h(gpui::relative(0.92))
                        .overflow_y_scroll()
                        .p(px(20.))
                        .rounded(px(14.))
                        .border_1()
                        .border_color(hsla(chrome.sidebar_line))
                        .bg(hsla(chrome.sidebar))
                        .shadow_lg()
                        .text_size(px(14.))
                        .text_color(hsla(chrome.text))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                this.close_remote_window(window, cx);
                            }
                        }))
                        .child(body),
                )
                .into_any_element(),
        )
    }
}

/// The QR code as squares on white, with the standard 4-module quiet zone, `side` wide.
fn qr_code(modules: Vec<bool>, width: usize, side: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            window.paint_quad(fill(bounds, gpui::white()));
            let unit = f32::from(bounds.size.width) / (width + 8) as f32;
            for (index, dark) in modules.iter().enumerate() {
                if !dark {
                    continue;
                }
                let (x, y) = ((index % width + 4) as f32, (index / width + 4) as f32);
                window.paint_quad(fill(
                    Bounds::new(
                        point(
                            bounds.origin.x + px(x * unit),
                            bounds.origin.y + px(y * unit),
                        ),
                        size(px(unit.ceil()), px(unit.ceil())),
                    ),
                    gpui::black(),
                ));
            }
        },
    )
    .flex_none()
    .size(px(side))
    .rounded(px(10.))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phone_app_link_fits_a_small_qr_code() {
        let (modules, width) = qr_modules(&crate::update::phone_app_link()).unwrap();
        assert_eq!(modules.len(), width * width);
        assert!(
            width <= 41,
            "version 6 or lower reads well at 132 px: {width}"
        );
    }
}
