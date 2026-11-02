//! The item header's Share (design, Lane B, Share controls). A slot for
//! now: Task 8 draws Share after Edit or Save/Done and sends `item sync`.

use gpui_kit::{AnyElement, App};

/// The tab's Share: what it last sent and how that stands.
#[derive(Default)]
pub(super) struct ShareState {}

impl ShareState {
    /// Share, after Edit or Save/Done in the header. Nothing yet.
    pub(super) fn render(&self, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// `ShareItem`: shares the item, when Share is offered. True when it
    /// was taken; nothing is offered yet.
    pub(super) fn share_requested(&mut self) -> bool {
        false
    }
}
