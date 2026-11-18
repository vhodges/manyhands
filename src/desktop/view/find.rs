//! What the ID read says, under the filter bar (design, Part D). The input
//! is the filter input, drawn in the filter bar.
//!
//! `secondary-p` focuses the input, and Enter in it runs
//! `Workspace::submit_filter`. Text that parses as a ULID opens its item in
//! a tab through `show_item`, a lifecycle-closed ticket as any other, a
//! document too; `filter-status` says "Opening…" while that read is held,
//! and nothing once the tab opens. There is no Retry control: Enter again
//! reads again. Other text filters the list, and Enter opens the one
//! ticket it matches or moves focus to the list. A short code is never a
//! tab key: a tab opens only from an item the library read by its ID.

use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, div, px,
};

use super::{shell::tokens::chrome_colors, workspace::Workspace};
use crate::desktop::model::find::{FindResult, FindState};

/// `filter-status` while the ID read is held.
pub(crate) const FIND_OPENING: &str = "Opening\u{2026}";
pub(crate) const FIND_NO_REPOSITORY: &str = "Select an available repository to find in";

/// "Nothing in {repo} has that ID."
pub(crate) fn nothing_found_text(repo: &str) -> String {
    format!("Nothing in {repo} has that ID.")
}

/// "{repo} is busy right now."
pub(crate) fn busy_text(repo: &str) -> String {
    format!("{repo} is busy right now.")
}

/// What `filter-status` says of the ID read in `repo`, if anything. A read
/// that opened its tab says nothing.
pub(crate) fn status_text(state: &FindState, repo: &str) -> Option<String> {
    match state {
        FindState::Idle | FindState::Done(FindResult::Opened) => None,
        FindState::NoRepository => Some(FIND_NO_REPOSITORY.to_owned()),
        FindState::Searching => Some(FIND_OPENING.to_owned()),
        FindState::Done(FindResult::None) => Some(nothing_found_text(repo)),
        FindState::Busy => Some(busy_text(repo)),
        FindState::Failed(failure) => Some(failure.code.message().to_owned()),
    }
}

impl Workspace {
    /// The line under the filter bar that says where the ID read is, when
    /// there is something to say.
    pub(super) fn filter_status(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let repo = self.find_repo_name().unwrap_or_default();
        let text: SharedString = status_text(self.find_state(), &repo)?.into();
        let colors = chrome_colors(cx);
        Some(
            div()
                .id("filter-status")
                .test_support()
                .aria_label(text.clone())
                .flex_none()
                .w_full()
                .px(px(10.))
                .py(px(3.))
                .text_size(px(12.))
                .text_color(colors.muted)
                .child(text),
        )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "find_tests.rs"]
mod tests;
