//! What the request model keeps of retirements and of running operations
//! (D6 Part E, design E3 and R-23).
//!
//! A retirement's answer is classified once, when the request finishes
//! (`model::polling::retire`), and the model dismisses the request when
//! attention or the next attempt covers it. The model also knows which
//! operation each running request and each running automatic poll holds, so
//! the pending lists never show or count the worker's own work, and which
//! operation an answered request is about, so a request and its pending row
//! are one thing to the person.

use std::collections::{BTreeMap, HashMap, HashSet};

use manyhands::{
    repository::{ItemDtoKind, OperationDto, OperationId, OperationListDto, ProgressEvent},
    results::{Envelope, RecoveryActionKind},
};

use super::Operations;
use crate::desktop::{
    model::{
        list_state::ListState,
        polling::retire::{Retirement, classify},
        stamp::RepoKey,
    },
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, RequestState},
        worker::RecordState,
    },
};

/// An item whose local copy is being retired, by its repository, kind and ID
/// (decision 19): the "Retiring…" state is keyed by all three.
pub(crate) type RetiringKey = (RepoKey, ItemDtoKind, String);

/// The retirement answers and running operations, by request.
#[derive(Default)]
pub(super) struct Retirements {
    /// The items the loop (or a Retire…) has marked "Retiring…" and not yet
    /// handed to the worker, by mark: each mark is cleared by its own owner,
    /// so one ending never clears another's share (decision 19).
    marks: BTreeMap<u64, RetiringKey>,
    next_mark: u64,
    /// The items of the in-flight record's retirement entries, one per
    /// entry: a retirement left pending stays "Retiring…" after a relaunch.
    entries: Vec<RetiringKey>,
    /// Where the worker's record of a request stood when its last attempt
    /// finished, kept for the `Finished` event that follows (E3).
    records: HashMap<DesktopRequestId, RecordState>,
    /// Each finished retirement's answer, classified.
    answers: HashMap<DesktopRequestId, Retirement>,
    /// The operation each running request's first progress event named.
    running: HashMap<DesktopRequestId, String>,
    /// The operations of the automatic polls now queued or running (D6 B1).
    polls: HashSet<OperationId>,
}

impl Operations {
    /// Where the worker's record of the request `id` stands after its last
    /// attempt (`WorkerHandle::record_state`). The drain tells the model
    /// before the `Finished` event, which carries no record state.
    pub(crate) fn record_stood(&mut self, id: DesktopRequestId, record: RecordState) {
        self.retirements.records.insert(id, record);
    }

    /// The finished retirement `id`'s answer, classified.
    pub(crate) fn retirement(&self, id: DesktopRequestId) -> Option<&Retirement> {
        self.retirements.answers.get(&id)
    }

    /// The request `id` ended, or its first progress event named its
    /// operation.
    pub(super) fn progressed(&mut self, id: DesktopRequestId, event: &ProgressEvent) {
        if let Some(operation) = &event.operation_id {
            self.retirements
                .running
                .entry(id)
                .or_insert_with(|| operation.clone());
        }
    }

    pub(super) fn left_running(&mut self, id: DesktopRequestId) {
        self.retirements.running.remove(&id);
    }

    /// Classifies a finished retirement and keeps its answer; `None` for
    /// any other request. A request the model has no record state for was
    /// finished (a test that never asked the worker).
    pub(super) fn retired(
        &mut self,
        id: DesktopRequestId,
        envelope: &Envelope<manyhands::repository::MutationDataDto>,
    ) -> Option<Retirement> {
        if !self
            .request(id)
            .is_some_and(|request| matches!(request.intent, Intent::ContextRetire { .. }))
        {
            self.retirements.records.remove(&id);
            return None;
        }
        let record = self
            .retirements
            .records
            .remove(&id)
            .unwrap_or(RecordState::Finished);
        let answer = classify(record, envelope);
        self.retirements.answers.insert(id, answer.clone());
        Some(answer)
    }

