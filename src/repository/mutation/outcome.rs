//! Domain outcomes and errors as the codes and effects of a result.
//!
//! Every mapping here is a `match` with no wildcard arm, so a variant added
//! later does not compile until it is given a code. None formats the error
//! it maps.

use crate::results::{
    CheckpointEffect, DiscoveryEffect, Effects, RecoveryAction, RecoveryActionKind, ResultCode,
    WriteEffect,
};

use super::{
    super::{
        EnableRepositoryOutcome, GeneratedKeyDeletionPreflight, LocalCheckpoint, OperationId,
        PublicationRemoteOutcome, PublishPendingReason, RefreshOutcome, RegisterSharedKeyOutcome,
        RemoteOutcome, RemoveRegistrationOutcome, RepositoryErrorKind, SaveOutcome,
        SharedKeySelectionOutcome, SynchronizationError, SynchronizationOutcome,
        SynchronizationResult, UnregisterSharedKeyOutcome,
        keys::{DeleteGeneratedKeyOutcome, GenerateSharedKeyOutcome, KeyMaterialErrorKind},
        transport::SshTransportErrorKind,
    },
    identity::RequestId,
};

/// The code a mutation reports for a domain error of `kind`.
///
/// Some commands report another code for a kind, from what the binding
/// knows: `remote select` reports `identity_required` for
/// `InvalidIdentity`, since the identity it lacks is the repository's
/// and not part of its input.
pub(crate) fn repository_error_code(kind: RepositoryErrorKind) -> ResultCode {
    match kind {
        RepositoryErrorKind::InvalidPath => ResultCode::InvalidPath,
        RepositoryErrorKind::InaccessibleRepository => ResultCode::RepositoryInaccessible,
        RepositoryErrorKind::NotRepository => ResultCode::NotRepository,
        RepositoryErrorKind::BareRepository => ResultCode::BareRepository,
        RepositoryErrorKind::DetachedHead | RepositoryErrorKind::WrongCheckedOutBranch => {
            ResultCode::WrongBranch
        }
        RepositoryErrorKind::DirtyWorktree | RepositoryErrorKind::DirtyConfigurationPath => {
            ResultCode::WorktreeNotClean
        }
        RepositoryErrorKind::ConflictedWorktree => ResultCode::WorktreeConflicted,
        RepositoryErrorKind::InvalidConfiguration => ResultCode::InvalidConfiguration,
        RepositoryErrorKind::RepositoryNotRegistered
        | RepositoryErrorKind::RepositoryNotEnabled => ResultCode::RepositoryNotRegistered,
        RepositoryErrorKind::IndexUnavailable => ResultCode::IndexUnavailable,
        RepositoryErrorKind::RepositoryBusy => ResultCode::Busy,
        RepositoryErrorKind::RecoveryRequired
        | RepositoryErrorKind::RollbackIncomplete
        | RepositoryErrorKind::MismatchedAuthoringContext => ResultCode::RecoveryRequired,
        RepositoryErrorKind::ExternalChange => ResultCode::ExternalChange,
        RepositoryErrorKind::InvalidIdentity => ResultCode::InvalidInput,
        RepositoryErrorKind::MissingAuthoringTarget => ResultCode::ItemNotFound,
        RepositoryErrorKind::OccupiedItemPath => ResultCode::OccupiedPath,
        // The boundary allocates operation IDs, so a mismatch is its own
        // fault.
        RepositoryErrorKind::OperationMismatch => ResultCode::InternalError,
        // A backend failure. Its text stays in the error.
        RepositoryErrorKind::Io
        | RepositoryErrorKind::Sqlite
        | RepositoryErrorKind::Git
        | RepositoryErrorKind::InjectedFailure => ResultCode::InternalError,
        RepositoryErrorKind::InvalidSharedKeyMetadata
        | RepositoryErrorKind::InvalidSharedKeySourcePath => ResultCode::InvalidInput,
        RepositoryErrorKind::SharedKeyRegistryUnavailable => ResultCode::IndexUnavailable,
        RepositoryErrorKind::SharedKeyMaterialPending => ResultCode::RecoveryRequired,
        RepositoryErrorKind::InvalidPublicationRemote
        | RepositoryErrorKind::UnavailablePublicationRemote => ResultCode::InvalidRemote,
        RepositoryErrorKind::SelectedRemoteRemoval => ResultCode::PublicationRemoteInUse,
        RepositoryErrorKind::RemoteNameConflict => ResultCode::RemoteNameConflict,
        // The initialization or publication commit exists and the registry
        // write failed.
        RepositoryErrorKind::RegistryRefreshPending => ResultCode::RegistrationPending,
    }
}

