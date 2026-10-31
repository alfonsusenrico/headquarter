//! New session (⌘N): a provider on the left, a folder from the folder browser on the
//! right, started with the provider's own command (redesign-session-modal D4). Folders
//! show how many sessions they hold, and starting where some are not in the sidebar
//! offers those first (add-folder-sessions).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    HighlightStyle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement,
    Render, ScrollStrategy, SharedString, StatefulInteractiveElement, Styled, StyledText,
    Subscription, Task, UniformListScrollHandle, Window, div, px, uniform_list,
};
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::Scrollbar;
use hq_protocol::SessionInfo;
use hq_protocol::sidebar::{age, display_title};
use hq_signals::event::Agent;

use super::folder_sessions;
use super::folders::{self, FolderRow, Listing};
use crate::marks::Marks;
use crate::terminal::view::hsla;
use crate::theme::Chrome;

const LINE: f32 = 38.0;
const DIALOG_WIDTH: f32 = 480.0;
/// The dialog's two buttons share one width (equal-width button groups).
const DIALOG_BUTTON_WIDTH: f32 = 150.0;

/// A provider in the left column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub agent: Agent,
    pub name: &'static str,
    pub version: Option<String>,
    /// `None` while discovery is still looking.
    pub installed: Option<bool>,
}

impl Provider {
    pub fn unknown(agent: Agent) -> Provider {
        Provider {
            agent,
            name: hq_discovery::known(agent.id()).map_or(agent.id(), |known| known.name),
            version: None,
            installed: None,
        }
    }
}

/// The version number in what an agent prints for `--version`, such as `2.1.306` in
/// "2.1.306 (Claude Code)" or `0.163.0` in "codex-cli 0.163.0".
pub fn version_number(printed: &str) -> &str {
    printed
        .split_whitespace()
        .find(|word| word.starts_with(|first: char| first.is_ascii_digit()))
        .unwrap_or(printed)
}

/// What a key did in the folder browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Not the browser's key; the field handles it.
    Ignored,
    Moved,
    /// Put this text in the field.
    Text(String),
    Start(PathBuf),
}

/// The folder field's text and the rows under it, with the highlighted row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Browser {
    pub text: String,
    pub index: usize,
    pub listing: Listing,
}

impl Browser {
    pub fn new(text: &str, home: &Path, found: &[PathBuf]) -> Browser {
        Browser {
            text: text.to_owned(),
            index: 0,
            listing: folders::list(text, home, found),
        }
    }

    /// Lists again for the same text, keeping the highlight where it can.
    pub fn refresh(&mut self, home: &Path, found: &[PathBuf]) {
        self.listing = folders::list(&self.text, home, found);
        self.index = self.index.min(self.listing.rows.len().saturating_sub(1));
    }

    pub fn set_text(&mut self, text: &str, home: &Path, found: &[PathBuf]) {
        *self = Browser::new(text, home, found);
    }

    pub fn highlighted(&self) -> Option<&FolderRow> {
        self.listing.rows.get(self.index)
    }

    /// The folder a session would start in: only while "Open here" is highlighted.
    pub fn start_folder(&self) -> Option<&Path> {
        match self.highlighted()? {
            FolderRow::Here(path) => Some(path),
            FolderRow::Sub(_) | FolderRow::Found(_) => None,
        }
    }

    /// ↑↓ choose a row; Enter, Tab and → (with the caret at the end) go into a folder;
    /// ← (at the end of a folder's path) goes up; Enter on "Open here" starts.
    pub fn key(&mut self, key: &str, caret_at_end: bool, home: &Path) -> Step {
        let into = |row: Option<&FolderRow>| match row {
            Some(FolderRow::Sub(path) | FolderRow::Found(path)) => {
                Some(Step::Text(folders::into_text(path, home)))
            }
            _ => None,
        };
        match key {
            "down" if !self.listing.rows.is_empty() => {
                self.index = (self.index + 1).min(self.listing.rows.len() - 1);
                Step::Moved
            }
            "up" if !self.listing.rows.is_empty() => {
                self.index = self.index.saturating_sub(1);
                Step::Moved
            }
            "enter" => match self.highlighted() {
                Some(FolderRow::Here(path)) => Step::Start(path.clone()),
                row => into(row).unwrap_or(Step::Moved),
            },
            // Tab never leaves the field.
            "tab" => into(self.highlighted()).unwrap_or(Step::Moved),
            "right" if caret_at_end => into(self.highlighted()).unwrap_or(Step::Ignored),
            "left" if caret_at_end && self.text.ends_with('/') => {
                folders::up_text(&self.text, home).map_or(Step::Ignored, Step::Text)
            }
            _ => Step::Ignored,
        }
    }
}

