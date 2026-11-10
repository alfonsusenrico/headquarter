//! Minimize, maximize and close on Windows, drawn by the app because the window hides the
//! system title bar (fix-windows-app D3). Each button is marked as the window's own control area,
//! so Windows handles the click, the keyboard and snap layouts.

use std::sync::OnceLock;

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, Styled, Window,
    WindowControlArea, div, px, rgb, white,
};

use super::Workspace;
use super::header::HEADER_HEIGHT;
use crate::terminal::view::hsla;

/// Each button's width, as Windows 11 draws its own.
const BUTTON_WIDTH: f32 = 46.0;
/// The room the buttons take at the window's top right.
pub(super) const CAPTION_WIDTH: f32 = 3.0 * BUTTON_WIDTH;
/// Close's hover colour in Windows 11.
const CLOSE_HOVER: u32 = 0xC42B1C;
/// The glyphs of Segoe Fluent Icons, which Segoe MDL2 Assets shares.
const MINIMIZE: &str = "\u{E921}";
const MAXIMIZE: &str = "\u{E922}";
const RESTORE: &str = "\u{E923}";
const CLOSE: &str = "\u{E8BB}";

/// The width the buttons leave out of the pane header: theirs on Windows, none elsewhere.
pub(super) fn caption_reserved() -> f32 {
    if cfg!(windows) { CAPTION_WIDTH } else { 0.0 }
}

/// Windows 11's icon font, else Windows 10's, looked up once.
fn icon_font(cx: &App) -> &'static str {
    static FONT: OnceLock<&'static str> = OnceLock::new();
    FONT.get_or_init(|| {
        let fluent = cx
            .text_system()
            .all_font_names()
            .iter()
            .any(|name| name == "Segoe Fluent Icons");
        if fluent {
            "Segoe Fluent Icons"
        } else {
            "Segoe MDL2 Assets"
        }
    })
}

impl Workspace {
    pub(super) fn render_caption_buttons(&self, window: &Window, cx: &App) -> Option<AnyElement> {
        if !cfg!(windows) {
            return None;
        }
        let chrome = self.theme.chrome;
        let font = icon_font(cx);
        let button = |id: &'static str, area: WindowControlArea, glyph: &'static str| {
            div()
                .id(id)
                .w(px(BUTTON_WIDTH))
                .h(px(HEADER_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .font_family(font)
                .text_size(px(10.))
                .text_color(hsla(chrome.text))
                .window_control_area(area)
                .child(glyph)
        };
        let maximize = if window.is_maximized() {
            RESTORE
        } else {
            MAXIMIZE
        };
        Some(
            div()
                .absolute()
                .top_0()
                .right_0()
                .flex()
                .child(
                    button("caption-minimize", WindowControlArea::Min, MINIMIZE)
                        .hover(|style| style.bg(hsla(chrome.selected))),
                )
                .child(
                    button("caption-maximize", WindowControlArea::Max, maximize)
                        .hover(|style| style.bg(hsla(chrome.selected))),
                )
                .child(
                    button("caption-close", WindowControlArea::Close, CLOSE)
                        .hover(|style| style.bg(rgb(CLOSE_HOVER)).text_color(white())),
                )
                .into_any_element(),
        )
    }
}
