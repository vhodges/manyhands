use super::*;
use crate::repository::keys::KeyMaterialErrorKind;

#[test]
fn polling_status_reads_stored_next_eligibility_as_utc_or_empty() {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    git2::Repository::init(&root).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(crate::repository::REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)", [root.to_str().unwrap()]).unwrap();
    // Supply the future column on the old production tree so the red assertion
    // proves the read omits a stored deadline, rather than a missing SQL column.
    let exists = connection
        .prepare("PRAGMA table_info(remote_polling_state)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .any(|name| name.unwrap() == "next_eligible_at");
    if !exists {
        connection
            .execute_batch("ALTER TABLE remote_polling_state ADD COLUMN next_eligible_at INTEGER")
            .unwrap();
    }
    let repo = service.resolve_repository(&root).unwrap();
    assert_eq!(
        service.polling_status(&repo).unwrap().next_eligible_at,
        None
    );
    connection
        .execute_batch("UPDATE remote_polling_state SET next_eligible_at=1767225600")
        .unwrap();
    assert_eq!(
        service
            .polling_status(&repo)
            .unwrap()
            .next_eligible_at
            .as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
    connection
        .execute_batch("UPDATE remote_polling_state SET next_eligible_at=NULL")
        .unwrap();
    assert_eq!(
        service.polling_status(&repo).unwrap().next_eligible_at,
        None
    );
}

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
    let local = &OperationAction::ALL[..16];
    for action in local {
        assert_eq!(local_action(action.as_str()), Some(*action));
    }
    for action in &OperationAction::ALL[16..] {
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
        ReleaseContext,
        RetireContext,
        SaveDocument,
        SaveTicket,
        SubmitComment,
        RefreshRepository,
        RebuildRepository,
        RepositorySnapshot,
        Read,
        AbandonOperation,
        SetLocalIdentity,
        CreateFolder,
        RepairItem,
        ContextSyncLocal,
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
            | ReleaseContext
            | RetireContext
            | SaveDocument
            | SaveTicket
            | SubmitComment
            | RefreshRepository
            | RebuildRepository
            | RepositorySnapshot
            | Read
            | AbandonOperation
            | SetLocalIdentity
            | CreateFolder
            | RepairItem
            | ContextSyncLocal => {}
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
    let known: std::collections::BTreeSet<_> = OperationAction::ALL[..16]
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

/// A process table that holds exactly these processes.
struct Table(Vec<u32>);

impl crate::repository::ProcessProbe for Table {
    fn exists(&self, pid: u32) -> bool {
        self.0.contains(&pid)
    }
}

/// Processes of this host other than this one: two that exist and one
/// that is gone.
const LIVE_A: u32 = 4_000_001;
const LIVE_B: u32 = 4_000_002;
const GONE: u32 = 4_000_009;

/// Who a writer refused for a lease or the index's lock is told is at
/// work. A row a call works under by its own evidence, a fresh index
/// heartbeat, comes first; then, of the rows a call may be at work under
/// only because the repository's lease is held, the newest, in a state a
/// call holds the lease in. A row whose call has ended, `failed` or a
/// stale `indexing`, is never named for the lease, so a lease whose holder
/// has no row of its own is answered with nothing at work.
#[test]
fn a_writer_held_off_by_a_lease_is_told_of_the_row_a_call_holding_it_works_under() {
    const A: &str = "01ARZ3NDEKTSV4RRFFQ69G5F0A";
    const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5F0B";
    let now = OffsetDateTime::now_utc().unix_timestamp();
    // (case, rows: (operation, state, pid, touched at), whether the
    // repository's lease is held, the operation named)
    type Row<'a> = (
        &'a str,
        &'a [(&'a str, &'a str, u32, i64)],
        bool,
        Option<&'a str>,
    );
    let rows: [Row<'_>; 8] = [
        (
            "a lease whose holder has no row, beside an older failed row",
            &[(A, "failed", LIVE_A, 1)],
            true,
            None,
        ),
        (
            "a failed row, then a row a call works under the lease in",
            &[(A, "failed", LIVE_A, 1), (B, "created", LIVE_B, 1)],
            true,
            Some(B),
        ),
        (
            "two rows a call may work under the lease in: the newest",
            &[(A, "created", LIVE_A, 1), (B, "created", LIVE_B, 1)],
            true,
            Some(B),
        ),
        (
            "a fresh heartbeat before a newer row of the lease",
            &[(A, "indexing", LIVE_A, now), (B, "created", LIVE_B, 1)],
            true,
            Some(A),
        ),
        (
            "a fresh heartbeat after an older row of the lease",
            &[(A, "created", LIVE_A, 1), (B, "indexing", LIVE_B, now)],
            true,
            Some(B),
        ),
        (
            "a stale heartbeat, the lease held",
            &[(A, "indexing", LIVE_A, 1)],
            true,
            None,
        ),
        (
            "a row of a process that is gone",
            &[(A, "created", GONE, 1)],
            true,
            None,
        ),
        (
            "the lease free: the index's lock was in the way",
            &[(A, "created", LIVE_A, 1)],
            false,
            None,
        ),
    ];
    for (case, stored, held, named) in rows {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        let git = git2::Repository::init(&root_path).unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path())
            .unwrap()
            .with_process_probe_for_testing(std::sync::Arc::new(Table(vec![LIVE_A, LIVE_B])));
        let db = Connection::open(data.path().join(crate::repository::REGISTRY_FILE)).unwrap();
        db.execute(
            "INSERT INTO repositories(root_path, enabled_at, accessibility, refresh_required)
             VALUES(?1, 7, 'accessible', 0)",
            [root_path.to_str().unwrap()],
        )
        .unwrap();
        for (operation, state, pid, touched) in stored {
            db.execute(
                "INSERT INTO operation_records
                    (repository_id, root_path, operation_ulid, action, target, state,
                     observed_at, host, pid)
                 VALUES (1, ?1, ?2, 'save_ticket', '', ?3, ?4, ?5, ?6)",
                params![
                    root_path.to_str().unwrap(),
                    operation,
                    state,
                    touched,
                    crate::repository::liveness::this_host(),
                    i64::from(*pid)
                ],
            )
            .unwrap();
        }
        let lease = held.then(|| {
            crate::repository::repository_lease(&git, &root_path, RepositoryOperation::Read)
                .unwrap()
        });

        let found = match service.live_holder(&root_path) {
            crate::repository::LiveHolder::Live(blocker) => {
                assert_eq!(blocker.live, Some(true), "{case}");
                Some(blocker.operation_id.to_string())
            }
            crate::repository::LiveHolder::Nothing => None,
            crate::repository::LiveHolder::Unreadable => panic!("{case}: unreadable"),
        };
        drop(lease);
        assert_eq!(found.as_deref(), named, "{case}");
    }
}