pub enum NewEvent {
    Start {
        agent: Agent,
        folder: PathBuf,
    },
    /// Add this session to the sidebar and resume it.
    Continue(String),
    /// Do not offer these sessions again (add-folder-sessions D4).
    NotAsked(Vec<String>),
    Close,
}

/// A session offered in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub id: String,
    pub agent: Agent,
    pub title: String,
    /// "Claude Code · last used 2h ago".
    pub detail: String,
}

impl Offer {
    pub fn new(session: &SessionInfo, now_ms: u64) -> Offer {
        Offer {
            id: session.id.clone(),
            agent: session.agent,
            title: display_title(session),
            detail: format!(
                "{} · last used {} ago",
                Provider::unknown(session.agent).name,
                age(now_ms.saturating_sub(session.last_activity_at))
            ),
        }
    }
}

/// The question asked on Start in a folder holding sessions not in the sidebar.
struct Asking {
    agent: Agent,
    folder: PathBuf,
    offers: Vec<Offer>,
    dont_ask: bool,
}

pub struct NewSession {
    providers: Vec<Provider>,
    agent: Option<Agent>,
    browser: Browser,
    /// Folders under the home folder found so far by the background walk (D6).
    found: Vec<PathBuf>,
    walking: bool,
    /// A host error from the last start, shown in the footer.
    error: Option<String>,
    starting: bool,
    sessions: Vec<SessionInfo>,
    /// Sessions per folder, for the rows' counts.
    counts: HashMap<PathBuf, usize>,
    /// Sessions the owner chose not to be offered again.
    not_asked: Vec<String>,
    asking: Option<Asking>,
    home: PathBuf,
    field: Entity<InputState>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    chrome: Chrome,
    marks: Marks,
    _field_changes: Subscription,
    _walk: Task<()>,
}

impl EventEmitter<NewEvent> for NewSession {}

