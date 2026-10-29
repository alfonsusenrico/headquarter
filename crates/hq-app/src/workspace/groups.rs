//! The owner's groups in the sidebar: headers, naming in place, the group menu, the
//! delete confirmation, and dragging sessions and groups
//! (`openspec/changes/add-session-groups/design.md`, D2 to D6).

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, AppContext, Context, DragMoveEvent, Entity, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::popover::Popover;
use gpui_component::tooltip::Tooltip;
use hq_protocol::{GroupColor, GroupInfo};
use hq_signals::event::Agent;

use super::Workspace;
use crate::marks::Marks;
use crate::sidebar::model::{self, Item, Target};
use crate::terminal::view::hsla;
use crate::theme::Chrome;

/// How long a drop target takes to light up.
const LIGHT_UP: Duration = Duration::from_millis(120);
/// One fade of a waiting group's amber dot, out and back.
pub(super) const WAITING_CYCLE: Duration = Duration::from_secs(2);

/// A session row being dragged; also the chip drawn under the pointer.
#[derive(Clone)]
pub struct DraggedSession {
    pub id: String,
    pub title: String,
    pub agent: Agent,
    pub chrome: Chrome,
    pub marks: Marks,
}

impl Render for DraggedSession {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .max_w(px(220.))
            .px(px(10.))
            .py(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(hsla(chrome.track))
            .bg(hsla(chrome.selected))
            .shadow_md()
            .text_size(px(13.))
            .text_color(hsla(chrome.text))
            .child(self.marks.element(self.agent, 18., chrome.muted))
            .child(div().truncate().child(self.title.clone()))
    }
}

/// A group header being dragged to reorder the groups.
#[derive(Clone)]
pub struct DraggedGroup {
    id: i64,
    name: String,
    chrome: Chrome,
}

impl Render for DraggedGroup {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = self.chrome;
        div()
            .px(px(10.))
            .py(px(4.))
            .rounded(px(6.))
            .bg(hsla(chrome.selected))
            .shadow_md()
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(hsla(chrome.text))
            .child(self.name.clone())
    }
}

/// What a name field names.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Naming {
    Group(i64),
    /// A group that does not exist yet: it is created, holding `session` when given,
    /// once the name is saved, and Escape leaves no group behind.
    New {
        session: Option<String>,
    },
}

/// A group's name being edited in place.
pub struct GroupRename {
    naming: Naming,
    input: Entity<InputState>,
    /// Saved when the field is left blank.
    placeholder: String,
    _events: Subscription,
}

impl Workspace {
    // ---- changes ------------------------------------------------------------------

    /// Opens the name field of a new group, "New Group #n", after the other groups;
    /// with `session`, the group will hold it. Nothing is created until the name is
    /// saved.
    pub(super) fn new_group(
        &mut self,
        session: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(rename) = &self.group_rename
            && matches!(rename.naming, Naming::New { .. })
        {
            window.focus(&rename.input.focus_handle(cx), cx);
            return;
        }
        let name =
            hq_protocol::default_group_name(self.groups.iter().map(|group| group.name.as_str()));
        self.open_name_field(Naming::New { session }, name, window, cx);
    }

    pub(super) fn start_group_rename(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(name) = self.group(id).map(|group| group.name.clone()) {
            self.open_name_field(Naming::Group(id), name, window, cx);
        }
    }

