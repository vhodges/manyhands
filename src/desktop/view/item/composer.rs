//! The discussion's comment composers (design, Lane C): the new-comment
//! composer after the threads and the one reply composer. A slot for now:
//! Task 11a builds the `Composer` entity, and Task 12 places it.

use gpui_kit::{AnyElement, Context, Window};

use super::ItemPane;

/// The tab's composers.
#[derive(Default)]
pub(super) struct Composers {}

impl ItemPane {
    /// The composers, at the discussion's end, after the threads and the
    /// posts being sent. Nothing yet.
    pub(super) fn composers_view(
        &self,
        _window: &Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }
}
