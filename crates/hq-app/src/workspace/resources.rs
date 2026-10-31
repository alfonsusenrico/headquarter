//! The Usage popover and window (add-usage-window D2-D4): what Headquarter and each
//! sidebar session cost the machine. ⌘U (Ctrl+Shift+U on Linux) shows the popover with
//! the five busiest sessions; its Show all, or Window ▸ Usage, shows the full table. The
//! host is asked every 2 s while either is shown, and not otherwise.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, Div, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, StatefulInteractiveElement, Styled, Task, Window,
    div, px,
};
use hq_client::HostPaths;
use hq_protocol::{Footprint, GroupInfo, SessionInfo, sidebar};
use hq_signals::event::Agent;
use hq_signals::resources::Sampler;

use super::{Modal, Pane, Workspace, overlay};
use crate::shortcuts;
use crate::sidebar::model::Glyph;
use crate::terminal::view::hsla;

const REFRESH: Duration = Duration::from_secs(2);
/// How many sessions the popover lists.
const TOP: usize = 5;
const POPOVER_WIDTH: f32 = 380.0;
const WINDOW_WIDTH: f32 = 760.0;
/// The window shrinks to leave this much of the app around it.
const MARGIN: f32 = 24.0;
const AGENT_WIDTH: f32 = 120.0;
const STATUS_WIDTH: f32 = 130.0;
const CPU_WIDTH: f32 = 64.0;
const MEMORY_WIDTH: f32 = 80.0;
const MARK_SIZE: f32 = 16.0;

pub(super) struct ResourcesView {
    shown: Shown,
    sort: Sort,
    focus: FocusHandle,
    reading: Option<Reading>,
    problem: Option<String>,
    _refresh: Task<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Popover,
    Window,
}

/// The table's order: the sidebar's, or the heaviest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sort {
    Sidebar,
    Cpu,
    Memory,
}

/// One measurement: the app's own, the host's, and each running session's.
#[derive(Debug, Clone, Default)]
struct Reading {
    app: Footprint,
    host: Footprint,
    sessions: HashMap<String, Footprint>,
}

impl Reading {
    /// Headquarter's own figure: the app and the host together (D3).
    fn headquarter(&self) -> Footprint {
        self.app.plus(self.host)
    }
}

/// A sidebar session as the popover and the window list it.
#[derive(Debug, Clone, PartialEq)]
struct Line {
    id: String,
    agent: Agent,
    title: String,
    folder: String,
    status: String,
    /// Whether its agent runs in this host.
    running: bool,
    /// `None` when it is not running, or not measured yet.
    footprint: Option<Footprint>,
}

/// Every sidebar session, in `sort`'s order. Sorting by a figure puts the measured
/// sessions first and keeps the sidebar's order among equals.
fn lines(
    sessions: &[SessionInfo],
    groups: &[GroupInfo],
    home: Option<&str>,
    reading: Option<&Reading>,
    sort: Sort,
) -> Vec<Line> {
    let layout = sidebar::layout(sessions, groups, home);
    let ordered = layout
        .groups
        .iter()
        .flat_map(|section| section.sessions.iter().copied())
        .chain(layout.ungrouped.iter().copied());
    let mut lines: Vec<Line> = ordered
        .map(|session| {
            let running = session.live && session.external.is_none();
            Line {
                id: session.id.clone(),
                agent: session.agent,
                title: sidebar::display_title(session),
                folder: layout.folder(session).to_owned(),
                status: status_word(session),
                running,
                footprint: reading
                    .filter(|_| running)
                    .and_then(|reading| reading.sessions.get(&session.id).copied()),
            }
        })
        .collect();
    let figure = |line: &Line, sort: Sort| -> f64 {
        match (line.footprint, sort) {
            (None, _) | (_, Sort::Sidebar) => -1.0,
            (Some(footprint), Sort::Cpu) => f64::from(footprint.cpu.unwrap_or(0.0)),
            (Some(footprint), Sort::Memory) => footprint.memory as f64,
        }
    };
    if sort != Sort::Sidebar {
        lines.sort_by(|one, two| figure(two, sort).total_cmp(&figure(one, sort)));
    }
    lines
}

