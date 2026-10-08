use super::*;
use crate::repository::keys::KeyMaterialErrorKind;

#[test]
fn a_local_state_is_passed_on_only_when_it_has_the_form_of_a_name() {
    for name in [
        "created",
        "indexing",
        "authoring_checkpoint_observed",
        "step_2",
        &"a".repeat(LONGEST_STORED_NAME),
    ] {
        assert_eq!(stored_name(name).unwrap(), name);
    }
    for text in [
        "",
        "Indexing",
        "retry-required",
        "two words",
        "_observed",
        "2_step",
        "naïve",
        "failed: No such file or directory (os error 2)",
        &"a".repeat(LONGEST_STORED_NAME + 1),
    ] {
        let error = stored_name(text).unwrap_err();
        assert_eq!(error.code(), ResultCode::InternalError, "{text:?}");
    }
}

#[test]
fn a_local_action_is_one_the_recovery_store_writes() {
    let local = &OperationAction::ALL[..12];
    for action in local {
        assert_eq!(local_action(action.as_str()), Some(*action));
    }
    for action in &OperationAction::ALL[12..] {
        assert_eq!(local_action(action.as_str()), None, "{action:?}");
    }
    // What the recovery store writes for an operation it has no name for.
    for stored in ["other", "", "Refresh"] {
        assert_eq!(local_action(stored), None, "{stored:?}");
    }
}

#[test]
fn a_key_material_phase_belongs_to_its_action() {
    let generate = OperationAction::GenerateKey;
    let delete = OperationAction::DeleteKey;
    let cases = [
        (generate, "reserved", Some("reserved")),
        (generate, "private-written", Some("private_written")),
        (generate, "pair-written", Some("pair_written")),
        (generate, "completed", Some("completed")),
        (
            generate,
            "retained-for-inspection",
            Some("retained_for_inspection"),
        ),
        (delete, "prepared", Some("prepared")),
        (delete, "private-removed", Some("private_removed")),
        (delete, "files-removed", Some("files_removed")),
        (delete, "completed", Some("completed")),
        (
            delete,
            "retained-for-inspection",
            Some("retained_for_inspection"),
        ),
        (generate, "prepared", None),
        (generate, "files-removed", None),
        (delete, "reserved", None),
        (delete, "pair-written", None),
        (generate, "pair_written", None),
        (generate, "", None),
    ];
    for (action, phase, expected) in cases {
        assert_eq!(
            key_material_state(action, phase),
            expected,
            "{action:?} {phase}"
        );
    }
}

#[test]
fn key_material_recovery_offers_what_its_own_list_offers() {
    let generate = OperationAction::GenerateKey;
    let delete = OperationAction::DeleteKey;
    let retry = Some(OperationNextAction::RetryGeneration);
    let review = Some(OperationNextAction::ReviewDeletionAgain);
    let inspect = Some(OperationNextAction::InspectRetainedFiles);
    let cases = [
        (generate, "reserved", false, retry),
        (generate, "private_written", true, retry),
        (generate, "pair_written", false, retry),
        (generate, "retained_for_inspection", true, inspect),
        (generate, "completed", true, inspect),
        (generate, "completed", false, None),
        (delete, "prepared", false, review),
        (delete, "private_removed", true, review),
        (delete, "files_removed", false, review),
        (delete, "retained_for_inspection", false, inspect),
        (delete, "completed", true, inspect),
        (delete, "completed", false, None),
    ];
    for (action, state, failed, expected) in cases {
        assert_eq!(
            key_material_next_action(action, state, failed),
            expected,
            "{action:?} {state} {failed}"
        );
    }
}

#[test]
fn only_a_remote_operation_that_ended_gives_up_the_reservation() {
    let ended = [
        (RemoteOperationPhase::Completed, "completed"),
        (RemoteOperationPhase::Interrupted, "interrupted"),
        (RemoteOperationPhase::Cancelled, "cancelled"),
        (RemoteOperationPhase::Failed, "failed"),
    ];
    for (phase, name) in ended {
        assert_eq!(remote_phase(phase), (name, false));
    }
    for phase in [
        RemoteOperationPhase::Reserved,
        RemoteOperationPhase::Advertising,
        RemoteOperationPhase::Persisting,
        RemoteOperationPhase::FetchPrepared,
        RemoteOperationPhase::FetchObserved,
        RemoteOperationPhase::LocalPrepared,
        RemoteOperationPhase::LocalFastForwarded,
        RemoteOperationPhase::PushPrepared,
        RemoteOperationPhase::PushReturned,
        RemoteOperationPhase::PushVerified,
        RemoteOperationPhase::Reconciling,
    ] {
        assert!(remote_phase(phase).1, "{phase:?}");
    }
}

#[test]
fn a_remote_outcome_other_than_completion_is_a_failure_code() {
    assert_eq!(remote_failure(RemoteOutcomeCategory::Completed), None);
    let cases = [
        (
            RemoteOutcomeCategory::ConfigurationRequired,
            "configuration_required",
        ),
        (
            RemoteOutcomeCategory::SelectedKeyUnavailable,
            "selected_key_unavailable",
        ),
        (RemoteOutcomeCategory::UnlockRequired, "unlock_required"),
        (
            RemoteOutcomeCategory::HostApprovalRequired,
            "host_approval_required",
        ),
        (
            RemoteOutcomeCategory::TransportUnavailable,
            "transport_unavailable",
        ),
        (RemoteOutcomeCategory::ProtocolRejected, "protocol_rejected"),
        (RemoteOutcomeCategory::Cancelled, "cancelled"),
        (
            RemoteOutcomeCategory::RepositoryUnavailable,
            "repository_unavailable",
        ),
    ];
    for (outcome, code) in cases {
        assert_eq!(remote_failure(outcome).unwrap().as_str(), code);
        assert_eq!(polling_outcome(outcome).as_str(), code);
    }
}

