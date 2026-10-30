//! A session's save, and what its answer does to the draft. A new item's
//! create goes the same way, over a new item's base (`create`).
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
    drafts::{DraftErrorKind, DraftFields, DraftId, DraftTarget, is_new_item_base},
    repository::MutationDataDto,
    results::{Envelope, ResultCode},
};

use super::{DraftDiscardLanded, DraftRegistry, Live, SessionDiscarded, off_thread, retire::take};
use crate::desktop::{
    model::drafts::{
        RecoveryState, Saving,
        create::create_intent,
        save::{SaveVerdict, save_intent, save_verdict},
    },
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent, Lane},
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
    /// What it was sent as: a new item's create (`create`) completes and is
    /// refused as a create.
    pub(super) verb: Verb,
}

/// What a session's newest revision is sent as.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Verb {
    /// A save over a verified base.
    Save,
    /// A new item's create (`create`), over a new item's base.
    Create,
}

impl DraftRegistry {
    /// Saves the session `id`'s newest revision: writes it to the store if
    /// it is not there yet, then sends it on the user lane. When that write
    /// fails, the revision is sent from memory all the same (controller
    /// ruling 2, Task 15 fix round 1). Answers the request, or `None`,
    /// having sent nothing, while a save of this session is in flight or
    /// its revision retires, when it has no base, and when its base is
    /// unverified, a new item's included: that is created (`create`).
    pub(crate) fn save(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Option<DesktopRequestId>> {
        self.flush_and_send(id, Verb::Save, cx)
    }

    /// Writes the session `id`'s newest revision, then sends it as `verb`,
    /// one save or create at a time. `None`, having sent nothing, when a
    /// save of the session is in flight or its revision retires, and when
    /// its base does not take `verb`: a verified base is saved, a new
    /// item's created.
    pub(super) fn flush_and_send(
        &mut self,
        id: DraftId,
        verb: Verb,
        cx: &mut Context<Self>,
    ) -> Task<Option<DesktopRequestId>> {
        let Some(live) = self
            .current_id(id)
            .and_then(|id| self.sessions.get_mut(&id))
        else {
            return Task::ready(None);
        };
        let session = &mut live.session;
        let takes = session.base().is_some_and(|base| match verb {
            Verb::Save => base.observation.is_some(),
            Verb::Create => is_new_item_base(base),
        });
        if session.saving().is_some() || session.retiring() || !takes {
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
                        .update(cx, |registry, cx| registry.send_save(id, flushed, verb, cx))
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

    /// Sends the save, or the create, once the newest revision is stored;
    /// another flush when an edit came after it.
    fn send_save(
        &mut self,
        id: DraftId,
        flushed: Result<(), DraftErrorKind>,
        verb: Verb,
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
        let intent = session.base().and_then(|base| match verb {
            Verb::Save => save_intent(session.target(), &fields, base),
            Verb::Create => create_intent(session.target(), &fields, base),
        });
        let Some(intent) = intent else {
            session.set_saving(None);
            return Send::Refused;
        };
        let revision = session.revision();
        let request = self.worker.submit(intent, Lane::User, None, None);
        session.set_saving(Some(Saving::Submitted(request, revision)));
        // An earlier create of the session, kept for the same request to be
        // sent again (`finished`), is over: this one is its save now.
        self.saves
            .retain(|_, record| !(record.verb == Verb::Create && record.draft == id));
        self.saves.insert(
            request,
            SaveRecord {
                draft: id,
                revision,
                fields,
                operation: None,
                verb,
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
            // A create stopped by what is at its path: the path is read in
            // the item's own context first (`create`), and the session saves
            // again once that read lands.
            SaveVerdict::Keep
                if record.verb == Verb::Create && envelope.code == ResultCode::OccupiedPath =>
            {
                self.saves.remove(&request);
                self.check_occupied(draft, cx);
            }
            SaveVerdict::Keep => {
                // A create whose effect is not known (an answer lost after
                // it ran, a busy or cancelled one: not a refusal, which
                // touched nothing) keeps its record, so the same request
                // sent again (the worker's `retry`) completes the draft once
                // it has created the item, as its first answer would have.
                // The session's next save or create drops it.
                //
                // The session is not saving meanwhile, so that request sent
                // again (the operation area's Retry, when its answer offers
                // `request.retry`) and a new Save can both be in flight for
                // the item: one save at a time is relaxed here, by ruling
                // (Task 22 fix 1, M7). It is harmless: the new send drops
                // the old record, so the old request's answer changes
                // nothing, and the library answers whichever runs second
                // `already_applied` or `occupied_path`.
                let again = record.verb == Verb::Create
                    && !Intent::refused_answer(envelope.outcome, envelope.code, &envelope.recovery);
                if !again {
                    self.saves.remove(&request);
                }
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
    /// lands, whatever it answered. The tab the session was attached to
    /// hears `SessionDiscarded` as the session ends, and again once the
    /// discard lands, with what it came to (R-71); a discard that deleted
    /// the draft is `DraftDiscardLanded` too, for release on leave.
    pub(crate) fn discard(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        // The tab the session is attached to learns it ended (R-71).
        let tab = self.tab_of(id);
        let Some(live) = self
            .current_id(id)
            .and_then(|current| self.sessions.remove(&current))
        else {
            return Task::ready(Err(DraftErrorKind::NotFound));
        };
        let id = live.session.id();
        self.saves.retain(|_, record| record.draft != id);
        self.detach_ended();
        if let Some(tab) = &tab {
            cx.emit(SessionDiscarded::Ended(tab.clone()));
        }
        cx.notify();
        let Live { session, write, .. } = live;
        let target = session.target().clone();
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
                if let Some(tab) = tab {
                    cx.emit(SessionDiscarded::Landed(tab, discarded.is_ok()));
                }
                if discarded.is_ok() {
                    cx.emit(DraftDiscardLanded(target));
                }
            });
            discarded
        })
    }

    /// Whether a save of a draft of `target`'s item is in flight in this
    /// process: being stored before it is sent, sent, or pending its
    /// checkpoint (design, Release on leave, condition 3).
    pub(crate) fn saving_item(&self, target: impl Fn(&DraftTarget) -> bool) -> bool {
        self.sessions
            .values()
            .any(|live| live.session.saving().is_some() && target(live.session.target()))
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

    /// Ends the session `id` at once, as a release lets it go, its draft
    /// kept and no tab told: for a test of a tab whose D2 session ends
    /// under a save or a write it waits on (Task 27 fix 1, B1 and B2).
    #[cfg(test)]
    pub(crate) fn end_session_for_testing(&mut self, id: DraftId, cx: &mut Context<Self>) {
        if let Some(current) = self.current_id(id) {
            self.sessions.remove(&current);
            self.saves.retain(|_, record| record.draft != current);
        }
        self.detach_ended();
        cx.notify();
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
