//! The update pill and window (add-in-app-update): checking for a newer release, the
//! What's new page in the pane with every release since the installed one
//! (add-update-notes), the window that installs it with its steps, the question about
//! busy sessions, and the restart onto the new version. What talks to GitHub and runs
//! the installer is in `crate::update`.

use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, AsyncWindowContext, Context, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, StatefulInteractiveElement, Styled, Task, WeakEntity,
    Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use hq_client::HostPaths;

use super::{Pane, Workspace, overlay};
use crate::terminal::view::hsla;
use crate::update::{self, Installed, ReleaseNotes, Step, Version};

/// Buttons shown together share this width, so a pair reads as one choice (owner,
/// 2026-10-26).
const BUTTON_WIDTH: f32 = 112.0;
/// The What's new page keeps its text to a readable width.
const NOTES_WIDTH: f32 = 720.0;
const RELEASES_PAGE: &str = "https://github.com/alfonsusenrico/headquarter/releases";

/// The notes of the releases since the installed one, for the available update.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Notes {
    NotLoaded,
    Loading,
    Loaded(Vec<ReleaseNotes>),
    Unavailable,
}

/// Where an update stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Phase {
    /// Nothing started; the window offers the update.
    Offer,
    /// Sessions are busy: the window names them and asks (D5).
    Asking,
    /// The installer runs and has reached this step.
    Running(Step),
    /// Installed; the restart waits until no session is busy.
    Waiting,
    Restarting,
    Failed(String),
}

/// The updater's state in the workspace.
pub(super) struct Updates {
    /// A newer release than this app.
    available: Option<Version>,
    /// The host runs an older version than this app (D7).
    host_behind: bool,
    /// The update window is open.
    open: bool,
    /// The What's new page shows in the pane, over the session it covers.
    page: bool,
    notes: Notes,
    phase: Phase,
    /// What a manual check found, when it found nothing to update.
    checked: Option<String>,
    /// The installer's copy and the app to reopen, when the installer installed this one.
    installed: Option<Installed>,
    /// Restart as soon as the install ends, without waiting for busy sessions.
    restart_now: bool,
    _schedule: Option<Task<()>>,
}

impl Updates {
    pub(super) fn new() -> Updates {
        Updates {
            available: None,
            host_behind: false,
            open: false,
            page: false,
            notes: Notes::NotLoaded,
            phase: Phase::Offer,
            checked: None,
            installed: update::installed_copy(),
            restart_now: false,
            _schedule: None,
        }
    }
}

