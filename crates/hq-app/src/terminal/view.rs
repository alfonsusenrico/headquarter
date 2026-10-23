//! The terminal pane: paints the attached session's screen and turns the owner's
//! keys, text input, clipboard and mouse into input for the agent.

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::CursorShape;
use gpui::{
    App, Bounds, ClipboardItem, Context, ElementInputHandler, Entity, EntityInputHandler,
    ExternalPaths, FocusHandle, Focusable, Font, FontStyle, FontWeight, Hsla, InteractiveElement,
    IntoElement, KeyDownEvent, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollDelta, ScrollWheelEvent,
    SharedString, StrikethroughStyle, Styled, TextAlign, TextRun, UTF16Selection, UnderlineStyle,
    Window, canvas, div, fill, point, px, size,
};

use super::frame::{self, Frame, TextStyle, Underline};
use super::keys::{self, Binding, KeyModes, KeyPress};
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

pub struct TerminalView {
    session: Entity<TerminalSession>,
    focus: FocusHandle,
    options: PaneOptions,
    layout: Rc<Cell<Option<Layout>>>,
    /// Text the input method is composing, shown at the cursor until committed.
    marked: Option<String>,
    selecting: bool,
    /// While set, the grid keeps its size, so the agent redraws once when the sidebar's
    /// divider is released rather than on every pixel of the drag.
    size_held: bool,
}

impl TerminalView {
    pub fn new(
        session: Entity<TerminalSession>,
        options: PaneOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
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
            selecting: false,
            size_held: false,
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
        let mode = self.mode(cx);
        let press = KeyPress {
            key: &keystroke.key,
            control: modifiers.control,
            alt: modifiers.alt,
            shift: modifiers.shift,
            platform: modifiers.platform,
        };
        let modes = KeyModes {
            app_cursor: mode.contains(TermMode::APP_CURSOR),
            alt_is_meta: self.options.alt_is_meta,
        };
        if let Some(bytes) = keys::encode(press, modes, &self.options.bindings) {
            self.session
                .update(cx, |session, cx| session.input(&bytes, cx));
            cx.stop_propagation();
        }
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
        if let Some(text) = self.session.read(cx).model().selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
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
        if self.mouse_reporting(cx) && !event.modifiers.shift {
            self.report_mouse(0, row, column, true, cx);
            return;
        }
        let kind = match event.click_count {
            2 => SelectionType::Semantic,
            3 => SelectionType::Lines,
            _ => SelectionType::Simple,
        };
        self.session.update(cx, |session, cx| {
            session.model_mut().start_selection(row, column, kind);
            cx.notify();
        });
        self.selecting = true;
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selecting || event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        if let Some((row, column)) = self.cell_at(event.position) {
            self.session.update(cx, |session, cx| {
                session.model_mut().extend_selection(row, column);
                cx.notify();
            });
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            self.selecting = false;
            return;
        }
        if self.mouse_reporting(cx)
            && let Some((row, column)) = self.cell_at(event.position)
        {
            self.report_mouse(0, row, column, false, cx);
        }
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
        let frame = frame::build(self.session.read(cx).model());
        let focused = self.focus.is_focused(window) && window.is_window_active();
        let ended = self.session.read(cx).ended.clone();
        let view = cx.entity();
        let session = self.session.clone();
        let focus = self.focus.clone();
        let layout = Rc::clone(&self.layout);
        let options = self.options.clone();
        let marked = self.marked.clone();
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
            .key_context("Terminal")
            .on_action(cx.listener(|this, _: &crate::menus::Copy, _, cx| this.copy_selection(cx)))
            .on_action(cx.listener(|this, _: &crate::menus::Paste, _, cx| this.paste_clipboard(cx)))
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
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
                        window.handle_input(&focus, ElementInputHandler::new(bounds, view), cx);
                    },
                )
                .size_full(),
            )
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
        Metrics {
            cell_width,
            // Line spacing as common terminals use it: about 1.2 times the font size.
            line_height: (font_size * 1.2).round(),
            font_size,
        }
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
        style: if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
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
        let cursor = frame::build(self.session.read(cx).model()).cursor?;
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
