//! What the scheduler does for a retirement request (D6 Part E, design E2,
//! E3, R-31): the in-flight entry written before the request is submitted
//! and ended when the request ends, and the answer's attention, refused set
//! and proof of authentication.
//!
//! The loop that chooses the items (`retire_loop`) and the Retire… row
//! (`retire_chosen`) submit through [`Scheduler::retire`]; a Resume or an
//! Unlock… resends a pending one (`resend_retirement`). A retirement's answer
//! reaches [`Scheduler::retiring`] from the drain.

use std::path::PathBuf;

use gpui_kit::Context;
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, OperationId, RequestId},
};

use super::{begin_retirement, entity::Scheduler, reconfigure::Retiring};
use crate::desktop::{
    model::{
        operations::Operations,
        polling::{
            KeyId, Root, TargetKind,
            retire::{Need, Verdict, reason_of, recorded_key},
        },
    },
    services::{
        inflight::RetirementEnd,
        requests::{DesktopRequestId, Intent, Lane},
        worker::{RecordState, WorkerEvent},
    },
};

/// The kind of item a retirement names, as the schedule knows it.
fn target_kind(kind: AuthoringKind) -> TargetKind {
    match kind {
        AuthoringKind::Ticket => TargetKind::Ticket,
        AuthoringKind::Document => TargetKind::Document,
    }
}

/// Tells the request model where the worker's record of a retirement stood
/// when its last attempt finished, before `Finished` is applied: the event
/// carries none, and the answer is read by it first (E3). `record_of` is the
/// worker's `record_state`; the drain calls this for every event.
pub(crate) fn note_record(
    event: &WorkerEvent,
    operations: &mut Operations,
    record_of: impl FnOnce(DesktopRequestId) -> Option<RecordState>,
) {
    let WorkerEvent::Finished(id, _) = event else {
        return;
    };
    let retires = operations
        .request(*id)
        .is_some_and(|request| matches!(request.intent, Intent::ContextRetire { .. }));
    if retires && let Some(record) = record_of(*id) {
        operations.record_stood(*id, record);
    }
}

impl Scheduler {
    /// Submits a retirement of `item` of `root` under a new request ID
    /// (R-28), after its in-flight entry is written (R-31). The loop sends
    /// it on the background lane, which never prompts (B5); Retire… sends it
    /// on the user lane. A queue that quitting closed answers it `Removed`,
    /// which ends the entry. Nothing is sent once quitting is asked (R-33):
    /// `None`.
    pub(crate) fn retire(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        lane: Lane,
    ) -> Option<DesktopRequestId> {
        self.retire_new(root, kind, item, lane).map(|(_, id)| id)
    }

    /// Retire… on a reported item the loop left alone (E4): a retirement of
    /// it on the user lane, so it may prompt, under a new request ID, and
    /// the item leaves the refused set (A19). RT-1 is checked again here:
    /// `None`, and nothing sent, when one is queued, running or pending, and
    /// once quitting is asked (R-33).
    pub(crate) fn retire_chosen(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        cx: &mut Context<Self>,
    ) -> Option<DesktopRequestId> {
        if super::retire_loop::retirement_out(&self.worker, &self.store, root, kind, item) {
            return None;
        }
        let sent = self.retire(root, kind, item, Lane::User)?;
        self.schedule.retire_chosen(root, item);
        self.publish(cx);
        cx.notify();
        Some(sent)
    }

    /// [`Self::retire`], and the library's request ID it was sent under.
    pub(super) fn retire_new(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        lane: Lane,
    ) -> Option<(RequestId, DesktopRequestId)> {
        if self.quitting {
            return None;
        }
        let request = RequestId::new();
        Some((
            request,
            self.resend_retirement(request, root, kind, item, lane),
        ))
    }

    /// Submits a retirement under the request ID `request`: the original of
    /// a pending one that Resume names (E4), or a new one. An entry already
    /// held for the request, with the operation it learned, is kept.
    pub(crate) fn resend_retirement(
        &mut self,
        request: RequestId,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        lane: Lane,
    ) -> DesktopRequestId {
        let intent = Intent::ContextRetire {
            root: PathBuf::from(root),
            kind: match kind {
                TargetKind::Ticket => AuthoringKind::Ticket,
                TargetKind::Document => AuthoringKind::Document,
            },
            id: item.clone(),
        };
        let worker = self.worker.clone();
        self.entry_epoch += 1;
        // The attempt's entry says "Retiring…" (E3).
        self.schedule.retire_started(root, kind, item);
        begin_retirement(&mut self.store, request, root, kind, item, || {
            worker.submit_with_request_id(intent, lane, request)
        })
    }

