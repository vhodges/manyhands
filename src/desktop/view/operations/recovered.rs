//! The launch's "Recovered drafts: N" entry: how many drafts the launch
//! found that this process does not hold, with Review, which opens the
//! Drafts page, and Dismiss, which hides it until the next launch. It
//! names no draft.

use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, component::h_flex,
};

use super::{DISMISS, button, text};
use crate::desktop::{model::operations::recovered_drafts, view::workspace::Workspace};

pub(crate) const REVIEW: &str = "Review";

impl Workspace {
    pub(super) fn recovered_entry(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let count = self.services().drafts().read(cx).recovered()?;
        let review = self.control_focus_for("recovered-drafts-review".to_owned(), cx);
        let dismiss = self.control_focus_for("recovered-drafts-dismiss".to_owned(), cx);
        Some(
            h_flex()
                .gap_2()
                .child(text("recovered-drafts", recovered_drafts(count)))
                .child(button(
                    "recovered-drafts-review",
                    REVIEW,
                    Some(&review),
                    cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.open_drafts(None, window, cx)
                    }),
                ))
                .child(button(
                    "recovered-drafts-dismiss",
                    DISMISS,
                    Some(&dismiss),
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.services()
                            .drafts()
                            .update(cx, |registry, cx| registry.dismiss_recovered(cx))
                    }),
                ))
                .into_any_element(),
        )
    }
}