impl NewSession {
    pub fn new(
        providers: Vec<Provider>,
        agent: Option<Agent>,
        home: PathBuf,
        chrome: Chrome,
        marks: Marks,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> NewSession {
        // Empty means the home folder, so there is nothing to delete before typing
        // (fix-new-session-home-folder D1).
        let start_text = "";
        let field = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Home folder, or type a path or a folder name")
        });
        let field_changes = cx.subscribe(
            &field,
            |this: &mut NewSession, field, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = field.read(cx).value().to_string();
                    this.browser.set_text(&text, &this.home, &this.found);
                    this.scroll.scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                }
            },
        );
        window.focus(&field.focus_handle(cx), cx);
        let walk = Self::start_walk(home.clone(), cx);
        let mut window_state = NewSession {
            agent: None,
            browser: Browser::new(start_text, &home, &[]),
            found: Vec::new(),
            walking: true,
            error: None,
            starting: false,
            sessions: Vec::new(),
            counts: HashMap::new(),
            not_asked: Vec::new(),
            asking: None,
            home,
            field,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            chrome,
            marks,
            providers: Vec::new(),
            _field_changes: field_changes,
            _walk: walk,
        };
        window_state.set_providers(providers, agent, cx);
        window_state
    }

    /// Walks the home folder in the background while the window is open; dropping the
    /// window drops the receiver, which stops the walk.
    fn start_walk(home: PathBuf, cx: &mut Context<Self>) -> Task<()> {
        let (sender, mut batches) = unbounded::<Vec<PathBuf>>();
        std::thread::spawn(move || {
            folders::walk(&home, |batch| sender.unbounded_send(batch).is_ok());
        });
        cx.spawn(async move |this, cx| {
            while let Some(batch) = batches.next().await {
                let kept = this.update(cx, |this, cx| {
                    this.found.extend(batch);
                    if !folders::is_path(&this.browser.text) {
                        this.browser.refresh(&this.home, &this.found);
                        cx.notify();
                    }
                });
                if kept.is_err() {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.walking = false;
                cx.notify();
            });
        })
    }

    /// The providers as discovery last reported them. `preferred` is chosen when it is
    /// usable; otherwise the first usable one.
    pub fn set_providers(
        &mut self,
        providers: Vec<Provider>,
        preferred: Option<Agent>,
        cx: &mut Context<Self>,
    ) {
        let usable = |agent: Agent| {
            providers
                .iter()
                .any(|provider| provider.agent == agent && provider.installed != Some(false))
        };
        let keep = self.agent.or(preferred).filter(|agent| usable(*agent));
        self.agent = keep.or_else(|| {
            providers
                .iter()
                .find(|provider| provider.installed != Some(false))
                .map(|provider| provider.agent)
        });
        self.providers = providers;
        cx.notify();
    }

    pub fn set_chrome(&mut self, chrome: Chrome, cx: &mut Context<Self>) {
        self.chrome = chrome;
        cx.notify();
    }

    pub fn set_marks(&mut self, marks: Marks, cx: &mut Context<Self>) {
        self.marks = marks;
        cx.notify();
    }

    /// Every session the host lists, for the counts and the check on Start.
    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>, cx: &mut Context<Self>) {
        self.counts = folder_sessions::counts(&sessions);
        self.sessions = sessions;
        cx.notify();
    }

    pub fn set_not_asked(&mut self, not_asked: Vec<String>) {
        self.not_asked = not_asked;
    }

    /// The host could not start the session; the window stays open and says why.
    pub fn failed(&mut self, error: String, cx: &mut Context<Self>) {
        self.starting = false;
        self.error = Some(error);
        cx.notify();
    }

    fn ready(&self) -> Option<(Agent, PathBuf)> {
        let agent = self.agent?;
        let folder = self.browser.start_folder()?;
        (!self.starting).then(|| (agent, folder.to_path_buf()))
    }

    /// Starts in the chosen folder, or first offers its sessions that are not in the
    /// sidebar (add-folder-sessions D2).
    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((agent, folder)) = self.ready() else {
            return;
        };
        let not_asked: HashSet<String> = self.not_asked.iter().cloned().collect();
        let now_ms = hq_signals::now_ms();
        let offers: Vec<Offer> = folder_sessions::forgotten(&self.sessions, &folder, &not_asked)
            .into_iter()
            .map(|session| Offer::new(session, now_ms))
            .collect();
        if offers.is_empty() {
            self.start_new(agent, folder, cx);
            return;
        }
        self.asking = Some(Asking {
            agent,
            folder,
            offers,
            dont_ask: false,
        });
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn start_new(&mut self, agent: Agent, folder: PathBuf, cx: &mut Context<Self>) {
        self.starting = true;
        self.error = None;
        cx.emit(NewEvent::Start { agent, folder });
        cx.notify();
    }

    /// Takes the dialog's answer away, keeping the offered sessions out of later
    /// questions when the owner ticked the box.
    fn answer(&mut self, cx: &mut Context<Self>) -> Option<Asking> {
        let asking = self.asking.take()?;
        if asking.dont_ask {
            let ids = asking.offers.iter().map(|offer| offer.id.clone()).collect();
            cx.emit(NewEvent::NotAsked(ids));
        }
        Some(asking)
    }

    fn start_new_anyway(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(asking) = self.answer(cx) {
            window.focus(&self.field.focus_handle(cx), cx);
            self.start_new(asking.agent, asking.folder, cx);
        }
    }

    fn continue_offer(&mut self, id: String, cx: &mut Context<Self>) {
        if self.answer(cx).is_some() {
            self.starting = true;
            self.error = None;
            cx.emit(NewEvent::Continue(id));
            cx.notify();
        }
    }

    fn cancel_asking(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = None;
        window.focus(&self.field.focus_handle(cx), cx);
        cx.notify();
    }

    fn go_to(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.field
            .update(cx, |field, cx| field.set_value(text.clone(), window, cx));
        self.browser.set_text(&text, &self.home, &self.found);
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    fn choose_row(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        match self.browser.listing.rows.get(index) {
            Some(FolderRow::Sub(path) | FolderRow::Found(path)) => {
                let text = folders::into_text(path, &self.home);
                self.go_to(text, window, cx);
            }
            Some(FolderRow::Here(_)) => {
                self.browser.index = index;
                cx.notify();
            }
            None => {}
        }
        window.focus(&self.field.focus_handle(cx), cx);
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !event.keystroke.modifiers.modified()
            && self.handle_key(&event.keystroke.key, window, cx)
        {
            cx.stop_propagation();
        }
    }

    fn handle_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.asking.is_some() {
            match key {
                "escape" => self.cancel_asking(window, cx),
                "enter" => self.start_new_anyway(window, cx),
                _ => return false,
            }
            return true;
        }
        if key == "escape" {
            cx.emit(NewEvent::Close);
            return true;
        }
        let field = self.field.read(cx);
        let caret_at_end = field.cursor() == field.value().len();
        match self.browser.key(key, caret_at_end, &self.home) {
            Step::Ignored => return false,
            Step::Moved => {
                self.scroll
                    .scroll_to_item(self.browser.index, ScrollStrategy::Nearest);
                cx.notify();
            }
            Step::Text(text) => self.go_to(text, window, cx),
            Step::Start(_) => self.start(window, cx),
        }
        true
    }

    fn render_nav(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chrome = self.chrome;
        let mut nav = super::nav(chrome);
        for provider in &self.providers {
            let agent = provider.agent;
            let missing = provider.installed == Some(false);
            let detail = if missing {
                "not found".to_owned()
            } else {
                provider
                    .version
                    .as_deref()
                    .map(version_number)
                    .unwrap_or_default()
                    .to_owned()
            };
            let mut row = super::nav_row(
                SharedString::from(format!("new-provider-{agent}")),
                self.agent == Some(agent),
                46.,
                chrome,
            )
            .child(self.marks.element(agent, 18., chrome.muted))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().child(provider.name))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(hsla(chrome.muted))
                            .child(detail),
                    ),
            );
            if missing {
                row = row.opacity(0.55);
            } else {
                row = row
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.agent = Some(agent);
                        this.error = None;
                        cx.notify();
                    }));
            }
            nav = nav.child(row);
        }
        nav
    }

    fn render_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.chrome;
        let Some(row) = self.browser.listing.rows.get(index) else {
            return div().into_any_element();
        };
        let shown = folders::shown(row.path(), &self.home);
        let label: AnyElement = match row {
            FolderRow::Here(_) => div()
                .truncate()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(shown)
                .into_any_element(),
            FolderRow::Sub(path) => div()
                .truncate()
                .child(
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or(shown),
                )
                .into_any_element(),
            FolderRow::Found(_) => {
                let head = shown.rfind('/').map_or(0, |slash| slash + 1);
                let mut highlights = vec![(
                    0..head,
                    HighlightStyle {
                        color: Some(hsla(chrome.muted)),
                        ..HighlightStyle::default()
                    },
                )];
                if let Some(range) = folders::match_range(&shown, &self.browser.text) {
                    highlights.push((
                        range,
                        HighlightStyle {
                            font_weight: Some(gpui::FontWeight::BOLD),
                            ..HighlightStyle::default()
                        },
                    ));
                }
                div()
                    .truncate()
                    .child(StyledText::new(shown).with_highlights(highlights))
                    .into_any_element()
            }
        };
        let highlighted = index == self.browser.index;
        // On the highlighted row's tint, muted text falls under AA in dark (D5).
        let quiet = if highlighted {
            chrome.group
        } else {
            chrome.muted
        };
        let end = match row {
            FolderRow::Here(_) => super::tag("Open here", chrome).text_color(hsla(quiet)),
            FolderRow::Sub(_) | FolderRow::Found(_) => {
                div().flex_none().text_color(hsla(quiet)).child("›")
            }
        };
        let count = self
            .counts
            .get(row.path())
            .and_then(|count| folder_sessions::count_label(*count))
            .map(|label| {
                div()
                    .flex_none()
                    .text_size(px(12.5))
                    .text_color(hsla(quiet))
                    .child(label)
            });
        let hover = super::tint(chrome, if highlighted { 0.2 } else { 0.06 });
        let mut line = div()
            .id(SharedString::from(format!("folder-{index}")))
            .w_full()
            .h(px(LINE))
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .rounded(px(10.))
            .text_size(px(14.))
            .text_color(hsla(chrome.text))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .child(div().flex_1().min_w_0().child(label))
            .children(count)
            .child(end)
            .on_click(cx.listener(move |this, _, window, cx| this.choose_row(index, window, cx)));
        if highlighted {
            line = line.bg(super::tint(chrome, 0.16));
        }
        line.into_any_element()
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chrome = self.chrome;
        let ready = self.ready();
        let summary = if let Some(error) = &self.error {
            div().text_color(hsla(chrome.amber)).child(error.clone())
        } else if self.starting {
            div().child("Starting…")
        } else {
            div().child(match (&ready, self.agent) {
                (Some((agent, folder)), _) => format!(
                    "{} in {}",
                    Provider::unknown(*agent).name,
                    folders::shown(folder, &self.home)
                ),
                (None, None) => "Choose a provider".to_owned(),
                (None, Some(_)) => "Choose a folder".to_owned(),
            })
        };
        super::footer(chrome)
            .child(summary.flex_1().min_w_0().truncate())
            .child(
                Button::new("new-cancel")
                    .label("Cancel")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(NewEvent::Close))),
            )
            .child(
                Button::new("new-start")
                    .primary()
                    .label("Start session")
                    .disabled(ready.is_none())
                    .on_click(cx.listener(|this, _, window, cx| this.start(window, cx))),
            )
    }
}

