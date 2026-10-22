//! The theme creator: a side panel that keeps the app in view, with four seeds, every
//! colour with its opacity, and the window's opacity and blur, each change applied to
//! the app at once (`openspec/changes/add-theme-creator/design.md`, D3, D4).

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use alacritty_terminal::vte::ansi::Rgb;
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Div, Entity, EventEmitter, FocusHandle,
    Focusable, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement, Pixels, Point, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Window, canvas, div, fill, linear_color_stop, linear_gradient, point, px,
    size,
};
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use hq_config::ColorSettings;
use hq_config::themes::ThemeFile;

use crate::terminal::view::hsla;
use crate::theme::{Chrome, Mode, Paint};
use crate::themes::{self, Library, Look, SEEDS, ThemeChoice, Token};

const WIDTH: f32 = 360.0;
const PICKER_HEIGHT: f32 = 150.0;
const BAR_HEIGHT: f32 = 12.0;
/// The window's background opacity the slider allows, as in the settings file.
const WINDOW_OPACITY_MIN: f32 = 0.3;

/// The parts of the panel a drag changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    Square,
    Hue,
    Opacity,
    Window,
}

/// Hue (0–360), saturation and value (0–1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
}

pub fn to_hsv(rgb: Rgb) -> Hsv {
    let (r, g, b) = (
        f32::from(rgb.r) / 255.0,
        f32::from(rgb.g) / 255.0,
        f32::from(rgb.b) / 255.0,
    );
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let range = high - low;
    let hue = if range == 0.0 {
        0.0
    } else if high == r {
        60.0 * ((g - b) / range).rem_euclid(6.0)
    } else if high == g {
        60.0 * ((b - r) / range + 2.0)
    } else {
        60.0 * ((r - g) / range + 4.0)
    };
    Hsv {
        hue,
        saturation: if high == 0.0 { 0.0 } else { range / high },
        value: high,
    }
}

pub fn from_hsv(hsv: Hsv) -> Rgb {
    let chroma = hsv.value * hsv.saturation;
    let sector = hsv.hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let lift = hsv.value - chroma;
    let channel = |value: f32| ((value + lift) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb {
        r: channel(r),
        g: channel(g),
        b: channel(b),
    }
}

pub fn hex_of(rgb: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b)
}

pub fn parse_hex(text: &str) -> Option<Rgb> {
    let digits = text.trim().trim_start_matches('#');
    (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
        .map(crate::theme::rgb)
}

/// What the creator is editing, kept apart from drawing so it can be tested.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub look: Look,
    /// The theme it started from, for Reset.
    pub start: Look,
    pub pinned: BTreeSet<Token>,
    pub dirty: bool,
}

impl Draft {
    pub fn new(start: Look) -> Draft {
        Draft {
            look: start.clone(),
            start,
            pinned: BTreeSet::new(),
            dirty: false,
        }
    }

    /// Sets a colour: a seed remakes the colours that are not pinned; any other colour
    /// is pinned.
    pub fn set(&mut self, token: Token, paint: Paint) {
        token.set(&mut self.look.theme, paint);
        if SEEDS.contains(&token) {
            themes::apply_seeds(&mut self.look.theme, &self.pinned);
        } else {
            self.pinned.insert(token);
        }
        self.dirty = true;
    }

    /// Puts a colour back as the starting theme had it and unpins it; colours made from
    /// seeds take the seeds' value again.
    pub fn reset(&mut self, token: Token) {
        self.pinned.remove(&token);
        token.set(&mut self.look.theme, token.get(&self.start.theme));
        if !matches!(token, Token::Terminal(_)) {
            themes::apply_seeds(&mut self.look.theme, &self.pinned);
        }
        self.dirty = true;
    }

    /// Starts again from another theme, keeping the window settings already set.
    pub fn start_from(&mut self, start: Look) {
        let window = (self.look.window_opacity, self.look.window_blur);
        self.look = start.clone();
        (self.look.window_opacity, self.look.window_blur) = window;
        self.start = start;
        self.pinned.clear();
        self.dirty = true;
    }

