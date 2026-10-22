//! The theme switcher (⌘G): the built-in themes, the templates, the settings file's
//! colours and the owner's themes, each applied as the owner moves to it, with
//! "+ Add theme", Duplicate, Edit and Delete
//! (`openspec/changes/add-theme-creator/design.md`, D2).

use gpui::{
    App, Context, Div, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use hq_config::ColorSettings;

use crate::terminal::view::hsla;
use crate::theme::{self, Chrome, Mode, Paint};
use crate::themes::{self, Library, ThemeChoice};

const WIDTH: f32 = 380.0;
const ROW: f32 = 48.0;
/// The list scrolls past this height.
const LIST_HEIGHT: f32 = 520.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    /// Built-in, template or Alacritty: can be duplicated, not changed.
    Fixed,
    /// One of the owner's theme files.
    Yours(String),
    /// A theme file that cannot be read; shown with why, never changed by the app.
    Broken(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeRow {
    pub choice: ThemeChoice,
    pub name: String,
    /// The heading this row starts under, when it starts a section.
    pub section: Option<&'static str>,
    /// The theme's background, text and accent.
    pub swatches: [Paint; 3],
    pub detail: Option<String>,
    pub available: bool,
    pub kind: RowKind,
}

/// The switcher's rows: System, Light, Dark; the templates; Alacritty; the owner's
/// themes. `source` names the file the settings file's colours come from.
pub fn rows(
    system: Mode,
    colors: &ColorSettings,
    source: &str,
    library: &Library,
) -> Vec<ThemeRow> {
    let swatches = |choice: &ThemeChoice| {
        let look = themes::look(Some(choice), system, colors, library).theme;
        [
            look.palette.background.into(),
            look.chrome.text,
            look.chrome.accent,
        ]
    };
    let mut rows = Vec::new();
    for (choice, name) in [
        (ThemeChoice::System, "System"),
        (ThemeChoice::Light, "Light"),
        (ThemeChoice::Dark, "Dark"),
    ] {
        rows.push(ThemeRow {
            swatches: swatches(&choice),
            choice,
            name: name.to_owned(),
            section: None,
            detail: None,
            available: true,
            kind: RowKind::Fixed,
        });
    }
    for (index, (id, file)) in library.templates.iter().enumerate() {
        let choice = ThemeChoice::Template(id.clone());
        rows.push(ThemeRow {
            swatches: swatches(&choice),
            choice,
            name: file.name.clone(),
            section: (index == 0).then_some("Templates"),
            detail: file.credit.clone(),
            available: true,
            kind: RowKind::Fixed,
        });
    }
    let has_colours = !colors.is_empty();
    rows.push(ThemeRow {
        swatches: swatches(&ThemeChoice::Alacritty),
        choice: ThemeChoice::Alacritty,
        name: "Alacritty".to_owned(),
        section: Some("From Alacritty"),
        detail: Some(if has_colours {
            format!("From {source}")
        } else {
            "No colours in your settings file".to_owned()
        }),
        available: has_colours,
        kind: RowKind::Fixed,
    });
    for (index, listed) in library.files.iter().enumerate() {
        let choice = ThemeChoice::File(listed.file.clone());
        let section = (index == 0).then_some("Yours");
        rows.push(match &listed.theme {
            Ok(file) => ThemeRow {
                swatches: swatches(&choice),
                choice,
                name: file.name.clone(),
                section,
                detail: None,
                available: true,
                kind: RowKind::Yours(listed.file.clone()),
            },
            Err(error) => ThemeRow {
                swatches: [Paint::solid(theme::rgb(0x808080)); 3],
                choice,
                name: listed.file.clone(),
                section,
                detail: Some(format!("Cannot be read: {error}")),
                available: false,
                kind: RowKind::Broken(listed.file.clone()),
            },
        });
    }
    rows
}

/// Where row `index` sits among the list's children, which include the section headings.
pub fn list_position(rows: &[ThemeRow], index: usize) -> usize {
    index
        + rows[..=index.min(rows.len().saturating_sub(1))]
            .iter()
            .filter(|row| row.section.is_some())
            .count()
}

/// The next row in `step`'s direction that can be chosen, or `from` at either end.
pub fn step(rows: &[ThemeRow], from: usize, step: isize) -> usize {
    let mut index = from;
    loop {
        let Some(next) = index
            .checked_add_signed(step)
            .filter(|next| *next < rows.len())
        else {
            return from;
        };
        if rows[next].available {
            return next;
        }
        index = next;
    }
}

pub enum ThemeEvent {
    /// Apply this theme for now.
    Preview(ThemeChoice),
    /// Keep this theme and close.
    Keep(ThemeChoice),
    /// Go back to the theme in use when the switcher opened, and close.
    Cancel,
    /// Open the creator starting from this theme; `name` is the new theme's name.
    Create { from: ThemeChoice, name: String },
    /// Open the creator on one of the owner's files.
    Edit(String),
    /// Remove one of the owner's files; the owner has confirmed.
    Delete(String),
}

pub struct ThemeSwitcher {
    rows: Vec<ThemeRow>,
    index: usize,
    /// The theme in use when the switcher opened.
    current: ThemeChoice,
    hovered: Option<usize>,
    confirm_delete: Option<usize>,
    chrome: Chrome,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

impl EventEmitter<ThemeEvent> for ThemeSwitcher {}

impl ThemeSwitcher {
    pub fn new(
        rows: Vec<ThemeRow>,
        current: ThemeChoice,
        chrome: Chrome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ThemeSwitcher {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let index = rows
            .iter()
            .position(|row| row.choice == current)
            .unwrap_or(0);
        let scroll = ScrollHandle::new();
        // The list has no layout until it is drawn once; scroll to the theme in use then.
        let position = list_position(&rows, index);
        cx.on_next_frame(window, move |this: &mut ThemeSwitcher, _, cx| {
            this.scroll.scroll_to_item(position);
            cx.notify();
        });
        ThemeSwitcher {
            rows,
            index,
            current,
            hovered: None,
            confirm_delete: None,
            chrome,
            focus,
            scroll,
        }
    }

    pub fn set_chrome(&mut self, chrome: Chrome, cx: &mut Context<Self>) {
        self.chrome = chrome;
        cx.notify();
    }

    fn move_to(&mut self, index: usize, cx: &mut Context<Self>) {
        if index != self.index {
            self.index = index;
            self.scroll.scroll_to_item(list_position(&self.rows, index));
            cx.emit(ThemeEvent::Preview(self.rows[index].choice.clone()));
            cx.notify();
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        match event.keystroke.key.as_str() {
            "down" => self.move_to(step(&self.rows, self.index, 1), cx),
            "up" => self.move_to(step(&self.rows, self.index, -1), cx),
            "enter" => cx.emit(ThemeEvent::Keep(self.rows[self.index].choice.clone())),
            "escape" if self.confirm_delete.is_some() => {
                self.confirm_delete = None;
                cx.notify();
            }
            "escape" => cx.emit(ThemeEvent::Cancel),
            // The pointer's "+ Add theme", Duplicate and Edit, for the keyboard.
            "n" => cx.emit(ThemeEvent::Create {
                from: self.rows[self.index].choice.clone(),
                name: "My theme".to_owned(),
            }),
            "d" if self.rows[self.index].kind == RowKind::Fixed => {
                let row = &self.rows[self.index];
                cx.emit(ThemeEvent::Create {
                    from: row.choice.clone(),
                    name: format!("{} copy", row.name),
                });
            }
            "e" => {
                if let RowKind::Yours(file) = &self.rows[self.index].kind {
                    cx.emit(ThemeEvent::Edit(file.clone()));
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    /// A small outlined button on a row; it never chooses the row itself.
    fn action(
        &self,
        id: String,
        label: &'static str,
        cx: &mut Context<Self>,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let chrome = self.chrome;
        div()
            .id(SharedString::from(id))
            .px(px(7.))
            .rounded(px(6.))
            .border_1()
            .border_color(hsla(chrome.track))
            .text_size(px(11.5))
            .line_height(px(18.))
            .text_color(hsla(chrome.muted))
            .cursor_pointer()
            .hover(move |style| style.text_color(hsla(chrome.text)))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                on_press(this, cx);
            }))
            .child(label)
    }

    fn render_actions(&self, index: usize, row: &ThemeRow, cx: &mut Context<Self>) -> Div {
        match &row.kind {
            RowKind::Fixed if row.available => {
                let from = row.choice.clone();
                let name = format!("{} copy", row.name);
                div().flex().gap(px(4.)).child(self.action(
                    format!("duplicate-{index}"),
                    "Duplicate",
                    cx,
                    move |_, cx| {
                        cx.emit(ThemeEvent::Create {
                            from: from.clone(),
                            name: name.clone(),
                        });
                    },
                ))
            }
            RowKind::Yours(file) => {
                let file = file.clone();
                div()
                    .flex()
                    .gap(px(4.))
                    .child(
                        self.action(format!("edit-{index}"), "Edit", cx, move |_, cx| {
                            cx.emit(ThemeEvent::Edit(file.clone()));
                        }),
                    )
                    .child(
                        self.action(format!("delete-{index}"), "✕", cx, move |this, cx| {
                            this.confirm_delete = Some(index);
                            cx.notify();
                        }),
                    )
            }
            RowKind::Fixed | RowKind::Broken(_) => div(),
        }
    }

    fn render_row(&self, index: usize, row: &ThemeRow, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        let swatch = |color: Paint| -> Div {
            div()
                .size(px(14.))
                .rounded_full()
                .border_1()
                .border_color(hsla(chrome.track))
                .bg(hsla(color))
        };
        let choice = row.choice.clone();
        let actions = (self.hovered == Some(index)).then(|| self.render_actions(index, row, cx));
        let mut line = div()
            .id(SharedString::from(format!("theme-row-{index}")))
            .h(px(ROW))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(12.))
            .mx(px(8.))
            .rounded(px(10.))
            .child(
                div()
                    .flex()
                    .gap(px(4.))
                    .children(row.swatches.into_iter().map(swatch)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .text_size(px(14.))
                            .text_color(hsla(chrome.text))
                            .child(row.name.clone()),
                    )
                    .children(row.detail.clone().map(|detail| {
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(hsla(chrome.muted))
                            .child(detail)
                    })),
            )
            .children(actions)
            .children(
                (row.choice == self.current)
                    .then(|| div().flex_none().text_color(hsla(chrome.accent)).child("✓")),
            )
            .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                if *over {
                    this.hovered = Some(index);
                } else if this.hovered == Some(index) {
                    this.hovered = None;
                }
                cx.notify();
            }));
        if index == self.index {
            line = line.bg(super::tint(chrome, 0.16));
        }
        if row.available {
            let hover = super::tint(chrome, if index == self.index { 0.2 } else { 0.06 });
            line = line
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(
                    cx.listener(move |_, _, _, cx| cx.emit(ThemeEvent::Keep(choice.clone()))),
                );
        } else {
            line = line.opacity(0.55);
        }
        line
    }

    fn render_confirm(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let index = self.confirm_delete?;
        let RowKind::Yours(file) = &self.rows.get(index)?.kind else {
            return None;
        };
        let chrome = self.chrome;
        let file = file.clone();
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .mx(px(8.))
                .px(px(12.))
                .py(px(8.))
                .rounded(px(10.))
                .bg(super::tint(chrome, 0.1))
                .text_size(px(13.))
                .text_color(hsla(chrome.text))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(format!("Delete “{}”?", self.rows[index].name)),
                )
                .child(
                    self.action("delete-yes".to_owned(), "Delete", cx, move |this, cx| {
                        this.confirm_delete = None;
                        cx.emit(ThemeEvent::Delete(file.clone()));
                    }),
                )
                .child(self.action("delete-no".to_owned(), "Keep", cx, |this, cx| {
                    this.confirm_delete = None;
                    cx.notify();
                })),
        )
    }
}

