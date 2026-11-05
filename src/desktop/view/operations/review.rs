//! Review changes in the operation area (D5 design, B5; decision 16): on
//! the entry of a request whose answer stopped on a conflict (a share, a
//! post, a local synchronization or a lifecycle's child), and on a pending
//! row whose conflict waits to be resolved, after a restart too. Either
//! opens the resolve conflict sheet on that conflict; focus goes back to
//! the button when it closes. Neither waits for an item's lifecycle lock:
//! resolving the conflict is how a stopped lifecycle goes on (P8, P9).

use std::path::PathBuf;

use gpui_kit::{AnyElement, ClickEvent, Context, Window, component::WindowExt};

use super::{button, request_id};
use crate::desktop::{
    model::stamp::RepoKey,
    services::requests::DesktopRequestId,
    view::{
        conflict::{Entry, REVIEW_CHANGES},
        workspace::Workspace,
    },
};

/// The element of a pending row's Review changes.
pub(crate) const PENDING_REVIEW: &str = "pending-review";

impl Workspace {
    /// The main branch and publication remote navigation lists for the
    /// repository at `root`, for a sheet that may open the lifecycle
    /// dialog again.
    pub(super) fn known(&self, root: &str) -> (Option<String>, Option<Option<String>>) {
        self.navigation()
            .entry(root)
            .map(|entry| {
                let configuration = &entry.summary.configuration;
                (
                    configuration.primary_branch.clone(),
                    Some(configuration.publication_remote.clone()),
                )
            })
            .unwrap_or_default()
    }

    /// The sheet's entry of the finished request `id`, when its answer
    /// stopped on a conflict of an item. Share main's names none, and is
    /// not offered.
    fn entry_of(&self, id: DesktopRequestId, cx: &Context<Self>) -> Option<Entry> {
        let operations = self.services().operations().read(cx);
        Entry::of_request(operations.request(id)?, operations)
    }

    /// Review changes on the finished request `id`'s entry, when its
    /// answer stopped on a conflict.
    pub(super) fn review_control(
        &mut self,
        id: DesktopRequestId,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.entry_of(id, cx)?;
        let focus = self.control_focus_for(request_id(id, "review"), cx);
        Some(button(
            request_id(id, "review"),
            REVIEW_CHANGES,
            Some(&focus),
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.review_request(id, window, cx)
            }),
        ))
    }

    fn review_request(
        &mut self,
        id: DesktopRequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(entry) = self.entry_of(id, cx) else {
            return;
        };
        let known = entry
            .root()
            .to_str()
            .map(|root| self.known(root))
            .unwrap_or_default();
        let origin = self.control_focus_for(request_id(id, "review"), cx);
        let services = self.services().clone();
        entry.open(known, origin, &services, window, cx);
    }

    /// Review changes on the pending row `index`, operation `operation_id`
    /// of `root`, when its conflict waits to be resolved.
    pub(super) fn pending_review(
        &mut self,
        root: &RepoKey,
        operation_id: Option<&str>,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let operation_id = operation_id?;
        self.pending_conflict(root, operation_id)?;
        let focus = self.control_focus_for(format!("review:{root}:{operation_id}"), cx);
        let (root, operation) = (root.clone(), operation_id.to_owned());
        Some(button(
            (PENDING_REVIEW, index),
            REVIEW_CHANGES,
            Some(&focus),
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.review_pending(&root, &operation, window, cx)
            }),
        ))
    }

    fn review_pending(
        &mut self,
        root: &RepoKey,
        operation_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(conflict) = self.pending_conflict(root, operation_id).cloned() else {
            return;
        };
        let entry = Entry::of_pending(PathBuf::from(root), &conflict);
        let known = self.known(root);
        let origin = self.control_focus_for(format!("review:{root}:{operation_id}"), cx);
        let services = self.services().clone();
        entry.open(known, origin, &services, window, cx);
    }
}
