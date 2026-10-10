//! Bindings: what connects one command to the boundary. A binding names
//! the command, its target and the fields of its identity, makes the
//! checks that need no lease, and makes the one domain call.
//!
//! The boundary does the rest for every binding alike: it looks the
//! request up, answers a finished one from its record, rejects a changed
//! one, accepts a new one and settles its record.

use std::path::Path;

use crate::results::{Effects, RecoveryAction, ResultCode, Scope};

use super::{
    super::{
        ExpectedPathObservation, OperationFamily, OperationId, ReadError, RepositoryService,
        recovery::{FinalKind, JournalRow},
    },
    dto::MutationDataDto,
    evidence::{PathEvidence, Position},
    identity::{IntentDigestBuilder, RequestId, ScopeKey},
    records::RequestRecord,
};

pub(crate) mod ticket;

/// What a request is answered with, before the boundary classifies it: the
/// outcome is derived from the code and the effects, never chosen here.
pub(crate) struct Answer {
    pub(crate) code: ResultCode,
    pub(crate) effects: Effects,
    pub(crate) data: Option<MutationDataDto>,
    /// The recovery a read suggested for its own failure. The boundary
    /// adds the actions that follow from the code.
    pub(crate) recovery: Vec<RecoveryAction>,
}

impl Answer {
    /// A request that stopped having done nothing.
    pub(crate) fn stopped(code: ResultCode) -> Self {
        Self {
            code,
            effects: Effects::not_requested(),
            data: None,
            recovery: Vec::new(),
        }
    }

    /// A check's read failed: the request is answered with the read's
    /// code and the recovery it suggests.
    pub(crate) fn of_read(error: &ReadError) -> Self {
        Self {
            recovery: error.recovery.clone(),
            ..Self::stopped(error.code())
        }
    }

    pub(crate) fn with_data(mut self, data: MutationDataDto) -> Self {
        self.data = Some(data);
        self
    }
}

/// What a binding's checks give the boundary to accept a request with.
pub(crate) struct Prepared {
    /// The journal the binding's domain operation is recorded in.
    pub(crate) family: OperationFamily,
    pub(crate) position: Position,
    /// The digest of the canonical file's bytes as they were before the
    /// request, which the domain operation is given as its expectation.
    pub(crate) expected: Option<ExpectedPathObservation>,
}

/// An accepted request, as the call that is running it holds it. A first
/// call makes one from what it inserted; a call that re-enters an accepted
/// record makes one from the record and the attempt it raised.
pub(crate) struct Accepted {
    pub(crate) request_id: RequestId,
    /// This call's attempt. Every change to the record is conditional on
    /// it still being the record's.
    pub(crate) attempt: u64,
    pub(crate) scope: ScopeKey,
    pub(crate) family: OperationFamily,
    pub(crate) operation_id: OperationId,
    pub(crate) position: Position,
    pub(crate) expected: Option<ExpectedPathObservation>,
}

impl Accepted {
    /// An accepted record, entered as `attempt`. `None` for a record with
    /// no operation, which no binding accepts.
    pub(crate) fn of_record(record: &RequestRecord, attempt: u64) -> Option<Self> {
        let operation = record.operations.first()?;
        Some(Self {
            request_id: record.request_id,
            attempt,
            scope: record.scope.clone(),
            family: operation.family,
            operation_id: operation.operation_id,
            position: Position {
                base_ref: record.base_ref.clone(),
                base_oid: record.base_oid,
            },
            expected: record.expected_digest.clone(),
        })
    }

    /// Where the request's operation stands in its journal. `None` when
    /// the lookup itself failed.
    pub(crate) fn journal_row(&self, service: &RepositoryService) -> Option<JournalRow> {
        service
            .journal_row(self.family, &self.scope, self.operation_id)
            .ok()
    }
}

/// What is left of a request once its domain call has returned.
pub(crate) enum Standing {
    /// The request's end state holds: the result is stored.
    Final,
    /// The domain returned a result and says work remains that a retry
    /// continues: the record stays accepted.
    Owed,
    /// The request stopped. Whether anything is in flight is the journal
    /// row's to say: `journal` is the row as the binding read it after the
    /// call, or `None` when it could not be read.
    Stopped { journal: Option<JournalRow> },
    /// Git could not be read to say what the request committed: after a
    /// domain call that returned a result, or on re-entry before any call.
    /// Nothing is stored and nothing is deleted: a result finished now
    /// would be a no-op for a save that may have committed. The record
    /// stays accepted for a retry to settle.
    Unconfirmed,
    /// A re-entry stopped before its domain call, for a reason that says
    /// nothing of what an earlier attempt did. `journal` is the row as it
    /// was read: when no attempt had started, nothing is in flight and the
    /// record is deleted; otherwise the request's result is still owed and
    /// the record stays accepted, also when the row has completed.
    NotRun { journal: JournalRow },
}

/// A domain call's result and what it leaves.
pub(crate) struct Ran {
    pub(crate) answer: Answer,
    pub(crate) standing: Standing,
}

/// What re-entry reads before it makes its domain call: where the
/// request's operation stood in its journal, and what Git showed of the
/// request's path. The same two are what the commit rule needs after the
/// call, because the row's state after the call proves nothing: a repeat
/// resets a rejected row before the domain runs.
pub(crate) struct Before {
    pub(crate) journal: JournalRow,
    pub(crate) evidence: PathEvidence,
}

