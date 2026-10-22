//! First-run setup: choose agents and a look, write the settings file, then open the
//! sidebar with "Add sessions" (`openspec/changes/add-first-run-setup/design.md`).

use std::collections::HashSet;
use std::path::PathBuf;

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Task,
    Window, WindowAppearance, div, px,
};
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use hq_config::setup::{self, AlacrittyLook, Choices};
use hq_config::{ColorSettings, Settings, SettingsError};
use hq_discovery::AgentReport;
use hq_signals::event::Agent;

use crate::marks::{self, Marks};
use crate::modal::theme_switcher::{self, ThemeRow};
use crate::terminal::view::hsla;
use crate::theme::{Mode, Paint, Theme};
use crate::themes::{self, Library, ThemeChoice};

/// Whether setup replaces the sidebar at launch (design D1).
pub fn should_show(settings_file_exists: bool, asked: bool) -> bool {
    asked || !settings_file_exists
}

/// `--setup` on the command line.
pub fn asked_from_args() -> bool {
    std::env::args().any(|arg| arg == "--setup")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowState {
    /// Installed: the owner may tick it.
    Choosable,
    NotInstalled,
    /// The settings file pins a path that is not an executable.
    Problem(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRow {
    pub id: &'static str,
    pub name: &'static str,
    pub agent: Option<Agent>,
    pub version: Option<String>,
    pub install_url: Option<&'static str>,
    pub state: RowState,
}

/// The agents step's rows, and which are ticked: only the agents Headquarter can host
/// (owner, 2026-10-22), ticked when installed unless the existing settings turn them
/// off (design D2).
pub fn agent_rows(
    reports: &[AgentReport],
    current: Option<&Settings>,
) -> (Vec<AgentRow>, HashSet<&'static str>) {
    let mut ticked = HashSet::new();
    let rows = reports
        .iter()
        .filter(|report| report.agent.hostable)
        .map(|report| {
            let agent = report.agent;
            let state = if let Some(problem) = &report.pin_problem {
                RowState::Problem(problem.clone())
            } else if report.used.is_none() {
                RowState::NotInstalled
            } else {
                RowState::Choosable
            };
            let enabled = current.is_none_or(|settings| settings.agent_enabled(agent.id));
            if state == RowState::Choosable && enabled {
                ticked.insert(agent.id);
            }
            AgentRow {
                id: agent.id,
                name: agent.name,
                agent: Agent::from_id(agent.id),
                install_url: agent.install_url,
                version: report.version.clone(),
                state,
            }
        })
        .collect();
    (rows, ticked)
}

/// Agents to turn off and on, from the rows the owner could choose.
pub fn agent_choices(
    rows: &[AgentRow],
    ticked: &HashSet<&'static str>,
) -> (Vec<String>, Vec<String>) {
    let mut off = Vec::new();
    let mut on = Vec::new();
    for row in rows.iter().filter(|row| row.state == RowState::Choosable) {
        if ticked.contains(row.id) {
            on.push(row.id.to_owned());
        } else {
            off.push(row.id.to_owned());
        }
    }
    (off, on)
}

/// The look step's rows: the theme switcher's, with its Alacritty row standing for the
/// whole Alacritty look (owner, 2026-10-22): shown when an Alacritty file exists, and
/// not choosable when it cannot be read.
pub fn look_rows(
    system: Mode,
    alacritty: Option<&Result<AlacrittyLook, SettingsError>>,
    library: &Library,
) -> Vec<ThemeRow> {
    let colors = alacritty_colors(alacritty);
    theme_switcher::rows(system, &colors, "", library)
        .into_iter()
        .filter_map(|mut row| {
            if row.choice != ThemeChoice::Alacritty {
                return Some(row);
            }
            row.name = "Your Alacritty look".to_owned();
            match alacritty? {
                Ok(_) => {
                    row.detail = Some(
                        "Window, padding, font, cursor, colours and keys from your Alacritty settings"
                            .to_owned(),
                    );
                    row.available = true;
                }
                Err(error) => {
                    row.detail = Some(format!("Cannot be used: {error}"));
                    row.available = false;
                }
            }
            Some(row)
        })
        .collect()
}

fn alacritty_colors(alacritty: Option<&Result<AlacrittyLook, SettingsError>>) -> ColorSettings {
    match alacritty {
        Some(Ok(look)) => look.appearance.colors.clone(),
        _ => ColorSettings::default(),
    }
}

/// The Alacritty setting names that do not carry over, from their warnings.
pub fn not_carried_names(warnings: &[SettingsError]) -> Vec<String> {
    warnings
        .iter()
        .filter_map(|warning| {
            let start = warning.message.find('`')? + 1;
            let end = start + warning.message[start..].find('`')?;
            Some(warning.message[start..end].to_owned())
        })
        .collect()
}

/// Runs discovery as `hq agents` does, with the settings file's pins (design D2).
pub(crate) fn discover(current: Option<&Settings>) -> Vec<AgentReport> {
    let login_shell = current.is_none_or(|settings| settings.login_shell);
    let login_path = login_shell
        .then(|| hq_discovery::shell::login_shell_path(hq_discovery::shell::LOGIN_SHELL_TIMEOUT))
        .flatten();
    let context =
        hq_discovery::SearchContext::current(hq_discovery::platform::detect(), login_path);
    let mut reports: Vec<AgentReport> = hq_discovery::KNOWN_AGENTS
        .iter()
        .map(|agent| {
            let pin = current.and_then(|settings| settings.agent_path(agent.id));
            hq_discovery::search(&context, agent, pin)
        })
        .collect();
    hq_discovery::add_versions(&mut reports, hq_discovery::search::VERSION_TIMEOUT);
    reports
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Agents,
    Look,
    Done,
}

/// What finishing did, for the last step. The file's path and backup are not shown
/// (owner, 2026-10-22: the Ready step says what to do next instead).
enum Outcome {
    Saved,
    /// A file appeared while first-run setup was open; it was left alone.
    LeftAlone(PathBuf),
    Failed(String),
}

pub enum SetupEvent {
    Finished,
}

pub struct Setup {
    step: Step,
    file: PathBuf,
    /// The settings file as it was when setup opened; `None` on first run.
    existing: Option<(String, Settings)>,
    rows: Option<Vec<AgentRow>>,
    ticked: HashSet<&'static str>,
    alacritty: Option<Result<AlacrittyLook, SettingsError>>,
    /// The look step's themes, and the one chosen, shown on this screen as it is chosen.
    looks: Vec<ThemeRow>,
    chosen: usize,
    system: Mode,
    library: Library,
    /// The agents' marks, as the sidebar draws them; drawn marks until fetched.
    marks: Marks,
    outcome: Option<Outcome>,
    theme: Theme,
    focus: FocusHandle,
    _discovery: Task<()>,
}

impl EventEmitter<SetupEvent> for Setup {}

impl Setup {
    pub fn new(file: PathBuf, theme: Theme, window: &mut Window, cx: &mut Context<Self>) -> Setup {
        let known: Vec<&str> = hq_discovery::KNOWN_AGENTS
            .iter()
            .map(|agent| agent.id)
            .collect();
        let existing = std::fs::read_to_string(&file).ok().and_then(|text| {
            hq_config::parse(&text, &file, &known, hq_signals::paths::home().as_deref())
                .ok()
                .map(|settings| (text, settings))
        });
        let home = hq_signals::paths::home();
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let alacritty = setup::alacritty_path(config_home.as_deref(), home.as_deref())
            .map(|path| setup::read_alacritty(&path, home.as_deref()));
        let current = existing.as_ref().map(|(_, settings)| settings.clone());
        let found = cx
            .background_executor()
            .spawn(async move { discover(current.as_ref()) });
        let discovery = cx.spawn(async move |this, cx| {
            let reports = found.await;
            let _ = this.update(cx, |setup, cx| {
                let current = setup.existing.as_ref().map(|(_, settings)| settings);
                let (rows, ticked) = agent_rows(&reports, current);
                setup.rows = Some(rows);
                setup.ticked = ticked;
                cx.notify();
            });
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let system = match window.appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Mode::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Mode::Light,
        };
        let library = Library::load(hq_config::themes::themes_dir().as_deref());
        let looks = look_rows(system, alacritty.as_ref(), &library);
        let current = crate::app_state::path()
            .and_then(|path| crate::app_state::load_theme(&path))
            .and_then(|key| ThemeChoice::from_key(&key))
            .unwrap_or(ThemeChoice::System);
        let chosen = looks
            .iter()
            .position(|row| row.choice == current && row.available)
            .unwrap_or(0);
        let mut setup = Setup {
            step: Step::Agents,
            file,
            existing,
            rows: None,
            ticked: HashSet::new(),
            alacritty,
            looks,
            chosen,
            system,
            library,
            marks: Marks::default(),
            outcome: None,
            theme,
            focus,
            _discovery: discovery,
        };
        setup.theme = setup.preview();
        setup.load_marks(cx);
        setup
    }

    /// The one-colour marks the sidebar uses by default, fetching any missing.
    fn load_marks(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = marks::dir() else {
            return;
        };
        self.marks = Marks::load(&dir, marks::Style::Mono);
        self.marks.set_ink(self.ink());
        let fetch = cx
            .background_executor()
            .spawn(async move { marks::fetch_missing(&dir, marks::Style::Mono) });
        cx.spawn(async move |this, cx| {
            let fetched = fetch.await;
            let _ = this.update(cx, |setup, cx| {
                setup.marks = fetched;
                setup.marks.set_ink(setup.ink());
                cx.notify();
            });
        })
        .detach();
    }

    /// One-colour marks in black on a light theme, white on a dark one.
    fn ink(&self) -> alacritty_terminal::vte::ansi::Rgb {
        let level = match self.theme.mode {
            Mode::Light => 0,
            Mode::Dark => 255,
        };
        alacritty_terminal::vte::ansi::Rgb {
            r: level,
            g: level,
            b: level,
        }
    }

    fn chosen_theme(&self) -> ThemeChoice {
        self.looks
            .get(self.chosen)
            .map_or(ThemeChoice::System, |row| row.choice.clone())
    }

    /// The chosen theme as it looks, for this screen.
    fn preview(&self) -> Theme {
        let colors = alacritty_colors(self.alacritty.as_ref());
        themes::look(
            Some(&self.chosen_theme()),
            self.system,
            &colors,
            &self.library,
        )
        .theme
    }

    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.looks.get(index).is_some_and(|row| row.available) {
            self.chosen = index;
            self.theme = self.preview();
            self.marks.set_ink(self.ink());
            cx.notify();
        }
    }

    /// The next choosable theme `step` rows away, staying put at either end.
    fn move_choice(&mut self, step: isize, cx: &mut Context<Self>) {
        let mut index = self.chosen as isize;
        loop {
            index += step;
            let Some(row) = usize::try_from(index)
                .ok()
                .and_then(|at| self.looks.get(at))
            else {
                return;
            };
            if row.available {
                self.choose(index as usize, cx);
                return;
            }
        }
    }

    /// The Alacritty look to write, when the owner chose it.
    fn look(&self) -> Option<&AlacrittyLook> {
        if self.chosen_theme() != ThemeChoice::Alacritty {
            return None;
        }
        self.alacritty.as_ref().and_then(|look| look.as_ref().ok())
    }

    fn previous_look(&self) -> Option<&AlacrittyLook> {
        self.alacritty.as_ref().and_then(|look| look.as_ref().ok())
    }

    /// Writes the file (design D4) and moves to the last step.
    fn finish(&mut self, cx: &mut Context<Self>) {
        let (agents_off, agents_on) = match &self.rows {
            Some(rows) => agent_choices(rows, &self.ticked),
            None => (Vec::new(), Vec::new()),
        };
        let choices = Choices {
            agents_off,
            agents_on,
            look: self.look(),
        };
        let now = chrono::Local::now();
        let known: Vec<&str> = hq_discovery::KNOWN_AGENTS
            .iter()
            .map(|agent| agent.id)
            .collect();
        let outcome = match &self.existing {
            None => {
                let text = setup::render(&choices, &now.format("%Y-%m-%d").to_string());
                match setup::write_new(&self.file, &text) {
                    Ok(true) => Outcome::Saved,
                    Ok(false) => Outcome::LeftAlone(self.file.clone()),
                    Err(error) => {
                        Outcome::Failed(format!("cannot write {}: {error}", self.file.display()))
                    }
                }
            }
            Some((text, _)) => {
                match setup::edit(text, &self.file, &choices, self.previous_look(), &known) {
                    Ok(edited) => {
                        let backup = setup::backup_path(
                            &self.file,
                            &now.format("%Y%m%d-%H%M%S").to_string(),
                        );
                        match setup::replace_with_copy(&self.file, &backup, &edited.text) {
                            Ok(()) => Outcome::Saved,
                            Err(error) => Outcome::Failed(format!(
                                "cannot write {}: {error}",
                                self.file.display()
                            )),
                        }
                    }
                    Err(error) => Outcome::Failed(format!("nothing was written: {error}")),
                }
            }
        };
        if !matches!(outcome, Outcome::Failed(_) | Outcome::LeftAlone(_))
            && let Some(path) = crate::app_state::path()
            && let Err(error) = crate::app_state::save_theme(&path, &self.chosen_theme().key())
        {
            log::warn!("setup could not keep the chosen theme: {error}");
        }
        self.outcome = Some(outcome);
        self.step = Step::Done;
        cx.notify();
    }

    /// Skip: on first run, a defaults file so setup does not return; on a rerun,
    /// nothing changes.
    fn skip(&mut self, cx: &mut Context<Self>) {
        if self.existing.is_none() {
            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
            let text = setup::render(&Choices::default(), &date);
            if let Err(error) = setup::write_new(&self.file, &text) {
                self.outcome = Some(Outcome::Failed(format!(
                    "cannot write {}: {error}",
                    self.file.display()
                )));
                self.step = Step::Done;
                cx.notify();
                return;
            }
        }
        cx.emit(SetupEvent::Finished);
    }

    fn advance(&mut self, cx: &mut Context<Self>) {
        match self.step {
            Step::Agents => {
                self.step = Step::Look;
                cx.notify();
            }
            Step::Look => self.finish(cx),
            Step::Done => cx.emit(SetupEvent::Finished),
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        if self.step == Step::Look {
            self.step = Step::Agents;
            cx.notify();
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => self.advance(cx),
            "escape" => self.back(cx),
            "up" if self.step == Step::Look => self.move_choice(-1, cx),
            "down" if self.step == Step::Look => self.move_choice(1, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    fn render_agents(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let chrome = self.theme.chrome;
        let Some(rows) = &self.rows else {
            return crate::loading::agents_checking_in(
                "setup-finding",
                &self.marks,
                chrome,
                "Looking for your agents…",
            );
        };
        let mut list = div().flex().flex_col().gap(px(4.));
        for row in rows {
            let choosable = row.state == RowState::Choosable;
            let id = row.id;
            let mark = row
                .agent
                .map(|agent| self.marks.element(agent, 24., chrome.muted));
            let trailing = match &row.state {
                RowState::Choosable => div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .children(row.version.clone().map(|version| {
                        div()
                            .text_size(px(12.5))
                            .text_color(hsla(chrome.muted))
                            .child(version)
                    }))
                    .child(
                        Checkbox::new(SharedString::from(format!("agent-{id}")))
                            .checked(self.ticked.contains(id))
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                if *checked {
                                    this.ticked.insert(id);
                                } else {
                                    this.ticked.remove(id);
                                }
                                cx.notify();
                            })),
                    )
                    .into_any_element(),
                RowState::NotInstalled => match row.install_url {
                    Some(url) => div()
                        .id(SharedString::from(format!("install-{id}")))
                        .text_size(px(12.5))
                        .text_color(hsla(chrome.accent))
                        .cursor_pointer()
                        .child("How to install")
                        .on_click(move |_, _, cx| cx.open_url(url))
                        .into_any_element(),
                    None => div().into_any_element(),
                },
                RowState::Problem(problem) => div()
                    .max_w(px(260.))
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(hsla(chrome.amber))
                    .child(problem.clone())
                    .into_any_element(),
            };
            let mut line = div()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(10.))
                .py(px(8.))
                .rounded(px(10.))
                .bg(crate::modal::tint(chrome, 0.04))
                .children(mark)
                .child(
                    div()
                        .flex_1()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(row.name),
                )
                .child(trailing);
            if !choosable {
                line = line.opacity(0.6);
            }
            list = list.child(line);
        }
        list.child(
            div()
                .pt(px(6.))
                .text_size(px(13.))
                .text_color(hsla(chrome.muted))
                .child("More agents are on the way."),
        )
        .into_any_element()
    }

    fn render_look(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let chrome = self.theme.chrome;
        let swatch = |color: Paint| {
            div()
                .size(px(14.))
                .rounded_full()
                .border_1()
                .border_color(hsla(chrome.track))
                .bg(hsla(color))
        };
        let mut list = div()
            .id("setup-themes")
            .max_h(px(320.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.));
        for (index, row) in self.looks.iter().enumerate() {
            if let Some(section) = row.section {
                list = list.child(
                    div()
                        .pt(px(8.))
                        .pb(px(2.))
                        .px(px(10.))
                        .text_size(px(12.))
                        .text_color(hsla(chrome.muted))
                        .child(section),
                );
            }
            let chosen = index == self.chosen;
            let mut line = div()
                .id(SharedString::from(format!("setup-theme-{index}")))
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(8.))
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .gap(px(4.))
                        .children(row.swatches.into_iter().map(swatch)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().truncate().child(row.name.clone()))
                        .children(row.detail.clone().map(|detail| {
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(hsla(chrome.muted))
                                .child(detail)
                        })),
                )
                .children(
                    chosen.then(|| div().flex_none().text_color(hsla(chrome.accent)).child("✓")),
                );
            if chosen {
                line = line.bg(crate::modal::tint(chrome, 0.16));
            }
            if row.available {
                let hover = crate::modal::tint(chrome, if chosen { 0.2 } else { 0.06 });
                line = line
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .on_click(cx.listener(move |this, _, _, cx| this.choose(index, cx)));
            } else {
                line = line.opacity(0.55);
            }
            list = list.child(line);
        }
        let mut panel = div().flex().flex_col().gap(px(10.)).child(list);
        if let Some(look) = self.look() {
            let names = not_carried_names(&look.not_carried);
            if !names.is_empty() {
                panel = panel.child(
                    div()
                        .text_size(px(13.))
                        .text_color(hsla(chrome.muted))
                        .child(format!("Not carried over: {}.", names.join(", "))),
                );
            }
        }
        panel
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(hsla(chrome.muted))
                    .child("Change it any time with Themes (⌘G)."),
            )
            .into_any_element()
    }

    fn render_done(&self) -> gpui::AnyElement {
        let chrome = self.theme.chrome;
        let problem = match &self.outcome {
            Some(Outcome::Failed(message)) => Some(message.clone()),
            Some(Outcome::LeftAlone(file)) => Some(format!(
                "{} appeared while setup was open, so it was left as it is.",
                file.display()
            )),
            _ => None,
        };
        if let Some(problem) = problem {
            return div()
                .text_color(hsla(chrome.amber))
                .child(problem)
                .into_any_element();
        }
        let key = |keys: &'static str, what: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    div()
                        .min_w(px(28.))
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(6.))
                        .border_1()
                        .border_b_2()
                        .border_color(hsla(chrome.sidebar_line))
                        .text_size(px(12.))
                        .flex()
                        .justify_center()
                        .child(keys),
                )
                .child(what)
        };
        let (new, add, next) = if cfg!(target_os = "macos") {
            ("⌘N", "⌘O", "⌘J")
        } else {
            ("Ctrl-Shift-N", "Ctrl-Shift-O", "Ctrl-Shift-J")
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(key(new, "New session"))
            .child(key(add, "Add sessions"))
            .child(key(next, "Next one that needs you"))
            .into_any_element()
    }
}

