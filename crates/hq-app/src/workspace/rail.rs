//! The sidebar's divider and its icon rail
//! (`openspec/changes/add-sidebar-rail/design.md`, D1 to D4).

use gpui::{
    AnyElement, AppContext, ClickEvent, Context, CursorStyle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, px,
};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Icon, IconName, Sizable};

use super::{Pane, Workspace, groups};
use crate::app_state::{self, SidebarState};
use crate::sidebar::model::{self, Item, RailItem, Row};
use crate::terminal::view::hsla;

/// How long the sidebar takes to settle on its width after a drag or a toggle.
const SETTLE_FOR: std::time::Duration = std::time::Duration::from_millis(160);
const SETTLE_FRAME: std::time::Duration = std::time::Duration::from_millis(8);

/// The divider being dragged; draws nothing under the pointer.
pub struct DividerDrag;

impl Render for DividerDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl Workspace {
    // ---- resizing -----------------------------------------------------------------

    /// The 6 px handle over the sidebar's right edge: drag to resize, double-click to
    /// switch between the full sidebar and the rail.
    pub(super) fn render_divider(&self, cx: &mut Context<Self>) -> AnyElement {
        let edge = self.sidebar_width();
        div()
            .id("sidebar-divider")
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(edge - 3.))
            .w(px(6.))
            .cursor(CursorStyle::ResizeLeftRight)
            .on_drag(DividerDrag, |_, _, _, cx| cx.new(|_| DividerDrag))
            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                if event.click_count() >= 2 {
                    this.toggle_rail(cx);
                }
            }))
            .into_any_element()
    }

    /// The sidebar's width on screen now.
    pub(super) fn sidebar_width(&self) -> f32 {
        self.live_width.unwrap_or(self.sidebar.shown_width())
    }

    /// Whether the sidebar is drawn as the rail now.
    pub(super) fn rail_shown(&self) -> bool {
        app_state::is_rail(self.sidebar_width())
    }

    /// The divider is at `x`: the sidebar follows it down to the rail's width, and the
    /// terminal keeps its size until the drag ends.
    pub(super) fn drag_divider(&mut self, x: f32, cx: &mut Context<Self>) {
        if !self.resizing {
            self.resizing = true;
            self.width_motion += 1;
            self.hold_terminal(true, cx);
        }
        let live = app_state::live_width(x);
        if self.live_width != Some(live) {
            self.live_width = Some(live);
            self.sidebar = self.sidebar.dragged_to(x);
            cx.notify();
        }
    }

    /// The drag ended, dropped anywhere or not: the sidebar settles on its kept width
    /// (the rail when released below the full sidebar's minimum), the terminal takes
    /// its new size and the state is kept.
    pub(super) fn finish_resize(&mut self, cx: &mut Context<Self>) {
        if !self.resizing {
            return;
        }
        self.resizing = false;
        self.keep_sidebar();
        self.settle(cx);
    }

    pub(super) fn toggle_rail(&mut self, cx: &mut Context<Self>) {
        if self.resizing {
            return;
        }
        let from = self.sidebar_width();
        self.sidebar = self.sidebar.toggled();
        self.keep_sidebar();
        self.live_width = Some(from);
        self.hold_terminal(true, cx);
        self.settle(cx);
    }

    /// Moves the shown width to the kept state's width over a short ease-out, or at
    /// once under Reduce motion; the terminal is resized once, at the end.
    fn settle(&mut self, cx: &mut Context<Self>) {
        self.width_motion += 1;
        let motion = self.width_motion;
        let target = self.sidebar.shown_width();
        let from = self.sidebar_width();
        if crate::settings::reduce_motion() || (from - target).abs() < 1. {
            self.live_width = None;
            self.hold_terminal(false, cx);
            cx.notify();
            return;
        }
        cx.spawn(async move |this, cx| {
            let started = std::time::Instant::now();
            loop {
                cx.background_executor().timer(SETTLE_FRAME).await;
                let progress = (started.elapsed().as_secs_f32() / SETTLE_FOR.as_secs_f32()).min(1.);
                let eased = 1. - (1. - progress).powi(3);
                let going = this.update(cx, |this, cx| {
                    if this.width_motion != motion {
                        return false;
                    }
                    if progress >= 1. {
                        this.live_width = None;
                        this.hold_terminal(false, cx);
                    } else {
                        this.live_width = Some(from + (target - from) * eased);
                    }
                    cx.notify();
                    progress < 1.
                });
                if !matches!(going, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn hold_terminal(&self, held: bool, cx: &mut Context<Self>) {
        if let Pane::Terminal { view, .. } = &self.pane {
            view.update(cx, |view, cx| view.hold_size(held, cx));
        }
    }

    fn keep_sidebar(&self) {
        let Some(path) = app_state::path() else {
            return;
        };
        if let Err(error) = app_state::save_sidebar(&path, self.sidebar) {
            eprintln!(
                "headquarter-app: the sidebar's width was not kept in {}: {error}",
                path.display()
            );
        }
    }

    // ---- the rail -----------------------------------------------------------------

    /// The rail's column: search, the sessions as marks with their status, chips for
    /// collapsed groups, and lines between sections.
    pub(super) fn render_rail(&self, items: &[Item], cx: &mut Context<Self>) -> Vec<AnyElement> {
        let chrome = self.theme.chrome;
        let mut hover = hsla(chrome.selected);
        hover.a = 0.55;
        let mut out = vec![
            div()
                .id("rail-search")
                .h(px(36.))
                .mb(px(4.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .text_color(hsla(chrome.muted))
                .child(Icon::new(IconName::Search).small())
                .tooltip(|window, cx| {
                    Tooltip::new(if crate::shortcuts::MAC {
                        "Add or find sessions (⌘O)"
                    } else {
                        "Add or find sessions (Ctrl-Shift-O)"
                    })
                    .build(window, cx)
                })
                .on_click(cx.listener(|this, _, window, cx| this.show_open_sessions(window, cx)))
                .into_any_element(),
        ];
        for item in model::rail_items(items) {
            out.push(match item {
                RailItem::Session(row) => self.render_rail_session(&row, cx),
                RailItem::Chip {
                    id,
                    name,
                    initials,
                    total,
                    waiting,
                    color,
                } => self.render_rail_chip(id, name, initials, total, waiting, color, cx),
                RailItem::Line => div()
                    .my(px(6.))
                    .mx(px(8.))
                    .h(px(1.))
                    .flex_none()
                    .bg(hsla(chrome.sidebar_line))
                    .into_any_element(),
            });
        }
        out
    }

    /// A session in the rail: only its agent's mark, centred. A session needing the
    /// owner gets the amber ring (and says so in its tooltip); an inactive one is
    /// dimmed. Other states are left to the full sidebar (owner review, 2026-10-20).
    fn render_rail_session(&self, row: &Row, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let active = self.shown_id() == Some(row.id.as_str());
        let needs_you = row.glyph == model::Glyph::NeedsYou;
        let mut hover = hsla(chrome.selected);
        hover.a = 0.55;
        let mut tooltip = format!("{}\n{}", row.title, row.path);
        if needs_you {
            tooltip.push_str("\nNeeds you");
        }
        let open_id = row.id.clone();
        let mark = div().child(self.marks.element(row.agent, 20., chrome.muted));
        let mut cell = div()
            .id(SharedString::from(format!(
                "rail-{}-{}",
                row.section.key(),
                row.id
            )))
            .relative()
            .h(px(40.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .child(mark)
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .on_click(cx.listener(move |this, _, window, cx| this.open(&open_id, window, cx)));
        if needs_you {
            cell = cell.border_2().border_color(hsla(chrome.glyph_amber));
        }
        if active {
            cell = cell.bg(hsla(chrome.selected)).child(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px(10.))
                    .bottom(px(10.))
                    .w(px(3.))
                    .rounded(px(2.))
                    .bg(hsla(chrome.text)),
            );
        }
        cell.into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_rail_chip(
        &self,
        id: i64,
        name: String,
        initials: String,
        total: usize,
        waiting: bool,
        color: hq_protocol::GroupColor,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let mut hover = hsla(chrome.selected);
        hover.a = 0.55;
        let mut tooltip = format!("{name}, {}", model::session_count(total));
        if waiting {
            let needing = self
                .sessions
                .iter()
                .filter(|session| {
                    session.in_sidebar
                        && session.group == Some(id)
                        && hq_protocol::sidebar::needs_you(session)
                })
                .count();
            tooltip.push_str(&format!(", {}", model::needing_note(needing)));
        }
        div()
            .id(SharedString::from(format!("rail-chip-{id}")))
            .h(px(32.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(8.))
            .border_1()
            // Outlined in the group's colour (add-group-colors D5).
            .border_color(hsla(self.theme.group_color(color)))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .text_size(px(11.))
            .children(
                waiting
                    .then(|| groups::waiting_dot(self.motion_phase(groups::WAITING_CYCLE), chrome)),
            )
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(hsla(chrome.group))
                    .child(initials),
            )
            .child(
                div()
                    .text_color(hsla(chrome.muted))
                    .child(total.to_string()),
            )
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_group_collapsed(id, false, window, cx);
            }))
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .into_any_element()
    }

    /// The usage ring of the agent with the highest share used, for the rail's foot.
    pub(super) fn render_rail_usage(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let now = hq_signals::now_ms() / 1000;
        let busiest = self.usage.iter().max_by(|a, b| {
            let share = |usage| hq_protocol::usage::fill(usage, now).unwrap_or(-1.0);
            share(a).total_cmp(&share(b))
        })?;
        Some(
            div()
                .flex()
                .justify_center()
                .py(px(8.))
                .border_t_1()
                .border_color(hsla(self.theme.chrome.sidebar_line))
                .child(self.usage_ring(busiest, now, cx))
                .into_any_element(),
        )
    }
}

/// The kept sidebar state, or the defaults.
pub(super) fn kept_sidebar() -> SidebarState {
    app_state::path()
        .map(|path| app_state::load_sidebar(&path))
        .unwrap_or_default()
}