/// The code a mutation reports for a failure of the SSH transport.
///
/// `host_mismatch` is not here: it is what the binding of a host approval
/// reports when the host presents another key than the one approved.
// Used by the synchronization and host bindings, which are not bound yet.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn ssh_transport_error_code(kind: &SshTransportErrorKind) -> ResultCode {
    match kind {
        // The remote's location is not one the transport can use.
        SshTransportErrorKind::ConfigurationInvalid | SshTransportErrorKind::UsernameRequired => {
            ResultCode::InvalidRemote
        }
        SshTransportErrorKind::PublicationRemoteMissing => ResultCode::PublicationRemoteRequired,
        SshTransportErrorKind::NoSelectedKey => ResultCode::NoSelectedKey,
        SshTransportErrorKind::KeyMissing
        | SshTransportErrorKind::KeyUnreadable
        | SshTransportErrorKind::KeyInvalidOrUnsupported => ResultCode::KeyUnavailable,
        // What the operation was begun against changed under it.
        SshTransportErrorKind::KeySourceChanged
        | SshTransportErrorKind::SelectionChanged
        | SshTransportErrorKind::EndpointChanged
        | SshTransportErrorKind::HostTrustChanged => ResultCode::ExternalChange,
        SshTransportErrorKind::KeyRejected => ResultCode::KeyRejected,
        // A prompt that was dismissed, or that nothing could show: the
        // action did not happen and needs an unlock.
        SshTransportErrorKind::UnlockCancelled | SshTransportErrorKind::ProviderUnavailable => {
            ResultCode::UnlockRequired
        }
        SshTransportErrorKind::UnlockFailed => ResultCode::UnlockFailed,
        SshTransportErrorKind::HostApprovalRequired { presented: _ } => {
            ResultCode::HostApprovalRequired
        }
        SshTransportErrorKind::HostReplacementRequired {
            expected: _,
            presented: _,
        } => ResultCode::HostReplacementRequired,
        SshTransportErrorKind::RegistryUnavailable => ResultCode::IndexUnavailable,
        SshTransportErrorKind::TransportUnavailable
        | SshTransportErrorKind::HostVerificationUnavailable => ResultCode::TransportUnavailable,
        SshTransportErrorKind::RemoteUnavailable => ResultCode::RemoteUnavailable,
        SshTransportErrorKind::PushRejected => ResultCode::PushRejected,
        // The library's own fault, or the server's, with nothing a caller
        // can act on.
        SshTransportErrorKind::RuntimeUninitialized | SshTransportErrorKind::ProtocolFailure => {
            ResultCode::InternalError
        }
    }
}