    /// A Resume of a pending retirement was answered
    /// `original_request_required`: the original is sent again, under the
    /// request ID the answer names, as a new user-lane request (E4, Fact
    /// 26). Nothing is sent once quitting is asked (R-33).
    pub(super) fn resend_resumed(
        &mut self,
        resume: DesktopRequestId,
        request: RequestId,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        cx: &mut Context<Self>,
    ) {
        if self.quitting {
            return;
        }
        self.resend_retirement(request, root, kind, item, Lane::User);
        // The Resume has done its work: the resend answers in its place, and
        // its blocked answer is not left to count as needing attention.
        let _ = self.operations.update(cx, |operations, cx| {
            operations.dismiss(resume);
            cx.notify();
        });
        self.publish(cx);
    }

    /// An Abandon or a Resume settled the pending retirement whose journal
    /// row is `operation`: the entry ends, so the item is no longer held
    /// "Retiring…", and its line in the attempt's entry says so (E3).
    pub(super) fn pending_ended(
        &mut self,
        root: &Root,
        operation: OperationId,
        finished: bool,
        cx: &mut Context<Self>,
    ) {
        let held = self.store.retirements().iter().find_map(|entry| {
            let (kind, item) = entry.item()?;
            (entry.root() == root && entry.operation() == Some(operation))
                .then(|| (entry.request(), kind, item))
        });
        let Some((request, kind, item)) = held else {
            return;
        };
        self.entry_epoch += 1;
        if let Some(request) = request {
            self.store
                .retirement_ended(request, RetirementEnd::Finished);
        }
        self.schedule.retire_ended(root, kind, &item, finished);
        self.publish(cx);
        self.tick(cx);
    }

