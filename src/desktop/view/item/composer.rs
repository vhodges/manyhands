//! The discussion's comment composers (design, Lane C): the new-comment
//! composer after the threads and the one reply composer. A slot for now:
//! Task 11a builds the `Composer` entity, and Task 12 places it.

use gpui_kit::{AnyElement, App};

/// The tab's composers.
#[derive(Default)]
pub(super) struct Composers {}

impl Composers {
    /// The composers, after the threads. Nothing yet.
    pub(super) fn render(&self, _cx: &App) -> Option<AnyElement> {
        None
    }
}
