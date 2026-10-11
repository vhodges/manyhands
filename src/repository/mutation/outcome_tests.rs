use super::*;
use crate::{
    repository::{
        IndexPending, OperationId, PrivateKeySourceState, PublicKeyMetadataState, RemoteChange,
        RepositoryError, RepositoryOperation, RepositorySnapshot, SharedKeyId, SharedKeyOwnership,
        SharedKeyRegistration, SnapshotConfiguration, SynchronizationTarget,
        keys::{KeyMaterialAction, KeyMaterialPhase, KeyMaterialRecovery},
        transport::{HostKeyIdentity, SshDirection, SshTransportError},
    },
    results::{CheckpointEffect, DiscoveryEffect, WriteEffect},
};

/// Every repository error kind and the code a mutation reports for it.
const KINDS: [(RepositoryErrorKind, ResultCode); 36] = [
    (
        RepositoryErrorKind::InvalidSharedKeyMetadata,
        ResultCode::InvalidInput,
    ),
    (
        RepositoryErrorKind::InvalidSharedKeySourcePath,
        ResultCode::InvalidInput,
    ),
    (
        RepositoryErrorKind::SharedKeyRegistryUnavailable,
        ResultCode::IndexUnavailable,
    ),
    (
        RepositoryErrorKind::SharedKeyMaterialPending,
        ResultCode::RecoveryRequired,
    ),
    (RepositoryErrorKind::InvalidPath, ResultCode::InvalidPath),
    (
        RepositoryErrorKind::InaccessibleRepository,
        ResultCode::RepositoryInaccessible,
    ),
    (
        RepositoryErrorKind::NotRepository,
        ResultCode::NotRepository,
    ),
    (
        RepositoryErrorKind::BareRepository,
        ResultCode::BareRepository,
    ),
    (RepositoryErrorKind::DetachedHead, ResultCode::WrongBranch),
    (
        RepositoryErrorKind::WrongCheckedOutBranch,
        ResultCode::WrongBranch,
    ),
    (
        RepositoryErrorKind::DirtyWorktree,
        ResultCode::WorktreeNotClean,
    ),
    (
        RepositoryErrorKind::ConflictedWorktree,
        ResultCode::WorktreeConflicted,
    ),
    (
        RepositoryErrorKind::InvalidConfiguration,
        ResultCode::InvalidConfiguration,
    ),
    (
        RepositoryErrorKind::InvalidPublicationRemote,
        ResultCode::InvalidRemote,
    ),
    (
        RepositoryErrorKind::UnavailablePublicationRemote,
        ResultCode::InvalidRemote,
    ),
    (
        RepositoryErrorKind::SelectedRemoteRemoval,
        ResultCode::PublicationRemoteInUse,
    ),
    (
        RepositoryErrorKind::RemoteNameConflict,
        ResultCode::RemoteNameConflict,
    ),
    (
        RepositoryErrorKind::RegistryRefreshPending,
        ResultCode::RegistrationPending,
    ),
    (
        RepositoryErrorKind::RepositoryNotRegistered,
        ResultCode::RepositoryNotRegistered,
    ),
    (
        RepositoryErrorKind::IndexUnavailable,
        ResultCode::IndexUnavailable,
    ),
    (RepositoryErrorKind::RepositoryBusy, ResultCode::Busy),
    (
        RepositoryErrorKind::OperationMismatch,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::OperationAbandoned,
        ResultCode::ExternalChange,
    ),
    (
        RepositoryErrorKind::RecoveryRequired,
        ResultCode::RecoveryRequired,
    ),
    (
        RepositoryErrorKind::ExternalChange,
        ResultCode::ExternalChange,
    ),
    (
        RepositoryErrorKind::DirtyConfigurationPath,
        ResultCode::WorktreeNotClean,
    ),
    (
        RepositoryErrorKind::InvalidIdentity,
        ResultCode::InvalidInput,
    ),
    (
        RepositoryErrorKind::RepositoryNotEnabled,
        ResultCode::RepositoryNotRegistered,
    ),
    (
        RepositoryErrorKind::MissingAuthoringTarget,
        ResultCode::ItemNotFound,
    ),
    (
        RepositoryErrorKind::OccupiedItemPath,
        ResultCode::OccupiedPath,
    ),
    (
        RepositoryErrorKind::MismatchedAuthoringContext,
        ResultCode::RecoveryRequired,
    ),
    (
        RepositoryErrorKind::RollbackIncomplete,
        ResultCode::RecoveryRequired,
    ),
    (RepositoryErrorKind::Io, ResultCode::InternalError),
    (RepositoryErrorKind::Sqlite, ResultCode::InternalError),
    (RepositoryErrorKind::Git, ResultCode::InternalError),
    (
        RepositoryErrorKind::InjectedFailure,
        ResultCode::InternalError,
    ),
];

/// How many repository error kinds there are. The match has no wildcard
/// arm, so a kind added later does not compile until it is counted here,
/// and the table test then fails until it is in the table.
fn kind_count(kind: RepositoryErrorKind) -> usize {
    use RepositoryErrorKind as Kind;
    match kind {
        Kind::InvalidSharedKeyMetadata
        | Kind::InvalidSharedKeySourcePath
        | Kind::SharedKeyRegistryUnavailable
        | Kind::SharedKeyMaterialPending
        | Kind::InvalidPath
        | Kind::InaccessibleRepository
        | Kind::NotRepository
        | Kind::BareRepository
        | Kind::DetachedHead
        | Kind::WrongCheckedOutBranch
        | Kind::DirtyWorktree
        | Kind::ConflictedWorktree
        | Kind::InvalidConfiguration
        | Kind::InvalidPublicationRemote
        | Kind::UnavailablePublicationRemote
        | Kind::SelectedRemoteRemoval
        | Kind::RemoteNameConflict
        | Kind::RegistryRefreshPending
        | Kind::RepositoryNotRegistered
        | Kind::IndexUnavailable
        | Kind::RepositoryBusy
        | Kind::OperationMismatch
        | Kind::OperationAbandoned
        | Kind::RecoveryRequired
        | Kind::ExternalChange
        | Kind::DirtyConfigurationPath
        | Kind::InvalidIdentity
        | Kind::RepositoryNotEnabled
        | Kind::MissingAuthoringTarget
        | Kind::OccupiedItemPath
        | Kind::MismatchedAuthoringContext
        | Kind::RollbackIncomplete
        | Kind::Io
        | Kind::Sqlite
        | Kind::Git
        | Kind::InjectedFailure => 36,
    }
}

