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

/// The code of what `generate_shared_key` returns.
///
/// `key generate` reports `ok` for `AlreadyCreated`, from what its
/// binding knows: the operation ID is the request's own, so a key that
/// operation already created is the key this request generated.
pub(crate) fn generate_key_outcome_code(outcome: &GenerateSharedKeyOutcome) -> ResultCode {
    match outcome {
        GenerateSharedKeyOutcome::Created(_) => ResultCode::Ok,
        GenerateSharedKeyOutcome::AlreadyCreated(_) => ResultCode::AlreadyApplied,
        GenerateSharedKeyOutcome::RecoveryRequired(_) => ResultCode::RecoveryRequired,
    }
}

/// The code of what `register_shared_key` returns. A source that is
/// already registered is the request's end state, whichever request
/// registered it.
pub(crate) fn register_key_outcome_code(outcome: &RegisterSharedKeyOutcome) -> ResultCode {
    match outcome {
        RegisterSharedKeyOutcome::Registered(_) => ResultCode::Ok,
        RegisterSharedKeyOutcome::SourceAlreadyRegistered { existing: _ } => {
            ResultCode::AlreadyApplied
        }
    }
}

/// The code of what `select_shared_key` and `clear_shared_key_selection`
/// return. A selection reads as `ok` whether or not the key was selected
/// before: `key select` reports a key that already was as a no-op from
/// what it read of the registry.
pub(crate) fn key_selection_outcome_code(outcome: &SharedKeySelectionOutcome) -> ResultCode {
    match outcome {
        SharedKeySelectionOutcome::Selected(_) | SharedKeySelectionOutcome::Cleared => {
            ResultCode::Ok
        }
        SharedKeySelectionOutcome::AlreadyCleared => ResultCode::AlreadyApplied,
    }
}

/// The code of what `unregister_shared_key` returns. A key that is not
/// registered is the request's end state: `key remove` refuses a key
/// that was never registered before it is accepted, as `key_not_found`.
pub(crate) fn unregister_key_outcome_code(outcome: UnregisterSharedKeyOutcome) -> ResultCode {
    match outcome {
        UnregisterSharedKeyOutcome::Unregistered => ResultCode::Ok,
        UnregisterSharedKeyOutcome::NotRegistered => ResultCode::AlreadyApplied,
        UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared => ResultCode::SelectedKeyInUse,
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
    /// The commit was made and the repository's registration could not
    /// be brought up to it. The same request completes it.
    RegistrationPending { claimed: Option<git2::Oid> },
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

    /// What `set_publication_remote` returned, read as a save of the
    /// configuration file: it writes that file and commits it.
    ///
    /// `Changed` names the head it found whenever the configuration
    /// equals the request, whoever committed it. As for any save, the
    /// commit is the request's only if the evidence check says so.
    pub(crate) fn of_publication_remote(outcome: PublicationRemoteOutcome) -> Self {
        match outcome {
            PublicationRemoteOutcome::Changed { commit_oid } => Self::Saved {
                claimed: Some(commit_oid),
                discovered: true,
            },
            PublicationRemoteOutcome::NoChange => Self::Saved {
                claimed: None,
                discovered: true,
            },
            PublicationRemoteOutcome::IndexPending { commit_oid } => Self::Saved {
                claimed: commit_oid,
                discovered: false,
            },
            PublicationRemoteOutcome::RegistrationPending { commit_oid } => {
                Self::RegistrationPending {
                    claimed: Some(commit_oid),
                }
            }
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

    /// The same report, naming no commit: for a domain call whose
    /// operation is known to have committed nothing, whatever commit it
    /// names.
    pub(crate) fn naming_no_commit(self) -> Self {
        match self {
            Self::IdentityRequired => Self::IdentityRequired,
            Self::Saved {
                claimed: _,
                discovered,
            } => Self::Saved {
                claimed: None,
                discovered,
            },
            Self::RegistrationPending { claimed: _ } => Self::RegistrationPending { claimed: None },
        }
    }

    /// The commit the domain names, for the evidence check to confirm.
    pub(crate) fn claimed(self) -> Option<git2::Oid> {
        match self {
            Self::IdentityRequired => None,
            Self::Saved { claimed, .. } | Self::RegistrationPending { claimed } => claimed,
        }
    }

    /// The code and effects of the save, given the commit the evidence
    /// check found to be this request's.
    pub(crate) fn result(self, commit: Option<git2::Oid>) -> (ResultCode, Effects) {
        // What the operation still owes, as the code that says so.
        let owed = match self {
            Self::IdentityRequired => {
                return (ResultCode::IdentityRequired, Effects::not_requested());
            }
            Self::Saved {
                discovered: true, ..
            } => None,
            Self::Saved {
                discovered: false, ..
            } => Some(ResultCode::DiscoveryPending),
            Self::RegistrationPending { .. } => Some(ResultCode::RegistrationPending),
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
            discovery: match (owed, commit) {
                (Some(_), _) => DiscoveryEffect::Pending,
                (None, Some(_)) => DiscoveryEffect::Current,
                (None, None) => DiscoveryEffect::NotRequested,
            },
            commit_oid: commit.map(|commit| commit.to_string()),
            ..Effects::not_requested()
        };
        let code = match (owed, commit) {
            (Some(owed), _) => owed,
            (None, Some(_)) => ResultCode::Ok,
            (None, None) => ResultCode::AlreadyApplied,
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
        // A result that names no repository has none to inspect: it is
        // of a request of the application, as a key generation is. When
        // its record was left accepted, as that of a generation that was
        // interrupted is, the same request is what continues it; when
        // the record is finished, as that of a generation whose files
        // were retained is, nothing is offered, and `operation show`
        // says what the operation left.
        ResultCode::RecoveryRequired if root.is_none() => {
            if retry {
                vec![retry_request()]
            } else {
                Vec::new()
            }
        }
        ResultCode::RecoveryRequired
        | ResultCode::InvalidSlugConfiguration
        | ResultCode::InvalidConfiguration => vec![at_root(RecoveryActionKind::RepoInspect)],
        // A rebuild is of one repository and takes its root. A result
        // that names no repository, as that of a key command on an index
        // that cannot be read is, has no root to give: an action without
        // its argument could not be carried out, so none is offered.
        ResultCode::RepositoryNotRegistered | ResultCode::IndexUnavailable => match root {
            Some(_) => vec![at_root(RecoveryActionKind::IndexRebuild)],
            None => Vec::new(),
        },
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
        // refreshed or rebuilt, a key generated, imported, selected,
        // cleared or removed. The same request is offered when its record
        // was left accepted, and nothing otherwise.
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
        // A key that is not registered, a key that is selected, and a
        // key store that cannot be used: the code says what is missing,
        // and the command that resolves it is the caller's to choose.
        | ResultCode::KeyNotFound
        | ResultCode::SelectedKeyInUse
        | ResultCode::KeyUnavailable
        // Not decided: the codes only a command that is not bound yet
        // returns. They share the arm above so that the match is complete,
        // and for no other reason: the design's recovery table gives
        // several of them an action, and the task that binds the command
        // moves its codes to an arm of their own.
        | ResultCode::InvalidId
        | ResultCode::PathNotFound
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
        | ResultCode::KeyRejected
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
