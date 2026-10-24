//! A session's save, and what its answer does to the draft.
//!
//! A save retires only the revision it saved, never newer text (review
//! focus 1). `save` first writes the newest revision N, then sends N's
//! fields (ruling R21), one save at a time (R15). On a committed answer the
//! item is read again for the new base (R5), any write in flight lands,
//! and only then is the session compared with N:
//!
//! - its revision in memory is still N, and N is what the store holds:
//!   the claim is taken out of its slot off the foreground and retired at
//!   N, and the session ends;
//! - anything newer, an edit or a base change, whether its write landed or
//!   failed: nothing is retired. The session is rebased onto the new base,
//!   which raises its revision, and is written at once.
//!
//! While the revision retires no write is taken; edits are still taken in
//! memory. An edit that lands after the retirement was decided is never
//! lost: if the store kept the draft, the claim goes back and the session
//! is rebased; if the draft was deleted, or the retirement failed (the
//! store consumed the claim and released the draft), the newer text is
//! stored again as a new draft over the new base and the session goes on
//! under that draft's ID. A failed retirement with nothing newer releases
//! the session; its revision stays on disk, unclaimed, and loses nothing.
//!
//! A pending checkpoint keeps the save (R22) until a Retry of the same
//! request finishes it, the request is dismissed, or its operation is
//! abandoned. A completion for a draft discarded meanwhile, or for a
//! request whose save is over, changes nothing.

use std::path::PathBuf;

use futures::future::FutureExt;
use gpui_kit::{Context, Task};
use manyhands::{
    drafts::{DraftBase, DraftClaim, DraftError, DraftErrorKind, DraftFields, DraftId, Retired},
    repository::{ItemDto, MutationDataDto},
    results::{Envelope, ResultCode},
};

use super::{DraftRegistry, Live, off_thread};
use crate::desktop::{
    model::drafts::{
        RecoveryState, Saving,
        save::{SaveVerdict, base_after_save, save_intent, save_verdict},
    },
    services::{
        ReadKind, ReadResult, Reader,
        requests::{DesktopRequestId, Lane},
    },
};

/// A save sent and not yet applied: the revision it saved and the fields
/// it sent. It holds draft text and derives no `Debug`.
pub(super) struct SaveRecord {
    draft: DraftId,
    revision: u64,
    fields: DraftFields,
    /// The operation a pending answer named: abandoning it ends the save.
    operation: Option<String>,
}

/// What the completion does next.
enum Settle {
    Done,
    Wait(futures::future::Shared<Task<()>>),
}