    fn open_name_field(
        &mut self,
        naming: Naming,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(name.clone())
                .default_value(name.clone());
            state.select_all(window, cx);
            state
        });
        // Enter or a click elsewhere saves; Escape (handled by the field's wrapper)
        // cancels.
        let events =
            cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } | InputEvent::Blur => {
                        this.finish_group_rename(true, window, cx);
                    }
                    _ => {}
                },
            );
        window.focus(&input.focus_handle(cx), cx);
        self.group_rename = Some(GroupRename {
            naming,
            input,
            placeholder: name,
            _events: events,
        });
        cx.notify();
    }

    /// Ends naming in place: saved, a blank name takes the placeholder; cancelled, a
    /// group keeps its name and a new group is not created.
    fn finish_group_rename(&mut self, save: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rename) = self.group_rename.take() else {
            return;
        };
        window.focus(&self.sidebar_focus, cx);
        cx.notify();
        if !save {
            return;
        }
        let typed = rename.input.read(cx).value().trim().to_owned();
        let name = if typed.is_empty() {
            rename.placeholder
        } else {
            typed
        };
        match rename.naming {
            Naming::Group(id) => {
                if self.group(id).is_none_or(|group| group.name == name) {
                    return;
                }
                if let Some(group) = self.groups.iter_mut().find(|group| group.id == id) {
                    group.name = name.clone();
                }
                self.host_call(
                    window,
                    cx,
                    move |paths| hq_client::rename_group(paths, id, &name),
                    |_, (), _, _| {},
                );
            }
            Naming::New { session } => {
                let member = session.clone();
                self.host_call(
                    window,
                    cx,
                    move |paths| hq_client::new_group(paths, Some(&name), session.as_deref()),
                    move |this, group, _, cx| {
                        if let Some(member) = &member
                            && let Some(known) =
                                this.sessions.iter_mut().find(|known| known.id == *member)
                        {
                            known.group = Some(group.id);
                        }
                        if !this.groups.iter().any(|known| known.id == group.id) {
                            this.groups.push(group);
                        }
                        cx.notify();
                    },
                );
            }
        }
    }

    /// Picks a group's colour, or `None` for automatic (add-group-colors D3).
    pub(super) fn set_group_color(
        &mut self,
        id: i64,
        color: Option<GroupColor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Shown at once; the host's answer confirms it.
        if let Some(group) = self.groups.iter_mut().find(|group| group.id == id) {
            group.color = color.map(|color| color.name().to_owned());
        }
        self.color_picker = None;
        cx.notify();
        self.host_call(
            window,
            cx,
            move |paths| hq_client::set_group_color(paths, id, color),
            |_, (), _, _| {},
        );
    }

    /// The colour dot that starts a group's header and opens its colour picker: the
    /// eight swatches, each named, and "Automatic"; the current choice is ticked.
    fn render_color_dot(
        &self,
        id: i64,
        (color, picked): (GroupColor, Option<GroupColor>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let open = self.color_picker == Some(id);
        let swatch = |color: GroupColor, size: f32| {
            div()
                .size(px(size))
                .flex_none()
                .rounded(px(3.))
                .bg(hsla(self.theme.group_color(color)))
        };
        let mut swatches = div()
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .w(px(4. * 22. + 3. * 6.));
        for choice in GroupColor::ALL {
            let chosen = picked == Some(choice);
            swatches = swatches.child(
                div()
                    .id(SharedString::from(format!(
                        "group-color-{id}-{}",
                        choice.name()
                    )))
                    .size(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .border_2()
                    .border_color(if chosen {
                        hsla(chrome.text)
                    } else {
                        gpui::transparent_black()
                    })
                    .cursor_pointer()
                    .child(swatch(choice, 14.))
                    .tooltip(move |window, cx| Tooltip::new(choice.label()).build(window, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.set_group_color(id, Some(choice), window, cx);
                    })),
            );
        }
        let automatic = div()
            .id(SharedString::from(format!("group-color-{id}-automatic")))
            .mt(px(8.))
            .px(px(6.))
            .py(px(4.))
            .rounded(px(6.))
            .flex()
            .items_center()
            .gap(px(6.))
            .cursor_pointer()
            .text_size(px(12.5))
            .text_color(hsla(chrome.text))
            .hover(move |style| style.bg(hsla(chrome.selected)))
            .child(
                div()
                    .w(px(12.))
                    .child(if picked.is_none() { "✓" } else { "" }),
            )
            .child("Automatic")
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.set_group_color(id, None, window, cx);
            }));
        let trigger = Button::new(SharedString::from(format!("group-dot-{id}")))
            .ghost()
            .xsmall()
            .child(swatch(color, 9.))
            .tooltip("Colour");
        let this = cx.entity().downgrade();
        div()
            .id(SharedString::from(format!("group-dot-slot-{id}")))
            .flex_none()
            // The dot picks a colour; it neither collapses nor drags the group.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                Popover::new(SharedString::from(format!("group-colors-{id}")))
                    .open(open)
                    .on_open_change(move |open, _, cx| {
                        let open = *open;
                        let _ = this.update(cx, |this, cx| {
                            this.color_picker = open.then_some(id);
                            cx.notify();
                        });
                    })
                    .trigger(trigger)
                    .child(div().p(px(4.)).child(swatches).child(automatic)),
            )
            .into_any_element()
    }

    pub(super) fn set_group_collapsed(
        &mut self,
        id: i64,
        collapsed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Shown at once; the host's answer confirms it.
        if let Some(group) = self.groups.iter_mut().find(|group| group.id == id) {
            group.collapsed = collapsed;
        }
        cx.notify();
        self.host_call(
            window,
            cx,
            move |paths| hq_client::collapse_group(paths, id, collapsed),
            |_, (), _, _| {},
        );
    }

    /// Puts a dragged session where it was dropped; dropping it where it already is
    /// changes nothing.
    fn drop_session(
        &mut self,
        dragged: &DraggedSession,
        target: Target,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.drop_target = None;
        let current = self.session(&dragged.id).and_then(|session| session.group);
        let group = match target {
            Target::NewGroup => return self.new_group(Some(dragged.id.clone()), window, cx),
            Target::Group(id) => Some(id),
            Target::Ungrouped => None,
        };
        if group != current {
            self.set_session_group(dragged.id.clone(), group, window, cx);
        }
        cx.notify();
    }

    fn set_session_group(
        &mut self,
        id: String,
        group: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) {
            session.group = group;
        }
        cx.notify();
        self.host_call(
            window,
            cx,
            move |paths| hq_client::set_group(paths, &id, group),
            |_, (), _, _| {},
        );
    }

    /// The keyboard alternative to dragging (D4): the selected session moves to the
    /// group above or below, the ungrouped list counting as the last place.
    pub(super) fn move_selected_to_neighbour_group(
        &mut self,
        step: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let current = self.session(&id).and_then(|session| session.group);
        let next = model::neighbour_group(&self.groups, current, step);
        if next != current {
            self.set_session_group(id, next, window, cx);
        }
    }

    /// Moves a dragged group to just before `before`, or to the end.
    fn drop_group(
        &mut self,
        dragged: &DraggedGroup,
        before: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if before == Some(dragged.id) {
            return;
        }
        let Some(from) = self.groups.iter().position(|group| group.id == dragged.id) else {
            return;
        };
        let moved = self.groups.remove(from);
        let to = before
            .and_then(|before| self.groups.iter().position(|group| group.id == before))
            .unwrap_or(self.groups.len());
        self.groups.insert(to, moved);
        cx.notify();
        let id = dragged.id;
        self.host_call(
            window,
            cx,
            move |paths| hq_client::move_group(paths, id, before),
            |_, (), _, _| {},
        );
    }

    fn delete_group(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_delete = None;
        self.groups.retain(|group| group.id != id);
        for session in &mut self.sessions {
            if session.group == Some(id) {
                session.group = None;
            }
        }
        window.focus(&self.sidebar_focus, cx);
        cx.notify();
        self.host_call(
            window,
            cx,
            move |paths| hq_client::delete_group(paths, id),
            |_, (), _, _| {},
        );
    }

    fn group(&self, id: i64) -> Option<&GroupInfo> {
        self.groups.iter().find(|group| group.id == id)
    }

    // ---- drawing ------------------------------------------------------------------

    /// The sidebar's items, with each drop target's items in one block that lights up
    /// while a dragged session is over it.
    pub(super) fn render_items(
        &self,
        items: &[Item],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        let mut index = 0;
        // A group being named goes after the last group: before the ungrouped
        // sessions, or at the end (D3).
        let pending_at = items
            .iter()
            .position(|item| item.target() == Some(Target::Ungrouped))
            .unwrap_or(items.len());
        let mut pending = self.render_pending_group(cx);
        while index < items.len() {
            if index == pending_at
                && let Some(field) = pending.take()
            {
                out.push(field);
            }
            let Some(target) = items[index].target() else {
                out.push(self.render_item(&items[index], window, cx));
                index += 1;
                continue;
            };
            let mut block = Vec::new();
            while index < items.len() && items[index].target() == Some(target) {
                block.push(self.render_item(&items[index], window, cx));
                index += 1;
            }
            out.push(self.drop_block(target, block, cx));
        }
        out.extend(pending);
        // With nothing ungrouped there is no "Ungrouped" label; during a drag a drop
        // area takes its place, so a session can always leave its group (D2).
        let ungrouped_shown = items
            .iter()
            .any(|item| item.target() == Some(Target::Ungrouped));
        if !self.groups.is_empty() && !ungrouped_shown && cx.has_active_drag() {
            let label = ungrouped_label(self.theme.chrome, "drop to ungroup".to_owned());
            out.push(self.drop_block(Target::Ungrouped, vec![label], cx));
        }
        out
    }

    fn drop_block(
        &self,
        target: Target,
        children: Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let key = match target {
            Target::Group(id) => format!("drop-group-{id}"),
            Target::Ungrouped => "drop-ungrouped".to_owned(),
            Target::NewGroup => "drop-new-group".to_owned(),
        };
        let block = div()
            .id(SharedString::from(key.clone()))
            .flex()
            .flex_col()
            .rounded(px(10.))
            .border_1()
            .border_color(gpui::transparent_black())
            .children(children)
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<DraggedSession>, _, cx| {
                    let inside = event.bounds.contains(&event.event.position);
                    if inside && this.drop_target != Some(target) {
                        this.drop_target = Some(target);
                        cx.notify();
                    } else if !inside && this.drop_target == Some(target) {
                        this.drop_target = None;
                        cx.notify();
                    }
                }),
            )
            .on_drop(
                cx.listener(move |this, dragged: &DraggedSession, window, cx| {
                    this.drop_session(dragged, target, window, cx);
                }),
            );
        let lit = self.drop_target == Some(target) && cx.has_active_drag();
        if !lit {
            return block.into_any_element();
        }
        let fill = hsla(chrome.selected);
        let mut edge = hsla(chrome.text);
        edge.a = 0.35;
        block
            .with_animation(
                SharedString::from(format!("{key}-lit")),
                Animation::new(LIGHT_UP),
                move |block, delta| {
                    let mut fill = fill;
                    fill.a *= delta;
                    let mut edge = edge;
                    edge.a *= delta;
                    block.bg(fill).border_color(edge)
                },
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_group_header(
        &self,
        id: i64,
        name: &str,
        (total, beside_note): (usize, Option<&str>),
        needing: usize,
        collapsed: bool,
        colors: (GroupColor, Option<GroupColor>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = self.theme.chrome;
        let mut hover = hsla(chrome.selected);
        hover.a = 0.55;
        let mut insertion = hsla(chrome.text);
        insertion.a = 0.6;
        let renaming = self
            .group_rename
            .as_ref()
            .filter(|rename| rename.naming == Naming::Group(id));
        // Clicking the name renames the group (the pointer becomes a text cursor over
        // it); clicking the rest of the header collapses it (D6).
        let label: AnyElement = match renaming {
            Some(rename) => self.name_field(rename, cx),
            None => div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(
                    div()
                        .id(SharedString::from(format!("group-name-{id}")))
                        .min_w_0()
                        .truncate()
                        .cursor_text()
                        .text_size(px(13.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(hsla(chrome.text))
                        .child(name.to_owned())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.start_group_rename(id, window, cx);
                        })),
                )
                .into_any_element(),
        };
        let hover_key = format!("group-{id}");
        // A collapsed group hides its rows, so a session waiting there shows as a
        // fading amber dot and an amber edge on the header (D5, option A).
        let waiting = collapsed && needing > 0;
        let dragged = DraggedGroup {
            id,
            name: name.to_owned(),
            chrome,
        };
        let mut header = div()
            .id(SharedString::from(format!("group-{id}")))
            .group(group_key(id))
            .relative()
            .flex()
            .items_center()
            .gap(px(HEADER_GAP))
            .mt(px(8.))
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .border_t_2()
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .text_size(px(12.5))
            .child(self.render_color_dot(id, colors, cx))
            .child(
                div()
                    .id(SharedString::from(format!("chevron-{id}")))
                    .w(px(12.))
                    .text_color(hsla(chrome.muted))
                    .child(if collapsed { "▸" } else { "▾" })
                    .tooltip(move |window, cx| {
                        Tooltip::new(if collapsed { "Expand" } else { "Collapse" })
                            .build(window, cx)
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.set_group_collapsed(id, !collapsed, window, cx);
                    })),
            )
            .children(waiting.then(|| waiting_dot(self.motion_phase(WAITING_CYCLE), chrome)))
            .child(label)
            .child(
                div()
                    .flex_none()
                    .text_color(hsla(chrome.muted))
                    .child(match beside_note {
                        Some(note) => format!("{} · {note}", model::session_count(total)),
                        None => model::session_count(total),
                    }),
            )
            .child(self.hover_close(
                &hover_key,
                HEADER_GAP,
                "Delete group",
                move |this, window, cx| {
                    this.confirm_delete = Some(id);
                    // The sidebar takes Escape, which cancels.
                    window.focus(&this.sidebar_focus, cx);
                    cx.notify();
                },
                cx,
            ))
            .on_hover({
                let hover_key = hover_key.clone();
                cx.listener(move |this, over: &bool, _, cx| this.set_hover(&hover_key, *over, cx))
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                if this.group_rename.is_none() {
                    this.set_group_collapsed(id, !collapsed, window, cx);
                }
            }))
            .on_drag(dragged, |dragged, _, _, cx| cx.new(|_| dragged.clone()))
            // A group dragged over this header lands above it; the line shows where.
            .drag_over::<DraggedGroup>(move |style, _, _, _| style.border_color(insertion))
            .on_drop(
                cx.listener(move |this, dragged: &DraggedGroup, window, cx| {
                    this.drop_group(dragged, Some(id), window, cx);
                }),
            );
        if waiting {
            let note = model::needing_note(needing);
            header = header
                .child(
                    div()
                        .absolute()
                        .left(px(0.))
                        .top(px(6.))
                        .bottom(px(6.))
                        .w(px(2.))
                        .rounded(px(1.))
                        .bg(hsla(chrome.glyph_amber)),
                )
                .tooltip(move |window, cx| Tooltip::new(note.clone()).build(window, cx));
        }
        header.into_any_element()
    }

    /// A name field; Escape cancels.
    fn name_field(&self, rename: &GroupRename, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_1()
            .min_w_0()
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.finish_group_rename(false, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(Input::new(&rename.input).small())
            .into_any_element()
    }

    /// The header of a group being named before it exists.
    fn render_pending_group(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let rename = self
            .group_rename
            .as_ref()
            .filter(|rename| matches!(rename.naming, Naming::New { .. }))?;
        let chrome = self.theme.chrome;
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .mt(px(8.))
                .h(px(28.))
                .px(px(8.))
                .text_size(px(12.5))
                .child(div().w(px(12.)).text_color(hsla(chrome.muted)).child("▾"))
                .child(self.name_field(rename, cx))
                .into_any_element(),
        )
    }

    pub(super) fn render_new_group_line(&self, cx: &mut Context<Self>) -> AnyElement {
        let chrome = self.theme.chrome;
        let mut hover = hsla(chrome.selected);
        hover.a = 0.55;
        div()
            .id("new-group")
            .flex()
            .items_center()
            .gap(px(6.))
            .mt(px(6.))
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .text_size(px(12.5))
            .text_color(hsla(chrome.muted))
            .child(div().w(px(12.)).child("+"))
            .child("New group")
            .on_click(cx.listener(|this, _, window, cx| this.new_group(None, window, cx)))
            // Groups dropped here move to the end.
            .on_drop(cx.listener(|this, dragged: &DraggedGroup, window, cx| {
                this.drop_group(dragged, None, window, cx);
            }))
            .into_any_element()
    }

    pub(super) fn render_ungrouped_header(&self, total: usize) -> AnyElement {
        let chrome = self.theme.chrome;
        ungrouped_label(chrome, model::session_count(total))
    }

    /// The modal asking before a group is deleted (D6).
    pub(super) fn render_confirm_delete(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = self.confirm_delete?;
        let group = self.group(id)?;
        let chrome = self.theme.chrome;
        let count = self
            .sessions
            .iter()
            .filter(|session| session.in_sidebar && session.group == Some(id))
            .count();
        let moved = match count {
            0 => "It has no sessions.".to_owned(),
            1 => "Its session moves to Ungrouped.".to_owned(),
            _ => format!("Its {count} sessions move to Ungrouped."),
        };
        Some(
            div()
                .id("confirm-delete-backdrop")
                // The click that closes the dialog must not also reach what is behind.
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(super::overlay(chrome))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.confirm_delete = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .id("confirm-delete")
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
                                .child(format!("Delete group \u{201c}{}\u{201d}?", group.name)),
                        )
                        .child(
                            div()
                                .text_color(hsla(chrome.muted))
                                .child(format!("{moved} No session is ended or removed.")),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(Button::new("delete-cancel").label("Cancel").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.confirm_delete = None;
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    Button::new("delete-confirm")
                                        .danger()
                                        .label("Delete")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.delete_group(id, window, cx);
                                        })),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}

