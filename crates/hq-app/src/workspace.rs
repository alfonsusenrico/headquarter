//! The window's content: the session sidebar beside the terminal pane
//! (`openspec/changes/add-session-sidebar/design.md`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use alacritty_terminal::vte::ansi::Rgb;
use anyhow::anyhow;
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use gpui::prelude::FluentBuilder;
use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription,
    SystemNotification, Task, Window, WindowAppearance, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::Scrollbar;
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Icon, IconName, Sizable};
use hq_client::{Change, FeedControl, HostPaths, SessionFeed};
use hq_config::{AgentMarks, Appearance, SettingsWatcher};
use hq_protocol::{AgentUsage, GroupInfo, SessionInfo, usage};
use hq_signals::details::SessionDetails;
use hq_signals::event::Agent;
use hq_signals::status::Status;

use crate::app_state::SidebarState;
use crate::marks::{self, Marks};
use crate::menus;
use crate::modal::new_session::{NewEvent, NewSession, Provider};
use crate::modal::open_sessions::{OpenEvent, OpenSessions};
use crate::modal::theme_creator::{CreatorEvent, CreatorStart, ThemeCreator};
use crate::modal::theme_switcher::{self, ThemeEvent, ThemeSwitcher};
use crate::modal::{self};
use crate::notifications;
use crate::settings;
use crate::shortcuts::{self, Keys, Shortcut};
use crate::sidebar::glyph;
use crate::sidebar::model::{self, Item, Row, Target};
use crate::terminal::session::TerminalSession;
use crate::terminal::view::{PaneOptions, TerminalView, hsla};
use crate::theme::{Chrome, Mode, Theme};
use crate::themes::{self, Library, Look, ThemeChoice, Token};

mod groups;
mod header;
mod rail;
mod remote;
mod shell;
mod sidebar_view;
mod update;

use groups::{DraggedSession, GroupRename};

/// How far a group's rows sit in from its header, where the group's colour line runs.
const GROUP_LINE_INDENT: f32 = 12.0;
/// Ages ("3m") are redrawn this often.
const CLOCK: Duration = Duration::from_secs(30);
const ROW_HEIGHT: f32 = 40.0;
/// The gap between the parts of a row or group header.
const ROW_GAP: f32 = 8.0;
/// The ✕ that slides in on hover, and how long it takes.
const CLOSE_SLOT: f32 = 22.0;
const SLIDE: Duration = Duration::from_millis(150);
/// The agents' marks in the pane while the host starts.
const STARTING_MARK: f32 = 44.0;
/// How much of an inactive row's age shows, beside its muted title.
const DIMMED: f32 = 0.6;
/// A usage ring and the gap between rings.
const RING_PX: f32 = 36.0;
const RING_GAP: f32 = 6.0;
/// The backdrop's close and the ring's click come from one press within this time.
const DISMISS_CLICK: Duration = Duration::from_millis(500);
/// The widest a note in the pane runs before it wraps.
const PANE_TEXT_WIDTH: f32 = 480.0;
/// The usage details' narrowest width; they float over the pane past a narrow sidebar.
const USAGE_PANEL_WIDTH: f32 = 300.0;
/// Room for a usage window's name ("5h", "week") in the usage details.
const WINDOW_NAME_PX: f32 = 36.0;

enum ListEvent {
    All {
        sessions: Vec<SessionInfo>,
        groups: Vec<GroupInfo>,
        usage: Vec<AgentUsage>,
    },
    Changed(SessionInfo),
    Groups(Vec<GroupInfo>),
    /// The handle for telling the host what the window shows.
    Control(FeedControl),
    Details {
        session: String,
        details: SessionDetails,
    },
    Usage(Vec<AgentUsage>),
    Failed(String),
}

/// The session window shown over the workspace; opening one replaces the other.
enum Modal {
    Open {
        view: Entity<OpenSessions>,
        _events: Subscription,
    },
    New {
        view: Entity<NewSession>,
        _events: Subscription,
    },
    Theme {
        view: Entity<ThemeSwitcher>,
        _events: Subscription,
    },
    /// The theme creator's side panel; it draws its own full-window layer.
    Creator {
        view: Entity<ThemeCreator>,
        _events: Subscription,
    },
}

/// What the terminal pane shows.
enum Pane {
    Empty,
    /// An attached session; `_focus` marks it seen when the pane gains focus.
    Terminal {
        id: String,
        view: Entity<TerminalView>,
        _focus: Subscription,
    },
    /// A session not attached here: ended, or running in another terminal.
    Details(String),
}

/// A session's name being edited in place, in one copy of its row (a session can be
/// listed twice).
struct Rename {
    id: String,
    row_key: String,
    input: Entity<InputState>,
    _enter: Subscription,
}

/// Which row or group header the pointer is over, for its sliding ✕. `generation`
/// restarts the slide each time the pointer arrives or leaves.
#[derive(Default)]
struct HoverSlide {
    over: Option<String>,
    left: Option<String>,
    generation: u64,
}

pub struct Workspace {
    sessions: Vec<SessionInfo>,
    /// Whether the host has answered with its first list; until then the sidebar shows
    /// the agents checking in rather than an empty list.
    listed: bool,
    /// The owner's groups, in the owner's order.
    groups: Vec<GroupInfo>,
    /// The owner's home folder, labelled `~` on rows.
    home: Option<String>,
    selected: Option<String>,
    pane: Pane,
    sidebar_focus: FocusHandle,
    modal: Option<Modal>,
    /// The providers as discovery last reported them, for New session.
    providers: Vec<Provider>,
    /// The provider of the last session started here, chosen first next time.
    last_agent: Option<Agent>,
    /// The theme the owner chose, kept in `app-state.json`; `None` when none is kept.
    theme_choice: Option<ThemeChoice>,
    /// The theme shown while the switcher previews one.
    theme_preview: Option<ThemeChoice>,
    /// The look the theme creator is showing while it is open.
    creator_look: Option<Look>,
    /// The templates and the owner's theme files.
    library: Library,
    /// The settings with the theme's window opacity and blur in force.
    look_appearance: Appearance,
    group_rename: Option<GroupRename>,
    /// The group waiting for "Delete group" to be confirmed.
    confirm_delete: Option<i64>,
    /// The group whose colour picker is open (add-group-colors D3).
    color_picker: Option<i64>,
    /// Where a dragged session would land now; drawn lit.
    drop_target: Option<Target>,
    /// The agents' marks present on this machine (`crate::marks`).
    marks: Marks,
    /// Each session's details as last reported, for the header (in memory only).
    details: HashMap<String, SessionDetails>,
    feed_control: Option<FeedControl>,
    /// The session whose details the host sends; `None` when none is followed yet.
    followed: Option<Option<String>>,
    /// The header's token details are open.
    details_open: bool,
    /// When the folder was last copied from the header, for the brief "Copied".
    copied_at: Option<std::time::Instant>,
    /// The sidebar's width and whether it is the icon rail, kept between runs.
    sidebar: SidebarState,
    /// Saves the window's place a second after it stops moving.
    window_save: Option<Task<()>>,
    /// The session to show again once the first list arrives (add-session-restore D3).
    pending_shown: Option<String>,
    /// The divider is being dragged.
    resizing: bool,
    /// The shown session's shell panel, while open (add-session-shell-panel).
    shell: Option<shell::ShellPanel>,
    /// Sessions whose shell panel is open; their shells run in the host.
    shell_panels: Vec<String>,
    /// The shell panel's share of the pane's height.
    shell_share: f32,
    /// The sidebar's width on screen while it is dragged or settling, when that differs
    /// from the kept state.
    live_width: Option<f32>,
    /// Counts width animations, so a newer one stops an older one.
    width_motion: u64,
    /// The marks setting the marks were last loaded for.
    marks_setting: Option<AgentMarks>,
    /// The session waiting for "End session" to be confirmed.
    /// The session waiting for "Close" to be confirmed.
    confirm_close: Option<String>,
    /// The session whose agent is being started again because it was opened.
    resuming: Option<String>,
    /// Why starting a session's agent again failed: (session id, reason).
    /// The session whose resume failed, why, and whether it was because Claude Code runs
    /// its conversation in the background (fix-claude-conversations D3).
    resume_failed: Option<(String, String, bool)>,
    /// The pane's keyboard while it shows an inactive session: Enter resumes it.
    pane_focus: FocusHandle,
    /// The latest start failure already notified (Unix milliseconds).
    start_failures_told: u64,
    /// The sidebar, drawn as a view GPUI keeps between frames (fix-terminal-redraw-cost).
    sidebar_view: Entity<sidebar_view::SidebarView>,
    /// Steps sidebar motion while any is on screen (fix-sidebar-motion-redraw).
    /// The motion clock and the rate it runs at.
    motion: Option<(model::MotionRate, Task<()>)>,
    motion_epoch: Instant,
    rename: Option<Rename>,
    hover: HoverSlide,
    message: Option<String>,
    theme: Theme,
    options: PaneOptions,
    system_mode: Mode,
    /// The mode last given to the controls (buttons, inputs), once set.
    controls_mode: Option<Mode>,
    appearance: Appearance,
    /// A settings error or warning the owner should see.
    settings_note: Option<String>,
    watcher: SettingsWatcher,
    /// Which system notifications to post (add-notifications D4).
    notifications: hq_config::Notifications,
    /// Whether to check for new releases by itself (`[updates] check`).
    check_updates: bool,
    /// New releases and the update in progress (add-in-app-update).
    updates: update::Updates,
    _list: Task<()>,
    _reload: Task<()>,
    _clock: Task<()>,
    /// Every enabled agent's usage, as the host last reported it.
    usage: Vec<AgentUsage>,
    /// The agent whose usage details are open.
    usage_open: Option<Agent>,
    /// The Remote Control window, while open (add-remote-gateway D12).
    remote_window: Option<remote::RemoteWindow>,
    /// The details list every agent: opened from the "+N" chip.
    usage_all: bool,
    /// A refresh asked of the host has not been answered yet.
    usage_refreshing: bool,
    /// The panel the backdrop just closed, so the click that closed it does not reopen
    /// it on the ring or chip underneath (`None` for the all-agents panel).
    usage_dismissed: Option<(Option<Agent>, Instant)>,
    /// One focus stop per usage ring, for keyboard use.
    ring_focus: HashMap<Agent, FocusHandle>,
    rows_scroll: ScrollHandle,
}

