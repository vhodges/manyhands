//! The draft registry: what this session knows of the draft store, shared
//! by the workspace, the tabs and the Drafts page.
//!
//! Store calls are blocking file I/O. They run on the background executor
//! as reads do, each behind the seam's gate of its `ReadKind`, and come
//! back to the asking entity with the stamp they were asked under, so a
//! late listing never replaces a newer one.
//!
//! When the store did not open, the desktop still starts: the registry
//! keeps why, lists nothing and offers no draft.

use std::fmt;

use gpui_kit::{Context, Task};
use manyhands::{
    drafts::{DraftError, DraftErrorKind, DraftId, DraftStore, Entry, Held, ItemKind, Listing},
    repository::ItemDtoKind,
    results::ResultCode,
};

use super::{ReadFailure, ReadKind, Services};
use crate::desktop::model::{
    list_state::ListState,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};

/// Why the draft store did not open: the desktop's copy of the error's
/// kind. Its text is the store's fixed sentence for that kind; it carries
/// no path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct DraftStoreUnavailable(pub DraftErrorKind);

impl From<DraftError> for DraftStoreUnavailable {
    fn from(error: DraftError) -> Self {
        Self(error.kind)
    }
}

impl fmt::Display for DraftStoreUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        DraftError { kind: self.0 }.fmt(formatter)
    }
}

/// The store's item kind of a desktop item.
fn item_kind(kind: ItemDtoKind) -> ItemKind {
    match kind {
        ItemDtoKind::Document => ItemKind::Document,
        ItemDtoKind::Ticket => ItemKind::Ticket,
    }
}

/// The drafts this session knows of. Later it also holds the sessions and
/// the tabs they are attached to, and the saves in flight.
pub(crate) struct DraftRegistry {
    /// Why the store did not open; `None` when it did.
    unavailable: Option<DraftStoreUnavailable>,
    /// The last listing of the store.
    listing: ListState<Vec<Listing>>,
    generations: Generations,
    /// Replacing a listing drops the previous one.
    listing_read: Option<Task<()>>,
}

impl DraftRegistry {
    pub(crate) fn new(unavailable: Option<DraftStoreUnavailable>) -> Self {
        Self {
            unavailable,
            listing: ListState::Loading,
            generations: Generations::default(),
            listing_read: None,
        }
    }

    /// Why the store did not open, if it did not.
    pub(crate) fn unavailable(&self) -> Option<DraftStoreUnavailable> {
        self.unavailable
    }

    /// Lists the store again; an earlier listing is dropped when it
    /// completes. The listing already shown stays until the new one
    /// lands. A store that did not open is not listed.
    pub(crate) fn reload_listing(&mut self, services: &Services, cx: &mut Context<Self>) -> Stamp {
        let stamp = self.generations.next(ViewScope::Drafts);
        if self.unavailable.is_some() {
            self.listing_read = None;
            return stamp;
        }
        self.listing_read = Some(services.draft_read(
            ReadKind::Drafts,
            stamp.clone(),
            DraftStore::list,
            cx,
            |registry: &mut Self, stamp, result, cx| {
                if registry.apply_listing(stamp, result) {
                    cx.notify();
                }
            },
        ));
        stamp
    }

    /// A listing completed. False when it is no longer wanted. A store
    /// that failed to list reads as a failed list; its error's text is
    /// not kept.
    pub(crate) fn apply_listing(
        &mut self,
        stamp: Stamp,
        result: Result<Vec<Listing>, DraftError>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        self.listing = match result {
            Ok(rows) => ListState::Ready {
                rows,
                complete: true,
                index: None,
            },
            // The store's kinds are not result codes: the list reads as
            // failed, as a read the library could not finish does.
            Err(_) => ListState::Failed(ReadFailure {
                code: ResultCode::InternalError,
                recovery: Vec::new(),
            }),
        };
        true
    }

    /// The last listing of the store.
    #[cfg_attr(not(test), allow(dead_code))] // Read by the Drafts page; tests until then.
    pub(crate) fn listing(&self) -> &ListState<Vec<Listing>> {
        &self.listing
    }

    /// The newest stored draft of the item `id` of `kind` in `repo` that
    /// this process has not claimed and can read. `None` while the store
    /// is unavailable or not yet listed. A comment's draft is not the
    /// item's.
    #[cfg_attr(not(test), allow(dead_code))] // Asked by a tab's banner; tests until then.
    pub(crate) fn stored_for(
        &self,
        repo: &RepoKey,
        kind: ItemDtoKind,
        id: &str,
    ) -> Option<DraftId> {
        if self.unavailable.is_some() {
            return None;
        }
        let kind = item_kind(kind);
        self.listing.rows()?.iter().find_map(|listing| {
            let Entry::Draft(draft) = &listing.entry else {
                return None;
            };
            let target = &draft.target;
            (listing.held != Held::Here
                && target.comment.is_none()
                && target.item_kind == kind
                && target.repository == *repo
                && target.item_id.to_string() == id)
                .then_some(listing.id)
        })
    }
}

impl Services {
    /// Runs `f` against the draft store on the background executor, after
    /// the gate of `kind`, and hands its result to `apply` on the asking
    /// entity, with `stamp`. A store that did not open answers with why.
    /// `apply` is not called if the entity is gone.
    pub(crate) fn draft_read<V: 'static, T: Send + 'static>(
        &self,
        kind: ReadKind,
        stamp: Stamp,
        f: impl FnOnce(&DraftStore) -> Result<T, DraftError> + Send + 'static,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, Result<T, DraftError>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let backend = self.backend.clone();
        // Taken now, in the order requests are made; awaited in the read.
        let gate = backend.read_gate(kind);
        let read = cx.background_executor().spawn(async move {
            if let Some(gate) = gate {
                // A dropped sender releases the read as a send does.
                let _ = gate.await;
            }
            let store = backend.drafts().map_err(|unavailable| DraftError {
                kind: unavailable.0,
            })?;
            f(store)
        });
        cx.spawn(async move |entity, cx| {
            let result = read.await;
            // The entity may be gone; then nobody wants the result.
            let _ = entity.update(cx, |entity, cx| apply(entity, stamp, result, cx));
        })
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_tests.rs"]
mod tests;