    /// Marks `key` "Retiring…" in this very update (RT-6, E4 step 1). The
    /// mark is the caller's until it clears it.
    pub(crate) fn mark_retiring(&mut self, key: RetiringKey) -> u64 {
        self.retirements.next_mark += 1;
        let mark = self.retirements.next_mark;
        self.retirements.marks.insert(mark, key);
        mark
    }

    /// Clears one mark. True when it was held.
    pub(crate) fn unmark_retiring(&mut self, mark: u64) -> bool {
        self.retirements.marks.remove(&mark).is_some()
    }

    /// The items of the retirement entries the in-flight record holds,
    /// which the scheduler sets whenever they change. True when they did.
    pub(crate) fn set_retirement_entries(&mut self, entries: Vec<RetiringKey>) -> bool {
        if self.retirements.entries == entries {
            return false;
        }
        self.retirements.entries = entries;
        true
    }

    /// Whether a mark or an in-flight entry holds the item "Retiring…". The
    /// worker's own queued or running retirement is asked of the worker
    /// (`Services::retiring`).
    #[cfg_attr(not(test), allow(dead_code))] // Called from the "Retiring…" lock.
    pub(crate) fn is_retiring(&self, root: &str, kind: ItemDtoKind, id: &str) -> bool {
        let held = |key: &RetiringKey| key.0 == root && key.1 == kind && key.2 == id;
        self.retirements.marks.values().any(held) || self.retirements.entries.iter().any(held)
    }

    /// The operations of the automatic polls now queued or running, which
    /// the scheduler sets whenever they change. True when they changed.
    pub(crate) fn set_running_polls(&mut self, polls: HashSet<OperationId>) -> bool {
        if self.retirements.polls == polls {
            return false;
        }
        self.retirements.polls = polls;
        true
    }

    /// Whether the pending row `row` is the worker's own running work: the
    /// operation a running request's progress named, or a running automatic
    /// poll's. It is in progress, not pending, so it is neither shown nor
    /// counted (R-23, E3).
    pub(crate) fn is_running(&self, row: &OperationDto) -> bool {
        let Some(operation) = row.operation_id.as_deref() else {
            return false;
        };
        self.retirements
            .running
            .iter()
            .any(|(id, running)| running == operation && self.is_active(*id))
            || OperationId::parse(operation)
                .is_ok_and(|operation| self.retirements.polls.contains(&operation))
    }

    fn is_active(&self, id: DesktopRequestId) -> bool {
        self.request(id)
            .is_some_and(|request| matches!(request.state, RequestState::Running { .. }))
    }

    /// Each listed repository's pending operations as Activity shows them:
    /// without the worker's own running work (R-23, E3).
    pub(crate) fn pending_shown(&self) -> Vec<(RepoKey, ListState<OperationListDto>)> {
        self.pending()
            .iter()
            .map(|(root, state)| {
                let shown = match state {
                    ListState::Ready {
                        rows,
                        complete,
                        index,
                    } => ListState::Ready {
                        rows: OperationListDto {
                            items: rows
                                .items
                                .iter()
                                .filter(|row| !self.is_running(row))
                                .cloned()
                                .collect(),
                            complete: rows.complete,
                        },
                        complete: *complete,
                        index: index.clone(),
                    },
                    other => other.clone(),
                };
                (root.clone(), shown)
            })
            .collect()
    }

    /// The operation an answered request is about, when its answer says:
    /// the retirement's journal row, or the pending operation of its own
    /// that its recovery actions offer to resume or abandon. A request and
    /// the pending row of one operation are counted once (R-23).
    pub(crate) fn operation_of(&self, request: &DesktopRequest) -> Option<String> {
        if let Some(operation) = self
            .retirements
            .answers
            .get(&request.id)
            .and_then(|answer| answer.operation.clone())
        {
            return Some(operation);
        }
        let RequestState::Finished(_, _, _, recovery, _) = &request.state else {
            return None;
        };
        recovery
            .iter()
            .filter(|action| {
                matches!(
                    action.action,
                    RecoveryActionKind::OperationAbandon | RecoveryActionKind::OperationResume
                )
            })
            .find_map(|action| action.operation_id.clone())
    }
}