impl Workspace {
    pub fn new(
        watcher: SettingsWatcher,
        appearance: Appearance,
        settings_note: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (tx, mut rx) = unbounded();
        std::thread::spawn(move || watch_sessions(&tx));
        let list = cx.spawn_in(window, async move |this, cx| {
            while let Some(event) = rx.next().await {
                let received = this.update_in(cx, |workspace, window, cx| {
                    workspace.receive(event, window, cx);
                });
                if received.is_err() {
                    break;
                }
            }
        });
        cx.observe_window_appearance(window, |this, window, cx| {
            this.system_mode = system_mode(window.appearance());
            this.apply_settings(window, cx);
        })
        .detach();
        cx.observe_window_activation(window, |this, window, cx| {
            this.mark_seen_if_watched(window, cx);
            this.tell_host_in_use(window, cx);
            cx.notify();
        })
        .detach();
        // The host scans for outside sessions less often while nobody can see them.
        cx.observe_window_visibility(window, |this, visibility, _, _| {
            if let Some(control) = this.feed_control.as_mut() {
                let _ = control.set_visible(visibility.is_visible());
            }
        })
        .detach();
        cx.observe_window_bounds(window, |this, window, cx| {
            this.schedule_window_save(window, cx);
        })
        .detach();
        let reload = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(settings::RELOAD_INTERVAL)
                    .await;
                let reloaded = this.update_in(cx, |workspace, window, cx| {
                    workspace.reload_settings(window, cx);
                });
                if reloaded.is_err() {
                    break;
                }
            }
        });
        let clock = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CLOCK).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        let workspace_entity = cx.entity();
        let sidebar_view = cx.new(|cx| sidebar_view::SidebarView::new(&workspace_entity, cx));
        let mut workspace = Workspace {
            sidebar_view,
            sessions: Vec::new(),
            listed: false,
            groups: Vec::new(),
            home: hq_signals::paths::home().map(|home| home.to_string_lossy().into_owned()),
            selected: None,
            pane: Pane::Empty,
            sidebar_focus: cx.focus_handle(),
            modal: None,
            providers: Vec::new(),
            last_agent: None,
            theme_choice: crate::app_state::path()
                .and_then(|path| crate::app_state::load_theme(&path))
                .and_then(|key| ThemeChoice::from_key(&key)),
            theme_preview: None,
            creator_look: None,
            library: Library::load(hq_config::themes::themes_dir().as_deref()),
            look_appearance: appearance.clone(),
            group_rename: None,
            confirm_delete: None,
            color_picker: None,
            drop_target: None,
            marks: Marks::default(),
            marks_setting: None,
            sidebar: rail::kept_sidebar(),
            window_save: None,
            pending_shown: crate::app_state::path()
                .and_then(|path| crate::app_state::load_shown(&path)),
            details: HashMap::new(),
            feed_control: None,
            followed: None,
            details_open: false,
            copied_at: None,
            resizing: false,
            shell: None,
            shell_panels: crate::app_state::path()
                .map(|path| crate::app_state::load_shell_panels(&path))
                .unwrap_or_default(),
            shell_share: crate::app_state::path().map_or(crate::app_state::SHELL_SHARE, |path| {
                crate::app_state::load_shell_share(&path)
            }),
            live_width: None,
            width_motion: 0,
            confirm_close: None,
            resuming: None,
            resume_failed: None,
            pane_focus: cx.focus_handle(),
            start_failures_told: crate::app_state::path()
                .map_or(0, |path| crate::app_state::load_start_failures_told(&path)),
            motion: None,
            motion_epoch: Instant::now(),
            rename: None,
            hover: HoverSlide::default(),
            message: None,
            theme: Theme::light(),
            options: PaneOptions::default(),
            system_mode: system_mode(window.appearance()),
            controls_mode: None,
            appearance,
            settings_note,
            notifications: hq_config::Notifications::default(),
            check_updates: true,
            updates: update::Updates::new(),
            watcher,
            _list: list,
            _reload: reload,
            _clock: clock,
            usage: Vec::new(),
            usage_open: None,
            remote_window: None,
            usage_all: false,
            usage_refreshing: false,
            usage_dismissed: None,
            rows_scroll: ScrollHandle::new(),
            ring_focus: Agent::ALL
                .into_iter()
                .map(|agent| (agent, cx.focus_handle()))
                .collect(),
        };
        workspace.notifications = settings::notifications(&mut workspace.watcher, None);
        workspace.check_updates = settings::check_updates(&mut workspace.watcher, None);
        workspace.listen_for_notification_clicks(window, cx);
        // Outside an app bundle macOS shows none of the app's notifications (D5).
        if cfg!(target_os = "macos") && cx.app_path().is_err() {
            workspace.message =
                Some("Notifications show only in the installed Headquarter.app.".to_owned());
        }
        workspace.apply_settings(window, cx);
        window.focus(&workspace.sidebar_focus, cx);
        workspace
    }

    /// Clicking a session's notification brings the app forward showing that session
    /// (add-notifications D3).
    fn listen_for_notification_clicks(&self, window: &Window, cx: &mut Context<Self>) {
        let workspace = cx.entity().downgrade();
        let handle = window.window_handle();
        cx.on_system_notification_response(move |response, cx| {
            cx.activate(true);
            let _ = handle.update(cx, |_, window, cx| {
                window.activate_window();
                let _ = workspace.update(cx, |workspace, cx| {
                    workspace.open_notified(&response.tag, window, cx);
                });
            });
        });
    }

    fn open_notified(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = notifications::clicked(&self.sessions, id).map(|session| session.id.clone())
        else {
            return;
        };
        self.open(&id, window, cx);
        if let Pane::Terminal { view, .. } = &self.pane {
            window.focus(&view.focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Posts the system notification a session's change calls for, if any (D1, D2).
    fn notify_change(
        &self,
        previous: Option<&SessionInfo>,
        current: &SessionInfo,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let watched = self.watching(&current.id, window, cx);
        let Some(notice) = notifications::notice(previous, current, watched, self.notifications)
        else {
            return;
        };
        cx.show_system_notification(SystemNotification {
            tag: notice.tag.into(),
            title: notice.title.into(),
            body: notice.body.into(),
            actions: Vec::new(),
        });
    }

    /// Notifies each session the host could not start with itself, once.
    fn tell_start_failures(&mut self, cx: &mut Context<Self>) {
        let failures = notifications::start_failures(&self.sessions, self.start_failures_told);
        if failures.is_empty() {
            return;
        }
        for notice in failures {
            cx.show_system_notification(SystemNotification {
                tag: notice.tag.into(),
                title: notice.title.into(),
                body: notice.body.into(),
                actions: Vec::new(),
            });
        }
        self.start_failures_told = self
            .sessions
            .iter()
            .filter_map(|session| session.start_error.as_ref().map(|error| error.at))
            .fold(self.start_failures_told, u64::max);
        if let Some(path) = crate::app_state::path() {
            // A lost write only means a failure may be told again after a restart.
            let _ = crate::app_state::save_start_failures_told(&path, self.start_failures_told);
        }
    }

    // ---- settings ----------------------------------------------------------------

    /// Picks up edits to the settings file, unless the owner turned live reload off.
    fn reload_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.appearance.live_reload {
            return;
        }
        self.notifications = settings::notifications(&mut self.watcher, Some(self.notifications));
        self.check_updates = settings::check_updates(&mut self.watcher, Some(self.check_updates));
        let (appearance, note) = settings::read(&mut self.watcher, Some(&self.appearance));
        if appearance == self.appearance && note == self.settings_note {
            return;
        }
        self.appearance = appearance;
        self.settings_note = note;
        self.apply_settings(window, cx);
    }

    /// Applies the settings and the system's light or dark mode to the window, the
    /// controls, and the open terminal pane.
    fn apply_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The creator's preview first, then the switcher's, then the kept choice.
        let look = self.creator_look.clone().unwrap_or_else(|| {
            themes::look(
                self.theme_preview.as_ref().or(self.theme_choice.as_ref()),
                self.system_mode,
                &self.appearance.colors,
                &self.library,
            )
        });
        // A theme's window settings replace the settings file's while it is in use.
        let mut look_appearance = self.appearance.clone();
        if let Some(opacity) = look.window_opacity {
            look_appearance.opacity = opacity;
        }
        if let Some(blur) = look.window_blur {
            look_appearance.blur = blur;
        }
        let palette_changed = look.theme.palette != self.theme.palette;
        self.theme = look.theme;
        self.options = settings::pane_options(&look_appearance);
        settings::apply_to_window(&look_appearance, window);
        self.look_appearance = look_appearance;
        self.apply_marks_setting(window, cx);
        if self.controls_mode != Some(self.theme.mode) {
            self.controls_mode = Some(self.theme.mode);
            settings::set_controls_mode(self.theme.mode, window, cx);
        }
        let views = match &self.pane {
            Pane::Terminal { view, .. } => std::iter::once(view.clone())
                .chain(self.shell_view().cloned())
                .collect(),
            _ => Vec::new(),
        };
        for view in &views {
            let options = self.options.clone();
            view.update(cx, |terminal, cx| terminal.set_options(options, cx));
            if palette_changed {
                let palette = self.theme.palette.clone();
                let session = view.read(cx).session().clone();
                session.update(cx, |session, cx| {
                    session.model_mut().set_palette(palette);
                    cx.notify();
                });
            }
        }
        let chrome = self.theme.chrome;
        match &self.modal {
            Some(Modal::Open { view: window, .. }) => {
                window.update(cx, |window, cx| window.set_chrome(chrome, cx));
            }
            Some(Modal::New { view: window, .. }) => {
                window.update(cx, |window, cx| window.set_chrome(chrome, cx));
            }
            Some(Modal::Theme { view: window, .. }) => {
                window.update(cx, |window, cx| window.set_chrome(chrome, cx));
            }
            Some(Modal::Creator { .. }) | None => {}
        }
        cx.notify();
    }

    /// Loads the marks already fetched and fetches the missing ones off the main
    /// thread, or drops them when the owner chose the drawn marks. One-colour marks
    /// follow the light or dark mode.
    fn apply_marks_setting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ink = match self.theme.mode {
            Mode::Light => Rgb { r: 0, g: 0, b: 0 },
            Mode::Dark => Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
        };
        let setting = self.appearance.agent_marks;
        if self.marks_setting == Some(setting) {
            let mut marks = self.marks.clone();
            marks.set_ink(ink);
            if marks != self.marks {
                self.set_marks(marks, cx);
            }
            return;
        }
        self.marks_setting = Some(setting);
        let style = match setting {
            AgentMarks::Mono => marks::Style::Mono,
            AgentMarks::Color => marks::Style::Color,
            AgentMarks::Drawn => {
                self.set_marks(Marks::default(), cx);
                return;
            }
        };
        let Some(dir) = marks::dir() else {
            self.set_marks(Marks::default(), cx);
            return;
        };
        let mut present = Marks::load(&dir, style);
        present.set_ink(ink);
        self.set_marks(present, cx);
        let fetch = cx
            .background_executor()
            .spawn(async move { marks::fetch_missing(&dir, style) });
        cx.spawn_in(window, async move |this, cx| {
            let mut fetched = fetch.await;
            let _ = this.update(cx, |this, cx| {
                if this.marks_setting == Some(setting) {
                    fetched.set_ink(ink);
                    this.set_marks(fetched, cx);
                }
            });
        })
        .detach();
    }

    fn set_marks(&mut self, marks: Marks, cx: &mut Context<Self>) {
        let shared = marks.clone();
        match &self.modal {
            Some(Modal::Open { view: window, .. }) => {
                window.update(cx, |window, cx| window.set_marks(shared, cx));
            }
            Some(Modal::New { view: window, .. }) => {
                window.update(cx, |window, cx| window.set_marks(shared, cx));
            }
            Some(Modal::Theme { .. } | Modal::Creator { .. }) | None => {}
        }
        self.marks = marks;
        cx.notify();
    }

    // ---- data from the host -------------------------------------------------------

    fn receive(&mut self, event: ListEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            ListEvent::All {
                sessions,
                groups,
                usage,
            } => {
                self.sessions = sessions;
                self.groups = groups;
                self.usage = usage;
                let first = !self.listed;
                self.listed = true;
                self.show_kept_session(window, cx);
                if first {
                    self.schedule_update_checks(window, cx);
                    self.check_host_version(window, cx);
                }
            }
            ListEvent::Usage(usage) => self.usage = usage,
            ListEvent::Changed(session) => self.upsert(session, window, cx),
            ListEvent::Groups(groups) => self.groups = groups,
            ListEvent::Control(mut control) => {
                // A new connection counts as visible until told otherwise.
                if !window.is_visible() {
                    let _ = control.set_visible(false);
                }
                self.feed_control = Some(control);
                // Follow the shown session again on the new connection.
                self.followed = None;
            }
            ListEvent::Details { session, details } => {
                self.details.insert(session, details);
            }
            ListEvent::Failed(message) => {
                self.listed = true;
                self.message = Some(message);
            }
        }
        self.tell_start_failures(cx);
        self.restart_if_idle(window, cx);
        if let Some(Modal::Open { view: window, .. }) = &self.modal {
            let sessions = self.sessions.clone();
            let loaded = self.listed;
            window.update(cx, |window, cx| {
                window.set_loaded(loaded, cx);
                window.update_sessions(sessions, cx);
            });
        }
        cx.notify();
    }

    fn upsert(&mut self, session: SessionInfo, window: &mut Window, cx: &mut Context<Self>) {
        let id = session.id.clone();
        let live = session.live;
        // The shown session left the sidebar (closed): show the row below it, else the
        // one above, else nothing. Worked out before the row disappears.
        let neighbour = (!session.in_sidebar
            && self.shown_id() == Some(id.as_str())
            && self.session(&id).is_some_and(|known| known.in_sidebar))
        .then(|| {
            let ids = model::row_ids(&self.items());
            model::next_selection(&ids, Some(&id), 1)
                .filter(|next| *next != id)
                .or_else(|| model::next_selection(&ids, Some(&id), -1))
                .filter(|next| *next != id)
        });
        let previous = self.session(&id).cloned();
        self.notify_change(previous.as_ref(), &session, window, cx);
        let started = live && previous.as_ref().is_some_and(|previous| !previous.live);
        match self.sessions.iter_mut().find(|known| known.id == id) {
            Some(known) => *known = session,
            None => self.sessions.push(session),
        }
        // The shown agent ended: offer Resume in place of the terminal.
        if !live
            && let Pane::Terminal { id: shown, .. } = &self.pane
            && *shown == id
        {
            self.pane = Pane::Details(id.clone());
            self.sync_shell(window, cx);
        }
        if let Some(next) = neighbour {
            if self.rename.as_ref().is_some_and(|rename| rename.id == id) {
                self.rename = None;
            }
            if self.confirm_close.as_deref() == Some(id.as_str()) {
                self.confirm_close = None;
            }
            match next {
                Some(next) => self.open(&next, window, cx),
                None => {
                    self.pane = Pane::Empty;
                    self.selected = None;
                    self.sync_shell(window, cx);
                }
            }
            return;
        }
        // The shown session's agent started meanwhile (with the host, or from `hq`):
        // show it, unless this window is starting it and will open it itself.
        if started
            && self.resuming.as_deref() != Some(id.as_str())
            && matches!(&self.pane, Pane::Details(shown) if *shown == id)
        {
            self.open(&id, window, cx);
            return;
        }
        if self.shown_id() == Some(id.as_str()) {
            self.mark_seen_if_watched(window, cx);
        }
    }

    fn session(&self, id: &str) -> Option<&SessionInfo> {
        self.sessions.iter().find(|session| session.id == id)
    }

    fn shown_id(&self) -> Option<&str> {
        match &self.pane {
            Pane::Terminal { id, .. } | Pane::Details(id) => Some(id),
            Pane::Empty => None,
        }
    }

    /// Runs `call` against the host off the main thread, then `done` with its result;
    /// a failure is shown under the sidebar.
    fn host_call<R: Send + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        call: impl FnOnce(&HostPaths) -> anyhow::Result<R> + Send + 'static,
        done: impl FnOnce(&mut Self, R, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let task = cx.background_executor().spawn(async move {
            let paths =
                HostPaths::standard().ok_or_else(|| anyhow!("cannot find the home folder"))?;
            call(&paths)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(value) => {
                        this.message = None;
                        done(this, value, window, cx);
                    }
                    Err(error) => this.message = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Whether the owner is watching session `id`: shown in the active window, with
    /// focus in its agent or its shell panel, which belong to the same pane.
    fn watching(&self, id: &str, window: &Window, cx: &App) -> bool {
        let Pane::Terminal {
            id: shown, view, ..
        } = &self.pane
        else {
            return false;
        };
        let focused = |view: &Entity<TerminalView>| view.focus_handle(cx).is_focused(window);
        let in_pane = focused(view) || self.shell_view().is_some_and(focused);
        shown == id && window.is_window_active() && in_pane
    }

    /// The terminal the owner works in, agent or shell, takes this window's size on the
    /// host, even when a phone used it meanwhile (add-remote-gateway D6).
    fn tell_host_in_use(&self, window: &Window, cx: &App) {
        if !window.is_window_active() {
            return;
        }
        let Pane::Terminal { view, .. } = &self.pane else {
            return;
        };
        let focused = [Some(view), self.shell_view()]
            .into_iter()
            .flatten()
            .find(|view| view.focus_handle(cx).is_focused(window));
        if let Some(view) = focused {
            view.read(cx).session().read(cx).focus();
        }
    }

    /// "Done, not read yet" clears once the owner can see the session.
    fn mark_seen_if_watched(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Pane::Terminal { id, .. } = &self.pane else {
            return;
        };
        let watched = self.watching(id, window, cx);
        let unread = self
            .session(id)
            .is_some_and(|session| session.live && session.status == Status::DoneUnread);
        if watched && unread {
            let id = id.clone();
            self.host_call(
                window,
                cx,
                move |paths| hq_client::mark_seen(paths, &id),
                |_, (), _, _| {},
            );
        }
    }

    // ---- opening sessions ---------------------------------------------------------

    fn open(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(info) = self.session(id).cloned() else {
            return;
        };
        self.leave_whats_new();
        self.selected = Some(info.id.clone());
        if let Some(path) = crate::app_state::path() {
            // A lost write only means the next launch shows nothing in particular.
            let _ = crate::app_state::save_shown(&path, &info.id);
        }
        if !info.live {
            // An inactive session waits for "Resume Session"; one still waiting its turn
            // to start with the host starts now, ahead of the others; one running in
            // another terminal is only observed (add-inactive-sessions D4).
            self.pane = Pane::Details(info.id.clone());
            self.sync_shell(window, cx);
            if info.external.is_none() {
                if info.starting {
                    if self.resuming.is_none() {
                        self.resume(info.id, window, cx);
                    }
                } else {
                    window.focus(&self.pane_focus, cx);
                }
            }
            cx.notify();
            return;
        }
        if let Pane::Terminal {
            id: shown, view, ..
        } = &self.pane
            && *shown == info.id
        {
            window.focus(&view.focus_handle(cx), cx);
            return;
        }
        let palette = self.theme.palette.clone();
        let id = info.id.clone();
        let session = cx.new(|cx| TerminalSession::attach(info, palette, cx));
        let options = self.options.clone();
        let view = cx.new(|cx| TerminalView::new(session, options, window, cx));
        let focus = cx.on_focus(&view.focus_handle(cx), window, |this, window, cx| {
            this.mark_seen_if_watched(window, cx);
            this.tell_host_in_use(window, cx);
        });
        window.focus(&view.focus_handle(cx), cx);
        self.pane = Pane::Terminal {
            id,
            view,
            _focus: focus,
        };
        self.sync_shell(window, cx);
        self.mark_seen_if_watched(window, cx);
        cx.notify();
    }

    /// Shows the session the window showed when the app last closed: attached when its
    /// agent runs, otherwise its details, without restarting it (add-session-restore D3).
    fn show_kept_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.pending_shown.take() else {
            return;
        };
        let Some(info) = model::kept_session(&self.sessions, &id).cloned() else {
            return;
        };
        if info.live || info.starting {
            // Starting with the host: the session the window shows starts first.
            self.open(&id, window, cx);
        } else {
            self.selected = Some(id.clone());
            self.pane = Pane::Details(id);
            self.sync_shell(window, cx);
        }
    }

    fn schedule_window_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.window_save = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let _ = this.update_in(cx, |_, window, _| Self::save_window_now(window));
        }));
    }

    fn save_window_now(window: &Window) {
        let bounds = window.bounds();
        let place = crate::app_state::WindowPlace {
            x: f32::from(bounds.origin.x),
            y: f32::from(bounds.origin.y),
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
        };
        if let Some(path) = crate::app_state::path() {
            // A lost write only means the next launch opens centred.
            let _ = crate::app_state::save_window(&path, place);
        }
    }

    fn resume(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.resuming = Some(id.clone());
        self.resume_failed = None;
        let wanted = id.clone();
        self.host_call(
            window,
            cx,
            // The failure is kept for the pane, so it is returned rather than raised.
            move |paths| Ok(hq_client::resume(paths, &id).map_err(refusal)),
            move |this, result, window, cx| {
                this.resuming = None;
                match result {
                    Ok(session) => {
                        let id = session.id.clone();
                        this.upsert(session, window, cx);
                        // Show it only if the owner has not moved on meanwhile.
                        if this.shown_id() == Some(wanted.as_str()) {
                            this.open(&id, window, cx);
                        }
                    }
                    Err((reason, background)) => {
                        this.resume_failed = Some((wanted, reason, background));
                    }
                }
            },
        );
    }

    /// Stops the session's conversation in Claude Code's background and resumes it here.
    fn bring_in(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.resuming = Some(id.clone());
        self.resume_failed = None;
        let wanted = id.clone();
        self.host_call(
            window,
            cx,
            move |paths| Ok(hq_client::bring_in(paths, &id).map_err(refusal)),
            move |this, result, window, cx| {
                this.resuming = None;
                match result {
                    Ok(session) => {
                        let id = session.id.clone();
                        this.upsert(session, window, cx);
                        if this.shown_id() == Some(wanted.as_str()) {
                            this.open(&id, window, cx);
                        }
                    }
                    Err((reason, background)) => {
                        this.resume_failed = Some((wanted, reason, background));
                    }
                }
            },
        );
    }

    fn jump_to_next_needing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = model::next_needing(&self.sessions, self.shown_id()) {
            self.open(&id, window, cx);
        }
    }

    /// The sidebar's elements: header, rows or rail, usage and notes. Built for
    /// [`sidebar_view::SidebarView`], which GPUI keeps between frames, so a busy
    /// terminal does not lay the sidebar out again on every frame it causes.
    fn render_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let opacity = self.look_appearance.opacity;
        let rail = self.rail_shown();
        // Room for the window buttons, or a slim strip to drag the window by.
        let top = if settings::buttonless(&self.appearance) {
            px(20.)
        } else {
            px(48.)
        };
        let items = self.items();
        self.keep_motion(model::motion_rate(&items), window, cx);
        let empty = items.is_empty();
        let rendered = if rail {
            self.render_rail(&items, cx)
        } else {
            self.render_items(&items, window, cx)
        };
        let header = (!rail).then(|| self.render_header(cx));
        // While the host starts, the pane shows it; the sidebar stays quiet.
        let empty_state = (empty && !rail && self.listed)
            .then(|| self.render_empty_sidebar(cx).into_any_element());
        let update_pill = self.render_update_pill(rail, cx);
        let usage_strip = if rail {
            self.render_rail_usage(cx)
        } else {
            self.render_usage_strip(cx)
                .map(IntoElement::into_any_element)
        };
        let notes: Vec<String> = [self.message.clone(), self.settings_note.clone()]
            .into_iter()
            .flatten()
            .collect();
        div()
            .id("sidebar")
            .track_focus(&self.sidebar_focus)
            .on_key_down(cx.listener(Self::sidebar_key_down))
            .w(px(self.sidebar_width()))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .px(px(if rail { 6. } else { 10. }))
            .bg(translucent(chrome.sidebar, opacity))
            .border_r_1()
            .border_color(hsla(chrome.sidebar_line))
            .child(
                // Room above the controls; also where the window is dragged from.
                div().id("window-drag").h(top).flex_none().on_mouse_down(
                    MouseButton::Left,
                    |_, window, _| {
                        window.start_window_move();
                    },
                ),
            )
            .children(header)
            .child(
                // A scrollbar shows when the rows do not all fit.
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.))
                    .child(
                        div()
                            .id("sidebar-rows")
                            .size_full()
                            .when(empty, |rows| rows.justify_center())
                            .overflow_y_scroll()
                            .track_scroll(&self.rows_scroll)
                            .flex()
                            .flex_col()
                            .children(empty_state)
                            .children(rendered),
                    )
                    .child(Scrollbar::vertical(&self.rows_scroll)),
            )
            .children(update_pill)
            .children(usage_strip)
            .children(notes.into_iter().map(|message| {
                // The rail has no room for the words; they are in the tooltip.
                if rail {
                    div()
                        .id("rail-note")
                        .py(px(6.))
                        .flex()
                        .justify_center()
                        .text_color(hsla(chrome.amber))
                        .child("!")
                        .tooltip(move |window, cx| Tooltip::new(message.clone()).build(window, cx))
                        .into_any_element()
                } else {
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(13.))
                        .text_color(hsla(chrome.amber))
                        .child(message)
                        .into_any_element()
                }
            }))
            .into_any_element()
    }

    // ---- sidebar motion -----------------------------------------------------------

    /// Runs the motion clock at the rate what moves needs, and stops it when nothing moves
    /// or the system asks for reduced motion. Each step redraws once, so motion costs 4
    /// frames a second while only working rows move, and 12 while a waiting dot pulses,
    /// however many rows move (refresh-pane-and-status D2).
    fn keep_motion(
        &mut self,
        rate: Option<model::MotionRate>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rate) = rate.filter(|_| !settings::reduce_motion()) else {
            self.motion = None;
            return;
        };
        if self
            .motion
            .as_ref()
            .is_some_and(|(running, _)| *running == rate)
        {
            return;
        }
        let clock = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(rate.step()).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        self.motion = Some((rate, clock));
    }

    /// Which of a working row's dots is lit now, or `None` while motion is held still.
    fn lit_dot(&self) -> Option<Option<usize>> {
        self.motion
            .is_some()
            .then(|| model::lit_dot(self.motion_epoch.elapsed()))
    }

    /// Where a motion of `period` is now, or `None` while motion is held still.
    fn motion_phase(&self, period: Duration) -> Option<f32> {
        self.motion
            .is_some()
            .then(|| model::motion_phase(self.motion_epoch.elapsed(), period))
    }

    // ---- sidebar changes ----------------------------------------------------------

    fn items(&self) -> Vec<Item> {
        model::items(
            &self.sessions,
            &self.groups,
            self.home.as_deref(),
            hq_signals::now_ms(),
        )
    }

    fn move_selection(&mut self, step: isize, cx: &mut Context<Self>) {
        let ids = model::row_ids(&self.items());
        self.selected = model::next_selection(&ids, self.selected.as_deref(), step);
        cx.notify();
    }

    /// The group holding the selected session, if any.
    fn group_of_selected(&self) -> Option<i64> {
        let id = self.selected.as_deref()?;
        self.session(id).and_then(|session| session.group)
    }

    /// Adds the sessions to the sidebar, in no group, and shows the first.
    fn add(&mut self, ids: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let first = ids.first().cloned();
        self.host_call(
            window,
            cx,
            move |paths| hq_client::add_to_sidebar(paths, &ids),
            move |this, (), window, cx| {
                this.close_modal(window, cx);
                if let Some(first) = &first {
                    this.open(first, window, cx);
                }
            },
        );
    }

    /// Asks before closing any session (owner, 2026-10-21).
    pub(super) fn request_close(&mut self, id: String, cx: &mut Context<Self>) {
        self.confirm_close = Some(id);
        cx.notify();
    }

    pub(super) fn close_session(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirm_close = None;
        self.host_call(
            window,
            cx,
            move |paths| hq_client::close(paths, &id),
            |_, (), _, _| {},
        );
    }

    /// Edits the open session's name in place in its sidebar row; Enter or a click
    /// elsewhere saves, Escape cancels, and a blank name gives the agent's title back.
    pub(super) fn start_rename(
        &mut self,
        id: String,
        row_key: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = self
            .session(&id)
            .map(hq_protocol::sidebar::display_title)
            .unwrap_or_default();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).default_value(name);
            state.select_all(window, cx);
            state
        });
        let enter = cx.subscribe_in(
            &input,
            window,
            |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    let name = input.read(cx).value().to_string();
                    this.finish_rename(Some(name), window, cx);
                }
                _ => {}
            },
        );
        window.focus(&input.focus_handle(cx), cx);
        self.rename = Some(Rename {
            id,
            row_key,
            input,
            _enter: enter,
        });
        cx.notify();
    }

    /// Ends renaming: `None` cancels. An unchanged title is not sent, so a session keeps
    /// following the agent's title until the owner really renames it.
    pub(super) fn finish_rename(
        &mut self,
        name: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rename) = self.rename.take() else {
            return;
        };
        cx.notify();
        let Some(name) = name else {
            return;
        };
        let id = rename.id;
        let unchanged = self
            .session(&id)
            .is_some_and(|session| hq_protocol::sidebar::display_title(session) == name.trim());
        if unchanged {
            return;
        }
        self.host_call(
            window,
            cx,
            move |paths| hq_client::rename(paths, &id, &name),
            |_, (), _, _| {},
        );
    }

    /// Opens Open sessions; when it is open already, it keeps its picks and search.
    pub fn show_open_sessions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Modal::Open { view: open, .. }) = &self.modal {
            window.focus(&open.focus_handle(cx), cx);
            return;
        }
        let sessions = self.sessions.clone();
        let agents = self.enabled_agents();
        let home = self.home_folder();
        let chrome = self.theme.chrome;
        let marks = self.marks.clone();
        let open =
            cx.new(|cx| OpenSessions::new(sessions, agents, home, chrome, marks, window, cx));
        let loaded = self.listed;
        open.update(cx, |open, cx| open.set_loaded(loaded, cx));
        let events =
            cx.subscribe_in(
                &open,
                window,
                |this, _, event: &OpenEvent, window, cx| match event {
                    OpenEvent::Open(ids) => this.add(ids.clone(), window, cx),
                    OpenEvent::Close => this.close_modal(window, cx),
                },
            );
        self.modal = Some(Modal::Open {
            view: open,
            _events: events,
        });
        cx.notify();
    }

    /// Opens New session, and looks for the providers again in the background.
    fn show_new_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Modal::New { view: new, .. }) = &self.modal {
            window.focus(&new.focus_handle(cx), cx);
            return;
        }
        if self.providers.is_empty() {
            self.providers = self
                .enabled_agents()
                .into_iter()
                .map(Provider::unknown)
                .collect();
        }
        let providers = self.providers.clone();
        let preferred = self.last_agent;
        let home = self.home_folder();
        let chrome = self.theme.chrome;
        let marks = self.marks.clone();
        let new =
            cx.new(|cx| NewSession::new(providers, preferred, home, chrome, marks, window, cx));
        let events =
            cx.subscribe_in(
                &new,
                window,
                |this, _, event: &NewEvent, window, cx| match event {
                    NewEvent::Start { agent, folder } => {
                        this.start_session(*agent, folder.clone(), window, cx);
                    }
                    NewEvent::Close => this.close_modal(window, cx),
                },
            );
        self.modal = Some(Modal::New {
            view: new,
            _events: events,
        });
        self.discover_providers(cx);
        cx.notify();
    }

    /// Opens the theme switcher, or closes it as Escape does when it is open.
    fn toggle_theme_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.modal, Some(Modal::Theme { .. })) {
            self.end_theme_preview(None, window, cx);
            return;
        }
        let settings = self.watcher.current().ok();
        let source = settings
            .as_ref()
            .and_then(|settings| settings.imports.last())
            .and_then(|import| import.file_name())
            .map_or_else(
                || "settings file".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
        self.library = Library::load(hq_config::themes::themes_dir().as_deref());
        let colors = &self.appearance.colors;
        let rows = theme_switcher::rows(self.system_mode, colors, &source, &self.library);
        let current = themes::effective(self.theme_choice.as_ref(), colors, &self.library);
        let chrome = self.theme.chrome;
        let switcher = cx.new(|cx| ThemeSwitcher::new(rows, current, chrome, window, cx));
        let events = cx.subscribe_in(
            &switcher,
            window,
            |this, _, event: &ThemeEvent, window, cx| match event {
                ThemeEvent::Preview(choice) => {
                    this.theme_preview = Some(choice.clone());
                    this.apply_settings(window, cx);
                }
                ThemeEvent::Keep(choice) => {
                    this.end_theme_preview(Some(choice.clone()), window, cx);
                }
                ThemeEvent::Cancel => this.end_theme_preview(None, window, cx),
                ThemeEvent::Create { from, name } => {
                    this.open_creator(from.clone(), name.clone(), None, window, cx);
                }
                ThemeEvent::Edit(file) => {
                    let name = this
                        .library
                        .file(file)
                        .map_or_else(|| file.clone(), |theme| theme.name.clone());
                    let from = ThemeChoice::File(file.clone());
                    this.open_creator(from, name, Some(file.clone()), window, cx);
                }
                ThemeEvent::Delete(file) => this.delete_theme(file.clone(), window, cx),
            },
        );
        self.modal = Some(Modal::Theme {
            view: switcher,
            _events: events,
        });
        cx.notify();
    }

    /// Closes the switcher, keeping `kept` (and remembering it) or going back to the
    /// theme in use before it opened.
    fn end_theme_preview(
        &mut self,
        kept: Option<ThemeChoice>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme_preview = None;
        if let Some(choice) = kept {
            self.keep_theme(choice);
        }
        self.apply_settings(window, cx);
        self.close_modal(window, cx);
    }

    /// Makes `choice` the theme in use and remembers it for the next run.
    fn keep_theme(&mut self, choice: ThemeChoice) {
        let saved =
            crate::app_state::path().map(|path| crate::app_state::save_theme(&path, &choice.key()));
        if let Some(Err(error)) = saved {
            self.message = Some(format!("Couldn't keep the theme for next time: {error}"));
        }
        self.theme_choice = Some(choice);
    }

    /// Removes one of the owner's theme files; the theme in use falls back to System.
    fn delete_theme(&mut self, file: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dir) = hq_config::themes::themes_dir() else {
            return;
        };
        if let Err(error) = hq_config::themes::delete(&dir, &file) {
            self.message = Some(format!("Couldn't delete the theme: {error}"));
        }
        if self.theme_choice == Some(ThemeChoice::File(file.clone())) {
            self.keep_theme(ThemeChoice::System);
        }
        self.theme_preview = None;
        self.library = Library::load(Some(&dir));
        self.apply_settings(window, cx);
        // Open the list again without the deleted theme.
        self.modal = None;
        self.toggle_theme_switcher(window, cx);
    }

    /// Turns the switcher into the creator, starting from `from`; `editing` names the
    /// file an edit saves into.
    fn open_creator(
        &mut self,
        from: ThemeChoice,
        name: String,
        editing: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let colors = self.appearance.colors.clone();
        let mut look = themes::look(Some(&from), self.system_mode, &colors, &self.library);
        // The creator always shows the window settings, starting from those in force.
        look.window_opacity = look.window_opacity.or(Some(self.appearance.opacity));
        look.window_blur = look.window_blur.or(Some(self.appearance.blur));
        let starts = theme_switcher::rows(self.system_mode, &colors, "", &self.library)
            .into_iter()
            .filter(|row| row.available)
            .map(|row| (row.choice, row.name))
            .collect();
        let start = CreatorStart {
            look: look.clone(),
            from,
            name,
            editing,
            starts,
            library: self.library.clone(),
            system: self.system_mode,
            colors,
        };
        let creator = cx.new(|cx| ThemeCreator::new(start, window, cx));
        let events = cx.subscribe_in(
            &creator,
            window,
            |this, creator, event: &CreatorEvent, window, cx| match event {
                CreatorEvent::Preview(look) => {
                    this.creator_look = Some((**look).clone());
                    this.apply_settings(window, cx);
                }
                CreatorEvent::Save { file, theme } => {
                    let saved = hq_config::themes::themes_dir()
                        .ok_or_else(|| "cannot find the settings folder".to_owned())
                        .and_then(|dir| {
                            hq_config::themes::save(&dir, theme, file.as_deref())
                                .map(|file| (dir, file))
                                .map_err(|error| error.to_string())
                        });
                    match saved {
                        Ok((dir, file)) => {
                            this.library = Library::load(Some(&dir));
                            this.keep_theme(ThemeChoice::File(file));
                            this.creator_look = None;
                            this.apply_settings(window, cx);
                            this.close_modal(window, cx);
                        }
                        Err(error) => creator.update(cx, |creator, cx| {
                            creator.failed(format!("Couldn't save the theme: {error}"), cx);
                        }),
                    }
                }
                CreatorEvent::Cancel => {
                    this.creator_look = None;
                    this.apply_settings(window, cx);
                    this.close_modal(window, cx);
                }
            },
        );
        self.theme_preview = None;
        self.creator_look = Some(look);
        self.modal = Some(Modal::Creator {
            view: creator,
            _events: events,
        });
        self.apply_settings(window, cx);
        cx.notify();
    }

    fn close_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = None;
        window.focus(&self.sidebar_focus, cx);
        cx.notify();
    }

    /// The providers the settings file leaves enabled.
    fn enabled_agents(&mut self) -> Vec<Agent> {
        let settings = self.watcher.current().ok();
        Agent::ALL
            .into_iter()
            .filter(|agent| {
                settings
                    .as_ref()
                    .is_none_or(|settings| settings.agent_enabled(agent.id()))
            })
            .collect()
    }

    fn home_folder(&self) -> PathBuf {
        self.home
            .as_ref()
            .map_or_else(|| PathBuf::from("/"), PathBuf::from)
    }

    /// Finds each enabled provider's executable and version as `hq agents` does, off
    /// the main thread, and passes them to New session if it is still open.
    fn discover_providers(&mut self, cx: &mut Context<Self>) {
        let settings = self.watcher.current().ok();
        let agents = self.enabled_agents();
        let task = cx.background_executor().spawn(async move {
            let reports = crate::setup::discover(settings.as_ref());
            agents
                .into_iter()
                .map(|agent| {
                    let report = reports.iter().find(|report| report.agent.id == agent.id());
                    Provider {
                        installed: Some(report.is_some_and(|report| report.used.is_some())),
                        version: report.and_then(|report| report.version.clone()),
                        ..Provider::unknown(agent)
                    }
                })
                .collect::<Vec<_>>()
        });
        cx.spawn(async move |this, cx| {
            let providers = task.await;
            let _ = this.update(cx, |this, cx| {
                this.providers = providers.clone();
                if let Some(Modal::New { view: new, .. }) = &this.modal {
                    let preferred = this.last_agent;
                    new.update(cx, |new, cx| new.set_providers(providers, preferred, cx));
                }
            });
        })
        .detach();
    }

    /// Starts a session with the provider's own command; a host error stays in the
    /// window's footer.
    fn start_session(
        &mut self,
        agent: Agent,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let task = cx.background_executor().spawn(async move {
            let paths =
                HostPaths::standard().ok_or_else(|| anyhow!("cannot find the home folder"))?;
            hq_client::create_session(&paths, agent, &folder, None)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| match result {
                Ok(session) => {
                    this.last_agent = Some(agent);
                    this.close_modal(window, cx);
                    let id = session.id.clone();
                    this.upsert(session, window, cx);
                    this.open(&id, window, cx);
                }
                Err(error) => {
                    if let Some(Modal::New { view: new, .. }) = &this.modal {
                        new.update(cx, |new, cx| new.failed(format!("{error:#}"), cx));
                    }
                }
            });
        })
        .detach();
    }

    // ---- keys ---------------------------------------------------------------------

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && self.usage_open.is_some() {
            self.usage_open = None;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let keys = Keys {
            control: modifiers.control,
            alt: modifiers.alt,
            shift: modifiers.shift,
            platform: modifiers.platform,
        };
        let Some(shortcut) = shortcuts::shortcut(&event.keystroke.key, keys, shortcuts::MAC) else {
            return;
        };
        self.run_shortcut(shortcut, window, cx);
        cx.stop_propagation();
    }

    /// What a shortcut does, from its key or its menu item (`menus`).
    fn run_shortcut(&mut self, shortcut: Shortcut, window: &mut Window, cx: &mut Context<Self>) {
        match shortcut {
            Shortcut::FocusSidebar => window.focus(&self.sidebar_focus, cx),
            Shortcut::FocusTerminal => match &self.pane {
                Pane::Terminal { view, .. } => window.focus(&view.focus_handle(cx), cx),
                Pane::Details(_) => window.focus(&self.pane_focus, cx),
                Pane::Empty => {}
            },
            Shortcut::NewSession => self.show_new_session(window, cx),
            Shortcut::OpenSessions => self.show_open_sessions(window, cx),
            Shortcut::NextNeedingYou => self.jump_to_next_needing(window, cx),
            Shortcut::ToggleRail => self.toggle_rail(cx),
            Shortcut::ThemeSwitcher => self.toggle_theme_switcher(window, cx),
            Shortcut::Quit => {
                Self::save_window_now(window);
                cx.quit();
            }
            Shortcut::Minimize => window.minimize_window(),
            Shortcut::Hide => cx.hide(),
            Shortcut::ToggleShell => self.toggle_shell(window, cx),
        }
    }

    fn sidebar_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        let move_keys =
            modifiers.alt && modifiers.shift && !modifiers.control && !modifiers.platform;
        if move_keys && matches!(event.keystroke.key.as_str(), "up" | "down") {
            let step = if event.keystroke.key == "up" { -1 } else { 1 };
            self.move_selected_to_neighbour_group(step, window, cx);
            cx.stop_propagation();
            return;
        }
        if modifiers.modified() {
            return;
        }
        match event.keystroke.key.as_str() {
            "up" => self.move_selection(-1, cx),
            "down" => self.move_selection(1, cx),
            "enter" => {
                if let Some(id) = self.selected.clone() {
                    self.open(&id, window, cx);
                }
            }
            "left" | "right" => {
                let Some(group) = self.group_of_selected() else {
                    return;
                };
                let collapse = event.keystroke.key == "left";
                self.set_group_collapsed(group, collapse, window, cx);
            }
            "escape" => {
                self.confirm_close = None;
                self.confirm_delete = None;
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    // ---- drawing ------------------------------------------------------------------

    fn render_item(&self, item: &Item, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        match item {
            Item::NeedsYouHeader { count } => div()
                .flex()
                .items_baseline()
                .gap(px(6.))
                .pt(px(6.))
                .pb(px(4.))
                .px(px(8.))
                .text_size(px(13.))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(hsla(chrome.amber))
                        .child("Needs you"),
                )
                .child(
                    div()
                        .text_color(hsla(chrome.muted))
                        .child(count.to_string()),
                )
                .into_any_element(),
            Item::GroupHeader {
                id,
                name,
                total,
                needing,
                collapsed,
                color,
                picked,
            } => self.render_group_header(
                *id,
                name,
                *total,
                *needing,
                *collapsed,
                (*color, *picked),
                cx,
            ),
            Item::NewGroup => self.render_new_group_line(cx),
            Item::UngroupedHeader { total } => self.render_ungrouped_header(*total),
            Item::Row(row) => self.render_row(row, window, cx),
            Item::Divider => div()
                .my(px(8.))
                .mx(px(8.))
                .h(px(1.))
                .bg(hsla(chrome.sidebar_line))
                .into_any_element(),
        }
    }

    fn render_row(&self, row: &Row, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        // The open session (shown in the pane) and the keyboard's place are different
        // things and look different: a filled row with a bar, and an outline.
        let active = self.shown_id() == Some(row.id.as_str());
        let keyboard_here = self.selected.as_deref() == Some(row.id.as_str())
            && !active
            && self.sidebar_focus.is_focused(window);
        let open_id = row.id.clone();
        let row_key = format!("row-{}-{}", row.section.key(), row.id);
        let title = self.render_row_title(row, &row_key, active, cx);
        let dragged = DraggedSession {
            id: row.id.clone(),
            title: row.title.clone(),
            agent: row.agent,
            chrome,
            marks: self.marks.clone(),
        };
        let close_id = row.id.clone();
        let hover_key = row_key.clone();
        let mut line = div()
            // A session can show twice (pinned and in its group); ids keep them apart.
            .id(SharedString::from(row_key.clone()))
            .group(SharedString::from(row_key.clone()))
            .h(px(ROW_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(ROW_GAP))
            .px(px(8.))
            .rounded(px(10.))
            .cursor_pointer()
            .child(self.marks.element(row.agent, 18., chrome.muted))
            .child(title)
            .children(row.external.then(|| modal::tag("ext", chrome)))
            .child(match row.status_word() {
                Some(word) if row.breathing() => working_word(word, self.lit_dot(), chrome),
                Some(word) => div()
                    .flex_none()
                    .text_right()
                    .text_size(px(12.5))
                    .text_color(hsla(chrome.amber))
                    .child(word)
                    .into_any_element(),
                None => div()
                    .w(px(32.))
                    .flex_none()
                    .text_right()
                    .text_size(px(13.))
                    .text_color(hsla(chrome.muted))
                    .when(row.dimmed(), |age| age.opacity(DIMMED))
                    .child(row.age.clone())
                    .into_any_element(),
            })
            .when(row.glyph_cell(), |line| {
                line.child(glyph::status(row.glyph, chrome))
            })
            .child(self.hover_close(
                &row_key,
                ROW_GAP,
                "Close",
                move |this, _, cx| this.request_close(close_id.clone(), cx),
                cx,
            ))
            .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                this.set_hover(&hover_key, *over, cx);
            }))
            .on_click(cx.listener(move |this, _, window, cx| this.open(&open_id, window, cx)))
            .on_drag(dragged, |dragged, _, _, cx| cx.new(|_| dragged.clone()));
        let mut hover = hsla(chrome.selected);
        hover.a = 0.55;
        line = line
            .relative()
            .border_1()
            .border_color(if keyboard_here {
                hsla(chrome.track)
            } else {
                gpui::transparent_black()
            })
            .hover(move |style| style.bg(hover));
        if active {
            line = line.bg(hsla(chrome.selected)).child(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px(9.))
                    .bottom(px(9.))
                    .w(px(3.))
                    .rounded(px(2.))
                    .bg(hsla(chrome.text)),
            );
        }
        // A group's rows stand beside a line in the group's colour (add-group-colors D4).
        let group_color = match row.section {
            model::Section::Group(id) => self
                .groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| self.theme.group_color(group.shown_color())),
            model::Section::Pinned | model::Section::Ungrouped => None,
        };
        match group_color {
            Some(color) => div()
                .flex_none()
                .ml(px(GROUP_LINE_INDENT))
                .pl(px(4.))
                .border_l_2()
                .border_color(hsla(color))
                .child(line)
                .into_any_element(),
            None => line.into_any_element(),
        }
    }

    /// A row's title. The open session's title renames it when clicked (the pointer
    /// becomes a text cursor over it); any other row's title opens the session like the
    /// rest of the row.
    fn render_row_title(
        &self,
        row: &Row,
        row_key: &str,
        active: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        if let Some(rename) = self
            .rename
            .as_ref()
            .filter(|rename| rename.row_key == row_key)
        {
            return div()
                .flex_1()
                .min_w_0()
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.finish_rename(None, window, cx);
                        cx.stop_propagation();
                    }
                }))
                .child(Input::new(&rename.input).small())
                .into_any_element();
        }
        let title_color = if row.dimmed() {
            chrome.muted
        } else {
            chrome.text
        };
        let mut title = div()
            .min_w_0()
            .truncate()
            .text_size(px(14.))
            .text_color(hsla(title_color))
            .child(row.title.clone());
        if row.bold() {
            title = title.font_weight(gpui::FontWeight::SEMIBOLD);
        }
        if !active {
            return div().flex_1().min_w_0().child(title).into_any_element();
        }
        let (id, key) = (row.id.clone(), row_key.to_owned());
        div()
            .id(SharedString::from(format!("title-{row_key}")))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .child(title.cursor_text())
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.start_rename(id.clone(), key.clone(), window, cx);
            }))
            .into_any_element()
    }

    /// Records which row or group header the pointer is over, so its ✕ slides in, and
    /// the one it left slides out.
    pub(super) fn set_hover(&mut self, key: &str, over: bool, cx: &mut Context<Self>) {
        let here = self.hover.over.as_deref() == Some(key);
        if over && !here {
            self.hover.left = self.hover.over.replace(key.to_owned());
        } else if !over && here {
            self.hover.left = self.hover.over.take();
        } else {
            return;
        }
        self.hover.generation += 1;
        cx.notify();
    }

    /// The ✕ at the right end of a row or group header. It widens from nothing while the
    /// pointer is over its owner, so what sits before it slides left instead of jumping,
    /// and narrows again when the pointer leaves (D6).
    pub(super) fn hover_close(
        &self,
        key: &str,
        gap: f32,
        tooltip: &'static str,
        on_press: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let over = self.hover.over.as_deref() == Some(key);
        let leaving = self.hover.left.as_deref() == Some(key);
        // The slot sits in a flex row with `gap` between items; a negative margin hides
        // that gap while the slot is closed.
        let slot = div()
            .flex_none()
            .h(px(CLOSE_SLOT))
            .overflow_hidden()
            .flex()
            .justify_end();
        if !over && !leaving {
            return slot.w(px(0.)).ml(px(-gap)).into_any_element();
        }
        let chrome = self.theme.chrome;
        let mut hover = hsla(chrome.track);
        hover.a = 0.6;
        let text = hsla(chrome.text);
        let button = div()
            .id(SharedString::from(format!("close-{key}")))
            .flex_none()
            .size(px(CLOSE_SLOT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.))
            .text_color(hsla(chrome.muted))
            .hover(move |style| style.bg(hover).text_color(text))
            .child(Icon::new(IconName::Close).small())
            .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                on_press(this, window, cx);
            }));
        let slot = slot.child(button);
        let open = move |slot: gpui::Div, amount: f32| {
            slot.w(px(CLOSE_SLOT * amount))
                .ml(px(-gap * (1.0 - amount)))
                .opacity(amount)
        };
        if crate::settings::reduce_motion() {
            return open(slot, if over { 1.0 } else { 0.0 }).into_any_element();
        }
        let direction = if over { "in" } else { "out" };
        slot.with_animation(
            SharedString::from(format!("slide-{key}-{}-{direction}", self.hover.generation)),
            Animation::new(SLIDE).with_easing(|t: f32| 1.0 - (1.0 - t).powi(3)),
            move |slot, delta| open(slot, if over { delta } else { 1.0 - delta }),
        )
        .into_any_element()
    }

    /// Asks before closing a session, saying what closing will do to it (D6).
    fn render_confirm_close(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = self.confirm_close.clone()?;
        let session = self.session(&id)?;
        let chrome = self.theme.chrome;
        let inactive = session.external.is_none() && !session.live && !session.starting;
        let state = if inactive {
            "Its conversation stays in the agent's history; Add sessions can bring it back."
                .to_owned()
        } else if session.external.is_some() || !session.live {
            "It leaves the sidebar. Its conversation stays in the agent's history.".to_owned()
        } else {
            let now = match session.status {
                Status::NeedsYou => "It needs you. ",
                Status::Working => "It is still working. ",
                _ => "",
            };
            format!(
                "{now}Closing stops the agent and takes the session out of the sidebar. Its conversation stays in the agent's history."
            )
        };
        let title = hq_protocol::sidebar::display_title(session);
        Some(
            div()
                .id("confirm-close-backdrop")
                // The click that closes the dialog must not also reach what is behind.
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
                    cx.listener(|this, _, _, cx| {
                        this.confirm_close = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .id("confirm-close")
                        .w(px(380.))
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
                        // Clicks inside the dialog do not reach the backdrop.
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(if inactive {
                                    format!("Remove \u{201c}{title}\u{201d} from the sidebar?")
                                } else {
                                    format!("Close \u{201c}{title}\u{201d}?")
                                }),
                        )
                        .child(div().text_color(hsla(chrome.muted)).child(state))
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(Button::new("close-cancel").label("Cancel").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.confirm_close = None;
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    Button::new("close-confirm")
                                        .danger()
                                        .label(if inactive { "Remove" } else { "Close" })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.close_session(id.clone(), window, cx);
                                        })),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }

    fn render_empty_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chrome = self.theme.chrome;
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .px(px(16.))
            .text_center()
            .text_size(px(14.))
            .text_color(hsla(chrome.muted))
            .child("Add the sessions you want to keep in view. Sessions you start here join on their own.")
            .child(
                Button::new("empty-add")
                    .primary()
                    .label("Add sessions")
                    .on_click(cx.listener(|this, _, window, cx| this.show_open_sessions(window, cx))),
            )
    }

    /// Opens or closes one agent's details; a second click on the open ring closes them.
    fn toggle_usage(&mut self, agent: Agent, cx: &mut Context<Self>) {
        if !self.just_dismissed(Some(agent)) {
            let open = self.usage_open == Some(agent) && !self.usage_all;
            self.usage_open = (!open).then_some(agent);
            self.usage_all = false;
        }
        cx.notify();
    }

    /// Opens or closes the details of every agent, from the "+N" chip.
    fn toggle_usage_all(&mut self, first: Agent, cx: &mut Context<Self>) {
        if !self.just_dismissed(None) {
            let open = self.usage_open.is_some() && self.usage_all;
            self.usage_open = (!open).then_some(first);
            self.usage_all = !open;
        }
        cx.notify();
    }

    /// Whether the backdrop has just closed this panel, in the same click.
    fn just_dismissed(&mut self, panel: Option<Agent>) -> bool {
        self.usage_dismissed
            .take()
            .is_some_and(|(closed, at)| closed == panel && at.elapsed() < DISMISS_CLICK)
    }

    /// The host answers once the new readings are in; they reach the details through
    /// the usage feed.
    fn refresh_usage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.usage_refreshing {
            return;
        }
        self.usage_refreshing = true;
        cx.notify();
        self.host_call(
            window,
            cx,
            |paths| Ok(hq_client::refresh_usage(paths)),
            |this, refreshed, _, _| {
                this.usage_refreshing = false;
                if let Err(error) = refreshed {
                    this.message = Some(format!("{error:#}"));
                }
            },
        );
    }

    /// Centred rings, nearest the limit first, as many as fit on one line; the rest
    /// fold into a "+N" chip that opens every agent's details. Below them, the label
    /// naming the agent nearest its limit (refresh-usage-on-activity D6, option A).
    fn render_usage_strip(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.usage.is_empty() {
            return None;
        }
        let chrome = self.theme.chrome;
        let now = hq_signals::now_ms() / 1000;
        let ordered = self.usage_by_limit(now);
        let room = self.sidebar_width() - 20.;
        let fit = (((room + RING_GAP) / (RING_PX + RING_GAP)).floor() as usize).max(1);
        let shown = if ordered.len() > fit {
            fit - 1
        } else {
            ordered.len()
        };
        let mut rings = div()
            .flex()
            .justify_center()
            .items_center()
            .gap(px(RING_GAP));
        for usage in &ordered[..shown] {
            rings = rings.child(self.usage_ring(usage, now, cx));
        }
        if let Some(first_hidden) = ordered.get(shown).map(|usage| usage.agent) {
            let hidden = ordered.len() - shown;
            let open = self.usage_open.is_some() && self.usage_all;
            let mut chip = div()
                .id("usage-more")
                .h(px(28.))
                .min_w(px(36.))
                .px(px(8.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(14.))
                .border_1()
                .border_color(hsla(chrome.sidebar_line))
                .cursor_pointer()
                .text_size(px(12.5))
                .text_color(hsla(chrome.text))
                .child(format!("+{hidden}"))
                .tooltip(move |window, cx| {
                    Tooltip::new(format!("{hidden} more agents")).build(window, cx)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_usage_all(first_hidden, cx);
                }));
            if open {
                chip = chip.bg(hsla(chrome.selected));
            }
            rings = rings.child(chip);
        }
        let label = usage::label(&self.usage, now);
        Some(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .py(px(8.))
                .border_t_1()
                .border_color(hsla(chrome.sidebar_line))
                .child(rings)
                .children(label.map(|label| {
                    div()
                        .text_size(px(12.5))
                        .text_color(hsla(chrome.muted))
                        .child(label)
                })),
        )
    }

    /// Agents nearest their limit first; unreadable ones last, in their usual order.
    fn usage_by_limit(&self, now: u64) -> Vec<&AgentUsage> {
        let mut ordered: Vec<&AgentUsage> = self.usage.iter().collect();
        ordered.sort_by(|a, b| {
            let fill = |usage: &AgentUsage| usage::fill(usage, now).unwrap_or(-1.0);
            fill(b).total_cmp(&fill(a))
        });
        ordered
    }

    /// One agent's usage ring: its share used, its mark, and a click or Enter to open
    /// the details.
    fn usage_ring(&self, usage: &AgentUsage, now: u64, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let agent = usage.agent;
        let rings = usage::rings(usage, now);
        let note = usage::update_note(usage, &chrono::Local::now());
        let color = |percent: Option<f64>| {
            if note.is_some() {
                chrome.muted
            } else if percent.is_some_and(|percent| percent >= usage::NEAR_LIMIT_PERCENT) {
                self.theme.near_limit()
            } else {
                chrome.text
            }
        };
        let who = usage.account.as_ref().map_or_else(
            || agent.to_string(),
            |account| format!("{agent} ({})", usage::account_text(account)),
        );
        let used: Vec<String> = usage::ring_windows(usage, now)
            .into_iter()
            .map(|window| {
                format!(
                    "{} {}",
                    usage::window_name(window),
                    usage::percent_text(window.used_percent)
                )
            })
            .collect();
        let description = match (usage.read_at, used.is_empty()) {
            (Some(_), false) => match &note {
                Some(note) => format!("{who}: {} used. {note}", used.join(", ")),
                None => format!("{who}: {} used", used.join(", ")),
            },
            _ => format!(
                "{who}: {}",
                usage.problem.as_deref().unwrap_or("usage not available")
            ),
        };
        let mark_size = if rings.inner.is_some() { 11. } else { 16. };
        let mut ring = div()
            .id(SharedString::from(format!("usage-{agent}")))
            .rounded(px(8.))
            .cursor_pointer()
            .child(glyph::usage_ring(
                agent,
                rings,
                color(rings.outer),
                color(rings.inner),
                chrome,
                self.marks
                    .has(agent)
                    .then(|| self.marks.element(agent, mark_size, chrome.muted)),
            ))
            .tooltip(move |window, cx| Tooltip::new(description.clone()).build(window, cx))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_usage(agent, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_usage(agent, cx);
                    cx.stop_propagation();
                }
            }));
        if let Some(focus) = self.ring_focus.get(&agent) {
            ring = ring.track_focus(focus);
        }
        if self.usage_open == Some(agent) && !self.usage_all {
            ring = ring.bg(hsla(chrome.selected));
        }
        ring.into_any_element()
    }

    /// Details for the open ring: each window with a bar, share used and reset time,
    /// the reading's age, what the limits belong to, any problem, and Refresh. The
    /// rail has one ring, so from the rail every agent is listed (owner review,
    /// 2026-10-20).
    fn render_usage_popover(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let agent = self.usage_open?;
        let rail = self.rail_shown();
        let shown: Vec<&AgentUsage> = if rail || self.usage_all {
            self.usage_by_limit(hq_signals::now_ms() / 1000)
        } else {
            self.usage
                .iter()
                .filter(|usage| usage.agent == agent)
                .collect()
        };
        if shown.is_empty() {
            return None;
        }
        let chrome = self.theme.chrome;
        let mut panel = div()
            .id("usage-popover")
            .absolute()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(12.))
            .rounded(px(12.))
            .border_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(hsla(chrome.sidebar))
            .shadow_lg()
            .text_size(px(13.))
            .text_color(hsla(chrome.text));
        // The details float at their own width, over the pane when the sidebar is
        // narrower (owner, 2026-10-21).
        panel = if rail {
            panel
                .left(px(self.sidebar_width() + 8.))
                .bottom(px(12.))
                .w(px(USAGE_PANEL_WIDTH))
        } else {
            panel
                .left(px(10.))
                .bottom(px(64.))
                .w(px((self.sidebar_width() - 20.).max(USAGE_PANEL_WIDTH)))
        }
        .max_h(px(560.))
        .overflow_y_scroll()
        // Presses inside the details stay with them, not the backdrop that closes them.
        .occlude();
        for (index, usage) in shown.into_iter().enumerate() {
            if index > 0 {
                panel = panel.child(div().h(px(1.)).bg(hsla(chrome.sidebar_line)));
            }
            panel = panel.child(self.usage_details(usage));
        }
        Some(
            panel.child(
                div().child(
                    Button::new("usage-refresh")
                        .icon(IconName::RefreshCw)
                        .label(if self.usage_refreshing {
                            "Refreshing…"
                        } else {
                            "Refresh"
                        })
                        .loading(self.usage_refreshing && !settings::reduce_motion())
                        .disabled(self.usage_refreshing)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.refresh_usage(window, cx)),
                        ),
                ),
            ),
        )
    }

    /// One agent's part of the usage details.
    fn usage_details(&self, usage: &AgentUsage) -> gpui::Div {
        let chrome = self.theme.chrome;
        let now = hq_signals::now_ms() / 1000;
        let local_now = chrono::Local::now();
        let mut part = div().flex().flex_col().gap(px(8.)).child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("{} usage", usage.agent)),
                )
                .children(usage.account.as_ref().map(|account| {
                    div()
                        .text_size(px(12.5))
                        .text_color(hsla(chrome.muted))
                        .child(usage::account_text(account))
                })),
        );
        if let Some(read_at) = usage.read_at {
            for row in usage::window_rows(usage) {
                let window = match row {
                    usage::WindowRow::Reading(window)
                        if !usage::has_reset(window, read_at, now) =>
                    {
                        window
                    }
                    usage::WindowRow::Reading(window) => {
                        part = part.child(reset_row(
                            usage::window_name(window),
                            usage::RESET_SINCE,
                            chrome.muted,
                        ));
                        continue;
                    }
                    usage::WindowRow::ResetBefore(minutes) => {
                        part = part.child(reset_row(
                            usage::length_name(minutes),
                            usage::RESET_BEFORE,
                            chrome.muted,
                        ));
                        continue;
                    }
                };
                let name = usage::window_name(window);
                let share = window.used_percent.clamp(0.0, 100.0);
                let bar_color = if share >= usage::NEAR_LIMIT_PERCENT {
                    self.theme.near_limit()
                } else {
                    chrome.text
                };
                let reset = window.resets_at.map_or_else(
                    || "reset time unknown".to_owned(),
                    |at| usage::reset_text(at, &local_now),
                );
                part = part.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(window_name_cell(name))
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(6.))
                                        .rounded(px(3.))
                                        .bg(hsla(chrome.track))
                                        .child(
                                            div()
                                                .h_full()
                                                .rounded(px(3.))
                                                .w(gpui::relative((share / 100.0) as f32))
                                                .bg(hsla(bar_color)),
                                        ),
                                )
                                .child(
                                    div()
                                        .w(px(40.))
                                        .text_right()
                                        .child(usage::percent_text(share)),
                                ),
                        )
                        .child(
                            div()
                                .pl(px(WINDOW_NAME_PX + 8.))
                                .text_size(px(12.5))
                                .text_color(hsla(chrome.muted))
                                .child(reset),
                        ),
                );
            }
            // No reading age: readings follow activity. Only a failed or overdue
            // update is said, with why (D4).
            if let Some(note) = usage::update_note(usage, &local_now) {
                part = part.child(
                    div()
                        .text_size(px(12.5))
                        .text_color(hsla(chrome.amber))
                        .child(note),
                );
                if let Some(problem) = &usage.problem {
                    part = part.child(
                        div()
                            .text_size(px(12.))
                            .text_color(hsla(chrome.muted))
                            .child(problem.clone()),
                    );
                }
            }
        } else if let Some(problem) = &usage.problem {
            part = part.child(
                div()
                    .text_size(px(12.5))
                    .text_color(hsla(chrome.amber))
                    .child(format!("Not available: {problem}")),
            );
        }
        part
    }

    /// The sidebar's top control: a command field that opens "Add sessions", where
    /// the owner can also find and open any session (owner's choice of three designs).
    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let mut soft = hsla(chrome.selected);
        soft.a = 0.6;
        let strong = hsla(chrome.selected);
        let shortcut = if shortcuts::MAC { "⌘O" } else { "Ctrl⇧O" };
        div()
            .flex_none()
            .pb(px(14.))
            .child(
                div()
                    .id("command-field")
                    .h(px(34.))
                    .overflow_hidden()
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(hsla(chrome.sidebar_line))
                    .bg(soft)
                    .hover(move |style| style.bg(strong))
                    .cursor_text()
                    .child(
                        Icon::new(IconName::Search)
                            .small()
                            .text_color(hsla(chrome.muted)),
                    )
                    .child(
                        // Narrow sidebars shorten the words, never push the shortcut out.
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.5))
                            .text_color(hsla(chrome.muted))
                            .child("Add or find sessions…"),
                    )
                    .child(
                        div()
                            .flex_none()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(hsla(chrome.track))
                            .text_size(px(11.5))
                            .text_color(hsla(chrome.muted))
                            .child(shortcut),
                    )
                    .on_click(
                        cx.listener(|this, _, window, cx| this.show_open_sessions(window, cx)),
                    ),
            )
            .into_any_element()
    }

    fn render_pane_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let background = translucent(
            Token::Background.get(&self.theme),
            self.look_appearance.opacity,
        );
        // Notes and titles wrap within a readable width instead of running off the pane.
        let note = |text: String| {
            div()
                .w_full()
                .max_w(px(PANE_TEXT_WIDTH))
                .text_center()
                .text_size(px(14.))
                .text_color(hsla(chrome.muted))
                .child(text)
        };
        let centred = div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .px(px(24.))
            .bg(background);
        match &self.pane {
            // Cached, so a sidebar frame (a breathing word) does not repaint the terminal.
            Pane::Terminal { view, .. } => match self.shell_view() {
                Some(shell) => self.render_split(view.clone(), shell.clone(), cx),
                None => div()
                    .size_full()
                    .child(view.clone().cached(StyleRefinement::default().size_full()))
                    .into_any_element(),
            },
            Pane::Empty if !self.listed => centred
                .child(crate::loading::agents_checking_in(
                    "workspace-starting",
                    &self.marks,
                    STARTING_MARK,
                    chrome,
                    "Starting Headquarter…",
                ))
                .into_any_element(),
            Pane::Empty => centred
                .child(note(if self.items().is_empty() {
                    "Add sessions with + to keep them in view here.".to_owned()
                } else {
                    "Choose a session in the sidebar.".to_owned()
                }))
                .into_any_element(),
            Pane::Details(id) => {
                let Some(session) = self.session(id) else {
                    return centred
                        .child(note(
                            "This session is no longer known to the host.".to_owned(),
                        ))
                        .into_any_element();
                };
                let title = hq_protocol::sidebar::display_title(session);
                let heading = div()
                    .w_full()
                    .max_w(px(PANE_TEXT_WIDTH))
                    .text_center()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(hsla(chrome.text))
                    .child(title);
                let id = id.clone();
                if let Some(external) = &session.external {
                    let pid = external
                        .pid
                        .map_or_else(String::new, |pid| format!(" (process {pid})"));
                    if !external.background {
                        return centred
                            .child(heading)
                            .child(note(format!(
                                "Running in another terminal{pid}. Headquarter shows its status; close it there to continue it here."
                            )))
                            .into_any_element();
                    }
                    if self.resuming.as_deref() == Some(id.as_str()) {
                        return centred
                            .child(heading)
                            .child(note("Bringing it into Headquarter…".to_owned()))
                            .into_any_element();
                    }
                    let failed = self
                        .resume_failed
                        .as_ref()
                        .filter(|(failed, _, _)| *failed == id)
                        .map(|(_, reason, _)| reason.clone());
                    return centred
                        .child(heading)
                        .child(note(format!(
                            "Running in Claude Code's background{pid}. Bring it into Headquarter to continue it here."
                        )))
                        .children(failed.map(note))
                        .child(
                            Button::new("pane-bring-in")
                                .primary()
                                .label("Bring into Headquarter")
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, window, cx| this.bring_in(id.clone(), window, cx)
                                })),
                        )
                        .into_any_element();
                }
                if session.live {
                    return centred
                        .child(heading)
                        .child(Button::new("pane-open").primary().label("Open").on_click(
                            cx.listener(move |this, _, window, cx| this.open(&id, window, cx)),
                        ))
                        .into_any_element();
                }
                if self.resuming.as_deref() == Some(id.as_str()) {
                    return centred
                        .child(heading)
                        .child(note("Opening…".to_owned()))
                        .into_any_element();
                }
                if session.starting {
                    return centred
                        .child(heading)
                        .child(note("Starting…".to_owned()))
                        .into_any_element();
                }
                // Inactive: the agent starts only when the owner asks (D4). The reason
                // a start failed comes from this window's try, else from the host's.
                let in_background = self
                    .resume_failed
                    .as_ref()
                    .is_some_and(|(failed, _, background)| *failed == id && *background);
                let reason = self
                    .resume_failed
                    .as_ref()
                    .filter(|(failed, _, _)| *failed == id)
                    .map(|(_, reason, _)| reason.clone())
                    .or_else(|| {
                        session
                            .start_error
                            .as_ref()
                            .map(|error| format!("Couldn't start: {}", error.reason))
                    });
                let folder = hq_protocol::sidebar::home_path(&session.cwd, self.home.as_deref());
                // With its folder gone the agent cannot start there, so the pane offers
                // to remove the session instead (owner, 2026-10-24).
                let folder_gone = !std::path::Path::new(&session.cwd).is_dir();
                let reason = if folder_gone {
                    Some(format!("The folder {folder} no longer exists."))
                } else {
                    reason
                };
                let action = if folder_gone {
                    Button::new("pane-remove")
                        .danger()
                        .label("Remove from Sidebar")
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |this, _, _, cx| this.request_close(id.clone(), cx)
                        }))
                } else if in_background {
                    Button::new("pane-bring-in")
                        .primary()
                        .label("Bring into Headquarter")
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |this, _, window, cx| this.bring_in(id.clone(), window, cx)
                        }))
                } else {
                    Button::new("pane-resume")
                        .primary()
                        .label("Resume Session")
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |this, _, window, cx| this.resume(id.clone(), window, cx)
                        }))
                };
                centred
                    .id("pane-inactive")
                    .track_focus(&self.pane_focus)
                    .on_key_down(cx.listener({
                        let id = id.clone();
                        move |this, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.key == "enter"
                                && this.resuming.is_none()
                                && !folder_gone
                            {
                                cx.stop_propagation();
                                this.resume(id.clone(), window, cx);
                            }
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(self.marks.element(session.agent, 18., chrome.muted))
                            .child(heading),
                    )
                    .child(note(folder))
                    .child(note("Inactive".to_owned()))
                    .children(reason.map(|reason| {
                        div()
                            .w_full()
                            .max_w(px(PANE_TEXT_WIDTH))
                            .text_center()
                            .text_size(px(13.))
                            .text_color(hsla(chrome.amber))
                            .child(reason)
                    }))
                    .child(action)
                    .into_any_element()
            }
        }
    }
}

