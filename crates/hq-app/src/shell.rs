//! The window's root: setup on first launch (or with `--setup`), then the workspace.

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    WindowAppearance, div,
};

use crate::settings;
use crate::setup::{Setup, SetupEvent};
use crate::theme::{Mode, Theme};
use crate::workspace::Workspace;

enum Screen {
    Setup {
        view: Entity<Setup>,
        _finished: Subscription,
    },
    Workspace(Entity<Workspace>),
}

pub struct Shell {
    screen: Screen,
}

impl Shell {
    pub fn new(show_setup: bool, window: &mut Window, cx: &mut Context<Self>) -> Shell {
        let screen = match settings::path().filter(|_| show_setup) {
            Some(file) => {
                let mode = system_mode(window.appearance());
                // The workspace sets the controls' mode once it opens; setup comes first.
                settings::set_controls_mode(mode, window, cx);
                let setup = cx.new(|cx| Setup::new(file, Theme::for_mode(mode), window, cx));
                let finished = cx.subscribe_in(&setup, window, |this, _, event, window, cx| {
                    let SetupEvent::Finished = event;
                    this.screen = Screen::Workspace(workspace(true, window, cx));
                    cx.notify();
                });
                Screen::Setup {
                    view: setup,
                    _finished: finished,
                }
            }
            None => Screen::Workspace(workspace(false, window, cx)),
        };
        Shell { screen }
    }
}

fn system_mode(appearance: WindowAppearance) -> Mode {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Mode::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => Mode::Light,
    }
}

/// The workspace with the settings as they are now; after setup, "Add sessions" opens
/// so the owner can fill the sidebar at once.
fn workspace(open_picker: bool, window: &mut Window, cx: &mut Context<Shell>) -> Entity<Workspace> {
    let mut watcher = settings::watcher();
    let (appearance, note) = settings::read(&mut watcher, None);
    if let Some(note) = &note {
        eprintln!("headquarter-app: {note}");
    }
    let workspace = cx.new(|cx| Workspace::new(watcher, appearance, note, window, cx));
    if open_picker {
        workspace.update(cx, |workspace, cx| workspace.show_open_sessions(window, cx));
    }
    workspace
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let child = match &self.screen {
            Screen::Setup { view, .. } => view.clone().into_any_element(),
            Screen::Workspace(workspace) => workspace.clone().into_any_element(),
        };
        div().size_full().child(child)
    }
}
