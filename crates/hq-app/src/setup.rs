//! First-run setup: choose agents and a look, write the settings file, then open the
//! sidebar with "Add sessions" (`openspec/changes/add-first-run-setup/design.md`).

use std::collections::HashSet;
use std::path::PathBuf;

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, SharedString, Styled, Task, Window, div, px,
};
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use hq_config::setup::{self, AlacrittyLook, Choices};
use hq_config::{Settings, SettingsError};
use hq_discovery::AgentReport;

use crate::terminal::view::hsla;
use crate::theme::Theme;

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
    /// Installed and hostable: the owner may tick it.
    Choosable,
    NotInstalled,
    NotSupported,
    /// The settings file pins a path that is not an executable.
    Problem(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRow {
    pub id: &'static str,
    pub name: &'static str,
    pub path: Option<String>,
    pub version: Option<String>,
    pub state: RowState,
}

impl AgentRow {
    pub fn note(&self) -> String {
        match &self.state {
            RowState::Choosable => self
                .version
                .clone()
                .unwrap_or_else(|| "version unknown".to_owned()),
            RowState::NotInstalled => "not installed".to_owned(),
            RowState::NotSupported => "found, not yet supported".to_owned(),
            RowState::Problem(problem) => problem.clone(),
        }
    }
}

/// The agents step's rows, and which are ticked: installed hostable agents, unless the
/// existing settings turn them off (design D2).
pub fn agent_rows(
    reports: &[AgentReport],
    current: Option<&Settings>,
) -> (Vec<AgentRow>, HashSet<&'static str>) {
    let mut ticked = HashSet::new();
    let rows = reports
        .iter()
        .map(|report| {
            let agent = report.agent;
            let state = if let Some(problem) = &report.pin_problem {
                RowState::Problem(problem.clone())
            } else if report.used.is_none() {
                RowState::NotInstalled
            } else if !agent.hostable {
                RowState::NotSupported
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
                path: report
                    .used_path()
                    .map(|path| path.to_string_lossy().into_owned()),
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

/// What finishing wrote, for the last step.
enum Outcome {
    Written(PathBuf),
    /// A file appeared while first-run setup was open; it was left alone.
    LeftAlone(PathBuf),
    Edited {
        file: PathBuf,
        backup: PathBuf,
        kept: Vec<String>,
    },
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
    use_alacritty: bool,
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
        Setup {
            step: Step::Agents,
            file,
            existing,
            rows: None,
            ticked: HashSet::new(),
            alacritty,
            use_alacritty: false,
            outcome: None,
            theme,
            focus,
            _discovery: discovery,
        }
    }

    fn look(&self) -> Option<&AlacrittyLook> {
        if !self.use_alacritty {
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
                    Ok(true) => Outcome::Written(self.file.clone()),
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
                            Ok(()) => Outcome::Edited {
                                file: self.file.clone(),
                                backup,
                                kept: edited.kept,
                            },
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
            _ => return,
        }
        cx.stop_propagation();
    }

    fn render_agents(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let chrome = self.theme.chrome;
        let Some(rows) = &self.rows else {
            return div()
                .text_color(hsla(chrome.muted))
                .child("Looking for installed agents…")
                .into_any_element();
        };
        let mut list = div().flex().flex_col().gap(px(10.));
        for row in rows {
            let choosable = row.state == RowState::Choosable;
            let id = row.id;
            let detail = match &row.path {
                Some(path) if choosable => format!("{} · {path}", row.note()),
                _ => row.note(),
            };
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        Checkbox::new(SharedString::from(format!("agent-{id}")))
                            .label(format!("{} ({id})", row.name))
                            .checked(self.ticked.contains(id))
                            .disabled(!choosable)
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                if *checked {
                                    this.ticked.insert(id);
                                } else {
                                    this.ticked.remove(id);
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .pl(px(24.))
                            .text_size(px(12.5))
                            .text_color(hsla(chrome.muted))
                            .truncate()
                            .child(detail),
                    ),
            );
        }
        list.into_any_element()
    }

    fn render_look(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let chrome = self.theme.chrome;
        let option = |id: &'static str, label: &'static str, chosen: bool| {
            let button = Button::new(id).label(label);
            if chosen { button.primary() } else { button }
        };
        let mut panel = div().flex().flex_col().gap(px(12.)).child(
            div()
                .flex()
                .gap(px(8.))
                .child(
                    option("look-headquarter", "Headquarter", !self.use_alacritty).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.use_alacritty = false;
                            cx.notify();
                        }),
                    ),
                )
                .children(match &self.alacritty {
                    Some(Ok(_)) => Some(
                        option(
                            "look-alacritty",
                            "Use my Alacritty look",
                            self.use_alacritty,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.use_alacritty = true;
                            cx.notify();
                        })),
                    ),
                    Some(Err(_)) => Some(
                        option("look-alacritty", "Use my Alacritty look", false).disabled(true),
                    ),
                    None => None,
                }),
        );
        let note = |text: String| {
            div()
                .text_size(px(13.))
                .text_color(hsla(chrome.muted))
                .child(text)
        };
        panel = panel.child(note(if self.use_alacritty {
            "Window opacity and blur, padding, buttons, font, cursor, colours, key bindings and Option as Alt carry over from your Alacritty settings.".to_owned()
        } else {
            "The reference light and dark themes, following the system.".to_owned()
        }));
        match &self.alacritty {
            Some(Ok(look)) if self.use_alacritty => {
                let names = not_carried_names(&look.not_carried);
                if !names.is_empty() {
                    panel = panel.child(note(format!("Not carried over: {}.", names.join(", "))));
                }
            }
            Some(Err(error)) => {
                panel = panel.child(note(format!(
                    "Your Alacritty settings cannot be used: {error}"
                )));
            }
            _ => {}
        }
        panel.into_any_element()
    }

    fn render_done(&self) -> gpui::AnyElement {
        let chrome = self.theme.chrome;
        let line = |text: String| div().text_color(hsla(chrome.text)).child(text);
        let muted = |text: String| {
            div()
                .text_size(px(13.))
                .text_color(hsla(chrome.muted))
                .child(text)
        };
        let mut panel = div().flex().flex_col().gap(px(8.));
        match &self.outcome {
            Some(Outcome::Written(file)) => {
                panel = panel.child(line(format!("Settings saved to {}.", file.display())));
            }
            Some(Outcome::LeftAlone(file)) => {
                panel = panel.child(line(format!(
                    "{} appeared while setup was open, so it was left as it is.",
                    file.display()
                )));
            }
            Some(Outcome::Edited { file, backup, kept }) => {
                panel = panel
                    .child(line(format!("Settings updated in {}.", file.display())))
                    .child(muted(format!(
                        "The previous file is kept as {}.",
                        backup.display()
                    )));
                if !kept.is_empty() {
                    panel = panel.child(muted(format!(
                        "Kept because you changed them after the import: {}.",
                        kept.join(", ")
                    )));
                }
            }
            Some(Outcome::Failed(message)) => {
                panel = panel.child(div().text_color(hsla(chrome.amber)).child(message.clone()));
            }
            None => {}
        }
        panel
            .child(muted(
                "Next: choose the sessions to keep in your sidebar.".to_owned(),
            ))
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
        assert_eq!(rows[0].state, RowState::Choosable);
        assert_eq!(rows[2].state, RowState::NotInstalled);
        assert_eq!(rows[3].state, RowState::NotSupported);
        assert_eq!(rows[3].note(), "found, not yet supported");
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