/// What re-entry does about what it read before the domain call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Found {
    /// The request's work is done and nothing is in flight: its operation
    /// completed and its commit is in range. Nothing remains for the domain
    /// to complete, and calling it again could only be refused for what
    /// has happened to the file since. The request is not run again; its
    /// commit is reported, and a later change by someone else, committed
    /// or not, is left as it is.
    Done { commit: git2::Oid },
    /// A commit in range changed the path to something the request did
    /// not intend, and the request's work is not known to be done: the
    /// answer is `external_change` and the request is not run again.
    /// `own` is the commit an earlier attempt of the request made before
    /// that, when one had started and made one.
    Foreign { own: Option<git2::Oid> },
    /// Nothing stands in the way: the domain operation is called.
    Continue,
}

impl Before {
    /// Whether an earlier attempt of the request had started: its
    /// operation's journal row was pending, or final and not rejected. A
    /// rejected row reads as absent.
    pub(crate) fn started(&self) -> bool {
        match self.journal {
            JournalRow::Absent => false,
            JournalRow::Pending(_) | JournalRow::Final { .. } => true,
        }
    }

    /// Whether an earlier attempt of the request may have committed: its
    /// operation's journal row was pending, or was final having recorded a
    /// step of its work. A row that completed with none never reached a
    /// checkpoint, and no commit is that request's, whatever Git shows.
    pub(crate) fn committing(&self) -> bool {
        match self.journal {
            JournalRow::Absent => false,
            JournalRow::Pending(_) => true,
            JournalRow::Final { checkpointed, .. } => checkpointed,
        }
    }

    pub(crate) fn found(&self) -> Found {
        // A commit of the intended content is the request's own only when
        // an attempt of the request may have committed.
        let own = self.evidence.own().filter(|_| self.committing());
        match (&self.journal, own) {
            (
                JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work: false,
                    checkpointed: _,
                },
                Some(commit),
            ) => Found::Done { commit },
            (
                JournalRow::Absent
                | JournalRow::Pending(_)
                | JournalRow::Final {
                    kind:
                        FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                    owes_work: _,
                    checkpointed: _,
                },
                own,
            ) => {
                if self.evidence.superseded() {
                    Found::Foreign { own }
                } else {
                    Found::Continue
                }
            }
        }
    }

    /// The commit a call reports, from the evidence read after its domain
    /// call.
    pub(crate) fn reported(&self, after: &PathEvidence) -> Option<git2::Oid> {
        after.reported(self.committing(), &self.evidence)
    }
}

pub(crate) trait Binding {
    /// The command, as the envelope names it.
    fn command(&self) -> &'static str;

    /// The repository root as the caller gave it.
    fn root(&self) -> &Path;

    /// The request's target within its repository.
    fn target(&self) -> String;

    /// Adds the binding's typed inputs to the intent digest, in a fixed
    /// order, each under its name, with absent, null and empty kept
    /// distinct. A body is fed here and stored nowhere.
    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder;

    /// The scope of the request's results. `position` is the recorded
    /// position once the request is accepted; before that the result names
    /// no branch or worktree, since none may exist.
    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope;

    /// Validates the input and makes the checks that need no lease. An
    /// `Err` answers the request without accepting it: nothing is
    /// recorded and no domain call is made.
    fn prepare(&mut self, service: &RepositoryService) -> Result<Prepared, Answer>;

    /// Makes the domain call for an accepted request, under the recorded
    /// operation ID and expectation, and reports what the repository shows
    /// of it afterwards.
    fn run(&self, service: &RepositoryService, accepted: &Accepted) -> Ran;

    /// Re-entry, before the domain call: an earlier call of the request
    /// started and its end was not recorded. Sets what `prepare` worked
    /// out for the domain call, without the checks a retry skips: the
    /// caller's token described the file before the first attempt, whose own
    /// write is allowed to have changed it. Then reads the journal row and
    /// the evidence the rules need.
    ///
    /// An `Err` answers the request without a domain call.
    fn reenter(&mut self, service: &RepositoryService, accepted: &Accepted) -> Result<Before, Ran>;

    /// Re-entry, the domain call: made with the recorded operation ID and
    /// the recorded expectation, which is the only way the work is
    /// continued. The commit is taken from the evidence read before and
    /// after the call, never from what the domain names.
    fn rerun(&self, service: &RepositoryService, accepted: &Accepted, before: &Before) -> Ran;
}

/// The effects the repository shows of the unfinished request `record`
/// holds. The original input is not needed, and is not available when the
/// request is being reused with another.
///
/// An earlier attempt ran when the operation's journal row is pending, or
/// final and not rejected: anything but absent. A completed row counts,
/// since a call can commit and complete its row and still fail to settle
/// its record. Then the effects are what Git and the item's context show.
/// With no row, no attempt reached the domain and there is no effect. A
/// row that could not be read is taken to be there: the effects still
/// have to be shown by the repository.
pub(crate) fn recorded_effects(service: &RepositoryService, record: &RequestRecord) -> Effects {
    let Some(accepted) = Accepted::of_record(record, record.attempt) else {
        return Effects::not_requested();
    };
    let ran = match accepted.journal_row(service) {
        Some(JournalRow::Absent) => false,
        Some(JournalRow::Pending(_) | JournalRow::Final { .. }) | None => true,
    };
    if ran && (record.command == ticket::TICKET_CREATE || record.command == ticket::TICKET_SAVE) {
        ticket::recorded_effects(record, &accepted)
    } else {
        Effects::not_requested()
    }
}

#[cfg(test)]
#[path = "before_tests.rs"]
mod before_tests;
