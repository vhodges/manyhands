//! The conflict banner's Review changes (D5 design, B5; decision 16; E-7,
//! E-8): a share or a post of the item that stopped on a conflict raises
//! the banner, which says the work is safe and offers Review changes. It
//! opens the resolve conflict sheet on that synchronization, naming the
//! item's work context as its folder; focus comes back to the button when
//! the sheet closes. It does not wait for the item's lifecycle lock:
//! resolving the conflict changes no text the tab holds.

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{Sizable, WindowExt, button::Button},
    div,
};
use manyhands::{canonical::ItemId, repository::OperationId};

use super::{ItemPane, header::repository_name};
use crate::desktop::view::{
    conflict::{self, Opening, REVIEW_CHANGES, SheetSource},
    lifecycle::Heading,
};

/// The banner's Review changes.
pub(crate) const REVIEW_BUTTON: &str = "item-review-changes";

impl ItemPane {
    /// The conflict the item's newest share, or else its newest post,
    /// stopped on: where it stopped and its synchronization.
    fn conflict_to_review(&self, cx: &App) -> Option<(SheetSource, OperationId)> {
        let operations = self.services.operations().read(cx);
        if let Some(outcome) = self
            .share
            .outcome
            .as_ref()
            .filter(|outcome| outcome.conflict)
            && let Some(review) = operations.review(outcome.request)
        {
            return Some((SheetSource::Share, review.operation_id));
        }
        let post = self.post_conflict_request(cx)?;
        Some((SheetSource::Post, operations.review(post)?.operation_id))
    }

    /// Review changes, when the conflict it would open is known.
    pub(super) fn review_button(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.conflict_to_review(cx)?;
        let id = ElementId::from(REVIEW_BUTTON);
        Some(
            div()
                .id(id.clone())
                .test_support()
                .aria_label(REVIEW_CHANGES)
                .flex_none()
                .track_focus(&self.review_focus)
                .on_click(cx.listener(|pane, event: &ClickEvent, window, cx| {
                    if event.is_keyboard() {
                        pane.review_conflict(window, cx);
                    }
                }))
                .child(
                    Button::new(ElementId::NamedChild(
                        std::sync::Arc::new(id),
                        "button".into(),
                    ))
                    .label(REVIEW_CHANGES)
                    .small()
                    .on_click(cx.listener(
                        |pane, _: &ClickEvent, window, cx| {
                            pane.review_conflict(window, cx);
                        },
                    )),
                )
                .into_any_element(),
        )
    }

    /// Review changes pressed: the sheet on the conflict, for the item as
    /// the tab read it.
    fn review_conflict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some((source, operation_id)) = self.conflict_to_review(cx) else {
            return;
        };
        let Some(item) = self.model.item().rows() else {
            return;
        };
        let Some(id) = item.id.as_deref() else {
            return;
        };
        let Ok(parsed) = id.parse::<ItemId>() else {
            return;
        };
        let folder = &item.context.worktree;
        let opening = Opening {
            source,
            root: PathBuf::from(self.repo()),
            operation_id,
            item: parsed,
            heading: Heading::of(item, id, repository_name(self.repo())),
            folder: (!folder.is_empty()).then(|| PathBuf::from(folder)),
            origin: self.review_focus.clone(),
        };
        conflict::open(opening, &self.services, window, cx);
    }
}