#[test]
fn every_repository_error_kind_has_its_code() {
    assert_complete(&KINDS, kind_count(KINDS[0].0));
    for (kind, code) in KINDS {
        assert_eq!(repository_error_code(kind), code, "{kind:?}");
        assert!(
            code.failure_class().is_some(),
            "{kind:?}: an error is never reported as a success"
        );
    }
}

fn oid(byte: u8) -> git2::Oid {
    git2::Oid::from_bytes(&[byte; 20]).unwrap()
}

#[test]
fn a_save_outcome_says_what_the_domain_claims_and_what_it_still_owes() {
    let saved = |checkpoint| SaveReport::of_checkpoint(checkpoint, true);
    assert_eq!(
        saved(LocalCheckpoint::Checkpointed { commit_oid: oid(1) }),
        SaveReport::Saved {
            claimed: Some(oid(1)),
            discovered: true
        }
    );
    assert_eq!(
        saved(LocalCheckpoint::NoChange),
        SaveReport::Saved {
            claimed: None,
            discovered: true
        }
    );
    assert_eq!(
        SaveReport::of_checkpoint(
            LocalCheckpoint::RefreshPending { commit_oid: oid(2) },
            false
        ),
        SaveReport::Saved {
            claimed: Some(oid(2)),
            discovered: false
        }
    );
}

