//! The mutation boundary: how a front end's request to change Manyhands
//! state is identified, recorded and answered.
//!
//! This holds what identifies a request and the records that link a
//! request ID to its command, target, input digest, operations and result.
//! The records are not a journal: they hold no step, no lease and no
//! authority over Git. Losing them loses the ability to answer a retry
//! from memory, never the ability to work.

// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use time::OffsetDateTime;

use crate::results::{
    Effects, Envelope, ResultCode, Scope, absolute_path_string, timestamp_string,
};

use super::{
    FailurePoint, OperationFamily, OperationId, ReadError, RepositoryOperation, RepositoryService,
    keys,
    keys::{SessionCredentialProvider, SessionCredentials},
    recovery::{self, FinalKind, JournalRow},
    remote,
};

mod bind;
mod dto;
mod evidence;
mod identity;
mod observe;
mod outcome;
mod records;

pub use bind::ticket::{TicketCreateInput, TicketSaveInput};
use bind::{Accepted, Answer, Binding, Ran, Standing, ticket::TicketBinding};
pub use dto::{
    MutationDataDto, RequestDto, RequestOperationDto, RequestResultDto, RequestState,
    TicketMutationDto,
};
use evidence::Position;
pub use identity::{ConfirmationId, ConfirmationIdParseError, RequestId, RequestIdParseError};
use identity::{DigestSalt, IntentDigest, ScopeKey};
pub(super) use records::migrate as migrate_request_tables;
use records::{InsertRequestOutcome, NewRequest, RequestOperation, RequestRecord, RequestResult};

/// What the service reads the time from when it times a record. Tests
/// inject one to move time; everything else uses `SystemClock`.
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

/// The system's clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl RepositoryService {
    /// The same service, timing its records by `clock`.
    #[doc(hidden)]
    pub fn with_clock_for_testing(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Where the operation `operation_id` stands in the journal of its
    /// family: absent, pending in a state or phase, or final in some way
    /// and owing work or not. `JournalRow::in_flight` is what settling asks.
    ///
    /// A local or remote operation is looked up under the root `scope`
    /// names; a key-material operation belongs to the application,
    /// whatever `scope` is. The lookup reads the index only. It needs no
    /// registration and no repository on disk, so it answers for a root
    /// that is not registered and for one that no longer exists.
    ///
    /// This is what settling a request asks, and not the display read
    /// `show_operation`: a local row closed as `rejected` is absent here,
    /// and a remote operation that was interrupted or that failed is
    /// pending.
    #[doc(hidden)]
    pub fn journal_row(
        &self,
        family: OperationFamily,
        scope: &ScopeKey,
        operation_id: OperationId,
    ) -> Result<JournalRow, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| match family {
            OperationFamily::Local => {
                recovery::lookup_operation(connection, scope.as_str(), operation_id)
            }
            OperationFamily::Remote => {
                remote::state::lookup_operation(connection, scope.as_str(), operation_id)
            }
            OperationFamily::KeyMaterial => {
                keys::lookup_material_operation(connection, operation_id)
            }
        })
    }
}

/// The command `show_request` answers.
const REQUEST_SHOW: &str = "request show";

/// A time a record holds, in seconds. One that cannot be written as a
/// timestamp was not written by any record function.
fn stored_time(seconds: i64) -> Result<String, ReadError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(timestamp_string)
        .ok_or_else(|| ReadError::new(ResultCode::InternalError))
}

/// What `request show` reports for a record, and the scope it is about.
fn request_dto(record: RequestRecord) -> Result<(Scope, RequestDto), ReadError> {
    let scope = Scope {
        repository: record.scope.repository().map(str::to_owned),
        ..Scope::default()
    };
    let dto = RequestDto {
        request_id: record.request_id.to_string(),
        state: record.state,
        command: record.command,
        accepted_at: stored_time(record.accepted_at)?,
        finished_at: record.finished_at.map(stored_time).transpose()?,
        operations: record
            .operations
            .into_iter()
            .map(|operation| RequestOperationDto {
                family: operation.family,
                operation_id: operation.operation_id.to_string(),
            })
            .collect(),
        result: record.result.map(|result| RequestResultDto {
            outcome: result.outcome,
            code: result.code,
            message: result.code.message().to_owned(),
            effects: result.effects,
            data: result.data,
        }),
    };
    Ok((scope, dto))
}