impl DraftRegistry {
    /// Saves the session `id`'s newest revision: writes it to the store if
    /// it is not there yet, then sends it on the user lane. Answers the
    /// request, or `None`, having sent nothing, while a save of this
    /// session is in flight or its revision retires, when it has no base
    /// (a new item is D3's), when its base is unverified, and when the
    /// write before it failed.
    #[cfg_attr(not(test), allow(dead_code))] // Called by D3's editor; tests until then.
    pub(crate) fn save(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Option<DesktopRequestId>> {
        let Some(live) = self.sessions.get_mut(&id) else {
            return Task::ready(None);
        };
        let session = &mut live.session;
        let verified = session
            .base()
            .is_some_and(|base| base.observation.is_some());
        if session.saving().is_some() || session.retiring() || !verified {
            return Task::ready(None);
        }
        session.set_saving(Some(Saving::Flushing));
        cx.spawn(async move |registry, cx| {
            loop {
                let flush = registry
                    .update(cx, |registry, cx| registry.flush_now(id, cx))
                    .ok()?;
                let flushed = flush.await;
                let sent = registry
                    .update(cx, |registry, cx| registry.send_save(id, flushed, cx))
                    .ok()?;
                match sent {
                    Send::Sent(request) => return Some(request),
                    Send::Refused => return None,
                    Send::Again => {}
                }
            }
        })
    }

    /// Sends the save once the newest revision is stored; another flush
    /// when an edit came after it.
    fn send_save(
        &mut self,
        id: DraftId,
        flushed: Result<(), DraftErrorKind>,
        cx: &mut Context<Self>,
    ) -> Send {
        let Some(live) = self.sessions.get_mut(&id) else {
            return Send::Refused;
        };
        let session = &mut live.session;
        if session.saving() != Some(Saving::Flushing) {
            return Send::Refused;
        }
        if flushed.is_err() {
            // The flush failure is shown; nothing is sent.
            session.set_saving(None);
            return Send::Refused;
        }
        if session.busy() || !matches!(session.recovery_state(), RecoveryState::Saved(_)) {
            return Send::Again;
        }
        let fields = session.current().clone();
        let intent = session
            .base()
            .and_then(|base| save_intent(session.target(), &fields, base));
        let Some(intent) = intent else {
            session.set_saving(None);
            return Send::Refused;
        };
        let revision = session.revision();
        let request = self.worker.submit(intent, Lane::User, None, None);
        session.set_saving(Some(Saving::Submitted(request, revision)));
        self.saves.insert(
            request,
            SaveRecord {
                draft: id,
                revision,
                fields,
                operation: None,
            },
        );
        cx.notify();
        Send::Sent(request)
    }

    /// A request finished. A save's answer applies to its draft; anything
    /// else, a save whose draft was discarded, and a save already applied
    /// (a replay answered from F2's record) change nothing.
    pub(crate) fn finished(
        &mut self,
        request: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.saves.get_mut(&request) else {
            return;
        };
        let draft = record.draft;
        let Some(live) = self.sessions.get_mut(&draft) else {
            self.saves.remove(&request);
            return;
        };
        match save_verdict(envelope.outcome, envelope.effects.checkpoint) {
            // F2 holds a pending operation; a Retry of the same request
            // applies its answer again.
            SaveVerdict::Pending => record.operation = envelope.operation_id.clone(),
            SaveVerdict::Keep => {
                self.saves.remove(&request);
                live.session.set_saving(None);
                cx.notify();
            }
            SaveVerdict::Retire => {
                let record = self.saves.remove(&request).expect("the record is here");
                self.complete(record, cx);
            }
        }
    }

    /// The pending save `request` is dismissed in the operation area: it
    /// is over, and the session may save again (ruling R22).
    pub(crate) fn dismissed(&mut self, request: DesktopRequestId, cx: &mut Context<Self>) {
        self.end_pending(|record_request, _| record_request == request, cx);
    }

    /// The pending operation `operation` was abandoned: the pending save
    /// that held it is over (ruling R22).
    pub(crate) fn operation_abandoned(&mut self, operation: &str, cx: &mut Context<Self>) {
        self.end_pending(
            |_, record| record.operation.as_deref() == Some(operation),
            cx,
        );
    }

    fn end_pending(
        &mut self,
        ended: impl Fn(DesktopRequestId, &SaveRecord) -> bool,
        cx: &mut Context<Self>,
    ) {
        let over: Vec<DesktopRequestId> = self
            .saves
            .iter()
            .filter(|(request, record)| ended(**request, record))
            .map(|(request, _)| *request)
            .collect();
        for request in over {
            let record = self.saves.remove(&request).expect("listed above");
            if let Some(live) = self.sessions.get_mut(&record.draft)
                && matches!(
                    live.session.saving(),
                    Some(Saving::Submitted(saving, _)) if saving == request
                )
            {
                live.session.set_saving(None);
            }
            cx.notify();
        }
    }

    /// Discards the session `id`'s draft: the session ends now, and its
    /// draft is deleted once any store call on its claim has landed. A
    /// save still in flight changes nothing when it finishes. On failure
    /// the store has released the draft, which stays on disk.
    #[cfg_attr(not(test), allow(dead_code))] // Called by Task 8's tabs; tests until then.
    pub(crate) fn discard(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        let Some(live) = self.sessions.remove(&id) else {
            return Task::ready(Err(DraftErrorKind::NotFound));
        };
        self.saves.retain(|_, record| record.draft != id);
        cx.notify();
        let Live { session, write, .. } = live;
        let slot = session.claim_slot();
        cx.spawn(async move |registry, cx| {
            if let Some(write) = write {
                write.await;
            }
            let discard = registry.update(cx, |registry, cx| {
                off_thread(
                    &registry.backend,
                    ReadKind::DraftWrite,
                    move |_| take(&slot)?.discard(),
                    cx,
                )
            });
            discard
                .map_err(|_| DraftErrorKind::Io)?
                .await
                .map_err(|error| error.kind)
        })
    }

    /// A committed save of `record`: reads the item again, then settles
    /// the session.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn complete(&mut self, record: SaveRecord, cx: &mut Context<Self>) {
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
                let live = registry.sessions.get(&record.draft)?;
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
                    registry.retire_landed(id, saved, base, result, cx)
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
        result: Result<Retired, DraftError>,
        cx: &mut Context<Self>,
    ) {
        let Some(live) = self.sessions.get_mut(&id) else {
            // Discarded meanwhile: a kept claim is released with its draft.
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
            Ok(Retired::Deleted) | Err(_) if newer => self.store_again(id, base, cx),
            Ok(Retired::Deleted) | Err(_) => {
                self.sessions.remove(&id);
            }
        }
        cx.notify();
    }