fn system_mode(appearance: WindowAppearance) -> Mode {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Mode::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => Mode::Light,
    }
}

/// `color` at the window's background opacity.
/// `color` at the window's background opacity, times the colour's own.
fn translucent(color: impl Into<crate::theme::Paint>, opacity: f32) -> gpui::Hsla {
    let mut color = hsla(color);
    color.a *= opacity;
    color
}

/// A usage window shown without a number, with why.
/// A window's name in usage details, shared by every row so all keep it on one line:
/// "week · Gemini 4 Argon" never wraps; the bar or the words beside it give way
/// (fix-usage-reset-row-names).
fn window_name_cell(name: String) -> gpui::Div {
    div()
        .flex_none()
        .min_w(px(WINDOW_NAME_PX))
        .whitespace_nowrap()
        .child(name)
}

fn reset_row(name: String, words: &'static str, muted: crate::theme::Paint) -> gpui::Div {
    div()
        .flex()
        .gap(px(8.))
        .child(window_name_cell(name))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_color(hsla(muted))
                .child(words),
        )
}

fn overlay(chrome: Chrome) -> gpui::Hsla {
    let mut shade = hsla(chrome.text);
    shade.l = 0.08;
    shade.a = 0.4;
    shade
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // A drag that ended outside any drop target still ends the resize.
        if self.resizing && !cx.has_active_drag() {
            self.finish_resize(cx);
        }
        self.follow_shown();
        let chrome = self.theme.chrome;
        let pane = self.render_pane(window, cx);
        let usage_popover = self.render_usage_popover(cx);
        // A click anywhere outside the details closes them.
        let usage_backdrop = usage_popover.is_some().then(|| {
            div()
                .id("usage-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        let panel = (!this.usage_all).then_some(this.usage_open).flatten();
                        this.usage_dismissed = Some((panel, Instant::now()));
                        this.usage_open = None;
                        this.usage_all = false;
                        cx.notify();
                    }),
                )
        });
        let creator = match &self.modal {
            Some(Modal::Creator { view, .. }) => Some(view.clone()),
            _ => None,
        };
        let modal = self.modal.as_ref().and_then(|modal| {
            let content = match modal {
                Modal::Open { view: open, .. } => open.clone().into_any_element(),
                Modal::New { view: new, .. } => new.clone().into_any_element(),
                Modal::Theme { view: switcher, .. } => switcher.clone().into_any_element(),
                // The creator keeps the app in view: no dimming, its own layer.
                Modal::Creator { .. } => return None,
            };
            Some(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(overlay(chrome))
                    // Clicks on the window's empty parts or around it must not reach the
                    // sidebar underneath, whose command field would open a new window and
                    // drop the picks.
                    .occlude()
                    .child(content),
            )
        });
        div()
            .size_full()
            .relative()
            .flex()
            .text_size(px(14.))
            .on_key_down(cx.listener(Self::key_down))
            .on_action(cx.listener(|this, _: &menus::ThemeSwitcher, window, cx| {
                this.run_shortcut(Shortcut::ThemeSwitcher, window, cx);
            }))
            .on_action(cx.listener(|this, _: &menus::Minimize, window, cx| {
                this.run_shortcut(Shortcut::Minimize, window, cx);
            }))
            .on_action(cx.listener(|this, _: &menus::NewSession, window, cx| {
                this.run_shortcut(Shortcut::NewSession, window, cx);
            }))
            .on_action(cx.listener(|this, _: &menus::OpenSessions, window, cx| {
                this.run_shortcut(Shortcut::OpenSessions, window, cx);
            }))
            .on_action(cx.listener(|this, _: &menus::CheckForUpdates, window, cx| {
                this.check_for_update(true, window, cx);
            }))
            .on_action(cx.listener(|this, _: &menus::RemoteControl, window, cx| {
                this.open_remote_window(window, cx);
            }))
            .on_action(cx.listener(|this, _: &menus::ToggleShell, window, cx| {
                this.run_shortcut(Shortcut::ToggleShell, window, cx);
            }))
            .on_drag_move(cx.listener(
                |this, event: &gpui::DragMoveEvent<rail::DividerDrag>, _, cx| {
                    this.drag_divider(f32::from(event.event.position.x), cx);
                },
            ))
            .on_drop(cx.listener(|this, _: &rail::DividerDrag, _, cx| {
                this.finish_resize(cx);
            }))
            .child(
                self.sidebar_view.clone().cached(
                    StyleRefinement::default()
                        .w(px(self.sidebar_width()))
                        .flex_none()
                        .h_full(),
                ),
            )
            .child(div().flex_1().min_w_0().h_full().child(pane))
            .child(self.render_divider(cx))
            .children(usage_backdrop)
            .children(usage_popover)
            .children(modal)
            .children(creator)
            .children(self.render_confirm_delete(cx))
            .children(self.render_confirm_close(cx))
            .children(self.render_update_window(cx))
            .children(self.render_remote_window(cx))
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.sidebar_focus.clone()
    }
}