/// The code a mutation reports for a failure of a key-material operation.
// Used by the key bindings, which are not bound yet.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn key_material_error_code(kind: KeyMaterialErrorKind) -> ResultCode {
    match kind {
        KeyMaterialErrorKind::InvalidLabel | KeyMaterialErrorKind::InvalidPassphrase => {
            ResultCode::InvalidInput
        }
        KeyMaterialErrorKind::RegistryUnavailable => ResultCode::IndexUnavailable,
        KeyMaterialErrorKind::Busy => ResultCode::Busy,
        KeyMaterialErrorKind::NotRegistered => ResultCode::KeyNotFound,
        KeyMaterialErrorKind::SelectedKeyMustBeCleared => ResultCode::SelectedKeyInUse,
        // Manyhands deletes only the key files it can show to be its own.
        KeyMaterialErrorKind::ImportedKey | KeyMaterialErrorKind::OwnershipUnverified => {
            ResultCode::KeyNotDeletable
        }
        KeyMaterialErrorKind::SourceMissing
        | KeyMaterialErrorKind::SourceUnreadable
        | KeyMaterialErrorKind::NotRegularFile
        | KeyMaterialErrorKind::UnsafePath
        | KeyMaterialErrorKind::InvalidGeneratedKey => ResultCode::KeyUnavailable,
        KeyMaterialErrorKind::UnlockFailed => ResultCode::UnlockFailed,
        KeyMaterialErrorKind::SourceChanged | KeyMaterialErrorKind::SelectionChanged => {
            ResultCode::ExternalChange
        }
        KeyMaterialErrorKind::ConfirmationRequired => ResultCode::ConfirmationRequired,
        // The boundary allocates operation IDs, so a mismatch is its own
        // fault.
        KeyMaterialErrorKind::OperationMismatch => ResultCode::InternalError,
        // The system the library runs on. Nothing in a request changes it.
        KeyMaterialErrorKind::HomeUnavailable
        | KeyMaterialErrorKind::ProtectionUnavailable
        | KeyMaterialErrorKind::RandomnessUnavailable
        | KeyMaterialErrorKind::GenerationFailed
        | KeyMaterialErrorKind::StorageUnavailable => ResultCode::InternalError,
    }
}

/// The code a mutation reports for a synchronization that stopped.
///
/// `Interrupted` is `recovery_required` here; the binding reports
/// `cancelled` when the operation's row says it was cancelled.
// Used by the synchronization bindings, which are not bound yet.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn synchronization_error_code(error: &SynchronizationError) -> ResultCode {
    match error {
        SynchronizationError::Repository(error) => repository_error_code(error.kind),
        SynchronizationError::Transport(error) => ssh_transport_error_code(&error.kind),
        SynchronizationError::Busy => ResultCode::Busy,
        SynchronizationError::PollYielding => ResultCode::PollYielding,
        SynchronizationError::TargetNotMaterialized => ResultCode::ItemNotFound,
        SynchronizationError::WorktreeNotClean { target: _ } => ResultCode::WorktreeNotClean,
        SynchronizationError::WorktreeConflicted { target: _ } => ResultCode::WorktreeConflicted,
        SynchronizationError::PrimaryMissing => ResultCode::InvalidConfiguration,
        SynchronizationError::RemoteContextDeleted => ResultCode::RemoteBranchDeleted,
        SynchronizationError::IdentityRequired {
            target: _,
            expected_configuration: _,
        } => ResultCode::IdentityRequired,
        SynchronizationError::MergeRequired { target: _ } => ResultCode::MergeRequired,
        SynchronizationError::PushRejected => ResultCode::PushRejected,
        SynchronizationError::ExternalChange => ResultCode::ExternalChange,
        // The bindings refuse divergence, so no request of theirs leaves a
        // merge to resolve; one made some other way is not theirs to
        // continue.
        SynchronizationError::ConflictPending {
            target: _,
            operation_id: _,
            stage: _,
        }
        | SynchronizationError::ExternalResolutionRequired {
            target: _,
            operation_id: _,
        }
        | SynchronizationError::Interrupted
        | SynchronizationError::HistoryUnknown
        | SynchronizationError::RecoveryRequired => ResultCode::RecoveryRequired,
    }
}

