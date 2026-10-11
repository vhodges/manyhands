//! The mutation boundary: how a front end's request to change Manyhands
//! state is identified, recorded and answered.
//!
//! This holds what identifies a request and the records that link a
//! request ID to its command, target, input digest, operations and result.
//! The records are not a journal: they hold no step, no lease and no
//! authority over Git. Losing them loses the ability to answer a retry
//! from memory, never the ability to work.
//!
//! This file is the boundary's public surface: the types a front end
//! calls it with, the reads of a request and of a journal row, and what
//! tests inject. The work of a call is in the modules below it:
//!
//! - `execute` looks a request up, decides what the call is, and accepts
//!   and runs a new one;
//! - `replay` answers from a record without running anything;
//! - `reenter` takes up an accepted record again;
//! - `settle` finishes, deletes or leaves the record afterwards;
//! - `reply` identifies the request and builds its envelopes;
//! - `bind` holds what connects each command to all of that.

// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use time::OffsetDateTime;

use crate::results::{Envelope, ResultCode, Scope, timestamp_string};

use super::{
    OperationFamily, OperationId, ReadError, RepositoryOperation, RepositoryService, keys,
    recovery::{self, JournalRow},
    remote,
};

mod bind;
mod command;
mod context;
mod dto;
mod evidence;
mod execute;
mod identity;
mod observe;
mod outcome;
mod records;
mod reenter;
mod replay;
mod reply;
mod settle;

use bind::{
    Binding,
    document::DocumentBinding,
    index::IndexBinding,
    remote::{RemoteAddBinding, RemoteSelectBinding},
    ticket::{SlugAssignBinding, TicketBinding},
};
pub use bind::{
    document::{DocumentCreateInput, DocumentMoveInput, DocumentSaveInput},
    index::{IndexRebuildInput, IndexRefreshInput},
    remote::{RemoteAddInput, RemoteSelectInput},
    ticket::{TicketCreateInput, TicketSaveInput, TicketSlugAssignInput},
};
use command::Command;
pub use dto::{
    DocumentMutationDto, MutationDataDto, RemoteMutationDto, RequestDto, RequestOperationDto,
    RequestResultDto, RequestState, TicketMutationDto,
};
use identity::ScopeKey;
pub use identity::{ConfirmationId, ConfirmationIdParseError, RequestId, RequestIdParseError};
pub(crate) use observe::absent_token;
use records::RequestRecord;
pub(super) use records::migrate as migrate_request_tables;

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

    /// Has `hook` run once in the mutation boundary, immediately before
    /// the next domain call an `execute` on this service makes: after the
    /// request is accepted, or, for a request entered again, after its
    /// journal row and evidence are read.
    #[doc(hidden)]
    pub fn set_request_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self.request_hook.lock().expect("test request hook lock") = Some(Box::new(hook));
    }

    fn run_request_hook(&self) {
        let hook = self
            .request_hook
            .lock()
            .expect("test request hook lock")
            .take();
        if let Some(hook) = hook {
            hook();
        }
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

impl RepositoryService {
    /// The step a local operation that has completed recorded of its own
    /// work; see `recovery::completed_step`.
    pub(crate) fn completed_local_step(
        &self,
        scope: &ScopeKey,
        operation_id: OperationId,
    ) -> Result<Option<String>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            recovery::completed_step(connection, scope.as_str(), operation_id)
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
    DocumentCreate(DocumentCreateInput),
    DocumentSave(DocumentSaveInput),
    DocumentMove(DocumentMoveInput),
    TicketCreate(TicketCreateInput),
    TicketSave(TicketSaveInput),
    TicketSlugAssign(TicketSlugAssignInput),
    RemoteAdd(RemoteAddInput),
    RemoteSelect(RemoteSelectInput),
    IndexRefresh(IndexRefreshInput),
    IndexRebuild(IndexRebuildInput),
}

impl Mutation {
    /// The command, as the envelope names it.
    pub fn command(&self) -> &'static str {
        self.kind().name()
    }

    fn kind(&self) -> Command {
        match self {
            Self::DocumentCreate(_) => Command::DocumentCreate,
            Self::DocumentSave(_) => Command::DocumentSave,
            Self::DocumentMove(_) => Command::DocumentMove,
            Self::TicketCreate(_) => Command::TicketCreate,
            Self::TicketSave(_) => Command::TicketSave,
            Self::TicketSlugAssign(_) => Command::TicketSlugAssign,
            Self::RemoteAdd(_) => Command::RemoteAdd,
            Self::RemoteSelect(_) => Command::RemoteSelect,
            Self::IndexRefresh(_) => Command::IndexRefresh,
            Self::IndexRebuild(_) => Command::IndexRebuild,
        }
    }

    fn into_binding(self) -> Box<dyn Binding> {
        match self {
            Self::DocumentCreate(input) => Box::new(DocumentBinding::from(input)),
            Self::DocumentSave(input) => Box::new(DocumentBinding::from(input)),
            Self::DocumentMove(input) => Box::new(DocumentBinding::from(input)),
            Self::TicketCreate(input) => Box::new(TicketBinding::from(input)),
            Self::TicketSave(input) => Box::new(TicketBinding::from(input)),
            Self::TicketSlugAssign(input) => Box::new(SlugAssignBinding::from(input)),
            Self::RemoteAdd(input) => Box::new(RemoteAddBinding::from(input)),
            Self::RemoteSelect(input) => Box::new(RemoteSelectBinding::from(input)),
            Self::IndexRefresh(input) => Box::new(IndexBinding::from(input)),
            Self::IndexRebuild(input) => Box::new(IndexBinding::from(input)),
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
