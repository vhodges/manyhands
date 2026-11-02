//! A comment's posting (design, Lane D, Posting): the pending rows drawn
//! in the thread until each post's outcome. A slot for now: Task 13 posts.

use gpui_kit::{AnyElement, App};

/// The tab's posts in flight.
#[derive(Default)]
pub(super) struct Posting {}

impl Posting {
    /// The pending rows of the posts being sent. Nothing yet.
    pub(super) fn render(&self, _cx: &App) -> Option<AnyElement> {
        None
    }
}