#[test]
fn a_save_that_committed_reports_the_commit_and_one_that_did_not_is_a_no_op() {
    let report = SaveReport::Saved {
        claimed: Some(oid(1)),
        discovered: true,
    };
    let (code, effects) = report.result(Some(oid(1)));
    assert_eq!(code, ResultCode::Ok);
    assert_eq!(effects.write, WriteEffect::Written);
    assert_eq!(effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(effects.discovery, DiscoveryEffect::Current);
    assert_eq!(effects.commit_oid, Some(oid(1).to_string()));

    // The domain names a commit this request did not make.
    let (code, effects) = report.result(None);
    assert_eq!(code, ResultCode::AlreadyApplied);
    assert_eq!(effects.write, WriteEffect::Unchanged);
    assert_eq!(effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(effects.discovery, DiscoveryEffect::NotRequested);
    assert_eq!(effects.commit_oid, None);
}

#[test]
fn a_save_whose_hand_off_is_owed_is_discovery_pending() {
    let report = SaveReport::Saved {
        claimed: Some(oid(1)),
        discovered: false,
    };
    let (code, effects) = report.result(Some(oid(1)));
    assert_eq!(code, ResultCode::DiscoveryPending);
    assert_eq!(effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(effects.discovery, DiscoveryEffect::Pending);
    assert!(effects.is_durable());

    let (code, effects) = report.result(None);
    assert_eq!(code, ResultCode::DiscoveryPending);
    assert_eq!(effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(effects.discovery, DiscoveryEffect::Pending);
    assert!(!effects.is_durable());
}

#[test]
fn a_save_that_needs_an_identity_has_done_nothing() {
    let (code, effects) = SaveReport::IdentityRequired.result(None);
    assert_eq!(code, ResultCode::IdentityRequired);
    assert_eq!(effects, Effects::not_requested());
}

#[test]
fn a_stopped_save_reports_what_the_repository_shows_of_it() {
    assert_eq!(stopped_save_effects(None, false), Effects::not_requested());
    let written = stopped_save_effects(None, true);
    assert_eq!(written.write, WriteEffect::Written);
    assert_eq!(written.checkpoint, CheckpointEffect::Pending);
    assert_eq!(written.discovery, DiscoveryEffect::Pending);
    assert_eq!(written.commit_oid, None);
    let committed = stopped_save_effects(Some(oid(3)), true);
    assert_eq!(committed.write, WriteEffect::Written);
    assert_eq!(committed.checkpoint, CheckpointEffect::Committed);
    assert_eq!(committed.discovery, DiscoveryEffect::Pending);
    assert_eq!(committed.commit_oid, Some(oid(3).to_string()));
}

fn actions(code: ResultCode, retry: bool) -> Vec<&'static str> {
    let request_id = RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FX1").unwrap();
    recovery(
        code,
        Command::TicketSave,
        Some("/repository"),
        Some(request_id),
        None,
        retry,
    )
    .iter()
    .map(|action| action.action.as_str())
    .collect()
}

#[test]
fn the_same_request_is_offered_again_only_where_repeating_it_can_help() {
    // A record left accepted is finished by the same request.
    assert_eq!(actions(ResultCode::InternalError, true), ["request.retry"]);
    assert!(actions(ResultCode::InternalError, false).is_empty());
    assert_eq!(actions(ResultCode::Busy, false), ["request.retry"]);
    // Except after an external change to an item: the same request is
    // refused again for as long as the change stands. The caller reads
    // the item again and submits a new request.
    assert!(actions(ResultCode::ExternalChange, true).is_empty());
    assert!(actions(ResultCode::ExternalChange, false).is_empty());
}

fn host_key() -> HostKeyIdentity {
    HostKeyIdentity {
        algorithm: "ssh-ed25519".to_owned(),
        sha256: SENTINEL.to_owned(),
    }
}

/// Every kind of SSH transport failure and the code a mutation reports
/// for it. `code_of_every_ssh_kind` below fails to compile for a kind
/// that is not here.
fn ssh_kinds() -> Vec<(SshTransportErrorKind, ResultCode)> {
    use ResultCode as Code;
    use SshTransportErrorKind as Kind;
    vec![
        (Kind::ConfigurationInvalid, Code::InvalidRemote),
        (
            Kind::PublicationRemoteMissing,
            Code::PublicationRemoteRequired,
        ),
        (Kind::UsernameRequired, Code::InvalidRemote),
        (Kind::NoSelectedKey, Code::NoSelectedKey),
        (Kind::KeyMissing, Code::KeyUnavailable),
        (Kind::KeyUnreadable, Code::KeyUnavailable),
        (Kind::KeySourceChanged, Code::ExternalChange),
        (Kind::SelectionChanged, Code::ExternalChange),
        (Kind::EndpointChanged, Code::ExternalChange),
        (Kind::KeyInvalidOrUnsupported, Code::KeyUnavailable),
        (Kind::KeyRejected, Code::KeyRejected),
        (Kind::UnlockCancelled, Code::UnlockRequired),
        (Kind::ProviderUnavailable, Code::UnlockRequired),
        (Kind::UnlockFailed, Code::UnlockFailed),
        (
            Kind::HostApprovalRequired {
                presented: host_key(),
            },
            Code::HostApprovalRequired,
        ),
        (
            Kind::HostReplacementRequired {
                expected: host_key(),
                presented: host_key(),
            },
            Code::HostReplacementRequired,
        ),
        (Kind::HostTrustChanged, Code::ExternalChange),
        (
            Kind::HostVerificationUnavailable,
            Code::TransportUnavailable,
        ),
        (Kind::RegistryUnavailable, Code::IndexUnavailable),
        (Kind::RuntimeUninitialized, Code::InternalError),
        (Kind::TransportUnavailable, Code::TransportUnavailable),
        (Kind::RemoteUnavailable, Code::RemoteUnavailable),
        (Kind::PushRejected, Code::PushRejected),
        (Kind::ProtocolFailure, Code::InternalError),
    ]
}

/// How many kinds of SSH transport failure there are. The match has no
/// wildcard arm, so a kind added later does not compile until it is
/// counted here, and the table test then fails until it is in the table.
fn ssh_kind_count(kind: &SshTransportErrorKind) -> usize {
    use SshTransportErrorKind as Kind;
    match kind {
        Kind::ConfigurationInvalid
        | Kind::PublicationRemoteMissing
        | Kind::UsernameRequired
        | Kind::NoSelectedKey
        | Kind::KeyMissing
        | Kind::KeyUnreadable
        | Kind::KeySourceChanged
        | Kind::SelectionChanged
        | Kind::EndpointChanged
        | Kind::KeyInvalidOrUnsupported
        | Kind::KeyRejected
        | Kind::UnlockCancelled
        | Kind::ProviderUnavailable
        | Kind::UnlockFailed
        | Kind::HostApprovalRequired { .. }
        | Kind::HostReplacementRequired { .. }
        | Kind::HostTrustChanged
        | Kind::HostVerificationUnavailable
        | Kind::RegistryUnavailable
        | Kind::RuntimeUninitialized
        | Kind::TransportUnavailable
        | Kind::RemoteUnavailable
        | Kind::PushRejected
        | Kind::ProtocolFailure => 24,
    }
}

/// Asserts that a table names each variant once: `count` is how many
/// there are, and no two entries are of the same variant.
fn assert_complete<T>(table: &[(T, ResultCode)], count: usize) {
    let mut variants: Vec<_> = table
        .iter()
        .map(|(variant, _)| std::mem::discriminant(variant))
        .collect();
    let listed = variants.len();
    variants.dedup();
    assert_eq!(variants.len(), listed, "a variant is listed twice in a row");
    for (index, variant) in variants.iter().enumerate() {
        assert!(
            !variants[..index].contains(variant),
            "a variant is listed twice"
        );
    }
    assert_eq!(listed, count, "the table does not name every variant");
}

#[test]
fn every_ssh_transport_error_kind_has_its_code() {
    let table = ssh_kinds();
    assert_complete(&table, ssh_kind_count(&table[0].0));
    for (kind, code) in &table {
        assert_eq!(ssh_transport_error_code(kind), *code, "{kind:?}");
        assert!(code.failure_class().is_some(), "{kind:?}");
    }
}

/// Every kind of key-material failure and its code.
const KEY_KINDS: [(KeyMaterialErrorKind, ResultCode); 23] = [
    (KeyMaterialErrorKind::InvalidLabel, ResultCode::InvalidInput),
    (
        KeyMaterialErrorKind::InvalidPassphrase,
        ResultCode::InvalidInput,
    ),
    (
        KeyMaterialErrorKind::HomeUnavailable,
        ResultCode::InternalError,
    ),
    (
        KeyMaterialErrorKind::RegistryUnavailable,
        ResultCode::IndexUnavailable,
    ),
    (KeyMaterialErrorKind::Busy, ResultCode::Busy),
    (KeyMaterialErrorKind::NotRegistered, ResultCode::KeyNotFound),
    (
        KeyMaterialErrorKind::SelectedKeyMustBeCleared,
        ResultCode::SelectedKeyInUse,
    ),
    (
        KeyMaterialErrorKind::ImportedKey,
        ResultCode::KeyNotDeletable,
    ),
    (
        KeyMaterialErrorKind::OwnershipUnverified,
        ResultCode::KeyNotDeletable,
    ),
    (KeyMaterialErrorKind::UnsafePath, ResultCode::KeyUnavailable),
    (
        KeyMaterialErrorKind::ProtectionUnavailable,
        ResultCode::InternalError,
    ),
    (
        KeyMaterialErrorKind::SourceMissing,
        ResultCode::KeyUnavailable,
    ),
    (
        KeyMaterialErrorKind::SourceUnreadable,
        ResultCode::KeyUnavailable,
    ),
    (
        KeyMaterialErrorKind::NotRegularFile,
        ResultCode::KeyUnavailable,
    ),
    (
        KeyMaterialErrorKind::InvalidGeneratedKey,
        ResultCode::KeyUnavailable,
    ),
    (KeyMaterialErrorKind::UnlockFailed, ResultCode::UnlockFailed),
    (
        KeyMaterialErrorKind::SourceChanged,
        ResultCode::ExternalChange,
    ),
    (
        KeyMaterialErrorKind::SelectionChanged,
        ResultCode::ExternalChange,
    ),
    (
        KeyMaterialErrorKind::OperationMismatch,
        ResultCode::InternalError,
    ),
    (
        KeyMaterialErrorKind::ConfirmationRequired,
        ResultCode::ConfirmationRequired,
    ),
    (
        KeyMaterialErrorKind::RandomnessUnavailable,
        ResultCode::InternalError,
    ),
    (
        KeyMaterialErrorKind::GenerationFailed,
        ResultCode::InternalError,
    ),
    (
        KeyMaterialErrorKind::StorageUnavailable,
        ResultCode::InternalError,
    ),
];

fn key_kind_count(kind: KeyMaterialErrorKind) -> usize {
    use KeyMaterialErrorKind as Kind;
    match kind {
        Kind::InvalidLabel
        | Kind::InvalidPassphrase
        | Kind::HomeUnavailable
        | Kind::RegistryUnavailable
        | Kind::Busy
        | Kind::NotRegistered
        | Kind::SelectedKeyMustBeCleared
        | Kind::ImportedKey
        | Kind::OwnershipUnverified
        | Kind::UnsafePath
        | Kind::ProtectionUnavailable
        | Kind::SourceMissing
        | Kind::SourceUnreadable
        | Kind::NotRegularFile
        | Kind::InvalidGeneratedKey
        | Kind::UnlockFailed
        | Kind::SourceChanged
        | Kind::SelectionChanged
        | Kind::OperationMismatch
        | Kind::ConfirmationRequired
        | Kind::RandomnessUnavailable
        | Kind::GenerationFailed
        | Kind::StorageUnavailable => 23,
    }
}

#[test]
fn every_key_material_error_kind_has_its_code() {
    assert_complete(&KEY_KINDS, key_kind_count(KEY_KINDS[0].0));
    for (kind, code) in KEY_KINDS {
        assert_eq!(key_material_error_code(kind), code, "{kind:?}");
        assert!(code.failure_class().is_some(), "{kind:?}");
    }
}

/// Text a domain error holds, in its message and in its source. No
/// mapping here can pass it on: each takes the error's kind and returns a
/// `ResultCode`, which holds no text. That an envelope carries none of an
/// error's text is shown where a real error is made, by
/// `remotes::stopped_inside_git` in `tests/mutation_replay`.
const SENTINEL: &str = "SENTINEL-ERROR-5d1e";

fn repository_error(kind: RepositoryErrorKind) -> RepositoryError {
    RepositoryError::with_source(
        RepositoryOperation::AddRemote,
        Some(format!("/{SENTINEL}").into()),
        kind,
        std::io::Error::other(format!("backend said {SENTINEL}")),
    )
}

fn transport_error(kind: SshTransportErrorKind) -> SshTransportError {
    SshTransportError {
        root: format!("/{SENTINEL}").into(),
        remote_name: SENTINEL.to_owned(),
        direction: SshDirection::Push,
        selected_key_id: None,
        authority: None,
        kind,
    }
}

fn context_target() -> SynchronizationTarget {
    SynchronizationTarget::Primary
}

/// Every way a synchronization stops and its code.
fn synchronization_errors() -> Vec<(SynchronizationError, ResultCode)> {
    use ResultCode as Code;
    use SynchronizationError as Error;
    let operation_id = OperationId::new();
    vec![
        (
            Error::Repository(repository_error(RepositoryErrorKind::RemoteNameConflict)),
            Code::RemoteNameConflict,
        ),
        (
            Error::Transport(transport_error(SshTransportErrorKind::KeyRejected)),
            Code::KeyRejected,
        ),
        (Error::Busy, Code::Busy),
        (Error::PollYielding, Code::PollYielding),
        (Error::Interrupted, Code::RecoveryRequired),
        (Error::TargetNotMaterialized, Code::ItemNotFound),
        (
            Error::WorktreeNotClean {
                target: context_target(),
            },
            Code::WorktreeNotClean,
        ),
        (
            Error::WorktreeConflicted {
                target: context_target(),
            },
            Code::WorktreeConflicted,
        ),
        (Error::PrimaryMissing, Code::InvalidConfiguration),
        (Error::RemoteContextDeleted, Code::RemoteBranchDeleted),
        (Error::HistoryUnknown, Code::RecoveryRequired),
        (
            Error::ConflictPending {
                target: context_target(),
                operation_id,
                stage: crate::repository::SynchronizationStage::Primary,
            },
            Code::RecoveryRequired,
        ),
        (
            Error::IdentityRequired {
                target: context_target(),
                expected_configuration: crate::repository::ExpectedConfiguration::for_testing(),
            },
            Code::IdentityRequired,
        ),
        (
            Error::ExternalResolutionRequired {
                target: context_target(),
                operation_id,
            },
            Code::RecoveryRequired,
        ),
        (
            Error::MergeRequired {
                target: context_target(),
            },
            Code::MergeRequired,
        ),
        (Error::PushRejected, Code::PushRejected),
        (Error::ExternalChange, Code::ExternalChange),
        (Error::RecoveryRequired, Code::RecoveryRequired),
    ]
}

fn synchronization_error_count(error: &SynchronizationError) -> usize {
    use SynchronizationError as Error;
    match error {
        Error::Repository(_)
        | Error::Transport(_)
        | Error::Busy
        | Error::PollYielding
        | Error::Interrupted
        | Error::TargetNotMaterialized
        | Error::WorktreeNotClean { .. }
        | Error::WorktreeConflicted { .. }
        | Error::PrimaryMissing
        | Error::RemoteContextDeleted
        | Error::HistoryUnknown
        | Error::ConflictPending { .. }
        | Error::IdentityRequired { .. }
        | Error::ExternalResolutionRequired { .. }
        | Error::MergeRequired { .. }
        | Error::PushRejected
        | Error::ExternalChange
        | Error::RecoveryRequired => 18,
    }
}

#[test]
fn every_synchronization_error_has_its_code() {
    let table = synchronization_errors();
    assert_complete(&table, synchronization_error_count(&table[0].0));
    for (error, code) in &table {
        assert_eq!(
            synchronization_error_code(error),
            *code,
            "{}",
            std::any::type_name_of_val(error)
        );
        assert!(code.failure_class().is_some());
    }
    // A repository or transport failure inside a synchronization has the
    // code it has anywhere.
    for (kind, code) in KINDS {
        assert_eq!(
            synchronization_error_code(&SynchronizationError::Repository(repository_error(kind))),
            code
        );
    }
    for (kind, code) in ssh_kinds() {
        assert_eq!(
            synchronization_error_code(&SynchronizationError::Transport(transport_error(kind))),
            code
        );
    }
}

fn registration() -> SharedKeyRegistration {
    SharedKeyRegistration {
        id: SharedKeyId::new(),
        label: "label".to_owned(),
        ownership: SharedKeyOwnership::Generated,
        private_key_path: "/keys/private".into(),
        public_key_path: None,
        public_key_fingerprint: None,
        private_source_state: PrivateKeySourceState::Available,
        public_metadata_state: PublicKeyMetadataState::NotProvided,
        selected: false,
    }
}

fn key_recovery() -> KeyMaterialRecovery {
    KeyMaterialRecovery {
        operation_id: OperationId::new(),
        key_id: SharedKeyId::new(),
        action: KeyMaterialAction::Generate,
        phase: KeyMaterialPhase::Reserved,
        failure_code: None,
        recovery_action: crate::repository::keys::RecoveryAction::RetryGeneration,
    }
}

fn snapshot() -> RepositorySnapshot {
    RepositorySnapshot {
        root: "/repository".into(),
        configuration: SnapshotConfiguration::Missing,
        refresh_required: false,
        contexts: Vec::new(),
        items: Vec::new(),
        problems: Vec::new(),
        remote: crate::repository::RemoteSnapshot::default(),
    }
}

/// Checks one outcome enum's table: it names each variant once, `count`
/// being how many there are, and `code` gives each the code beside it.
fn assert_outcomes<T>(
    name: &str,
    table: Vec<(T, ResultCode)>,
    count: fn(&T) -> usize,
    code: impl Fn(&T) -> ResultCode,
) {
    assert_complete(&table, count(&table[0].0));
    for (index, (outcome, expected)) in table.iter().enumerate() {
        assert_eq!(code(outcome), *expected, "{name}, entry {index}");
    }
}

// How many variants each outcome enum has. Each match has no wildcard
// arm, so a variant added later does not compile until it is counted,
// and the table test then fails until the variant is in its table.

fn remote_outcomes(outcome: &RemoteOutcome) -> usize {
    match outcome {
        RemoteOutcome::Changed | RemoteOutcome::NoChange | RemoteOutcome::IndexPending { .. } => 3,
    }
}

fn refresh_outcomes(outcome: &RefreshOutcome) -> usize {
    match outcome {
        RefreshOutcome::IndexPending { .. }
        | RefreshOutcome::Refreshed { .. }
        | RefreshOutcome::RetryRequired { .. } => 3,
    }
}

fn publication_remote_outcomes(outcome: &PublicationRemoteOutcome) -> usize {
    match outcome {
        PublicationRemoteOutcome::Changed { .. }
        | PublicationRemoteOutcome::NoChange
        | PublicationRemoteOutcome::RegistrationPending { .. }
        | PublicationRemoteOutcome::IndexPending { .. } => 4,
    }
}

fn enable_outcomes(outcome: &EnableRepositoryOutcome) -> usize {
    match outcome {
        EnableRepositoryOutcome::Enabled { .. }
        | EnableRepositoryOutcome::AlreadyEnabled
        | EnableRepositoryOutcome::IdentityRequired
        | EnableRepositoryOutcome::IndexPending(_)
        | EnableRepositoryOutcome::RegistrationPending { .. } => 5,
    }
}

fn remove_registration_outcomes(outcome: &RemoveRegistrationOutcome) -> usize {
    match outcome {
        RemoveRegistrationOutcome::Removed | RemoveRegistrationOutcome::NotRegistered => 2,
    }
}

fn synchronization_outcomes(outcome: &SynchronizationOutcome) -> usize {
    match outcome {
        SynchronizationOutcome::Published { .. }
        | SynchronizationOutcome::AlreadyCurrent { .. }
        | SynchronizationOutcome::PublishPending { .. } => 3,
    }
}

fn synchronization_results(result: &SynchronizationResult) -> usize {
    match result {
        SynchronizationResult::Complete(_) | SynchronizationResult::IndexPending(_) => 2,
    }
}

fn register_key_outcomes(outcome: &RegisterSharedKeyOutcome) -> usize {
    match outcome {
        RegisterSharedKeyOutcome::Registered(_)
        | RegisterSharedKeyOutcome::SourceAlreadyRegistered { .. } => 2,
    }
}

fn key_selection_outcomes(outcome: &SharedKeySelectionOutcome) -> usize {
    match outcome {
        SharedKeySelectionOutcome::Selected(_)
        | SharedKeySelectionOutcome::Cleared
        | SharedKeySelectionOutcome::AlreadyCleared => 3,
    }
}

fn unregister_key_outcomes(outcome: &UnregisterSharedKeyOutcome) -> usize {
    match outcome {
        UnregisterSharedKeyOutcome::Unregistered
        | UnregisterSharedKeyOutcome::NotRegistered
        | UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared => 3,
    }
}

fn generate_key_outcomes(outcome: &GenerateSharedKeyOutcome) -> usize {
    match outcome {
        GenerateSharedKeyOutcome::Created(_)
        | GenerateSharedKeyOutcome::AlreadyCreated(_)
        | GenerateSharedKeyOutcome::RecoveryRequired(_) => 3,
    }
}

fn delete_key_outcomes(outcome: &DeleteGeneratedKeyOutcome) -> usize {
    match outcome {
        DeleteGeneratedKeyOutcome::Deleted
        | DeleteGeneratedKeyOutcome::AlreadyDeleted
        | DeleteGeneratedKeyOutcome::Cancelled
        | DeleteGeneratedKeyOutcome::RecoveryRequired(_) => 4,
    }
}

fn key_deletion_preflights(preflight: &GeneratedKeyDeletionPreflight) -> usize {
    match preflight {
        GeneratedKeyDeletionPreflight::ConfirmationRequired(_)
        | GeneratedKeyDeletionPreflight::NotRegistered
        | GeneratedKeyDeletionPreflight::ImportedKey
        | GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared => 4,
    }
}

fn save_outcomes(outcome: &SaveOutcome) -> usize {
    match outcome {
        SaveOutcome::IdentityRequired { .. }
        | SaveOutcome::Saved { .. }
        | SaveOutcome::IndexPending { .. } => 3,
    }
}

#[test]
fn every_domain_outcome_has_its_code() {
    use ResultCode as Code;
    use unbound::*;
    let commit_oid = oid(1);
    let root = || std::path::PathBuf::from("/repository");

    assert_outcomes(
        "RemoteOutcome",
        vec![
            (RemoteOutcome::Changed, Code::Ok),
            (RemoteOutcome::NoChange, Code::AlreadyApplied),
            (
                RemoteOutcome::IndexPending {
                    authoritative: RemoteChange::Changed,
                },
                Code::DiscoveryPending,
            ),
        ],
        remote_outcomes,
        remote_outcome_code,
    );
    assert_eq!(
        remote_outcome_code(&RemoteOutcome::IndexPending {
            authoritative: RemoteChange::NoChange
        }),
        Code::DiscoveryPending
    );

    assert_outcomes(
        "RefreshOutcome",
        vec![
            (
                RefreshOutcome::Refreshed {
                    snapshot: snapshot(),
                },
                Code::Ok,
            ),
            (RefreshOutcome::IndexPending { root: root() }, Code::Busy),
            (
                RefreshOutcome::RetryRequired {
                    root: root(),
                    context: None,
                },
                Code::Busy,
            ),
        ],
        refresh_outcomes,
        refresh_outcome_code,
    );

    // A selection is read as a save of the configuration file: its code
    // is the save's, given the commit the evidence check found. Each
    // table is the code with a commit found, then with none.
    let selections = || {
        vec![
            (
                PublicationRemoteOutcome::Changed { commit_oid },
                Code::Ok,
                // The head it names is not this request's commit.
                Code::AlreadyApplied,
            ),
            (
                PublicationRemoteOutcome::NoChange,
                Code::Ok,
                Code::AlreadyApplied,
            ),
            (
                PublicationRemoteOutcome::RegistrationPending { commit_oid },
                Code::RegistrationPending,
                Code::RegistrationPending,
            ),
            (
                PublicationRemoteOutcome::IndexPending {
                    commit_oid: Some(commit_oid),
                },
                Code::DiscoveryPending,
                Code::DiscoveryPending,
            ),
        ]
    };
    assert_outcomes(
        "PublicationRemoteOutcome, with its commit",
        selections()
            .into_iter()
            .map(|(outcome, with_commit, _)| (outcome, with_commit))
            .collect(),
        publication_remote_outcomes,
        |outcome| {
            SaveReport::of_publication_remote(clone_selection(outcome))
                .result(Some(commit_oid))
                .0
        },
    );
    assert_outcomes(
        "PublicationRemoteOutcome, with no commit",
        selections()
            .into_iter()
            .map(|(outcome, _, without)| (outcome, without))
            .collect(),
        publication_remote_outcomes,
        |outcome| {
            SaveReport::of_publication_remote(clone_selection(outcome))
                .result(None)
                .0
        },
    );

    let context = || crate::repository::ItemContext {
        root: root(),
        kind: crate::repository::AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        branch: "manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned(),
        worktree: root(),
    };
    // A report is made from a save's outcome by value.
    let saves = vec![
        (
            SaveOutcome::IdentityRequired { context: context() },
            Code::IdentityRequired,
        ),
        (
            SaveOutcome::Saved {
                context: context(),
                checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            },
            Code::Ok,
        ),
        (
            SaveOutcome::IndexPending {
                context: context(),
                checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            },
            Code::DiscoveryPending,
        ),
    ];
    assert_complete(&saves, save_outcomes(&saves[0].0));
    for (outcome, code) in saves {
        assert_eq!(SaveReport::of(outcome).result(Some(commit_oid)).0, code);
    }

    assert_outcomes(
        "EnableRepositoryOutcome",
        vec![
            (EnableRepositoryOutcome::Enabled { commit_oid }, Code::Ok),
            (
                EnableRepositoryOutcome::AlreadyEnabled,
                Code::AlreadyApplied,
            ),
            (
                EnableRepositoryOutcome::IdentityRequired,
                Code::IdentityRequired,
            ),
            (
                EnableRepositoryOutcome::IndexPending(IndexPending::new(commit_oid)),
                Code::DiscoveryPending,
            ),
            (
                EnableRepositoryOutcome::RegistrationPending { commit_oid },
                Code::RegistrationPending,
            ),
        ],
        enable_outcomes,
        enable_outcome_code,
    );

    assert_outcomes(
        "RemoveRegistrationOutcome",
        vec![
            (RemoveRegistrationOutcome::Removed, Code::Ok),
            (
                RemoveRegistrationOutcome::NotRegistered,
                Code::AlreadyApplied,
            ),
        ],
        remove_registration_outcomes,
        remove_registration_outcome_code,
    );

    let published = || SynchronizationOutcome::Published {
        target: context_target(),
        oid: commit_oid,
    };
    let synchronizations = || {
        vec![
            (published(), Code::Ok),
            (
                SynchronizationOutcome::AlreadyCurrent {
                    target: context_target(),
                    oid: commit_oid,
                },
                Code::AlreadyApplied,
            ),
            (
                SynchronizationOutcome::PublishPending {
                    target: context_target(),
                    local_oid: commit_oid,
                    reason: PublishPendingReason::NoPublicationRemote,
                },
                Code::PublicationRemoteRequired,
            ),
        ]
    };
    assert_outcomes(
        "SynchronizationOutcome",
        synchronizations(),
        synchronization_outcomes,
        synchronization_outcome_code,
    );
    for (outcome, code) in synchronizations() {
        assert_eq!(
            synchronization_result_code(&SynchronizationResult::Complete(outcome)),
            code
        );
    }
    assert_outcomes(
        "SynchronizationResult",
        vec![
            (SynchronizationResult::Complete(published()), Code::Ok),
            (
                SynchronizationResult::IndexPending(IndexPending::new(published())),
                Code::DiscoveryPending,
            ),
        ],
        synchronization_results,
        synchronization_result_code,
    );

    assert_outcomes(
        "RegisterSharedKeyOutcome",
        vec![
            (
                RegisterSharedKeyOutcome::Registered(registration()),
                Code::Ok,
            ),
            (
                RegisterSharedKeyOutcome::SourceAlreadyRegistered {
                    existing: SharedKeyId::new(),
                },
                Code::AlreadyApplied,
            ),
        ],
        register_key_outcomes,
        register_key_outcome_code,
    );
    assert_outcomes(
        "SharedKeySelectionOutcome",
        vec![
            (
                SharedKeySelectionOutcome::Selected(registration()),
                Code::Ok,
            ),
            (SharedKeySelectionOutcome::Cleared, Code::Ok),
            (
                SharedKeySelectionOutcome::AlreadyCleared,
                Code::AlreadyApplied,
            ),
        ],
        key_selection_outcomes,
        key_selection_outcome_code,
    );
    assert_outcomes(
        "UnregisterSharedKeyOutcome",
        vec![
            (UnregisterSharedKeyOutcome::Unregistered, Code::Ok),
            (
                UnregisterSharedKeyOutcome::NotRegistered,
                Code::AlreadyApplied,
            ),
            (
                UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared,
                Code::SelectedKeyInUse,
            ),
        ],
        unregister_key_outcomes,
        |outcome| unregister_key_outcome_code(*outcome),
    );
    assert_outcomes(
        "GenerateSharedKeyOutcome",
        vec![
            (GenerateSharedKeyOutcome::Created(registration()), Code::Ok),
            (
                GenerateSharedKeyOutcome::AlreadyCreated(registration()),
                Code::AlreadyApplied,
            ),
            (
                GenerateSharedKeyOutcome::RecoveryRequired(key_recovery()),
                Code::RecoveryRequired,
            ),
        ],
        generate_key_outcomes,
        generate_key_outcome_code,
    );
    assert_outcomes(
        "DeleteGeneratedKeyOutcome",
        vec![
            (DeleteGeneratedKeyOutcome::Deleted, Code::Ok),
            (
                DeleteGeneratedKeyOutcome::AlreadyDeleted,
                Code::AlreadyApplied,
            ),
            (DeleteGeneratedKeyOutcome::Cancelled, Code::Cancelled),
            (
                DeleteGeneratedKeyOutcome::RecoveryRequired(key_recovery()),
                Code::RecoveryRequired,
            ),
        ],
        delete_key_outcomes,
        delete_key_outcome_code,
    );
    assert_outcomes(
        "GeneratedKeyDeletionPreflight",
        vec![
            (
                GeneratedKeyDeletionPreflight::ConfirmationRequired(registration()),
                Code::ConfirmationRequired,
            ),
            (
                GeneratedKeyDeletionPreflight::NotRegistered,
                Code::KeyNotFound,
            ),
            (
                GeneratedKeyDeletionPreflight::ImportedKey,
                Code::KeyNotDeletable,
            ),
            (
                GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared,
                Code::SelectedKeyInUse,
            ),
        ],
        key_deletion_preflights,
        key_deletion_preflight_code,
    );
}

/// `PublicationRemoteOutcome` is not `Clone`, and a report is made from
/// one by value.
fn clone_selection(outcome: &PublicationRemoteOutcome) -> PublicationRemoteOutcome {
    match outcome {
        PublicationRemoteOutcome::Changed { commit_oid } => PublicationRemoteOutcome::Changed {
            commit_oid: *commit_oid,
        },
        PublicationRemoteOutcome::NoChange => PublicationRemoteOutcome::NoChange,
        PublicationRemoteOutcome::RegistrationPending { commit_oid } => {
            PublicationRemoteOutcome::RegistrationPending {
                commit_oid: *commit_oid,
            }
        }
        PublicationRemoteOutcome::IndexPending { commit_oid } => {
            PublicationRemoteOutcome::IndexPending {
                commit_oid: *commit_oid,
            }
        }
    }
}

/// A request of the application names no repository, so there is none
/// to inspect: an interrupted key generation is continued by the same
/// request, and one that is final offers nothing.
#[test]
fn recovery_required_with_no_repository_offers_the_request_or_nothing() {
    let request_id = RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FX1").unwrap();
    let offered = |retry| {
        recovery(
            ResultCode::RecoveryRequired,
            Command::KeyGenerate,
            None,
            Some(request_id),
            None,
            retry,
        )
    };
    let accepted = offered(true);
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].action.as_str(), "request.retry");
    assert_eq!(accepted[0].arguments["request_id"], request_id.to_string());
    assert!(offered(false).is_empty());
    // With a repository it is that repository that is inspected, as before.
    assert_eq!(
        actions(ResultCode::RecoveryRequired, true),
        ["repo.inspect"]
    );
    assert_eq!(
        actions(ResultCode::RecoveryRequired, false),
        ["repo.inspect"]
    );
}

/// An external change under a confirmed request whose record stays
/// accepted is looked at in the repository. A request of the
/// application has none: nothing is offered.
#[test]
fn an_external_change_under_an_accepted_key_deletion_offers_nothing() {
    let request_id = RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FX1").unwrap();
    let offered = |command, root, retry| {
        recovery(
            ResultCode::ExternalChange,
            command,
            root,
            Some(request_id),
            None,
            retry,
        )
        .into_iter()
        .map(|action| action.action.as_str())
        .collect::<Vec<_>>()
    };
    assert!(offered(Command::KeyDelete, None, true).is_empty());
    assert_eq!(
        offered(Command::KeyDelete, None, false),
        ["request.prepare"]
    );
    assert_eq!(
        offered(Command::RemoteRemove, Some("/repository"), true),
        ["repo.inspect"]
    );
    assert_eq!(
        offered(Command::RemoteRemove, Some("/repository"), false),
        ["request.prepare"]
    );
}

#[test]
fn a_pending_registration_is_finished_by_the_same_request() {
    assert_eq!(
        actions(ResultCode::RegistrationPending, false),
        ["request.retry"]
    );
}

#[test]
fn a_selection_whose_registration_is_pending_reports_its_commit_and_is_owed() {
    let report = SaveReport::of_publication_remote(PublicationRemoteOutcome::RegistrationPending {
        commit_oid: oid(4),
    });
    assert_eq!(report.claimed(), Some(oid(4)));
    let (code, effects) = report.result(Some(oid(4)));
    assert_eq!(code, ResultCode::RegistrationPending);
    assert_eq!(effects.write, WriteEffect::Written);
    assert_eq!(effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(effects.commit_oid, Some(oid(4).to_string()));
    assert!(effects.is_durable());
}

/// `index.rebuild` takes the root of the repository to rebuild. A result
/// that names no repository, as one of a key command is, has none to
/// give, so it offers no such action.
#[test]
fn a_result_that_names_no_repository_offers_no_rebuild() {
    let request_id = RequestId::new();
    for code in [
        ResultCode::IndexUnavailable,
        ResultCode::RepositoryNotRegistered,
    ] {
        for retry in [false, true] {
            assert!(
                recovery(
                    code,
                    Command::KeySelect,
                    None,
                    Some(request_id),
                    None,
                    retry
                )
                .is_empty(),
                "{code:?}, retry {retry}"
            );
        }
        let offered = recovery(
            code,
            Command::KeySelect,
            Some("/repository"),
            Some(request_id),
            None,
            false,
        );
        assert_eq!(offered.len(), 1, "{code:?}");
        assert_eq!(offered[0].action, RecoveryActionKind::IndexRebuild);
        assert_eq!(
            offered[0]
                .arguments
                .get("root")
                .and_then(|root| root.as_str()),
            Some("/repository")
        );
    }
}

/// The actions of `code` for a request of `command` on a repository.
fn actions_of(command: Command, code: ResultCode, retry: bool) -> Vec<&'static str> {
    recovery(
        code,
        command,
        Some("/repository"),
        Some(RequestId::new()),
        None,
        retry,
    )
    .into_iter()
    .map(|action| action.action.as_str())
    .collect()
}

#[test]
fn a_request_with_no_usable_consent_is_offered_a_new_preview() {
    for code in [
        ResultCode::ConfirmationRequired,
        ResultCode::ConfirmationExpired,
        ResultCode::ConfirmationNotFound,
    ] {
        for command in [Command::RepoCreate, Command::RepoEnable] {
            assert_eq!(
                actions_of(command, code, false),
                ["request.prepare"],
                "{code:?}"
            );
        }
    }
    // A caller error: no action corrects it.
    for code in [
        ResultCode::ConfirmationMismatch,
        ResultCode::ConfirmationUsed,
        ResultCode::NotConfirmable,
    ] {
        assert!(
            actions_of(Command::RepoEnable, code, false).is_empty(),
            "{code:?}"
        );
    }
}

/// What was previewed is no longer there. A confirmed command whose
/// request left nothing is offered a new preview. One whose record was
/// left accepted has its own operation pending, which refuses every
/// other operation on the repository: a new preview could not be used,
/// so the repository is what is looked at. An item write is offered
/// nothing, as before.
#[test]
fn an_external_change_to_a_confirmed_command_is_offered_a_new_preview_or_an_inspection() {
    for command in Command::ALL {
        for retry in [false, true] {
            let expected: &[&str] = match (command.needs_confirmation(), retry) {
                (true, false) => &["request.prepare"],
                (true, true) => &["repo.inspect"],
                (false, _) => &[],
            };
            assert_eq!(
                actions_of(command, ResultCode::ExternalChange, retry),
                expected,
                "{command:?}, retry {retry}"
            );
        }
    }
}

/// `repo create` has no repository yet to set an identity on.
#[test]
fn an_identity_is_set_on_a_repository_that_exists() {
    for command in Command::ALL {
        let expected: &[&str] = if command == Command::RepoCreate {
            &[]
        } else {
            &["repo.identity_set"]
        };
        assert_eq!(
            actions_of(command, ResultCode::IdentityRequired, false),
            expected,
            "{command:?}"
        );
    }
}

/// A result of `prepare` is of no request: nothing is offered to be
/// sent again.
#[test]
fn a_result_of_no_request_is_never_offered_a_retry() {
    for code in ResultCode::ALL {
        for retry in [false, true] {
            let offered = recovery(
                code,
                Command::RepoEnable,
                Some("/repository"),
                None,
                None,
                retry,
            );
            assert!(
                offered
                    .iter()
                    .all(|action| action.action != RecoveryActionKind::RequestRetry),
                "{code:?}"
            );
        }
    }
}