/// The code of what `add_remote` and `remove_remote` return.
// Used by the remote bindings.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn remote_outcome_code(outcome: &RemoteOutcome) -> ResultCode {
    match outcome {
        RemoteOutcome::Changed => ResultCode::Ok,
        RemoteOutcome::NoChange => ResultCode::AlreadyApplied,
        RemoteOutcome::IndexPending { authoritative: _ } => ResultCode::DiscoveryPending,
    }
}

/// The code of what `refresh_repository` returns. A refresh that another
/// owner holds, or that found the repository changing while it observed
/// it, has stored nothing: the same request is sent again.
pub(crate) fn refresh_outcome_code(outcome: &RefreshOutcome) -> ResultCode {
    match outcome {
        RefreshOutcome::Refreshed { snapshot: _ } => ResultCode::Ok,
        RefreshOutcome::IndexPending { root: _ }
        | RefreshOutcome::RetryRequired {
            root: _,
            context: _,
        } => ResultCode::Busy,
    }
}

/// The code of what `set_publication_remote` returns, before the evidence
/// check: `Changed` is `ok` only if the check finds the request's commit,
/// and `already_applied` otherwise.
// Its binding reads the outcome as a save's; this states the codes.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn publication_remote_outcome_code(outcome: &PublicationRemoteOutcome) -> ResultCode {
    match outcome {
        PublicationRemoteOutcome::Changed { commit_oid: _ } => ResultCode::Ok,
        PublicationRemoteOutcome::NoChange => ResultCode::AlreadyApplied,
        PublicationRemoteOutcome::RegistrationPending { commit_oid: _ } => {
            ResultCode::RegistrationPending
        }
        PublicationRemoteOutcome::IndexPending { commit_oid: _ } => ResultCode::DiscoveryPending,
    }
}