/// The popover's sessions: the measured ones using the most CPU.
fn top(lines: &[Line]) -> Vec<&Line> {
    let mut measured: Vec<&Line> = lines
        .iter()
        .filter(|line| line.footprint.is_some())
        .collect();
    let cpu = |line: &Line| {
        line.footprint
            .and_then(|footprint| footprint.cpu)
            .unwrap_or(0.0)
    };
    measured.sort_by(|one, two| cpu(two).total_cmp(&cpu(one)));
    measured.truncate(TOP);
    measured
}

/// All agents together, with how many run and how many do not.
/// Headquarter's own figures and every running session's together (add-usage-total D1).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Totals {
    headquarter: Footprint,
    sessions: Footprint,
    running: usize,
}

impl Totals {
    fn of(reading: &Reading, lines: &[Line]) -> Totals {
        let (sessions, running, _) = all_agents(lines);
        Totals {
            headquarter: reading.headquarter(),
            sessions,
            running,
        }
    }

    fn total(self) -> Footprint {
        self.headquarter.plus(self.sessions)
    }
}

/// "All 12 sessions", "1 session".
fn sessions_label(running: usize) -> String {
    match running {
        1 => "1 session".to_owned(),
        count => format!("All {count} sessions"),
    }
}

fn all_agents(lines: &[Line]) -> (Footprint, usize, usize) {
    let total = lines.iter().filter(|line| line.running).fold(
        Footprint {
            cpu: Some(0.0),
            ..Footprint::default()
        },
        |total, line| total.plus(line.footprint.unwrap_or_default()),
    );
    let running = lines.iter().filter(|line| line.running).count();
    (total, running, lines.len() - running)
}

fn status_word(session: &SessionInfo) -> String {
    // What runs in the background, or a plan to review (add-background-work-status D5).
    match sidebar::beside(session) {
        Some(sidebar::Beside::Review) => return "plan to review".to_owned(),
        Some(sidebar::Beside::Background) => {
            let work = &session.background;
            let running = work.agents + work.commands + work.monitors;
            return format!("{running} in background");
        }
        None => {}
    }
    match Glyph::of(session) {
        Glyph::Working => "working",
        Glyph::NeedsYou => "needs you",
        Glyph::IdleUnread => "idle, not read",
        Glyph::Idle => "idle",
        Glyph::Inactive => "not running",
        Glyph::Starting => "starting",
        Glyph::Elsewhere => "in another terminal",
    }
    .to_owned()
}

/// CPU as Activity Monitor shows it, 100% being one core; "…" until there are two samples.
fn cpu_text(cpu: Option<f32>) -> String {
    match cpu {
        None => "…".to_owned(),
        Some(cpu) if cpu < 10.0 => format!("{cpu:.1}%"),
        Some(cpu) => format!("{cpu:.0}%"),
    }
}

fn memory_text(bytes: u64) -> String {
    let megabytes = bytes as f64 / (1024.0 * 1024.0);
    if megabytes < 1024.0 {
        format!("{megabytes:.0} MB")
    } else {
        format!("{:.1} GB", megabytes / 1024.0)
    }
}

/// What a session's figure includes besides its agent.
fn child_note(footprint: Option<Footprint>) -> Option<String> {
    match footprint?.processes.saturating_sub(1) {
        0 => None,
        1 => Some("+1 child process".to_owned()),
        children => Some(format!("+{children} child processes")),
    }
}

fn agent_name(agent: Agent) -> &'static str {
    hq_discovery::known(agent.id()).map_or(agent.id(), |known| known.name)
}

fn key_label(mac: bool) -> &'static str {
    if mac { "⌘U" } else { "Ctrl+Shift+U" }
}

