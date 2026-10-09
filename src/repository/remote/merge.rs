//! Contracts for ordered synchronization merges and conflict resolution.
//!
//! This module contains no transport or branch mutation: it holds the pure
//! graph classification, the conflict eligibility boundary and the public
//! request, capability and outcome types the synchronization service uses.

use std::fmt;

use git2::Oid;

use super::SynchronizationTarget;
use crate::repository::{CommitIdentity, OperationId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntegrationStage {
    Context,
    Primary,
}

pub(super) fn integration_stages(
    target: &SynchronizationTarget,
    has_context_tracking: bool,
) -> Vec<IntegrationStage> {
    match target {
        SynchronizationTarget::Primary => vec![IntegrationStage::Primary],
        SynchronizationTarget::Context { .. } if has_context_tracking => {
            vec![IntegrationStage::Context, IntegrationStage::Primary]
        }
        SynchronizationTarget::Context { .. } => vec![IntegrationStage::Primary],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntegrationDisposition {
    Equal,
    IncomingAlreadyIntegrated,
    FastForward,
    MergeRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntegrationClassificationError<E> {
    Ancestry(E),
    UnrelatedHistories,
}

/// Classify exactly one ordered `(local, incoming)` integration pair.  The
/// callbacks are intentionally supplied by the caller so this pure seam cannot
/// accidentally inspect stale repository state or mutate it.
pub(super) fn classify_integration<E>(
    local: Oid,
    incoming: Oid,
    mut is_ancestor: impl FnMut(Oid, Oid) -> Result<bool, E>,
    mut merge_base_exists: impl FnMut(Oid, Oid) -> Result<bool, E>,
) -> Result<IntegrationDisposition, IntegrationClassificationError<E>> {
    if local == incoming {
        return Ok(IntegrationDisposition::Equal);
    }
    if is_ancestor(incoming, local).map_err(IntegrationClassificationError::Ancestry)? {
        return Ok(IntegrationDisposition::IncomingAlreadyIntegrated);
    }
    if is_ancestor(local, incoming).map_err(IntegrationClassificationError::Ancestry)? {
        return Ok(IntegrationDisposition::FastForward);
    }
    if !merge_base_exists(local, incoming).map_err(IntegrationClassificationError::Ancestry)? {
        return Err(IntegrationClassificationError::UnrelatedHistories);
    }
    Ok(IntegrationDisposition::MergeRequired)
}

/// Opaque optimistic precondition for one observed synchronization conflict.
/// Its private fields bind the operation, stage, target state and conflict set;
/// formatting never exposes those details.
#[derive(Clone, PartialEq, Eq)]
pub struct ConflictObservation {
    pub(super) operation_id: OperationId,
    pub(super) window_number: u32,
    pub(super) ordinal: u8,
    pub(super) fingerprint: [u8; 32],
    pub(super) head: Oid,
    pub(super) configuration: [u8; 32],
    pub(super) root: std::path::PathBuf,
}

impl ConflictObservation {
    /// The configuration observation this inspection is bound to. A caller
    /// confirming a commit identity for `resolve_synchronization` copies it
    /// into `ConfirmedCommitIdentity::expected_configuration`. It is a digest,
    /// never configuration text.
    pub fn expected_configuration(&self) -> [u8; 32] {
        self.configuration
    }

    #[cfg(test)]
    pub(super) fn for_testing(value: [u8; 32]) -> Self {
        Self {
            operation_id: OperationId::new(),
            window_number: 0,
            ordinal: 0,
            fingerprint: value,
            head: Oid::zero(),
            configuration: [0; 32],
            root: std::path::PathBuf::new(),
        }
    }
}

impl fmt::Debug for ConflictObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConflictObservation(<redacted>)")
    }
}

/// Opaque, inspection-issued capability for exactly one conflict entry. It
/// carries no caller-supplied path text.
#[derive(Clone, PartialEq, Eq)]
pub struct ConflictPathToken {
    pub(super) observation: ConflictObservation,
    pub(super) ordinal: u32,
    pub(super) path: Vec<u8>,
    pub(super) base: Option<Oid>,
    pub(super) base_mode: Option<u32>,
    pub(super) local: Option<Oid>,
    pub(super) local_mode: Option<u32>,
    pub(super) incoming: Option<Oid>,
    pub(super) incoming_mode: Option<u32>,
}

impl fmt::Debug for ConflictPathToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConflictPathToken(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictEligibility {
    EligibleCanonical,
    ExternalResolutionRequired,
}

/// Classification is deliberately structural rather than marker-text based:
/// these facts are derived from the actual three index stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConflictEntryKind {
    Document,
    Ticket,
    Comment,
    Noncanonical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConflictStructure {
    RegularUtf8SamePath,
    Binary,
    Executable,
    Symlink,
    Rename,
    Delete,
    IdentityChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ConflictCandidate {
    pub kind: ConflictEntryKind,
    pub structure: ConflictStructure,
}

/// A whole conflict set is eligible only if every entry is an owned canonical
/// regular UTF-8 same-path conflict.  This encodes the approved mixed-set
/// boundary: a single unsupported entry requires external repair for all.
pub(super) fn conflict_eligibility(entries: &[ConflictCandidate]) -> ConflictEligibility {
    if !entries.is_empty()
        && entries.iter().all(|entry| {
            matches!(
                entry.kind,
                ConflictEntryKind::Document
                    | ConflictEntryKind::Ticket
                    | ConflictEntryKind::Comment
            ) && entry.structure == ConflictStructure::RegularUtf8SamePath
        })
    {
        ConflictEligibility::EligibleCanonical
    } else {
        ConflictEligibility::ExternalResolutionRequired
    }
}

/// Ephemeral conflict content. Callers may consume bytes but diagnostics never
/// render their contents.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedConflictBytes(Vec<u8>);

impl RedactedConflictBytes {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    #[cfg(test)]
    fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl fmt::Debug for RedactedConflictBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RedactedConflictBytes(<redacted>)")
    }
}

/// Caller-confirmed identity is accepted only at a committing boundary when
/// the effective Git configuration has no complete identity; it never replaces
/// an existing one.
///
/// `expected_configuration` is the observation the service returned at the
/// identity-required boundary, so a confirmation goes stale when that
/// boundary changes. It is a digest, never configuration text, and is copied
/// from one of two places:
///
/// - `synchronize_remote`: the `expected_configuration` of the returned
///   `SynchronizationError::IdentityRequired` (the observed Git identity
///   configuration). A different value is refused as an external change.
/// - `resolve_synchronization`: `ConflictObservation::expected_configuration`
///   of the inspection the request is built from (the observed repository
///   configuration). A different value is not accepted as a confirmation and
///   the outcome stays `IdentityRequired`.
#[derive(Clone)]
pub struct ConfirmedCommitIdentity {
    pub confirmation_id: OperationId,
    pub identity: CommitIdentity,
    pub expected_configuration: [u8; 32],
}

impl fmt::Debug for ConfirmedCommitIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConfirmedCommitIdentity(<redacted>)")
    }
}

/// Explicit, observation-bound canonical conflict resolution. Paths can only
/// be supplied through tokens issued by `inspect_synchronization_recovery`;
/// no raw repository path, ref, OID or force authority is caller input, and
/// result bytes stay redacted in diagnostics.
#[derive(Clone)]
pub struct ResolveSynchronizationRequest {
    pub root: std::path::PathBuf,
    pub synchronization_id: OperationId,
    pub attempt_id: OperationId,
    pub(super) observation: ConflictObservation,
    pub(super) resolutions: Vec<(ConflictPathToken, RedactedConflictBytes)>,
    pub identity: Option<ConfirmedCommitIdentity>,
}

impl ResolveSynchronizationRequest {
    pub fn new(
        root: std::path::PathBuf,
        synchronization_id: OperationId,
        attempt_id: OperationId,
        observation: ConflictObservation,
        resolutions: Vec<(ConflictPathToken, RedactedConflictBytes)>,
        identity: Option<ConfirmedCommitIdentity>,
    ) -> Self {
        Self {
            root,
            synchronization_id,
            attempt_id,
            observation,
            resolutions,
            identity,
        }
    }
}

impl fmt::Debug for ResolveSynchronizationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolveSynchronizationRequest")
            .field("root", &self.root)
            .field("synchronization_id", &self.synchronization_id)
            .field("attempt_id", &self.attempt_id)
            .field("observation", &self.observation)
            .field("resolutions", &"<redacted>")
            .field("identity", &self.identity)
            .finish()
    }
}

/// Local-only result of an explicit synchronization conflict resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveSynchronizationOutcome {
    LocalCheckpointComplete { commit_oid: Oid },
    IdentityRequired,
    StaleObservation,
    ValidationFailed,
    RecoveryRequired,
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
