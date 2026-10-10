//! The mutation boundary: how a front end's request to change Manyhands
//! state is identified, recorded and answered.
//!
//! This holds what identifies a request and the records that link a
//! request ID to its command, target, input digest, operations and result.
//! The records are not a journal: they hold no step, no lease and no
//! authority over Git. Losing them loses the ability to answer a retry
//! from memory, never the ability to work.

mod identity;

pub use identity::{ConfirmationId, ConfirmationIdParseError, RequestId, RequestIdParseError};

/// What the boundary is built from: the intent digest, the request records
/// and the journal lookup. A front end uses the boundary itself; this is
/// public so that the integration tests can reach it.
#[doc(hidden)]
pub mod request_store {
    pub use super::identity::{
        DigestSalt, FieldValue, IntentDigest, IntentDigestBuilder, ScopeKey,
    };
}
