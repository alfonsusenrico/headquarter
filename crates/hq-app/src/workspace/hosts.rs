//! The host switcher and Connect to a Host (add-remote-desktop-client D6): the sidebar
//! header names the host on screen and lists this computer and the paired hosts; Connect
//! to a Host pairs with a host on another computer from the code it shows.

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::popover::Popover;
use gpui_component::{Disableable, Icon, IconName, Sizable};

use super::{Pane, Workspace, overlay};
use crate::hosts::{self, Shown};
use crate::terminal::view::hsla;

/// Buttons shown together share this width (owner, 2026-10-26).
const BUTTON_WIDTH: f32 = 132.0;

pub(super) struct ConnectWindow {
    focus: FocusHandle,
    input: Entity<InputState>,
    state: Connecting,
    _events: Subscription,
}

#[derive(Clone, PartialEq, Eq)]
enum Connecting {
    Idle,
    Pairing,
    Failed(String),
}

/// What this computer is called in the switcher.
pub(super) fn this_computer() -> &'static str {
    if cfg!(target_os = "macos") {
        "This Mac"
    } else {
        "This computer"
    }
}

impl Workspace {
    /// The name of the host on screen.
    pub(super) fn host_name_shown(&self) -> String {
        hosts::shown_name().unwrap_or_else(|| this_computer().to_owned())
    }

    /// Shows `shown` instead of the host on screen: the terminals close and the sidebar
    /// lists the chosen host's sessions.
    pub(super) fn switch_host(
        &mut self,
        shown: Shown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.host_menu_open = false;
        if hosts::shown() == shown {
            cx.notify();
            return;
        }
        hosts::show(shown);
        self.restart_feed(window, cx);
    }

    pub(super) fn forget_host(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let was_shown = hosts::shown() == Shown::Remote(id.clone());
        hosts::forget(&id);
        if was_shown {
            self.restart_feed(window, cx);
        }
        cx.notify();
    }

