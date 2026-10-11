use super::*;
use crate::{
    repository::{
        IndexPending, OperationId, PrivateKeySourceState, PublicKeyMetadataState, RemoteChange,
        RepositoryError, RepositoryOperation, RepositorySnapshot, SharedKeyId, SharedKeyOwnership,
        SharedKeyRegistration, SnapshotConfiguration, SynchronizationTarget,
        keys::{KeyMaterialAction, KeyMaterialPhase, KeyMaterialRecovery},
        transport::{HostKeyIdentity, SshDirection, SshTransportError},
    },
    results::{CheckpointEffect, DiscoveryEffect, Envelope, Scope, WriteEffect},
};

/// Every repository error kind and the code a mutation reports for it.
const KINDS: [(RepositoryErrorKind, ResultCode); 35] = [
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

#[test]
fn every_repository_error_kind_has_its_code() {
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
    recovery(code, Some("/repository"), request_id, None, retry)
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

/// What a domain error holds that no result may: here, in its message and
/// in its source.
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

/// The envelope a binding answers a stopped request with, as text.
fn stopped_envelope(code: ResultCode) -> String {
    let envelope: Envelope<()> = Envelope::classified_mutation(
        "remote add",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        Effects::not_requested(),
        false,
    )
    .with_recovery(recovery(
        code,
        None,
        RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FX1").unwrap(),
        None,
        true,
    ));
    serde_json::to_string(&envelope).unwrap()
}

#[test]
fn an_error_s_message_and_source_do_not_reach_its_envelope() {
    // The errors hold the sentinel where a formatter would find it.
    let error = repository_error(RepositoryErrorKind::Git);
    assert!(format!("{error:?}").contains(SENTINEL));
    assert!(error.to_string().contains(SENTINEL));
    assert!(
        format!(
            "{:?}",
            transport_error(SshTransportErrorKind::ProtocolFailure)
        )
        .contains(SENTINEL)
    );

    for (kind, _) in KINDS {
        let envelope = stopped_envelope(repository_error_code(repository_error(kind).kind));
        assert!(!envelope.contains(SENTINEL), "{kind:?}: {envelope}");
    }
    for (kind, _) in ssh_kinds() {
        let error = transport_error(kind);
        let envelope = stopped_envelope(ssh_transport_error_code(&error.kind));
        assert!(!envelope.contains(SENTINEL), "{envelope}");
    }
    for (error, _) in synchronization_errors() {
        let envelope = stopped_envelope(synchronization_error_code(&error));
        assert!(!envelope.contains(SENTINEL), "{envelope}");
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

#[test]
fn every_domain_outcome_has_its_code() {
    use ResultCode as Code;
    use unbound::*;

    assert_eq!(remote_outcome_code(&RemoteOutcome::Changed), Code::Ok);
    assert_eq!(
        remote_outcome_code(&RemoteOutcome::NoChange),
        Code::AlreadyApplied
    );
    for authoritative in [RemoteChange::Changed, RemoteChange::NoChange] {
        assert_eq!(
            remote_outcome_code(&RemoteOutcome::IndexPending { authoritative }),
            Code::DiscoveryPending
        );
    }

    assert_eq!(
        refresh_outcome_code(&RefreshOutcome::Refreshed {
            snapshot: snapshot()
        }),
        Code::Ok
    );
    assert_eq!(
        refresh_outcome_code(&RefreshOutcome::IndexPending {
            root: "/repository".into()
        }),
        Code::Busy
    );
    assert_eq!(
        refresh_outcome_code(&RefreshOutcome::RetryRequired {
            root: "/repository".into(),
            context: None
        }),
        Code::Busy
    );

    // A selection is read as a save of the configuration file: its code
    // is the save's, given the commit the evidence check found.
    let commit_oid = oid(1);
    for (outcome, with_commit, without) in [
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
        (
            PublicationRemoteOutcome::IndexPending { commit_oid: None },
            Code::DiscoveryPending,
            Code::DiscoveryPending,
        ),
    ] {
        let report = SaveReport::of_publication_remote(outcome);
        assert_eq!(report.result(Some(commit_oid)).0, with_commit);
        assert_eq!(report.result(None).0, without);
    }

    for (outcome, code) in [
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
    ] {
        assert_eq!(enable_outcome_code(&outcome), code);
    }

    assert_eq!(
        remove_registration_outcome_code(&RemoveRegistrationOutcome::Removed),
        Code::Ok
    );
    assert_eq!(
        remove_registration_outcome_code(&RemoveRegistrationOutcome::NotRegistered),
        Code::AlreadyApplied
    );

    let target = context_target;
    let published = || SynchronizationOutcome::Published {
        target: target(),
        oid: commit_oid,
    };
    for (outcome, code) in [
        (published(), Code::Ok),
        (
            SynchronizationOutcome::AlreadyCurrent {
                target: target(),
                oid: commit_oid,
            },
            Code::AlreadyApplied,
        ),
        (
            SynchronizationOutcome::PublishPending {
                target: target(),
                local_oid: commit_oid,
                reason: PublishPendingReason::NoPublicationRemote,
            },
            Code::PublicationRemoteRequired,
        ),
    ] {
        assert_eq!(synchronization_outcome_code(&outcome), code);
        assert_eq!(
            synchronization_result_code(&SynchronizationResult::Complete(outcome)),
            code
        );
    }
    assert_eq!(
        synchronization_result_code(&SynchronizationResult::IndexPending(IndexPending::new(
            published()
        ))),
        Code::DiscoveryPending
    );

    assert_eq!(
        register_key_outcome_code(&RegisterSharedKeyOutcome::Registered(registration())),
        Code::Ok
    );
    assert_eq!(
        register_key_outcome_code(&RegisterSharedKeyOutcome::SourceAlreadyRegistered {
            existing: SharedKeyId::new()
        }),
        Code::AlreadyApplied
    );
    for (outcome, code) in [
        (
            SharedKeySelectionOutcome::Selected(registration()),
            Code::Ok,
        ),
        (SharedKeySelectionOutcome::Cleared, Code::Ok),
        (
            SharedKeySelectionOutcome::AlreadyCleared,
            Code::AlreadyApplied,
        ),
    ] {
        assert_eq!(key_selection_outcome_code(&outcome), code);
    }
    for (outcome, code) in [
        (UnregisterSharedKeyOutcome::Unregistered, Code::Ok),
        (
            UnregisterSharedKeyOutcome::NotRegistered,
            Code::AlreadyApplied,
        ),
        (
            UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared,
            Code::SelectedKeyInUse,
        ),
    ] {
        assert_eq!(unregister_key_outcome_code(outcome), code);
    }
    for (outcome, code) in [
        (GenerateSharedKeyOutcome::Created(registration()), Code::Ok),
        (
            GenerateSharedKeyOutcome::AlreadyCreated(registration()),
            Code::AlreadyApplied,
        ),
        (
            GenerateSharedKeyOutcome::RecoveryRequired(key_recovery()),
            Code::RecoveryRequired,
        ),
    ] {
        assert_eq!(generate_key_outcome_code(&outcome), code);
    }
    for (outcome, code) in [
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
    ] {
        assert_eq!(delete_key_outcome_code(&outcome), code);
    }
    for (preflight, code) in [
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
    ] {
        assert_eq!(key_deletion_preflight_code(&preflight), code);
    }
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
