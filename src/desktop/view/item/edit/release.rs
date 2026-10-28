//! The tab's part in release on leave (design, Done and leaving: Release on
//! leave; R-37, R-40; `services::release`, which checks the four
//! conditions and sends the release).
//!
//! - **Editing.** The tab tells the release service, from its creation,
//!   whether it edits its item: from Preparing, through Loading and
//!   Editing, to a kept session. No release of the item is decided while
//!   it does (controller ruling, Task 17 fix 1).
//! - **Leaving clean.** Done that ends the session, closing a tab with
//!   nothing unsaved, and a discard of unsaved text with no draft stored
//!   hand the tab's item over (`release_on_leave`). A discard of a stored
//!   draft is the registry's to report, from whichever surface it came,
//!   once the store has deleted it; a draft still beginning is discarded
//!   once it lands (`leave`). Done's keep and Keep draft hand over nothing:
//!   a draft is kept.
//! - **Closed while preparing.** The tab's prepare is handed over instead;
//!   its context is released once the answer is `ok` with `created: true`.
//! - **A stranded context** (R-40). An Edit refused with
//!   `recovery_required` while the read shows no context offers "Release
//!   stranded work context", which sends the release at the user's word;
//!   the operation area shows what it came to.

use gpui_kit::{App, Context};
use manyhands::{repository::ItemContextKind, results::ResultCode};

use crate::desktop::{
    model::{edit::Phase, tabs::TabKey},
    services::{created_contexts::ContextKey, release::Editing},
    view::item::ItemPane,
};

/// R-40's offer, in the design's words (Phases, step 1).
pub(crate) const RELEASE_STRANDED: &str = "Release stranded work context";

impl ItemPane {
    /// The context key of the tab's item; `None` for an Inspect tab.
    fn context_key(&self) -> Option<ContextKey> {
        let TabKey::Item { repo, kind, id } = self.tab_key() else {
            return None;
        };
        Some(ContextKey::new(&repo, kind, &id))
    }

    /// Tells the release service whether this tab edits its item, for as
    /// long as the tab lives.
    pub(in crate::desktop::view::item) fn watch_editing(&self, cx: &mut Context<Self>) {
        let Some(key) = self.context_key() else {
            return;
        };
        let pane = cx.weak_entity();
        let editing: Editing =
            Box::new(move |cx: &App| pane.upgrade().map(|pane| pane.read(cx).holds_session()));
        let releases = self.services.releases().clone();
        releases.update(cx, |releases, _| releases.watch(key, editing));
    }

    /// Whether the tab holds an edit session of its item, live or kept, is
    /// getting one ready, or waits to discard a draft still beginning.
    fn holds_session(&self) -> bool {
        self.edit.phase != Phase::Viewing
            || self.edit.session.is_some()
            || self.edit.discard_begun.is_some()
    }

    /// Release on leave (R-37, R-75, decision 18): the tab leaves clean,
    /// and its item is handed to the release service. A tab closed while
    /// its Edit prepares hands over the prepare. While a Discard waits for
    /// a draft still beginning, nothing is handed over: that discard hands
    /// it over once the draft is gone.
    pub(in crate::desktop::view) fn release_on_leave(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.context_key() else {
            return;
        };
        if self.edit.discard_begun.is_some() {
            return;
        }
        let phase = self.edit.phase;
        let releases = self.services.releases().clone();
        releases.update(cx, |releases, cx| match phase {
            Phase::Preparing(request) => releases.closed_while_preparing(request, key),
            _ => releases.leave(key, cx),
        });
    }

    /// R-40: whether a prepare refused with `code` offers "Release stranded
    /// work context": `recovery_required`, while the read shows no context.
    pub(super) fn stranded_offered(&self, code: ResultCode) -> bool {
        code == ResultCode::RecoveryRequired
            && self
                .model
                .item()
                .rows()
                .is_some_and(|item| item.context.kind != ItemContextKind::Active)
    }

    /// "Release stranded work context": the release is sent, and the
    /// refusal's banner goes.
    pub(super) fn release_stranded(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.context_key() else {
            return;
        };
        let releases = self.services.releases().clone();
        releases.update(cx, |releases, _| releases.stranded(key));
        self.edit.stopped = None;
        cx.notify();
    }
}