/// Measures the app here and asks the host for the rest; runs off the main thread.
fn read(sampler: &Mutex<Sampler>) -> anyhow::Result<Reading> {
    let paths =
        HostPaths::standard().ok_or_else(|| anyhow::anyhow!("cannot find the home folder"))?;
    let (host, sessions) = hq_client::resources(&paths)?;
    let mut sampler = sampler.lock().unwrap_or_else(PoisonError::into_inner);
    sampler.refresh();
    let app = sampler.alone(std::process::id()).unwrap_or_default();
    Ok(Reading {
        app,
        host,
        sessions: sessions
            .into_iter()
            .map(|entry| (entry.session, entry.footprint))
            .collect(),
    })
}

impl Workspace {
    /// ⌘U: shows the popover, or closes whichever is shown.
    pub(super) fn toggle_resources(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.resources.is_some() {
            self.close_resources(window, cx);
        } else {
            self.open_resources(Shown::Popover, window, cx);
        }
    }

    /// Window ▸ Usage and the popover's Show all.
    pub(super) fn show_resources_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.resources.as_mut() {
            Some(view) => {
                view.shown = Shown::Window;
                window.focus(&view.focus, cx);
                cx.notify();
            }
            None => self.open_resources(Shown::Window, window, cx),
        }
    }

    fn open_resources(&mut self, shown: Shown, window: &mut Window, cx: &mut Context<Self>) {
        match &self.modal {
            // The creator's unsaved edits are never dropped by a shortcut.
            Some(Modal::Creator { .. }) => return,
            Some(Modal::Theme { .. }) => {
                self.theme_preview = None;
                self.apply_settings(window, cx);
                self.modal = None;
            }
            Some(_) => self.modal = None,
            None => {}
        }
        self.usage_open = None;
        let sampler = Arc::new(Mutex::new(Sampler::new()));
        let refresh = cx.spawn_in(window, async move |this, cx| {
            loop {
                let sampler = Arc::clone(&sampler);
                let reading = cx
                    .background_executor()
                    .spawn(async move { read(&sampler) })
                    .await;
                let open = this
                    .update_in(cx, |this, _, cx| {
                        let Some(view) = this.resources.as_mut() else {
                            return false;
                        };
                        match reading {
                            Ok(reading) => {
                                view.reading = Some(reading);
                                view.problem = None;
                            }
                            Err(error) => view.problem = Some(format!("{error:#}")),
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !open {
                    break;
                }
                cx.background_executor().timer(REFRESH).await;
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.resources = Some(ResourcesView {
            shown,
            sort: Sort::Sidebar,
            focus,
            reading: None,
            problem: None,
            _refresh: refresh,
        });
        cx.notify();
    }

    /// Dropping the view ends its refresh task, so nothing is measured once it closes.
    fn close_resources(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.resources = None;
        match &self.pane {
            Pane::Terminal { view, .. } => window.focus(&view.focus_handle(cx), cx),
            Pane::Details(_) => window.focus(&self.pane_focus, cx),
            Pane::Empty => window.focus(&self.sidebar_focus, cx),
        }
        cx.notify();
    }

    fn set_resources_sort(&mut self, sort: Sort, cx: &mut Context<Self>) {
        if let Some(view) = self.resources.as_mut() {
            view.sort = sort;
            cx.notify();
        }
    }

    pub(super) fn render_resources(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let view = self.resources.as_ref()?;
        let sort = match view.shown {
            Shown::Popover => Sort::Sidebar,
            Shown::Window => view.sort,
        };
        let lines = lines(
            &self.sessions,
            &self.groups,
            self.home.as_deref(),
            view.reading.as_ref(),
            sort,
        );
        let content = match view.shown {
            Shown::Popover => self.render_resources_popover(view, &lines, cx),
            Shown::Window => self.render_resources_window(view, &lines, window, cx),
        };
        Some(content)
    }

    fn resources_header(&self, cx: &mut Context<Self>) -> Div {
        let chrome = self.theme.chrome;
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Usage"),
            )
            .child(self.resources_tag(key_label(shortcuts::MAC)))
            .child(div().flex_1())
            .child(
                div()
                    .id("usage-close")
                    .size(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .text_color(hsla(chrome.muted))
                    .cursor_pointer()
                    .hover(move |style| style.text_color(hsla(chrome.text)))
                    .child("✕")
                    .on_click(cx.listener(|this, _, window, cx| this.close_resources(window, cx))),
            )
    }

    fn resources_tag(&self, text: &'static str) -> Div {
        let chrome = self.theme.chrome;
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

    /// A figure column: right-aligned, at its own width.
    fn resources_figure(width: f32, text: String) -> Div {
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .justify_end()
            .child(text)
    }

    /// A session's mark, title and what it includes, sharing the row with figures.
    fn resources_session_cell(&self, line: &Line, under: Option<String>) -> Div {
        let chrome = self.theme.chrome;
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .flex_none()
                    .child(self.marks.element(line.agent, MARK_SIZE, chrome.muted)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().child(line.title.clone()))
                    .children(under.map(|under| {
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(hsla(chrome.muted))
                            .child(under)
                    })),
            )
    }

    fn render_resources_popover(
        &self,
        view: &ResourcesView,
        lines: &[Line],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let mut panel = div()
            .id("usage-popover-panel")
            .track_focus(&view.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_resources(window, cx);
                    cx.stop_propagation();
                }
            }))
            .occlude()
            .absolute()
            .top(px(46.))
            .right(px(12.))
            .w(px(POPOVER_WIDTH))
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(14.))
            .rounded(px(12.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.selected))
            .shadow_lg()
            .text_size(px(13.))
            .text_color(hsla(chrome.text))
            .child(self.resources_header(cx));

        let Some(reading) = &view.reading else {
            return panel
                .child(
                    div().text_color(muted).child(
                        view.problem
                            .clone()
                            .unwrap_or_else(|| "Measuring…".to_owned()),
                    ),
                )
                .into_any_element();
        };
        panel = panel.child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(muted)
                .child(format!("TOP {TOP} BY CPU")),
        );
        let busiest = top(lines);
        if busiest.is_empty() {
            panel = panel.child(div().text_color(muted).child("No session is running."));
        }
        for line in busiest {
            let footprint = line.footprint.unwrap_or_default();
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(self.resources_session_cell(line, child_note(line.footprint)))
                    .child(Self::resources_figure(CPU_WIDTH, cpu_text(footprint.cpu)))
                    .child(Self::resources_figure(
                        MEMORY_WIDTH,
                        memory_text(footprint.memory),
                    )),
            );
        }
        // What Headquarter and every running session use together, the five listed or
        // not (add-usage-total D1).
        let totals = Totals::of(reading, lines);
        let total_line = |label: String, footprint: Footprint, strong: bool| {
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .when(strong, |line| line.font_weight(gpui::FontWeight::SEMIBOLD))
                .child(div().flex_1().child(label))
                .child(Self::resources_figure(CPU_WIDTH, cpu_text(footprint.cpu)))
                .child(Self::resources_figure(
                    MEMORY_WIDTH,
                    memory_text(footprint.memory),
                ))
        };
        panel = panel.child(
            div()
                .pt(px(10.))
                .border_t_1()
                .border_color(hsla(chrome.sidebar_line))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(total_line(
                    "Headquarter".to_owned(),
                    totals.headquarter,
                    false,
                ))
                .child(total_line(
                    sessions_label(totals.running),
                    totals.sessions,
                    false,
                ))
                .child(total_line("Total".to_owned(), totals.total(), true)),
        );
        panel
            .children(
                view.problem
                    .clone()
                    .map(|problem| div().text_color(hsla(chrome.amber)).child(problem)),
            )
            .child(
                div()
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(hsla(chrome.sidebar_line))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id("usage-show-all")
                            .text_color(hsla(chrome.accent))
                            .cursor_pointer()
                            .hover(|style| style.underline())
                            .child(format!("Show all {} sessions", lines.len()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.show_resources_window(window, cx)
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(muted)
                            .child("100% = one core"),
                    ),
            )
            .into_any_element()
    }

    fn render_resources_window(
        &self,
        view: &ResourcesView,
        lines: &[Line],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let width = WINDOW_WIDTH.min(f32::from(window.viewport_size().width) - 2. * MARGIN);

        let figure_pair = |footprint: Option<Footprint>| {
            let big = |text: String, label: &'static str| {
                div()
                    .w(px(MEMORY_WIDTH + 10.))
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(text),
                    )
                    .child(div().text_size(px(11.5)).text_color(muted).child(label))
            };
            div()
                .flex()
                .gap(px(8.))
                .child(big(
                    footprint.map_or_else(|| "…".to_owned(), |footprint| cpu_text(footprint.cpu)),
                    "CPU",
                ))
                .child(big(
                    footprint
                        .map_or_else(|| "…".to_owned(), |footprint| memory_text(footprint.memory)),
                    "memory",
                ))
        };
        let card_row = |label: &'static str, detail: String, footprint: Option<Footprint>| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(16.))
                .py(px(12.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child(label),
                        )
                        .child(div().text_size(px(12.5)).text_color(muted).child(detail)),
                )
                .child(figure_pair(footprint))
        };
        let (_, running, stopped) = all_agents(lines);
        let totals = view
            .reading
            .as_ref()
            .map(|reading| Totals::of(reading, lines));
        let headquarter_detail = view.reading.as_ref().map_or_else(
            || "Measuring…".to_owned(),
            |reading| {
                format!(
                    "app {} · {}, host {} · {}",
                    cpu_text(reading.app.cpu),
                    memory_text(reading.app.memory),
                    cpu_text(reading.host.cpu),
                    memory_text(reading.host.memory)
                )
            },
        );
        let card = div()
            .flex_none()
            .flex()
            .flex_col()
            .rounded(px(12.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.sidebar))
            .child(card_row(
                "HEADQUARTER",
                headquarter_detail,
                totals.map(|totals| totals.headquarter),
            ))
            .child(
                card_row(
                    "ALL SESSIONS",
                    format!("{running} running, {stopped} not running"),
                    totals.map(|totals| totals.sessions),
                )
                .border_t_1()
                .border_color(hsla(chrome.sidebar_line)),
            )
            .child(
                card_row(
                    "TOTAL",
                    "Headquarter and every running session".to_owned(),
                    totals.map(|totals| totals.total()),
                )
                .border_t_1()
                .border_color(hsla(chrome.sidebar_line)),
            );

        let heading = |id: &'static str, text: &'static str, sort: Option<Sort>| {
            let active = sort == Some(view.sort);
            let label = if active && sort != Some(Sort::Sidebar) {
                format!("{text} ↓")
            } else {
                text.to_owned()
            };
            let cell = div()
                .id(id)
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(if active { hsla(chrome.text) } else { muted })
                .child(label);
            match sort {
                Some(sort) => cell
                    .cursor_pointer()
                    .hover(move |style| style.text_color(hsla(chrome.text)))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_resources_sort(sort, cx))),
                None => cell,
            }
        };
        let table_head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(10.))
            .pb(px(6.))
            .border_b_1()
            .border_color(hsla(chrome.sidebar_line))
            .child(div().flex_1().pl(px(MARK_SIZE + 10.)).child(heading(
                "usage-sort-session",
                "Session",
                Some(Sort::Sidebar),
            )))
            .child(
                div()
                    .w(px(AGENT_WIDTH))
                    .child(heading("usage-agent", "Agent", None)),
            )
            .child(
                div()
                    .w(px(STATUS_WIDTH))
                    .child(heading("usage-status", "Status", None)),
            )
            .child(div().w(px(CPU_WIDTH)).flex().justify_end().child(heading(
                "usage-sort-cpu",
                "CPU",
                Some(Sort::Cpu),
            )))
            .child(
                div()
                    .w(px(MEMORY_WIDTH))
                    .flex()
                    .justify_end()
                    .child(heading("usage-sort-memory", "Memory", Some(Sort::Memory))),
            );

        let mut rows = div().flex().flex_col();
        if lines.is_empty() {
            rows = rows.child(
                div()
                    .px(px(10.))
                    .py(px(20.))
                    .text_color(muted)
                    .child("No sessions in the sidebar."),
            );
        }
        for line in lines {
            let under = match child_note(line.footprint) {
                Some(note) => format!("{} · {note}", line.folder),
                None => line.folder.clone(),
            };
            let (cpu, memory) = match (line.running, line.footprint) {
                (false, _) => (String::new(), String::new()),
                (true, None) => ("…".to_owned(), "…".to_owned()),
                (true, Some(footprint)) => (cpu_text(footprint.cpu), memory_text(footprint.memory)),
            };
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    .py(px(7.))
                    .border_b_1()
                    .border_color(hsla(chrome.sidebar_line))
                    // Not running rows are told apart by their word and a muted colour, not
                    // by opacity, which would fail contrast.
                    .when(!line.running, |row| row.text_color(muted))
                    .child(self.resources_session_cell(line, Some(under)))
                    .child(
                        div()
                            .w(px(AGENT_WIDTH))
                            .flex_none()
                            .truncate()
                            .child(agent_name(line.agent)),
                    )
                    .child(
                        div()
                            .w(px(STATUS_WIDTH))
                            .flex_none()
                            .truncate()
                            .child(line.status.clone()),
                    )
                    .child(Self::resources_figure(CPU_WIDTH, cpu))
                    .child(Self::resources_figure(MEMORY_WIDTH, memory)),
            );
        }

        let panel = div()
            .id("usage-window")
            .track_focus(&view.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_resources(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(px(width.max(360.)))
            .max_h(gpui::relative(0.92))
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(18.))
            .rounded(px(14.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.selected))
            .shadow_lg()
            .text_size(px(13.5))
            .text_color(hsla(chrome.text))
            .child(self.resources_header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(table_head)
                    .child(
                        div()
                            .id("usage-rows")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(rows),
                    ),
            )
            .child(card)
            .children(
                view.problem
                    .clone()
                    .map(|problem| div().text_color(hsla(chrome.amber)).child(problem)),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(muted)
                    .child(
                        "CPU: 100% is one core. Each session counts the processes it started and its shell panel. Refreshed every 2 s while open.",
                    ),
            );

        div()
            .id("usage-backdrop")
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
                cx.listener(|this, _, window, cx| this.close_resources(window, cx)),
            )
            .child(panel)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hq_protocol::Origin;
    use hq_signals::status::Status;

    fn session(id: &str, live: bool, activity: u64) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: "/work/app".to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status: if live { Status::Done } else { Status::Inactive },
            needs_reason: None,
            created_at: 0,
            last_activity_at: activity,
            ended_at: None,
            live,
            origin: Origin::Hosted,
            title: None,
            external: None,
            in_sidebar: true,
            group: None,
            owner_named: true,
            starting: false,
            start_error: None,
            offered: true,
            phone: None,
            background: Default::default(),
            review: false,
        }
    }

    fn footprint(cpu: f32, megabytes: u64, processes: u32) -> Footprint {
        Footprint {
            cpu: Some(cpu),
            memory: megabytes << 20,
            processes,
        }
    }

    fn reading() -> Reading {
        Reading {
            app: footprint(1.0, 100, 1),
            host: footprint(0.5, 40, 1),
            sessions: HashMap::from([
                ("light".to_owned(), footprint(2.0, 900, 1)),
                ("busy".to_owned(), footprint(140.0, 300, 4)),
            ]),
        }
    }

    fn ids(lines: &[Line]) -> Vec<&str> {
        lines.iter().map(|line| line.id.as_str()).collect()
    }

    fn sessions() -> Vec<SessionInfo> {
        let mut outside = session("outside", true, 9);
        outside.in_sidebar = false;
        vec![
            session("light", true, 3),
            session("busy", true, 2),
            session("stopped", false, 4),
            outside,
        ]
    }

    #[test]
    fn the_table_lists_sidebar_sessions_and_sorts_by_figures() {
        let sessions = sessions();
        let reading = reading();
        let by = |sort| lines(&sessions, &[], None, Some(&reading), sort);
        assert_eq!(
            ids(&by(Sort::Sidebar)),
            ["stopped", "light", "busy"],
            "the sidebar's order, outside sessions left out"
        );
        assert_eq!(ids(&by(Sort::Cpu)), ["busy", "light", "stopped"]);
        assert_eq!(ids(&by(Sort::Memory)), ["light", "busy", "stopped"]);
        let stopped = &by(Sort::Sidebar)[0];
        assert_eq!(stopped.status, "not running");
        assert_eq!(stopped.footprint, None);
    }

    #[test]
    fn the_total_adds_headquarter_and_every_running_session() {
        let sessions = sessions();
        let reading = reading();
        let lines = lines(&sessions, &[], None, Some(&reading), Sort::Sidebar);
        let totals = Totals::of(&reading, &lines);
        assert_eq!(totals.headquarter.memory, 140 * 1024 * 1024);
        assert_eq!(
            totals.sessions.cpu,
            Some(142.0),
            "the stopped one adds nothing"
        );
        assert_eq!(totals.running, 2);
        let total = totals.total();
        assert_eq!(total.cpu, Some(143.5));
        assert_eq!(total.memory, (140 + 1200) * 1024 * 1024);
        assert_eq!(sessions_label(2), "All 2 sessions");
        assert_eq!(sessions_label(1), "1 session");
    }

    #[test]
    fn the_popover_shows_the_busiest_running_sessions() {
        let mut sessions = sessions();
        for index in 0..6 {
            sessions.push(session(&format!("idle-{index}"), true, 1));
        }
        let mut reading = reading();
        for index in 0..6 {
            reading
                .sessions
                .insert(format!("idle-{index}"), footprint(0.1, 50, 1));
        }
        let all = lines(&sessions, &[], None, Some(&reading), Sort::Sidebar);
        let busiest = top(&all);
        assert_eq!(busiest.len(), TOP);
        assert_eq!(
            ids(&busiest.into_iter().cloned().collect::<Vec<_>>())[..2],
            ["busy", "light"]
        );
    }

    #[test]
    fn totals_and_headquarter_add_up() {
        let sessions = sessions();
        let reading = reading();
        let all = lines(&sessions, &[], None, Some(&reading), Sort::Sidebar);
        let (total, running, stopped) = all_agents(&all);
        assert_eq!(total.cpu, Some(142.0));
        assert_eq!(total.memory, 1200 << 20);
        assert_eq!((running, stopped), (2, 1));
        assert_eq!(reading.headquarter().cpu, Some(1.5));
        assert_eq!(reading.headquarter().memory, 140 << 20);

        let unmeasured = lines(&sessions, &[], None, None, Sort::Sidebar);
        assert_eq!(
            all_agents(&unmeasured).0.cpu,
            None,
            "unknown until measured"
        );
    }

    #[test]
    fn figures_read_like_activity_monitor() {
        assert_eq!(cpu_text(None), "…");
        assert_eq!(cpu_text(Some(0.25)), "0.2%");
        assert_eq!(cpu_text(Some(140.4)), "140%");
        assert_eq!(memory_text(300 << 20), "300 MB");
        assert_eq!(memory_text(1536 << 20), "1.5 GB");
        assert_eq!(child_note(Some(footprint(1.0, 1, 1))), None);
        assert_eq!(
            child_note(Some(footprint(1.0, 1, 2))).as_deref(),
            Some("+1 child process")
        );
        assert_eq!(
            child_note(Some(footprint(1.0, 1, 4))).as_deref(),
            Some("+3 child processes")
        );
        assert_eq!(key_label(true), "⌘U");
        assert_eq!(key_label(false), "Ctrl+Shift+U");
    }
}
