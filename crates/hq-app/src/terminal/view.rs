//! The terminal pane: paints the attached session's screen and turns the owner's
//! keys, text input, clipboard and mouse into input for the agent.

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use alacritty_terminal::index::Side;
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::CursorShape;
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, DispatchPhase, ElementInputHandler, Entity,
    EntityInputHandler, ExternalPaths, FocusHandle, Focusable, Font, FontStyle, FontWeight, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollDelta,
    ScrollWheelEvent, SharedString, StrikethroughStyle, Styled, TextAlign, TextRun, UTF16Selection,
    UnderlineStyle, Window, canvas, div, fill, point, px, size,
};

use super::copy;
use super::frame::{self, Frame, TextStyle, Underline};
use super::keys::{self, Binding, KeyModes, KeyPress};
use super::links::{self, Link, LinkFinder};
use super::model::GridSize;
use super::session::TerminalSession;
use crate::shortcuts;
use crate::theme::Paint;

/// Appearance and keyboard options for the pane, from settings.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneOptions {
    pub font_family: SharedString,
    pub font_size: f32,
    pub normal_weight: FontWeight,
    pub bold_weight: FontWeight,
    pub italic_weight: FontWeight,
    pub bold_italic_weight: FontWeight,
    /// False when the italic or bold italic style falls back to the upright normal face.
    pub italic_slanted: bool,
    pub bold_italic_slanted: bool,
    pub padding: (f32, f32),
    pub alt_is_meta: bool,
    pub bindings: Vec<Binding>,
    /// 0.0 none, up to 0.6: how much to fade the pane while it or the window lacks focus.
    pub dim_unfocused: f32,
    /// Alpha of the terminal's default background; cells with their own colour stay solid.
    pub opacity: f32,
    /// Cursor shape until the agent asks for another; `None` keeps the block.
    pub cursor: Option<CursorShape>,
}

impl Default for PaneOptions {
    fn default() -> Self {
        PaneOptions {
            font_family: default_font().into(),
            font_size: 13.0,
            normal_weight: FontWeight::NORMAL,
            bold_weight: FontWeight::BOLD,
            italic_weight: FontWeight::NORMAL,
            bold_italic_weight: FontWeight::BOLD,
            italic_slanted: true,
            bold_italic_slanted: true,
            padding: (12.0, 10.0),
            alt_is_meta: false,
            bindings: Vec::new(),
            dim_unfocused: 0.0,
            opacity: 1.0,
            cursor: None,
        }
    }
}

pub fn default_font() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else {
        "DejaVu Sans Mono"
    }
}

/// Where the grid was last painted, so mouse positions map to cells.
#[derive(Debug, Clone, Copy)]
struct Layout {
    origin: Point<Pixels>,
    cell_width: Pixels,
    line_height: Pixels,
    columns: usize,
    rows: usize,
}

/// A left-button press, which becomes a drag once the pointer reaches another cell.
#[derive(Debug, Clone, Copy)]
struct Press {
    row: usize,
    column: usize,
    side: Side,
    shift: bool,
}

/// A left-button press on a link: it opens on release unless the pointer reaches
/// another cell first, which makes it an ordinary drag (add-terminal-links).
#[derive(Debug, Clone)]
struct LinkPress {
    row: usize,
    column: usize,
    side: Side,
    link: Link,
}

/// What releasing the left button does (fix-agent-clipboard-copy D1). Headquarter owns
/// drags for every agent: a drag selects and its release copies. A click without a drag
/// goes to an agent that uses the mouse (Shift keeps it here), else clears the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Release {
    Copy,
    ClickToAgent,
    ClearSelection,
}

/// How long the note after a copy stays.
const COPIED_NOTE: std::time::Duration = std::time::Duration::from_millis(1500);
/// How often a drag past an edge scrolls.
const AUTOSCROLL_STEP: std::time::Duration = std::time::Duration::from_millis(50);

/// Lines to scroll per step for a drag at `y`: positive (back in history) above `top`,
/// negative below `bottom`, one plus one per line height further out, at most ten.
fn autoscroll_lines(y: f32, top: f32, bottom: f32, line_height: f32) -> i32 {
    let past = if y < top {
        top - y
    } else if y > bottom {
        bottom - y
    } else {
        return 0;
    };
    let lines = (1.0 + (past.abs() / line_height.max(1.0)).floor()).min(10.0) as i32;
    if past > 0.0 { lines } else { -lines }
}

