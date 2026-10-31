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
    KeyDownEvent, MouseButton, ParentElement, ScrollHandle, StatefulInteractiveElement, Styled,
    Task, WeakEntity, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::scroll::Scrollbar;
use hq_client::HostPaths;

use super::{Pane, Workspace, overlay};
use crate::terminal::view::hsla;
use crate::update::{self, Installed, LogLine, ReleaseNotes, Step, Version};

/// Buttons shown together share this width, so a pair reads as one choice (owner,
/// 2026-10-26).
const BUTTON_WIDTH: f32 = 112.0;
/// The pair's width when one of them says "Open Release Page", which does not fit 112.
const WIDE_BUTTON_WIDTH: f32 = 156.0;
/// The What's new page is one column of this readable width, centred in the pane
/// (refresh-pane-and-status D4).
const NOTES_WIDTH: f32 = 760.0;
/// The column's padding on each side.
const COLUMN_PADDING: f32 = 36.0;
const RELEASES_PAGE: &str = "https://github.com/alfonsusenrico/headquarter/releases";
/// How close to the end of the log, in pixels, still counts as at the end.
const LOG_END_SLACK: f32 = 2.0;

/// What the update has done so far, for the log under the steps (fix-update-progress):
/// the app's own lines around the installer's notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct UpdateLog {
    before: Vec<LogLine>,
    installer: Vec<LogLine>,
    after: Vec<LogLine>,
}

impl UpdateLog {
    fn lines(&self) -> impl Iterator<Item = &LogLine> {
        self.before.iter().chain(&self.installer).chain(&self.after)
    }
}

/// What the installer's thread reports.
enum News {
    /// The app fetched the installer and it matched this digest.
    Checked(String),
    /// The installer's step and notes so far.
    Progress(Step, Vec<LogLine>),
    Finished,
    Failed(String),
}

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
    /// Installed; the restart waits for the owner's Update Now (update-restart-on-demand D1).
    Installed,
    Restarting,
    Failed(String),
}

/// The sidebar button's label, if it shows: "Update Now" once installed and waiting for
/// the owner (update-restart-on-demand D2).
fn pill_label(phase: &Phase, available: Option<Version>, host_behind: bool) -> Option<String> {
    match (phase, available) {
        (Phase::Installed, _) => Some("Update Now".to_owned()),
        (_, Some(version)) => Some(format!("Update Headquarter v{version}")),
        (_, None) if host_behind => Some("Restart to finish update".to_owned()),
        _ => None,
    }
}

