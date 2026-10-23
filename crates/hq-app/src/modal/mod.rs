//! The two session windows, Open sessions (⌘O) and New session (⌘N), in one frame
//! (`openspec/changes/redesign-session-modal/design.md`, D1 and D2).

pub mod folders;
pub mod new_session;
pub mod open_sessions;
pub mod theme_creator;
pub mod theme_switcher;

use gpui::{
    Context, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Stateful, Styled,
    Window, div, px,
};
use gpui_component::input::{
    Backspace, Enter, Escape, IndentInline, MoveDown, MoveLeft, MoveRight, MoveUp,
};

use crate::terminal::view::hsla;
use crate::theme::Chrome;

const FRAME_WIDTH: f32 = 840.0;
const FRAME_HEIGHT: f32 = 560.0;
/// The frame shrinks to leave this much of the app around it.
const FRAME_MARGIN: f32 = 24.0;
const NAV_WIDTH: f32 = 196.0;

/// The window's frame, 840 by 560 or less to fit, with `nav` on the left and `body`
/// beside it.
pub fn frame(
    id: &'static str,
    chrome: Chrome,
    window: &Window,
    nav: impl IntoElement,
    body: impl IntoElement,
) -> Stateful<Div> {
    let viewport = window.viewport_size();
    let fit =
        |want: f32, room: gpui::Pixels| want.min(f32::from(room) - 2. * FRAME_MARGIN).max(320.);
    div()
        .id(id)
        .w(px(fit(FRAME_WIDTH, viewport.width)))
        .h(px(fit(FRAME_HEIGHT, viewport.height)))
        .flex()
        .overflow_hidden()
        .rounded(px(14.))
        .border_1()
        .border_color(hsla(chrome.sidebar_line))
        .bg(hsla(chrome.selected))
        .shadow_lg()
        .child(nav)
        .child(body)
}

/// The "Providers" column, on the sidebar's slightly darker surface.
pub fn nav(chrome: Chrome) -> Div {
    div()
        .w(px(NAV_WIDTH))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(2.))
        .px(px(10.))
        .py(px(14.))
        .bg(hsla(chrome.sidebar))
        .border_r_1()
        .border_color(hsla(chrome.sidebar_line))
        .child(
            div()
                .mx(px(8.))
                .mt(px(4.))
                .mb(px(8.))
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(hsla(chrome.muted))
                .child("PROVIDERS"),
        )
}

/// One entry in the "Providers" column; the chosen one is filled with a bar, as the
/// sidebar's open row is.
pub fn nav_row(
    id: impl Into<ElementId>,
    chosen: bool,
    height: f32,
    chrome: Chrome,
) -> Stateful<Div> {
    let mut hover = hsla(chrome.selected);
    hover.a = 0.55;
    let row = div()
        .id(id)
        .relative()
        .h(px(height))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(9.))
        .px(px(10.))
        .rounded(px(8.))
        .text_size(px(14.))
        .text_color(hsla(chrome.text))
        .hover(move |style| style.bg(hover));
    if !chosen {
        return row;
    }
    row.bg(hsla(chrome.selected)).child(
        div()
            .absolute()
            .left(px(0.))
            .top(px(8.))
            .bottom(px(8.))
            .w(px(3.))
            .rounded(px(2.))
            .bg(hsla(chrome.text)),
    )
}

/// The list's place for the arrow keys and a picked row: the accent, faintly.
pub fn tint(chrome: Chrome, strength: f32) -> gpui::Hsla {
    let mut tint = hsla(chrome.accent);
    tint.a = strength;
    tint
}

/// The round tick at the end of an Open sessions row; filled when picked.
/// The tick's width, kept as an empty slot on rows that cannot be picked so the
/// columns before it line up.
pub const TICK_SIZE: f32 = 20.0;

pub fn tick(picked: bool, chrome: Chrome) -> Div {
    let circle = div()
        .size(px(TICK_SIZE))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::BOLD);
    if picked {
        circle
            .bg(hsla(chrome.accent))
            .border_color(hsla(chrome.accent))
            .text_color(hsla(chrome.selected))
            .child("✓")
    } else {
        circle.border_color(hsla(chrome.track))
    }
}

/// A small outlined label, such as "ext" or "Open here".
pub fn tag(text: &'static str, chrome: Chrome) -> Div {
    div()
        .flex_none()
        .px(px(6.))
        .rounded(px(5.))
        .border_1()
        .border_color(hsla(chrome.track))
        .text_size(px(11.5))
        .line_height(px(17.))
        .text_color(hsla(chrome.muted))
        .child(text)
}

/// A list section heading, such as a day.
pub fn heading(chrome: Chrome) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(8.))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(hsla(chrome.muted))
}

/// The footer along the bottom of the window's body.
pub fn footer(chrome: Chrome) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(16.))
        .py(px(12.))
        .border_t_1()
        .border_color(hsla(chrome.sidebar_line))
        .text_size(px(13.))
        .text_color(hsla(chrome.muted))
}

/// What a window does with a key: true when it used the key.
pub type KeyHandler<V> = fn(&mut V, &str, &mut Window, &mut Context<V>) -> bool;

/// Sends the search field's keys to `handle` before the field and the app's Tab focus
/// order see them. Keys `handle` does not use reach the field as usual.
pub fn route_field_keys<V: 'static>(
    element: Stateful<Div>,
    handle: KeyHandler<V>,
    cx: &mut Context<V>,
) -> Stateful<Div> {
    fn routed<V: 'static, A: gpui::Action>(
        element: Stateful<Div>,
        key: &'static str,
        handle: KeyHandler<V>,
        cx: &mut Context<V>,
    ) -> Stateful<Div> {
        element.capture_action(cx.listener(move |this, _: &A, window, cx| {
            if handle(this, key, window, cx) {
                cx.stop_propagation();
            }
        }))
    }
    let element = routed::<V, IndentInline>(element, "tab", handle, cx);
    let element = routed::<V, MoveUp>(element, "up", handle, cx);
    let element = routed::<V, MoveDown>(element, "down", handle, cx);
    let element = routed::<V, MoveLeft>(element, "left", handle, cx);
    let element = routed::<V, MoveRight>(element, "right", handle, cx);
    let element = routed::<V, Enter>(element, "enter", handle, cx);
    let element = routed::<V, Escape>(element, "escape", handle, cx);
    routed::<V, Backspace>(element, "backspace", handle, cx)
}
