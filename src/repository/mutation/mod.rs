//! The mutation boundary: how a front end's request to change Manyhands
//! state is identified, recorded and answered.
//!
//! This holds what identifies a request and the records that link a
//! request ID to its command, target, input digest, operations and result.
//! The records are not a journal: they hold no step, no lease and no
//! authority over Git. Losing them loses the ability to answer a retry
//! from memory, never the ability to work.

use std::sync::Arc;

use time::OffsetDateTime;

use super::{
    OperationFamily, OperationId, ReadError, RepositoryOperation, RepositoryService, keys,
    recovery::{self, JournalRow},
    remote,
};

mod dto;
mod identity;
mod records;

pub use dto::RequestState;
use identity::ScopeKey;
pub use identity::{ConfirmationId, ConfirmationIdParseError, RequestId, RequestIdParseError};
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
    /// family: absent, pending with its state and step, or completed.
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
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
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
        recovery::JournalRow,
    };
}