/// The codes of the outcomes of the commands that are not bound yet. Each
/// is a `match` with no wildcard arm, as every mapping here is.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) mod unbound {
    use super::*;

    pub(crate) fn enable_outcome_code(outcome: &EnableRepositoryOutcome) -> ResultCode {
        match outcome {
            EnableRepositoryOutcome::Enabled { commit_oid: _ } => ResultCode::Ok,
            EnableRepositoryOutcome::AlreadyEnabled => ResultCode::AlreadyApplied,
            EnableRepositoryOutcome::IdentityRequired => ResultCode::IdentityRequired,
            EnableRepositoryOutcome::IndexPending(_) => ResultCode::DiscoveryPending,
            EnableRepositoryOutcome::RegistrationPending { commit_oid: _ } => {
                ResultCode::RegistrationPending
            }
        }
    }

    pub(crate) fn remove_registration_outcome_code(
        outcome: &RemoveRegistrationOutcome,
    ) -> ResultCode {
        match outcome {
            RemoveRegistrationOutcome::Removed => ResultCode::Ok,
            RemoveRegistrationOutcome::NotRegistered => ResultCode::AlreadyApplied,
        }
    }

    pub(crate) fn synchronization_result_code(result: &SynchronizationResult) -> ResultCode {
        match result {
            SynchronizationResult::Complete(outcome) => synchronization_outcome_code(outcome),
            SynchronizationResult::IndexPending(_) => ResultCode::DiscoveryPending,
        }
    }

    pub(crate) fn synchronization_outcome_code(outcome: &SynchronizationOutcome) -> ResultCode {
        match outcome {
            SynchronizationOutcome::Published { target: _, oid: _ } => ResultCode::Ok,
            SynchronizationOutcome::AlreadyCurrent { target: _, oid: _ } => {
                ResultCode::AlreadyApplied
            }
            SynchronizationOutcome::PublishPending {
                target: _,
                local_oid: _,
                reason: PublishPendingReason::NoPublicationRemote,
            } => ResultCode::PublicationRemoteRequired,
        }
    }

    pub(crate) fn register_key_outcome_code(outcome: &RegisterSharedKeyOutcome) -> ResultCode {
        match outcome {
            RegisterSharedKeyOutcome::Registered(_) => ResultCode::Ok,
            RegisterSharedKeyOutcome::SourceAlreadyRegistered { existing: _ } => {
                ResultCode::AlreadyApplied
            }
        }
    }

    pub(crate) fn key_selection_outcome_code(outcome: &SharedKeySelectionOutcome) -> ResultCode {
        match outcome {
            SharedKeySelectionOutcome::Selected(_) | SharedKeySelectionOutcome::Cleared => {
                ResultCode::Ok
            }
            SharedKeySelectionOutcome::AlreadyCleared => ResultCode::AlreadyApplied,
        }
    }

    pub(crate) fn unregister_key_outcome_code(outcome: UnregisterSharedKeyOutcome) -> ResultCode {
        match outcome {
            UnregisterSharedKeyOutcome::Unregistered => ResultCode::Ok,
            UnregisterSharedKeyOutcome::NotRegistered => ResultCode::AlreadyApplied,
            UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared => ResultCode::SelectedKeyInUse,
        }
    }

    pub(crate) fn generate_key_outcome_code(outcome: &GenerateSharedKeyOutcome) -> ResultCode {
        match outcome {
            GenerateSharedKeyOutcome::Created(_) => ResultCode::Ok,
            GenerateSharedKeyOutcome::AlreadyCreated(_) => ResultCode::AlreadyApplied,
            GenerateSharedKeyOutcome::RecoveryRequired(_) => ResultCode::RecoveryRequired,
        }
    }

    pub(crate) fn delete_key_outcome_code(outcome: &DeleteGeneratedKeyOutcome) -> ResultCode {
        match outcome {
            DeleteGeneratedKeyOutcome::Deleted => ResultCode::Ok,
            DeleteGeneratedKeyOutcome::AlreadyDeleted => ResultCode::AlreadyApplied,
            DeleteGeneratedKeyOutcome::Cancelled => ResultCode::Cancelled,
            DeleteGeneratedKeyOutcome::RecoveryRequired(_) => ResultCode::RecoveryRequired,
        }
    }

    /// What the review of a key deletion found, as the code of its
    /// preview: a key that can be deleted needs the confirmation.
    pub(crate) fn key_deletion_preflight_code(
        preflight: &GeneratedKeyDeletionPreflight,
    ) -> ResultCode {
        match preflight {
            GeneratedKeyDeletionPreflight::ConfirmationRequired(_) => {
                ResultCode::ConfirmationRequired
            }
            GeneratedKeyDeletionPreflight::NotRegistered => ResultCode::KeyNotFound,
            GeneratedKeyDeletionPreflight::ImportedKey => ResultCode::KeyNotDeletable,
            GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared => ResultCode::SelectedKeyInUse,
        }
    }
}

/// What a save returned without an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveReport {
    /// No identity to commit as: nothing was written.
    IdentityRequired,
    Saved {
        /// The commit the domain names. It is this request's only if the
        /// evidence check says so: a save that changed nothing can name
        /// the head it found.
        claimed: Option<git2::Oid>,
        /// Whether the index hand-off completed.
        discovered: bool,
    },
}

impl SaveReport {
    pub(crate) fn of(outcome: SaveOutcome) -> Self {
        match outcome {
            SaveOutcome::IdentityRequired { context: _ } => Self::IdentityRequired,
            SaveOutcome::Saved {
                context: _,
                checkpoint,
            } => Self::of_checkpoint(checkpoint, true),
            SaveOutcome::IndexPending {
                context: _,
                checkpoint,
            } => Self::of_checkpoint(checkpoint, false),
        }
    }

    pub(crate) fn of_checkpoint(checkpoint: LocalCheckpoint, discovered: bool) -> Self {
        let claimed = match checkpoint {
            LocalCheckpoint::Checkpointed { commit_oid }
            | LocalCheckpoint::RefreshPending { commit_oid } => Some(commit_oid),
            LocalCheckpoint::NoChange => None,
        };
        Self::Saved {
            claimed,
            discovered,
        }
    }