impl Workspace {
    /// Checks at once, then every `CHECK_EVERY` while the app runs; automatic checks
    /// honour the setting each time (D1, D8).
    pub(super) fn schedule_update_checks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.check_for_update(false, window, cx);
        self.updates._schedule = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(update::CHECK_EVERY).await;
                let checked = this.update_in(cx, |this, window, cx| {
                    this.check_for_update(false, window, cx);
                });
                if checked.is_err() {
                    break;
                }
            }
        }));
    }

    /// Looks for a newer release. A manual check opens the window with what it found;
    /// an automatic one stays quiet unless there is an update.
    pub(super) fn check_for_update(
        &mut self,
        manual: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !manual && !self.check_updates {
            return;
        }
        let task = cx
            .background_executor()
            .spawn(async move { update::latest() });
        cx.spawn_in(window, async move |this, cx| {
            let found = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let running = Version::running();
                match found {
                    Ok(latest) if latest > running => {
                        if this.updates.available != Some(latest) {
                            this.updates.notes = Notes::NotLoaded;
                        }
                        this.updates.available = Some(latest);
                        this.updates.checked = None;
                        if manual {
                            this.show_whats_new(window, cx);
                        }
                    }
                    Ok(_) if manual => {
                        this.updates.checked =
                            Some(format!("Headquarter {running} is the latest version."));
                        this.updates.open = true;
                    }
                    Err(error) if manual => {
                        this.updates.checked =
                            Some(format!("Couldn't check for updates: {error:#}"));
                        this.updates.open = true;
                    }
                    // Automatic checks that find nothing, or fail, say nothing.
                    _ => {}
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Notices a host older than this app, left by an update made elsewhere (D7).
    pub(super) fn check_host_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let task = cx.background_executor().spawn(async move {
            let paths = HostPaths::standard()?;
            hq_client::host_version(&paths).ok()
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(host) = task.await.as_deref().and_then(Version::parse) else {
                return;
            };
            let _ = this.update_in(cx, |this, _, cx| {
                this.updates.host_behind = host < Version::running();
                cx.notify();
            });
        })
        .detach();
    }

    /// The pill at the bottom of the sidebar while an update is available or a restart
    /// is due; `None` otherwise.
    pub(super) fn render_update_pill(
        &self,
        rail: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let label = match (&self.updates.phase, self.updates.available) {
            (Phase::Waiting, _) => "Restart to update".to_owned(),
            (_, Some(version)) => format!("Update Headquarter v{version}"),
            (_, None) if self.updates.host_behind => "Restart to finish update".to_owned(),
            _ => return None,
        };
        let chrome = self.theme.chrome;
        Some(
            div()
                .id("update-pill")
                .flex_none()
                .mb(px(8.))
                .mx(px(if rail { 0. } else { 2. }))
                .py(px(5.))
                .px(px(10.))
                .rounded(px(999.))
                .border_1()
                .border_color(hsla(chrome.accent))
                .text_size(px(12.5))
                .text_center()
                .text_color(hsla(chrome.accent))
                .cursor_pointer()
                .child(if rail { "↑".to_owned() } else { label })
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.updates.available.is_some() && this.updates.phase == Phase::Offer {
                        this.show_whats_new(window, cx);
                    } else {
                        this.updates.open = true;
                    }
                    cx.notify();
                }))
                .into_any_element(),
        )
    }

    /// Opens the What's new page over the shown session, loading the notes once.
    pub(super) fn show_whats_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.page = true;
        self.updates.open = false;
        window.focus(&self.pane_focus, cx);
        if matches!(self.updates.notes, Notes::NotLoaded | Notes::Unavailable) {
            self.load_notes(window, cx);
        }
        cx.notify();
    }

    /// Whether the What's new page covers the pane.
    pub(super) fn whats_new_shown(&self) -> bool {
        self.updates.page && self.updates.available.is_some()
    }

    /// Later: back to the session the page covered, with its focus.
    pub(super) fn close_whats_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.page = false;
        match &self.pane {
            Pane::Terminal { view, .. } => window.focus(&view.focus_handle(cx), cx),
            Pane::Details(_) => window.focus(&self.pane_focus, cx),
            Pane::Empty => window.focus(&self.sidebar_focus, cx),
        }
        cx.notify();
    }

    /// Opening a session from the sidebar leaves the page.
    pub(super) fn leave_whats_new(&mut self) {
        self.updates.page = false;
    }

    fn load_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(latest) = self.updates.available else {
            return;
        };
        self.updates.notes = Notes::Loading;
        let task = cx
            .background_executor()
            .spawn(async move { update::notes_since(Version::running(), latest) });
        cx.spawn_in(window, async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.updates.available != Some(latest) {
                    return;
                }
                this.updates.notes = match found {
                    Ok(notes) if !notes.is_empty() => Notes::Loaded(notes),
                    Ok(_) => Notes::Unavailable,
                    Err(error) => {
                        log::info!("release notes not loaded: {error:#}");
                        Notes::Unavailable
                    }
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// The What's new page: the releases after the installed one, newest first, with
    /// Later and Update Now.
    pub(super) fn render_whats_new(&self, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let Some(latest) = self.updates.available else {
            return div().into_any_element();
        };
        let background = super::translucent(self.theme.palette.background, self.appearance.opacity);
        let muted = hsla(chrome.muted);
        let primary = match &self.updates.installed {
            Some(_) => Button::new("whats-new-update")
                .primary()
                .w(px(BUTTON_WIDTH))
                .label("Update Now")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.updates.open = true;
                    this.begin_update(window, cx);
                })),
            None => {
                // Not installed by the installer: the release page only (add-in-app-update D2).
                let url = update::release_page(latest);
                Button::new("whats-new-page")
                    .primary()
                    .w(px(BUTTON_WIDTH))
                    .label("Open Release Page")
                    .on_click(move |_, _, cx| cx.open_url(&url))
            }
        };
        let top = div()
            .flex_none()
            .flex()
            .items_end()
            .gap(px(12.))
            .px(px(36.))
            .pt(px(22.))
            .pb(px(16.))
            .border_b_1()
            .border_color(hsla(chrome.sidebar_line))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_size(px(12.5)).text_color(muted).child(format!(
                        "Update available · Installed version: v{}",
                        Version::running()
                    )))
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(format!("What's new in Headquarter v{latest}")),
                    ),
            )
            .child(
                Button::new("whats-new-later")
                    .w(px(BUTTON_WIDTH))
                    .label("Later")
                    .on_click(cx.listener(|this, _, window, cx| this.close_whats_new(window, cx))),
            )
            .child(primary);
        let link = |label: &'static str| {
            div()
                .id(label)
                .mt(px(10.))
                .text_size(px(13.))
                .text_color(hsla(chrome.accent))
                .cursor_pointer()
                .hover(|style| style.underline())
                .child(label)
                .on_click(|_, _, cx| cx.open_url(RELEASES_PAGE))
        };
        let mut content = div()
            .w_full()
            .max_w(px(NOTES_WIDTH))
            .flex()
            .flex_col()
            .gap(px(26.))
            .px(px(36.))
            .pt(px(20.))
            .pb(px(32.));
        match &self.updates.notes {
            Notes::NotLoaded | Notes::Loading => {
                content =
                    content.child(div().text_color(muted).child("Loading the release notes…"));
            }
            Notes::Unavailable => {
                content = content
                    .child(
                        div()
                            .text_color(muted)
                            .child("The release notes couldn't be loaded."),
                    )
                    .child(link("Release notes on GitHub ↗"));
            }
            Notes::Loaded(releases) => {
                for (index, release) in releases.iter().enumerate() {
                    content = content.child(self.render_release(release, index == 0));
                }
                content = content.child(link("All releases on GitHub ↗"));
            }
        }
        div()
            .id("whats-new")
            .track_focus(&self.pane_focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_size(px(14.))
            .text_color(hsla(chrome.text))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this.close_whats_new(window, cx);
                }
            }))
            .child(top)
            .child(
                div()
                    .id("whats-new-notes")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .child(content),
            )
            .into_any_element()
    }

    /// One release: its version and date over its changes, each headline in bold.
    fn render_release(&self, release: &ReleaseNotes, latest: bool) -> AnyElement {
        let chrome = self.theme.chrome;
        let muted = hsla(chrome.muted);
        let mut heading = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .pb(px(6.))
            .border_b_1()
            .border_color(hsla(chrome.sidebar_line))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("v{}", release.version)),
            );
        if latest {
            heading = heading.child(
                div()
                    .px(px(7.))
                    .rounded(px(999.))
                    .border_1()
                    .border_color(hsla(chrome.track))
                    .text_size(px(11.5))
                    .text_color(muted)
                    .child("latest"),
            );
        }
        heading = heading.child(div().flex_1()).children(
            release
                .date
                .clone()
                .map(|date| div().text_size(px(12.5)).text_color(muted).child(date)),
        );
        let changes = release.changes.iter().map(|change| {
            div()
                .flex()
                .gap(px(10.))
                .child(
                    div()
                        .flex_none()
                        .mt(px(7.))
                        .size(px(7.))
                        .rounded(px(999.))
                        .bg(hsla(chrome.accent)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(change.lead.clone()),
                        )
                        .when(!change.rest.is_empty(), |entry| {
                            entry.child(div().text_color(muted).child(change.rest.clone()))
                        }),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(heading)
            .children(changes)
            .into_any_element()
    }

    /// Update Now, or Restart Now when only the restart is due: asks first while
    /// sessions are busy (D5).
    fn begin_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !update::busy(&self.sessions).is_empty() {
            self.updates.phase = Phase::Asking;
        } else {
            self.updates.restart_now = true;
            self.proceed(window, cx);
        }
        cx.notify();
    }

    /// Installs when there is something to install, else restarts.
    fn proceed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match (self.updates.available, self.updates.installed.clone()) {
            (Some(version), Some(installed)) => self.install(version, installed, window, cx),
            _ => self.restart_for_update(window, cx),
        }
    }

    fn install(
        &mut self,
        version: Version,
        installed: Installed,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.updates.phase = Phase::Running(Step::Download);
        let (tx, mut rx) = unbounded::<Result<Option<Step>, String>>();
        std::thread::spawn(move || {
            let mut run = match update::start_install(version, &installed) {
                Ok(run) => run,
                Err(error) => {
                    let _ = tx.unbounded_send(Err(format!("{error:#}")));
                    return;
                }
            };
            let _ = tx.unbounded_send(Ok(Some(Step::Verify)));
            loop {
                let exited = run.child.try_wait();
                let output = update::read_log(&run.log);
                match exited {
                    Ok(Some(status)) if status.success() => {
                        run.clean_up();
                        let _ = tx.unbounded_send(Ok(None));
                        return;
                    }
                    Ok(Some(_)) => {
                        run.clean_up();
                        let _ = tx.unbounded_send(Err(update::failure_in(&output)));
                        return;
                    }
                    Ok(None) => {
                        if let Some(step) = update::step_in(&output) {
                            let _ = tx.unbounded_send(Ok(Some(step)));
                        }
                    }
                    Err(error) => {
                        let _ = tx.unbounded_send(Err(format!("the installer was lost: {error}")));
                        return;
                    }
                }
                std::thread::sleep(update::PROGRESS_POLL);
            }
        });
        cx.spawn_in(
            window,
            async move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                while let Some(news) = rx.next().await {
                    let alive = this.update_in(cx, |this, window, cx| {
                        match news {
                            Ok(Some(step)) => this.updates.phase = Phase::Running(step),
                            Ok(None) => this.installed_update(window, cx),
                            Err(reason) => this.updates.phase = Phase::Failed(reason),
                        }
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            },
        )
        .detach();
    }

    /// Wait: installs now when there is something to install, and restarts by itself once
    /// no session is busy (D5).
    fn wait_for_idle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.restart_now = false;
        match (self.updates.available, self.updates.installed.clone()) {
            (Some(version), Some(installed)) => self.install(version, installed, window, cx),
            _ => self.updates.phase = Phase::Waiting,
        }
    }

    /// The install finished: restart now, or wait for busy sessions (D5).
    fn installed_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.available = None;
        if self.updates.restart_now || update::busy(&self.sessions).is_empty() {
            self.restart_for_update(window, cx);
        } else {
            self.updates.phase = Phase::Waiting;
        }
    }

    /// While a restart waits, runs it once no session is busy.
    pub(super) fn restart_if_idle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.updates.phase == Phase::Waiting && update::busy(&self.sessions).is_empty() {
            self.restart_for_update(window, cx);
        }
    }

    /// Stops the host, which marks its running sessions, then reopens the app from the
    /// new copy once this one has quit (D6).
    fn restart_for_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.phase = Phase::Restarting;
        self.updates.open = true;
        let app = self
            .updates
            .installed
            .as_ref()
            .map(|installed| installed.app.clone())
            .or_else(this_app);
        let task = cx.background_executor().spawn(async move {
            let paths = HostPaths::standard().ok_or("cannot find the home folder")?;
            let app = app.ok_or("cannot tell where this app is")?;
            if let Err(error) = hq_client::stop_host(&paths) {
                return Err(format!("couldn't stop the host: {error:#}"));
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while std::os::unix::net::UnixStream::connect(&paths.control_socket).is_ok() {
                if std::time::Instant::now() >= deadline {
                    return Err("the host did not stop within 10 seconds".to_owned());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            update::relaunch_after_exit(&app).map_err(|error| format!("{error:#}"))
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| match result {
                Ok(()) => {
                    Self::save_window_now(window);
                    cx.quit();
                }
                Err(reason) => {
                    this.updates.phase = Phase::Failed(reason);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn close_update_window(&mut self, cx: &mut Context<Self>) {
        self.updates.open = false;
        self.updates.checked = None;
        if matches!(self.updates.phase, Phase::Asking | Phase::Failed(_)) {
            self.updates.phase = Phase::Offer;
        }
        cx.notify();
    }

    /// The update window, over everything else, while open.
    pub(super) fn render_update_window(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.updates.open {
            return None;
        }
        let chrome = self.theme.chrome;
        let muted = |text: String| div().text_color(hsla(chrome.muted)).child(text);
        let heading = |text: String| {
            div()
                .text_size(px(16.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(text)
        };
        let later = |label: &'static str| {
            Button::new("update-later")
                .w(px(BUTTON_WIDTH))
                .label(label)
                .on_click(cx.listener(|this, _, _, cx| this.close_update_window(cx)))
        };
        let running = Version::running();
        let mut body = div().flex().flex_col().gap(px(12.));
        let mut buttons = div().flex().justify_end().gap(px(8.));
        match (&self.updates.phase, self.updates.available) {
            (Phase::Offer, None) if self.updates.checked.is_some() || !self.updates.host_behind => {
                body = body.child(muted(self.updates.checked.clone().unwrap_or_default()));
                buttons = buttons.child(later("OK"));
            }
            (Phase::Offer, None) => {
                body = body
                    .child(heading(format!("Restart to finish updating to {running}")))
                    .child(muted(
                        "The background host still runs the version before. Restarting starts your running sessions again on their conversations."
                            .to_owned(),
                    ));
                buttons = buttons.child(later("Later")).child(
                    Button::new("update-restart")
                        .w(px(BUTTON_WIDTH))
                        .primary()
                        .label("Restart Now")
                        .on_click(cx.listener(|this, _, window, cx| this.begin_update(window, cx))),
                );
            }
            (Phase::Offer, Some(version)) => {
                body = body.child(heading(format!("Headquarter {version} is available")));
                buttons = buttons.child(later("Later"));
                buttons = match &self.updates.installed {
                    Some(_) => buttons.child(
                        Button::new("update-now")
                            .w(px(BUTTON_WIDTH))
                            .primary()
                            .label("Update Now")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.begin_update(window, cx)),
                            ),
                    ),
                    None => {
                        // Not installed by the installer: the release page only (D2).
                        let url = update::release_page(version);
                        body = body.child(muted(
                            "This copy was not installed by the Headquarter installer, so it is not updated in place."
                                .to_owned(),
                        ));
                        buttons.child(
                            Button::new("update-page")
                                .w(px(BUTTON_WIDTH))
                                .primary()
                                .label("Open Release Page")
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        )
                    }
                };
            }
            (Phase::Asking, _) => {
                let busy: Vec<String> = update::busy(&self.sessions)
                    .iter()
                    .map(|session| hq_protocol::sidebar::display_title(session))
                    .collect();
                let count = match busy.len() {
                    1 => "1 session is busy".to_owned(),
                    count => format!("{count} sessions are busy"),
                };
                body = body
                    .child(heading(format!("{count}: {}", busy.join(", "))))
                    .child(muted(
                        "Restarting interrupts them. Wait restarts by itself as soon as none is busy."
                            .to_owned(),
                    ));
                buttons = buttons
                    .child(
                        Button::new("update-wait")
                            .w(px(BUTTON_WIDTH))
                            .label("Wait")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.wait_for_idle(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("update-restart-now")
                            .w(px(BUTTON_WIDTH))
                            .danger()
                            .label("Restart Now")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.updates.restart_now = true;
                                this.proceed(window, cx);
                                cx.notify();
                            })),
                    );
            }
            (Phase::Running(step), version) => {
                body = body
                    .child(heading(match version {
                        Some(version) => format!("Updating to {version}"),
                        None => "Updating".to_owned(),
                    }))
                    .child(self.render_steps(*step));
            }
            (Phase::Waiting, _) => {
                let count = update::busy(&self.sessions).len();
                body = body
                    .child(heading("Update installed".to_owned()))
                    .child(self.render_steps(Step::Restart))
                    .child(muted(format!(
                        "Waiting for {count} busy session{} before restarting.",
                        if count == 1 { "" } else { "s" }
                    )));
                buttons = buttons.child(later("Close")).child(
                    Button::new("update-restart-waiting")
                        .w(px(BUTTON_WIDTH))
                        .danger()
                        .label("Restart Now")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.restart_for_update(window, cx);
                            cx.notify();
                        })),
                );
            }
            (Phase::Restarting, _) => {
                body = body
                    .child(heading("Restarting…".to_owned()))
                    .child(self.render_steps(Step::Restart));
            }
            (Phase::Failed(reason), _) => {
                body = body
                    .child(heading("The update did not finish".to_owned()))
                    .child(div().text_color(hsla(chrome.amber)).child(reason.clone()))
                    .child(muted("The installed version is unchanged.".to_owned()));
                buttons = buttons.child(later("Close"));
            }
        }
        Some(
            div()
                .id("update-backdrop")
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(overlay(chrome))
                .child(
                    div()
                        .id("update-window")
                        .w(px(400.))
                        .flex()
                        .flex_col()
                        .gap(px(16.))
                        .p(px(20.))
                        .rounded(px(14.))
                        .border_1()
                        .border_color(hsla(chrome.sidebar_line))
                        .bg(hsla(chrome.sidebar))
                        .shadow_lg()
                        .text_size(px(14.))
                        .text_color(hsla(chrome.text))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(body)
                        .child(buttons),
                )
                .into_any_element(),
        )
    }

    /// Download, verify, install and restart, each done, running or still to come.
    fn render_steps(&self, current: Step) -> impl IntoElement {
        let chrome = self.theme.chrome;
        let reached = Step::ALL
            .iter()
            .position(|step| *step == current)
            .unwrap_or(0);
        div()
            .flex()
            .gap(px(14.))
            .children(Step::ALL.iter().enumerate().map(|(index, step)| {
                let (mark, color) = if index < reached {
                    ("✓", chrome.text)
                } else if index == reached {
                    ("●", chrome.accent)
                } else {
                    ("○", chrome.muted)
                };
                div()
                    .flex()
                    .gap(px(5.))
                    .text_color(hsla(color))
                    .child(mark)
                    .child(step.label())
            }))
    }
}

/// What to open to start this app again when the installer did not install it: the
/// bundle holding the executable on macOS, else the executable.
fn this_app() -> Option<std::path::PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let bundle = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(std::path::Path::to_path_buf);
    Some(bundle.unwrap_or(executable))
}