/// A full-screen agent that reports the mouse keeps its history itself, so a drag is
/// its to handle: it selects, scrolls and copies (D1).
fn agent_owns_drags(mode: TermMode) -> bool {
    mode.contains(TermMode::ALT_SCREEN) && mode.intersects(TermMode::MOUSE_MODE)
}

fn release(selecting: bool, agent_uses_mouse: bool, shift: bool) -> Release {
    if selecting {
        Release::Copy
    } else if agent_uses_mouse && !shift {
        Release::ClickToAgent
    } else {
        Release::ClearSelection
    }
}

pub struct TerminalView {
    session: Entity<TerminalSession>,
    focus: FocusHandle,
    options: PaneOptions,
    layout: Rc<Cell<Option<Layout>>>,
    /// Text the input method is composing, shown at the cursor until committed.
    marked: Option<String>,
    /// Where the left button went down, until it comes up.
    press: Option<Press>,
    selecting: bool,
    /// Where the pointer is during a drag, inside the pane or not.
    pointer: Option<Point<Pixels>>,
    /// Whether a drag past the top or bottom edge is scrolling the view.
    autoscrolling: bool,
    /// The note after a copy, and which copy it is, so an older timer leaves a newer
    /// note alone.
    copied: Option<(u64, String)>,
    copies: u64,
    /// The agent's copies already noted.
    agent_copies_seen: u64,
    /// While a drag is handed to a full-screen agent: the last cell reported to it.
    forwarding: Option<(usize, usize)>,
    /// While set, the grid keeps its size, so the agent redraws once when the sidebar's
    /// divider is released rather than on every pixel of the drag.
    size_held: bool,
    links: LinkFinder,
    /// The cell under the pointer while no button is down, and the link there, found
    /// again on each draw as the output changes.
    hover_cell: Option<(usize, usize)>,
    hovered: Option<Link>,
    link_press: Option<LinkPress>,
}