    pub fn set_window(&mut self, opacity: Option<f32>, blur: Option<bool>) {
        if let Some(opacity) = opacity {
            self.look.window_opacity = Some(opacity.clamp(WINDOW_OPACITY_MIN, 1.0));
        }
        if let Some(blur) = blur {
            self.look.window_blur = Some(blur);
        }
        self.dirty = true;
    }
}

pub enum CreatorEvent {
    /// Show this look on the app for now.
    Preview(Box<Look>),
    /// Save the theme, into `file` when editing one.
    Save {
        file: Option<String>,
        theme: Box<ThemeFile>,
    },
    Cancel,
}

/// What the workspace hands the creator when it opens.
pub struct CreatorStart {
    pub look: Look,
    pub from: ThemeChoice,
    pub name: String,
    pub editing: Option<String>,
    pub starts: Vec<(ThemeChoice, String)>,
    pub library: Library,
    pub system: Mode,
    pub colors: ColorSettings,
}

pub struct ThemeCreator {
    draft: Draft,
    /// What "Start from" offers, with names.
    starts: Vec<(ThemeChoice, String)>,
    start_from: ThemeChoice,
    start_open: bool,
    library: Library,
    system: Mode,
    colors: ColorSettings,
    /// The file being edited, if any.
    editing: Option<String>,
    selected: Token,
    hsv: Hsv,
    expanded: bool,
    confirm_discard: bool,
    error: Option<String>,
    drag: Option<Area>,
    bounds: Rc<RefCell<[Option<Bounds<Pixels>>; 4]>>,
    name: Entity<InputState>,
    hex: Entity<InputState>,
    percent: Entity<InputState>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    _inputs: Vec<Subscription>,
}

impl EventEmitter<CreatorEvent> for ThemeCreator {}

