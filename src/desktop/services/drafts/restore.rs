//! A stored draft restored into an item's tab, and let go of again when
//! the tab closes keeping it.
//!
//! Restore claims the draft off the foreground, behind the `Drafts` gate,
//! makes its session and attaches it to the tab. It writes nothing to the
//! repository. One session of an item at a time: another draft of an item
//! that already has one in this process is refused, before the claim and
//! again once it lands. A draft gone since the listing is no longer kept,
//! and the store is listed again. A draft another process holds may be a
//! passing collision with a probe: the answer says so, the store is listed
//! again so that a hold that lasts reads as one, and Restore may be
//! tried again.
//!
//! A tab is attached by its key. The session may go on under another
//! draft ID (its text stored again after a save); `attached` follows it
//! with `current_id`, so the tab keeps its session. Continue editing
//! moves a restored session onto the token of its item's context
//! (`adopt_base`, R-42).
//!
//! Release, a close that keeps the draft, writes the newest text first,
//! then ends the session and lets go of the claim: the draft stays on disk,
//! unclaimed, and is listed again. A write that fails keeps the session,
//! and its tab, with the failure shown. Nothing here discards.

use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{
        Draft, DraftBase, DraftClaim, DraftError, DraftErrorKind, DraftId, DraftTarget, Entry, Held,
    },
    repository::ItemDtoKind,
    results::ResultCode,
};

use super::{DraftRegistry, Live, item_kind, off_thread};
use crate::desktop::{
    model::{
        drafts::{DraftSession, RecoveryState},
        tabs::TabKey,
    },
    services::ReadKind,
};

/// Why a draft was not restored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RestoreRefused {
    /// Another draft of the item is restored in this process.
    AnotherOpen,
    /// The draft is gone since the listing.
    NoLongerKept,
    /// Another process holds the draft: perhaps only for a moment, so
    /// Restore may be tried again.
    OpenElsewhere,
    /// The draft is not of the tab's item, or is a comment's.
    NotThisItem,
    /// The store could not claim it, for this reason.
    Failed(DraftErrorKind),
}

impl DraftRegistry {
    /// The session attached to the tab `tab`, by the ID it goes on under.
    pub(crate) fn attached(&self, tab: &TabKey) -> Option<DraftId> {
        self.current_id(*self.attached.get(tab)?)
    }

    /// Attaches the live session `id`, which the item tab `tab` began in its
    /// edit editor, to that tab, as a restore attaches one: the tab holds
    /// it, the strip marks it unsaved, closing asks first and the Drafts
    /// page lists it as held here: the store is listed again, as a restore
    /// lists it. A session already ended is not attached.
    pub(crate) fn attach(&mut self, tab: TabKey, id: DraftId, cx: &mut Context<Self>) {
        let Some(id) = self.current_id(id) else {
            return;
        };
        self.attached.insert(tab, id);
        self.reload_listing(cx);
        cx.notify();
    }

    /// The tab the session of `id` is attached to, by the ID it goes on
    /// under: where the Drafts page's Show goes.
    pub(crate) fn tab_of(&self, id: DraftId) -> Option<TabKey> {
        let id = self.current_id(id)?;
        self.attached
            .iter()
            .find(|(_, attached)| self.current_id(**attached) == Some(id))
            .map(|(tab, _)| tab.clone())
    }

    /// The live session of `id`, or of the draft its text went on as.
    pub(crate) fn session(&self, id: DraftId) -> Option<&DraftSession> {
        self.sessions
            .get(&self.current_id(id)?)
            .map(|live| &live.session)
    }

    /// The stored draft `id` as the last listing read it, and who held it.
    pub(crate) fn listed(&self, id: DraftId) -> Option<(&Draft, Held)> {
        self.listing
            .rows()?
            .iter()
            .find_map(|listing| match (&listing.entry, listing.id == id) {
                (Entry::Draft(draft), true) => Some((draft, listing.held)),
                _ => None,
            })
    }