impl TerminalView {
    pub fn new(
        session: Entity<TerminalSession>,
        options: PaneOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&session, |this: &mut TerminalView, session, cx| {
            let (count, note) = session.read(cx).agent_copies.clone();
            if count != this.agent_copies_seen {
                this.agent_copies_seen = count;
                this.show_copied(note, cx);
            }
            cx.notify();
        })
        .detach();
        cx.observe_window_activation(window, |_, _, cx| cx.notify())
            .detach();
        let cursor = options.cursor;
        session.update(cx, |session, _| {
            session.model_mut().set_default_cursor(cursor);
        });
        TerminalView {
            session,
            focus: cx.focus_handle(),
            options,
            layout: Rc::new(Cell::new(None)),
            marked: None,
            press: None,
            selecting: false,
            pointer: None,
            autoscrolling: false,
            copied: None,
            copies: 0,
            agent_copies_seen: 0,
            forwarding: None,
            size_held: false,
            links: LinkFinder::new(),
            hover_cell: None,
            hovered: None,
            link_press: None,
        }
    }

    pub fn hold_size(&mut self, held: bool, cx: &mut Context<Self>) {
        if self.size_held != held {
            self.size_held = held;
            cx.notify();
        }
    }

    pub fn session(&self) -> &Entity<TerminalSession> {
        &self.session
    }

    pub fn set_options(&mut self, options: PaneOptions, cx: &mut Context<Self>) {
        if self.options == options {
            return;
        }
        if self.options.cursor != options.cursor {
            let cursor = options.cursor;
            self.session.update(cx, |session, cx| {
                session.model_mut().set_default_cursor(cursor);
                cx.notify();
            });
        }
        self.options = options;
        cx.notify();
    }

    fn mode(&self, cx: &App) -> TermMode {
        self.session.read(cx).model().mode()
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        let keys = shortcuts::Keys {
            control: modifiers.control,
            alt: modifiers.alt,
            shift: modifiers.shift,
            platform: modifiers.platform,
        };
        if shortcuts::shortcut(&keystroke.key, keys, shortcuts::MAC).is_some() {
            // The app's own shortcut: left to the window.
            return;
        }
        if self.clipboard_shortcut(&keystroke.key, modifiers, window, cx) {
            cx.stop_propagation();
            return;
        }
        let press = KeyPress {
            key: &keystroke.key,
            control: modifiers.control,
            alt: modifiers.alt,
            shift: modifiers.shift,
            platform: modifiers.platform,
        };
        if self.send_key(press, cx) {
            cx.stop_propagation();
        }
    }

    /// Tab and Shift-Tab, which arrive as actions (`terminal::bind_keys`).
    fn send_tab(&mut self, shift: bool, cx: &mut Context<Self>) {
        let press = KeyPress {
            key: "tab",
            shift,
            ..KeyPress::default()
        };
        self.send_key(press, cx);
    }

    /// Sends what `press` encodes to; false when it is not the terminal's key.
    fn send_key(&mut self, press: KeyPress<'_>, cx: &mut Context<Self>) -> bool {
        let modes = KeyModes {
            app_cursor: self.mode(cx).contains(TermMode::APP_CURSOR),
            alt_is_meta: self.options.alt_is_meta,
        };
        let Some(bytes) = keys::encode(press, modes, &self.options.bindings) else {
            return false;
        };
        self.session
            .update(cx, |session, cx| session.input(&bytes, cx));
        true
    }

    /// Copy and paste: Cmd-C/Cmd-V on macOS, Ctrl-Shift-C/V elsewhere.
    fn clipboard_shortcut(
        &mut self,
        key: &str,
        modifiers: Modifiers,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let chord = if cfg!(target_os = "macos") {
            modifiers.platform && !modifiers.control && !modifiers.alt
        } else {
            modifiers.control && modifiers.shift && !modifiers.alt
        };
        if !chord {
            return false;
        }
        match key {
            "c" => {
                self.copy_selection(cx);
                true
            }
            "v" => {
                self.paste_clipboard(cx);
                true
            }
            _ => false,
        }
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let model = self.session.read(cx).model();
        let Some(text) = model.selected_text() else {
            return;
        };
        let text = copy::tidy(&text, model.selection_starts_mid_row());
        if text.is_empty() {
            return;
        }
        let note = copy::copied_note(&text);
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.show_copied(note, cx);
    }

    /// Says what was copied for a moment (fix-agent-clipboard-copy D4).
    fn show_copied(&mut self, note: String, cx: &mut Context<Self>) {
        self.copies += 1;
        let copy = self.copies;
        self.copied = Some((copy, note));
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_NOTE).await;
            let _ = this.update(cx, |this, cx| {
                if this
                    .copied
                    .as_ref()
                    .is_some_and(|(shown, _)| *shown == copy)
                {
                    this.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.paste(&text, cx);
        }
    }

    fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        let bracketed = self.mode(cx).contains(TermMode::BRACKETED_PASTE);
        let bytes = keys::paste(text, bracketed);
        self.session
            .update(cx, |session, cx| session.input(&bytes, cx));
    }

    /// Files dropped on the pane are typed as their paths, as a paste (add-terminal-file-drop).
    fn drop_paths(&mut self, paths: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        let text = keys::dropped_paths(paths.paths());
        if text.is_empty() {
            return;
        }
        window.focus(&self.focus, cx);
        self.paste(&text, cx);
    }

    /// The half of the cell under the pointer; past the grid's left or right edge, the
    /// outer half of the edge cell.
    fn side_at(&self, position: Point<Pixels>) -> Side {
        let Some(layout) = self.layout.get() else {
            return Side::Left;
        };
        let x = (position.x - layout.origin.x) / layout.cell_width;
        if x <= 0.0 {
            Side::Left
        } else if x >= layout.columns as f32 || x.fract() >= 0.5 {
            Side::Right
        } else {
            Side::Left
        }
    }

    fn cell_at(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let layout = self.layout.get()?;
        let x = (position.x - layout.origin.x) / layout.cell_width;
        let y = (position.y - layout.origin.y) / layout.line_height;
        let column = x.max(0.0).floor() as usize;
        let row = y.max(0.0).floor() as usize;
        Some((
            row.min(layout.rows.saturating_sub(1)),
            column.min(layout.columns.saturating_sub(1)),
        ))
    }

    /// The visible cell at a window position, or `None` outside the grid.
    fn cell_inside(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let layout = self.layout.get()?;
        let x = (position.x - layout.origin.x) / layout.cell_width;
        let y = (position.y - layout.origin.y) / layout.line_height;
        let inside = x >= 0.0 && y >= 0.0;
        let (column, row) = (x.floor() as usize, y.floor() as usize);
        (inside && column < layout.columns && row < layout.rows).then_some((row, column))
    }

    fn link_at(&mut self, cell: Option<(usize, usize)>, cx: &App) -> Option<Link> {
        let (row, column) = cell?;
        let term = self.session.read(cx).model().term();
        self.links.at(term, links::view_point(term, row, column))
    }

    /// The pointer moved with no button down, anywhere in the window.
    fn hover(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let cell = self.cell_inside(position);
        if cell == self.hover_cell {
            return;
        }
        self.hover_cell = cell;
        let link = self.link_at(cell, cx);
        if link != self.hovered {
            self.hovered = link;
            cx.notify();
        }
    }

    fn agent_owns_drags(&self, cx: &App) -> bool {
        agent_owns_drags(self.mode(cx))
    }

    fn mouse_reporting(&self, cx: &App) -> bool {
        self.mode(cx).intersects(TermMode::MOUSE_MODE)
    }

    /// An SGR (1006) mouse report; the only encoding sent, as agents ask for it.
    fn report_mouse(&self, button: u8, row: usize, column: usize, pressed: bool, cx: &App) {
        if !self.mode(cx).contains(TermMode::SGR_MOUSE) {
            return;
        }
        let action = if pressed { 'M' } else { 'm' };
        let report = format!("\x1b[<{button};{};{}{action}", column + 1, row + 1);
        self.session.read(cx).report(report.as_bytes());
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let Some((row, column)) = self.cell_at(event.position) else {
            return;
        };
        if event.click_count == 1
            && !event.modifiers.shift
            && let Some(link) = self.link_at(self.cell_inside(event.position), cx)
        {
            self.clear_selection(cx);
            self.link_press = Some(LinkPress {
                row,
                column,
                side: self.side_at(event.position),
                link,
            });
            return;
        }
        // A full-screen agent that uses the mouse keeps its own history, so it selects,
        // scrolls and copies itself; Shift keeps the drag here (D1).
        if self.agent_owns_drags(cx) && !event.modifiers.shift {
            self.clear_selection(cx);
            self.report_mouse(0, row, column, true, cx);
            self.forwarding = Some((row, column));
            return;
        }
        let side = self.side_at(event.position);
        self.press = Some(Press {
            row,
            column,
            side,
            shift: event.modifiers.shift,
        });
        // A double or triple click selects a word or a line at once; a single press
        // waits to see whether it becomes a drag.
        let kind = match event.click_count {
            2 => SelectionType::Semantic,
            3 => SelectionType::Lines,
            _ => return,
        };
        self.session.update(cx, |session, cx| {
            session.model_mut().start_selection(row, column, side, kind);
            cx.notify();
        });
        self.selecting = true;
    }

    /// The pointer moved with the left button down, anywhere in the window: a press
    /// that reaches another cell becomes a selection, which follows the pointer and
    /// scrolls when it goes past the top or bottom (D3).
    fn drag_to(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(press) = self.link_press.take() {
            if self.cell_at(position) == Some((press.row, press.column)) {
                self.link_press = Some(press);
                return;
            }
            // Left the link's cell: a drag from there, as if pressed off a link.
            let LinkPress {
                row, column, side, ..
            } = press;
            if self.agent_owns_drags(cx) {
                self.report_mouse(0, row, column, true, cx);
                self.forwarding = Some((row, column));
            } else {
                self.press = Some(Press {
                    row,
                    column,
                    side,
                    shift: false,
                });
            }
        }
        if let Some(last) = self.forwarding {
            // Reported cell by cell; past an edge it stays on the edge row, where the
            // agent scrolls.
            if let Some(cell) = self.cell_at(position)
                && cell != last
                && self
                    .mode(cx)
                    .intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
            {
                self.report_mouse(32, cell.0, cell.1, true, cx);
                self.forwarding = Some(cell);
            }
            return;
        }
        let Some(press) = self.press else {
            return;
        };
        self.pointer = Some(position);
        let Some((row, column)) = self.cell_at(position) else {
            return;
        };
        if !self.selecting {
            if (press.row, press.column) == (row, column) {
                return;
            }
            self.session.update(cx, |session, _| {
                session.model_mut().start_selection(
                    press.row,
                    press.column,
                    press.side,
                    SelectionType::Simple,
                );
            });
            self.selecting = true;
        }
        let side = self.side_at(position);
        self.session.update(cx, |session, cx| {
            session.model_mut().extend_selection(row, column, side);
            cx.notify();
        });
        self.keep_scrolling(cx);
    }

    /// Lines to scroll now for a drag past an edge; 0 inside the pane.
    fn autoscroll_now(&self) -> i32 {
        let (Some(layout), Some(pointer)) = (self.layout.get(), self.pointer) else {
            return 0;
        };
        let top = f32::from(layout.origin.y);
        let bottom = top + f32::from(layout.line_height) * layout.rows as f32;
        autoscroll_lines(
            f32::from(pointer.y),
            top,
            bottom,
            f32::from(layout.line_height),
        )
    }

    fn keep_scrolling(&mut self, cx: &mut Context<Self>) {
        if self.autoscrolling || !self.selecting || self.autoscroll_now() == 0 {
            return;
        }
        self.autoscrolling = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(AUTOSCROLL_STEP).await;
                let going = this
                    .update(cx, |this, cx| this.autoscroll_step(cx))
                    .unwrap_or(false);
                if !going {
                    break;
                }
            }
        })
        .detach();
    }

    fn autoscroll_step(&mut self, cx: &mut Context<Self>) -> bool {
        let lines = if self.selecting {
            self.autoscroll_now()
        } else {
            0
        };
        if lines == 0 {
            self.autoscrolling = false;
            return false;
        }
        let edge = self
            .pointer
            .and_then(|pointer| Some((self.cell_at(pointer)?, self.side_at(pointer))));
        self.session.update(cx, |session, cx| {
            let model = session.model_mut();
            model.scroll(lines);
            if let Some(((row, column), side)) = edge {
                model.extend_selection(row, column, side);
            }
            cx.notify();
        });
        true
    }

    /// The left button came up, anywhere in the window, after a press in the pane.
    fn finish_press(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(press) = self.link_press.take() {
            cx.open_url(press.link.address());
            return;
        }
        if let Some(last) = self.forwarding.take() {
            let (row, column) = self.cell_at(position).unwrap_or(last);
            self.report_mouse(0, row, column, false, cx);
            return;
        }
        let Some(press) = self.press.take() else {
            return;
        };
        self.pointer = None;
        match release(self.selecting, self.mouse_reporting(cx), press.shift) {
            Release::Copy => {
                self.selecting = false;
                self.copy_selection(cx);
            }
            Release::ClickToAgent => {
                self.clear_selection(cx);
                // The press was held back to tell a click from a drag; both go now.
                let (row, column) = self.cell_at(position).unwrap_or((press.row, press.column));
                self.report_mouse(0, press.row, press.column, true, cx);
                self.report_mouse(0, row, column, false, cx);
            }
            Release::ClearSelection => self.clear_selection(cx),
        }
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.session.update(cx, |session, cx| {
            session.model_mut().clear_selection();
            cx.notify();
        });
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(layout) = self.layout.get() else {
            return;
        };
        let lines = match event.delta {
            ScrollDelta::Lines(delta) => delta.y,
            ScrollDelta::Pixels(delta) => delta.y / layout.line_height,
        };
        let steps = lines.round() as i32;
        if steps == 0 {
            return;
        }
        let mode = self.mode(cx);
        if mode.intersects(TermMode::MOUSE_MODE) {
            if let Some((row, column)) = self.cell_at(event.position) {
                let button = if steps > 0 { 64 } else { 65 };
                for _ in 0..steps.unsigned_abs().min(10) {
                    self.report_mouse(button, row, column, true, cx);
                }
            }
        } else if mode.contains(TermMode::ALT_SCREEN) {
            // Full-screen agents without mouse reporting scroll with arrow keys, as
            // common terminals do.
            let arrow: &[u8] = match (steps > 0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            let bytes = arrow.repeat(steps.unsigned_abs().min(10) as usize);
            self.session.read(cx).report(&bytes);
        } else {
            self.session.update(cx, |session, cx| {
                session.model_mut().scroll(steps);
                cx.notify();
            });
        }
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// A colour, or a colour with its opacity, as GPUI paints it.
pub fn hsla(color: impl Into<Paint>) -> Hsla {
    let paint = color.into();
    gpui::Rgba {
        r: f32::from(paint.rgb.r) / 255.0,
        g: f32::from(paint.rgb.g) / 255.0,
        b: f32::from(paint.rgb.b) / 255.0,
        a: paint.alpha(),
    }
    .into()
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.hovered = self.link_at(self.hover_cell, cx);
        let frame = frame::build(self.session.read(cx).model(), self.hovered.as_ref());
        let pointer = if self.hovered.is_some() {
            CursorStyle::PointingHand
        } else {
            CursorStyle::Arrow
        };
        let focused = self.focus.is_focused(window) && window.is_window_active();
        let ended = self.session.read(cx).ended.clone();
        let view = cx.entity();
        let session = self.session.clone();
        let focus = self.focus.clone();
        let layout = Rc::clone(&self.layout);
        let options = self.options.clone();
        let marked = self.marked.clone();
        let copied = self.copied.as_ref().map(|(_, note)| note.clone());
        let size_held = self.size_held;
        let fade = if focused { 0.0 } else { options.dim_unfocused };
        let background = hsla(frame.background);
        let badge = hsla(frame.cursor_color);
        let mut see_through = background;
        // The window's background opacity and the terminal background's own multiply.
        see_through.a = options.opacity * background.a;

        div()
            .size_full()
            .bg(see_through)
            .track_focus(&self.focus)
            .cursor(pointer)
            .key_context(super::CONTEXT)
            .on_action(cx.listener(|this, _: &crate::menus::Copy, _, cx| this.copy_selection(cx)))
            .on_action(cx.listener(|this, _: &crate::menus::Paste, _, cx| this.paste_clipboard(cx)))
            .on_action(cx.listener(|this, _: &super::SendTab, _, cx| this.send_tab(false, cx)))
            .on_action(cx.listener(|this, _: &super::SendShiftTab, _, cx| this.send_tab(true, cx)))
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_scroll_wheel(cx.listener(Self::scroll))
            .on_drop(cx.listener(Self::drop_paths))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let metrics = Metrics::measure(&options, window);
                        let origin = point(
                            bounds.origin.x + px(options.padding.0),
                            bounds.origin.y + px(options.padding.1),
                        );
                        let width = bounds.size.width - px(options.padding.0 * 2.0);
                        let height = bounds.size.height - px(options.padding.1 * 2.0);
                        let grid = grid_size(
                            f32::from(width),
                            f32::from(height),
                            f32::from(metrics.cell_width),
                            f32::from(metrics.line_height),
                        );
                        let GridSize { columns, rows } = grid;
                        if !size_held {
                            session.update(cx, |session, cx| session.set_size(grid, cx));
                        }
                        layout.set(Some(Layout {
                            origin,
                            cell_width: metrics.cell_width,
                            line_height: metrics.line_height,
                            columns,
                            rows,
                        }));
                        (metrics, origin, options)
                    },
                    move |bounds, (metrics, origin, options), window, cx| {
                        paint_frame(
                            &frame,
                            &metrics,
                            origin,
                            &options,
                            focused,
                            marked.as_deref(),
                            window,
                            cx,
                        );
                        if fade > 0.0 {
                            let mut veil = background;
                            veil.a = fade;
                            window.paint_quad(fill(bounds, veil));
                        }
                        // A drag goes on outside the pane: the window reports the
                        // pointer and the release, not only this element (D3).
                        let mover = view.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            match event.pressed_button {
                                Some(MouseButton::Left) => {
                                    mover.update(cx, |this, cx| this.drag_to(event.position, cx));
                                }
                                None => mover.update(cx, |this, cx| this.hover(event.position, cx)),
                                Some(_) => {}
                            }
                        });
                        let releaser = view.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                                releaser
                                    .update(cx, |this, cx| this.finish_press(event.position, cx));
                            }
                        });
                        window.handle_input(&focus, ElementInputHandler::new(bounds, view), cx);
                    },
                )
                .size_full(),
            )
            .children(copied.map(|note| {
                div()
                    .absolute()
                    .bottom(px(12.))
                    .right(px(12.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(8.))
                    .bg(badge)
                    .text_color(background)
                    .text_size(px(13.))
                    .child(note)
            }))
            .children(ended.map(|message| {
                div()
                    .absolute()
                    .bottom(px(12.))
                    .left(px(12.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(8.))
                    .bg(badge)
                    .text_color(background)
                    .text_size(px(13.))
                    .child(message)
            }))
    }
}

