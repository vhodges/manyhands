//! Domain outcomes and errors as the codes and effects of a result.
//!
//! Every mapping here is a `match` with no wildcard arm, so a variant added
//! later does not compile until it is given a code. None formats the error
//! it maps.

use std::path::Path;

use crate::results::{
    CheckpointEffect, DiscoveryEffect, Effects, Envelope, RecoveryAction, RecoveryActionKind,
    ResultCode, WriteEffect,
};

use super::{
    super::{
        GeneratedKeyDeletionPreflight, LocalCheckpoint, OperationId, PublicationRemoteOutcome,
        PublishPendingReason, RefreshOutcome, RemoteOutcome, RemoveRegistrationOutcome,
        RepositoryErrorKind, RepositoryOperation, RepositoryService, SaveOutcome,
        SynchronizationError, SynchronizationOutcome, SynchronizationResult,
        UnregisterSharedKeyOutcome,
        bridges::abandon::stale_heartbeat,
        keys::{
            DeleteGeneratedKeyOutcome, GenerateSharedKeyOutcome, KeyMaterialErrorKind,
            KeyRegistryChange, MarkedRegistryChange,
        },
        leases_are_free, recovery,
        transport::SshTransportErrorKind,
    },
    bind::{comment, resume},
    command::Command,
    identity::RequestId,
    records,
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
        // A domain operation says a repository is not enabled only of
        // one it resolved, which is registered. `repository_not_enabled`
        // is not that: it is of a repository that is not registered and
        // that Git shows was never enabled; see `bind::never_enabled`.
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
        // What `repair_item` refuses a repair for.
        RepositoryErrorKind::NotRepairable => ResultCode::NotRepairable,
        RepositoryErrorKind::AmbiguousIdentity => ResultCode::IdentityAmbiguous,
        RepositoryErrorKind::InvalidItemSource => ResultCode::InvalidInput,
        // The domain guard of a closed ticket (C5); the bindings refuse
        // earlier with the same code.
        RepositoryErrorKind::TicketClosed => ResultCode::TicketClosed,
        // The boundary allocates operation IDs, so a mismatch is its own
        // fault.
        RepositoryErrorKind::OperationMismatch => ResultCode::InternalError,
        // The request's operation was abandoned while a call of the
        // request was under way: its record went with the row. What the
        // request was based on is no longer there to go on with; an item
        // is read again and a confirmed command prepared again.
        RepositoryErrorKind::OperationAbandoned => ResultCode::ExternalChange,
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
/// reports when the host presents another key than the one approved; see
/// `bind::host`.
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
        // The host gave no key that could be identified, so there is
        // none to approve, and sending the request again changes
        // nothing: not a transient failure, and not one a host approval
        // answers. Something about the host or the way to it has to be
        // looked at first.
        SshTransportErrorKind::HostVerificationUnavailable => ResultCode::RecoveryRequired,
        SshTransportErrorKind::TransportUnavailable => ResultCode::TransportUnavailable,
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
        // Only a caller set to refuse divergence is told this. The
        // bindings merge, and none of their requests is answered with it.
        SynchronizationError::MergeRequired { target: _ } => ResultCode::MergeRequired,
        SynchronizationError::PushRejected => ResultCode::PushRejected,
        SynchronizationError::ExternalChange => ResultCode::ExternalChange,
        // A merge left to resolve or to repair outside, or a
        // synchronization that cannot go on as it stands: the repository
        // is what is looked at. A binding whose own merge stopped on a
        // conflict says the conflict's next step in its data.
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

/// The code of a change to the key registry as it is recorded against
/// its operation: `ok` for one that changed the registry, and a no-op for
/// one that found it already as asked, whichever request made it so.
pub(crate) fn key_registry_change_code(change: &KeyRegistryChange) -> ResultCode {
    if change.changed {
        ResultCode::Ok
    } else {
        ResultCode::AlreadyApplied
    }
}

/// The code of what a change to the key registry made under a mark
/// returns. A setting that has moved since the request observed it is
/// someone else's change, and a key that is not registered is the input's
/// fault as it is before the request is accepted.
pub(crate) fn marked_registry_change_code(made: &MarkedRegistryChange) -> ResultCode {
    match made {
        MarkedRegistryChange::Recorded(change) => key_registry_change_code(change),
        MarkedRegistryChange::Moved => ResultCode::ExternalChange,
        MarkedRegistryChange::NotRegistered => ResultCode::KeyNotFound,
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

/// The code of what `remove_registration` returns. A root that is not
/// registered is the request's end state, whoever removed it.
pub(crate) fn remove_registration_outcome_code(outcome: &RemoveRegistrationOutcome) -> ResultCode {
    match outcome {
        RemoveRegistrationOutcome::Removed => ResultCode::Ok,
        RemoveRegistrationOutcome::NotRegistered => ResultCode::AlreadyApplied,
    }
}

/// The code of what `delete_generated_key` returns. `key delete` never
/// asks for a deletion it has no consent for, so it is never answered
/// `Cancelled`, and reports that as its own fault.
pub(crate) fn delete_key_outcome_code(outcome: &DeleteGeneratedKeyOutcome) -> ResultCode {
    match outcome {
        DeleteGeneratedKeyOutcome::Deleted => ResultCode::Ok,
        DeleteGeneratedKeyOutcome::AlreadyDeleted => ResultCode::AlreadyApplied,
        DeleteGeneratedKeyOutcome::Cancelled => ResultCode::Cancelled,
        DeleteGeneratedKeyOutcome::RecoveryRequired(_) => ResultCode::RecoveryRequired,
    }
}

/// What the preflight of a key deletion found, as a code: a key that
/// can be deleted needs the confirmation. `prepare` gives that key its
/// preview instead, and answers with the code for every other.
pub(crate) fn key_deletion_preflight_code(preflight: &GeneratedKeyDeletionPreflight) -> ResultCode {
    match preflight {
        GeneratedKeyDeletionPreflight::ConfirmationRequired(_) => ResultCode::ConfirmationRequired,
        GeneratedKeyDeletionPreflight::NotRegistered => ResultCode::KeyNotFound,
        GeneratedKeyDeletionPreflight::ImportedKey => ResultCode::KeyNotDeletable,
        GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared => ResultCode::SelectedKeyInUse,
    }
}

/// The code of what a synchronization returns without an error. One
/// whose index hand-off is owed is `discovery_pending`, whatever it
/// came to before that.
pub(crate) fn synchronization_result_code(result: &SynchronizationResult) -> ResultCode {
    match result {
        SynchronizationResult::Complete(outcome) => synchronization_outcome_code(outcome),
        SynchronizationResult::IndexPending(_) => ResultCode::DiscoveryPending,
    }
}

/// The code of what a synchronization came to. A remote that already
/// held the commit is a no-op; a repository with no publication remote
/// has nowhere to publish to.
pub(crate) fn synchronization_outcome_code(outcome: &SynchronizationOutcome) -> ResultCode {
    match outcome {
        SynchronizationOutcome::Published { target: _, oid: _ } => ResultCode::Ok,
        SynchronizationOutcome::AlreadyCurrent { target: _, oid: _ } => ResultCode::AlreadyApplied,
        SynchronizationOutcome::PublishPending {
            target: _,
            local_oid: _,
            reason: PublishPendingReason::NoPublicationRemote,
        } => ResultCode::PublicationRemoteRequired,
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

/// The code and effects of an enablement that registered a repository
/// enabled already, and wrote and committed nothing: `ok` once the index
/// hand-off completed, and otherwise the hand-off owed. A registration
/// is no effect the result shows: the answer carries it as a local
/// change.
pub(crate) fn registration_result(discovered: bool) -> (ResultCode, Effects) {
    let (code, discovery) = if discovered {
        (ResultCode::Ok, DiscoveryEffect::Current)
    } else {
        (ResultCode::DiscoveryPending, DiscoveryEffect::Pending)
    };
    let effects = Effects {
        write: WriteEffect::Unchanged,
        checkpoint: CheckpointEffect::Unchanged,
        discovery,
        ..Effects::not_requested()
    };
    (code, effects)
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

/// The action that names the request that began an operation: what a
/// resume offers for work that needs that request's input.
pub(crate) fn original_request(request_id: String) -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [(REQUEST_ARGUMENT, request_id.into())],
    )
}

/// The action that offers the request `request_id` itself again: what
/// a result offers when its record was left accepted and the same
/// request is what goes on, whatever its code offers of its own.
pub(crate) fn same_request(request_id: RequestId) -> RecoveryAction {
    original_request(request_id.to_string())
}

/// The action a path below a repository's root is answered with: to
/// inspect the root the path lies in, which the action names. A read
/// that resolves such a path suggests the same; this is for a binding
/// that finds it without that read.
pub(crate) fn inspect_the_root_it_lies_in(root: &Path) -> Vec<RecoveryAction> {
    crate::results::absolute_path_string(root)
        .map(|root| {
            RecoveryAction::new(
                RecoveryActionKind::RepoInspect,
                [(ROOT_ARGUMENT, root.into())],
            )
        })
        .into_iter()
        .collect()
}

/// What a result names, for the actions it offers.
pub(crate) struct Named<'a> {
    pub(crate) code: ResultCode,
    /// The command the result is of.
    pub(crate) command: Command,
    /// The repository the result names, when it names one.
    pub(crate) root: Option<&'a str>,
    /// `None` for a result of `prepare`, which has no request.
    pub(crate) request_id: Option<RequestId>,
    pub(crate) operation_id: Option<OperationId>,
    /// Whether the request's record was left accepted.
    pub(crate) retry: bool,
    /// Whether the call meant to finish or delete the request's record,
    /// that did not apply, and a record still stands: the write failed,
    /// or the record was no longer this call's to settle and is accepted
    /// still or was finished by another call. The result is then not
    /// the request's last word, whatever its code.
    pub(crate) unsettled: bool,
}

/// The actions a result offers, from its answer. This and
/// `RepositoryService::offering` are the two halves of the one place
/// that decides what a result offers and in what order: this one from
/// what the call's answer says, that one from what the journal holds
/// once the call is over.
///
/// In order:
///
/// 1. An answer that `offers_nothing` offers nothing of its own.
/// 2. `suggested` is what the binding or a read suggested. It is kept,
///    unless it is empty or holds an action that lacks the root it
///    takes: a read suggests its action with the root it knows, and one
///    that failed before it knew any names none. Then the actions are
///    the code's own, made with this result's root; see `recovery`.
/// 3. A result whose record could not be settled offers the same
///    request, first, with whatever the rules above gave it: a record
///    still stands, and sending the request again finishes it where it
///    is accepted and reads its stored result where another call
///    finished it. Nothing else of the result is changed for that.
/// 4. A host that is not trusted is offered its approval first; see
///    `with_host_action`. `authority` is the host as the result's data
///    names it.
pub(crate) fn offered(
    named: &Named<'_>,
    suggested: Vec<RecoveryAction>,
    offers_nothing: bool,
    authority: Option<&str>,
) -> Vec<RecoveryAction> {
    let mut actions = if offers_nothing {
        Vec::new()
    } else {
        suggested_or(suggested, || {
            recovery(
                named.code,
                Some(named.command),
                named.root,
                named.request_id,
                named.operation_id,
                named.retry,
            )
        })
    };
    if named.unsettled
        && let Some(request_id) = named.request_id
        && !actions
            .iter()
            .any(|action| action.action == RecoveryActionKind::RequestRetry)
    {
        actions.insert(0, same_request(request_id));
    }
    with_host_action(actions, named.code, named.root, authority)
}

/// Rule 2 of `offered`: what was suggested, unless it is empty or holds
/// an action that lacks the root it takes; then `own`.
fn suggested_or(
    suggested: Vec<RecoveryAction>,
    own: impl FnOnce() -> Vec<RecoveryAction>,
) -> Vec<RecoveryAction> {
    let rootless = suggested
        .iter()
        .any(|action| action.arguments.is_empty() && !action.action.argument_keys().is_empty());
    if suggested.is_empty() || rootless {
        own()
    } else {
        suggested
    }
}

/// The actions a failed `host observe` offers. It is a read over the
/// network, of no command of `execute` and of no request: what the read
/// that failed suggested is kept, as rule 2 of `offered` keeps it, and
/// otherwise the action is the code's own with no command's rules
/// applied, so that the same code on the same repository offers what it
/// offers a mutation: a rebuild for a repository that is not registered
/// or an index that cannot be read, an inspection for a configuration
/// that is not valid. Nothing is offered to be sent again or previewed:
/// there is no request and no confirmation.
pub(crate) fn offered_by_observation(
    code: ResultCode,
    root: Option<&str>,
    suggested: Vec<RecoveryAction>,
) -> Vec<RecoveryAction> {
    suggested_or(suggested, || recovery(code, None, root, None, None, false))
}

/// Whose envelope `RepositoryService::offering` is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallKind {
    /// A call of `execute`.
    Execute,
    /// A call of `prepare`, which names no request and no operation.
    Prepare,
}

/// What a client does about a code that stopped a request. Every code is
/// named: one added later does not compile until its action is decided.
///
/// `command` is the command the result is of, and `None` for a result
/// of `host observe`, which is no command of `execute`: no rule of a
/// command then applies. `root` is the repository
/// the result names, when it names one. `request_id` is the request's,
/// and `None` for a result of `prepare`, which has none: nothing is
/// then offered to be sent again. `retry` says the request's record was
/// left accepted, so repeating the same request continues it; a code with
/// an action of its own keeps that one, and `external_change` is never
/// offered the same request: for a confirmed command it says instead
/// whether a new preview can be used.
///
/// A synchronization that stopped with its record left accepted holds
/// the repository's reservation, whatever stopped it, and the same
/// request is what takes the reservation up again: once the cause is
/// dealt with it goes on, and after `request cancel` it ends cancelled
/// and lets the reservation go. So such a result always offers the same
/// request, first, with whatever else its code offers.
fn recovery(
    code: ResultCode,
    command: Option<Command>,
    root: Option<&str>,
    request_id: Option<RequestId>,
    operation_id: Option<OperationId>,
    retry: bool,
) -> Vec<RecoveryAction> {
    let at_root =
        |action| RecoveryAction::new(action, root.map(|root| (ROOT_ARGUMENT, root.into())));
    let retry_request = || -> Vec<RecoveryAction> {
        request_id
            .map(|request_id| {
                RecoveryAction::new(
                    RecoveryActionKind::RequestRetry,
                    [(REQUEST_ARGUMENT, request_id.to_string().into())],
                )
            })
            .into_iter()
            .collect()
    };
    let prepare_again = || vec![RecoveryAction::new(RecoveryActionKind::RequestPrepare, [])];
    let mut actions = match code {
        // A hand-off that is no operation's to owe: nothing is pending
        // under the request's operation ID, and any refresh brings the
        // index up to date. Not so while the request's record is left
        // accepted: its refresh's row could not be closed, and refuses
        // every other refresh, so the same request is what takes it up.
        ResultCode::DiscoveryPending if command.is_some_and(Command::hands_off_to_any_refresh) => {
            if retry {
                retry_request()
            } else {
                vec![at_root(RecoveryActionKind::IndexRefresh)]
            }
        }
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
            // A command that makes its repository has none yet to set an
            // identity on: the request is sent again with one.
            if command.is_some_and(Command::creates_its_repository) {
                Vec::new()
            } else {
                vec![at_root(RecoveryActionKind::RepoIdentitySet)]
            }
        }
        // No consent the request could be run with: none was presented,
        // or the one presented is gone or too old. A new preview gives
        // one.
        ResultCode::ConfirmationRequired
        | ResultCode::ConfirmationExpired
        | ResultCode::ConfirmationNotFound => prepare_again(),
        // A result that names no repository has none to inspect: it is
        // of a request of the application, as a key generation is. When
        // its record was left accepted, as that of a generation that was
        // interrupted is, the same request is what continues it; when
        // the record is finished, as that of a generation whose files
        // were retained is, nothing is offered, and `operation show`
        // says what the operation left.
        ResultCode::RecoveryRequired if root.is_none() => {
            if retry {
                retry_request()
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
        // When the record of such a request was left accepted, the same
        // request is what goes on, as for every code with no action of
        // its own: a key registry change that could not reach the
        // registry on its retry keeps its record, and says so.
        ResultCode::RepositoryNotRegistered | ResultCode::IndexUnavailable => match root {
            Some(_) => vec![at_root(RecoveryActionKind::IndexRebuild)],
            None if retry => retry_request(),
            None => Vec::new(),
        },
        // An external change is not answered by the same request: it is
        // refused again for as long as the change stands, also when its
        // record stays accepted because its operation is still in flight.
        // For a command that needs confirmation, what was previewed is no
        // longer what is there, and a new preview shows what is. Not so
        // when the record was left accepted: the request then has an
        // operation of its own pending, which refuses every other
        // operation on the repository, a new request with a new preview
        // among them. What is offered is to look at the repository: with
        // what was previewed put back, the same request goes on. For an
        // item write the caller reads the item again and submits a new
        // request, and no action is offered.
        //
        // A result that names no repository has none to look at: it is
        // of a key deletion that was interrupted, whose key or files
        // are no longer what its own operation left. That operation
        // refuses any other deletion of the key, and with what it left
        // put back the same request goes on; nothing is offered.
        ResultCode::ExternalChange => match (
            command.is_some_and(Command::needs_confirmation),
            retry,
            root,
        ) {
            (true, false, _) => prepare_again(),
            (true, true, Some(_)) => vec![at_root(RecoveryActionKind::RepoInspect)],
            (true, true, None) | (false, _, _) => Vec::new(),
        },
        // Nothing was accepted, or what was accepted is still there:
        // either way the same request is the one to send. So it is for
        // a failure that passes by itself: a poll that is giving way, a
        // remote or a network that could not be reached.
        ResultCode::Busy
        | ResultCode::PollYielding
        | ResultCode::RemoteUnavailable
        | ResultCode::TransportUnavailable => retry_request(),
        // The commit exists and the registry write failed: the same
        // request completes the registration.
        ResultCode::RegistrationPending => retry_request(),
        // What a resume is answered with when the operation it names
        // needs the input of the request that began it, or is no
        // journal's. The resume sent again is refused the same way, so
        // it is never offered, whatever became of its record. The
        // request that is to be sent is another one, which this
        // function is not told of: the binding names it, when a record
        // holds it; see `original_request`.
        ResultCode::OriginalRequestRequired | ResultCode::OperationNotFound => Vec::new(),
        // Decided: these have no action of their own. They are the codes
        // a bound command, or the boundary itself, ends with: a write of
        // a ticket or a document, a remote added or selected, an index
        // refreshed or rebuilt, a key generated, imported, selected,
        // cleared, removed or deleted, a repository created, enabled or
        // removed from the list, a remote removed. The same
        // request is offered when its record was left accepted, and
        // nothing otherwise.
        ResultCode::Ok
        | ResultCode::AlreadyApplied
        // A command line the CLI could not run: no action corrects it.
        | ResultCode::UsageError
        | ResultCode::CommandUnavailable
        | ResultCode::Cancelled
        | ResultCode::InternalError
        | ResultCode::InvalidInput
        | ResultCode::InvalidPath
        | ResultCode::NotRepository
        | ResultCode::NotRepositoryRoot
        | ResultCode::BareRepository
        // A repository that was never enabled. Enabling it is the
        // user's to ask for and needs a confirmation of its own; no
        // registered action names it, so none is offered, and never the
        // rebuild: a rebuild does not enable a repository.
        | ResultCode::RepositoryNotEnabled
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
        // A key that is not registered, a key that is selected, a key
        // Manyhands did not make and so does not delete, and a key store
        // that cannot be used: the code says what is missing, and the
        // command that resolves it is the caller's to choose.
        | ResultCode::KeyNotFound
        | ResultCode::SelectedKeyInUse
        | ResultCode::KeyNotDeletable
        | ResultCode::KeyUnavailable
        // A caller error, which no action corrects: a confirmation that
        // is of another command, repository or input, or that another
        // request has used. The caller uses a new preview.
        | ResultCode::ConfirmationMismatch
        | ResultCode::ConfirmationUsed
        // What a repair is refused for. A file or a path a repair is
        // not made of stays a visible problem, and no command of this
        // contract moves or rewrites it. An identity that is ambiguous
        // is the user's to settle: which file is to keep the ID, or
        // which other ID the corrected source is to carry. Neither is
        // answered by an action. Nor is a path a repair finds no file
        // at: the caller reads the list of nonconforming entries again.
        // A read that found the index behind suggests its refresh
        // itself, and that suggestion is kept.
        | ResultCode::NotRepairable
        | ResultCode::IdentityAmbiguous
        | ResultCode::PathNotFound
        // The publication remote is not removed while it is selected:
        // the selection is cleared first, which is the caller's to do.
        | ResultCode::PublicationRemoteInUse
        // A synchronization with nowhere to publish to: a publication
        // remote is selected first, which is the caller's to do.
        | ResultCode::PublicationRemoteRequired
        // The host presented another key than the one a host approval
        // names, and nothing was pinned. No action answers that: the
        // result's data holds the key the host presented, and whether
        // that key is to be trusted is the caller's to decide, with a
        // new preview of it.
        | ResultCode::HostMismatch
        // What stops a synchronization once it holds the reservation:
        // history to merge, a branch deleted or a push refused at the
        // remote, a key that is missing, refused or locked, a host that
        // is not trusted. None is answered by an action of the code's
        // own here. The same request is offered, by the rule for a
        // synchronization below, or its cancellation where the same
        // request cannot go on, and `with_host_action` adds the
        // approval of a host the result names.
        | ResultCode::MergeRequired
        | ResultCode::RemoteBranchDeleted
        | ResultCode::PushRejected
        | ResultCode::NoSelectedKey
        | ResultCode::KeyRejected
        | ResultCode::UnlockRequired
        | ResultCode::UnlockFailed
        | ResultCode::HostApprovalRequired
        | ResultCode::HostReplacementRequired
        // Codes no command of `execute` or `prepare` returns: the first
        // three are a read's, and the fourth is of `request show` and
        // `request cancel`, which offer nothing. They share the arm
        // above so that the match is complete, and for no other reason.
        | ResultCode::InvalidId
        | ResultCode::PublicKeyUnavailable
        | ResultCode::AuthorityNotFound
        | ResultCode::RequestNotFound => {
            if retry {
                retry_request()
            } else {
                Vec::new()
            }
        }
    };
    if never_finished_by_the_same_request(code, command) {
        actions.retain(|action| action.action != RecoveryActionKind::RequestRetry);
    }
    // The rule for a synchronization whose reservation is held. The
    // hand-off of one that published needs no input from the caller,
    // and keeps the action that says so.
    let resumes = code == ResultCode::DiscoveryPending;
    // Two causes the same request cannot go on from, whatever is done
    // about them. The operation recorded the branch as it was: once
    // history is merged or rebased the branch is another, and the
    // operation refuses it; and a branch deleted at the remote stays
    // deleted. The same request is not offered for what it cannot do.
    // The reservation is let go by `request cancel` and one more call
    // of the request, and a new request goes on: that exit is what is
    // offered, for a request whose record was left accepted, since the
    // reservation is held for as long as it is. `offering_cancel` keeps
    // the action only while the reservation is the request's own.
    let never = never_finished_by_the_same_request(code, command);
    if never
        && retry
        && let Some(request_id) = request_id
    {
        actions.push(RecoveryAction::new(
            RecoveryActionKind::RequestCancel,
            [(REQUEST_ARGUMENT, request_id.to_string().into())],
        ));
    }
    let offered = actions
        .iter()
        .any(|action| action.action == RecoveryActionKind::RequestRetry);
    if command.is_some_and(Command::stops_at_safe_points) && retry && !resumes && !never && !offered
    {
        actions.splice(0..0, retry_request());
    }
    actions
}

/// Whether `code`, from a synchronization, is one the same request
/// never gets past; see the closing rule of `recovery`.
fn never_finished_by_the_same_request(code: ResultCode, command: Option<Command>) -> bool {
    command.is_some_and(Command::stops_at_safe_points)
        && (code == ResultCode::MergeRequired || code == ResultCode::RemoteBranchDeleted)
}

/// The argument of a recovery action that names a host.
const AUTHORITY_ARGUMENT: &str = "authority";

/// Adds the approval of a host to what a result offers, when the result
/// is that a host is not trusted and its data names the host:
/// `host.approve` for a host with no pin, `host.replace` for one that
/// presented another key than its pin. `authority` is the host as the
/// data gives it, and `root` the repository whose remote names it.
///
/// The approval comes first: it is what deals with the cause, and the
/// same request, which the result also offers when its reservation is
/// held, is what follows it. Nothing is added to a result of another
/// code, or to one that cannot give the action both its arguments.
fn with_host_action(
    mut recovery: Vec<RecoveryAction>,
    code: ResultCode,
    root: Option<&str>,
    authority: Option<&str>,
) -> Vec<RecoveryAction> {
    let action = if code == ResultCode::HostApprovalRequired {
        RecoveryActionKind::HostApprove
    } else if code == ResultCode::HostReplacementRequired {
        RecoveryActionKind::HostReplace
    } else {
        return recovery;
    };
    let (Some(root), Some(authority)) = (root, authority) else {
        return recovery;
    };
    if recovery.iter().any(|offered| offered.action == action) {
        return recovery;
    }
    recovery.insert(
        0,
        RecoveryAction::new(
            action,
            [
                (AUTHORITY_ARGUMENT, authority.into()),
                (ROOT_ARGUMENT, root.into()),
            ],
        ),
    );
    recovery
}

/// Adds `operation.abandon` to what a result of a request that stopped
/// offers, where a pending local operation stands in the way.
///
/// A pending local operation refuses every other operation on its
/// repository, and is cleared by a repeat of its own request or, where
/// that cannot be, by abandoning it. There are two rules:
///
/// 1. **Another's operation, or any.** A result that already says to
///    look at the repository, `recovery_required` or the
///    `external_change` of a confirmed request, is offered the abandon
///    of each operation in `pending`. The offer is made whatever the
///    result's own cause was: an active reservation or a pending merge
///    can be why a request was `recovery_required` while a local
///    operation is pending as well. That is harmless: the preview of
///    the abandon says what it is. A pending operation does refuse the
///    repository's other operations, with one exception, which is
///    offered all the same: a comment that waits for its registration
///    refuses only the reservation of a synchronization, so its abandon
///    can be offered to a request it was not in the way of.
///
///    An operation among them that is in `resumable` is offered its
///    resume as well, before `repo.inspect`: the way on comes before
///    the way out, as it does for a request's own operation. The
///    request that began such an operation is not offered: it is
///    another caller's to send.
/// 2. **The request's own.** A result that left the request's record
///    accepted with the request's own local operation pending is offered
///    the abandon of that operation, whatever its code, with
///    `repo.inspect` before it: the same request may still finish the
///    work, and what it keeps of its own actions says so, but where it
///    cannot, this is the way out and the result is where its caller
///    learns the operation's ID. `own` is that operation, and `None` for
///    a result that left no such record.
///
/// `pending` is the operation IDs of the repository's pending local rows
/// that nothing is seen against abandoning, oldest first: a row a call
/// is at work under, or one the remote or key journal also holds, is
/// not in it, and so is not offered under either rule. `resumable` is
/// those of them that are not the request's own and that a resume would
/// take up as things are; see `resume::takes_up`.
///
/// Nothing is added to a result that names no repository, since the
/// action takes its root. The action follows `repo.inspect` and never
/// replaces it or any other action: what an operation left is looked at
/// before it is given up.
pub(crate) fn offering_abandon(
    mut recovery: Vec<RecoveryAction>,
    code: ResultCode,
    root: Option<&str>,
    pending: &[String],
    resumable: &[String],
    own: Option<&str>,
) -> Vec<RecoveryAction> {
    let Some(root) = root else {
        return recovery;
    };
    let offered: Vec<&str> = if abandon_may_be_offered(code, &recovery) {
        pending.iter().map(String::as_str).collect()
    } else {
        own.filter(|own| pending.iter().any(|pending| pending == own))
            .into_iter()
            .collect()
    };
    if offered.is_empty() {
        return recovery;
    }
    let inspects = recovery
        .iter()
        .any(|action| action.action == RecoveryActionKind::RepoInspect);
    if !inspects {
        recovery.push(RecoveryAction::new(
            RecoveryActionKind::RepoInspect,
            [(ROOT_ARGUMENT, root.into())],
        ));
    }
    let resumes: Vec<RecoveryAction> = offered
        .iter()
        .filter(|operation_id| resumable.iter().any(|resumable| resumable == *operation_id))
        .filter(|operation_id| {
            !recovery.iter().any(|action| {
                action.action == RecoveryActionKind::OperationResume
                    && action.operation_id.as_deref() == Some(**operation_id)
            })
        })
        .map(|operation_id| {
            RecoveryAction::for_operation(RecoveryActionKind::OperationResume, *operation_id, [])
        })
        .collect();
    let inspect = recovery
        .iter()
        .position(|action| action.action == RecoveryActionKind::RepoInspect)
        .unwrap_or(recovery.len());
    recovery.splice(inspect..inspect, resumes);
    for operation_id in offered {
        let already = recovery.iter().any(|action| {
            action.action == RecoveryActionKind::OperationAbandon
                && action.operation_id.as_deref() == Some(operation_id)
        });
        if !already {
            recovery.push(RecoveryAction::for_operation(
                RecoveryActionKind::OperationAbandon,
                operation_id,
                [(ROOT_ARGUMENT, root.into())],
            ));
        }
    }
    recovery
}

/// Whether a result of `code` that offers `recovery` already says to
/// look at the repository for something a pending local operation can be
/// the cause of: the first rule of `offering_abandon`.
fn abandon_may_be_offered(code: ResultCode, recovery: &[RecoveryAction]) -> bool {
    let inspects = recovery
        .iter()
        .any(|action| action.action == RecoveryActionKind::RepoInspect);
    matches!(
        code,
        ResultCode::RecoveryRequired | ResultCode::ExternalChange
    ) && inspects
}

impl RepositoryService {
    /// What an envelope offers once its call is over: the actions its
    /// answer gave it, see `offered`, corrected and added to from what
    /// the journal holds now. In order:
    ///
    /// 1. For a call of `execute`, `operation.resume` is kept only where
    ///    a resume would take the operation up; see `offering_resume`.
    ///    `request.cancel` is kept only while the reservation is the
    ///    request's own; see `offering_cancel`.
    /// 2. `operation.abandon` is added where a pending local operation
    ///    stands in the way, with `operation.resume` before
    ///    `repo.inspect` and the abandon after it; see
    ///    `offering_abandon`.
    ///
    /// Every envelope of `execute` and of `prepare` passes through here,
    /// a replayed one among them.
    pub(super) fn offering<T>(&self, envelope: Envelope<T>, call: CallKind) -> Envelope<T> {
        let envelope = match call {
            CallKind::Execute => self.offering_cancel(self.offering_resume(envelope)),
            CallKind::Prepare => envelope,
        };
        self.offering_abandon(envelope)
    }

    /// Keeps `request.cancel` among what an envelope offers only while
    /// the repository's reservation is the operation the envelope names:
    /// the request's own. The action is the exit of a synchronization
    /// that holds its reservation and cannot go on; where the reservation
    /// is another operation's, or there is none, a cancellation would let
    /// nothing of this request's go.
    ///
    /// The reservation is read only for an envelope that offers the
    /// action. One that cannot be read drops it: the envelope then offers
    /// what it did before the action existed.
    ///
    /// A comment's request holds its comment's operation, and the
    /// reservation is held by the synchronization that publishes the
    /// comment: that synchronization is the request's own as well (R-5).
    fn offering_cancel<T>(&self, mut envelope: Envelope<T>) -> Envelope<T> {
        let offers = envelope
            .recovery
            .iter()
            .any(|action| action.action == RecoveryActionKind::RequestCancel);
        if !offers {
            return envelope;
        }
        let own = envelope
            .scope
            .repository
            .as_deref()
            .zip(envelope.operation_id.as_deref())
            .is_some_and(|(root, operation_id)| {
                let Some(active) = self.active_remote_operation(Path::new(root)).ok().flatten()
                else {
                    return false;
                };
                let active = active.operation_id();
                active.to_string() == operation_id
                    || OperationId::parse(operation_id).is_ok_and(|operation_id| {
                        let bound = self.read_session(RepositoryOperation::Read, |connection| {
                            comment::comment_synchronization(
                                connection,
                                Path::new(root),
                                operation_id,
                            )
                        });
                        // An envelope carries no second error: a registry
                        // that cannot say whose the reservation is offers
                        // no cancellation of it.
                        match bound {
                            Ok(Some((comment, synchronization))) => {
                                comment == operation_id && synchronization == active
                            }
                            Ok(None) | Err(_) => false,
                        }
                    })
            });
        if !own {
            envelope
                .recovery
                .retain(|action| action.action != RecoveryActionKind::RequestCancel);
        }
        envelope
    }

    /// Keeps `operation.resume` among what an envelope offers only
    /// where a resume of the operation it names would take the
    /// operation up, as things are once the call is over: it is asked
    /// with the question a resume itself asks; see `resume::takes_up`.
    ///
    /// An index hand-off can be owed by a row a resume leaves to its
    /// request: one that reached its hand-off without recording the
    /// step that says its own work is done, as an enablement or a
    /// selection does when it is taken up after its commit. A resume of
    /// it is `original_request_required`, so the action would be a dead
    /// end. The request's own result offers the same request in its
    /// place, which its record, left accepted, is there for. A resume's
    /// own result offers nothing in its place.
    fn offering_resume<T>(&self, mut envelope: Envelope<T>) -> Envelope<T> {
        let offers = envelope
            .recovery
            .iter()
            .any(|action| action.action == RecoveryActionKind::OperationResume);
        if !offers {
            return envelope;
        }
        let root = envelope.scope.repository.clone();
        let own = envelope.operation_id.clone();
        let request_id = envelope.request_id.clone();
        let mut offered = Vec::with_capacity(envelope.recovery.len());
        for action in std::mem::take(&mut envelope.recovery) {
            if action.action != RecoveryActionKind::OperationResume {
                offered.push(action);
                continue;
            }
            let taken_up = root
                .as_deref()
                .zip(action.operation_id.as_deref())
                .and_then(|(root, operation_id)| {
                    Some((root, OperationId::parse(operation_id).ok()?))
                })
                .is_some_and(|(root, operation_id)| resume::takes_up(self, root, operation_id));
            if taken_up {
                offered.push(action);
            } else if own.is_some()
                && action.operation_id == own
                && let Some(request_id) = request_id.clone()
            {
                offered.push(original_request(request_id));
            }
        }
        envelope.recovery = offered;
        envelope
    }

    /// Adds `operation.abandon` to an envelope of a request that
    /// stopped, where a pending local operation is what stands in the way;
    /// see `offering_abandon` for the two rules.
    ///
    /// The journal and the records are read only for an envelope either
    /// rule can apply to, after everything the call did, so the action
    /// names what is pending now. That is also so for a result given
    /// again from a finished record: its stored actions are none, and
    /// what it is offered is what is true when it is read.
    ///
    /// An operation is offered only when the lookup can see nothing
    /// against abandoning it: it is an operation of the local journal
    /// alone, no call is at work under it with a heartbeat, and no call
    /// holds the repository's lease or its bootstrap lease, which is
    /// looked at without waiting and let go at once. The look is taken
    /// last, and only when there is something to offer.
    ///
    /// An operation offered so that is another request's is asked, with
    /// the question a resume itself asks, whether a resume would take
    /// it up: where it would, the resume is offered before the abandon.
    ///
    /// A journal that cannot be read adds nothing: the envelope keeps
    /// what it has.
    fn offering_abandon<T>(&self, mut envelope: Envelope<T>) -> Envelope<T> {
        if envelope.code.failure_class().is_none() {
            return envelope;
        }
        let Some(root) = envelope.scope.repository.clone() else {
            return envelope;
        };
        let inspects = abandon_may_be_offered(envelope.code, &envelope.recovery);
        // The request's own operation, when its envelope names one.
        let own = envelope
            .operation_id
            .as_deref()
            .and_then(|operation_id| OperationId::parse(operation_id).ok())
            .zip(envelope.request_id.clone());
        if !inspects && own.is_none() {
            return envelope;
        }
        let read = self.read_session(RepositoryOperation::Read, |connection| {
            let stale_before = stale_heartbeat();
            let mut pending = Vec::new();
            for (operation_id, row) in recovery::pending_rows(connection, &root)? {
                if !row.has_live_heartbeat(stale_before)
                    && !recovery::held_by_another_journal(connection, &operation_id)?
                {
                    pending.push(operation_id);
                }
            }
            // Whether the request's record was left accepted, holding
            // that operation as a local one.
            let own = match &own {
                Some((operation_id, request_id)) => {
                    let owner = records::operation_owner(connection, &root, *operation_id)?;
                    (owner.as_deref() == Some(request_id.as_str()))
                        .then(|| operation_id.to_string())
                }
                None => None,
            };
            Ok((pending, own))
        });
        let Ok((pending, own)) = read else {
            return envelope;
        };
        if pending.is_empty() {
            return envelope;
        }
        let data_directory = self
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        if !leases_are_free(data_directory, Path::new(&root)) {
            return envelope;
        }
        // Asked only under the rule that offers another's operation.
        let resumable: Vec<String> = if inspects {
            pending
                .iter()
                .filter(|pending| envelope.operation_id.as_ref() != Some(*pending))
                .filter(|pending| {
                    OperationId::parse(pending)
                        .is_ok_and(|operation_id| resume::takes_up(self, &root, operation_id))
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        envelope.recovery = offering_abandon(
            std::mem::take(&mut envelope.recovery),
            envelope.code,
            Some(&root),
            &pending,
            &resumable,
            own.as_deref(),
        );
        envelope
    }
}

#[cfg(test)]
#[path = "outcome_tests.rs"]
mod tests;