    /// Where the session `id`'s item stands as `item` reads it now.
    pub(crate) fn classify(
        &self,
        id: DraftId,
        item: Result<&manyhands::repository::ItemDto, ResultCode>,
    ) -> Option<manyhands::drafts::TargetState> {
        Some(self.session(id)?.classify(item))
    }

    /// R-42: the restored session `id` goes on over `base`, its own with the
    /// token and context of the item's read in the context Continue editing
    /// made ready. A base change raises the revision and is written at
    /// once. A session already ended has no base to move.
    pub(crate) fn adopt_base(&mut self, id: DraftId, base: DraftBase, cx: &mut Context<Self>) {
        let Some(id) = self.current_id(id) else {
            return;
        };
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        live.session.rebase(base, cx.background_executor().now());
        self.schedule(id, cx);
        cx.notify();
    }

    /// Restores the stored draft `id` into the item tab `tab`: claims it,
    /// makes its session and attaches it. Answers the session's ID, or why
    /// it was refused.
    pub(crate) fn restore(
        &mut self,
        id: DraftId,
        tab: TabKey,
        cx: &mut Context<Self>,
    ) -> Task<Result<DraftId, RestoreRefused>> {
        if self.item_session(&tab).is_some() {
            return Task::ready(Err(RestoreRefused::AnotherOpen));
        }
        // A claim is a listing's kin: it waits behind `Drafts`.
        let claim = off_thread(
            &self.backend,
            ReadKind::Drafts,
            move |store| store.claim(id),
            cx,
        );
        cx.spawn(async move |registry, cx| {
            let claimed = claim.await;
            registry
                .update(cx, |registry, cx| registry.restored(tab, claimed, cx))
                // Gone with the services: the claim is let go of.
                .unwrap_or(Err(RestoreRefused::Failed(DraftErrorKind::Io)))
        })
    }

    fn restored(
        &mut self,
        tab: TabKey,
        claimed: Result<DraftClaim, DraftError>,
        cx: &mut Context<Self>,
    ) -> Result<DraftId, RestoreRefused> {
        let claim = match claimed {
            Ok(claim) => claim,
            Err(error) => {
                return Err(match error.kind {
                    DraftErrorKind::NotFound => {
                        self.reload_listing(cx);
                        RestoreRefused::NoLongerKept
                    }
                    // Listed again: a hold that lasts then reads as open
                    // elsewhere, and a passing probe as unheld.
                    DraftErrorKind::OpenElsewhere => {
                        self.reload_listing(cx);
                        RestoreRefused::OpenElsewhere
                    }
                    kind => RestoreRefused::Failed(kind),
                });
            }
        };
        // A refused claim is dropped here, which lets it go.
        if !of_item(&claim.draft().target, &tab) {
            return Err(RestoreRefused::NotThisItem);
        }
        // Another restore of the item may have landed meanwhile.
        if self.item_session(&tab).is_some() {
            return Err(RestoreRefused::AnotherOpen);
        }
        let session = DraftSession::new(claim, cx.background_executor().now());
        let id = session.id();
        self.sessions.insert(
            id,
            Live {
                session,
                timer: None,
                write: None,
            },
        );
        self.attached.insert(tab, id);
        self.reload_listing(cx);
        cx.notify();
        Ok(id)
    }

    /// The live session of the item `tab` shows, if this process has one.
    fn item_session(&self, tab: &TabKey) -> Option<DraftId> {
        self.sessions
            .iter()
            .find(|(_, live)| of_item(live.session.target(), tab))
            .map(|(id, _)| *id)
    }

    /// Lets go of the session `id`, keeping its draft: writes the newest
    /// text, then ends the session and releases the claim, and lists the
    /// store again. On a failed write the session, its text and its tab
    /// stay, failed for that reason. A session already ended has nothing
    /// left to keep.
    pub(crate) fn release(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        cx.spawn(async move |registry, cx| {
            loop {
                let flush = registry
                    .update(cx, |registry, cx| registry.flush_now(id, cx))
                    .map_err(|_| DraftErrorKind::Io)?;
                let flushed = flush.await;
                let step = registry
                    .update(cx, |registry, cx| registry.let_go(id, flushed, cx))
                    .map_err(|_| DraftErrorKind::Io)?;
                if let Some(result) = step {
                    return result;
                }
            }
        })
    }