impl NewSession {
    /// The dialog over the window (add-folder-sessions D3).
    fn render_asking(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let asking = self.asking.as_ref()?;
        let chrome = self.chrome;
        let rows = asking.offers.iter().enumerate().map(|(index, offer)| {
            let id = offer.id.clone();
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .py(px(8.))
                .when(index > 0, |row| {
                    row.border_t_1().border_color(hsla(chrome.sidebar_line))
                })
                .child(self.marks.element(offer.agent, 18., chrome.muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().truncate().child(offer.title.clone()))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.5))
                                .text_color(hsla(chrome.muted))
                                .child(offer.detail.clone()),
                        ),
                )
                .child(
                    Button::new(SharedString::from(format!("continue-{index}")))
                        .label("Continue")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.continue_offer(id.clone(), cx);
                        })),
                )
        });
        let button = |id: &'static str, label: &'static str| {
            Button::new(id).label(label).w(px(DIALOG_BUTTON_WIDTH))
        };
        let dialog = div()
            .id("not-in-sidebar")
            .w(px(DIALOG_WIDTH))
            .max_h_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(20.))
            .rounded(px(14.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.sidebar))
            .shadow_lg()
            .text_size(px(14.))
            .text_color(hsla(chrome.text))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Not in your sidebar"),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_color(hsla(chrome.muted))
                            .child(folders::shown(&asking.folder, &self.home)),
                    ),
            )
            .child(
                div()
                    .id("not-in-sidebar-rows")
                    .flex_shrink(1.)
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(rows),
            )
            .child(
                Checkbox::new("not-in-sidebar-dont-ask")
                    .label("Don't ask again for these")
                    .checked(asking.dont_ask)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        if let Some(asking) = &mut this.asking {
                            asking.dont_ask = *checked;
                            cx.notify();
                        }
                    })),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(button("not-in-sidebar-cancel", "Cancel").on_click(
                        cx.listener(|this, _, window, cx| this.cancel_asking(window, cx)),
                    ))
                    .child(
                        button("not-in-sidebar-start", "Start new session")
                            .primary()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.start_new_anyway(window, cx);
                            })),
                    ),
            );
        Some(
            div()
                .id("not-in-sidebar-backdrop")
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .bg(super::shade(chrome))
                .child(dialog)
                .into_any_element(),
        )
    }
}