impl ThemeCreator {
    pub fn new(start: CreatorStart, window: &mut Window, cx: &mut Context<Self>) -> ThemeCreator {
        let selected = Token::Accent;
        let paint = selected.get(&start.look.theme);
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Theme name")
                .default_value(start.name.clone())
        });
        let hex = cx.new(|cx| InputState::new(window, cx).default_value(hex_of(paint.rgb)));
        let percent =
            cx.new(|cx| InputState::new(window, cx).default_value(paint.opacity.to_string()));
        let inputs = vec![
            cx.subscribe(
                &hex,
                |this: &mut ThemeCreator, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change)
                        && let Some(rgb) = parse_hex(&input.read(cx).value())
                    {
                        let paint = this.selected.get(&this.draft.look.theme);
                        this.hsv = to_hsv(rgb);
                        this.change(Paint { rgb, ..paint }, cx);
                    }
                },
            ),
            cx.subscribe(
                &percent,
                |this: &mut ThemeCreator, input, event: &InputEvent, cx| {
                    let text = input.read(cx).value();
                    if matches!(event, InputEvent::Change)
                        && let Ok(opacity) = text.trim().trim_end_matches('%').parse::<u8>()
                        && opacity <= 100
                    {
                        let paint = this.selected.get(&this.draft.look.theme);
                        this.change(paint.with_opacity(opacity), cx);
                    }
                },
            ),
            cx.subscribe(
                &name,
                |this: &mut ThemeCreator, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.draft.dirty = true;
                        this.error = None;
                        cx.notify();
                    }
                },
            ),
        ];
        window.focus(&name.focus_handle(cx), cx);
        ThemeCreator {
            hsv: to_hsv(paint.rgb),
            draft: Draft::new(start.look),
            starts: start.starts,
            start_from: start.from,
            start_open: false,
            library: start.library,
            system: start.system,
            colors: start.colors,
            editing: start.editing,
            selected,
            expanded: false,
            confirm_discard: false,
            error: None,
            drag: None,
            bounds: Rc::new(RefCell::new([None; 4])),
            name,
            hex,
            percent,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            _inputs: inputs,
        }
    }

    /// The theme could not be saved; the panel stays open and says why.
    pub fn failed(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        cx.notify();
    }

    fn chrome(&self) -> Chrome {
        self.draft.look.theme.chrome
    }

    fn preview(&mut self, cx: &mut Context<Self>) {
        self.confirm_discard = false;
        cx.emit(CreatorEvent::Preview(Box::new(self.draft.look.clone())));
        cx.notify();
    }

    fn change(&mut self, paint: Paint, cx: &mut Context<Self>) {
        self.draft.set(self.selected, paint);
        self.preview(cx);
    }

    /// Refreshes the hex and percent fields from the selected colour.
    fn sync_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paint = self.selected.get(&self.draft.look.theme);
        self.hex.update(cx, |input, cx| {
            input.set_value(hex_of(paint.rgb), window, cx)
        });
        self.percent.update(cx, |input, cx| {
            input.set_value(paint.opacity.to_string(), window, cx);
        });
    }

    fn select(&mut self, token: Token, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = token;
        self.hsv = to_hsv(token.get(&self.draft.look.theme).rgb);
        self.sync_fields(window, cx);
        cx.notify();
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.draft.reset(self.selected);
        self.hsv = to_hsv(self.selected.get(&self.draft.look.theme).rgb);
        self.sync_fields(window, cx);
        self.preview(cx);
    }

    fn start_from(&mut self, choice: ThemeChoice, window: &mut Window, cx: &mut Context<Self>) {
        let look = themes::look(Some(&choice), self.system, &self.colors, &self.library);
        self.start_from = choice;
        self.start_open = false;
        self.draft.start_from(look);
        self.hsv = to_hsv(self.selected.get(&self.draft.look.theme).rgb);
        self.sync_fields(window, cx);
        self.preview(cx);
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            self.error = Some("Give the theme a name".to_owned());
            cx.notify();
            return;
        }
        cx.emit(CreatorEvent::Save {
            file: self.editing.clone(),
            theme: Box::new(themes::to_file(&self.draft.look, &name)),
        });
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(CreatorEvent::Cancel);
    }

    /// Escape: asks first when anything changed.
    fn escape(&mut self, cx: &mut Context<Self>) {
        if self.start_open {
            self.start_open = false;
        } else if self.draft.dirty && !self.confirm_discard {
            self.confirm_discard = true;
        } else {
            self.cancel(cx);
            return;
        }
        cx.notify();
    }

    fn handle_key(&mut self, key: &str, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        if key == "escape" {
            self.escape(cx);
            return true;
        }
        false
    }

    // ---- dragging ---------------------------------------------------------------

    fn drag_to(
        &mut self,
        area: Area,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = self.bounds.borrow()[area as usize] else {
            return;
        };
        let x = ((f32::from(position.x) - f32::from(bounds.origin.x))
            / f32::from(bounds.size.width))
        .clamp(0.0, 1.0);
        let y = ((f32::from(position.y) - f32::from(bounds.origin.y))
            / f32::from(bounds.size.height))
        .clamp(0.0, 1.0);
        let paint = self.selected.get(&self.draft.look.theme);
        match area {
            Area::Square => {
                self.hsv.saturation = x;
                self.hsv.value = 1.0 - y;
                self.change(
                    Paint {
                        rgb: from_hsv(self.hsv),
                        ..paint
                    },
                    cx,
                );
            }
            Area::Hue => {
                self.hsv.hue = x * 359.0;
                self.change(
                    Paint {
                        rgb: from_hsv(self.hsv),
                        ..paint
                    },
                    cx,
                );
            }
            Area::Opacity => {
                let opacity = (x * 100.0).round() as u8;
                self.change(paint.with_opacity(opacity), cx);
            }
            Area::Window => {
                let opacity = WINDOW_OPACITY_MIN + x * (1.0 - WINDOW_OPACITY_MIN);
                self.draft
                    .set_window(Some((opacity * 100.0).round() / 100.0), None);
                self.preview(cx);
                return;
            }
        }
        self.sync_fields(window, cx);
    }

    fn start_drag(
        &mut self,
        area: Area,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.drag = Some(area);
        self.drag_to(area, event.position, window, cx);
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(area) = self.drag {
            if event.pressed_button == Some(MouseButton::Left) {
                self.drag_to(area, event.position, window, cx);
            } else {
                self.drag = None;
            }
        }
    }

    // ---- drawing ----------------------------------------------------------------

    /// An element that records where it is drawn, for turning pointer positions into
    /// values.
    fn measured(&self, area: Area) -> impl IntoElement {
        let bounds = Rc::clone(&self.bounds);
        canvas(
            move |drawn, _, _| bounds.borrow_mut()[area as usize] = Some(drawn),
            |_, (), _, _| {},
        )
        .absolute()
        .size_full()
    }

    fn heading(&self, text: &'static str) -> Div {
        div()
            .px(px(14.))
            .pt(px(12.))
            .pb(px(4.))
            .text_size(px(11.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(hsla(self.chrome().muted))
            .child(text)
    }

    fn swatch(&self, paint: Paint, side: f32) -> Div {
        let chrome = self.chrome();
        div()
            .size(px(side))
            .flex_none()
            .relative()
            .rounded(px(5.))
            .overflow_hidden()
            .border_1()
            .border_color(hsla(chrome.track))
            .child(checker(6.0))
            .child(div().absolute().size_full().bg(hsla(paint)))
    }

    fn render_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome();
        let paint = self.selected.get(&self.draft.look.theme);
        let pure = hsla(from_hsv(Hsv {
            hue: self.hsv.hue,
            saturation: 1.0,
            value: 1.0,
        }));
        let mut clear_white: Hsla = gpui::white();
        clear_white.a = 0.0;
        let mut clear_black: Hsla = gpui::black();
        clear_black.a = 0.0;
        let solid = hsla(paint.rgb);
        let mut clear = solid;
        clear.a = 0.0;
        let square = div()
            .id("picker-square")
            .relative()
            .w_full()
            .h(px(PICKER_HEIGHT))
            .rounded(px(8.))
            .overflow_hidden()
            .cursor_crosshair()
            .bg(pure)
            .child(div().absolute().size_full().bg(linear_gradient(
                90.,
                linear_color_stop(gpui::white(), 0.),
                linear_color_stop(clear_white, 1.),
            )))
            .child(div().absolute().size_full().bg(linear_gradient(
                180.,
                linear_color_stop(clear_black, 0.),
                linear_color_stop(gpui::black(), 1.),
            )))
            .child(self.measured(Area::Square))
            .child(marker(self.hsv.saturation, Some(1.0 - self.hsv.value)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.start_drag(Area::Square, event, window, cx);
                }),
            );
        let hue = div()
            .id("picker-hue")
            .relative()
            .w_full()
            .h(px(BAR_HEIGHT))
            .rounded(px(6.))
            .overflow_hidden()
            .cursor_pointer()
            .child(rainbow())
            .child(self.measured(Area::Hue))
            .child(marker(self.hsv.hue / 359.0, None))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.start_drag(Area::Hue, event, window, cx);
                }),
            );
        let opacity = div()
            .id("picker-opacity")
            .relative()
            .w_full()
            .h(px(BAR_HEIGHT))
            .rounded(px(6.))
            .overflow_hidden()
            .cursor_pointer()
            .child(checker(6.0))
            .child(div().absolute().size_full().bg(linear_gradient(
                90.,
                linear_color_stop(clear, 0.),
                linear_color_stop(solid, 1.),
            )))
            .child(self.measured(Area::Opacity))
            .child(marker(paint.alpha(), None))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.start_drag(Area::Opacity, event, window, cx);
                }),
            );
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .px(px(14.))
            .py(px(6.))
            .child(square)
            .child(hue)
            .child(opacity)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(self.swatch(paint, 24.))
                    .child(div().w(px(96.)).child(Input::new(&self.hex).small()))
                    .child(div().w(px(56.)).child(Input::new(&self.percent).small()))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(hsla(chrome.muted))
                            .child("%"),
                    )
                    .child(
                        div()
                            .id("picker-reset")
                            .ml_auto()
                            .text_size(px(12.5))
                            .text_color(hsla(chrome.accent))
                            .cursor_pointer()
                            .child("Reset")
                            .on_click(cx.listener(|this, _, window, cx| this.reset(window, cx))),
                    ),
            )
    }

    fn render_seeds(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome();
        let names = ["Background", "Text", "Accent", "Needs you"];
        let mut row = div().flex().gap(px(6.)).px(px(14.)).py(px(4.));
        for (token, name) in SEEDS.into_iter().zip(names) {
            let chosen = self.selected == token;
            row = row.child(
                div()
                    .id(SharedString::from(format!("seed-{name}")))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(5.))
                    .py(px(8.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(hsla(if chosen {
                        chrome.accent
                    } else {
                        chrome.sidebar_line
                    }))
                    .text_size(px(12.))
                    .text_color(hsla(chrome.text))
                    .cursor_pointer()
                    .child(self.swatch(token.get(&self.draft.look.theme), 28.))
                    .child(name)
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.select(token, window, cx)),
                    ),
            );
        }
        row
    }

    fn render_window(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome();
        let opacity = self.draft.look.window_opacity.unwrap_or(1.0);
        let blur = self.draft.look.window_blur.unwrap_or(false);
        let position = (opacity - WINDOW_OPACITY_MIN) / (1.0 - WINDOW_OPACITY_MIN);
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .px(px(14.))
            .py(px(4.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .text_color(hsla(chrome.text))
                            .child(format!(
                                "Background opacity · {}%",
                                (opacity * 100.).round()
                            )),
                    )
                    .child(
                        div()
                            .id("window-opacity")
                            .relative()
                            .w(px(130.))
                            .h(px(BAR_HEIGHT))
                            .rounded(px(6.))
                            .bg(hsla(chrome.track))
                            .cursor_pointer()
                            .child(
                                div()
                                    .absolute()
                                    .h_full()
                                    .w(gpui::relative(position))
                                    .rounded(px(6.))
                                    .bg(hsla(chrome.accent)),
                            )
                            .child(self.measured(Area::Window))
                            .child(marker(position, None))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                                    this.start_drag(Area::Window, event, window, cx);
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .text_size(px(13.))
                            .text_color(hsla(chrome.text))
                            .child("Blur behind the window")
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(hsla(chrome.muted))
                                    .child("On or off; macOS has no blur strength to set"),
                            ),
                    )
                    .child(
                        div()
                            .id("window-blur")
                            .w(px(36.))
                            .h(px(20.))
                            .flex_none()
                            .relative()
                            .rounded_full()
                            .bg(hsla(if blur { chrome.accent } else { chrome.track }))
                            .cursor_pointer()
                            .child(
                                div()
                                    .absolute()
                                    .top(px(2.))
                                    .left(px(if blur { 18. } else { 2. }))
                                    .size(px(16.))
                                    .rounded_full()
                                    .bg(hsla(chrome.text)),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.draft.set_window(None, Some(!blur));
                                this.preview(cx);
                            })),
                    ),
            )
    }

    fn render_token_row(&self, token: Token, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.chrome();
        let paint = token.get(&self.draft.look.theme);
        let chosen = self.selected == token;
        let pinned = self.draft.pinned.contains(&token);
        let value = if paint.opacity < 100 {
            format!("{} · {}%", hex_of(paint.rgb), paint.opacity)
        } else {
            hex_of(paint.rgb)
        };
        let mut row = div()
            .id(SharedString::from(format!("token-{}", token.file_key())))
            .flex()
            .items_center()
            .gap(px(10.))
            .mx(px(6.))
            .px(px(10.))
            .min_h(px(34.))
            .rounded(px(8.))
            .cursor_pointer()
            .child(self.swatch(paint, 20.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .text_color(hsla(chrome.text))
                    .child(if pinned {
                        format!("● {}", token.label())
                    } else {
                        token.label()
                    }),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(hsla(chrome.muted))
                    .child(value),
            )
            .on_click(cx.listener(move |this, _, window, cx| this.select(token, window, cx)));
        if chosen {
            row = row.bg(super::tint(chrome, 0.16));
        }
        let mut block = div().flex().flex_col().child(row);
        if chosen && !SEEDS.contains(&token) {
            block = block.child(
                div()
                    .px(px(6.))
                    .child(
                        div()
                            .px(px(14.))
                            .pb(px(2.))
                            .text_size(px(11.5))
                            .text_color(hsla(chrome.muted))
                            .child(token.use_text()),
                    )
                    .child(self.render_picker(cx)),
            );
        }
        block.into_any_element()
    }

    fn render_every_colour(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().flex().flex_col();
        let mut group = "";
        for token in Token::all() {
            if matches!(token, Token::Terminal(_)) {
                continue;
            }
            if token.group() != group {
                group = token.group();
                list = list.child(self.heading(match group {
                    "Status" => "STATUS",
                    "Terminal" => "TERMINAL",
                    _ => "INTERFACE",
                }));
            }
            list = list.child(self.render_token_row(token, cx));
        }
        let chrome = self.chrome();
        let mut grid = div().flex().flex_wrap().gap(px(6.)).px(px(14.)).py(px(4.));
        for index in 0..16u8 {
            let token = Token::Terminal(index);
            let chosen = self.selected == token;
            grid = grid.child(
                div()
                    .id(SharedString::from(format!("terminal-{index}")))
                    .w(px(36.))
                    .h(px(26.))
                    .rounded(px(6.))
                    .border_2()
                    .border_color(if chosen {
                        hsla(chrome.accent)
                    } else {
                        gpui::transparent_black()
                    })
                    .cursor_pointer()
                    .child(
                        self.swatch(token.get(&self.draft.look.theme), 22.)
                            .size_full(),
                    )
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.select(token, window, cx)),
                    ),
            );
        }
        let terminal_selected = matches!(self.selected, Token::Terminal(_));
        list.child(self.heading("TERMINAL COLOURS"))
            .child(grid)
            .child(
                div()
                    .px(px(14.))
                    .pb(px(6.))
                    .text_size(px(11.5))
                    .text_color(hsla(chrome.muted))
                    .child("The 16 colours programs ask for by name (red for errors, green for added lines). Top row normal, bottom row bright. Agents that draw their own exact colours ignore them."),
            )
            .children(terminal_selected.then(|| {
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(14.))
                            .text_size(px(13.))
                            .text_color(hsla(chrome.text))
                            .child(self.selected.label()),
                    )
                    .child(self.render_picker(cx))
            }))
    }

    fn render_start(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome();
        let current = self
            .starts
            .iter()
            .find(|(choice, _)| *choice == self.start_from)
            .map_or_else(String::new, |(_, name)| name.clone());
        let mut block = div().flex().flex_col().gap(px(4.)).px(px(14.)).child(
            div()
                .id("start-from")
                .flex()
                .items_center()
                .gap(px(8.))
                .h(px(32.))
                .px(px(10.))
                .rounded(px(8.))
                .border_1()
                .border_color(hsla(chrome.sidebar_line))
                .bg(hsla(chrome.sidebar))
                .cursor_pointer()
                .text_size(px(13.))
                .text_color(hsla(chrome.text))
                .child(div().text_color(hsla(chrome.muted)).child("Start from"))
                .child(div().flex_1().truncate().child(current))
                .child(
                    div()
                        .text_color(hsla(chrome.muted))
                        .child(if self.start_open { "▴" } else { "▾" }),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.start_open = !this.start_open;
                    cx.notify();
                })),
        );
        if self.start_open {
            let mut list = div()
                .id("start-list")
                .max_h(px(220.))
                .overflow_y_scroll()
                .rounded(px(8.))
                .border_1()
                .border_color(hsla(chrome.sidebar_line))
                .bg(hsla(chrome.sidebar))
                .py(px(4.));
            for (index, (choice, name)) in self.starts.iter().enumerate() {
                let choice = choice.clone();
                let chosen = choice == self.start_from;
                let mut item = div()
                    .id(SharedString::from(format!("start-{index}")))
                    .px(px(10.))
                    .py(px(6.))
                    .text_size(px(13.))
                    .text_color(hsla(chrome.text))
                    .cursor_pointer()
                    .child(name.clone())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.start_from(choice.clone(), window, cx);
                    }));
                if chosen {
                    item = item.bg(super::tint(chrome, 0.16));
                }
                list = list.child(item);
            }
            block = block.child(list);
        }
        block
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome();
        let note: Option<(String, bool)> = if self.confirm_discard {
            Some(("Discard changes?".to_owned(), false))
        } else {
            self.error.clone().map(|error| (error, true))
        };
        let mut footer = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(14.))
            .py(px(12.))
            .border_t_1()
            .border_color(hsla(chrome.sidebar_line));
        if let Some((text, error)) = note {
            footer = footer.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(hsla(if error { chrome.amber } else { chrome.text }))
                    .child(text),
            );
        } else {
            footer = footer.child(div().flex_1());
        }
        if self.confirm_discard {
            footer
                .child(
                    Button::new("creator-keep-editing")
                        .label("Keep editing")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.confirm_discard = false;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("creator-discard")
                        .primary()
                        .label("Discard")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                )
        } else {
            footer
                .child(
                    Button::new("creator-cancel")
                        .label("Cancel")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                )
                .child(
                    Button::new("creator-save")
                        .primary()
                        .label(if self.editing.is_some() {
                            "Save changes"
                        } else {
                            "Save theme"
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                )
        }
    }
}

