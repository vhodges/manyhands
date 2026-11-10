//! What the scheduler does for a retirement request (D6 Part E, design E2,
//! E3, R-31): the in-flight entry written before the request is submitted
//! and ended when the request ends, and the answer's attention, refused set
//! and proof of authentication.
//!
//! The loop that chooses the items (Task 11b) and the Retire… action (Task
//! 12) submit through [`Scheduler::retire`]. A retirement's answer reaches
//! [`Scheduler::retiring`] from the drain.

use std::path::PathBuf;

use gpui_kit::Context;
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, RequestId},
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
    /// which ends the entry.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
    pub(crate) fn retire(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        lane: Lane,
    ) -> DesktopRequestId {
        let request = RequestId::new();
        self.resend_retirement(request, root, kind, item, lane)
    }

    /// Submits a retirement under the request ID `request`: the original of
    /// a pending one that Resume names (E4), or a new one. An entry already
    /// held for the request, with the operation it learned, is kept.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
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
        begin_retirement(&mut self.store, request, root, kind, item, || {
            worker.submit_with_request_id(intent, lane, request)
        })
    }

    /// A retirement request's progress, answer or end (the drain, E3).
    pub(super) fn retiring(&mut self, event: Retiring, cx: &mut Context<Self>) {
        match event {
            Retiring::Operation { request, operation } => {
                self.store.retirement_operation(request, operation);
            }
            Retiring::Gone { request, end } => self.store.retirement_ended(request, end),
            Retiring::Answered(id) => self.retirement_answered(id, cx),
        }
    }

    /// The retirement `id` was answered: its entry ends unless it is Owed
    /// (R-31), the refused set hears it, an answer that got past
    /// authentication proves the repository's key and host (C3), and one
    /// that needs the person becomes attention (E3).
    fn retirement_answered(&mut self, id: DesktopRequestId, cx: &mut Context<Self>) {
        let seen = self
            .operations
            .read_with(cx, |operations, _| {
                let request = operations.request(id)?;
                let Intent::ContextRetire { root, id: item, .. } = &request.intent else {
                    return None;
                };
                Some((
                    request.request_id,
                    root.to_str()?.to_owned(),
                    item.clone(),
                    operations.retirement(id)?.clone(),
                ))
            })
            .ok()
            .flatten();
        let Some((request, root, item, answer)) = seen else {
            return;
        };
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
            .retirement_answered(&root, &item, &answer.verdict);
        if answer.authenticated() {
            self.checked(&root, cx);
        }
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

    fn raise(&mut self, root: Root, need: Need, key: Option<KeyId>, cx: &mut Context<Self>) {
        let host = self.authorities.get(&root).cloned().flatten();
        let reason = reason_of(need, &root, key, host);
        if self.schedule.raise(&root, reason) {
            self.publish(cx);
            self.tick(cx);
        }
    }
}