/// Whether Escape closes the update window: in the phases that offer OK, Later or Close
/// (fix-update-window-keys D2).
fn escape_closes(phase: &Phase) -> bool {
    matches!(phase, Phase::Offer | Phase::Installed | Phase::Failed(_))
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
    /// The installer's copy and the app to reopen, when the installer installed this one.
    installed: Option<Installed>,
    /// Restart as soon as the install ends, without waiting for busy sessions.
    restart_now: bool,
    /// The update is installed and only the owner's restart is left, so closing the
    /// confirmation returns to `Installed` (update-restart-on-demand D2).
    installed_waiting: bool,
    log: UpdateLog,
    /// Where the log box is scrolled; it follows the newest line while it shows the end
    /// (fix-update-log-scroll D2).
    log_scroll: ScrollHandle,
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
            installed: update::installed_copy(),
            restart_now: false,
            installed_waiting: false,
            log: UpdateLog::default(),
            log_scroll: ScrollHandle::new(),
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
                        if manual {
                            this.show_whats_new(window, cx);
                        }
                    }
                    // A quiet notice, so the pane stays uncovered (add-quiet-notice D3).
                    Ok(_) if manual => {
                        let text = format!("Headquarter {running} is the latest version.");
                        this.show_notice(text, false, window, cx);
                    }
                    Err(error) if manual => {
                        let text = format!("Couldn't check for updates: {error:#}");
                        this.show_notice(text, true, window, cx);
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
        let label = pill_label(
            &self.updates.phase,
            self.updates.available,
            self.updates.host_behind,
        )?;
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
                    } else if this.updates.phase == Phase::Installed {
                        this.restart_when_asked(window, cx);
                    } else {
                        this.open_update_window(window, cx);
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
        // The page takes the keyboard in the window's place.
        self.update_return = None;
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
        // Later matches the other button's width, so the pair reads as one choice.
        let pair_width = match &self.updates.installed {
            Some(_) => BUTTON_WIDTH,
            None => WIDE_BUTTON_WIDTH,
        };
        let primary = match &self.updates.installed {
            Some(_) => Button::new("whats-new-update")
                .primary()
                .w(px(pair_width))
                .label("Update Now")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_update_window(window, cx);
                    this.begin_update(window, cx);
                })),
            None => {
                // Not installed by the installer: the release page only (add-in-app-update D2).
                let url = update::release_page(latest);
                Button::new("whats-new-page")
                    .primary()
                    .w(px(pair_width))
                    .label("Open Release Page")
                    .on_click(move |_, _, cx| cx.open_url(&url))
            }
        };
        let chip = |text: String, new: bool| {
            let mut tint = hsla(chrome.accent);
            tint.a = 0.16;
            // The track colour, faint, shows on both themes' backgrounds.
            let mut plain = hsla(chrome.track);
            plain.a = 0.45;
            div()
                .flex_none()
                .px(px(9.))
                .py(px(2.))
                .rounded(px(999.))
                .text_size(px(12.5))
                .bg(if new { tint } else { plain })
                .text_color(if new { hsla(chrome.accent) } else { muted })
                .child(text)
        };
        let top = column(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(muted)
                        .child("Update available"),
                )
                .child(
                    div()
                        .text_size(px(22.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("What's new in Headquarter"),
                )
                .child(
                    div()
                        .mt(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(chip(format!("v{} installed", Version::running()), false))
                        .child(div().text_color(muted).child("→"))
                        .child(chip(format!("v{latest}"), true)),
                ),
        )
        .flex_none()
        .pt(px(30.))
        .pb(px(6.));
        let what_happens = match &self.updates.installed {
            Some(_) => "Restarts Headquarter. Your sessions resume on their conversations.",
            None => "This copy updates from the release page.",
        };
        // The buttons stay in view while the notes scroll; the sentence wraps above them
        // when the pane is too narrow for both.
        let bar = div()
            .flex_none()
            .border_t_1()
            .border_color(hsla(chrome.sidebar_line))
            .bg(background)
            .child(
                column(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(220.))
                                .text_size(px(13.))
                                .text_color(muted)
                                .child(what_happens),
                        )
                        .child(
                            Button::new("whats-new-later")
                                .w(px(pair_width))
                                .label("Later")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_whats_new(window, cx)
                                })),
                        )
                        .child(primary),
                )
                .py(px(12.)),
            );
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
        let mut content = div().flex().flex_col().gap(px(26.)).pt(px(20.)).pb(px(32.));
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
                    .child(column(content)),
            )
            .child(bar)
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
        self.updates.log = UpdateLog {
            before: vec![LogLine::doing(format!(
                "Downloading the installer for v{version}"
            ))],
            ..UpdateLog::default()
        };
        self.updates.log_scroll = ScrollHandle::new();
        let (tx, mut rx) = unbounded::<News>();
        std::thread::spawn(move || {
            let mut run = match update::start_install(version, &installed) {
                Ok(run) => run,
                Err(error) => {
                    let _ = tx.unbounded_send(News::Failed(format!("{error:#}")));
                    return;
                }
            };
            let _ = tx.unbounded_send(News::Checked(run.digest.clone()));
            loop {
                let exited = run.child.try_wait();
                let output = update::read_log(&run.log);
                let progress =
                    News::Progress(update::shown_step(&output), update::notes_in(&output));
                let _ = tx.unbounded_send(progress);
                match exited {
                    Ok(Some(status)) if status.success() => {
                        run.clean_up();
                        let _ = tx.unbounded_send(News::Finished);
                        return;
                    }
                    Ok(Some(_)) => {
                        run.clean_up();
                        let _ = tx.unbounded_send(News::Failed(update::failure_in(&output)));
                        return;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        let _ = tx.unbounded_send(News::Failed(format!(
                            "the installer was lost: {error}"
                        )));
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
                            News::Checked(digest) => {
                                this.change_log(|log| {
                                    log.before.push(LogLine::done(format!(
                                        "The installer matches GitHub's checksum (sha256 {})",
                                        update::short_digest(&digest)
                                    )));
                                });
                            }
                            News::Progress(step, notes) => {
                                this.updates.phase = Phase::Running(step);
                                this.change_log(|log| log.installer = notes);
                            }
                            News::Finished => this.installed_update(window, cx),
                            News::Failed(reason) => this.updates.phase = Phase::Failed(reason),
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

    /// Wait: installs now when there is something to install, and leaves the restart to
    /// the owner (update-restart-on-demand D1).
    fn install_and_wait(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.restart_now = false;
        match (self.updates.available, self.updates.installed.clone()) {
            (Some(version), Some(installed)) => self.install(version, installed, window, cx),
            _ => {
                self.updates.phase = Phase::Installed;
                self.updates.installed_waiting = true;
            }
        }
    }

    /// The install finished: restart when the owner asked for it with nothing busy,
    /// otherwise wait for their Update Now; never by itself (D1).
    fn installed_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.available = None;
        if self.updates.restart_now {
            self.restart_for_update(window, cx);
        } else {
            self.updates.phase = Phase::Installed;
            self.updates.installed_waiting = true;
        }
    }

    /// The sidebar's Update Now for an installed update: restarts at once when nothing is
    /// busy, otherwise asks first (D2).
    fn restart_when_asked(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if update::busy(&self.sessions).is_empty() {
            self.restart_for_update(window, cx);
        } else {
            self.updates.phase = Phase::Asking;
            self.open_update_window(window, cx);
        }
    }

    /// Stops the host, which marks its running sessions, then reopens the app from the
    /// new copy once this one has quit (D6).
    fn restart_for_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.phase = Phase::Restarting;
        self.open_update_window(window, cx);
        self.change_log(|log| {
            log.after.push(LogLine::doing(
                "Stopping the background host and its sessions",
            ));
        });
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

    /// Opens the update window with the keyboard, so nothing typed reaches the session
    /// behind it (fix-update-window-keys D1).
    fn open_update_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.updates.open {
            self.update_return = window.focused(cx);
        }
        self.updates.open = true;
        window.focus(&self.update_focus, cx);
        cx.notify();
    }

    /// Closes the update window and gives the keyboard back to what had it.
    fn close_update_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updates.open = false;
        if matches!(self.updates.phase, Phase::Asking | Phase::Failed(_)) {
            self.updates.phase = if self.updates.installed_waiting {
                Phase::Installed
            } else {
                Phase::Offer
            };
        }
        if let Some(focus) = self.update_return.take() {
            window.focus(&focus, cx);
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
                .on_click(cx.listener(|this, _, window, cx| this.close_update_window(window, cx)))
        };
        let running = Version::running();
        let mut body = div().flex().flex_col().gap(px(12.));
        let mut buttons = div().flex().justify_end().gap(px(8.));
        match (&self.updates.phase, self.updates.available) {
            (Phase::Offer, None) if !self.updates.host_behind => {
                body = body.child(muted(format!(
                    "Headquarter {running} is the latest version."
                )));
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
                // Before an install, Wait installs now; once installed, Later keeps it
                // installed. Either way the restart is the owner's (D2).
                let installing = !self.updates.installed_waiting;
                body = body
                    .child(heading(format!("{count}: {}", busy.join(", "))))
                    .child(muted(if installing {
                        "Restarting interrupts them. Wait installs it now; restart with Update Now in the sidebar when you're ready."
                            .to_owned()
                    } else {
                        "Restarting interrupts them.".to_owned()
                    }));
                buttons = buttons
                    .child(
                        Button::new("update-wait")
                            .w(px(BUTTON_WIDTH))
                            .label(if installing { "Wait" } else { "Later" })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if installing {
                                    this.install_and_wait(window, cx);
                                } else {
                                    this.close_update_window(window, cx);
                                }
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
                        Some(version) => format!("Updating to v{version}"),
                        None => "Updating".to_owned(),
                    }))
                    .child(self.render_steps(*step))
                    .children(self.render_log());
            }
            (Phase::Installed, _) => {
                body = body
                    .child(heading("Update installed".to_owned()))
                    .child(self.render_steps(Step::Restart))
                    .children(self.render_log())
                    .child(muted(
                        "Restart with Update Now in the sidebar when you're ready.".to_owned(),
                    ));
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
                    .child(self.render_steps(Step::Restart))
                    .children(self.render_log());
            }
            (Phase::Failed(reason), _) => {
                body = body
                    .child(heading("The update did not finish".to_owned()))
                    .child(div().text_color(hsla(chrome.amber)).child(reason.clone()))
                    .children(self.render_log())
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
                        .track_focus(&self.update_focus)
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                if escape_closes(&this.updates.phase) {
                                    this.close_update_window(window, cx);
                                }
                            }
                        }))
                        .w(px(480.))
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

    /// Changes the log, and keeps the newest line in view when the box showed the end
    /// before (fix-update-log-scroll D2).
    fn change_log(&mut self, change: impl FnOnce(&mut UpdateLog)) {
        let scroll = &self.updates.log_scroll;
        let follow = at_log_end(
            f32::from(scroll.offset().y),
            f32::from(scroll.max_offset().y),
        );
        change(&mut self.updates.log);
        if follow {
            self.updates.log_scroll.scroll_to_bottom();
        }
    }

    /// Every line of the update so far, from the top, newest brightest; it scrolls once
    /// it fills the box. `None` before anything happened (fix-update-log-scroll D1).
    fn render_log(&self) -> Option<AnyElement> {
        let chrome = self.theme.chrome;
        let lines: Vec<&LogLine> = self.updates.log.lines().collect();
        let last = lines.len().checked_sub(1)?;
        let rows = lines.into_iter().enumerate().map(|(index, line)| {
            let color = if index == last {
                chrome.text
            } else {
                chrome.muted
            };
            div()
                .flex()
                .gap(px(6.))
                .text_color(hsla(color))
                .when(line.done, |row| {
                    row.child(div().flex_none().text_color(hsla(chrome.accent)).child("✓"))
                })
                .child(div().min_w_0().child(line.text.clone()))
        });
        Some(
            div()
                .relative()
                .h(px(132.))
                .rounded(px(8.))
                .border_1()
                .border_color(hsla(chrome.sidebar_line))
                .bg(super::translucent(self.theme.palette.background, 1.0))
                .child(
                    div()
                        .id("update-log")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.updates.log_scroll)
                        .flex()
                        .flex_col()
                        .px(px(12.))
                        .py(px(10.))
                        .font_family(self.options.font_family.clone())
                        .text_size(px(12.))
                        .line_height(px(19.))
                        .children(rows),
                )
                .child(Scrollbar::vertical(&self.updates.log_scroll))
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

/// The What's new page's column: `content` at full width up to a readable measure,
/// centred in the pane (refresh-pane-and-status D4).
fn column(content: impl IntoElement) -> gpui::Div {
    div().w_full().flex().justify_center().child(
        div()
            .w_full()
            .max_w(px(NOTES_WIDTH + 2.0 * COLUMN_PADDING))
            .px(px(COLUMN_PADDING))
            .child(content),
    )
}

/// Whether the log box shows its end: its last line is in view, or it does not scroll
/// yet. `offset_y` grows more negative as the box scrolls down; `max_y` is how far it can.
fn at_log_end(offset_y: f32, max_y: f32) -> bool {
    -offset_y >= max_y - LOG_END_SLACK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_follows_only_from_its_end() {
        assert!(at_log_end(0.0, 0.0), "not scrolling yet");
        assert!(at_log_end(-80.0, 80.0), "at the end");
        assert!(at_log_end(-79.0, 80.0), "within the slack");
        assert!(!at_log_end(-30.0, 80.0), "scrolled back");
        assert!(!at_log_end(0.0, 80.0), "scrolled back to the top");
    }

    #[test]
    fn the_pill_offers_update_now_once_installed() {
        let version = Version::parse("0.1.30");
        assert_eq!(
            pill_label(&Phase::Installed, None, false).as_deref(),
            Some("Update Now"),
            "installed, waiting for the owner"
        );
        assert_eq!(
            pill_label(&Phase::Offer, version, false).as_deref(),
            Some("Update Headquarter v0.1.30")
        );
        assert_eq!(
            pill_label(&Phase::Offer, None, true).as_deref(),
            Some("Restart to finish update")
        );
        assert_eq!(pill_label(&Phase::Offer, None, false), None);
    }

    #[test]
    fn escape_closes_only_what_offers_a_way_out() {
        assert!(escape_closes(&Phase::Offer), "OK, Later");
        assert!(escape_closes(&Phase::Installed), "Close");
        assert!(escape_closes(&Phase::Failed("no network".into())), "Close");
        assert!(!escape_closes(&Phase::Asking), "Wait or Restart Now only");
        assert!(!escape_closes(&Phase::Running(Step::Download)));
        assert!(!escape_closes(&Phase::Restarting));
    }
}
