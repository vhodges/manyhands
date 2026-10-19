//! The main window's view. For now a placeholder that holds the service
//! and renders Root's layers; the shell grows here.

use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
    component::Root, div,
};
use manyhands::repository::RepositoryService;

pub(crate) struct Workspace {
    // Held for the window's life; the views that read it come later.
    _service: RepositoryService,
}

impl Workspace {
    pub(crate) fn new(service: RepositoryService) -> Self {
        Self { _service: service }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Root does not draw these layers itself: the window's first view
        // below it does.
        div()
            .id("workspace")
            .relative()
            .size_full()
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
