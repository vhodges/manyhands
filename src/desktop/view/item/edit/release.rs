//! The tab's part in release on leave (design, Done and leaving: Release on
//! leave; R-37, R-40; `services::release`, which checks the four
//! conditions and sends the release).
//!
//! - **Leaving clean.** Done that ends the session, closing a tab with
//!   nothing unsaved, and a discard of unsaved text with no draft stored
//!   hand the tab's item over (`release_on_leave`). A discard of a stored
//!   draft is the registry's to report, from whichever surface it came,
//!   once the store has deleted it. Done's keep and Keep draft hand over
//!   nothing: a draft is kept.
//! - **Closed while preparing.** The tab's prepare is handed over instead;
//!   its context is released once the answer is `ok` with `created: true`.
//! - **A stranded context** (R-40). An Edit refused with
//!   `recovery_required` while the read shows no context offers "Release
//!   stranded work context", which sends the release at the user's word;
//!   the operation area shows what it came to.

use gpui_kit::Context;
use manyhands::{repository::ItemContextKind, results::ResultCode};

use crate::desktop::{
    model::{edit::Phase, tabs::TabKey},
    services::created_contexts::ContextKey,
    view::item::ItemPane,
};

impl ItemPane {
    /// The context key of the tab's item; `None` for an Inspect tab.
    fn context_key(&self) -> Option<ContextKey> {
        let TabKey::Item { repo, kind, id } = self.tab_key() else {
            return None;
        };
        Some(ContextKey::new(&repo, kind, &id))
    }

    /// Release on leave (R-37, R-75, decision 18): the tab leaves clean,
    /// and its item is handed to the release service. A tab closed while
    /// its Edit prepares hands over the prepare.
    pub(in crate::desktop::view) fn release_on_leave(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.context_key() else {
            return;
        };
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
