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

use std::sync::Arc;

use time::OffsetDateTime;

use crate::results::{Envelope, ResultCode, Scope, timestamp_string};

use super::{
    OperationFamily, OperationId, ReadError, RepositoryOperation, RepositoryService, keys,
    recovery::{self, JournalRow},
    remote,
};

mod dto;
mod identity;
// Until `execute` is built on it, in the commits that follow.
#[allow(dead_code)]
mod outcome;
mod records;

pub use dto::{RequestDto, RequestOperationDto, RequestResultDto, RequestState};
use identity::ScopeKey;
pub use identity::{ConfirmationId, ConfirmationIdParseError, RequestId, RequestIdParseError};
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
