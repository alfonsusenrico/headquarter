//! The sidebar as its own view, so GPUI can keep it from one frame to the next
//! (fix-terminal-redraw-cost). Without it, every frame a busy terminal causes would lay
//! out and paint the whole sidebar again. The workspace keeps all sidebar state and
//! builds the elements; this view only redraws when the workspace notifies.

use gpui::{Context, Entity, IntoElement, Render, Window, div};

use super::Workspace;

pub(super) struct SidebarView {
    workspace: gpui::WeakEntity<Workspace>,
}

impl SidebarView {
    pub(super) fn new(workspace: &Entity<Workspace>, cx: &mut Context<Self>) -> Self {
        cx.observe(workspace, |_, _, cx| cx.notify()).detach();
        SidebarView {
            workspace: workspace.downgrade(),
        }
    }
}

impl Render for SidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.workspace
            .update(cx, |workspace, cx| workspace.render_sidebar(window, cx))
            .unwrap_or_else(|_| div().into_any_element())
    }
}