impl RepositoryService {
    /// `request show`: what the records hold for a request ID. Its state,
    /// its operation IDs and, once it is finished, its stored result.
    ///
    /// This is the read a client that lost its output uses to inspect
    /// recovery. It reads the records only: nothing is observed, resumed
    /// or changed, and a request whose record was deleted or never
    /// written is `request_not_found` whatever it did. A successful
    /// envelope reports the read; what the request came to is in
    /// `data.result`.
    pub fn show_request(&self, request_id: RequestId) -> Envelope<RequestDto> {
        let shown = self.request_record(request_id).and_then(|record| {
            record
                .map(request_dto)
                .transpose()?
                .ok_or_else(|| ReadError::new(ResultCode::RequestNotFound))
        });
        match shown {
            Ok((scope, dto)) => Envelope::read_success(REQUEST_SHOW, scope, dto),
            Err(error) => error.to_envelope(REQUEST_SHOW),
        }
    }
}

/// One request to change Manyhands state: the caller's request ID, the
/// confirmation it presents, if any, and the typed mutation.
pub struct MutationCall {
    pub request_id: RequestId,
    pub confirmation: Option<ConfirmationId>,
    pub mutation: Mutation,
}

/// The mutations a front end can ask for: one variant for each bound
/// command, holding that command's typed input. The variant fixes the
/// envelope's `command`.
pub enum Mutation {
    TicketCreate(TicketCreateInput),
    TicketSave(TicketSaveInput),
}

impl Mutation {
    /// The command, as the envelope names it.
    pub fn command(&self) -> &'static str {
        match self {
            Self::TicketCreate(_) => bind::ticket::TICKET_CREATE,
            Self::TicketSave(_) => bind::ticket::TICKET_SAVE,
        }
    }

    fn into_binding(self) -> Box<dyn Binding> {
        match self {
            Self::TicketCreate(input) => Box::new(TicketBinding::from(input)),
            Self::TicketSave(input) => Box::new(TicketBinding::from(input)),
        }
    }
}

/// How a caller asks a running request to stop. A request that is
/// cancelled before it is accepted does nothing.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Where a request reports its progress. No command reports any yet; the
/// events are added with the commands that have stages to report.
pub trait ProgressSink {}

/// A sink for a caller that wants no progress.
pub struct NoProgress;

impl ProgressSink for NoProgress {}

/// A request as `execute` identifies it: what a record of it must hold to
/// be the same request.
struct RequestIdentity {
    request_id: RequestId,
    /// `None` when the root cannot be resolved to a canonical path: no
    /// record can be of this request then.
    scope: Option<ScopeKey>,
    digest: Option<IntentDigest>,
}

impl RequestIdentity {
    fn of(request_id: RequestId, binding: &dyn Binding) -> Self {
        let scope = ScopeKey::for_repository(binding.root());
        let digest = scope.as_ref().map(|scope| {
            binding
                .fields(IntentDigest::builder(
                    DigestSalt::Request(request_id),
                    binding.command(),
                    scope,
                    &binding.target(),
                ))
                .finish()
        });
        Self {
            request_id,
            scope,
            digest,
        }
    }

    /// The repository, as a result's scope names it.
    fn repository(&self, binding: &dyn Binding) -> Option<String> {
        match &self.scope {
            Some(scope) => scope.repository().map(str::to_owned),
            None => absolute_path_string(binding.root()),
        }
    }

    fn matches(&self, binding: &dyn Binding, record: &RequestRecord) -> bool {
        self.scope.as_ref() == Some(&record.scope)
            && record.command == binding.command()
            && record.target == binding.target()
            && self.digest == Some(record.intent_digest)
    }
}

/// What the envelope of one call is built with.
struct Reply<'a> {
    binding: &'a dyn Binding,
    identity: &'a RequestIdentity,
}

