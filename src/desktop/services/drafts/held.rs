//! What the registry knows of an item's drafts for the retirement loop
//! (D6 design RT-4 to RT-6, R-36): a three-way answer, since "no draft" and
//! "not known" must not read alike, and a fresh listing to settle it.

use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{DraftErrorKind, DraftId, DraftStore, Entry},
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

/// How a fresh listing ended (RT-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fresh {
    /// It landed as the registry's current listing and is ready.
    Landed,
    /// A newer listing was asked for meanwhile and took its place, so the
    /// registry's listing is not known to be fresh. Not a failure.
    Overtaken,
    /// The store is unavailable or could not be listed.
    Failed,
}

/// How many drafts an item has, for Retire…'s question (E4 step 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kept {
    /// The store is unavailable, its listing has not landed or failed, or
    /// it holds an entry that cannot be read: the number is not known.
    Unknown,
    /// The drafts of the item itself and of comments on it, a live
    /// session's included, each counted once.
    Counted { item: usize, comments: usize },
}

impl DraftRegistry {
    /// The drafts the item `id` of `kind` in `repo` has, whoever holds
    /// them. A session that is not yet in the listing counts, once.
    pub(crate) fn kept_of(&self, repo: &RepoKey, kind: ItemDtoKind, id: &ItemId) -> Kept {
        if self.held_by(repo, kind, id) == Held::Unknown {
            return Kept::Unknown;
        }
        let kind = item_kind(kind);
        let of_item = |target: &manyhands::drafts::DraftTarget| {
            target.repository == *repo && target.item_kind == kind && target.item_id == *id
        };
        let mut seen = std::collections::BTreeMap::new();
        for (draft, live) in &self.sessions {
            if of_item(live.session.target()) {
                seen.insert(*draft, live.session.target().comment.is_some());
            }
        }
        if let ListState::Ready { rows, .. } = &self.listing {
            for row in rows {
                if let Entry::Draft(draft) = &row.entry
                    && of_item(&draft.target)
                {
                    seen.insert(draft.id, draft.target.comment.is_some());
                }
            }
        }
        let comments = seen.values().filter(|comment| **comment).count();
        Kept::Counted {
            item: seen.len() - comments,
            comments,
        }
    }

    /// Writes the newest revision of every live session of the item now
    /// (E4 step 4), while the item is read-only. Answers the drafts that
    /// could not be written. A session that ended meanwhile left nothing
    /// unsaved, as `flush_all` judges it.
    pub(crate) fn flush_item(
        &mut self,
        repo: &RepoKey,
        kind: ItemDtoKind,
        id: &ItemId,
        cx: &mut Context<Self>,
    ) -> Task<Vec<(DraftId, DraftErrorKind)>> {
        let kind = item_kind(kind);
        let ids: Vec<DraftId> = self
            .sessions
            .iter()
            .filter(|(_, live)| {
                let target = live.session.target();
                target.repository == *repo && target.item_kind == kind && target.item_id == *id
            })
            .map(|(draft, _)| *draft)
            .collect();
        let flushes: Vec<_> = ids
            .into_iter()
            .map(|draft| {
                let flush = self.flush_now(draft, cx);
                async move { (draft, flush.await) }
            })
            .collect();
        cx.spawn(async move |registry, cx| {
            let failed: Vec<(DraftId, DraftErrorKind)> = futures::future::join_all(flushes)
                .await
                .into_iter()
                .filter_map(|(draft, result)| result.err().map(|kind| (draft, kind)))
                .collect();
            registry
                .read_with(cx, |registry, _| {
                    failed
                        .iter()
                        .filter_map(|(draft, kind)| match (registry.current_id(*draft), kind) {
                            (None, DraftErrorKind::NotFound) => None,
                            (current, kind) => Some((current.unwrap_or(*draft), *kind)),
                        })
                        .collect()
                })
                .unwrap_or(failed)
        })
    }

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
    /// gate, and resolves, once the listing is applied, to how it ended
    /// (RT-6): landed as the registry's current one, overtaken by a newer
    /// listing, or failed.
    pub(crate) fn relist(&mut self, cx: &mut Context<Self>) -> Task<Fresh> {
        let stamp = self.generations.next(ViewScope::Drafts);
        if self.unavailable.is_some() {
            self.listing_read = None;
            return Task::ready(Fresh::Failed);
        }
        let list = off_thread(&self.backend, ReadKind::Drafts, DraftStore::list, cx);
        cx.spawn(async move |registry, cx| {
            let result = list.await;
            registry
                .update(cx, |registry, cx| {
                    if !registry.apply_listing(stamp, result) {
                        return Fresh::Overtaken;
                    }
                    cx.notify();
                    match registry.listing {
                        ListState::Ready { .. } => Fresh::Landed,
                        _ => Fresh::Failed,
                    }
                })
                .unwrap_or(Fresh::Failed)
        })
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "held_tests.rs"]
mod tests;
