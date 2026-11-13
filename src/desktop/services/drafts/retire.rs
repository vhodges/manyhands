//! A committed save's completion, a create's among them: the item read
//! again for the new base, by D2's unverified rule, or for a create by
//! R-20b's (`model::drafts::create::base_after_create`), which clears the
//! new item's marker; `DraftSaved` emitted with it (R-69), then the saved
//! revision retired,
//! only if it is still the session's newest, or the session rebased onto
//! that base. A session whose text is still what the save sent, though it
//! was never stored as that revision (its write failed), ends too: its
//! draft is discarded, any older revision with it, since the commit holds
//! the text (controller ruling, Task 15 fix round 1).
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
    drafts::{DraftBase, DraftClaim, DraftError, DraftErrorKind, DraftFields, DraftId, Retired},
    repository::ItemDto,
    results::ResultCode,
};

use super::{
    DraftRegistry, DraftSaved, off_thread,
    save::{SaveRecord, Verb},
};
use crate::desktop::{
    model::drafts::{create::base_after_create, save::base_after_save},
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
                let read = read_item(&read);
                // A create's clears the new item's marker either way (R-20b).
                Some(match record.verb {
                    Verb::Create => base_after_create(&record.fields, read, previous),
                    Verb::Save => base_after_save(&record.fields, read, previous),
                })
            }) else {
                return;
            };
            loop {
                let step = registry.update(cx, |registry, cx| {
                    registry.settle(record.draft, record.revision, &record.fields, &base, cx)
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
    /// memory and in the store; discards the draft when the session's text
    /// is still `submitted`, the fields the save sent, but was not stored
    /// as `saved`; otherwise rebases the session onto `base`. Waits first
    /// for any write in flight. Whichever it does, `DraftSaved` tells the
    /// tab first, with `submitted` and `base` (R-69).
    fn settle(
        &mut self,
        id: DraftId,
        saved: u64,
        submitted: &DraftFields,
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
        cx.emit(DraftSaved {
            draft: id,
            submitted: submitted.clone(),
            base: base.clone(),
        });
        let session = &mut live.session;
        session.set_saving(None);
        let written = session.revision() == saved && session.flushed() == saved;
        // The commit holds the session's whole text, though the store does
        // not hold it as its newest revision: its write failed, or an edit
        // came and was undone. The session ends, and the draft is
        // discarded, any older revision with it (controller ruling, Task 15
        // fix round 1). A discard that fails leaves the draft stored, for
        // the Drafts page.
        let whole = !written && session.current() == submitted;
        if !written && !whole {
            // Newer text, or a base change, came after the save: it is not
            // retired, and takes the base the save left.
            session.rebase(base.clone(), cx.background_executor().now());
            self.schedule(id, cx);
            cx.notify();
            return Settle::Done;
        }
        // Text that comes while the claim is out is newer than this.
        let at = session.revision();
        let slot = session.begin_retire();
        let kept = slot.clone();
        let base = base.clone();
        // The slot is taken off the foreground: no write holds it now,
        // and none starts while the session retires.
        let work = off_thread(
            &self.backend,
            ReadKind::DraftWrite,
            move |_| {
                let claim = take(&slot)?;
                if whole {
                    claim.discard().map(|()| Retired::Deleted)
                } else {
                    claim.retire(saved)
                }
            },
            cx,
        );
        let task = cx
            .spawn(async move |registry, cx| {
                let result = work.await;
                let _ = registry.update(cx, |registry, cx| {
                    registry.retire_landed(id, at, base, &kept, result, cx)
                });
            })
            .shared();
        live.write = Some(task);
        Settle::Done
    }

    /// The retirement or discard of the claim, decided at the session's
    /// revision `at`, landed.
    fn retire_landed(
        &mut self,
        id: DraftId,
        at: u64,
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
        let newer = live.session.revision() != at;
        match result {
            Ok(Retired::Kept(claim)) => {
                live.session.retire_kept(claim);
                live.session.rebase(base, cx.background_executor().now());
                self.schedule(id, cx);
            }
            // Deleted, or the store consumed the claim and released the
            // draft: newer text is stored again; otherwise the session is
            // over.
            // The store is listed again either way: the draft is gone, or
            // released and no longer held here.
            Ok(Retired::Deleted) | Err(_) if newer => {
                self.store_again(id, Some(base), cx);
                self.reload_listing(cx);
            }
            Ok(Retired::Deleted) | Err(_) => {
                self.sessions.remove(&id);
                self.detach_ended();
                self.reload_listing(cx);
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
                self.release_if_pending(new, cx);
            }
            Err(error) => {
                // The text stays in memory, failed for its own reason, over
                // the base the save left; the next write stores it again.
                live.session.recreate_failed(error.kind, base);
                live.write = None;
                self.sessions.insert(id, live);
                self.schedule(id, cx);
            }
        }
        // A new draft held here, or none: the store is listed as it is.
        self.reload_listing(cx);
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