    pub(super) fn open_connect_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.host_menu_open = false;
        if self.connect_window.is_some() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("hqp2|…"));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.connect_with_code(window, cx);
            }
        });
        window.focus(&input.focus_handle(cx), cx);
        self.connect_window = Some(ConnectWindow {
            focus: cx.focus_handle(),
            input,
            state: Connecting::Idle,
            _events: events,
        });
        cx.notify();
    }

    fn close_connect_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .connect_window
            .as_ref()
            .is_some_and(|connect| connect.state == Connecting::Pairing)
        {
            return;
        }
        self.connect_window = None;
        match &self.pane {
            Pane::Terminal { view, .. } => window.focus(&view.focus_handle(cx), cx),
            Pane::Details(_) => window.focus(&self.pane_focus, cx),
            Pane::Empty => window.focus(&self.sidebar_focus, cx),
        }
        cx.notify();
    }

    fn connect_with_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(connect) = self.connect_window.as_mut() else {
            return;
        };
        if connect.state == Connecting::Pairing {
            return;
        }
        let code = connect.input.read(cx).value().trim().to_owned();
        if code.is_empty() {
            connect.state = Connecting::Failed(
                "Paste the code the host shows: Remote Control on a Mac, or `hq remote pair` on a server.".into(),
            );
            cx.notify();
            return;
        }
        connect.state = Connecting::Pairing;
        cx.notify();
        let task = cx
            .background_executor()
            .spawn(async move { hosts::pair(&code) });
        cx.spawn_in(window, async move |this, cx| {
            let paired = task.await;
            let _ = this.update_in(cx, |this, window, cx| match paired {
                Ok(host) => {
                    if let Some(connect) = this.connect_window.as_mut() {
                        connect.state = Connecting::Idle;
                    }
                    this.close_connect_window(window, cx);
                    this.restart_feed(window, cx);
                    this.show_notice(format!("Paired with {}.", host.name), false, window, cx);
                }
                Err(error) => {
                    if let Some(connect) = this.connect_window.as_mut() {
                        connect.state = Connecting::Failed(error.to_string());
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The switcher at the top of the sidebar: the host on screen, opening the host list.
    pub(super) fn render_host_switcher(&self, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let shown = hosts::shown();
        let known = hosts::known();
        let row = |id: &'static str| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(6.))
                .cursor_pointer()
                .hover(|style| style.bg(hsla(chrome.selected)))
        };
        let tick = |on: bool| {
            div()
                .flex_none()
                .w(px(12.))
                .text_color(hsla(chrome.accent))
                .child(if on { "✓" } else { "" })
        };
        let mut list = div().w(px(260.)).p(px(4.)).flex().flex_col();
        if hosts::has_local_host() {
            list = list.child(
                row("host-local")
                    .child(tick(shown == Shown::Local))
                    .child(div().flex_1().child(this_computer()))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.switch_host(Shown::Local, window, cx);
                    })),
            );
        }
        for (index, host) in known.into_iter().enumerate() {
            let on = shown == Shown::Remote(host.id.clone());
            let choose = host.id.clone();
            let forget = host.id.clone();
            list = list.child(
                div()
                    .id(SharedString::from(format!("host-remote-{index}")))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|style| style.bg(hsla(chrome.selected)))
                    .child(tick(on))
                    .child(div().flex_1().min_w_0().truncate().child(host.name.clone()))
                    .child(
                        Button::new(SharedString::from(format!("host-forget-{index}")))
                            .ghost()
                            .xsmall()
                            .label("Forget")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.forget_host(forget.clone(), window, cx);
                            })),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.switch_host(Shown::Remote(choose.clone()), window, cx);
                    })),
            );
        }
        list = list.child(
            row("host-connect")
                .border_t_1()
                .border_color(hsla(chrome.sidebar_line))
                .child(tick(false))
                .child(div().flex_1().text_color(muted).child("Connect to a Host…"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_connect_window(window, cx);
                })),
        );
        let name = self.host_name_shown();
        let trigger = Button::new("host-switcher").ghost().small().w_full().child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(Icon::new(IconName::Globe).small().text_color(muted))
                .child(div().flex_1().min_w_0().truncate().text_left().child(name))
                .child(Icon::new(IconName::ChevronDown).small().text_color(muted)),
        );
        let this = cx.entity().downgrade();
        div()
            .flex_none()
            .pb(px(8.))
            .child(
                Popover::new("host-list")
                    .open(self.host_menu_open)
                    .on_open_change(move |open, _, cx| {
                        let open = *open;
                        let _ = this.update(cx, |this, cx| {
                            this.host_menu_open = open;
                            cx.notify();
                        });
                    })
                    .trigger(trigger)
                    .child(list),
            )
            .into_any_element()
    }

    pub(super) fn render_connect_window(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let connect = self.connect_window.as_ref()?;
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let pairing = connect.state == Connecting::Pairing;
        let note = match &connect.state {
            Connecting::Idle => None,
            Connecting::Pairing => Some(div().text_color(muted).child("Pairing…")),
            Connecting::Failed(reason) => {
                Some(div().text_color(hsla(chrome.amber)).child(reason.clone()))
            }
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Connect to a Host"),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(muted)
                            .child("Control the sessions on another computer"),
                    ),
            )
            .child(div().text_size(px(13.)).text_color(muted).child(
                "On the host, show a pairing code: Remote Control ▸ Pair a Phone or Computer on a Mac, \
                 or `hq remote pair` on a server. Paste it here. The code works once, for 2 minutes.",
            ))
            .child(Input::new(&connect.input))
            .children(note)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("connect-cancel")
                            .w(px(BUTTON_WIDTH))
                            .label("Cancel")
                            .disabled(pairing)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_connect_window(window, cx);
                            })),
                    )
                    .child(
                        Button::new("connect-pair")
                            .primary()
                            .w(px(BUTTON_WIDTH))
                            .label("Connect")
                            .disabled(pairing)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.connect_with_code(window, cx);
                            })),
                    ),
            );
        Some(
            div()
                .id("connect-backdrop")
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
                    cx.listener(|this, _, window, cx| this.close_connect_window(window, cx)),
                )
                .child(
                    div()
                        .id("connect-window")
                        .track_focus(&connect.focus)
                        .w(px(520.))
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
                                this.close_connect_window(window, cx);
                            }
                        }))
                        .when(pairing, |window| window.opacity(0.95))
                        .child(body),
                )
                .into_any_element(),
        )
    }
}
