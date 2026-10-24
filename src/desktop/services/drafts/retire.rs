//! A committed save's completion: the item read again for the new base,
//! then the saved revision retired, only if it is still the session's
//! newest, or the session rebased onto that base.
//!
//! While the revision retires no write is taken; edits are still taken in
//! memory. If the store kept the draft, the claim goes back and the
//! session is rebased. If the draft was deleted, or the retirement failed
//! (the store consumed the claim and released the draft), newer text is
//! stored again as a new draft over the new base, and the session goes on
//! under that draft's ID (`DraftRegistry::current_id` follows it). A store
//! again that fails keeps the text in memory, failed for its own reason,
//! and the next write stores it again. A call that lands after its session
//! was discarded puts any claim it holds back in the session's slot, for
//! the discard waiting on it to discard.

use std::{path::PathBuf, sync::Mutex};

use futures::future::{FutureExt, Shared};
use gpui_kit::{Context, Task};
use manyhands::{
    drafts::{DraftBase, DraftClaim, DraftError, DraftErrorKind, DraftId, Retired},
    repository::ItemDto,
    results::ResultCode,
};

use super::{DraftRegistry, off_thread, save::SaveRecord};
use crate::desktop::{
    model::drafts::save::base_after_save,
    services::{ReadKind, ReadResult, Reader},
};

/// What the completion does next.
enum Settle {
    Done,
    Wait(Shared<Task<()>>),
}