impl Focusable for Setup {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Setup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.theme.chrome;
        let (title, body) = match self.step {
            Step::Agents => (
                "Which agents should Headquarter use?",
                self.render_agents(cx),
            ),
            Step::Look => ("How should it look?", self.render_look(cx)),
            Step::Done => ("Ready", self.render_done()),
        };
        let step_number = match self.step {
            Step::Agents => 1,
            Step::Look => 2,
            Step::Done => 3,
        };
        let continue_label = match self.step {
            Step::Agents => "Continue",
            Step::Look => "Save settings",
            Step::Done => "Open Headquarter",
        };
        let rerun = self.existing.is_some();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(self.theme.palette.background))
            .child(
                div()
                    .id("setup")
                    .key_context("Setup")
                    .track_focus(&self.focus)
                    .on_key_down(cx.listener(Self::key_down))
                    .w(px(560.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .p(px(28.))
                    .rounded(px(16.))
                    .border_1()
                    .border_color(hsla(chrome.sidebar_line))
                    .bg(hsla(chrome.sidebar))
                    .text_size(px(14.))
                    .text_color(hsla(chrome.text))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(hsla(chrome.muted))
                            .child(format!("Headquarter setup · step {step_number} of 3")),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .pt(px(8.))
                            .children((self.step == Step::Look).then(|| {
                                Button::new("setup-back")
                                    .label("Back")
                                    .on_click(cx.listener(|this, _, _, cx| this.back(cx)))
                            }))
                            .children((self.step != Step::Done).then(|| {
                                Button::new("setup-skip")
                                    .label(if rerun { "Cancel" } else { "Skip" })
                                    .on_click(cx.listener(|this, _, _, cx| this.skip(cx)))
                            }))
                            .child(div().flex_1())
                            .child(
                                Button::new("setup-continue")
                                    .primary()
                                    .label(continue_label)
                                    .disabled(self.step == Step::Agents && self.rows.is_none())
                                    .on_click(cx.listener(|this, _, _, cx| this.advance(cx))),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use hq_discovery::search::{Found, Source};

    use super::*;

    fn report(id: &str, installed: bool) -> AgentReport {
        let agent = hq_discovery::known(id).unwrap();
        let found = if installed {
            vec![Found {
                path: PathBuf::from(format!("/opt/bin/{id}")),
                target: None,
                source: Source::ProcessPath,
            }]
        } else {
            Vec::new()
        };
        AgentReport {
            agent,
            used: installed.then_some(0),
            found,
            searched: Vec::new(),
            version: installed.then(|| "1.0".to_owned()),
            pin_problem: None,
        }
    }

    #[test]
    fn launch_rule() {
        assert!(should_show(false, false), "no settings file");
        assert!(!should_show(true, false));
        assert!(should_show(true, true), "--setup");
    }

    #[test]
    fn installed_hostable_agents_are_ticked() {
        let reports = [
            report("claude", true),
            report("codex", true),
            report("agy", false),
            report("gemini", true),
        ];
        let (rows, ticked) = agent_rows(&reports, None);
        let ids: Vec<&str> = rows.iter().map(|row| row.id).collect();
        assert_eq!(
            ids,
            ["claude", "codex", "agy"],
            "agents not yet hosted are left out"
        );
        assert_eq!(rows[0].state, RowState::Choosable);
        assert_eq!(rows[2].state, RowState::NotInstalled);
        assert_eq!(ticked, HashSet::from(["claude", "codex"]));

        let mut ticked = ticked;
        ticked.remove("codex");
        let (off, on) = agent_choices(&rows, &ticked);
        assert_eq!(off, ["codex"]);
        assert_eq!(on, ["claude"]);
    }

    #[test]
    fn rerun_starts_from_the_current_settings() {
        let settings = hq_config::parse(
            "[agents.codex]\nenabled = false\n",
            Path::new("/c/config.toml"),
            &["claude", "codex", "agy"],
            None,
        )
        .unwrap();
        let (_, ticked) = agent_rows(
            &[report("claude", true), report("codex", true)],
            Some(&settings),
        );
        assert_eq!(ticked, HashSet::from(["claude"]));
    }

    #[test]
    fn the_alacritty_look_is_one_theme_among_the_others() {
        let library = Library::load(None);
        let without = look_rows(Mode::Dark, None, &library);
        assert!(
            without
                .iter()
                .all(|row| row.choice != ThemeChoice::Alacritty),
            "no Alacritty file, no Alacritty row"
        );
        assert_eq!(without[0].choice, ThemeChoice::System);

        let broken = Err(SettingsError {
            file: PathBuf::from("/a/alacritty.toml"),
            line: Some(3),
            message: "bad value".to_owned(),
        });
        let with = look_rows(Mode::Dark, Some(&broken), &library);
        let row = with
            .iter()
            .find(|row| row.choice == ThemeChoice::Alacritty)
            .unwrap();
        assert_eq!(row.name, "Your Alacritty look");
        assert!(!row.available, "an unreadable file cannot be chosen");
        assert_eq!(with.len(), without.len() + 1);
    }

    #[test]
    fn names_of_settings_that_do_not_carry_over() {
        let warning = |message: &str| SettingsError {
            file: PathBuf::from("/a/alacritty.toml"),
            line: Some(5),
            message: message.to_owned(),
        };
        let names = not_carried_names(&[
            warning("`env` is an Alacritty setting Headquarter does not support; it has no effect"),
            warning("`cursor.style.blinking` is an Alacritty setting Headquarter does not support"),
        ]);
        assert_eq!(names, ["env", "cursor.style.blinking"]);
    }
}
