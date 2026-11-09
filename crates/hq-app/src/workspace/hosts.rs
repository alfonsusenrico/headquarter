//! The Hosts menu and Connect to a Host (add-merged-host-list D6): the sidebar header lists
//! this computer and the paired hosts with their state, and Forget asks in place; Connect to
//! a Host pairs with a host on another computer from the code it shows.

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
use crate::hosts::{self, HostId};
use crate::terminal::view::hsla;

/// Buttons shown together share this width (owner, 2026-10-26).
const BUTTON_WIDTH: f32 = 132.0;
/// Cancel and Forget in the menu's narrower row, equal to each other.
const FORGET_WIDTH: f32 = 96.0;

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

/// A host's state as the Hosts menu says it.
fn state_word(workspace: &Workspace, host: &HostId) -> &'static str {
    if workspace.hosts.failed.contains_key(host) {
        "not reachable"
    } else if workspace.hosts.away.contains_key(host) {
        "reconnecting"
    } else if workspace.hosts.listed.contains(host) {
        "connected"
    } else {
        "connecting"
    }
}

impl Workspace {
    /// Forget on a paired host's row: the first click asks in place, the second forgets
    /// (add-merged-host-list D6).
    pub(super) fn forget_host(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.hosts.forget_asked.as_deref() != Some(id.as_str()) {
            self.hosts.forget_asked = Some(id);
            cx.notify();
            return;
        }
        self.hosts.forget_asked = None;
        hosts::forget(&id);
        self.drop_host(&HostId::Remote(id), window, cx);
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
                    this.add_host(HostId::Remote(host.id.clone()), window, cx);
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

    /// The Hosts menu at the top of the sidebar (D6).
    pub(super) fn render_host_switcher(&self, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let known = hosts::known();
        let row = |id: SharedString| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(6.))
        };
        let state = |host: &HostId| {
            div()
                .flex_none()
                .text_size(px(12.))
                .text_color(muted)
                .child(state_word(self, host))
        };
        let mut list = div().w(px(300.)).p(px(4.)).flex().flex_col();
        if hosts::has_local_host() {
            list = list.child(
                row("host-local".into())
                    .child(div().flex_1().child(hosts::this_computer()))
                    .child(state(&HostId::Local)),
            );
        }
        for (index, host) in known.into_iter().enumerate() {
            let id = HostId::Remote(host.id.clone());
            let forget = host.id.clone();
            if self.hosts.forget_asked.as_deref() == Some(host.id.as_str()) {
                let keep = host.id.clone();
                list = list.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .px(px(10.))
                        .py(px(8.))
                        .rounded(px(6.))
                        .bg(hsla(chrome.selected))
                        .child(format!("Forget {}?", host.name))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(muted)
                                .child("You'll need a new code from it to connect again."),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(
                                    Button::new(SharedString::from(format!(
                                        "forget-cancel-{index}"
                                    )))
                                    .small()
                                    .w(px(FORGET_WIDTH))
                                    .label("Cancel")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.hosts.forget_asked.as_deref() == Some(keep.as_str())
                                        {
                                            this.hosts.forget_asked = None;
                                        }
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    Button::new(SharedString::from(format!(
                                        "forget-confirm-{index}"
                                    )))
                                    .small()
                                    .danger()
                                    .w(px(FORGET_WIDTH))
                                    .label("Forget")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.forget_host(forget.clone(), window, cx);
                                    })),
                                ),
                        ),
                );
                continue;
            }
            list = list.child(
                row(SharedString::from(format!("host-remote-{index}")))
                    .child(div().flex_1().min_w_0().truncate().child(host.name.clone()))
                    .child(state(&id))
                    .child(
                        Button::new(SharedString::from(format!("host-forget-{index}")))
                            .ghost()
                            .xsmall()
                            .label("Forget")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.forget_host(forget.clone(), window, cx);
                            })),
                    ),
            );
        }
        list = list.child(
            row("host-connect".into())
                .cursor_pointer()
                .hover(|style| style.bg(hsla(chrome.selected)))
                .border_t_1()
                .border_color(hsla(chrome.sidebar_line))
                .child(div().flex_1().text_color(muted).child("Connect to a Host…"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_connect_window(window, cx);
                })),
        );
        let count = hosts::all().len();
        let name = if count == 1 {
            "1 host".to_owned()
        } else {
            format!("{count} hosts")
        };
        let trigger = Button::new("host-switcher").ghost().small().w_full().child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(Icon::new(IconName::Globe).small().text_color(muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_left()
                        .child("Hosts"),
                )
                .child(div().text_color(muted).child(name))
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
                            // Closing the menu (Escape, a click elsewhere) cancels a Forget
                            // that waits for its second click.
                            if !open {
                                this.hosts.forget_asked = None;
                            }
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