/// Whole cells that fit in the padded area; at least one of each.
fn grid_size(width: f32, height: f32, cell_width: f32, line_height: f32) -> GridSize {
    let fit = |space: f32, cell: f32| {
        if cell > 0.0 {
            (space / cell).floor().max(1.0) as usize
        } else {
            1
        }
    };
    GridSize {
        columns: fit(width, cell_width),
        rows: fit(height, line_height),
    }
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    cell_width: Pixels,
    line_height: Pixels,
    font_size: Pixels,
}

impl Metrics {
    fn measure(options: &PaneOptions, window: &mut Window) -> Metrics {
        let font_size = px(options.font_size);
        let text_system = window.text_system();
        let font_id = text_system.resolve_font(&regular_font(options));
        let cell_width = text_system
            .advance(font_id, font_size, 'm')
            .map(|advance| advance.width)
            .unwrap_or(font_size * 0.6);
        let ascent = f32::from(text_system.ascent(font_id, font_size));
        let descent = f32::from(text_system.descent(font_id, font_size));
        Metrics {
            cell_width,
            line_height: px(line_height(ascent, descent, options.font_size)),
            font_size,
        }
    }
}

/// The line height from the font's own ascent and descent, each rounded, as Alacritty
/// does (fix-font-styles-like-alacritty D4); 1.2 times the size when the font reports
/// none. GPUI does not expose the line gap, which terminal fonts commonly set to 0.
fn line_height(ascent: f32, descent: f32, font_size: f32) -> f32 {
    let height = ascent.abs().round() + descent.abs().round();
    if height > 0.0 {
        height
    } else {
        (font_size * 1.2).round()
    }
}