/// Every repository operation. The `match` has no wildcard arm, so a
/// variant added later does not compile until it is listed here.
fn repository_operations() -> Vec<RepositoryOperation> {
    use RepositoryOperation::*;
    let all = vec![
        OpenRegistry,
        RegisterSharedKey,
        SelectSharedKey,
        ClearSharedKeySelection,
        UnregisterSharedKey,
        ListSharedKeys,
        PreflightGeneratedKeyDeletion,
        Inspect,
        CreateAndEnable,
        Enable,
        RemoveRegistration,
        ListRemotes,
        AddRemote,
        RemoveRemote,
        SetPublicationRemote,
        PrepareContext,
        SaveDocument,
        SaveTicket,
        SubmitComment,
        RefreshRepository,
        RebuildRepository,
        RepositorySnapshot,
        Read,
    ];
    for operation in &all {
        match operation {
            OpenRegistry
            | RegisterSharedKey
            | SelectSharedKey
            | ClearSharedKeySelection
            | UnregisterSharedKey
            | ListSharedKeys
            | PreflightGeneratedKeyDeletion
            | Inspect
            | CreateAndEnable
            | Enable
            | RemoveRegistration
            | ListRemotes
            | AddRemote
            | RemoveRemote
            | SetPublicationRemote
            | PrepareContext
            | SaveDocument
            | SaveTicket
            | SubmitComment
            | RefreshRepository
            | RebuildRepository
            | RepositorySnapshot
            | Read => {}
        }
    }
    all
}

#[test]
fn every_action_the_recovery_store_writes_is_one_the_read_knows() {
    let mut written = std::collections::BTreeSet::new();
    for operation in repository_operations() {
        let stored = crate::repository::recovery::action_name(operation);
        // What the store writes for an operation that is not recoverable;
        // no such operation begins a record.
        if stored == "other" {
            continue;
        }
        let action = local_action(stored);
        assert!(action.is_some(), "{operation:?} is stored as {stored:?}");
        written.insert(action.unwrap().as_str());
    }
    let known: std::collections::BTreeSet<_> = OperationAction::ALL[..12]
        .iter()
        .map(|action| action.as_str())
        .collect();
    assert_eq!(written, known);
}

/// Every key-material error kind, listed the same way.
fn key_material_error_kinds() -> Vec<KeyMaterialErrorKind> {
    use KeyMaterialErrorKind::*;
    let all = vec![
        InvalidLabel,
        InvalidPassphrase,
        HomeUnavailable,
        RegistryUnavailable,
        Busy,
        NotRegistered,
        SelectedKeyMustBeCleared,
        ImportedKey,
        OwnershipUnverified,
        UnsafePath,
        ProtectionUnavailable,
        SourceMissing,
        SourceUnreadable,
        NotRegularFile,
        InvalidGeneratedKey,
        UnlockFailed,
        SourceChanged,
        SelectionChanged,
        OperationMismatch,
        ConfirmationRequired,
        RandomnessUnavailable,
        GenerationFailed,
        StorageUnavailable,
    ];
    for kind in &all {
        match kind {
            InvalidLabel
            | InvalidPassphrase
            | HomeUnavailable
            | RegistryUnavailable
            | Busy
            | NotRegistered
            | SelectedKeyMustBeCleared
            | ImportedKey
            | OwnershipUnverified
            | UnsafePath
            | ProtectionUnavailable
            | SourceMissing
            | SourceUnreadable
            | NotRegularFile
            | InvalidGeneratedKey
            | UnlockFailed
            | SourceChanged
            | SelectionChanged
            | OperationMismatch
            | ConfirmationRequired
            | RandomnessUnavailable
            | GenerationFailed
            | StorageUnavailable => {}
        }
    }
    all
}

#[test]
fn every_failure_a_key_material_operation_stores_is_in_the_registry() {
    let mut stored = std::collections::BTreeSet::new();
    for kind in key_material_error_kinds() {
        let code = crate::repository::keys::stored_failure_code(kind);
        assert_ne!(
            OperationFailureCode::from_stored_key_material(code),
            OperationFailureCode::UnknownFailure,
            "{kind:?} is stored as {code:?}"
        );
        stored.insert(code);
    }
    let registered: std::collections::BTreeSet<_> = OperationFailureCode::ALL
        .into_iter()
        .filter_map(OperationFailureCode::stored_key_material)
        .collect();
    assert_eq!(stored, registered);
}

#[test]
fn a_local_synchronization_target_names_what_was_synchronized() {
    const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
    let oid = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(
        local_synchronization(&format!("primary/{oid}")).unwrap(),
        (OperationAction::SynchronizePrimary, None)
    );
    for kind in ["document", "ticket"] {
        assert_eq!(
            local_synchronization(&format!("{kind}/{ITEM}/{oid}")).unwrap(),
            (OperationAction::SynchronizeContext, Some(ITEM.to_owned())),
        );
    }
    for rest in [
        String::new(),
        "primary".to_owned(),
        "primary/not-an-oid".to_owned(),
        format!("primary/{}", oid.to_uppercase()),
        format!("primary/{oid}/extra"),
        format!("primary/{ITEM}/{oid}"),
        format!("comment/{ITEM}/{oid}"),
        format!("ticket/{oid}"),
        format!("ticket/not-an-item/{oid}"),
        format!("ticket/{ITEM}/{}", &oid[..39]),
        format!("ticket/{ITEM}/{oid}/extra"),
    ] {
        let error = local_synchronization(&rest).unwrap_err();
        assert_eq!(error.code(), ResultCode::InternalError, "{rest:?}");
    }
}
