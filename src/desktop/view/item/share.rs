//! The item header's Share (design, Lane B, Share controls). A slot for
//! now: Task 8 draws Share after Edit or Save/Done and sends `item sync`.

use gpui_kit::{AnyElement, Context, Window};

use super::ItemPane;

/// The tab's Share: what it last sent and how that stands.
#[derive(Default)]
pub(super) struct ShareState {}

impl ItemPane {
    /// Share, after Edit or Save/Done in the header. Nothing yet.
    pub(super) fn share_button(
        &self,
        _window: &Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }

    /// `ShareItem`: shares the item, when Share is offered. True when it
    /// was taken, so the action goes no further; nothing is offered yet.
    pub(super) fn share_item(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> bool {
        false
    }
}
