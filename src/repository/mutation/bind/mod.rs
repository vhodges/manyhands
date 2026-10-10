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
        recovery::JournalRow,
    },
    dto::MutationDataDto,
    evidence::Position,
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
}

/// A domain call's result and what it leaves.
pub(crate) struct Ran {
    pub(crate) answer: Answer,
    pub(crate) standing: Standing,
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
}

/// Whether the repository shows a change by the unfinished request
/// `record` holds: its operation is in flight and its file is no longer
/// what it was before the request. The original input is not needed, and
/// is not available when the request is being reused with another.
pub(crate) fn recorded_change(service: &RepositoryService, record: &RequestRecord) -> bool {
    let Some(accepted) = Accepted::of_record(record, record.attempt) else {
        return false;
    };
    if !accepted
        .journal_row(service)
        .is_none_or(|row| row.in_flight())
    {
        return false;
    }
    if record.command == ticket::TICKET_CREATE || record.command == ticket::TICKET_SAVE {
        ticket::recorded_change(record)
    } else {
        false
    }
}