    /// The commit the domain names, for the evidence check to confirm.
    pub(crate) fn claimed(self) -> Option<git2::Oid> {
        match self {
            Self::IdentityRequired => None,
            Self::Saved { claimed, .. } => claimed,
        }
    }

    /// The code and effects of the save, given the commit the evidence
    /// check found to be this request's.
    pub(crate) fn result(self, commit: Option<git2::Oid>) -> (ResultCode, Effects) {
        let discovered = match self {
            Self::IdentityRequired => {
                return (ResultCode::IdentityRequired, Effects::not_requested());
            }
            Self::Saved { discovered, .. } => discovered,
        };
        let effects = Effects {
            write: match commit {
                Some(_) => WriteEffect::Written,
                None => WriteEffect::Unchanged,
            },
            checkpoint: match commit {
                Some(_) => CheckpointEffect::Committed,
                None => CheckpointEffect::Unchanged,
            },
            discovery: match (discovered, commit) {
                (false, _) => DiscoveryEffect::Pending,
                (true, Some(_)) => DiscoveryEffect::Current,
                (true, None) => DiscoveryEffect::NotRequested,
            },
            commit_oid: commit.map(|commit| commit.to_string()),
            ..Effects::not_requested()
        };
        let code = match (discovered, commit) {
            (false, _) => ResultCode::DiscoveryPending,
            (true, Some(_)) => ResultCode::Ok,
            (true, None) => ResultCode::AlreadyApplied,
        };
        (code, effects)
    }
}

/// The effects of a save that stopped with an error while its operation
/// was in flight, from what the repository shows: `commit` is the commit
/// in range that left the file as the request intended, and `written` says
/// the file in the worktree is what the request intended and is not what
/// it was before the request.
pub(crate) fn stopped_save_effects(commit: Option<git2::Oid>, written: bool) -> Effects {
    match commit {
        Some(commit) => Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Committed,
            discovery: DiscoveryEffect::Pending,
            commit_oid: Some(commit.to_string()),
            ..Effects::not_requested()
        },
        None if written => Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Pending,
            discovery: DiscoveryEffect::Pending,
            ..Effects::not_requested()
        },
        None => Effects::not_requested(),
    }
}

/// The argument of a recovery action that names a repository root.
const ROOT_ARGUMENT: &str = "root";
/// The argument of `request.retry`.
const REQUEST_ARGUMENT: &str = "request_id";