fn regular_font(options: &PaneOptions) -> Font {
    Font {
        weight: options.normal_weight,
        ..gpui::font(options.font_family.clone())
    }
}

fn styled_font(options: &PaneOptions, style: &TextStyle) -> Font {
    Font {
        weight: match (style.bold, style.italic) {
            (false, false) => options.normal_weight,
            (true, false) => options.bold_weight,
            (false, true) => options.italic_weight,
            (true, true) => options.bold_italic_weight,
        },
        style: match (style.bold, style.italic) {
            (false, true) if options.italic_slanted => FontStyle::Italic,
            (true, true) if options.bold_italic_slanted => FontStyle::Italic,
            _ => FontStyle::Normal,
        },
        ..regular_font(options)
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_frame(
    frame: &Frame,
    metrics: &Metrics,
    origin: Point<Pixels>,
    options: &PaneOptions,
    focused: bool,
    marked: Option<&str>,
    window: &mut Window,
    cx: &mut App,
) {
    let cell_bounds = |row: usize, column: usize, cells: usize| {
        Bounds::new(
            point(
                origin.x + metrics.cell_width * column as f32,
                origin.y + metrics.line_height * row as f32,
            ),
            size(metrics.cell_width * cells as f32, metrics.line_height),
        )
    };

    for run in frame::background_runs(frame) {
        window.paint_quad(fill(
            cell_bounds(run.row, run.column, run.cells),
            hsla(run.color),
        ));
    }

    let cursor = frame.cursor;
    if let Some(cursor) = cursor
        && focused
        && matches!(cursor.shape, CursorShape::Block)
    {
        window.paint_quad(fill(
            cell_bounds(cursor.row, cursor.column, 1),
            hsla(frame.cursor_color),
        ));
    }

    for (row, cells) in frame.rows.iter().enumerate() {
        for run in frame::text_runs(row, cells) {
            let mut style = run.style;
            if let Some(cursor) = cursor
                && focused
                && cursor.shape == CursorShape::Block
                && cursor.row == row
                && run.column == cursor.column
                && run.cells <= 2
            {
                style.color = frame.background;
            }
            paint_text(
                &run.text,
                run.column,
                row,
                run.wide,
                &style,
                metrics,
                &cell_bounds,
                options,
                window,
                cx,
            );
        }
    }

    // A block cursor over a multi-cell run needs its character redrawn in the
    // background colour; single runs are handled above.
    if let Some(cursor) = cursor {
        let bounds = cell_bounds(cursor.row, cursor.column, 1);
        let color = hsla(frame.cursor_color);
        match (cursor.shape, focused) {
            (CursorShape::Block, true) => {
                let cell = &frame.rows[cursor.row][cursor.column];
                if !cell.spacer && cell.ch != ' ' {
                    let style = TextStyle {
                        color: frame.background,
                        ..cell.style
                    };
                    paint_text(
                        &cell.ch.to_string(),
                        cursor.column,
                        cursor.row,
                        cell.wide,
                        &style,
                        metrics,
                        &cell_bounds,
                        options,
                        window,
                        cx,
                    );
                }
            }
            (CursorShape::Beam, true) => {
                window.paint_quad(fill(
                    Bounds::new(bounds.origin, size(px(2.), bounds.size.height)),
                    color,
                ));
            }
            (CursorShape::Underline, true) => {
                let y = bounds.origin.y + bounds.size.height - px(2.);
                window.paint_quad(fill(
                    Bounds::new(point(bounds.origin.x, y), size(bounds.size.width, px(2.))),
                    color,
                ));
            }
            _ => {
                // Unfocused or hollow: an outline, so the position stays visible.
                window.paint_quad(
                    fill(bounds, gpui::transparent_black())
                        .border_widths(px(1.))
                        .border_color(color),
                );
            }
        }
        if let Some(text) = marked.filter(|text| !text.is_empty()) {
            let style = TextStyle {
                color: frame.cursor_color,
                bold: false,
                italic: false,
                underline: Underline::Single,
                strike: false,
            };
            paint_text(
                text,
                cursor.column,
                cursor.row,
                false,
                &style,
                metrics,
                &cell_bounds,
                options,
                window,
                cx,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_text(
    text: &str,
    column: usize,
    row: usize,
    wide: bool,
    style: &TextStyle,
    metrics: &Metrics,
    cell_bounds: &dyn Fn(usize, usize, usize) -> Bounds<Pixels>,
    options: &PaneOptions,
    window: &mut Window,
    cx: &mut App,
) {
    let color = hsla(style.color);
    let underline = match style.underline {
        Underline::None => None,
        kind => Some(UnderlineStyle {
            thickness: px(if kind == Underline::Double { 2. } else { 1. }),
            color: Some(color),
            wavy: kind == Underline::Curly,
        }),
    };
    let run = TextRun {
        len: text.len(),
        font: styled_font(options, style),
        color,
        background_color: None,
        underline,
        strikethrough: style.strike.then_some(StrikethroughStyle {
            thickness: px(1.),
            color: Some(color),
        }),
    };
    // Every glyph gets exactly one cell so columns line up; wide characters keep their
    // natural width across two cells.
    let force_width = (!wide).then_some(metrics.cell_width);
    let shaped = window.text_system().shape_line(
        SharedString::from(text.to_owned()),
        metrics.font_size,
        &[run],
        force_width,
    );
    let origin = cell_bounds(row, column, 1).origin;
    let _ = shaped.paint(
        origin,
        metrics.line_height,
        TextAlign::Left,
        None,
        window,
        cx,
    );
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        _range: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|text| 0..text.encode_utf16().count())
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked = None;
        self.session
            .update(cx, |session, cx| session.input(text.as_bytes(), cx));
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked = Some(new_text.to_owned());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Places the input method's candidate window at the cursor.
        let layout = self.layout.get()?;
        let cursor = frame::build(self.session.read(cx).model(), None).cursor?;
        Some(Bounds::new(
            point(
                layout.origin.x + layout.cell_width * cursor.column as f32,
                layout.origin.y + layout.line_height * cursor.row as f32,
            ),
            size(layout.cell_width, layout.line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_past_an_edge_scrolls_faster_further_out() {
        assert_eq!(autoscroll_lines(50.0, 10.0, 400.0, 18.0), 0, "inside");
        assert_eq!(
            autoscroll_lines(5.0, 10.0, 400.0, 18.0),
            1,
            "just above: back one line"
        );
        assert_eq!(autoscroll_lines(-40.0, 10.0, 400.0, 18.0), 3);
        assert_eq!(
            autoscroll_lines(410.0, 10.0, 400.0, 18.0),
            -1,
            "just below: forward"
        );
        assert_eq!(
            autoscroll_lines(-2000.0, 10.0, 400.0, 18.0),
            10,
            "at most ten"
        );
    }

    #[test]
    fn only_full_screen_mouse_agents_take_drags() {
        let full_screen = TermMode::ALT_SCREEN | TermMode::MOUSE_DRAG;
        assert!(
            agent_owns_drags(full_screen),
            "Claude Code's full-screen view"
        );
        assert!(
            !agent_owns_drags(TermMode::MOUSE_DRAG),
            "normal screen: history is ours"
        );
        assert!(
            !agent_owns_drags(TermMode::ALT_SCREEN),
            "no mouse: Headquarter selects"
        );
    }

    #[test]
    fn drags_select_and_copy_while_clicks_reach_the_agent() {
        assert_eq!(
            release(true, true, false),
            Release::Copy,
            "a drag copies, mouse or not"
        );
        assert_eq!(release(true, false, false), Release::Copy);
        assert_eq!(release(false, true, false), Release::ClickToAgent);
        assert_eq!(
            release(false, true, true),
            Release::ClearSelection,
            "Shift keeps a click here"
        );
        assert_eq!(release(false, false, false), Release::ClearSelection);
    }

    #[test]
    fn line_height_comes_from_the_font_like_alacritty() {
        // JetBrains Mono at 14: ascent 1020 and descent 300 per 1000 units.
        assert_eq!(line_height(14.28, 4.2, 14.0), 18.0);
        assert_eq!(
            line_height(14.28, -4.2, 14.0),
            18.0,
            "either sign of descent"
        );
        assert_eq!(
            line_height(0.0, 0.0, 14.0),
            17.0,
            "no metrics: 1.2 times the size"
        );
    }

    #[test]
    fn grid_size_counts_whole_cells() {
        assert_eq!(
            grid_size(805.0, 490.0, 8.0, 17.0),
            GridSize {
                columns: 100,
                rows: 28
            }
        );
        assert_eq!(
            grid_size(-20.0, 3.0, 8.0, 17.0),
            GridSize {
                columns: 1,
                rows: 1
            },
            "a pane smaller than its padding still has one cell"
        );
        assert_eq!(grid_size(100.0, 100.0, 0.0, 0.0).columns, 1);
    }
}