/// Runs on its own thread: lists sessions, then forwards every change.
fn watch_sessions(tx: &UnboundedSender<ListEvent>) {
    let fail = |message: String| {
        let _ = tx.unbounded_send(ListEvent::Failed(message));
    };
    let Some(paths) = HostPaths::standard() else {
        return fail("cannot find the home folder".to_owned());
    };
    let (mut feed, catalog) = match SessionFeed::open(&paths, true) {
        Ok(opened) => opened,
        Err(error) => return fail(format!("cannot reach the Headquarter host: {error:#}")),
    };
    if let Ok(control) = feed.control()
        && tx.unbounded_send(ListEvent::Control(control)).is_err()
    {
        return;
    }
    let all = ListEvent::All {
        sessions: catalog.sessions,
        groups: catalog.groups,
        usage: catalog.usage,
    };
    if tx.unbounded_send(all).is_err() {
        return;
    }
    loop {
        let event = match feed.next_change() {
            Ok(Some(Change::Session(session))) => ListEvent::Changed(session),
            Ok(Some(Change::Groups(groups))) => ListEvent::Groups(groups),
            Ok(Some(Change::Usage(agents))) => ListEvent::Usage(agents),
            Ok(Some(Change::Details { session, details })) => {
                ListEvent::Details { session, details }
            }
            Ok(None) => return fail("the Headquarter host closed the connection".to_owned()),
            Err(error) => return fail(format!("{error:#}")),
        };
        if tx.unbounded_send(event).is_err() {
            return;
        }
    }
}

/// A working or starting row's word in the accent colour, then three dots lit one after
/// another (refresh-pane-and-status D1). `lit` is `None` while motion is held still, when
/// all three show at full strength, and `Some(None)` on the step where all rest.
fn working_word(word: &'static str, lit: Option<Option<usize>>, chrome: Chrome) -> AnyElement {
    let accent = hsla(chrome.accent);
    let dot = |index: usize| {
        let bright = match lit {
            None => true,
            Some(lit) => lit == Some(index),
        };
        div()
            .size(px(3.5))
            .rounded_full()
            .bg(accent)
            .opacity(if bright { 1.0 } else { 0.3 })
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(2.5))
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(accent)
        .child(div().mr(px(1.5)).child(word))
        .child(dot(0))
        .child(dot(1))
        .child(dot(2))
        .into_any_element()
}

/// A failed resume for the session pane: what the host said, and whether it was because the
/// conversation runs in Claude Code's background.
fn refusal(error: anyhow::Error) -> (String, bool) {
    let background = error
        .downcast_ref::<hq_client::Refused>()
        .is_some_and(|refused| refused.code == hq_protocol::ErrorCode::RunningInBackground);
    (format!("{error:#}"), background)
}
