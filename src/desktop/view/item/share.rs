//! The item header's Share (design, Lane B, Share controls). A slot for
//! now: Task 8 draws Share after Edit or Save/Done and sends `item sync`.

use gpui_kit::{AnyElement, Context, Window};

use super::ItemPane;

/// The tab's Share: what it last sent and how that stands.
#[derive(Default)]
pub(super) struct ShareState {}

impl ItemPane {
    /// The tab opened: what Share reads of the repository (its
    /// publication remote) is read into `ShareState`, so the header reads
    /// it from there. Nothing yet: Task 8 reads it.
    pub(super) fn share_opened(&mut self, _cx: &mut Context<Self>) {}

    /// The request model changed: how the tab's share stands, its stage
    /// and its outcome, is taken into `ShareState`. Nothing yet: Tasks 8
    /// to 10b follow it.
    pub(super) fn share_progressed(&mut self, _cx: &mut Context<Self>) {}

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