    /// Ends the session `id` once its newest text is stored. `None` when
    /// an edit came after the flush: flush again.
    fn let_go(
        &mut self,
        id: DraftId,
        flushed: Result<(), DraftErrorKind>,
        cx: &mut Context<Self>,
    ) -> Option<Result<(), DraftErrorKind>> {
        // Retired or discarded meanwhile: nothing unsaved is left.
        let Some(current) = self.current_id(id) else {
            return Some(Ok(()));
        };
        if let Err(kind) = flushed {
            return Some(Err(kind));
        }
        let live = self.sessions.get(&current)?;
        if live.session.busy() {
            return None;
        }
        match live.session.recovery_state() {
            RecoveryState::Saved(_) => {}
            RecoveryState::Failed(kind) => return Some(Err(kind)),
            RecoveryState::Pending => return None,
        }
        // Nothing holds the claim's slot: no store call is in flight. The
        // claim goes with the session, which unlocks it.
        self.sessions.remove(&current);
        self.saves.retain(|_, record| record.draft != current);
        self.detach_ended();
        self.reload_listing(cx);
        cx.notify();
        Some(Ok(()))
    }

    /// Releases every session attached to a tab of `root`, as its tabs
    /// close: a removal's completion, or the repository leaving the list.
    /// Each is written first and kept, never discarded; one whose write
    /// fails stays, failed, with its text, and is let go of once a later
    /// write (its timer, Retry, a flush) stores it.
    pub(crate) fn release_repository(&mut self, root: &str, cx: &mut Context<Self>) {
        let ids: Vec<DraftId> = self
            .attached
            .iter()
            .filter(|(tab, _)| tab.repo() == root)
            .filter_map(|(_, id)| self.current_id(*id))
            .collect();
        for id in ids {
            self.release_when_stored(id, cx);
        }
    }

    /// Lets go of the session `id`, whose tab is gone: now if its text
    /// can be written, otherwise as soon as a later write stores it. Until
    /// then it stays live, with its failure and Retry in the operation
    /// area.
    pub(crate) fn release_when_stored(&mut self, id: DraftId, cx: &mut Context<Self>) {
        self.releasing.insert(id);
        self.release(id, cx).detach();
    }

    /// A write of `id` landed: a release waiting for its text to be
    /// stored completes if it now is.
    pub(super) fn release_if_pending(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let pending = self
            .releasing
            .iter()
            .any(|pending| self.current_id(*pending) == Some(id));
        if pending {
            let _ = self.let_go(id, Ok(()), cx);
        }
    }

    /// Forgets the tabs, and the releases waiting, whose session has
    /// ended.
    pub(super) fn detach_ended(&mut self) {
        let ended: Vec<TabKey> = self
            .attached
            .iter()
            .filter(|(_, id)| self.current_id(**id).is_none())
            .map(|(tab, _)| tab.clone())
            .collect();
        for tab in ended {
            self.attached.remove(&tab);
        }
        let ended: Vec<DraftId> = self
            .releasing
            .iter()
            .copied()
            .filter(|pending| self.current_id(*pending).is_none())
            .collect();
        for pending in ended {
            self.releasing.remove(&pending);
        }
    }
}

/// Whether `target` is the item the tab `tab` shows: not a comment of it,
/// and not a nonconforming entry's tab.
fn of_item(target: &DraftTarget, tab: &TabKey) -> bool {
    let TabKey::Item { repo, kind, id } = tab else {
        return false;
    };
    let kind: ItemDtoKind = *kind;
    target.comment.is_none()
        && target.repository == *repo
        && target.item_kind == item_kind(kind)
        && id.parse::<ItemId>().is_ok_and(|id| target.item_id == id)
}