impl DraftRegistry {
    /// A committed save of `record`: reads the item again, then settles
    /// the session.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    pub(super) fn complete(&mut self, mut record: SaveRecord, cx: &mut Context<Self>) {
        let Some(draft) = self.current_id(record.draft) else {
            return;
        };
        record.draft = draft;
        let Some(live) = self.sessions.get(&record.draft) else {
            return;
        };
        let target = live.session.target();
        let root = PathBuf::from(&target.repository);
        let item = target.item_id.clone();
        let reader = Reader::new(self.backend.clone(), cx.background_executor().clone());
        let read = cx.background_executor().spawn(async move {
            reader
                .call(ReadKind::DraftItem, move |service| {
                    let repo = service.resolve_repository(&root)?;
                    service.show_item(&repo, &item)
                })
                .await
        });
        cx.spawn(async move |registry, cx| {
            let read = read.await;
            let Ok(Some(base)) = registry.update(cx, |registry, _| {
                let live = registry.sessions.get(&registry.current_id(record.draft)?)?;
                let previous = live.session.base()?;
                Some(base_after_save(&record.fields, read_item(&read), previous))
            }) else {
                return;
            };
            loop {
                let step = registry.update(cx, |registry, cx| {
                    registry.settle(record.draft, record.revision, &base, cx)
                });
                match step {
                    Ok(Settle::Wait(write)) => write.await,
                    Ok(Settle::Done) | Err(_) => return,
                }
            }
        })
        .detach();
    }

    /// Retires revision `saved` if it is still the session's newest, in
    /// memory and in the store; otherwise rebases the session onto `base`.
    /// Waits first for any write in flight.
    fn settle(
        &mut self,
        id: DraftId,
        saved: u64,
        base: &DraftBase,
        cx: &mut Context<Self>,
    ) -> Settle {
        let Some(id) = self.current_id(id) else {
            return Settle::Done;
        };
        let Some(live) = self.sessions.get_mut(&id) else {
            return Settle::Done;
        };
        if live.session.busy()
            && let Some(write) = live.write.clone()
        {
            return Settle::Wait(write);
        }
        let session = &mut live.session;
        session.set_saving(None);
        if session.revision() != saved || session.flushed() != saved {
            // Newer text, or a base change, came after the save: it is not
            // retired, and takes the base the save left.
            session.rebase(base.clone(), cx.background_executor().now());
            self.schedule(id, cx);
            cx.notify();
            return Settle::Done;
        }
        let slot = session.begin_retire();
        let kept = slot.clone();
        let base = base.clone();
        // The slot is taken off the foreground: no write holds it now,
        // and none starts while the session retires.
        let work = off_thread(
            &self.backend,
            ReadKind::DraftWrite,
            move |_| take(&slot)?.retire(saved),
            cx,
        );
        let task = cx
            .spawn(async move |registry, cx| {
                let result = work.await;
                let _ = registry.update(cx, |registry, cx| {
                    registry.retire_landed(id, saved, base, &kept, result, cx)
                });
            })
            .shared();
        live.write = Some(task);
        Settle::Done
    }

    fn retire_landed(
        &mut self,
        id: DraftId,
        saved: u64,
        base: DraftBase,
        slot: &Mutex<Option<DraftClaim>>,
        result: Result<Retired, DraftError>,
        cx: &mut Context<Self>,
    ) {
        let Some(live) = self.sessions.get_mut(&id) else {
            // Discarded meanwhile: a kept claim goes back in the slot, for
            // the discard waiting on this call to discard it.
            if let Ok(Retired::Kept(claim)) = result {
                put(slot, claim);
            }
            return;
        };
        let newer = live.session.revision() != saved;
        match result {
            Ok(Retired::Kept(claim)) => {
                live.session.retire_kept(claim);
                live.session.rebase(base, cx.background_executor().now());
                self.schedule(id, cx);
            }
            // Deleted, or the store consumed the claim and released the
            // draft: newer text is stored again; otherwise the session is
            // over.
            Ok(Retired::Deleted) | Err(_) if newer => {
                self.store_again(id, Some(base), cx);
            }
            Ok(Retired::Deleted) | Err(_) => {
                self.sessions.remove(&id);
            }
        }
        cx.notify();
    }

    /// Stores the session's text as a new draft over `base`: after its
    /// draft retired, or its retirement failed, under newer text, and
    /// again on the next write after such a store failed. Writes stay
    /// blocked until it lands; the task is the session's store call in
    /// flight, for a flush or a discard to wait on.
    pub(super) fn store_again(
        &mut self,
        id: DraftId,
        base: Option<DraftBase>,
        cx: &mut Context<Self>,
    ) -> Option<Shared<Task<()>>> {
        let live = self.sessions.get_mut(&id)?;
        let session = &mut live.session;
        session.begin_store_again();
        let (target, current, from, slot) = (
            session.target().clone(),
            session.current().clone(),
            session.revision(),
            session.claim_slot(),
        );
        let kept = base.clone();
        let create = off_thread(
            &self.backend,
            ReadKind::Drafts,
            move |store| store.create(target, base, current),
            cx,
        );
        let task = cx
            .spawn(async move |registry, cx| {
                let created = create.await;
                let _ = registry.update(cx, |registry, cx| {
                    registry.stored_again(id, from, kept, &slot, created, cx)
                });
            })
            .shared();
        live.write = Some(task.clone());
        Some(task)
    }

    fn stored_again(
        &mut self,
        id: DraftId,
        from: u64,
        base: Option<DraftBase>,
        slot: &Mutex<Option<DraftClaim>>,
        created: Result<DraftClaim, DraftError>,
        cx: &mut Context<Self>,
    ) {
        let Some(mut live) = self.sessions.remove(&id) else {
            // Discarded meanwhile: the new draft's claim goes in the slot,
            // for the discard waiting on this call to discard it.
            if let Ok(claim) = created {
                put(slot, claim);
            }
            return;
        };
        match created {
            Ok(claim) => {
                let now = cx.background_executor().now();
                live.session.recreated(claim, from, now);
                live.write = None;
                live.timer = None;
                let new = live.session.id();
                // Whoever holds the old ID reaches the session through it.
                self.moved.insert(id, new);
                self.sessions.insert(new, live);
                self.schedule(new, cx);
            }
            Err(error) => {
                // The text stays in memory, failed for its own reason, over
                // the base the save left; the next write stores it again.
                live.session.recreate_failed(error.kind, base);
                live.write = None;
                self.sessions.insert(id, live);
            }
        }
        cx.notify();
    }
}

/// The claim, taken out of its slot. Blocking only while a write holds
/// the slot; the callers wait for that before.
pub(super) fn take(slot: &std::sync::Mutex<Option<DraftClaim>>) -> Result<DraftClaim, DraftError> {
    slot.lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
        .ok_or(DraftError {
            kind: DraftErrorKind::NotFound,
        })
}

/// Puts `claim` back in a slot it was taken from.
fn put(slot: &Mutex<Option<DraftClaim>>, claim: DraftClaim) {
    *slot.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(claim);
}

/// The registry's read of an item, as `classify` takes one.
fn read_item(read: &ReadResult<ItemDto>) -> Result<&ItemDto, ResultCode> {
    match read {
        ReadResult::Ok(item) => Ok(item),
        ReadResult::Busy => Err(ResultCode::Busy),
        ReadResult::Failed(failure) => Err(failure.code),
    }
}