impl Focusable for ThemeSwitcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ThemeSwitcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        let mut list = div()
            .id("theme-list")
            .max_h(px(LIST_HEIGHT))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .gap(px(2.));
        for (index, row) in self.rows.clone().iter().enumerate() {
            if let Some(section) = row.section {
                list = list.child(
                    div()
                        .px(px(20.))
                        .pt(px(10.))
                        .pb(px(2.))
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(hsla(chrome.muted))
                        .child(section.to_uppercase()),
                );
            }
            list = list.child(self.render_row(index, row, cx));
        }
        let from = self.rows[self.index].choice.clone();
        div()
            .id("theme-switcher")
            .key_context("ThemeSwitcher")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            // A click outside counts as Escape.
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(ThemeEvent::Cancel)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(px(WIDTH))
            .flex()
            .flex_col()
            .gap(px(2.))
            .py(px(10.))
            .rounded(px(14.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.selected))
            .shadow_lg()
            .child(
                div()
                    .px(px(20.))
                    .pt(px(4.))
                    .pb(px(6.))
                    .text_size(px(11.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(hsla(chrome.muted))
                    .child("THEME"),
            )
            .child(list)
            .children(self.render_confirm(cx))
            .child(
                div()
                    .id("add-theme")
                    .mx(px(8.))
                    .mt(px(6.))
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .rounded(px(10.))
                    .border_1()
                    .border_dashed()
                    .border_color(hsla(chrome.track))
                    .text_color(hsla(chrome.muted))
                    .cursor_pointer()
                    .hover(move |style| style.text_color(hsla(chrome.text)))
                    .child("＋  Add theme")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.emit(ThemeEvent::Create {
                            from: from.clone(),
                            name: "My theme".to_owned(),
                        });
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use hq_config::themes::Listed;

    use super::*;

    fn colours() -> ColorSettings {
        ColorSettings {
            background: Some(0x1E1E2E),
            foreground: Some(0xCDD6F4),
            ..ColorSettings::default()
        }
    }

    fn library_with_files() -> Library {
        let mut library = Library::load(None);
        let mine = hq_config::themes::parse(
            "[colors.primary]\nbackground = \"#101010\"\n[headquarter]\nname = \"Mine\"\n",
        )
        .unwrap();
        library.files = vec![
            Listed {
                file: "mine.toml".to_owned(),
                theme: Ok(mine),
            },
            Listed {
                file: "broken.toml".to_owned(),
                theme: Err("expected a value".to_owned()),
            },
        ];
        library
    }

    #[test]
    fn sections_in_order_with_credits() {
        let rows = rows(Mode::Light, &colours(), "mocha.toml", &library_with_files());
        let names: Vec<_> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "System",
                "Light",
                "Dark",
                "Catppuccin Mocha",
                "Dracula",
                "Nord",
                "Gruvbox Dark",
                "Tokyo Night",
                "Catppuccin Latte",
                "Solarized Light",
                "Gruvbox Light",
                "Alacritty",
                "Mine",
                "broken.toml",
            ]
        );
        let sections: Vec<_> = rows.iter().filter_map(|row| row.section).collect();
        assert_eq!(sections, ["Templates", "From Alacritty", "Yours"]);
        assert_eq!(rows[4].detail.as_deref(), Some("Dracula · MIT"));
        assert_eq!(rows[11].detail.as_deref(), Some("From mocha.toml"));
        assert_eq!(rows[12].kind, RowKind::Yours("mine.toml".to_owned()));
        assert_eq!(rows[12].swatches[0].rgb, theme::rgb(0x101010));
        assert!(!rows[13].available);
        assert!(
            rows[13]
                .detail
                .as_deref()
                .unwrap()
                .starts_with("Cannot be read")
        );
    }

    #[test]
    fn without_colours_alacritty_is_greyed_and_skipped() {
        let rows = rows(
            Mode::Dark,
            &ColorSettings::default(),
            "settings file",
            &Library::load(None),
        );
        let alacritty = rows
            .iter()
            .position(|row| row.choice == ThemeChoice::Alacritty)
            .unwrap();
        assert!(!rows[alacritty].available);
        assert_eq!(
            rows[alacritty].detail.as_deref(),
            Some("No colours in your settings file")
        );
        assert_eq!(
            step(&rows, alacritty - 1, 1),
            alacritty - 1,
            "nothing to move to after it"
        );
        assert_eq!(step(&rows, 0, -1), 0);
        assert_eq!(step(&rows, 1, 1), 2);
    }

    #[test]
    fn list_positions_count_the_headings_above() {
        let rows = rows(Mode::Light, &colours(), "mocha.toml", &library_with_files());
        assert_eq!(list_position(&rows, 0), 0);
        assert_eq!(
            list_position(&rows, 3),
            4,
            "the first template is under its heading"
        );
        assert_eq!(
            list_position(&rows, 12),
            15,
            "three headings above the owner's first"
        );
    }
}