impl Focusable for NewSession {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for NewSession {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        let count = self.browser.listing.rows.len();
        let empty = if let Some(problem) = &self.browser.listing.problem {
            Some(problem.clone())
        } else if count > 0 {
            None
        } else if !folders::is_path(&self.browser.text) && self.walking {
            Some("Searching your folders…".to_owned())
        } else {
            Some("No folder matches.".to_owned())
        };
        let list = uniform_list(
            "new-session-folders",
            count,
            cx.processor(
                move |this: &mut NewSession, range: std::ops::Range<usize>, _window, cx| {
                    range
                        .map(|index| this.render_row(index, cx))
                        .collect::<Vec<_>>()
                },
            ),
        )
        .track_scroll(&self.scroll)
        .flex_1()
        .min_h(px(0.));
        let body = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .mx(px(16.))
                    .mt(px(14.))
                    .mb(px(8.))
                    .child(Input::new(&self.field)),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.))
                    .px(px(10.))
                    .flex()
                    .flex_col()
                    .children(empty.map(|note| {
                        div()
                            .px(px(12.))
                            .py(px(8.))
                            .text_size(px(13.5))
                            .text_color(hsla(chrome.muted))
                            .child(note)
                    }))
                    .child(list)
                    .child(Scrollbar::vertical(&self.scroll)),
            )
            .child(self.render_footer(cx));
        let frame = super::frame("new-session", chrome, window, self.render_nav(cx), body)
            .relative()
            .children(self.render_asking(cx))
            .key_context("NewSession")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down));
        super::route_field_keys(frame, Self::handle_key, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("hq-new-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        for folder in ["code/project/headquarter/src", "code/notes"] {
            fs::create_dir_all(home.join(folder)).unwrap();
        }
        home
    }

    #[test]
    fn an_empty_field_browses_and_starts_in_the_home_folder() {
        let home = home("empty");
        let mut browser = Browser::new("", &home, &[]);
        assert_eq!(browser.start_folder(), Some(home.as_path()));
        assert_eq!(browser.key("enter", true, &home), Step::Start(home.clone()));
        assert_eq!(browser.key("down", true, &home), Step::Moved);
        assert_eq!(
            browser.key("tab", true, &home),
            Step::Text("~/code/".to_owned()),
            "Tab from empty goes into the subfolder"
        );
    }

    #[test]
    fn keys_move_into_and_out_of_folders_and_start_only_from_open_here() {
        let home = home("keys");
        let mut browser = Browser::new("~/code/", &home, &[]);
        assert_eq!(browser.start_folder(), Some(home.join("code").as_path()));
        assert_eq!(
            browser.key("enter", true, &home),
            Step::Start(home.join("code")),
            "Enter on \"Open here\" starts"
        );

        assert_eq!(browser.key("down", true, &home), Step::Moved);
        assert_eq!(browser.start_folder(), None, "Start is off on a subfolder");
        assert_eq!(
            browser.key("enter", true, &home),
            Step::Text("~/code/notes/".to_owned()),
            "Enter on a subfolder goes into it"
        );

        browser.index = 2;
        assert_eq!(
            browser.key("tab", true, &home),
            Step::Text("~/code/project/".to_owned())
        );
        assert_eq!(
            browser.key("right", true, &home),
            Step::Text("~/code/project/".to_owned())
        );
        assert_eq!(
            browser.key("right", false, &home),
            Step::Ignored,
            "→ inside the text moves the caret"
        );

        browser.set_text("~/code/project/", &home, &[]);
        assert_eq!(
            browser.key("left", true, &home),
            Step::Text("~/code/".to_owned())
        );
        browser.set_text("~/code/proj", &home, &[]);
        assert_eq!(
            browser.key("left", true, &home),
            Step::Ignored,
            "← in a half-typed name moves the caret"
        );
        assert_eq!(
            browser.key("tab", true, &home),
            Step::Text("~/code/project/".to_owned())
        );
        assert_eq!(browser.key("up", true, &home), Step::Moved);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn versions_show_the_number_only() {
        assert_eq!(version_number("2.1.306 (Claude Code)"), "2.1.306");
        assert_eq!(version_number("codex-cli 0.163.0"), "0.163.0");
        assert_eq!(version_number("1.4.3"), "1.4.3");
        assert_eq!(version_number("unknown"), "unknown");
    }

    #[test]
    fn a_found_folder_is_gone_into() {
        let home = home("found");
        let found = vec![home.join("code/project/headquarter")];
        let mut browser = Browser::new("headq", &home, &found);
        assert_eq!(browser.start_folder(), None);
        assert_eq!(
            browser.key("enter", true, &home),
            Step::Text("~/code/project/headquarter/".to_owned())
        );
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn refreshing_keeps_the_highlight_in_range() {
        let home = home("refresh");
        let mut browser = Browser::new("note", &home, &[home.join("code/notes")]);
        browser.index = 0;
        browser.refresh(&home, &[]);
        assert_eq!(browser.index, 0);
        assert!(browser.listing.rows.is_empty());
        fs::remove_dir_all(&home).unwrap();
    }
}