    /// Stores the session's text as a new draft over `base`, after its
    /// draft retired under an edit. Writes stay blocked until it lands.
    fn store_again(&mut self, id: DraftId, base: DraftBase, cx: &mut Context<Self>) {
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        let session = &live.session;
        let (target, current, from) = (
            session.target().clone(),
            session.current().clone(),
            session.revision(),
        );
        let create = off_thread(
            &self.backend,
            ReadKind::Drafts,
            move |store| store.create(target, Some(base), current),
            cx,
        );
        let task = cx
            .spawn(async move |registry, cx| {
                let created = create.await;
                let _ = registry.update(cx, |registry, cx| {
                    registry.stored_again(id, from, created, cx)
                });
            })
            .shared();
        live.write = Some(task);
    }

    fn stored_again(
        &mut self,
        id: DraftId,
        from: u64,
        created: Result<DraftClaim, DraftError>,
        cx: &mut Context<Self>,
    ) {
        let Some(mut live) = self.sessions.remove(&id) else {
            // Discarded meanwhile: the new draft is released, kept on disk.
            return;
        };
        match created {
            Ok(claim) => {
                let now = cx.background_executor().now();
                live.session.recreated(claim, from, now);
                live.write = None;
                live.timer = None;
                let new = live.session.id();
                self.sessions.insert(new, live);
                self.schedule(new, cx);
            }
            Err(error) => {
                // The text stays in memory, shown as failed.
                live.session.recreate_failed(error.kind);
                live.write = None;
                self.sessions.insert(id, live);
            }
        }
        cx.notify();
    }

    /// Whether the registry holds no session and no save: for a test.
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.sessions.is_empty() && self.saves.is_empty()
    }

    /// The sessions' draft IDs: for a test.
    #[cfg(test)]
    pub(crate) fn session_ids(&self) -> Vec<DraftId> {
        self.sessions.keys().copied().collect()
    }
}

/// What `send_save` did.
enum Send {
    Sent(DesktopRequestId),
    Refused,
    /// An edit came after the flush: flush again.
    Again,
}

/// The claim, taken out of its slot. Blocking only while a write holds
/// the slot; the callers wait for that before.
fn take(slot: &std::sync::Mutex<Option<DraftClaim>>) -> Result<DraftClaim, DraftError> {
    slot.lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
        .ok_or(DraftError {
            kind: DraftErrorKind::NotFound,
        })
}

/// The registry's read of an item, as `classify` takes one.
fn read_item(read: &ReadResult<ItemDto>) -> Result<&ItemDto, ResultCode> {
    match read {
        ReadResult::Ok(item) => Ok(item),
        ReadResult::Busy => Err(ResultCode::Busy),
        ReadResult::Failed(failure) => Err(failure.code),
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "save_tests.rs"]
mod tests;