/// What a client does about a code that stopped a request. Every code is
/// named: one added later does not compile until its action is decided.
///
/// `root` is the repository the result names, when it names one. `retry`
/// says the request's record was left accepted, so repeating the same
/// request continues it; a code with an action of its own keeps that one,
/// and `external_change` is offered none.
pub(crate) fn recovery(
    code: ResultCode,
    root: Option<&str>,
    request_id: RequestId,
    operation_id: Option<OperationId>,
    retry: bool,
) -> Vec<RecoveryAction> {
    let at_root =
        |action| RecoveryAction::new(action, root.map(|root| (ROOT_ARGUMENT, root.into())));
    let retry_request = || {
        RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [(REQUEST_ARGUMENT, request_id.to_string().into())],
        )
    };
    match code {
        // The authoritative work is recorded and the hand-off is owed,
        // which needs no input from the caller.
        ResultCode::DiscoveryPending => operation_id
            .map(|operation_id| {
                RecoveryAction::for_operation(
                    RecoveryActionKind::OperationResume,
                    operation_id.to_string(),
                    [],
                )
            })
            .into_iter()
            .collect(),
        ResultCode::IdentityRequired | ResultCode::InitialsRequired => {
            vec![at_root(RecoveryActionKind::RepoIdentitySet)]
        }
        ResultCode::RecoveryRequired
        | ResultCode::InvalidSlugConfiguration
        | ResultCode::InvalidConfiguration => vec![at_root(RecoveryActionKind::RepoInspect)],
        ResultCode::RepositoryNotRegistered | ResultCode::IndexUnavailable => {
            vec![at_root(RecoveryActionKind::IndexRebuild)]
        }
        // An external change to an item is not answered by the same
        // request: it is refused again for as long as the change stands,
        // also when its record stays accepted because its operation is
        // still in flight. The caller reads the item again and submits a
        // new request.
        ResultCode::ExternalChange => Vec::new(),
        // Nothing was accepted, or what was accepted is still there:
        // either way the same request is the one to send.
        ResultCode::Busy => vec![retry_request()],
        // The commit exists and the registry write failed: the same
        // request completes the registration.
        ResultCode::RegistrationPending => vec![retry_request()],
        // Decided: these have no action of their own. They are the codes
        // a bound command, or the boundary itself, ends with: a write of
        // a ticket or a document, a remote added or selected, an index
        // refreshed or rebuilt. The same request is offered when its
        // record was left accepted, and nothing otherwise.
        ResultCode::Ok
        | ResultCode::AlreadyApplied
        | ResultCode::Cancelled
        | ResultCode::InternalError
        | ResultCode::InvalidInput
        | ResultCode::InvalidPath
        | ResultCode::NotRepository
        | ResultCode::NotRepositoryRoot
        | ResultCode::BareRepository
        | ResultCode::RepositoryInaccessible
        | ResultCode::ItemNotFound
        | ResultCode::RelationshipCycle
        | ResultCode::InvalidRelationship
        | ResultCode::RequestMismatch
        | ResultCode::NotConfirmable
        | ResultCode::OccupiedPath
        | ResultCode::SlugAlreadyAssigned
        | ResultCode::TicketClosed
        | ResultCode::WrongBranch
        | ResultCode::WorktreeNotClean
        | ResultCode::WorktreeConflicted
        | ResultCode::RemoteNameConflict
        | ResultCode::InvalidRemote
        // Not decided: the codes only a command that is not bound yet
        // returns. They share the arm above so that the match is complete,
        // and for no other reason: the design's recovery table gives
        // several of them an action, and the task that binds the command
        // moves its codes to an arm of their own.
        | ResultCode::InvalidId
        | ResultCode::PathNotFound
        | ResultCode::KeyNotFound
        | ResultCode::PublicKeyUnavailable
        | ResultCode::AuthorityNotFound
        | ResultCode::OperationNotFound
        | ResultCode::RequestNotFound
        | ResultCode::ConfirmationMismatch
        | ResultCode::NotRepairable
        | ResultCode::KeyNotDeletable
        | ResultCode::ConfirmationRequired
        | ResultCode::ConfirmationExpired
        | ResultCode::ConfirmationNotFound
        | ResultCode::ConfirmationUsed
        | ResultCode::OriginalRequestRequired
        | ResultCode::IdentityAmbiguous
        | ResultCode::PublicationRemoteRequired
        | ResultCode::PublicationRemoteInUse
        | ResultCode::MergeRequired
        | ResultCode::RemoteBranchDeleted
        | ResultCode::PushRejected
        | ResultCode::NoSelectedKey
        | ResultCode::KeyUnavailable
        | ResultCode::KeyRejected
        | ResultCode::SelectedKeyInUse
        | ResultCode::UnlockRequired
        | ResultCode::UnlockFailed
        | ResultCode::HostApprovalRequired
        | ResultCode::HostReplacementRequired
        | ResultCode::HostMismatch
        | ResultCode::PollYielding
        | ResultCode::RemoteUnavailable
        | ResultCode::TransportUnavailable => {
            if retry {
                vec![retry_request()]
            } else {
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
#[path = "outcome_tests.rs"]
mod tests;