impl Reply<'_> {
    fn scope(&self, position: Option<&Position>) -> Scope {
        self.binding
            .scope(self.identity.repository(self.binding), position)
    }

    /// The envelope of an answer. The outcome is derived from the code and
    /// the effects; `retry` says the record was left accepted, so the same
    /// request finishes the work.
    fn envelope(
        &self,
        answer: Answer,
        accepted: Option<&Accepted>,
        retry: bool,
    ) -> Envelope<MutationDataDto> {
        let Answer {
            code,
            effects,
            data,
            recovery,
        } = answer;
        let scope = self.scope(accepted.map(|accepted| &accepted.position));
        let recovery = if recovery.is_empty() {
            outcome::recovery(
                code,
                scope.repository.as_deref(),
                self.identity.request_id,
                accepted.map(|accepted| accepted.operation_id),
                retry,
            )
        } else {
            recovery
        };
        let mut envelope = Envelope::classified_mutation(
            self.binding.command(),
            scope,
            code == ResultCode::Cancelled,
            code,
            effects,
            false,
        )
        .with_request_id(self.identity.request_id.to_string())
        .with_recovery(recovery);
        if let Some(accepted) = accepted {
            envelope = envelope.with_operation_id(accepted.operation_id.to_string());
        }
        match data {
            Some(data) => envelope.with_data(data),
            None => envelope,
        }
    }

    /// A request that stopped before it was accepted, having done nothing.
    fn stopped(&self, code: ResultCode) -> Envelope<MutationDataDto> {
        self.envelope(Answer::stopped(code), None, false)
    }

    fn read_failure(&self, error: &ReadError) -> Envelope<MutationDataDto> {
        self.envelope(Answer::of_read(error), None, false)
    }
}