/// A two-tone checkerboard, so see-through colours look see-through.
fn checker(square: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let light = hsla(crate::theme::rgb(0xD4D7DD));
            let dark = hsla(crate::theme::rgb(0x8A8F99));
            window.paint_quad(fill(bounds, light));
            let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let mut y = 0.0;
            let mut row = 0;
            while y < height {
                let mut x = if row % 2 == 0 { 0.0 } else { square };
                while x < width {
                    let cell = Bounds::new(
                        point(bounds.origin.x + px(x), bounds.origin.y + px(y)),
                        size(px(square.min(width - x)), px(square.min(height - y))),
                    );
                    window.paint_quad(fill(cell, dark));
                    x += square * 2.0;
                }
                y += square;
                row += 1;
            }
        },
    )
    .absolute()
    .size_full()
}

/// The hue bar: every hue from left to right.
fn rainbow() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, (), window, _| {
            let width = f32::from(bounds.size.width);
            let slices = 60;
            for slice in 0..slices {
                let from = width * slice as f32 / slices as f32;
                let to = width * (slice + 1) as f32 / slices as f32;
                let color = hsla(from_hsv(Hsv {
                    hue: 360.0 * slice as f32 / slices as f32,
                    saturation: 1.0,
                    value: 1.0,
                }));
                let cell = Bounds::new(
                    point(bounds.origin.x + px(from), bounds.origin.y),
                    size(px(to - from + 0.5), bounds.size.height),
                );
                window.paint_quad(fill(cell, color));
            }
        },
    )
    .absolute()
    .size_full()
}

