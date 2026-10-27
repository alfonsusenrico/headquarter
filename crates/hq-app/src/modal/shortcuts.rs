//! The Keyboard Shortcuts window (⌘/): every key by area, and typing filters them at once
//! (`openspec/changes/add-shortcuts-window/design.md`, D2).

use gpui::prelude::FluentBuilder;
use gpui::{
    App, Context, Div, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, StatefulInteractiveElement, Styled, Window,
    div, px,
};

use crate::shortcuts::{self, Area, Listing};
use crate::terminal::view::hsla;
use crate::theme::Chrome;

const WIDTH: f32 = 560.0;
const HEIGHT: f32 = 560.0;
/// The window shrinks to leave this much of the app around it.
const MARGIN: f32 = 24.0;
const KEYS_WIDTH: f32 = 190.0;
const AREA_WIDTH: f32 = 80.0;

pub enum ShortcutsEvent {
    Close,
}

pub struct ShortcutsWindow {
    rows: Vec<Listing>,
    mac: bool,
    /// What the owner typed; empty shows the rows by area.
    filter: String,
    chrome: Chrome,
    focus: FocusHandle,
}

impl EventEmitter<ShortcutsEvent> for ShortcutsWindow {}

impl ShortcutsWindow {
    pub fn new(chrome: Chrome, window: &mut Window, cx: &mut Context<Self>) -> ShortcutsWindow {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        ShortcutsWindow {
            rows: shortcuts::listings(shortcuts::MAC),
            mac: shortcuts::MAC,
            filter: String::new(),
            chrome,
            focus,
        }
    }

    pub fn set_chrome(&mut self, chrome: Chrome, cx: &mut Context<Self>) {
        self.chrome = chrome;
        cx.notify();
    }

    /// Typing filters; Escape clears the filter, then closes. Keys with the command
    /// modifier go on to the app, so ⌘/ closes the window and ⌘Q still quits.
    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.platform || modifiers.control {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" if !self.filter.is_empty() => self.filter.clear(),
            "escape" => {
                cx.emit(ShortcutsEvent::Close);
                cx.stop_propagation();
                return;
            }
            "backspace" => {
                self.filter.pop();
            }
            _ => match &event.keystroke.key_char {
                Some(typed) if !typed.chars().any(char::is_control) => {
                    self.filter.push_str(typed);
                }
                _ => return,
            },
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_caps(&self, row: &Listing) -> Div {
        let chrome = self.chrome;
        let cap = |text: String| {
            div()
                .min_w(px(22.))
                .h(px(22.))
                .px(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .border_1()
                .border_color(hsla(chrome.track))
                .bg(hsla(chrome.sidebar))
                .text_size(px(12.))
                .text_color(hsla(chrome.text))
                .child(text)
        };
        let mut keys = div()
            .w(px(KEYS_WIDTH))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(3.));
        for (index, group) in row.press.caps(self.mac).into_iter().enumerate() {
            if index > 0 {
                keys = keys.child(
                    div()
                        .mx(px(3.))
                        .text_size(px(12.))
                        .text_color(hsla(chrome.muted))
                        .child("/"),
                );
            }
            keys = keys.children(group.into_iter().map(cap));
        }
        keys
    }

    fn render_row(&self, row: &Listing, show_area: bool) -> Div {
        let chrome = self.chrome;
        let mut hover = hsla(chrome.sidebar);
        hover.a = 0.6;
        div()
            .min_h(px(34.))
            .flex()
            .items_center()
            .gap(px(14.))
            .px(px(10.))
            .py(px(3.))
            .rounded(px(9.))
            .hover(move |style| style.bg(hover))
            .child(self.render_caps(row))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_none()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(row.name),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(hsla(chrome.muted))
                            .child(row.what),
                    ),
            )
            .when(show_area, |element| {
                element.child(
                    div()
                        .w(px(AREA_WIDTH))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .child(
                            div()
                                .px(px(6.))
                                .rounded(px(5.))
                                .border_1()
                                .border_color(hsla(chrome.track))
                                .text_size(px(11.5))
                                .line_height(px(17.))
                                .text_color(hsla(chrome.muted))
                                .child(row.area.name()),
                        ),
                )
            })
    }

    fn render_area_heading(&self, area: Area) -> Div {
        div()
            .px(px(10.))
            .pt(px(14.))
            .pb(px(4.))
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(hsla(self.chrome.muted))
            .child(area.name().to_uppercase())
    }

    fn render_list(&self) -> Div {
        let mut list = div().flex().flex_col().px(px(8.)).pb(px(8.));
        if self.filter.is_empty() {
            for area in Area::ALL {
                list = list.child(self.render_area_heading(area)).children(
                    self.rows
                        .iter()
                        .filter(|row| row.area == area)
                        .map(|row| self.render_row(row, false)),
                );
            }
            return list;
        }
        let hits: Vec<&Listing> = self
            .rows
            .iter()
            .filter(|row| shortcuts::matches(row, &self.filter, self.mac))
            .collect();
        if hits.is_empty() {
            return list.child(
                div()
                    .px(px(10.))
                    .py(px(24.))
                    .text_color(hsla(self.chrome.muted))
                    .child("No shortcut matches."),
            );
        }
        list.pt(px(6.))
            .children(hits.into_iter().map(|row| self.render_row(row, true)))
    }

    fn render_filter(&self) -> Option<Div> {
        if self.filter.is_empty() {
            return None;
        }
        let chrome = self.chrome;
        Some(
            div()
                .mx(px(16.))
                .mt(px(12.))
                .h(px(34.))
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                .rounded(px(9.))
                .border_1()
                .border_color(hsla(chrome.accent))
                .bg(hsla(chrome.sidebar))
                .child(div().text_color(hsla(chrome.muted)).child("Filter"))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(self.filter.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(hsla(chrome.muted))
                        .child("Esc clears"),
                ),
        )
    }
}