impl RepositoryService {
    /// Runs one mutation under the caller's request ID and returns its
    /// result. Every handled outcome, success or not, is an envelope.
    ///
    /// The request ID decides what the call is. A request seen for the
    /// first time is validated, checked, accepted and run once. One whose
    /// record is finished gets the stored result again, whatever has
    /// happened to its target since, and nothing is written. The same ID
    /// with another command, target or input is `request_mismatch`, and
    /// nothing runs.
    ///
    /// The call is synchronous and runs on the caller's thread. It takes
    /// no lease and holds no lock between its steps: every Git or
    /// canonical effect is made by the domain operation the binding calls,
    /// under that operation's own lease and journal row.
    pub fn execute<P: SessionCredentialProvider>(
        &self,
        call: MutationCall,
        // No bound command needs a credential, reports a stage or can be
        // stopped once it is accepted; the commands that do use these.
        _credentials: &mut SessionCredentials<P>,
        _progress: &mut dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Envelope<MutationDataDto> {
        let MutationCall {
            request_id,
            confirmation,
            mutation,
        } = call;
        let mut binding = mutation.into_binding();
        let identity = RequestIdentity::of(request_id, binding.as_ref());
        // Two processes that submit the same new request ID race on the
        // record's key. The one that loses finds a record on its second
        // look and is answered from it.
        for _ in 0..2 {
            let record = match self.request_record(request_id) {
                Ok(record) => record,
                Err(error) => {
                    return Reply {
                        binding: binding.as_ref(),
                        identity: &identity,
                    }
                    .read_failure(&error);
                }
            };
            match record {
                Some(record) => {
                    let reply = Reply {
                        binding: binding.as_ref(),
                        identity: &identity,
                    };
                    if !identity.matches(binding.as_ref(), &record) {
                        return self.mismatched(&reply, &record);
                    }
                    match record.state {
                        RequestState::Finished => return self.replay(&reply, record),
                        RequestState::Accepted => {
                            // `None` when another call settled the record
                            // meanwhile: the next look answers from what
                            // it left.
                            if let Some(envelope) =
                                self.reenter(binding.as_mut(), &identity, &record)
                            {
                                return envelope;
                            }
                        }
                    }
                }
                None => {
                    if let Some(envelope) =
                        self.first_call(binding.as_mut(), &identity, confirmation, cancel)
                    {
                        return envelope;
                    }
                }
            }
        }
        Reply {
            binding: binding.as_ref(),
            identity: &identity,
        }
        .stopped(ResultCode::Busy)
    }

    /// A request no record holds: validate, check, accept, run, settle.
    /// `None` when another call accepted the same request ID meanwhile.
    fn first_call(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        confirmation: Option<ConfirmationId>,
        cancel: &CancellationToken,
    ) -> Option<Envelope<MutationDataDto>> {
        // Neither bound command takes a confirmation.
        if confirmation.is_some() {
            return Some(Reply { binding, identity }.stopped(ResultCode::NotConfirmable));
        }
        let prepared = match binding.prepare(self) {
            Ok(prepared) => prepared,
            Err(answer) => return Some(Reply { binding, identity }.envelope(answer, None, false)),
        };
        let reply = Reply { binding, identity };
        // `prepare` resolved the repository, so the root has a scope key.
        let (Some(scope), Some(digest)) = (identity.scope.clone(), identity.digest) else {
            return Some(reply.stopped(ResultCode::InternalError));
        };
        if cancel.is_cancelled() {
            return Some(reply.stopped(ResultCode::Cancelled));
        }
        let accepted = Accepted {
            request_id: identity.request_id,
            attempt: 1,
            scope: scope.clone(),
            family: prepared.family,
            // A new acceptance gets a new operation ID, so the journal's
            // own mismatch check never reaches a caller.
            operation_id: OperationId::new(),
            position: prepared.position,
            expected: prepared.expected,
        };
        let inserted = self.insert_request(&NewRequest {
            request_id: accepted.request_id,
            scope,
            command: binding.command().to_owned(),
            target: binding.target(),
            intent_digest: digest,
            confirmation: None,
            base_ref: accepted.position.base_ref.clone(),
            base_oid: accepted.position.base_oid,
            expected_digest: accepted.expected.clone(),
            operations: vec![RequestOperation {
                family: accepted.family,
                operation_id: accepted.operation_id,
            }],
        });
        match inserted {
            Ok(InsertRequestOutcome::Inserted) => {}
            Ok(InsertRequestOutcome::RequestExists) => return None,
            // No confirmation was presented.
            Ok(InsertRequestOutcome::ConfirmationUnavailable) => {
                return Some(reply.stopped(ResultCode::InternalError));
            }
            Err(error) => return Some(reply.read_failure(&error)),
        }
        Some(self.run_accepted(&reply, &accepted))
    }

    /// Makes the binding's domain call for an accepted request and settles
    /// its record from what the call left.
    fn run_accepted(&self, reply: &Reply<'_>, accepted: &Accepted) -> Envelope<MutationDataDto> {
        self.called(reply, accepted, || reply.binding.run(self, accepted))
    }

    /// Makes a domain call, `call`, for an accepted request and settles its
    /// record from what the call left.
    ///
    /// The two fault points stand in for a process that died: at either,
    /// the call ends with its record exactly as it was, and what it
    /// returns is not a result of the request.
    fn called(
        &self,
        reply: &Reply<'_>,
        accepted: &Accepted,
        call: impl FnOnce() -> Ran,
    ) -> Envelope<MutationDataDto> {
        let died = |point| {
            self.dies_at(point, accepted)
                .then(|| reply.envelope(Answer::stopped(ResultCode::InternalError), None, false))
        };
        if let Some(envelope) = died(FailurePoint::BeforeRequestDomainCall) {
            return envelope;
        }
        // The cache guard is not held here: the record functions release
        // it before they return, and the domain call takes it itself.
        let ran = call();
        if let Some(envelope) = died(FailurePoint::BeforeRequestSettlement) {
            return envelope;
        }
        self.settled(reply, accepted, ran)
    }

    /// Whether a test has this call end at `point`.
    fn dies_at(&self, point: FailurePoint, accepted: &Accepted) -> bool {
        self.should_inject(
            point,
            RepositoryOperation::Read,
            Path::new(accepted.scope.as_str()),
        )
        .unwrap_or(false)
    }

    /// The envelope of what a call found or did, with its record settled
    /// from it.
    fn settled(
        &self,
        reply: &Reply<'_>,
        accepted: &Accepted,
        ran: Ran,
    ) -> Envelope<MutationDataDto> {
        let Ran { answer, standing } = ran;
        let settlement = Settlement::of(&standing);
        let envelope = reply.envelope(answer, Some(accepted), settlement == Settlement::Leave);
        self.settle(accepted, settlement, &envelope);
        envelope
    }

    /// Changes the record as `settlement` says. Every change is
    /// conditional on the record still being accepted with this call's
    /// attempt, so a call never settles a record another call has since
    /// entered. A change that fails leaves the record as it is: the next
    /// call of the request re-enters it.
    fn settle(
        &self,
        accepted: &Accepted,
        settlement: Settlement,
        envelope: &Envelope<MutationDataDto>,
    ) {
        match settlement {
            Settlement::Finish => {
                let data = match &envelope.data {
                    Some(data) => match serde_json::to_value(data) {
                        Ok(data) => Some(data),
                        // The record stays accepted and the request is
                        // answered again by re-entering it.
                        Err(_) => return,
                    },
                    None => None,
                };
                let _ = self.finish_request(
                    accepted.request_id,
                    accepted.attempt,
                    &RequestResult {
                        outcome: envelope.outcome,
                        code: envelope.code,
                        effects: envelope.effects.clone(),
                        data,
                    },
                );
            }
            Settlement::Delete => {
                let _ = self.delete_request(accepted.request_id, accepted.attempt);
            }
            Settlement::Leave => {}
        }
    }

    /// A finished record of this request: its stored result, unchanged.
    /// No domain call is made and nothing is written.
    ///
    /// The envelope is rebuilt from the record: the outcome, code, effects,
    /// commit and data it stored and the operation it names. The record
    /// matches this call's command, repository and target, and its
    /// recorded position gives the scope its branch. It stores no recovery
    /// action, and a finished ticket create or save has none.
    fn replay(&self, reply: &Reply<'_>, record: RequestRecord) -> Envelope<MutationDataDto> {
        let (Some(accepted), Some(result)) =
            (Accepted::of_record(&record, record.attempt), record.result)
        else {
            return reply.stopped(ResultCode::InternalError);
        };
        // A result that names a commit is given again only while that
        // commit can be reached from the branch the request committed to,
        // or from primary once that branch is gone.
        if let Some(commit) = result.effects.commit_oid.as_deref() {
            let reachable = git2::Oid::from_str(commit).is_ok_and(|commit| {
                record.scope.repository().is_some_and(|root| {
                    evidence::still_reachable(
                        Path::new(root),
                        accepted.position.base_ref.as_deref(),
                        commit,
                    )
                })
            });
            if !reachable {
                return reply.stopped(ResultCode::RecoveryRequired);
            }
        }
        let data = match result.data.as_ref() {
            Some(stored) => match MutationDataDto::from_stored(&record.command, stored) {
                Some(data) => Some(data),
                None => return reply.stopped(ResultCode::InternalError),
            },
            None => None,
        };
        // The one place an outcome is not derived: it is the stored one.
        let envelope = Envelope::mutation(
            record.command,
            reply.scope(Some(&accepted.position)),
            result.outcome,
            result.code,
        )
        .with_effects(result.effects)
        .with_request_id(record.request_id.to_string())
        .with_operation_id(accepted.operation_id.to_string());
        match data {
            Some(data) => envelope.with_data(data),
            None => envelope,
        }
    }

    /// A record exists and is not of this request: the same request ID
    /// with another repository, command, target or input. Nothing runs and
    /// the record is not changed.
    ///
    /// When the recorded request is unfinished and an attempt of it ran,
    /// the result reports what the repository shows of it, a commit to its
    /// path among it, and the outcome is `partial` when that is a durable
    /// effect. Otherwise it is an input error: a finished request's
    /// effects are its own result's, which `request show` gives, and are
    /// not reported as this call's.
    fn mismatched(&self, reply: &Reply<'_>, record: &RequestRecord) -> Envelope<MutationDataDto> {
        let effects = match record.state {
            RequestState::Accepted => bind::recorded_effects(self, record),
            RequestState::Finished => Effects::not_requested(),
        };
        reply.envelope(
            Answer {
                effects,
                ..Answer::stopped(ResultCode::RequestMismatch)
            },
            None,
            false,
        )
    }

    /// An accepted record matches this call: an earlier call of the
    /// request started and its end was not recorded. `None` when the
    /// record was settled between the look that found it and this.
    ///
    /// The call raises the record's attempt, so that it is the one call
    /// that may settle it: a call that entered before it and is still
    /// running reports what it did and changes nothing. The caller's
    /// token is not checked again. Then, in order:
    ///
    /// 1. The binding reads the operation's journal row and what Git shows
    ///    of the request's path since the recorded position. A row or a
    ///    repository that cannot be read answers the call and leaves the
    ///    record as it is.
    /// 2. When someone else has changed the path since, the request is not
    ///    run again: it is `external_change`, or, when its operation
    ///    completed and its own commit is in range beneath the change, it
    ///    is finished with that commit.
    /// 3. Otherwise the domain operation is called under the recorded
    ///    operation ID and expectation. This is the only way the work is
    ///    continued: only the domain completes its row and its hand-off.
    /// 4. The commit reported comes from the evidence read before and
    ///    after that call, and the record is settled as a first call's is.
    fn reenter(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        record: &RequestRecord,
    ) -> Option<Envelope<MutationDataDto>> {
        let attempt = match self.enter_request(record.request_id) {
            Ok(Some(attempt)) => attempt,
            Ok(None) => return None,
            Err(error) => return Some(Reply { binding, identity }.read_failure(&error)),
        };
        let Some(accepted) = Accepted::of_record(record, attempt) else {
            return Some(Reply { binding, identity }.stopped(ResultCode::InternalError));
        };
        let before = binding.reenter(self, &accepted);
        let reply = Reply { binding, identity };
        Some(match before {
            Ok(before) => self.called(&reply, &accepted, || {
                binding.rerun(self, &accepted, &before)
            }),
            Err(ran) => self.settled(&reply, &accepted, ran),
        })
    }
}

/// What a call does to its record once its domain call has returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Settlement {
    /// The result is stored and the record is finished.
    Finish,
    /// The record is deleted and the confirmation it accepted released:
    /// the request ID is free again.
    Delete,
    /// The record stays accepted: a retry continues the request.
    Leave,
}