/// The round handle on a picker area or bar, at `x` (and `y`) from 0 to 1.
fn marker(x: f32, y: Option<f32>) -> impl IntoElement {
    let handle = div()
        .absolute()
        .size(px(12.))
        .rounded_full()
        .border_2()
        .border_color(gpui::white())
        .shadow_sm()
        .left(gpui::relative(x.clamp(0.0, 1.0)))
        .ml(px(-6.));
    match y {
        Some(y) => handle.top(gpui::relative(y.clamp(0.0, 1.0))).mt(px(-6.)),
        None => handle.top(px(0.)),
    }
}

impl Focusable for ThemeCreator {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ThemeCreator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome();
        let seed_selected = SEEDS.contains(&self.selected);
        let made = {
            let mut strip = div().flex().flex_wrap().gap(px(4.)).px(px(14.)).py(px(4.));
            for token in Token::all() {
                strip = strip.child(self.swatch(token.get(&self.draft.look.theme), 18.));
            }
            strip
        };
        let panel = div()
            .id("theme-creator-panel")
            .w(px(WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(hsla(chrome.selected))
            .border_l_1()
            .border_color(hsla(chrome.sidebar_line))
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(14.))
                    .pt(px(16.))
                    .pb(px(8.))
                    .text_size(px(11.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(hsla(chrome.muted))
                    .child(
                        div()
                            .id("creator-back")
                            .text_size(px(16.))
                            .cursor_pointer()
                            .child("←")
                            .on_click(cx.listener(|this, _, _, cx| this.escape(cx))),
                    )
                    .child(if self.editing.is_some() {
                        "EDIT THEME"
                    } else {
                        "NEW THEME"
                    }),
            )
            .child(
                div()
                    .id("creator-body")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().px(px(14.)).pb(px(6.)).child(Input::new(&self.name)))
                    .child(self.render_start(cx))
                    .child(self.heading("SEEDS"))
                    .child(self.render_seeds(cx))
                    .children(seed_selected.then(|| self.render_picker(cx)))
                    .child(self.heading("MADE FROM THESE"))
                    .child(made)
                    .child(self.heading("WINDOW"))
                    .child(self.render_window(cx))
                    .child(
                        div()
                            .id("creator-expand")
                            .px(px(14.))
                            .pt(px(14.))
                            .pb(px(6.))
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(hsla(chrome.muted))
                            .cursor_pointer()
                            .child(if self.expanded {
                                "▾ ADJUST EVERY COLOUR"
                            } else {
                                "▸ ADJUST EVERY COLOUR"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.expanded = !this.expanded;
                                cx.notify();
                            })),
                    )
                    .children(self.expanded.then(|| self.render_every_colour(cx)))
                    .child(div().h(px(12.))),
            )
            .child(self.render_footer(cx));
        let layer = div()
            .id("theme-creator")
            .key_context("ThemeCreator")
            .track_focus(&self.focus)
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_end()
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.drag = None;
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if this.handle_key(&event.keystroke.key, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(panel);
        super::route_field_keys(layer, Self::handle_key, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Theme, rgb};

    fn draft() -> Draft {
        Draft::new(Look {
            theme: Theme::dark(),
            window_opacity: Some(0.9),
            window_blur: Some(false),
        })
    }

    #[test]
    fn colours_round_trip_through_hsv_and_hex() {
        for hex in [0x000000, 0xFFFFFF, 0x89B4FA, 0xF38BA8, 0x2E3440, 0x7F7F7F] {
            let color = rgb(hex);
            assert_eq!(from_hsv(to_hsv(color)), color, "{hex:06X}");
            assert_eq!(parse_hex(&hex_of(color)), Some(color));
        }
        assert_eq!(parse_hex("89b4fa"), Some(rgb(0x89B4FA)));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("zzzzzz"), None);
    }

