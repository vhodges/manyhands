//! A session's save, and what its answer does to the draft.
//!
//! A save retires only the revision it saved, never newer text (review
//! focus 1). `save` first writes the newest revision N, then sends N's
//! fields (ruling R21), one save at a time (R15). When that write fails, N
//! is sent from memory all the same (controller ruling 2). On a committed
//! answer the item is read again for the new base (R5), any write in
//! flight lands, and only then is the session compared with N:
//!
//! - its revision in memory is still N, and N is what the store holds:
//!   the claim is taken out of its slot off the foreground and retired at
//!   N, and the session ends;
//! - its text is still what was sent, but that text was never stored as
//!   the store's newest revision (its write failed, or an edit came and
//!   was undone): the commit holds the session's whole text, so the draft
//!   is discarded, any older revision with it, and the session ends;
//! - anything newer, an edit or a base change, whether its write landed or
//!   failed: nothing is retired. The session is rebased onto the new base,
//!   which raises its revision, and is written at once.
//!
//! The completion of a committed save, the retirement and storing newer
//! text again, is `retire.rs`. No draft text is ever left only in memory
//! with no way to store it.
//!
//! A pending checkpoint keeps the save (R22) until a Retry of the same
//! request finishes it, the request is dismissed, or its operation is
//! abandoned. A completion for a draft discarded meanwhile, or for a
//! request whose save is over, changes nothing.

use futures::FutureExt;
use gpui_kit::{Context, Task};
use manyhands::{
    drafts::{DraftErrorKind, DraftFields, DraftId},
    repository::MutationDataDto,
    results::Envelope,
};

use super::{DraftRegistry, Live, off_thread, retire::take};
use crate::desktop::{
    model::drafts::{
        RecoveryState, Saving,
        save::{SaveVerdict, save_intent, save_verdict},
    },
    services::{
        ReadKind,
        requests::{DesktopRequestId, Lane},
    },
};

/// A save sent and not yet applied: the revision it saved and the fields
/// it sent. It holds draft text and derives no `Debug`.
pub(super) struct SaveRecord {
    pub(super) draft: DraftId,
    pub(super) revision: u64,
    pub(super) fields: DraftFields,
    /// The operation a pending answer named: abandoning it ends the save.
    pub(super) operation: Option<String>,
}

impl DraftRegistry {
    /// Saves the session `id`'s newest revision: writes it to the store if
    /// it is not there yet, then sends it on the user lane. When that write
    /// fails, the revision is sent from memory all the same (controller
    /// ruling 2, Task 15 fix round 1). Answers the request, or `None`,
    /// having sent nothing, while a save of this session is in flight or
    /// its revision retires, when it has no base (a new item is D3's), and
    /// when its base is unverified.
    pub(crate) fn save(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Option<DesktopRequestId>> {
        let Some(live) = self
            .current_id(id)
            .and_then(|id| self.sessions.get_mut(&id))
        else {
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
        // The flush and the send run to their end whether or not the
        // caller keeps the task: a dropped task must not leave the
        // session flushing, refusing every later save.
        let work = cx
            .spawn(async move |registry, cx| {
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
            .shared();
        let driven = work.clone();
        cx.spawn(async move |_, _| {
            driven.await;
        })
        .detach();
        cx.spawn(async move |_, _| work.await)
    }

    /// Sends the save once the newest revision is stored; another flush
    /// when an edit came after it.
    fn send_save(
        &mut self,
        id: DraftId,
        flushed: Result<(), DraftErrorKind>,
        cx: &mut Context<Self>,
    ) -> Send {
        // The flush may have stored the text again under a new ID.
        let Some(id) = self.current_id(id) else {
            return Send::Refused;
        };
        let Some(live) = self.sessions.get_mut(&id) else {
            return Send::Refused;
        };
        let session = &mut live.session;
        if session.saving() != Some(Saving::Flushing) {
            return Send::Refused;
        }
        // A flush that failed is shown, and the text is sent from memory at
        // once (controller ruling 2): Save is what protects it now. It is
        // never asked to be stored first again, which a failing store would
        // answer the same way for ever. The revision is not written, so it
        // never retires as written (`retire.rs`).
        if flushed.is_ok()
            && (session.busy() || !matches!(session.recovery_state(), RecoveryState::Saved(_)))
        {
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
        let Some(draft) = self.saves.get(&request).map(|record| record.draft) else {
            return;
        };
        // The session may go on under another ID; one discarded meanwhile
        // changes nothing.
        let Some(draft) = self.current_id(draft) else {
            self.saves.remove(&request);
            return;
        };
        let record = self.saves.get_mut(&request).expect("looked up above");
        record.draft = draft;
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
            // The session may go on under another ID.
            if let Some(live) = self
                .current_id(record.draft)
                .and_then(|current| self.sessions.get_mut(&current))
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
    /// retirement the store kept, or a store again that made a new draft,
    /// puts its claim back in the session's slot, so that draft is
    /// discarded too. A save still in flight changes nothing when it
    /// finishes. A slot left empty by a draft already gone answers `Ok`.
    /// On failure the store has released the draft, which stays on disk:
    /// the session is over either way. The store is listed again once it
    /// lands, whatever it answered.
    pub(crate) fn discard(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        let Some(live) = self
            .current_id(id)
            .and_then(|current| self.sessions.remove(&current))
        else {
            return Task::ready(Err(DraftErrorKind::NotFound));
        };
        let id = live.session.id();
        self.saves.retain(|_, record| record.draft != id);
        self.detach_ended();
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
                    move |store| match take(&slot) {
                        Ok(claim) => claim.discard(),
                        // No claim is left: a retirement deleted the
                        // draft, or consumed the claim and released it
                        // and a store again failed. Already gone is
                        // discarded; a released draft is still there.
                        Err(empty) => match store.raw_bytes(id) {
                            Err(gone) if gone.kind == DraftErrorKind::NotFound => Ok(()),
                            _ => Err(empty),
                        },
                    },
                    cx,
                )
            });
            let discarded = discard
                .map_err(|_| DraftErrorKind::Io)?
                .await
                .map_err(|error| error.kind);
            // Gone or released, the draft is listed as it now is.
            let _ = registry.update(cx, |registry, cx| {
                registry.reload_listing(cx);
            });
            discarded
        })
    }

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

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "save_tests.rs"]
mod tests;
