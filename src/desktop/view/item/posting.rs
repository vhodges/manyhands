//! A comment's posting (design, Lane D, Posting): the pending row of each
//! post being sent, until its outcome. A slot for now: Task 13 posts.

use gpui_kit::{AnyElement, Context, Window};

use super::ItemPane;

/// The tab's posts in flight.
#[derive(Default)]
pub(super) struct Posting {}

impl ItemPane {
    /// The pending rows of the posts being sent, rendered after the threads
    /// and before the composers; Lane D places each row in its thread.
    /// Nothing yet.
    pub(super) fn posting_rows(
        &self,
        _window: &Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }
}
