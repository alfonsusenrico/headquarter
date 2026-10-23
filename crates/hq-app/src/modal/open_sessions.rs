//! Open sessions (⌘O): every session not in the sidebar, those running in another
//! terminal first and the rest newest first by day, picked by clicking and added in one
//! step (redesign-session-modal D3).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, ScrollStrategy,
    SharedString, StatefulInteractiveElement, Styled, Subscription, UniformListScrollHandle,
    Window, div, px, uniform_list,
};
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::Scrollbar;
use hq_protocol::SessionInfo;
use hq_protocol::sidebar::{age, display_title};
use hq_signals::event::Agent;

use super::folders::{self, FolderRow};
use crate::marks::Marks;
use crate::terminal::view::hsla;
use crate::theme::Chrome;

/// Every list line has this height, so the list can render only what is visible.
const LINE: f32 = 52.0;
/// Folder suggestions shown under the search field.
const SUGGESTIONS: usize = 6;
const ELSEWHERE_INFO: &str = "Open in another terminal. Headquarter shows their status; close one there to continue it here.";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filters {
    /// Title text; a path (`~/…`, `/…`) searches folders instead and filters nothing.
    pub text: String,
    /// `None` is "All".
    pub agent: Option<Agent>,
    /// Only sessions in this folder and its subfolders.
    pub folder: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickRow {
    pub id: String,
    pub agent: Agent,
    pub title: String,
    pub folder: String,
    pub age: String,
    pub external: bool,
    /// Why the session cannot be added, when it cannot; shown as a tag on the row.
    pub blocked: Option<&'static str>,
}

/// The tag on a session running in another terminal: that terminal owns it, so it
/// cannot be added here (owner, 2026-10-22).
pub const IN_ANOTHER_TERMINAL: &str = "In another terminal";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickItem {
    Day(String),
    Session(PickRow),
}

/// The heading for a day: Today, Yesterday, a weekday within the week, then dates.
pub fn day_heading(day: NaiveDate, today: NaiveDate) -> String {
    let days_ago = (today - day).num_days();
    match days_ago {
        0 => "Today".to_owned(),
        1 => "Yesterday".to_owned(),
        2..=6 => day.format("%A").to_string(),
        _ if day.year() == today.year() => day.format("%-d %B").to_string(),
        _ => day.format("%-d %B %Y").to_string(),
    }
}

/// Sessions the window can list: everything not in the sidebar.
fn listable(session: &SessionInfo) -> bool {
    !session.in_sidebar
}

fn matches(session: &SessionInfo, filters: &Filters) -> bool {
    if !listable(session) {
        return false;
    }
    if filters.agent.is_some_and(|agent| agent != session.agent) {
        return false;
    }
    if let Some(folder) = &filters.folder
        && !Path::new(&session.cwd).starts_with(folder)
    {
        return false;
    }
    let needle = filters.text.trim().to_lowercase();
    if needle.is_empty() || folders::is_path(&needle) {
        return true;
    }
    [display_title(session), session.name.clone()]
        .iter()
        .any(|field| field.to_lowercase().contains(&needle))
}

/// How many sessions each provider entry lists: `None` is "All".
pub fn counts(sessions: &[SessionInfo]) -> HashMap<Option<Agent>, usize> {
    let mut counts = HashMap::new();
    for session in sessions.iter().filter(|session| listable(session)) {
        *counts.entry(None).or_default() += 1;
        *counts.entry(Some(session.agent)).or_default() += 1;
    }
    counts
}

/// The window's lines for `sessions` under `filters`: sessions running in another
/// terminal first, then the rest grouped by the local day of their last activity,
/// newest first. `order` holds each session's activity when the window opened, so the
/// list keeps its order while it is open.
pub fn pick_items<Tz: TimeZone>(
    sessions: &[SessionInfo],
    filters: &Filters,
    order: &HashMap<String, u64>,
    home: &Path,
    now: &DateTime<Tz>,
) -> Vec<PickItem> {
    let when = |session: &SessionInfo| {
        order
            .get(&session.id)
            .copied()
            .unwrap_or(session.last_activity_at)
    };
    let mut shown: Vec<&SessionInfo> = sessions
        .iter()
        .filter(|session| matches(session, filters))
        .collect();
    shown.sort_by_key(|session| (std::cmp::Reverse(when(session)), session.id.as_str()));
    let zone = now.timezone();
    let today = now.date_naive();
    let now_ms = u64::try_from(now.timestamp_millis()).unwrap_or(0);

    let mut items = Vec::new();
    let mut current_day: Option<NaiveDate> = None;
    for session in shown {
        let millis = i64::try_from(when(session)).unwrap_or(i64::MAX);
        let day = zone
            .timestamp_millis_opt(millis)
            .single()
            .map_or(today, |at| at.date_naive());
        if current_day != Some(day) {
            items.push(PickItem::Day(day_heading(day, today)));
            current_day = Some(day);
        }
        items.push(PickItem::Session(PickRow {
            id: session.id.clone(),
            agent: session.agent,
            title: display_title(session),
            folder: folders::shown(Path::new(&session.cwd), home),
            age: age(now_ms.saturating_sub(session.last_activity_at)),
            external: session.external.is_some(),
            blocked: session.external.is_some().then_some(IN_ANOTHER_TERMINAL),
        }));
    }
    items
}

/// The folders offered under the search field while it holds a path.
pub fn suggestions(text: &str, home: &Path) -> Vec<PathBuf> {
    if !folders::is_path(text) {
        return Vec::new();
    }
    folders::list(text, home, &[])
        .rows
        .into_iter()
        .filter_map(|row| match row {
            FolderRow::Sub(path) => Some(path),
            FolderRow::Here(_) | FolderRow::Found(_) => None,
        })
        .take(SUGGESTIONS)
        .collect()
}

pub enum OpenEvent {
    /// Add these sessions to the sidebar, in list order, and show the first.
    Open(Vec<String>),
    Close,
}

pub struct OpenSessions {
    sessions: Vec<SessionInfo>,
    filters: Filters,
    order: HashMap<String, u64>,
    picked: HashSet<String>,
    /// The line the arrow keys are on.
    cursor: Option<usize>,
    /// Folder suggestions for a path in the search field, and the highlighted one.
    suggestions: Vec<PathBuf>,
    suggestion: usize,
    info_open: bool,
    agents: Vec<Agent>,
    home: PathBuf,
    search: Entity<InputState>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    chrome: Chrome,
    marks: Marks,
    /// Whether the host's first list has arrived; until then the agents check in
    /// where the sessions will be.
    loaded: bool,
    _search_changes: Subscription,
}

impl EventEmitter<OpenEvent> for OpenSessions {}

impl OpenSessions {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sessions: Vec<SessionInfo>,
        agents: Vec<Agent>,
        home: PathBuf,
        chrome: Chrome,
        marks: Marks,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> OpenSessions {
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search titles, or type ~/ for a folder")
        });
        let search_changes = cx.subscribe(
            &search,
            |this: &mut OpenSessions, search, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = search.read(cx).value().to_string();
                    this.text_changed(text, cx);
                }
            },
        );
        window.focus(&search.focus_handle(cx), cx);
        let order = sessions
            .iter()
            .map(|session| (session.id.clone(), session.last_activity_at))
            .collect();
        OpenSessions {
            sessions,
            filters: Filters::default(),
            order,
            picked: HashSet::new(),
            cursor: None,
            suggestions: Vec::new(),
            suggestion: 0,
            info_open: false,
            agents,
            home,
            search,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            chrome,
            marks,
            loaded: true,
            _search_changes: search_changes,
        }
    }

    pub fn set_marks(&mut self, marks: Marks, cx: &mut Context<Self>) {
        self.marks = marks;
        cx.notify();
    }

    pub fn set_chrome(&mut self, chrome: Chrome, cx: &mut Context<Self>) {
        self.chrome = chrome;
        cx.notify();
    }

    /// New data from the host while the window is open; the order stays as it was.
    pub fn set_loaded(&mut self, loaded: bool, cx: &mut Context<Self>) {
        self.loaded = loaded;
        cx.notify();
    }

    pub fn update_sessions(&mut self, sessions: Vec<SessionInfo>, cx: &mut Context<Self>) {
        self.sessions = sessions;
        cx.notify();
    }

    fn items(&self) -> Vec<PickItem> {
        pick_items(
            &self.sessions,
            &self.filters,
            &self.order,
            &self.home,
            &Local::now(),
        )
    }

    fn text_changed(&mut self, text: String, cx: &mut Context<Self>) {
        self.suggestions = suggestions(&text, &self.home);
        self.suggestion = 0;
        if !folders::is_path(&text) || !folders::is_path(&self.filters.text) {
            self.cursor = None;
        }
        self.filters.text = text;
        cx.notify();
    }

    fn set_search(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.search
            .update(cx, |search, cx| search.set_value(text.clone(), window, cx));
        self.text_changed(text, cx);
    }

    fn choose_agent(&mut self, agent: Option<Agent>, cx: &mut Context<Self>) {
        self.filters.agent = agent;
        self.cursor = None;
        cx.notify();
    }

    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.picked.remove(id) {
            self.picked.insert(id.to_owned());
        }
        cx.notify();
    }

    /// Opens the picked sessions, or the highlighted one when none is picked.
    fn confirm(&mut self, cx: &mut Context<Self>) {
        let items = self.items();
        let mut ids: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                PickItem::Session(row) if self.picked.contains(&row.id) => Some(row.id.clone()),
                _ => None,
            })
            .collect();
        // Picks hidden by the current provider or search still count, after the shown ones.
        let mut hidden: Vec<String> = self
            .picked
            .iter()
            .filter(|id| !ids.contains(id))
            .cloned()
            .collect();
        hidden.sort();
        ids.extend(hidden);
        if ids.is_empty()
            && let Some(PickItem::Session(row)) = self.cursor.and_then(|cursor| items.get(cursor))
            && row.blocked.is_none()
        {
            ids.push(row.id.clone());
        }
        if !ids.is_empty() {
            cx.emit(OpenEvent::Open(ids));
        }
    }

    /// Turns the folder in the search field, or the highlighted suggestion, into the
    /// folder filter.
    fn choose_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.filters.text.clone();
        let typed = folders::expand(text.trim().trim_end_matches('/'), &self.home);
        let folder = if typed.is_dir() {
            Some(typed)
        } else {
            self.suggestions.get(self.suggestion).cloned()
        };
        if let Some(folder) = folder {
            self.filters.folder = Some(folder);
            self.cursor = None;
            self.set_search(String::new(), window, cx);
        }
    }

    fn complete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(folder) = self.suggestions.get(self.suggestion) {
            let text = folders::into_text(folder, &self.home);
            self.set_search(text, window, cx);
        }
    }

    /// Moves the arrow-key line to the next session in `step`'s direction.
    fn move_cursor(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.items();
        let sessions: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, PickItem::Session(_)))
            .map(|(index, _)| index)
            .collect();
        if sessions.is_empty() {
            return;
        }
        let position = self
            .cursor
            .and_then(|cursor| sessions.iter().position(|index| *index == cursor));
        let next = match position {
            Some(position) => position.saturating_add_signed(step).min(sessions.len() - 1),
            None => 0,
        };
        self.cursor = Some(sessions[next]);
        self.scroll
            .scroll_to_item(sessions[next], ScrollStrategy::Center);
        // Space picks the highlighted row once the list has the keys.
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn suggesting(&self) -> bool {
        !self.suggestions.is_empty() && folders::is_path(&self.filters.text)
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !event.keystroke.modifiers.modified()
            && self.handle_key(&event.keystroke.key, window, cx)
        {
            cx.stop_propagation();
        }
    }

    fn handle_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let list_focused = self.focus.is_focused(window);
        match key {
            "escape" if self.info_open => {
                self.info_open = false;
                cx.notify();
            }
            "escape" => cx.emit(OpenEvent::Close),
            // Tab completes a folder and never leaves the field.
            "tab" => self.complete(window, cx),
            "enter" if folders::is_path(&self.filters.text) => self.choose_folder(window, cx),
            "enter" => self.confirm(cx),
            "down" | "up" if self.suggesting() => {
                let last = self.suggestions.len() - 1;
                self.suggestion = if key == "down" {
                    (self.suggestion + 1).min(last)
                } else {
                    self.suggestion.saturating_sub(1)
                };
                cx.notify();
            }
            "down" => self.move_cursor(1, window, cx),
            "up" => self.move_cursor(-1, window, cx),
            "space" if list_focused => {
                if let Some(PickItem::Session(row)) = self
                    .cursor
                    .and_then(|cursor| self.items().get(cursor).cloned())
                    && row.blocked.is_none()
                {
                    self.toggle(&row.id, cx);
                }
            }
            "backspace" if self.filters.text.is_empty() && self.filters.folder.is_some() => {
                self.filters.folder = None;
                cx.notify();
            }
            _ => return false,
        }
        true
    }

    fn render_line(&self, item: &PickItem, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.chrome;
        match item {
            PickItem::Day(day) => super::heading(chrome)
                .h(px(LINE))
                .items_end()
                .pb(px(6.))
                .child(day.to_uppercase())
                .into_any_element(),
            PickItem::Session(row) => self.render_session(row, index, cx),
        }
    }

    fn render_session(&self, row: &PickRow, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.chrome;
        let picked = self.picked.contains(&row.id);
        let pickable = row.blocked.is_none();
        let id = row.id.clone();
        let mut line = div()
            .id(SharedString::from(format!("pick-{}", row.id)))
            .w_full()
            .h(px(LINE))
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(10.))
            .rounded(px(10.))
            .child(self.marks.element(row.agent, 18., chrome.muted))
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
                            .child(row.title.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(hsla(chrome.muted))
                            .child(row.folder.clone()),
                    ),
            )
            .children(row.blocked.map(|why| {
                div()
                    .flex_none()
                    .px(px(8.))
                    .py(px(1.))
                    .rounded_full()
                    .border_1()
                    .border_color(hsla(chrome.sidebar_line))
                    .text_size(px(11.5))
                    .text_color(hsla(chrome.muted))
                    .child(why)
            }))
            .child(
                div()
                    .flex_none()
                    .text_size(px(12.5))
                    .text_color(hsla(chrome.muted))
                    .child(row.age.clone()),
            );
        if pickable {
            line = line.child(super::tick(picked, chrome));
        }
        if picked {
            line = line.bg(super::tint(chrome, 0.16));
        } else if self.cursor == Some(index) {
            line = line.bg(super::tint(chrome, 0.08));
        }
        if pickable {
            let hover = super::tint(chrome, if picked { 0.2 } else { 0.06 });
            line = line
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.cursor = Some(index);
                    this.toggle(&id, cx);
                }));
        } else {
            line = line.opacity(0.55);
        }
        line.into_any_element()
    }

    fn render_nav(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chrome = self.chrome;
        let counts = counts(&self.sessions);
        let entries = std::iter::once(None).chain(self.agents.iter().copied().map(Some));
        let mut nav = super::nav(chrome);
        for agent in entries {
            let chosen = self.filters.agent == agent;
            let key = agent.map_or("all", Agent::id);
            let name = agent.map_or("All", |agent| {
                hq_discovery::known(agent.id()).map_or(agent.id(), |known| known.name)
            });
            let icon = match agent {
                Some(agent) => self.marks.element(agent, 18., chrome.muted),
                None => div()
                    .size(px(18.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(hsla(chrome.muted))
                    .child("▦")
                    .into_any_element(),
            };
            nav = nav.child(
                super::nav_row(
                    SharedString::from(format!("provider-{key}")),
                    chosen,
                    34.,
                    chrome,
                )
                .cursor_pointer()
                .child(icon)
                .child(div().flex_1().min_w_0().truncate().child(name))
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(hsla(chrome.muted))
                        .child(counts.get(&agent).copied().unwrap_or(0).to_string()),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.choose_agent(agent, cx))),
            );
        }
        nav
    }

    fn render_search(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chrome = self.chrome;
        let chip = self.filters.folder.as_ref().map(|folder| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .px(px(8.))
                .py(px(2.))
                .rounded(px(7.))
                .bg(super::tint(chrome, 0.14))
                .text_size(px(12.5))
                .text_color(hsla(chrome.text))
                .child(folders::shown(folder, &self.home))
                .child(
                    div()
                        .id("clear-folder")
                        .cursor_pointer()
                        .text_color(hsla(chrome.muted))
                        .child("✕")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.filters.folder = None;
                            this.cursor = None;
                            cx.notify();
                        })),
                )
        });
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.))
            .mx(px(16.))
            .mt(px(14.))
            .mb(px(6.))
            .children(chip)
            .child(div().flex_1().child(Input::new(&self.search)))
    }

    fn render_suggestions(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if !self.suggesting() {
            return None;
        }
        let chrome = self.chrome;
        let mut list = div()
            .id("folder-suggestions")
            .absolute()
            .top(px(58.))
            .left(px(16.))
            .right(px(16.))
            .p(px(4.))
            .flex()
            .flex_col()
            .rounded(px(10.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.selected))
            .shadow_lg()
            .occlude();
        for (index, folder) in self.suggestions.iter().enumerate() {
            let text = folders::into_text(folder, &self.home);
            let mut line = div()
                .id(SharedString::from(format!("suggestion-{index}")))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(7.))
                .text_size(px(13.5))
                .text_color(hsla(chrome.text))
                .cursor_pointer()
                .child(text.clone())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_search(text.clone(), window, cx);
                    window.focus(&this.search.focus_handle(cx), cx);
                }));
            if index == self.suggestion {
                line = line.bg(super::tint(chrome, 0.16));
            }
            list = list.child(line);
        }
        Some(list)
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let picked = self.picked.len();
        let open_label = if picked == 0 {
            "Open".to_owned()
        } else {
            format!("Open {picked}")
        };
        super::footer(self.chrome)
            .child(div().flex_1().child(if picked == 0 {
                String::new()
            } else {
                format!("{picked} selected")
            }))
            .children((picked > 0).then(|| {
                Button::new("open-clear")
                    .ghost()
                    .label("Clear")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.picked.clear();
                        cx.notify();
                    }))
            }))
            .child(
                Button::new("open-cancel")
                    .label("Cancel")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(OpenEvent::Close))),
            )
            .child(
                Button::new("open-confirm")
                    .primary()
                    .label(open_label)
                    .disabled(picked == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm(cx))),
            )
    }
}