    #[test]
    fn a_seed_remakes_and_a_colour_set_by_hand_is_pinned() {
        let mut draft = draft();
        draft.set(Token::Lines, Paint::solid(rgb(0xFF0000)));
        assert!(draft.pinned.contains(&Token::Lines));
        draft.set(Token::Sidebar, Paint::solid(rgb(0xFDF6E3)));
        assert_eq!(Token::Lines.get(&draft.look.theme).rgb, rgb(0xFF0000));
        assert_eq!(Token::Background.get(&draft.look.theme).rgb, rgb(0xFDF6E3));
        assert!(
            !draft.pinned.contains(&Token::Sidebar),
            "seeds are never pinned"
        );
        assert!(draft.dirty);

        draft.reset(Token::Lines);
        assert!(!draft.pinned.contains(&Token::Lines));
        assert_ne!(
            Token::Lines.get(&draft.look.theme).rgb,
            rgb(0xFF0000),
            "reset takes the seeds' value again"
        );
    }

    #[test]
    fn starting_from_another_theme_keeps_the_window_settings() {
        let mut draft = draft();
        draft.set_window(Some(0.1), Some(true));
        assert_eq!(draft.look.window_opacity, Some(WINDOW_OPACITY_MIN));
        draft.set(Token::Accent, Paint::solid(rgb(0xFAB387)));
        draft.start_from(Look {
            theme: Theme::light(),
            window_opacity: None,
            window_blur: None,
        });
        assert_eq!(draft.look.theme.chrome, Theme::light().chrome);
        assert_eq!(draft.look.window_blur, Some(true));
        assert!(draft.pinned.is_empty());
    }
}