impl Settlement {
    /// Settling is decided from what the binding's call left and from the
    /// operation's journal row, never from the kind of error the domain
    /// returned.
    ///
    /// For a request that stopped, a result the domain has made final for
    /// the operation is asked for before whether anything is in flight: a
    /// cancelled synchronization that still owes its index hand-off is in
    /// flight and is final all the same, and asking in the other order
    /// would leave its record accepted for good. A row that could not be
    /// read leaves the record as it is.
    fn of(standing: &Standing) -> Self {
        match standing {
            Standing::Final => Self::Finish,
            Standing::Owed | Standing::Unconfirmed => Self::Leave,
            Standing::Stopped { journal: None } => Self::Leave,
            Standing::NotRun { journal } => match journal {
                JournalRow::Absent => Self::Delete,
                JournalRow::Pending(_) | JournalRow::Final { .. } => Self::Leave,
            },
            Standing::Stopped {
                journal: Some(journal),
            } => match journal {
                JournalRow::Final {
                    kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                    owes_work: _,
                } => Self::Finish,
                JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work: _,
                }
                | JournalRow::Absent
                | JournalRow::Pending(_) => {
                    if journal.in_flight() {
                        Self::Leave
                    } else {
                        Self::Delete
                    }
                }
            },
        }
    }
}

/// What the boundary is built from: the intent digest, the request records
/// and the journal lookup. A front end uses the boundary itself; this is
/// public so that the integration tests can reach it.
#[doc(hidden)]
pub mod request_store {
    pub use super::{
        Clock, SystemClock,
        identity::{DigestSalt, FieldValue, IntentDigest, IntentDigestBuilder, ScopeKey},
        records::{
            ConfirmationRecord, InsertRequestOutcome, NewConfirmation, NewRequest,
            RequestOperation, RequestRecord, RequestResult,
        },
        recovery::{FinalKind, JournalRow, PendingOperation},
    };
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod settle_tests;