impl Focusable for OpenSessions {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for OpenSessions {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        let count = self.items().len();
        let list = uniform_list(
            "open-sessions-lines",
            count,
            cx.processor(
                move |this: &mut OpenSessions, range: std::ops::Range<usize>, _window, cx| {
                    let items = this.items();
                    range
                        .filter_map(|index| {
                            items
                                .get(index)
                                .map(|item| this.render_line(item, index, cx))
                        })
                        .collect::<Vec<_>>()
                },
            ),
        )
        .track_scroll(&self.scroll)
        .size_full();
        let info = self.info_open.then(|| {
            div()
                .id("elsewhere-info-text")
                .absolute()
                .top(px(64.))
                .left(px(26.))
                .w(px(300.))
                .p(px(10.))
                .rounded(px(9.))
                .border_1()
                .border_color(hsla(chrome.sidebar_line))
                .bg(hsla(chrome.sidebar))
                .shadow_lg()
                .text_size(px(12.5))
                .text_color(hsla(chrome.text))
                .occlude()
                .child(ELSEWHERE_INFO)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.info_open = false;
                    cx.notify();
                }))
        });
        let body = div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(self.render_search(cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.))
                    .px(px(10.))
                    .children((count == 0).then(|| {
                        if self.loaded {
                            div()
                                .p(px(16.))
                                .text_color(hsla(chrome.muted))
                                .child("No sessions match.")
                                .into_any_element()
                        } else {
                            div()
                                .pt(px(48.))
                                .child(crate::loading::agents_checking_in(
                                    "open-sessions-loading",
                                    &self.marks,
                                    28.,
                                    chrome,
                                    "Loading your sessions…",
                                ))
                                .into_any_element()
                        }
                    }))
                    .child(list)
                    .child(Scrollbar::vertical(&self.scroll))
                    .children(info),
            )
            .child(self.render_footer(cx))
            .children(self.render_suggestions(cx));
        let frame = super::frame("open-sessions", chrome, window, self.render_nav(cx), body)
            .key_context("OpenSessions")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down));
        super::route_field_keys(frame, Self::handle_key, cx)
    }
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;
    use hq_protocol::{External, Origin};
    use hq_signals::status::Status;

    use super::*;

    const HOUR: u64 = 3_600_000;

    /// Noon on 20 October 2026, UTC+7.
    fn now() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(7 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 20, 12, 0, 0)
            .unwrap()
    }

    fn home() -> PathBuf {
        PathBuf::from("/u")
    }

    fn session(id: &str, agent: Agent, cwd: &str, hours_ago: u64) -> SessionInfo {
        let now_ms = u64::try_from(now().timestamp_millis()).unwrap();
        SessionInfo {
            id: id.to_owned(),
            agent,
            cwd: cwd.to_owned(),
            name: id.to_owned(),
            agent_session_id: Some(format!("conv-{id}")),
            status: Status::Inactive,
            needs_reason: None,
            created_at: 0,
            last_activity_at: now_ms - hours_ago * HOUR,
            ended_at: None,
            live: false,
            origin: Origin::History,
            title: None,
            external: None,
            in_sidebar: false,
            group: None,
            owner_named: false,
            interrupted: false,
        }
    }

    fn fixture() -> Vec<SessionInfo> {
        let mut titled = session("fix", Agent::Claude, "/u/api", 1);
        titled.title = Some("Fix the login flow".into());
        let mut added = session("kept", Agent::Codex, "/u/web", 2);
        added.in_sidebar = true;
        let mut elsewhere = session("ext-linked", Agent::Claude, "/u/web", 3);
        elsewhere.external = Some(External {
            pid: Some(5),
            status: None,
        });
        let mut unlinked = session("external-agy-9", Agent::Agy, "/u/web", 0);
        unlinked.origin = Origin::External;
        unlinked.external = Some(External {
            pid: Some(9),
            status: None,
        });
        vec![
            titled,
            added,
            elsewhere,
            unlinked,
            session("web-today", Agent::Codex, "/u/web/app", 2),
            session("yesterday", Agent::Agy, "/u/api", 24),
            session("tuesday-before", Agent::Codex, "/u/api", 24 * 7),
            session("last-year", Agent::Claude, "/u/api", 24 * 400),
        ]
    }

    fn items(filters: &Filters) -> Vec<PickItem> {
        pick_items(&fixture(), filters, &HashMap::new(), &home(), &now())
    }

    fn lines(items: &[PickItem]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                PickItem::Day(day) => format!("# {day}"),
                PickItem::Session(row) => row.id.clone(),
            })
            .collect()
    }

    fn row(items: &[PickItem], id: &str) -> PickRow {
        items
            .iter()
            .find_map(|item| match item {
                PickItem::Session(row) if row.id == id => Some(row.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn by_day_with_sessions_elsewhere_in_place_without_sidebar_sessions() {
        assert_eq!(
            lines(&items(&Filters::default())),
            [
                "# Today",
                "external-agy-9",
                "fix",
                "web-today",
                "ext-linked",
                "# Yesterday",
                "yesterday",
                "# 13 October",
                "tuesday-before",
                "# 15 September 2025",
                "last-year",
            ]
        );
    }

    #[test]
    fn rows_show_title_folder_and_why_they_cannot_be_added() {
        let items = items(&Filters::default());
        let fix = row(&items, "fix");
        assert_eq!(fix.title, "Fix the login flow");
        assert_eq!(fix.folder, "~/api");
        assert!(fix.blocked.is_none());
        assert!(row(&items, "ext-linked").external);
        assert_eq!(
            row(&items, "ext-linked").blocked,
            Some(IN_ANOTHER_TERMINAL),
            "a session open in another terminal cannot be added"
        );
        assert_eq!(
            row(&items, "external-agy-9").blocked,
            Some(IN_ANOTHER_TERMINAL)
        );
    }

    #[test]
    fn the_order_holds_while_the_window_is_open() {
        let mut sessions = fixture();
        let order: HashMap<String, u64> = sessions
            .iter()
            .map(|session| (session.id.clone(), session.last_activity_at))
            .collect();
        // "last-year" becomes active while the window is open.
        let now_ms = u64::try_from(now().timestamp_millis()).unwrap();
        sessions.last_mut().unwrap().last_activity_at = now_ms;
        let kept = pick_items(&sessions, &Filters::default(), &order, &home(), &now());
        assert_eq!(lines(&kept), lines(&items(&Filters::default())));
        assert_eq!(
            row(&kept, "last-year").age,
            "0m",
            "the age itself is current"
        );
    }

    #[test]
    fn provider_text_and_folder_filters_combine() {
        let codex = Filters {
            agent: Some(Agent::Codex),
            ..Filters::default()
        };
        assert_eq!(
            lines(&items(&codex)),
            ["# Today", "web-today", "# 13 October", "tuesday-before"]
        );
        let login = Filters {
            text: "LOGIN".into(),
            ..Filters::default()
        };
        assert_eq!(lines(&items(&login)), ["# Today", "fix"]);
        let web = Filters {
            folder: Some(PathBuf::from("/u/web")),
            ..Filters::default()
        };
        assert_eq!(
            lines(&items(&web)),
            ["# Today", "external-agy-9", "web-today", "ext-linked"],
            "the folder and its subfolders; not /u/webby"
        );
        let path_text = Filters {
            text: "~/we".into(),
            ..Filters::default()
        };
        assert_eq!(
            items(&path_text).len(),
            items(&Filters::default()).len(),
            "a path being typed filters nothing until it is chosen"
        );
    }

    #[test]
    fn counts_leave_out_sidebar_sessions() {
        let counts = counts(&fixture());
        assert_eq!(counts[&None], 7);
        assert_eq!(counts[&Some(Agent::Codex)], 2);
        assert_eq!(counts[&Some(Agent::Claude)], 3);
        assert_eq!(counts.get(&Some(Agent::Agy)).copied(), Some(2));
    }

    #[test]
    fn day_headings() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let day = |m, d| NaiveDate::from_ymd_opt(2026, m, d).unwrap();
        assert_eq!(day_heading(today, today), "Today");
        assert_eq!(day_heading(day(10, 19), today), "Yesterday");
        assert_eq!(day_heading(day(10, 16), today), "Friday");
        assert_eq!(day_heading(day(10, 13), today), "13 October");
    }

    #[test]
    fn folder_suggestions_complete_a_half_typed_path() {
        let root = std::env::temp_dir().join(format!("hq-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for folder in ["work/api", "web", ".hidden"] {
            std::fs::create_dir_all(root.join(folder)).unwrap();
        }
        let found = suggestions("~/w", &root);
        let all = suggestions("~/", &root);
        let none = suggestions("work", &root);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(found, [root.join("web"), root.join("work")]);
        assert_eq!(all, [root.join("web"), root.join("work")]);
        assert!(none.is_empty(), "a bare word searches titles");
        assert_eq!(folders::into_text(&root.join("work"), &root), "~/work/");
    }

    #[test]
    fn two_thousand_sessions_stay_fast() {
        let sessions: Vec<SessionInfo> = (0..2_000)
            .map(|index| {
                session(
                    &format!("s{index}"),
                    Agent::Claude,
                    &format!("/u/p{}", index % 40),
                    index,
                )
            })
            .collect();
        let started = std::time::Instant::now();
        let items = pick_items(
            &sessions,
            &Filters::default(),
            &HashMap::new(),
            &home(),
            &now(),
        );
        let elapsed = started.elapsed();
        assert!(items.len() > 2_000);
        assert!(
            elapsed < std::time::Duration::from_millis(200),
            "{elapsed:?} for 2,000 sessions"
        );
    }
}