    /// Holds a retirement of `item` pending in the in-flight record, as a
    /// previous launch left it (a test): the lock is taken, the entry
    /// written with the operation its journal row has, and left Owed.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn hold_retirement(
        &mut self,
        request: RequestId,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        operation: OperationId,
        cx: &mut Context<Self>,
    ) {
        use crate::desktop::services::inflight::RetirementEntry;
        self.store.try_acquire();
        self.store
            .retirement_submitting(RetirementEntry::new(request, root, kind, item));
        self.store.retirement_operation(request, operation);
        self.store
            .retirement_ended(request, RetirementEnd::Accepted);
        self.publish(cx);
    }

    /// A retirement request's progress, answer or end (the drain, E3).
    pub(super) fn retiring(&mut self, event: Retiring, cx: &mut Context<Self>) {
        self.entry_epoch += 1;
        match event {
            Retiring::Operation { request, operation } => {
                self.store.retirement_operation(request, operation);
            }
            Retiring::Gone { id, request, end } => {
                // A retirement the library already holds pending stays so:
                // its entry, and with it the hold and Resume, outlive a
                // resend taken from the queue (review I-3).
                let ended = self.store.retirement_ended(request, end);
                self.retirement_gone(id, ended, cx);
                // Removed or never sent: the loop that waited on it stops
                // (RT-9).
                self.loop_gone(id, request, cx);
            }
            Retiring::Answered { id, stopped } => self.retirement_answered(id, stopped, cx),
        }
        // The record's entries are what a relaunch finds "Retiring…" (R-31).
        self.publish(cx);
    }

    /// The retirement `id` was removed from the queue or never sent: the
    /// attempt's entry no longer says "Retiring…" of its item (E3), unless
    /// the retirement is still pending (`ended` is false), when it says
    /// that it did not finish.
    fn retirement_gone(&mut self, id: DesktopRequestId, ended: bool, cx: &mut Context<Self>) {
        let item = self
            .operations
            .read_with(cx, |operations, _| {
                let Intent::ContextRetire {
                    root,
                    kind,
                    id: item,
                } = &operations.request(id)?.intent
                else {
                    return None;
                };
                Some((root.to_str()?.to_owned(), target_kind(*kind), item.clone()))
            })
            .ok()
            .flatten();
        if let Some((root, kind, item)) = item {
            let verdict = if ended {
                Verdict::Failed
            } else {
                Verdict::Owed {
                    attention: None,
                    catching_up: false,
                }
            };
            self.schedule.retire_settled(&root, kind, &item, &verdict);
        }
    }

    /// The retirement `id` was answered: its entry ends unless it is Owed
    /// (R-31), the refused set hears it, an answer that got past
    /// authentication proves the repository's key and host (C3), and one
    /// that needs the person becomes attention (E3).
    fn retirement_answered(&mut self, id: DesktopRequestId, stopped: bool, cx: &mut Context<Self>) {
        let seen = self
            .operations
            .read_with(cx, |operations, _| {
                let request = operations.request(id)?;
                let Intent::ContextRetire {
                    root,
                    kind,
                    id: item,
                } = &request.intent
                else {
                    return None;
                };
                Some((
                    request.request_id,
                    root.to_str()?.to_owned(),
                    target_kind(*kind),
                    item.clone(),
                    operations.retirement(id)?.clone(),
                ))
            })
            .ok()
            .flatten();
        let Some((request, root, kind, item, answer)) = seen else {
            // No answer to read: a loop waiting on it stops (R-37).
            return self.loop_lost(id, cx);
        };
        // The answer names the journal row too: an entry whose request was
        // answered before a progress event named it still gets its
        // operation, which Resume matches the pending row by (E4).
        if let Some(operation) = answer
            .operation
            .as_deref()
            .and_then(|operation| OperationId::parse(operation).ok())
        {
            self.store.retirement_operation(request, operation);
        }
        // The entry ends with the worker's record of the request (R-31). It
        // stays while the request is Owed: Resume needs the item.
        let record = self
            .worker
            .record_state(id)
            .map_or(RetirementEnd::Finished, RetirementEnd::from);
        let end = if matches!(answer.verdict, Verdict::Owed { .. }) {
            RetirementEnd::Accepted
        } else {
            record
        };
        self.store.retirement_ended(request, end);
        self.schedule
            .retirement_answered(&root, kind, &item, &answer.verdict);
        // The attempt's entry says what came of it (E3).
        self.schedule
            .retire_settled(&root, kind, &item, &answer.verdict);
        if answer.authenticated() {
            self.checked(&root, cx);
        }
        // The loop goes on to its next item, or stops (RT-7 to RT-9).
        self.loop_answered(id, &root, request, &answer.verdict, stopped, cx);
        if let Some(need) = answer.verdict.need() {
            self.retirement_needs(id, root, need, cx);
        }
        cx.notify();
    }

    /// The repository waits for `need` (E3). The key is the one the
    /// background provider refused, or the user-lane request asked for, else
    /// the repository's selected key, read off the UI thread.
    fn retirement_needs(
        &mut self,
        id: DesktopRequestId,
        root: Root,
        need: Need,
        cx: &mut Context<Self>,
    ) {
        let key = recorded_key(&self.worker.refused_keys(id), &self.worker.asked_keys(id));
        if key.is_some() || !matches!(need, Need::KeyLocked | Need::KeyProblem) {
            return self.raise(root, need, key, cx);
        }
        let backend = self.backend.clone();
        cx.spawn(async move |this, cx| {
            let key = cx
                .background_executor()
                .spawn(async move { backend.selected_key() })
                .await;
            let _ = this.update(cx, |scheduler, cx| scheduler.raise(root, need, key, cx));
        })
        .detach();
    }

    pub(super) fn raise(
        &mut self,
        root: Root,
        need: Need,
        key: Option<KeyId>,
        cx: &mut Context<Self>,
    ) {
        let host = self.authorities.get(&root).cloned().flatten();
        let reason = reason_of(need, &root, key, host);
        // A loop that began while the key was being read ends with the
        // repository's wait (RT-8, R-32): it must not send into it. It ends
        // even when `schedule.raise` returns false: a loop cannot coexist
        // with a poll in flight (R-32), so that is intended.
        self.end_loop(&root, cx);
        if self.schedule.raise(&root, reason) {
            self.publish(cx);
            self.tick(cx);
        }
    }
}