/// The window's own key, as shown in its title.
fn title_keys(mac: bool) -> &'static str {
    if mac { "⌘/" } else { "Ctrl+Shift+/" }
}

/// What the footer says about the command modifier on this kind of system.
fn footer_text(mac: bool) -> &'static str {
    if mac {
        "App shortcuts use ⌘, which agents never receive. Type to filter."
    } else {
        "App shortcuts use Ctrl+Shift, so plain Ctrl keys still reach the agents. Type to filter."
    }
}

impl Focusable for ShortcutsWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ShortcutsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        let viewport = window.viewport_size();
        let fit = |want: f32, room: gpui::Pixels| want.min(f32::from(room) - 2. * MARGIN).max(320.);
        div()
            .id("keyboard-shortcuts")
            .key_context("KeyboardShortcuts")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            // A click outside counts as Escape.
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(ShortcutsEvent::Close)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(px(fit(WIDTH, viewport.width)))
            .h(px(fit(HEIGHT, viewport.height)))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(14.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.selected))
            .shadow_lg()
            .text_size(px(14.))
            .text_color(hsla(chrome.text))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(18.))
                    .h(px(48.))
                    .border_b_1()
                    .border_color(hsla(chrome.sidebar_line))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Keyboard Shortcuts"),
                    )
                    .child(
                        div()
                            .px(px(6.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(hsla(chrome.track))
                            .text_size(px(11.5))
                            .line_height(px(17.))
                            .text_color(hsla(chrome.muted))
                            .child(title_keys(self.mac)),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("shortcuts-close")
                            .size(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .text_color(hsla(chrome.muted))
                            .cursor_pointer()
                            .hover(move |style| style.text_color(hsla(chrome.text)))
                            .child("✕")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(ShortcutsEvent::Close))),
                    ),
            )
            .children(self.render_filter())
            .child(
                div()
                    .id("shortcuts-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.render_list()),
            )
            .child(
                div()
                    .flex_none()
                    .px(px(18.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(hsla(chrome.sidebar_line))
                    .text_size(px(13.))
                    .text_color(hsla(chrome.muted))
                    .child(footer_text(self.mac)),
            )
    }
}
