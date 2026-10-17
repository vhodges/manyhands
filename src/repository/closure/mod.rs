//! Confirmed ticket closure. Cycle 10 Task 2 adds its parent record; the
//! observation, checkpoint and lifecycle arrive in later tasks.
use std::fmt;

pub(super) mod record;

/// Declaration order is the stable order of preflight blockers and the bit
/// order of the stored refusal mask.
#[allow(dead_code)] // Reported by the closure preflight (Task 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureBlocker {
    NotATicket,
    ContextMissing,
    ContextNonconforming,
    ContextDirty,
    ContextConflicted,
    ConflictPending,
    LocalOperationPending,
    CommentPublicationPending,
    PrimaryMissing,
    PrimaryNotCheckedOut,
    PrimaryDirty,
    PrimaryConflicted,
    PrimaryAheadOfRemote,
    ContextBehindRemote,
    IntegrationConflict,
    RemoteContextDeleted,
    HistoryUnknown,
    AlreadyClosed,
    PrimaryReopened,
    TicketInvalidAfterClosure,
    IdentityRequired,
}

/// The 32-byte consent digest of a ticket closure. Its `Debug` never shows
/// the bytes: they hash low-entropy input that includes `closed_by`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClosureExpectation([u8; 32]);
impl ClosureExpectation {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}
impl fmt::Debug for ClosureExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClosureExpectation(<redacted>)")
    }
}
