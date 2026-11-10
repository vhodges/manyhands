//! What the registry knows of an item's drafts for the retirement loop
//! (D6 design RT-4 to RT-6, R-36): a three-way answer, since "no draft" and
//! "not known" must not read alike, and a fresh listing to settle it.

use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{DraftStore, Entry},
    repository::ItemDtoKind,
};

use super::{DraftRegistry, item_kind, off_thread};
use crate::desktop::{
    model::{
        list_state::ListState,
        polling::retire_loop::Held,
        stamp::{RepoKey, ViewScope},
    },
    services::ReadKind,
};

impl DraftRegistry {
    /// Whether the store holds a draft of the item `id` of `kind` in `repo`,
    /// or of a comment on it, whoever holds it. A live session counts too:
    /// the cached listing is not reloaded when a session begins.
    ///
    /// `Unknown` when the answer cannot be read: the store did not open, no
    /// listing has landed, the last one failed, or it holds an entry that
    /// cannot be read (it names no target, so no item is known to be free of
    /// it). An unknown listing counts as a draft (R-36).
    pub(crate) fn held_by(&self, repo: &RepoKey, kind: ItemDtoKind, id: &ItemId) -> Held {
        if self.unavailable.is_some() {
            return Held::Unknown;
        }
        let kind = item_kind(kind);
        let of_item = |target: &manyhands::drafts::DraftTarget| {
            target.repository == *repo && target.item_kind == kind && target.item_id == *id
        };
        if self
            .sessions
            .values()
            .any(|live| of_item(live.session.target()))
        {
            return Held::Yes;
        }
        let ListState::Ready { rows, .. } = &self.listing else {
            return Held::Unknown;
        };
        if self.listed == 0 {
            return Held::Unknown;
        }
        let held = rows.iter().any(|row| match &row.entry {
            Entry::Draft(draft) => of_item(&draft.target),
            Entry::Unreadable(_) => false,
        });
        if held {
            Held::Yes
        } else if rows
            .iter()
            .any(|row| matches!(row.entry, Entry::Unreadable(_)))
        {
            Held::Unknown
        } else {
            Held::No
        }
    }

    /// Lists the store again, off the UI thread and behind the `Drafts`
    /// gate, and resolves, once the listing is applied, to whether it
    /// landed as the registry's current one and is ready (RT-6). A listing
    /// that a newer one overtook, or that failed, is `false`.
    pub(crate) fn relist(&mut self, cx: &mut Context<Self>) -> Task<bool> {
        let stamp = self.generations.next(ViewScope::Drafts);
        if self.unavailable.is_some() {
            self.listing_read = None;
            return Task::ready(false);
        }
        let list = off_thread(&self.backend, ReadKind::Drafts, DraftStore::list, cx);
        cx.spawn(async move |registry, cx| {
            let result = list.await;
            registry
                .update(cx, |registry, cx| {
                    let landed = registry.apply_listing(stamp, result);
                    if landed {
                        cx.notify();
                    }
                    landed && matches!(registry.listing, ListState::Ready { .. })
                })
                .unwrap_or(false)
        })
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "held_tests.rs"]
mod tests;