/// The quiet "Ungrouped" label, with `right` (a count or a hint) at its end.
fn ungrouped_label(chrome: Chrome, right: String) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .mt(px(8.))
        .h(px(28.))
        .px(px(8.))
        .text_size(px(12.5))
        .child(div().w(px(12.)))
        .child(
            div()
                .flex_1()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(hsla(chrome.muted))
                .child("Ungrouped"),
        )
        .child(
            div()
                .flex_none()
                .text_color(hsla(chrome.muted))
                .child(right),
        )
        .into_any_element()
}

/// The amber dot of a collapsed group waiting on the owner: it fades between 35% and
/// full strength every 2 s on the workspace's motion clock, or holds still when the
/// system asks for reduced motion.
pub(super) fn waiting_dot(phase: Option<f32>, chrome: Chrome) -> AnyElement {
    let dot = div()
        .flex_none()
        .size(px(7.))
        .rounded(px(4.))
        .bg(hsla(chrome.glyph_amber));
    match phase {
        Some(phase) => dot
            .opacity(gpui::pulsating_between(0.35, 1.0)(phase))
            .into_any_element(),
        None => dot.into_any_element(),
    }
}

/// The hover group name of a group header, so its "⋯" shows while it is hovered.
fn group_key(id: i64) -> SharedString {
    SharedString::from(format!("group-header-{id}"))
}

/// The gap between the parts of a group header.
const HEADER_GAP: f32 = 6.0;
