use super::*;
use crate::repository::{REGISTRY_FILE, discovery::migrate_registry, recovery, *};
use rusqlite::{Connection, params};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
fn db() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    migrate_registry(&mut db).unwrap();
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,'/fixture',123,'accessible',0)", []).unwrap();
    db
}
fn begin(tx: &rusqlite::Transaction<'_>, op: OperationId) -> PromotionRecord {
    begin_record(
        tx,
        1,
        op,
        &ITEM.parse().unwrap(),
        PromotionExpectation::from_bytes([7; 32]),
        None,
    )
    .unwrap()
}
fn oid() -> git2::Oid {
    git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap()
}
fn through_integration(tx: &rusqlite::Transaction<'_>, r: &mut PromotionRecord) {
    r.context_oid = Some(oid());
    r.step = PromotionStep::ContextSynchronized;
    save_record(tx, 1, r).unwrap();
    r.base_oid = Some(oid());
    r.integration_kind = Some(IntegrationKind::AlreadyContained);
    r.step = PromotionStep::Integrating;
    save_record(tx, 1, r).unwrap();
    r.integration_oid = Some(oid());
    r.step = PromotionStep::Integrated;
    save_record(tx, 1, r).unwrap();
}

fn promotion_folder_fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let repo = git2::Repository::init(root.path()).unwrap();
    repo.set_head("refs/heads/main").unwrap();
    let signature = git2::Signature::new(
        "Fixture",
        "fixture@example.invalid",
        &git2::Time::new(123, 0),
    )
    .unwrap();
    let tree = repo
        .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
        .unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[])
        .unwrap();
    repo.config()
        .unwrap()
        .set_str("user.name", "Fixture")
        .unwrap();
    repo.config()
        .unwrap()
        .set_str("user.email", "fixture@example.invalid")
        .unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(EnableRepositoryRequest {
            root: root.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    std::fs::create_dir_all(root.path().join("docs/existing")).unwrap();
    std::fs::write(root.path().join("docs/existing/kept.txt"), b"fixture\n").unwrap();
    assert!(matches!(
        service
            .refresh_repository(RefreshRepositoryRequest {
                root: root.path().into(),
                operation_id: OperationId::new()
            })
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
    (data, root, service)
}

fn folder_cache_image(db: &Connection) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut query = db
                .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
                .unwrap();
            let columns = query.column_count();
            let rows = query
                .query_map([], |r| {
                    (0..columns)
                        .map(|column| r.get(column))
                        .collect::<Result<Vec<rusqlite::types::Value>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}

fn folder_directory_image(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    fn walk(
        root: &std::path::Path,
        path: &std::path::Path,
        image: &mut std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if entry.file_type().unwrap().is_dir() {
                image.insert(relative, None);
                walk(root, &path, image);
            } else {
                image.insert(relative, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut image = std::collections::BTreeMap::new();
    walk(root, &root.join("docs"), &mut image);
    image
}

fn assert_promotion_folder_refused_before_effect(child: bool) {
    let (data, root, service) = promotion_folder_fixture();
    let operation_id = state::with_transaction(&service, root.path(), |tx, id| {
        let r = begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            None,
        )?;
        if child {
            Ok(next_attempt(tx, id, r.operation_id, ChildRole::Context)?
                .context_sync_id
                .unwrap())
        } else {
            Ok(r.operation_id)
        }
    })
    .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT refresh_required FROM repositories WHERE id=1",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let before_cache = folder_cache_image(&db);
    let before_directory = folder_directory_image(root.path());
    let result = service.create_folder(CreateFolderRequest {
        root: root.path().into(),
        path: DocumentFolderPath::parse("docs/forbidden/nested").unwrap(),
        operation_id,
    });
    let directory_unchanged = before_directory == folder_directory_image(root.path());
    let cache_unchanged = before_cache == folder_cache_image(&db);
    assert!(
        directory_unchanged && cache_unchanged,
        "pre-effect refusal: directory unchanged={directory_unchanged}, cache unchanged={cache_unchanged}"
    );
    assert!(!root.path().join("docs/forbidden").exists());
    let error = result.expect_err("a promotion-bound ID must be refused before folder creation");
    assert_eq!(error.error.kind, RepositoryErrorKind::OperationMismatch);
    assert!(!error.left_directories);
}

#[test]
fn spec_p2_create_folder_parent_id_preserves_directory_and_cache() {
    assert_promotion_folder_refused_before_effect(false);
}

#[test]
fn spec_p2_create_folder_unrun_child_id_preserves_directory_and_cache() {
    assert_promotion_folder_refused_before_effect(true);
}

#[test]
fn spec_p2_create_folder_authoritative_child_keeps_exact_refresh_handoff() {
    use crate::repository::remote::reservation;
    for role in [ChildRole::Context, ChildRole::Primary] {
        let (data, root, service) = promotion_folder_fixture();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let r = state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)?;
            let mut r = begin_record(
                tx,
                id,
                OperationId::new(),
                &ITEM.parse().unwrap(),
                PromotionExpectation::from_bytes([7; 32]),
                Some("origin"),
            )?;
            if role == ChildRole::Primary {
                through_integration(tx, &mut r);
            }
            next_attempt(tx, id, r.operation_id, role)
        })
        .unwrap();
        let (child, target) = match role {
            ChildRole::Context => (
                r.context_sync_id.unwrap(),
                RemoteOperationTarget::for_context(
                    &plan,
                    RemoteOperationAction::SynchronizeContext,
                    AuthoringKind::Document,
                    r.item_id.clone(),
                )
                .unwrap(),
            ),
            ChildRole::Publication => unreachable!("a promotion has no publication child"),
            ChildRole::Primary => (
                r.primary_sync_id.unwrap(),
                RemoteOperationTarget::for_primary_synchronization(&plan),
            ),
        };
        let owner = match service
            .reserve_remote_operation(root.path(), child, &target)
            .unwrap()
        {
            RemoteReservationOutcome::Reserved(owner) => owner,
            other => panic!("{other:?}"),
        };
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::FetchPrepared,
                &state::SynchronizationEvidence::default(),
            )
            .unwrap();
        service
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
            .unwrap();
        reservation::commit_observation_batch(&service, root.path(), &owner, &plan, &[], 123)
            .unwrap();
        let oid = git2::Repository::open(root.path())
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap();
        let evidence = state::SynchronizationEvidence {
            expected_oid: Some(oid),
            local_oid: Some(oid),
            tracking_oid: Some(oid),
            primary_tracking_oid: Some(oid),
            push_oid: Some(oid),
            push_advertised_oid: None,
        };
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::PushPrepared,
                &evidence,
            )
            .unwrap();
        let evidence = state::SynchronizationEvidence {
            push_advertised_oid: Some(oid),
            ..evidence
        };
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::PushVerified,
                &evidence,
            )
            .unwrap();
        service
            .classify_synchronization(
                root.path(),
                &owner,
                state::SynchronizationAuthority::AlreadyCurrent(oid),
            )
            .unwrap();
        let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        recovery::require_refresh_can_begin(
            &db,
            root.path(),
            RepositoryOperation::CreateFolder,
            child,
        )
        .unwrap();
        let foreign_root =
            tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        assert_eq!(
            recovery::require_refresh_can_begin(
                &db,
                foreign_root.path(),
                RepositoryOperation::CreateFolder,
                child
            )
            .unwrap_err()
            .kind,
            RepositoryErrorKind::OperationMismatch
        );
        assert!(
            recovery::begin_or_reconcile_operation(
                &mut db,
                root.path(),
                RepositoryOperation::SaveDocument,
                child,
                "",
                0
            )
            .is_err()
        );
        assert!(
            recovery::begin_or_reconcile_operation(
                &mut db,
                root.path(),
                RepositoryOperation::RefreshRepository,
                child,
                "other",
                0
            )
            .is_err()
        );
        assert_eq!(
            service
                .create_folder(CreateFolderRequest {
                    root: root.path().into(),
                    path: DocumentFolderPath::parse("docs/allowed/nested").unwrap(),
                    operation_id: child
                })
                .unwrap(),
            CreateFolderOutcome::Created(FolderDiscovery::Current)
        );
        assert!(root.path().join("docs/allowed/nested").is_dir());
        let local: (String, String, String) = db
            .query_row(
                "SELECT action,target,state FROM operation_records WHERE operation_ulid=?1",
                [child.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(local, ("refresh".into(), "".into(), "completed".into()));
    }
}

#[test]
fn migration_preserves_legacy_rows_is_idempotent_and_audited() {
    let mut db = db();
    db.execute("INSERT INTO operation_records(operation_ulid,repository_id,root_path,action,target,state,observed_at) VALUES(?1,1,'/fixture','refresh','','completed',123)",[OperationId::new().to_string()]).unwrap();
    db.execute(
        "INSERT INTO retired_contexts VALUES(1,'document',?1,?2,?3)",
        params![ITEM, OperationId::new().to_string(), oid().to_string()],
    )
    .unwrap();
    db.execute_batch("DROP TABLE document_promotions; DELETE FROM registry_migrations WHERE name='cycle_09_document_promotions';").unwrap();
    migrate_registry(&mut db).unwrap();
    let changes = db.total_changes();
    let tx = db.transaction().unwrap();
    migrate(&tx).unwrap();
    tx.commit().unwrap();
    assert_eq!(db.total_changes(), changes);
    assert_eq!(
        db.query_row("SELECT root_path FROM repositories WHERE id=1", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "/fixture"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM operation_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT retired_oid FROM retired_contexts", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        oid().to_string()
    );
    audit(&db).unwrap();
}
#[test]
fn half_created_migration_and_missing_marked_table_fail_closed() {
    for marked in [false, true] {
        let mut db = db();
        db.execute_batch("DROP TABLE document_promotions;").unwrap();
        if !marked {
            db.execute_batch("DELETE FROM registry_migrations WHERE name='cycle_09_document_promotions'; CREATE TABLE document_promotions(operation_id TEXT);").unwrap();
        }
        assert_eq!(
            migrate_registry(&mut db).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired
        );
    }
}
#[test]
fn record_forward_transitions_and_invalid_backwards_recover() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new());
    r.step = PromotionStep::Integrated;
    assert_eq!(
        save_record(&tx, 1, &r).unwrap_err().kind,
        RepositoryErrorKind::RecoveryRequired
    );
    r.step = PromotionStep::Confirmed;
    through_integration(&tx, &mut r);
    r.step = PromotionStep::PublicationPending;
    r.publication = Some(PublicationCategory::NoPublicationRemote);
    save_record(&tx, 1, &r).unwrap();
    for step in [
        PromotionStep::WorktreeRemoving,
        PromotionStep::WorktreeRemoved,
        PromotionStep::BranchDeleted,
    ] {
        r.step = step;
        save_record(&tx, 1, &r).unwrap();
    }
    r.step = PromotionStep::Confirmed;
    assert_eq!(
        save_record(&tx, 1, &r).unwrap_err().kind,
        RepositoryErrorKind::RecoveryRequired
    );
    r.step = PromotionStep::Complete;
    assert!(
        save_record(&tx, 1, &r).is_err(),
        "completion requires the retirement transaction"
    );
}
#[test]
fn attempts_are_minted_and_persisted_before_first_execution_and_never_reused() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let r = begin(&tx, OperationId::new());
    let first = next_attempt(&tx, 1, r.operation_id, ChildRole::Context).unwrap();
    assert_eq!(first.context_attempt, 1);
    assert!(first.context_sync_id.is_some());
    assert_eq!(
        read_record(&tx, 1, r.operation_id)
            .unwrap()
            .unwrap()
            .context_sync_id,
        first.context_sync_id
    );
    let second = next_attempt(&tx, 1, r.operation_id, ChildRole::Context).unwrap();
    assert_eq!(second.context_attempt, 2);
    assert_ne!(first.context_sync_id, second.context_sync_id);
    assert!(next_attempt(&tx, 1, r.operation_id, ChildRole::Primary).is_err());
}
#[test]
fn exact_replay_identity_busy_and_stored_abandoned_refusals() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let op = OperationId::new();
    let mut r = begin(&tx, op);
    for (item, digest) in [(OTHER, [7; 32]), (ITEM, [8; 32])] {
        assert_eq!(
            begin_record(
                &tx,
                1,
                op,
                &item.parse().unwrap(),
                PromotionExpectation::from_bytes(digest),
                None
            )
            .unwrap_err()
            .kind,
            RepositoryErrorKind::OperationMismatch
        );
    }
    assert_eq!(
        begin_record(
            &tx,
            1,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            r.expectation,
            None
        )
        .unwrap_err()
        .kind,
        RepositoryErrorKind::RepositoryBusy
    );
    for refusal in [
        Refusal::ExternalChange,
        Refusal::ReconfirmationRequired,
        Refusal::Blocked(vec![
            PromotionBlocker::ContextDirty,
            PromotionBlocker::PrimaryMissing,
        ]),
    ] {
        r.refusal = Some(refusal.clone());
        r.step = PromotionStep::Abandoned;
        save_record(&tx, 1, &r).unwrap();
        assert_eq!(begin(&tx, r.operation_id).refusal, Some(refusal));
        let next = OperationId::new();
        r = begin(&tx, next);
    }
}
#[test]
fn supersession_only_the_three_approved_cleanup_categories() {
    for category in [
        CleanupCategory::ContextChanged,
        CleanupCategory::IgnoredFiles,
        CleanupCategory::RemoteContextChanged,
        CleanupCategory::RecoveryRequired,
        CleanupCategory::Cancelled,
        CleanupCategory::AwaitingPublication,
    ] {
        let mut db = db();
        let tx = db.transaction().unwrap();
        let mut r = begin(&tx, OperationId::new());
        through_integration(&tx, &mut r);
        r.step = PromotionStep::PublicationPending;
        r.publication = Some(PublicationCategory::NoPublicationRemote);
        r.cleanup = Some(category);
        save_record(&tx, 1, &r).unwrap();
        let result = begin_record(&tx, 1, OperationId::new(), &r.item_id, r.expectation, None);
        if matches!(
            category,
            CleanupCategory::ContextChanged
                | CleanupCategory::IgnoredFiles
                | CleanupCategory::RemoteContextChanged
        ) {
            result.unwrap();
            assert_eq!(
                read_record(&tx, 1, r.operation_id).unwrap().unwrap().step,
                PromotionStep::Superseded
            );
            assert!(save_record(&tx, 1, &r).is_err());
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                RepositoryErrorKind::RepositoryBusy
            );
        }
    }
}
#[test]
fn schema_rejects_free_text_categories_steps_and_refusal_payloads() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    begin(&tx, OperationId::new());
    for assignment in [
        "step='body sentinel'",
        "publication_category='ssh://endpoint'",
        "cleanup_category='path sentinel'",
        "refusal_category='title sentinel'",
        "refusal_blockers=131072",
        "expectation=x'01'",
        "context_attempt=-1",
        "integration_mode='body sentinel'",
        "answer_code='path sentinel'",
        "situation='title sentinel'",
        "endpoint_digest=x'01'",
        "primary_branch=''",
    ] {
        assert!(
            tx.execute_batch(&format!("UPDATE document_promotions SET {assignment}"))
                .is_err(),
            "{assignment}"
        );
    }
    // Each new column's own CHECK, on a row where nothing else can refuse the
    // value: a pull-request `complete` row, and a refused direct row that
    // holds none of the six. The value that fits is accepted on the same row.
    let complete = pull_request(PromotionStep::Complete, ITEMS[1]);
    insert_row(&tx, &complete).unwrap();
    let mut direct = pull_request(PromotionStep::Abandoned, ITEMS[2]);
    direct.integration_mode = Integration::Direct;
    direct.answer_code = None;
    direct.refusal = Some(Refusal::Blocked(vec![PromotionBlocker::ContextDirty]));
    bundle(&mut direct, false);
    insert_row(&tx, &direct).unwrap();
    let zero = "0".repeat(40);
    let not_hex = "g".repeat(40);
    for (row, bad, good) in [
        (
            &complete,
            "answer_code='path sentinel'",
            "answer_code='already_promoted'",
        ),
        (
            &complete,
            "answer_code='merged'",
            "answer_code='primary_holds'",
        ),
        (
            &complete,
            "answer_code='Promoted'",
            "answer_code='primary_holds'",
        ),
        (&complete, "situation='title sentinel'", "situation='again'"),
        (
            &complete,
            "endpoint_digest=x'01'",
            "endpoint_digest=randomblob(32)",
        ),
        (
            &complete,
            "endpoint_digest='text sentinel'",
            "endpoint_digest=randomblob(32)",
        ),
        (&complete, "primary_branch=''", "primary_branch='trunk'"),
        (
            &complete,
            &format!("primary_branch='{}'", "a".repeat(256)),
            "primary_branch='trunk'",
        ),
        (
            &complete,
            &format!("consented_context_oid='{zero}'"),
            "consented_context_oid='2222222222222222222222222222222222222222'",
        ),
        (
            &complete,
            &format!("consented_context_oid='{not_hex}'"),
            "consented_context_oid='2222222222222222222222222222222222222222'",
        ),
        (
            &complete,
            "consented_context_oid='1111'",
            "consented_context_oid='2222222222222222222222222222222222222222'",
        ),
        (
            &direct,
            "integration_mode='body sentinel'",
            "integration_mode='direct'",
        ),
        (
            &direct,
            "integration_mode='Pull_Request'",
            "integration_mode='direct'",
        ),
    ] {
        let id = row.operation_id.to_string();
        for (assignment, accepted) in [(bad, false), (good, true)] {
            tx.execute_batch("SAVEPOINT column_check").unwrap();
            let result = tx.execute(
                &format!("UPDATE document_promotions SET {assignment} WHERE operation_id=?1"),
                [&id],
            );
            tx.execute_batch("ROLLBACK TO column_check; RELEASE column_check")
                .unwrap();
            assert_eq!(result.is_ok(), accepted, "{assignment}");
        }
    }
}
#[test]
fn privacy_column_inventory_for_both_new_tables() {
    let db = db();
    for (table, allowed) in [
        (
            "document_promotions",
            vec![
                "repository_id",
                "operation_id",
                "item_id",
                "expectation",
                "remote_name",
                "step",
                "context_attempt",
                "context_sync_id",
                "primary_attempt",
                "primary_sync_id",
                "base_oid",
                "context_oid",
                "integration_oid",
                "integration_kind",
                "remote_context_oid",
                "publication_category",
                "cleanup_category",
                "cancel_requested",
                "updated_at",
                "refusal_category",
                "refusal_blockers",
                "integration_mode",
                "answer_code",
                "consented_context_oid",
                "situation",
                "primary_branch",
                "endpoint_digest",
                "accepted_context_oid",
                "context_pushed",
                "action",
                "unheld_digest",
                "remote_branch",
            ],
        ),
        (
            "retired_contexts",
            vec![
                "repository_id",
                "kind",
                "item_id",
                "operation_id",
                "retired_oid",
            ],
        ),
    ] {
        let columns = db
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(columns, allowed);
    }
}
#[test]
fn local_journal_cannot_begin_with_a_parent_or_unrun_child_id() {
    let mut db = db();
    let (parent, child) = {
        let tx = db.transaction().unwrap();
        let r = begin(&tx, OperationId::new());
        let r = next_attempt(&tx, 1, r.operation_id, ChildRole::Context).unwrap();
        tx.commit().unwrap();
        (r.operation_id, r.context_sync_id.unwrap())
    };
    for op in [parent, child] {
        assert_eq!(
            recovery::begin_or_reconcile_operation(
                &mut db,
                std::path::Path::new("/fixture"),
                RepositoryOperation::RefreshRepository,
                op,
                "",
                0
            )
            .err()
            .unwrap()
            .kind,
            RepositoryErrorKind::OperationMismatch
        );
    }
}
#[test]
fn service_start_audits_promotion_rows_and_schema() {
    let (data, root, service) = service_fixture();
    state::with_transaction(&service, root.path(), |tx, id| {
        let r = begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )?;
        next_attempt(tx, id, r.operation_id, ChildRole::Context)?;
        Ok(())
    })
    .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("DROP INDEX document_promotions_unfinished")
        .unwrap();
    assert_eq!(
        RepositoryService::open_at(data.path()).err().unwrap().kind,
        RepositoryErrorKind::RecoveryRequired
    );
}
fn service_fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,?1,123,'accessible',0)",[root.path().to_str().unwrap()]).unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(
            tx,
            id,
            Some(&RemoteRefPlan::from_configuration("origin", "main").unwrap()),
            state::ConfigurationChange::Plan,
        )
    })
    .unwrap();
    (data, root, service)
}
#[test]
fn promotion_status_reports_parent_step_once_and_offers_resume() {
    let (_data, root, service) = service_fixture();
    git2::Repository::init(root.path()).unwrap();
    let resolved = service.resolve_repository(root.path()).unwrap();
    let r = state::with_transaction(&service, root.path(), |tx, id| {
        begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )
    })
    .unwrap();
    let shown = service.show_operation(&resolved, r.operation_id).unwrap();
    assert_eq!(shown.state, "confirmed");
    assert_eq!(shown.completed_step.as_deref(), Some("confirmed"));
    assert_eq!(shown.next_action, Some(OperationNextAction::Resume));
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    service
        .reserve_remote_operation(
            root.path(),
            r.operation_id,
            &RemoteOperationTarget::for_context(
                &plan,
                RemoteOperationAction::Promote,
                AuthoringKind::Document,
                r.item_id.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    let listed = service.list_operations(&resolved).unwrap();
    assert_eq!(
        listed
            .items
            .iter()
            .filter(|v| v.operation_id.as_deref() == Some(&r.operation_id.to_string()))
            .count(),
        1
    );
    state::with_transaction(&service, root.path(), |tx, id| {
        let mut r = read_record(tx, id, r.operation_id)?.unwrap();
        through_integration(tx, &mut r);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        service
            .show_operation(&resolved, r.operation_id)
            .unwrap()
            .state,
        "integrated"
    );
}
#[test]
fn primary_advance_is_the_only_reset_and_mints_a_new_context_attempt() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let r = begin_record(
        &tx,
        1,
        OperationId::new(),
        &ITEM.parse().unwrap(),
        PromotionExpectation::from_bytes([7; 32]),
        Some("origin"),
    )
    .unwrap();
    assert!(reset_context_after_primary_advance(&tx, 1, r.operation_id).is_err());
    let mut r = next_attempt(&tx, 1, r.operation_id, ChildRole::Context).unwrap();
    let child = r.context_sync_id;
    r.step = PromotionStep::ContextSynchronized;
    r.context_oid = Some(oid());
    r.remote_context_oid = Some(oid());
    save_record(&tx, 1, &r).unwrap();
    let r = reset_context_after_primary_advance(&tx, 1, r.operation_id).unwrap();
    assert_eq!(r.step, PromotionStep::Confirmed);
    assert_eq!(r.context_attempt, 2);
    assert_ne!(r.context_sync_id, child);
    assert!(r.context_oid.is_none());
    assert!(r.remote_context_oid.is_none());
}
#[test]
fn integrated_evidence_and_observed_publication_cannot_be_rewritten() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new());
    through_integration(&tx, &mut r);
    r.step = PromotionStep::PublicationPending;
    r.publication = Some(PublicationCategory::NoPublicationRemote);
    save_record(&tx, 1, &r).unwrap();
    let original = r.integration_oid;
    r.integration_oid =
        Some(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap());
    assert!(save_record(&tx, 1, &r).is_err());
    r.integration_oid = original;
    let mut remote = begin_record(
        &tx,
        1,
        OperationId::new(),
        &OTHER.parse().unwrap(),
        r.expectation,
        Some("origin"),
    )
    .unwrap();
    through_integration(&tx, &mut remote);
    remote.step = PromotionStep::Published;
    remote.publication = Some(PublicationCategory::Published);
    save_record(&tx, 1, &remote).unwrap();
    remote.publication = Some(PublicationCategory::RecoveryRequired);
    assert!(save_record(&tx, 1, &remote).is_err());
}
#[test]
fn audit_rejects_foreign_child_scope_instead_of_treating_it_as_absence() {
    let (data, root, service) = service_fixture();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let r = state::with_transaction(&service, root.path(), |tx, id| {
        begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )
    })
    .unwrap();
    let other = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)",[other.path().to_str().unwrap()]).unwrap();
    state::with_transaction(&service, other.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let child = OperationId::new();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Document,
        r.item_id.clone(),
    )
    .unwrap();
    service
        .reserve_remote_operation(other.path(), child, &target)
        .unwrap();
    db.execute(
        "UPDATE document_promotions SET context_attempt=1,context_sync_id=?1 WHERE operation_id=?2",
        params![child.to_string(), r.operation_id.to_string()],
    )
    .unwrap();
    let r = read_record(&db, 1, r.operation_id).unwrap().unwrap();
    assert!(child_state(&db, &r, ChildRole::Context).is_err());
    assert!(audit(&db).is_err());
}
#[test]
fn cancellation_flags_parent_and_owned_child_without_replacing_retained_attempt() {
    let (_data, root, service) = service_fixture();
    let r = state::with_transaction(&service, root.path(), |tx, id| {
        let r = begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )?;
        next_attempt(tx, id, r.operation_id, ChildRole::Context)
    })
    .unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        request_cancellation(tx, id, r.operation_id)?;
        assert!(
            read_record(tx, id, r.operation_id)?
                .unwrap()
                .cancel_requested
        );
        Ok(())
    })
    .unwrap();
    assert_eq!(r.context_attempt, 1);
}
#[test]
fn supersession_fences_and_releases_only_the_old_parents_promote_reservation() {
    let (_data, root, service) = service_fixture();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let old = state::with_transaction(&service, root.path(), |tx, id| {
        let mut r = begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )?;
        through_integration(tx, &mut r);
        r.step = PromotionStep::Published;
        r.publication = Some(PublicationCategory::Published);
        r.cleanup = Some(CleanupCategory::RemoteContextChanged);
        save_record(tx, id, &r)?;
        Ok(r)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::Promote,
        AuthoringKind::Document,
        old.item_id.clone(),
    )
    .unwrap();
    let owner = match service
        .reserve_remote_operation(root.path(), old.operation_id, &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    state::with_transaction(&service, root.path(), |tx, id| {
        begin_record(
            tx,
            id,
            OperationId::new(),
            &old.item_id,
            old.expectation,
            Some("origin"),
        )
    })
    .unwrap();
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
    assert!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::BeforeTransport
            )
            .is_err()
    );
    assert!(
        service
            .restart_remote_promotion(root.path(), old.operation_id, &target)
            .is_err()
    );
}
#[test]
fn parent_and_child_ids_cross_scope_but_own_promote_reservation_is_accepted() {
    let (_data, root, service) = service_fixture();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let r = state::with_transaction(&service, root.path(), |tx, id| {
        let r = begin_record(
            tx,
            id,
            OperationId::new(),
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )?;
        next_attempt(tx, id, r.operation_id, ChildRole::Context)
    })
    .unwrap();
    let context = |item: &str| {
        RemoteOperationTarget::for_context(
            &plan,
            RemoteOperationAction::SynchronizeContext,
            AuthoringKind::Document,
            item.parse().unwrap(),
        )
        .unwrap()
    };
    for (op, target) in [
        (r.operation_id, context(ITEM)),
        (r.context_sync_id.unwrap(), context(OTHER)),
        (
            r.context_sync_id.unwrap(),
            RemoteOperationTarget::for_primary_synchronization(&plan),
        ),
    ] {
        assert_eq!(
            service
                .reserve_remote_operation(root.path(), op, &target)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::OperationMismatch
        );
    }
    let other = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    Connection::open(&service.registry_path).unwrap().execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)",[other.path().to_str().unwrap()]).unwrap();
    state::with_transaction(&service, other.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    assert_eq!(
        service
            .reserve_remote_operation(other.path(), r.context_sync_id.unwrap(), &context(ITEM))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    let own = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::Promote,
        AuthoringKind::Document,
        r.item_id.clone(),
    )
    .unwrap();
    let owner = match service
        .reserve_remote_operation(root.path(), r.operation_id, &own)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    state::with_transaction(&service, root.path(), |tx, id| {
        begin_record(
            tx,
            id,
            r.operation_id,
            &r.item_id,
            r.expectation,
            Some("origin"),
        )?;
        Ok(())
    })
    .unwrap();
    service
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
}
#[test]
fn promotion_rejects_local_foreign_remote_and_comment_binding_ids() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let op = OperationId::new();
    tx.execute("INSERT INTO operation_records(repository_id,operation_ulid,root_path,action,target,state,observed_at) VALUES(1,?1,'/fixture','refresh','','completed',123)",[op.to_string()]).unwrap();
    assert_eq!(
        begin_record(
            &tx,
            1,
            op,
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            None
        )
        .unwrap_err()
        .kind,
        RepositoryErrorKind::OperationMismatch
    );
    let comment_child = OperationId::new();
    tx.execute("INSERT INTO comment_publication_bindings(operation_ulid,local_record_id,root_digest,kind,item_ulid,comment_ulid,synchronization_ulid,pre_checkpoint_oid) SELECT operation_ulid,id,?1,'document',?2,?3,?4,?5 FROM operation_records WHERE operation_ulid=?6",params![[7u8;32].as_slice(),ITEM,OTHER,comment_child.to_string(),oid().to_string(),op.to_string()]).unwrap();
    assert_eq!(
        begin_record(
            &tx,
            1,
            comment_child,
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            None
        )
        .unwrap_err()
        .kind,
        RepositoryErrorKind::OperationMismatch
    );
    drop(tx);
    let (_data, root, service) = service_fixture();
    let op = OperationId::new();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    service
        .reserve_remote_operation(
            root.path(),
            op,
            &RemoteOperationTarget::for_primary_synchronization(&plan),
        )
        .unwrap();
    assert_eq!(
        state::with_transaction(&service, root.path(), |tx, id| begin_record(
            tx,
            id,
            op,
            &ITEM.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin")
        ))
        .unwrap_err()
        .kind,
        RepositoryErrorKind::OperationMismatch
    );
}

#[test]
fn final_sql_rollback_preserves_history_parent_reservation_and_tracking_guard() {
    use crate::repository::cleanup::ContextCleanup;
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let repo = git2::Repository::init(root.path()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,?1,123,'accessible',0)",[root.path().to_str().unwrap()]).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let item: crate::canonical::ItemId = ITEM.parse().unwrap();
    let target = plan.context(AuthoringKind::Document, &item);
    let blob = repo.blob(b"fixture").unwrap();
    repo.reference(target.tracking_ref(), blob, false, "fixture")
        .unwrap();
    let op = OperationId::new();
    state::with_transaction(&service,root.path(),|tx,id| {
        state::configure(tx,id,Some(&plan),state::ConfigurationChange::Plan)?;
        let mut r=begin_record(tx,id,op,&item,PromotionExpectation::from_bytes([7;32]),Some("origin"))?;
        through_integration(tx,&mut r);
        r.step=PromotionStep::Published; r.publication=Some(PublicationCategory::Published); save_record(tx,id,&r)?;
        for step in [PromotionStep::RemoteDeleted,PromotionStep::WorktreeRemoving,PromotionStep::WorktreeRemoved,PromotionStep::BranchDeleted] {r.step=step;save_record(tx,id,&r)?;}
        tx.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,last_advertised_oid,publication_evidence,state,observed_at) VALUES(?1,?2,?3,'document',?4,?5,'observed_published','remotely_deleted',123)",params![id,target.remote_ref(),target.tracking_ref(),ITEM,blob.to_string()]).unwrap();
        Ok(())
    }).unwrap();
    let remote = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::Promote,
        AuthoringKind::Document,
        item.clone(),
    )
    .unwrap();
    let owner = match service
        .reserve_remote_operation(root.path(), op, &remote)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
    ] {
        service
            .remote_safe_point(root.path(), &owner, point)
            .unwrap();
    }
    let cleanup = ContextCleanup::acquire(root.path()).unwrap();
    let evidence = cleanup
        .delete_tracking_ref(
            Some(&plan),
            AuthoringKind::Document,
            &item,
            Some(blob),
            op,
            oid(),
        )
        .unwrap();
    let sql_image = |db: &Connection| {
        [
            "repositories",
            "remote_polling_state",
            "remote_observation_batches",
            "remote_ref_observations",
            "remote_context_states",
            "retired_contexts",
            "document_promotions",
            "remote_operation_records",
        ]
        .map(|table| {
            let mut query = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let columns = query.column_count();
            query
                .query_map([], |r| {
                    (0..columns)
                        .map(|column| r.get::<_, rusqlite::types::Value>(column))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        })
    };
    let before_sql = sql_image(&db);
    db.execute_batch("CREATE TRIGGER fail_completion BEFORE UPDATE OF phase ON remote_operation_records WHEN NEW.phase='completed' BEGIN SELECT RAISE(ABORT,'fixed failure'); END").unwrap();
    assert!(
        state::with_transaction(&service, root.path(), |tx, id| complete_record(
            tx,
            &service,
            id,
            op,
            &evidence,
            Some(&owner)
        ))
        .is_err()
    );
    assert!(
        before_sql == sql_image(&db),
        "every completion SQL field and the reservation must roll back together"
    );
    assert!(repo.find_reference(target.tracking_ref()).is_err());
    assert!(
        repo.reference(target.tracking_ref(), blob, true, "foreign")
            .is_err(),
        "absent slot lock survives rollback and reconciliation"
    );
    assert_eq!(
        db.query_row(
            "SELECT step FROM document_promotions WHERE operation_id=?1",
            [op.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "branch_deleted"
    );
    assert_eq!(
        db.query_row(
            "SELECT publication_evidence FROM remote_context_states",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "observed_published"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_some()
    );
    db.execute_batch("DROP TRIGGER fail_completion").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        complete_record(tx, &service, id, op, &evidence, Some(&owner))
    })
    .unwrap();
    // Simulate loss of the commit result: actual stored completion wins, including
    // reservation completion, without relying on the stale active-owner token.
    state::with_transaction(&service, root.path(), |tx, id| {
        assert!(reconcile_completion(tx, id, op)?);
        Ok(())
    })
    .unwrap();
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    // A later observed publication removes the retirement. It does not revoke
    // the old parent's authoritative completion or require replayed cleanup.
    db.execute_batch("DELETE FROM retired_contexts").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        assert!(reconcile_completion(tx, id, op)?);
        Ok(())
    })
    .unwrap();
    assert!(
        repo.reference(target.tracking_ref(), blob, true, "foreign")
            .is_err()
    );
    drop(evidence);
    drop(cleanup);
    repo.reference(target.tracking_ref(), blob, true, "foreign")
        .unwrap();
}

#[test]
fn finalization_without_applicable_reservation_does_not_fabricate_history() {
    for remote in [false, true] {
        let (data, root, service) = service_fixture();
        git2::Repository::init(root.path()).unwrap();
        let plan = remote.then(|| RemoteRefPlan::from_configuration("origin", "main").unwrap());
        let op = OperationId::new();
        let r = state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, plan.as_ref(), state::ConfigurationChange::Plan)?;
            let mut r = begin_record(
                tx,
                id,
                op,
                &ITEM.parse().unwrap(),
                PromotionExpectation::from_bytes([7; 32]),
                remote.then_some("origin"),
            )?;
            through_integration(tx, &mut r);
            r.step = if remote {
                PromotionStep::Published
            } else {
                PromotionStep::PublicationPending
            };
            r.publication = Some(if remote {
                PublicationCategory::AlreadyCurrent
            } else {
                PublicationCategory::NoPublicationRemote
            });
            save_record(tx, id, &r)?;
            if remote {
                r.step = PromotionStep::RemoteDeleted;
                save_record(tx, id, &r)?;
            }
            for step in [
                PromotionStep::WorktreeRemoving,
                PromotionStep::WorktreeRemoved,
                PromotionStep::BranchDeleted,
            ] {
                r.step = step;
                save_record(tx, id, &r)?;
            }
            Ok(r)
        })
        .unwrap();
        let cleanup = crate::repository::cleanup::ContextCleanup::acquire(root.path()).unwrap();
        let evidence = cleanup
            .delete_tracking_ref(
                plan.as_ref(),
                AuthoringKind::Document,
                &r.item_id,
                None,
                op,
                oid(),
            )
            .unwrap();
        state::with_transaction(&service, root.path(), |tx, id| {
            complete_record(tx, &service, id, op, &evidence, None)
        })
        .unwrap();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        state::with_transaction(&service, root.path(), |tx, id| {
            assert!(reconcile_completion(tx, id, op)?);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
    }
}

fn refused_promotion(
    tx: &rusqlite::Transaction<'_>,
    blockers: Vec<PromotionBlocker>,
) -> PromotionRecord {
    refuse_record(
        tx,
        1,
        OperationId::new(),
        &ITEM.parse().unwrap(),
        PromotionExpectation::from_bytes([7; 32]),
        None,
        &Refusal::Blocked(blockers),
    )
    .unwrap()
}
#[test]
fn blocker_mask_round_trips_all_17() {
    assert_eq!(BLOCKERS.len(), 17);
    assert!(BLOCKERS.windows(2).all(|p| p[0] < p[1]));
    assert_eq!(BLOCKERS[16], PromotionBlocker::HistoryUnresolved);
    let mut db = db();
    let tx = db.transaction().unwrap();
    for (bit, blocker) in BLOCKERS.iter().enumerate() {
        let r = refused_promotion(&tx, vec![*blocker]);
        let mask: i64 = tx
            .query_row(
                "SELECT refusal_blockers FROM document_promotions WHERE operation_id=?1",
                [r.operation_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mask, 1 << bit);
        assert_eq!(
            read_record(&tx, 1, r.operation_id)
                .unwrap()
                .unwrap()
                .refusal,
            Some(Refusal::Blocked(vec![*blocker]))
        );
    }
    let every = refused_promotion(&tx, BLOCKERS.to_vec());
    assert_eq!(
        read_record(&tx, 1, every.operation_id)
            .unwrap()
            .unwrap()
            .refusal,
        Some(Refusal::Blocked(BLOCKERS.to_vec()))
    );
}
/// A registry from before the widened mask CHECK is rebuilt once, keeping
/// its rows, and then takes the new bit.
#[test]
fn promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let old = refused_promotion(&tx, vec![PromotionBlocker::ContextDirty]);
    // The table as v1 left it: the v2 layout with the mask one bit narrower.
    tx.execute_batch(&format!(
        "DROP INDEX document_promotions_unfinished; ALTER TABLE document_promotions RENAME TO before; {}; {INDEX}; INSERT INTO document_promotions({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM before; DROP TABLE before; DELETE FROM registry_migrations WHERE name IN ('promotion_refusal_blockers_v2','wave_06_promotion_modes_v1','wave_06_promotion_cleanup_v1');",
        SCHEMA_V2.replace("131071", "65535")
    ))
    .unwrap();
    assert!(
        tx.execute("UPDATE document_promotions SET refusal_blockers=65536", [])
            .is_err()
    );
    for _ in 0..2 {
        migrate(&tx).unwrap();
        assert_eq!(
            read_record(&tx, 1, old.operation_id)
                .unwrap()
                .unwrap()
                .refusal,
            old.refusal
        );
        let marked: i64 = tx
            .query_row("SELECT count(*) FROM registry_migrations WHERE name='promotion_refusal_blockers_v2'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(marked, 1);
    }
    let new = refused_promotion(&tx, vec![PromotionBlocker::HistoryUnresolved]);
    assert_eq!(
        read_record(&tx, 1, new.operation_id)
            .unwrap()
            .unwrap()
            .refusal,
        new.refusal
    );
}

const ITEMS: [&str; 5] = [
    "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    "01BX5ZZKBKACTAV9WEVGEMMVRZ",
    "01CX5ZZKBKACTAV9WEVGEMMVRZ",
    "01DX5ZZKBKACTAV9WEVGEMMVRZ",
    "01EX5ZZKBKACTAV9WEVGEMMVRZ",
];
const ALL_STEPS: [PromotionStep; 14] = [
    PromotionStep::Confirmed,
    PromotionStep::ContextSynchronized,
    PromotionStep::Integrating,
    PromotionStep::Integrated,
    PromotionStep::Published,
    PromotionStep::PublicationPending,
    PromotionStep::RemoteDeleting,
    PromotionStep::RemoteDeleted,
    PromotionStep::WorktreeRemoving,
    PromotionStep::WorktreeRemoved,
    PromotionStep::BranchDeleted,
    PromotionStep::Complete,
    PromotionStep::Abandoned,
    PromotionStep::Superseded,
];
fn dump(db: &Connection, columns: &str) -> Vec<Vec<rusqlite::types::Value>> {
    let mut statement = db
        .prepare(&format!(
            "SELECT {columns} FROM document_promotions ORDER BY rowid"
        ))
        .unwrap();
    let width = statement.column_count();
    statement
        .query_map([], |row| {
            (0..width)
                .map(|i| row.get::<_, rusqlite::types::Value>(i))
                .collect()
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
fn marker_count(db: &Connection, name: &str) -> i64 {
    db.query_row(
        "SELECT count(*) FROM registry_migrations WHERE name=?1",
        [name],
        |r| r.get(0),
    )
    .unwrap()
}
const MODES_MARKER: &str = "wave_06_promotion_modes_v1";

/// A registry that holds a v2 table (the layout before the modes) with rows of
/// several steps is rebuilt once: every row stays, byte for byte, as `direct`
/// with the new columns null; a second migration changes nothing; a fresh
/// registry is created with the new table and every marker.
#[test]
fn promotion_modes_migration_rebuilds_a_v2_table_once() {
    let mut fresh = db();
    let mut db = db();
    let tx = db.transaction().unwrap();
    let item = |n: usize| -> ItemId { ITEMS[n].parse().unwrap() };
    let begin_for = |n: usize| {
        begin_record(
            &tx,
            1,
            OperationId::new(),
            &item(n),
            PromotionExpectation::from_bytes([n as u8; 32]),
            Some("origin"),
        )
        .unwrap()
    };
    let _confirmed = begin_for(0);
    let mut synchronized = begin_for(1);
    synchronized.context_oid = Some(oid());
    synchronized.step = PromotionStep::ContextSynchronized;
    save_record(&tx, 1, &synchronized).unwrap();
    let mut integrated = begin_for(2);
    through_integration(&tx, &mut integrated);
    let blocked = refuse_record(
        &tx,
        1,
        OperationId::new(),
        &item(3),
        PromotionExpectation::from_bytes([3; 32]),
        Some("origin"),
        &Refusal::Blocked(vec![
            PromotionBlocker::ContextDirty,
            PromotionBlocker::HistoryUnresolved,
        ]),
    )
    .unwrap();
    let external = refuse_record(
        &tx,
        1,
        OperationId::new(),
        &item(4),
        PromotionExpectation::from_bytes([4; 32]),
        None,
        &Refusal::ExternalChange,
    )
    .unwrap();
    let before = records(&tx, 1).unwrap();
    assert_eq!(before.len(), 5);
    let v2_before = dump(&tx, V2_COLUMNS);
    // The refused row's mask is stored as the two blockers' bits.
    assert!(
        v2_before
            .iter()
            .any(|row| row.contains(&rusqlite::types::Value::Integer((1 << 3) | (1 << 16)))),
        "the refused row holds its blockers"
    );
    // The table as the v2 rebuild left it, with the modes marker not yet run.
    tx.execute_batch(&format!(
        "DROP INDEX document_promotions_unfinished; ALTER TABLE document_promotions RENAME TO before; {SCHEMA_V2}; {INDEX}; INSERT INTO document_promotions({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM before ORDER BY rowid; DROP TABLE before; DELETE FROM registry_migrations WHERE name IN ('{MODES_MARKER}','{CLEANUP_MARKER}');"
    ))
    .unwrap();
    assert_eq!(dump(&tx, "*").first().map(Vec::len), Some(21));
    assert_eq!(dump(&tx, V2_COLUMNS), v2_before);
    assert_eq!(marker_count(&tx, MODES_MARKER), 0);
    migrate(&tx).unwrap();
    assert_eq!(marker_count(&tx, MODES_MARKER), 1);
    assert_eq!(
        dump(&tx, V2_COLUMNS),
        v2_before,
        "every v2 column, byte for byte"
    );
    let after = records(&tx, 1).unwrap();
    assert_eq!(after, before);
    for r in &after {
        assert_eq!(r.integration_mode, Integration::Direct);
        assert_eq!(r.path(), RecordPath::Direct);
        assert!(!r.answered());
        assert_eq!(
            (
                r.answer_code,
                r.consented_context_oid,
                r.situation,
                &r.primary_branch,
                r.endpoint_digest
            ),
            (None, None, None, &None, None)
        );
    }
    assert_eq!(
        after
            .iter()
            .find(|r| r.operation_id == blocked.operation_id)
            .unwrap()
            .refusal,
        blocked.refusal
    );
    assert_eq!(
        after
            .iter()
            .find(|r| r.operation_id == external.operation_id)
            .unwrap()
            .remote_name,
        None
    );
    assert_eq!(
        tx.query_row(
            "SELECT sql FROM sqlite_master WHERE name='document_promotions'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        SCHEMA
    );
    let changes = tx.total_changes();
    migrate(&tx).unwrap();
    assert_eq!(
        tx.total_changes(),
        changes,
        "a second migration changes nothing"
    );
    assert_eq!(marker_count(&tx, MODES_MARKER), 1);
    assert_eq!(records(&tx, 1).unwrap(), before);
    drop(tx);

    // A fresh registry is created with the new table and every marker.
    let tx = fresh.transaction().unwrap();
    for marker in [
        "cycle_09_document_promotions",
        "promotion_refusal_blockers_v2",
        MODES_MARKER,
        CLEANUP_MARKER,
    ] {
        assert_eq!(marker_count(&tx, marker), 1, "{marker}");
    }
    validate_schema(&tx).unwrap();
    let r = begin(&tx, OperationId::new());
    assert_eq!(r.integration_mode, Integration::Direct);
}

/// Inserts every column of `r` as it stands, as a begin or a finish would
/// leave it; the SQL decides whether the row may exist.
fn insert_row(db: &Connection, r: &PromotionRecord) -> rusqlite::Result<usize> {
    let (category, mask) = Refusal::encoded_columns(r.refusal.as_ref()).unwrap();
    db.execute(
        "INSERT INTO document_promotions(repository_id,operation_id,item_id,expectation,remote_name,step,context_attempt,context_sync_id,primary_attempt,primary_sync_id,base_oid,context_oid,integration_oid,integration_kind,remote_context_oid,publication_category,cleanup_category,cancel_requested,updated_at,refusal_category,refusal_blockers,integration_mode,answer_code,consented_context_oid,situation,primary_branch,endpoint_digest,accepted_context_oid,context_pushed) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29)",
        params![
            r.repository_id,
            r.operation_id.to_string(),
            r.item_id.to_string(),
            r.expectation.to_bytes().as_slice(),
            r.remote_name,
            r.step.name(),
            r.context_attempt,
            r.context_sync_id.map(|o| o.to_string()),
            r.primary_attempt,
            r.primary_sync_id.map(|o| o.to_string()),
            r.base_oid.map(|o| o.to_string()),
            r.context_oid.map(|o| o.to_string()),
            r.integration_oid.map(|o| o.to_string()),
            r.integration_kind.map(IntegrationKind::name),
            r.remote_context_oid.map(|o| o.to_string()),
            r.publication.map(PublicationCategory::name),
            r.cleanup.map(CleanupCategory::name),
            r.cancel_requested,
            r.updated_at,
            category,
            mask,
            r.integration_mode.as_str(),
            r.answer_code.map(PromotionAnswer::name),
            r.consented_context_oid.map(|o| o.to_string()),
            r.situation.map(PromotionSituation::name),
            r.primary_branch,
            r.endpoint_digest.as_ref().map(|d| d.as_slice()),
            r.accepted_context_oid.map(|o| o.to_string()),
            r.context_pushed,
        ],
    )
}
/// A pull-request row as the design says it stands at `step`, valid.
fn pull_request(step: PromotionStep, item: &str) -> PromotionRecord {
    use PromotionStep::*;
    PromotionRecord {
        repository_id: 1,
        operation_id: OperationId::new(),
        item_id: item.parse().unwrap(),
        expectation: PromotionExpectation::from_bytes([7; 32]),
        remote_name: Some("origin".into()),
        step,
        context_attempt: 0,
        context_sync_id: None,
        primary_attempt: 0,
        primary_sync_id: None,
        base_oid: None,
        context_oid: matches!(step, ContextSynchronized | Complete).then(oid),
        integration_oid: None,
        integration_kind: None,
        remote_context_oid: None,
        publication: (step == Complete).then_some(PublicationCategory::Published),
        cleanup: None,
        cancel_requested: false,
        updated_at: 1,
        refusal: (step == Abandoned).then_some(Refusal::ReconfirmationRequired),
        integration_mode: Integration::PullRequest,
        answer_code: match step {
            Complete => Some(PromotionAnswer::Promoted),
            Abandoned => Some(PromotionAnswer::ContextGone),
            _ => None,
        },
        consented_context_oid: Some(oid()),
        situation: Some(PromotionSituation::First),
        primary_branch: Some("main".into()),
        endpoint_digest: Some([9; 32]),
        accepted_context_oid: None,
        context_pushed: false,
        action: PromotionAction::Promote,
        unheld_digest: None,
        remote_branch: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    /// The SQL accepts, `validate` accepts and the row reads back as written.
    Ok,
    /// The SQL accepts and `validate` refuses it, on read as on save.
    Rust,
    /// The SQL refuses it, and so does `validate`.
    Both,
}
type Mutate = fn(&mut PromotionRecord);
fn bundle(r: &mut PromotionRecord, set: bool) {
    r.consented_context_oid = set.then(oid);
    r.situation = set.then_some(PromotionSituation::Again);
    r.primary_branch = set.then(|| "main".to_owned());
    r.endpoint_digest = set.then_some([9; 32]);
}

#[test]
fn pull_request_promotion_steps_and_checks() {
    use PromotionStep::*;
    let cases: Vec<(&str, PromotionStep, Mutate, Expect)> = vec![
        ("confirmed", Confirmed, |_| {}, Expect::Ok),
        (
            "context_synchronized",
            ContextSynchronized,
            |_| {},
            Expect::Ok,
        ),
        ("complete promoted", Complete, |_| {}, Expect::Ok),
        (
            "complete already_current, already_promoted",
            Complete,
            |r| {
                r.publication = Some(PublicationCategory::AlreadyCurrent);
                r.answer_code = Some(PromotionAnswer::AlreadyPromoted);
                r.situation = Some(PromotionSituation::Again);
            },
            Expect::Ok,
        ),
        (
            "complete primary_holds",
            Complete,
            |r| r.answer_code = Some(PromotionAnswer::PrimaryHolds),
            Expect::Ok,
        ),
        ("abandoned context_gone", Abandoned, |_| {}, Expect::Ok),
        (
            "abandoned primary_holds",
            Abandoned,
            |r| r.answer_code = Some(PromotionAnswer::PrimaryHolds),
            Expect::Ok,
        ),
        (
            "abandoned remote_branch_deleted",
            Abandoned,
            |r| r.answer_code = Some(PromotionAnswer::RemoteBranchDeleted),
            Expect::Ok,
        ),
        (
            "abandoned history_unresolved",
            Abandoned,
            |r| r.answer_code = Some(PromotionAnswer::HistoryUnresolved),
            Expect::Ok,
        ),
        (
            "refused blocked, no begin",
            Abandoned,
            |r| {
                r.refusal = Some(Refusal::Blocked(vec![PromotionBlocker::ContextDirty]));
                r.answer_code = None;
                r.remote_name = None;
                bundle(r, false);
            },
            Expect::Ok,
        ),
        (
            "refused external_change, no begin, a remote",
            Abandoned,
            |r| {
                r.refusal = Some(Refusal::ExternalChange);
                r.answer_code = None;
                bundle(r, false);
            },
            Expect::Ok,
        ),
        (
            "refused reconfirmation_required, no begin",
            Abandoned,
            |r| {
                r.answer_code = None;
                bundle(r, false);
            },
            Expect::Ok,
        ),
        (
            "refused external_change after begin (RR4, RR5)",
            Abandoned,
            |r| {
                r.refusal = Some(Refusal::ExternalChange);
                r.answer_code = None;
            },
            Expect::Ok,
        ),
        // A refused record whose primary branch is set but whose remote is not.
        (
            "refused, primary_branch set, remote_name null",
            Abandoned,
            |r| {
                r.refusal = Some(Refusal::ExternalChange);
                r.answer_code = None;
                r.remote_name = None;
            },
            Expect::Rust,
        ),
        // The integration and cleanup columns.
        (
            "base_oid",
            Confirmed,
            |r| r.base_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "integration_oid",
            Confirmed,
            |r| r.integration_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "integration_kind",
            Confirmed,
            |r| r.integration_kind = Some(IntegrationKind::Merged),
            Expect::Both,
        ),
        (
            "cleanup",
            Confirmed,
            |r| r.cleanup = Some(CleanupCategory::ContextChanged),
            Expect::Both,
        ),
        (
            "a primary child",
            Confirmed,
            |r| {
                r.primary_attempt = 1;
                r.primary_sync_id = Some(OperationId::new());
            },
            Expect::Both,
        ),
        (
            "a publication at confirmed",
            Confirmed,
            |r| r.publication = Some(PublicationCategory::Published),
            Expect::Rust,
        ),
        (
            "a publication at abandoned",
            Abandoned,
            |r| r.publication = Some(PublicationCategory::Published),
            Expect::Rust,
        ),
        // Every step a pull-request row never reaches.
        (
            "integrating",
            Integrating,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "integrated",
            Integrated,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "published",
            Published,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "publication_pending",
            PublicationPending,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "remote_deleted",
            RemoteDeleted,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "worktree_removing",
            WorktreeRemoving,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "worktree_removed",
            WorktreeRemoved,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        (
            "branch_deleted",
            BranchDeleted,
            |r| r.context_oid = Some(oid()),
            Expect::Both,
        ),
        ("superseded", Superseded, |_| {}, Expect::Both),
        // The four begin-time columns.
        (
            "none of the four at confirmed",
            Confirmed,
            |r| bundle(r, false),
            Expect::Both,
        ),
        (
            "none of the four at context_synchronized",
            ContextSynchronized,
            |r| bundle(r, false),
            Expect::Both,
        ),
        (
            "none of the four at complete",
            Complete,
            |r| bundle(r, false),
            Expect::Both,
        ),
        (
            "a consented tip alone",
            Abandoned,
            |r| {
                bundle(r, false);
                r.consented_context_oid = Some(oid());
            },
            Expect::Both,
        ),
        (
            "a situation alone",
            Abandoned,
            |r| {
                bundle(r, false);
                r.situation = Some(PromotionSituation::First);
            },
            Expect::Both,
        ),
        (
            "a digest alone",
            Abandoned,
            |r| {
                bundle(r, false);
                r.endpoint_digest = Some([1; 32]);
            },
            Expect::Both,
        ),
        (
            "three of the four",
            Confirmed,
            |r| r.primary_branch = None,
            Expect::Both,
        ),
        (
            "an empty primary branch",
            Confirmed,
            |r| r.primary_branch = Some(String::new()),
            Expect::Both,
        ),
        (
            "a long primary branch",
            Confirmed,
            |r| r.primary_branch = Some("a".repeat(256)),
            Expect::Both,
        ),
        (
            "HEAD as the primary branch",
            Confirmed,
            |r| r.primary_branch = Some("HEAD".into()),
            Expect::Rust,
        ),
        (
            "an invalid primary branch",
            Confirmed,
            |r| r.primary_branch = Some("a..b".into()),
            Expect::Rust,
        ),
        // The answer.
        (
            "an answer at confirmed",
            Confirmed,
            |r| r.answer_code = Some(PromotionAnswer::Promoted),
            Expect::Both,
        ),
        (
            "an answer at context_synchronized",
            ContextSynchronized,
            |r| r.answer_code = Some(PromotionAnswer::Promoted),
            Expect::Both,
        ),
        (
            "complete with no answer",
            Complete,
            |r| r.answer_code = None,
            Expect::Both,
        ),
        (
            "complete with an ended answer",
            Complete,
            |r| r.answer_code = Some(PromotionAnswer::RemoteBranchDeleted),
            Expect::Rust,
        ),
        (
            "complete with context_gone",
            Complete,
            |r| r.answer_code = Some(PromotionAnswer::ContextGone),
            Expect::Rust,
        ),
        (
            "an answer with no begin-time columns",
            Abandoned,
            |r| bundle(r, false),
            Expect::Both,
        ),
        (
            "abandoned with promoted",
            Abandoned,
            |r| r.answer_code = Some(PromotionAnswer::Promoted),
            Expect::Rust,
        ),
        (
            "abandoned with already_promoted",
            Abandoned,
            |r| r.answer_code = Some(PromotionAnswer::AlreadyPromoted),
            Expect::Rust,
        ),
        (
            "an answer with a blocked refusal",
            Abandoned,
            |r| r.refusal = Some(Refusal::Blocked(vec![PromotionBlocker::ContextDirty])),
            Expect::Both,
        ),
        (
            "an answer with external_change",
            Abandoned,
            |r| r.refusal = Some(Refusal::ExternalChange),
            Expect::Both,
        ),
        (
            "abandoned with no refusal",
            Abandoned,
            |r| {
                r.refusal = None;
                r.answer_code = None;
            },
            Expect::Both,
        ),
        (
            "complete with a refusal",
            Complete,
            |r| r.refusal = Some(Refusal::ExternalChange),
            Expect::Both,
        ),
        // The child's category and the rest at complete.
        (
            "complete with no publication",
            Complete,
            |r| r.publication = None,
            Expect::Both,
        ),
        (
            "complete with a conflict publication",
            Complete,
            |r| r.publication = Some(PublicationCategory::Conflict),
            Expect::Both,
        ),
        (
            "complete with no_publication_remote",
            Complete,
            |r| r.publication = Some(PublicationCategory::NoPublicationRemote),
            Expect::Both,
        ),
        (
            "context_synchronized with no context",
            ContextSynchronized,
            |r| r.context_oid = None,
            Expect::Rust,
        ),
        (
            "complete with no context",
            Complete,
            |r| r.context_oid = None,
            Expect::Rust,
        ),
        (
            "confirmed with no remote",
            Confirmed,
            |r| r.remote_name = None,
            Expect::Rust,
        ),
        (
            "complete with no remote",
            Complete,
            |r| r.remote_name = None,
            Expect::Rust,
        ),
        // The re-run table's two columns (Cycle 04, RR1 and RR3). Each row
        // is one where only one CHECK can refuse it.
        (
            "an accepted head on a refused record with no begin-time columns",
            Abandoned,
            |r| {
                r.refusal = Some(Refusal::ExternalChange);
                r.answer_code = None;
                bundle(r, false);
                r.accepted_context_oid = Some(oid());
            },
            Expect::Both,
        ),
        (
            "context_pushed on a refused record with no begin-time columns",
            Abandoned,
            |r| {
                r.refusal = Some(Refusal::ExternalChange);
                r.answer_code = None;
                bundle(r, false);
                r.context_pushed = true;
            },
            Expect::Both,
        ),
        (
            "an accepted head on a direct row",
            Confirmed,
            |r| {
                r.integration_mode = Integration::Direct;
                bundle(r, false);
                r.accepted_context_oid = Some(oid());
            },
            Expect::Both,
        ),
        (
            "context_pushed on a direct row",
            Confirmed,
            |r| {
                r.integration_mode = Integration::Direct;
                bundle(r, false);
                r.context_pushed = true;
            },
            Expect::Both,
        ),
        (
            "a zero accepted head",
            Confirmed,
            |r| r.accepted_context_oid = git2::Oid::from_bytes(&[0; 20]).ok(),
            Expect::Both,
        ),
        (
            "both columns on a begun confirmed record",
            Confirmed,
            |r| {
                r.accepted_context_oid =
                    Some(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap());
                r.context_pushed = true;
            },
            Expect::Ok,
        ),
        // A direct row holds none of the six.
        (
            "direct, the four",
            Confirmed,
            |r| r.integration_mode = Integration::Direct,
            Expect::Both,
        ),
        (
            "direct, a consented tip alone",
            Confirmed,
            |r| {
                r.integration_mode = Integration::Direct;
                bundle(r, false);
                r.consented_context_oid = Some(oid());
            },
            Expect::Both,
        ),
        (
            "direct, an answer",
            Abandoned,
            |r| {
                r.integration_mode = Integration::Direct;
                bundle(r, false);
            },
            Expect::Both,
        ),
        (
            "direct, nothing",
            Confirmed,
            |r| {
                r.integration_mode = Integration::Direct;
                bundle(r, false);
            },
            Expect::Ok,
        ),
    ];
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut seen = Vec::new();
    for (label, step, mutate, expect) in cases {
        let mut r = pull_request(step, ITEMS[0]);
        mutate(&mut r);
        let validated = validate(&r);
        tx.execute_batch("SAVEPOINT case_row").unwrap();
        let inserted = insert_row(&tx, &r);
        let read = (inserted.is_ok()).then(|| read_record(&tx, 1, r.operation_id));
        tx.execute_batch("ROLLBACK TO case_row; RELEASE case_row")
            .unwrap();
        match expect {
            Expect::Ok => {
                validated.unwrap_or_else(|e| panic!("{label}: {e:?}"));
                inserted.unwrap_or_else(|e| panic!("{label}: {e:?}"));
                assert_eq!(read.unwrap().unwrap().as_ref(), Some(&r), "{label}");
                if r.integration_mode == Integration::PullRequest {
                    assert_eq!(r.path(), RecordPath::Branch, "{label}");
                    assert_eq!(r.answered(), r.answer_code.is_some(), "{label}");
                    seen.extend(r.answer_code);
                }
            }
            Expect::Rust => {
                assert!(validated.is_err(), "{label}: validate");
                assert!(inserted.is_ok(), "{label}: the SQL accepts it");
                assert!(read.unwrap().is_err(), "{label}: it does not read");
            }
            Expect::Both => {
                assert!(validated.is_err(), "{label}: validate");
                assert!(inserted.is_err(), "{label}: the SQL refuses it");
            }
        }
    }
    // The two columns' own CHECKs, which a typed value cannot reach (M4): a
    // head of 39 characters, one with a non-hex character, and a flag of 2.
    let begun = pull_request(Confirmed, ITEMS[4]);
    tx.execute_batch("SAVEPOINT raw_columns").unwrap();
    insert_row(&tx, &begun).unwrap();
    for (assignment, accepted) in [
        (
            "accepted_context_oid='111111111111111111111111111111111111111'",
            false,
        ),
        (
            "accepted_context_oid='111111111111111111111111111111111111111g'",
            false,
        ),
        (
            "accepted_context_oid='1111111111111111111111111111111111111111'",
            true,
        ),
        ("context_pushed=2", false),
        ("context_pushed=1", true),
    ] {
        tx.execute_batch("SAVEPOINT raw_column").unwrap();
        let result = tx.execute(
            &format!("UPDATE document_promotions SET {assignment} WHERE operation_id=?1"),
            [begun.operation_id.to_string()],
        );
        tx.execute_batch("ROLLBACK TO raw_column; RELEASE raw_column")
            .unwrap();
        assert_eq!(result.is_ok(), accepted, "{assignment}");
    }
    tx.execute_batch("ROLLBACK TO raw_columns; RELEASE raw_columns")
        .unwrap();
    for answer in [
        PromotionAnswer::Promoted,
        PromotionAnswer::AlreadyPromoted,
        PromotionAnswer::PrimaryHolds,
        PromotionAnswer::RemoteBranchDeleted,
        PromotionAnswer::HistoryUnresolved,
        PromotionAnswer::ContextGone,
    ] {
        assert!(seen.contains(&answer), "{answer:?} round-trips");
        assert_eq!(PromotionAnswer::parse(answer.name()).unwrap(), answer);
    }
    assert!(PromotionAnswer::parse("free text").is_err());
    assert!(PromotionSituation::parse("sometimes").is_err());

    // The graph: a pull-request row moves confirmed -> context_synchronized
    // -> complete, or confirmed -> abandoned, and nothing else; a direct
    // row's graph is as it was.
    for from in ALL_STEPS {
        for to in ALL_STEPS {
            let expected = !from.terminal()
                && (from == to
                    || matches!(
                        (from, to),
                        (Confirmed, ContextSynchronized)
                            | (ContextSynchronized, Complete)
                            | (Confirmed, Abandoned)
                    ));
            assert_eq!(
                from.permits(to, true, Integration::PullRequest),
                expected,
                "pull request {from:?} -> {to:?}"
            );
        }
    }
    // `remote_deleting` is a cleanup record's step: neither mode's graph
    // enters it or leaves it.
    for step in ALL_STEPS {
        for mode in [Integration::Direct, Integration::PullRequest] {
            for remote in [true, false] {
                assert!(!step.permits(RemoteDeleting, remote, mode) || step == RemoteDeleting);
                assert!(!RemoteDeleting.permits(step, remote, mode) || step == RemoteDeleting);
            }
        }
    }
    for (from, to, direct) in [
        (Confirmed, ContextSynchronized, true),
        (ContextSynchronized, Integrating, true),
        (ContextSynchronized, Abandoned, true),
        (Confirmed, Abandoned, true),
        (Integrated, Published, true),
        (Confirmed, Integrating, false),
        (Confirmed, Complete, false),
        (ContextSynchronized, Complete, false),
    ] {
        assert_eq!(
            from.permits(to, true, Integration::Direct),
            direct,
            "direct {from:?} -> {to:?}"
        );
    }
    let direct = begin(&tx, OperationId::new());
    assert_eq!(direct.integration_mode, Integration::Direct);
    assert_eq!(direct.path(), RecordPath::Direct);
    assert!(!direct.answered());

    // save_record: the mode, the answer and the four begin-time columns are
    // fixed; the context tip is set once, at context_synchronized.
    let mut stored = pull_request(Confirmed, ITEMS[1]);
    insert_row(&tx, &stored).unwrap();
    let changes: [(&str, Mutate); 6] = [
        ("mode", |r| r.integration_mode = Integration::Direct),
        ("answer", |r| {
            r.answer_code = Some(PromotionAnswer::Promoted)
        }),
        ("consented tip", |r| {
            r.consented_context_oid =
                Some(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap())
        }),
        ("situation", |r| {
            r.situation = Some(PromotionSituation::Again)
        }),
        ("primary branch", |r| {
            r.primary_branch = Some("trunk".into())
        }),
        ("endpoint digest", |r| r.endpoint_digest = Some([8; 32])),
    ];
    for (label, change) in changes {
        let mut r = stored.clone();
        change(&mut r);
        assert_eq!(
            save_record(&tx, 1, &r).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "{label}"
        );
        let mut r = stored.clone();
        change(&mut r);
        r.context_oid = Some(oid());
        r.step = ContextSynchronized;
        assert!(save_record(&tx, 1, &r).is_err(), "{label} with the step");
    }
    assert_eq!(
        read_record(&tx, 1, stored.operation_id).unwrap().unwrap(),
        stored
    );
    for step in [
        Integrating,
        Integrated,
        Published,
        PublicationPending,
        Complete,
    ] {
        let mut r = stored.clone();
        r.step = step;
        r.context_oid = Some(oid());
        assert!(save_record(&tx, 1, &r).is_err(), "{step:?} from confirmed");
    }
    stored.context_oid = Some(oid());
    stored.step = ContextSynchronized;
    save_record(&tx, 1, &stored).unwrap();
    // A save stamps the record's time.
    let saved = read_record(&tx, 1, stored.operation_id).unwrap().unwrap();
    stored.updated_at = saved.updated_at;
    assert_eq!(saved, stored);
    let mut moved = stored.clone();
    moved.context_oid =
        Some(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap());
    assert!(save_record(&tx, 1, &moved).is_err(), "the tip is set once");
    let mut abandoned = stored.clone();
    abandoned.step = Abandoned;
    abandoned.refusal = Some(Refusal::ExternalChange);
    assert!(
        save_record(&tx, 1, &abandoned).is_err(),
        "context_synchronized does not abandon"
    );
    // confirmed -> abandoned as a refusal with the begin-time columns it began with.
    let mut ended = pull_request(Confirmed, ITEMS[2]);
    insert_row(&tx, &ended).unwrap();
    ended.step = Abandoned;
    ended.refusal = Some(Refusal::ExternalChange);
    save_record(&tx, 1, &ended).unwrap();
    let saved = read_record(&tx, 1, ended.operation_id).unwrap().unwrap();
    ended.updated_at = saved.updated_at;
    assert_eq!(saved, ended);
    // A refused record's nulls stay null.
    let mut refused = pull_request(Abandoned, ITEMS[3]);
    refused.answer_code = None;
    refused.refusal = Some(Refusal::ExternalChange);
    bundle(&mut refused, false);
    insert_row(&tx, &refused).unwrap();
    let mut filled = refused.clone();
    bundle(&mut filled, true);
    assert!(save_record(&tx, 1, &filled).is_err());
}

/// The re-run table's two columns are moved only by their writers (Cycle 04
/// RR1, RR3): the head only from the head the record holds, and never once an
/// attempt may have pushed; the flag once, on a record that began and is not
/// finished; and a record snapshot taken before either move overwrites
/// neither.
#[test]
fn rerun_columns_are_moved_only_by_their_writers() {
    use PromotionStep::*;
    let expectation = PromotionExpectation::from_bytes([7; 32]);
    let item = |n: usize| -> ItemId { ITEMS[n].parse().unwrap() };
    let head = |digit: char| git2::Oid::from_str(&digit.to_string().repeat(40)).unwrap();
    let begin_data = || PullRequestBegin {
        consented: oid(),
        situation: PromotionSituation::Again,
        primary_branch: "main".into(),
        endpoint_digest: [9; 32],
    };
    let mut registry = db();
    let tx = registry.transaction().unwrap();
    let read =
        |tx: &rusqlite::Transaction<'_>, op: OperationId| read_record(tx, 1, op).unwrap().unwrap();
    let begun = |tx: &rusqlite::Transaction<'_>, n: usize| {
        let op = OperationId::new();
        begin_pull_request_record(
            tx,
            1,
            op,
            &item(n),
            expectation,
            Some("origin"),
            &begin_data(),
        )
        .unwrap();
        op
    };

    // The head moves from the consented tip, then from that head, in order.
    let op = begun(&tx, 0);
    let before_any_move = read(&tx, op);
    assert_eq!(before_any_move.accepted_context_oid, None);
    accept_context_head(&tx, 1, op, oid(), head('2')).unwrap();
    assert_eq!(read(&tx, op).accepted_context_oid, Some(head('2')));
    accept_context_head(&tx, 1, op, head('2'), head('3')).unwrap();
    assert_eq!(read(&tx, op).accepted_context_oid, Some(head('3')));
    // From any other head it is refused, and the row is unchanged: the
    // consented tip no longer is the head it holds.
    let held = read(&tx, op);
    for from in [oid(), head('2'), head('4')] {
        assert_eq!(
            accept_context_head(&tx, 1, op, from, head('5'))
                .unwrap_err()
                .kind,
            RepositoryErrorKind::RecoveryRequired
        );
        assert_eq!(read(&tx, op), held);
    }
    // A snapshot read before the moves saves with neither column touched.
    save_record(&tx, 1, &before_any_move).unwrap();
    assert_eq!(read(&tx, op).accepted_context_oid, Some(head('3')));
    note_context_pushed(&tx, 1, op).unwrap();
    assert!(read(&tx, op).context_pushed);
    save_record(&tx, 1, &before_any_move).unwrap();
    let after = read(&tx, op);
    assert_eq!(
        (after.accepted_context_oid, after.context_pushed),
        (Some(head('3')), true)
    );
    // After the flag the head no longer moves; the flag stays on a second call.
    assert!(accept_context_head(&tx, 1, op, head('3'), head('6')).is_err());
    assert_eq!(read(&tx, op).accepted_context_oid, Some(head('3')));
    note_context_pushed(&tx, 1, op).unwrap();
    assert!(read(&tx, op).context_pushed);

    // At context_synchronized the head is refused; the flag is still noted.
    let synchronized = pull_request(ContextSynchronized, ITEMS[1]);
    insert_row(&tx, &synchronized).unwrap();
    assert!(accept_context_head(&tx, 1, synchronized.operation_id, oid(), head('2')).is_err());
    note_context_pushed(&tx, 1, synchronized.operation_id).unwrap();
    assert!(read(&tx, synchronized.operation_id).context_pushed);
    // At complete the flag is refused too.
    let complete = pull_request(Complete, ITEMS[2]);
    insert_row(&tx, &complete).unwrap();
    assert!(note_context_pushed(&tx, 1, complete.operation_id).is_err());
    assert!(!read(&tx, complete.operation_id).context_pushed);

    // Both writers refuse a direct row and a refused pull-request row.
    let direct = begin_record(&tx, 1, OperationId::new(), &item(4), expectation, None).unwrap();
    let refused = refuse_record_on(
        &tx,
        1,
        OperationId::new(),
        &item(3),
        expectation,
        Some("origin"),
        &Refusal::ExternalChange,
        Integration::PullRequest,
    )
    .unwrap();
    for other in [direct.operation_id, refused.operation_id] {
        let stored = read(&tx, other);
        assert!(accept_context_head(&tx, 1, other, oid(), head('2')).is_err());
        assert!(note_context_pushed(&tx, 1, other).is_err());
        assert_eq!(read(&tx, other), stored);
    }
}

/// The pull-request begin writes the mode and the four begin-time columns, a
/// refused record takes the confirm's mode with none of them, and the two
/// finishes store the answer, the child's category and what the link record
/// is told, each pairing held by the record layer.
#[test]
fn pull_request_begin_and_finish_write_their_columns() {
    use crate::forge::PullRequestLink;
    use crate::repository::pull_request_links::{link_of, upsert_link};
    use PromotionStep::*;
    let expectation = PromotionExpectation::from_bytes([7; 32]);
    let item = |n: usize| -> ItemId { ITEMS[n].parse().unwrap() };
    let link = |target: &str| PullRequestLink {
        source: format!("manyhands/document/{}", ITEMS[0]),
        target: target.into(),
        forge: None,
        url: None,
    };
    let begin_data = || PullRequestBegin {
        consented: oid(),
        situation: PromotionSituation::Again,
        primary_branch: "main".into(),
        endpoint_digest: [9; 32],
    };

    // The begin.
    let mut registry = db();
    let tx = registry.transaction().unwrap();
    let op = OperationId::new();
    let r = begin_pull_request_record(
        &tx,
        1,
        op,
        &item(0),
        expectation,
        Some("origin"),
        &begin_data(),
    )
    .unwrap();
    assert_eq!(r.step, Confirmed);
    assert_eq!(r.integration_mode, Integration::PullRequest);
    assert_eq!(r.path(), RecordPath::Branch);
    assert_eq!(r.remote_name.as_deref(), Some("origin"));
    assert_eq!(r.consented_context_oid, Some(oid()));
    assert_eq!(r.situation, Some(PromotionSituation::Again));
    assert_eq!(r.primary_branch.as_deref(), Some("main"));
    assert_eq!(r.endpoint_digest, Some([9; 32]));
    assert_eq!(
        (r.answer_code, r.publication, r.refusal.clone()),
        (None, None, None)
    );
    assert_eq!(read_record(&tx, 1, op).unwrap().unwrap(), r);
    // The same operation finds its record, whatever it is asked again.
    let mut other = begin_data();
    other.situation = PromotionSituation::First;
    assert_eq!(
        begin_pull_request_record(&tx, 1, op, &item(0), expectation, Some("origin"), &other)
            .unwrap(),
        r
    );
    // Another operation is held off by the unfinished record, and a pull-request
    // begin has a remote to publish to.
    assert!(
        begin_pull_request_record(
            &tx,
            1,
            OperationId::new(),
            &item(0),
            expectation,
            Some("origin"),
            &begin_data()
        )
        .is_err()
    );
    assert!(
        begin_pull_request_record(
            &tx,
            1,
            OperationId::new(),
            &item(1),
            expectation,
            None,
            &begin_data()
        )
        .is_err()
    );

    // A refused record takes the confirm's mode and none of the four.
    let refused = refuse_record_on(
        &tx,
        1,
        OperationId::new(),
        &item(1),
        expectation,
        Some("origin"),
        &Refusal::ExternalChange,
        Integration::PullRequest,
    )
    .unwrap();
    assert_eq!(refused.integration_mode, Integration::PullRequest);
    assert_eq!(refused.step, Abandoned);
    assert_eq!(
        (
            refused.consented_context_oid,
            refused.situation,
            refused.primary_branch.clone(),
            refused.endpoint_digest
        ),
        (None, None, None, None)
    );
    let direct = refuse_record(
        &tx,
        1,
        OperationId::new(),
        &item(2),
        expectation,
        Some("origin"),
        &Refusal::ExternalChange,
    )
    .unwrap();
    assert_eq!(direct.integration_mode, Integration::Direct);
    drop(tx);
    drop(registry);

    // The finishes: a service-backed registry each, with a record at `step`,
    // its context child as `child` says, and the link row `row` (its
    // `given_at` is 1).
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Child {
        Absent,
        Complete,
        Live,
        Retained,
        Ended,
    }
    let fixture = |step: PromotionStep, child: Child, row: bool| {
        let (data, root, service) = promotion_folder_fixture();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let op = OperationId::new();
        let r = state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)?;
            let mut r = begin_pull_request_record(
                tx,
                id,
                op,
                &item(0),
                expectation,
                Some("origin"),
                &begin_data(),
            )?;
            if child != Child::Absent {
                r = next_attempt(tx, id, op, ChildRole::Context)?;
            }
            if row {
                upsert_link(tx, id, "document", ITEMS[0], &link("main"), 1)
                    .map_err(|_| state::recovery_required())?;
            }
            Ok(r)
        })
        .unwrap();
        if child != Child::Absent {
            let child_id = r.context_sync_id.unwrap();
            let execution = child_execution(&r, ChildRole::Context).unwrap();
            let target = RemoteOperationTarget::for_context(
                &plan,
                RemoteOperationAction::SynchronizeContext,
                AuthoringKind::Document,
                r.item_id.clone(),
            )
            .unwrap();
            assert!(matches!(
                service
                    .reserve_promotion_child(root.path(), child_id, &target, &execution, true)
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
            match child {
                Child::Complete => crate::repository::promotion::run_tests::complete_child_row(
                    &service,
                    root.path(),
                    &execution,
                    child_id,
                    oid(),
                    "published",
                ),
                Child::Ended => state::with_transaction(&service, root.path(), |tx, _| {
                    tx.execute(
                        "UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled' WHERE operation_ulid=?1",
                        [child_id.to_string()],
                    )
                    .map_err(|_| state::recovery_required())?;
                    Ok(())
                })
                .unwrap(),
                // A child stopped mid-integration, as its reservation layer
                // leaves it: an integration step prepared and not applied.
                Child::Retained => state::with_transaction(&service, root.path(), |tx, id| {
                    state::prepare_step_for_test(tx, id, child_id, oid())
                })
                .unwrap(),
                Child::Absent | Child::Live => {}
            }
        }
        if step == ContextSynchronized {
            state::with_transaction(&service, root.path(), |tx, id| {
                let mut r = read_record(tx, id, op)?.unwrap();
                r.context_oid = Some(oid());
                r.step = step;
                save_record(tx, id, &r)
            })
            .unwrap();
        }
        (data, root, service, op)
    };
    let given_at = |tx: &rusqlite::Transaction<'_>, id: i64| -> Option<i64> {
        tx.query_row(
            "SELECT given_at FROM pull_request_links WHERE repository_id=?1 AND kind='document' AND item_ulid=?2",
            params![id, ITEMS[0]],
            |row| row.get(0),
        )
        .ok()
    };
    use PromotionAnswer::*;
    use PublicationCategory as Category;
    type Make = fn(PullRequestLink) -> BranchFinish;
    // (label, step, a link row first, the finish, whether it is accepted,
    // the row's given_at after, the link's target after).
    type Row = (
        &'static str,
        PromotionStep,
        bool,
        Make,
        bool,
        Option<i64>,
        &'static str,
    );
    let complete: Vec<Row> = vec![
        (
            "promoted gives a link",
            ContextSynchronized,
            false,
            |l| BranchFinish {
                answer: Promoted,
                category: Category::Published,
                link: LinkWrite::Give {
                    link: l,
                    given_at: 5,
                },
            },
            true,
            Some(5),
            "other",
        ),
        (
            "promoted over a row replaces it",
            ContextSynchronized,
            true,
            |l| BranchFinish {
                answer: Promoted,
                category: Category::AlreadyCurrent,
                link: LinkWrite::Give {
                    link: l,
                    given_at: 5,
                },
            },
            true,
            Some(5),
            "other",
        ),
        (
            "already_promoted renews given_at alone",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: AlreadyPromoted,
                category: Category::AlreadyCurrent,
                link: LinkWrite::Renew { given_at: 9 },
            },
            true,
            Some(9),
            "main",
        ),
        (
            "already_promoted with no row renews nothing",
            ContextSynchronized,
            false,
            |_| BranchFinish {
                answer: AlreadyPromoted,
                category: Category::Published,
                link: LinkWrite::Renew { given_at: 9 },
            },
            true,
            None,
            "main",
        ),
        (
            "primary_holds leaves the row",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: PrimaryHolds,
                category: Category::Published,
                link: LinkWrite::Untouched,
            },
            true,
            Some(1),
            "main",
        ),
        (
            "primary_holds writes none",
            ContextSynchronized,
            false,
            |_| BranchFinish {
                answer: PrimaryHolds,
                category: Category::Published,
                link: LinkWrite::Untouched,
            },
            true,
            None,
            "main",
        ),
        // RR8: a finish whose context branch is gone writes or renews no row.
        (
            "promoted withheld over a row leaves it",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: Promoted,
                category: Category::Published,
                link: LinkWrite::Withheld,
            },
            true,
            Some(1),
            "main",
        ),
        (
            "promoted withheld writes none",
            ContextSynchronized,
            false,
            |_| BranchFinish {
                answer: Promoted,
                category: Category::Published,
                link: LinkWrite::Withheld,
            },
            true,
            None,
            "main",
        ),
        (
            "already_promoted withheld leaves given_at",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: AlreadyPromoted,
                category: Category::AlreadyCurrent,
                link: LinkWrite::Withheld,
            },
            true,
            Some(1),
            "main",
        ),
        (
            "primary_holds withheld",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: PrimaryHolds,
                category: Category::Published,
                link: LinkWrite::Withheld,
            },
            false,
            Some(1),
            "main",
        ),
        (
            "promoted with a renewal",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: Promoted,
                category: Category::Published,
                link: LinkWrite::Renew { given_at: 9 },
            },
            false,
            Some(1),
            "main",
        ),
        (
            "primary_holds with a link",
            ContextSynchronized,
            true,
            |l| BranchFinish {
                answer: PrimaryHolds,
                category: Category::Published,
                link: LinkWrite::Give {
                    link: l,
                    given_at: 5,
                },
            },
            false,
            Some(1),
            "main",
        ),
        (
            "already_promoted untouched",
            ContextSynchronized,
            true,
            |_| BranchFinish {
                answer: AlreadyPromoted,
                category: Category::Published,
                link: LinkWrite::Untouched,
            },
            false,
            Some(1),
            "main",
        ),
        (
            "a conflict category",
            ContextSynchronized,
            false,
            |l| BranchFinish {
                answer: Promoted,
                category: Category::Conflict,
                link: LinkWrite::Give {
                    link: l,
                    given_at: 5,
                },
            },
            false,
            None,
            "main",
        ),
        (
            "an ended answer",
            ContextSynchronized,
            false,
            |l| BranchFinish {
                answer: RemoteBranchDeleted,
                category: Category::Published,
                link: LinkWrite::Give {
                    link: l,
                    given_at: 5,
                },
            },
            false,
            None,
            "main",
        ),
        (
            "at confirmed",
            Confirmed,
            false,
            |l| BranchFinish {
                answer: Promoted,
                category: Category::Published,
                link: LinkWrite::Give {
                    link: l,
                    given_at: 5,
                },
            },
            false,
            None,
            "main",
        ),
    ];
    for (label, step, row, finish, accepted, given, target) in complete {
        let (_data, root, service, op) = fixture(step, Child::Complete, row);
        state::with_transaction(&service, root.path(), |tx, id| {
            let finish = finish(link("other"));
            let result = complete_branch_record(tx, id, op, &finish);
            assert_eq!(result.is_ok(), accepted, "{label}");
            let r = read_record(tx, id, op)?.unwrap();
            if accepted {
                assert_eq!(r.step, Complete, "{label}");
                assert_eq!(r.answer_code, Some(finish.answer), "{label}");
                assert_eq!(r.publication, Some(finish.category), "{label}");
                assert!(!r.cancel_requested, "{label}");
            } else {
                assert_eq!(r.step, step, "{label}");
                assert_eq!((r.answer_code, r.publication), (None, None), "{label}");
            }
            assert_eq!(given_at(tx, id), given, "{label}");
            let stored = link_of(tx, id, "document", ITEMS[0]).unwrap();
            assert_eq!(
                stored.map(|l| l.target),
                given.map(|_| target.to_owned()),
                "{label}"
            );
            Ok(())
        })
        .unwrap();
    }
    // A record completes only over a context child that completed: none, a
    // live one and one that ended are each refused, and leave the record and
    // the link record as they were.
    for (child, accepted) in [
        (Child::Complete, true),
        (Child::Absent, false),
        (Child::Live, false),
        (Child::Retained, false),
        (Child::Ended, false),
    ] {
        let (_data, root, service, op) = fixture(ContextSynchronized, child, true);
        state::with_transaction(&service, root.path(), |tx, id| {
            let result = complete_branch_record(
                tx,
                id,
                op,
                &BranchFinish {
                    answer: Promoted,
                    category: Category::Published,
                    link: LinkWrite::Give {
                        link: link("other"),
                        given_at: 5,
                    },
                },
            );
            assert_eq!(result.is_ok(), accepted, "complete over {child:?}");
            let r = read_record(tx, id, op)?.unwrap();
            assert_eq!(
                r.step,
                if accepted {
                    Complete
                } else {
                    ContextSynchronized
                },
                "complete over {child:?}"
            );
            assert_eq!(
                given_at(tx, id),
                Some(if accepted { 5 } else { 1 }),
                "complete over {child:?}"
            );
            Ok(())
        })
        .unwrap();
    }
    // The ended finish: from `confirmed` only, with the four answers an ended
    // record has; a deleted branch takes its row, the rest keep it. Its child
    // may be none or ended, and is never one still live.
    let ended: [(PromotionAnswer, bool); 6] = [
        (PrimaryHolds, true),
        (RemoteBranchDeleted, true),
        (HistoryUnresolved, true),
        (ContextGone, true),
        (Promoted, false),
        (AlreadyPromoted, false),
    ];
    for (answer, accepted) in ended {
        let (_data, root, service, op) = fixture(Confirmed, Child::Ended, true);
        state::with_transaction(&service, root.path(), |tx, id| {
            let result = abandon_branch_record(tx, id, op, answer);
            assert_eq!(result.is_ok(), accepted, "{answer:?}");
            let r = read_record(tx, id, op)?.unwrap();
            if accepted {
                assert_eq!(r.step, Abandoned, "{answer:?}");
                assert_eq!(
                    r.refusal,
                    Some(Refusal::ReconfirmationRequired),
                    "{answer:?}"
                );
                assert_eq!(r.answer_code, Some(answer), "{answer:?}");
                assert_eq!(r.publication, None, "{answer:?}");
                assert_eq!(
                    given_at(tx, id),
                    if answer == RemoteBranchDeleted {
                        None
                    } else {
                        Some(1)
                    },
                    "{answer:?}"
                );
            } else {
                assert_eq!(r.step, Confirmed, "{answer:?}");
                assert_eq!(given_at(tx, id), Some(1), "{answer:?}");
            }
            Ok(())
        })
        .unwrap();
    }
    for (child, accepted) in [
        (Child::Absent, true),
        (Child::Ended, true),
        (Child::Live, false),
        (Child::Retained, false),
    ] {
        let (_data, root, service, op) = fixture(Confirmed, child, true);
        state::with_transaction(&service, root.path(), |tx, id| {
            let result = abandon_branch_record(tx, id, op, RemoteBranchDeleted);
            assert_eq!(result.is_ok(), accepted, "abandon over {child:?}");
            let r = read_record(tx, id, op)?.unwrap();
            assert_eq!(
                r.step,
                if accepted { Abandoned } else { Confirmed },
                "abandon over {child:?}"
            );
            assert_eq!(
                given_at(tx, id),
                if accepted { None } else { Some(1) },
                "abandon over {child:?}"
            );
            Ok(())
        })
        .unwrap();
    }
    let (_data, root, service, op) = fixture(ContextSynchronized, Child::Complete, false);
    state::with_transaction(&service, root.path(), |tx, id| {
        assert!(
            abandon_branch_record(tx, id, op, ContextGone).is_err(),
            "not from context_synchronized"
        );
        Ok(())
    })
    .unwrap();
}

/// A cleanup test's row: its label, its step and the columns it sets.
type Case<'a> = (&'a str, &'a str, Vec<(&'static str, String)>);
type Change = dyn Fn(&mut PromotionRecord);
/// The `n`th test document: a valid ULID of its own.
fn doc_n(n: u8) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    format!(
        "01ARZ3NDEKTSV4RRFFQ69G5F{}{}",
        ALPHABET[usize::from(n / 32)] as char,
        ALPHABET[usize::from(n % 32)] as char
    )
}
/// `digit` repeated as a quoted SQL OID literal.
fn quoted(digit: char) -> String {
    format!("'{}'", digit.to_string().repeat(40))
}
/// A row of either action, straight in SQL, returning the schema's answer:
/// the identity columns, with `columns` replacing or adding to them. `NULL`
/// as a value clears an identity column.
fn try_row(
    tx: &Transaction<'_>,
    item: &str,
    step: &str,
    columns: &[(&str, &str)],
) -> rusqlite::Result<OperationId> {
    let op = OperationId::new();
    let mut cells: Vec<(String, String)> = [
        ("repository_id", "1".to_owned()),
        ("operation_id", format!("'{op}'")),
        ("item_id", format!("'{item}'")),
        ("expectation", format!("x'{}'", "07".repeat(32))),
        ("remote_name", "'origin'".to_owned()),
        ("step", format!("'{step}'")),
        ("updated_at", "1".to_owned()),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value))
    .collect();
    for (name, value) in columns {
        match cells.iter().position(|(n, _)| n == name) {
            Some(at) => cells[at].1 = (*value).to_owned(),
            None => cells.push(((*name).to_owned(), (*value).to_owned())),
        }
    }
    let (names, values): (Vec<_>, Vec<_>) = cells.into_iter().unzip();
    tx.execute_batch(&format!(
        "INSERT INTO document_promotions({}) VALUES({})",
        names.join(","),
        values.join(",")
    ))
    .map(|()| op)
}
/// Whether saving `r` is accepted. The save is rolled back.
fn saves(tx: &Transaction<'_>, r: &PromotionRecord) -> bool {
    tx.execute_batch("SAVEPOINT probe").unwrap();
    let saved = save_record(tx, 1, r).is_ok();
    tx.execute_batch("ROLLBACK TO probe; RELEASE probe")
        .unwrap();
    saved
}

/// P7, R-28, R-29, R-33. A registry whose table is today's (`SCHEMA_V3`),
/// holding a row of every step and of both modes, is rebuilt once: every row
/// reads as the promotion it was, byte for byte, with the new columns null or
/// defaulted; a second run changes nothing, and a half-done migration fails
/// closed. A cleanup row then keeps to its CHECKs, its step graph and its
/// immutables, in either mode, and is never superseded.
#[test]
fn cleanup_records_rebuild_once() {
    use crate::repository::promotion::lifecycle::{
        DocumentPromotionLifecycle, Lifecycle, LifecycleRecord,
    };
    let (o1, o2, o3, o5) = (quoted('1'), quoted('2'), quoted('3'), quoted('5'));
    let blob = |byte: &str| format!("x'{}'", byte.repeat(32));
    let mut db = db();
    db.execute_batch(&format!(
        "DROP INDEX document_promotions_unfinished; DROP TABLE document_promotions; {SCHEMA_V3}; {INDEX}; DELETE FROM registry_migrations WHERE name='{CLEANUP_MARKER}';"
    ))
    .unwrap();
    // Direct rows at every step, and the pull-request rows the mode has.
    let synchronized = [("context_oid", &o1[..])];
    let integrating = [
        ("context_oid", &o1[..]),
        ("base_oid", &o2[..]),
        ("integration_kind", "'merged'"),
    ];
    let integrated = [&integrating[..], &[("integration_oid", &o3[..])]].concat();
    let published = [&integrated[..], &[("publication_category", "'published'")]].concat();
    let pending = [&integrated[..], &[("publication_category", "'transport'")]].concat();
    let abandoned = [("refusal_category", "'external_change'")];
    let direct: [(&str, &[(&str, &str)]); 13] = [
        ("confirmed", &[]),
        ("context_synchronized", &synchronized),
        ("integrating", &integrating),
        ("integrated", &integrated),
        ("published", &published),
        ("publication_pending", &pending),
        ("remote_deleted", &published),
        ("worktree_removing", &published),
        ("worktree_removed", &published),
        ("branch_deleted", &published),
        ("complete", &published),
        ("abandoned", &abandoned),
        ("superseded", &[]),
    ];
    let tx = db.transaction().unwrap();
    let mut old: Vec<(OperationId, Integration, PromotionStep)> = Vec::new();
    for (n, (step, columns)) in direct.iter().enumerate() {
        let op = try_row(&tx, &doc_n(n as u8), step, columns).unwrap();
        old.push((op, Integration::Direct, PromotionStep::parse(step).unwrap()));
    }
    for (n, step) in [
        PromotionStep::Confirmed,
        PromotionStep::ContextSynchronized,
        PromotionStep::Complete,
        PromotionStep::Abandoned,
    ]
    .into_iter()
    .enumerate()
    {
        let r = pull_request(step, &doc_n(20 + n as u8));
        insert_row(&tx, &r).unwrap();
        old.push((r.operation_id, Integration::PullRequest, step));
    }
    // A pull-request row that keeps its re-run columns.
    let mut rerun = pull_request(PromotionStep::Confirmed, &doc_n(30));
    rerun.accepted_context_oid = Some(git2::Oid::from_str(&"2".repeat(40)).unwrap());
    rerun.context_pushed = true;
    insert_row(&tx, &rerun).unwrap();
    old.push((
        rerun.operation_id,
        Integration::PullRequest,
        PromotionStep::Confirmed,
    ));
    assert_eq!(old.len(), 18);
    // Every step is held by an old row.
    for step in ALL_STEPS
        .into_iter()
        .filter(|s| *s != PromotionStep::RemoteDeleting)
    {
        assert!(old.iter().any(|(_, _, held)| *held == step), "{step:?}");
    }
    tx.commit().unwrap();
    let before = dump(&db, V3_COLUMNS);
    assert_eq!(before.len(), 18);
    // The columns that carry values survive the rebuild (byte for byte, by
    // the dump below): they are set on the rows that hold them.
    let held = |db: &Connection, column: &str| -> i64 {
        db.query_row(
            &format!("SELECT count(*) FROM document_promotions WHERE {column} IS NOT NULL"),
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(held(&db, "answer_code"), 2);
    assert_eq!(held(&db, "accepted_context_oid"), 1);
    assert_eq!(held(&db, "consented_context_oid"), 5);
    assert_eq!(dump(&db, "*")[0].len(), 29);
    assert_eq!(V3_COLUMNS.split(',').count(), 29);
    // A V3 table has no `cleanup` action, and no `remote_deleting` step.
    assert!(
        db.execute("UPDATE document_promotions SET step='remote_deleting'", [])
            .is_err()
    );
    migrate_registry(&mut db).unwrap();
    // One rebuild: every old row byte for byte, reading as a promotion in its
    // mode and step; the new columns null, and the action `promote`.
    assert_eq!(dump(&db, V3_COLUMNS), before);
    let tx = db.transaction().unwrap();
    for (op, mode, step) in &old {
        let r = read_record(&tx, 1, *op).unwrap().unwrap();
        assert_eq!((r.integration_mode, r.step), (*mode, *step));
        assert_eq!(r.action, PromotionAction::Promote);
        assert_ne!(r.path(), RecordPath::Cleanup);
        assert!(DocumentPromotionLifecycle::continues(&r));
        assert_eq!((r.unheld_digest, r.remote_branch), (None, None));
    }
    tx.rollback().unwrap();
    assert_eq!(marker_count(&db, CLEANUP_MARKER), 1);
    audit(&db).unwrap();
    let changes = db.total_changes();
    migrate_registry(&mut db).unwrap();
    assert_eq!(db.total_changes(), changes, "a second run changes nothing");
    assert_eq!(dump(&db, V3_COLUMNS), before);
    // A half-done migration fails closed: the marker with a table that is not
    // the new one, a table of the rebuild left under its rename's name, and a
    // fault at the marker, which undoes the whole rebuild.
    {
        let mut half = self::db();
        half.execute_batch(&format!(
            "DROP INDEX document_promotions_unfinished; DROP TABLE document_promotions; {SCHEMA_V3}; {INDEX};"
        ))
        .unwrap();
        assert_eq!(
            migrate_registry(&mut half).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "marked, but still the old table"
        );
        half.execute_batch(&format!(
            "DELETE FROM registry_migrations WHERE name='{CLEANUP_MARKER}'; CREATE TABLE document_promotions_before_cleanup(x);"
        ))
        .unwrap();
        assert_eq!(
            migrate_registry(&mut half).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "a leftover table of the rebuild"
        );
        half.execute_batch(&format!(
            "DROP TABLE document_promotions_before_cleanup; CREATE TRIGGER marker_fault BEFORE INSERT ON registry_migrations WHEN NEW.name='{CLEANUP_MARKER}' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;"
        ))
        .unwrap();
        assert!(migrate_registry(&mut half).is_err());
        assert_eq!(marker_count(&half, CLEANUP_MARKER), 0);
        let stored: String = half
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='document_promotions'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, SCHEMA_V3, "still the old table");
        half.execute_batch("DROP TRIGGER marker_fault").unwrap();
        migrate_registry(&mut half).unwrap();
        assert_eq!(marker_count(&half, CLEANUP_MARKER), 1);
    }

    // A cleanup row's CHECKs, in either mode.
    let tx = db.transaction().unwrap();
    let mut n = 100u8;
    let mut next = || {
        n += 1;
        doc_n(n)
    };
    let s = |text: &str| text.to_owned();
    let none = || s("NULL");
    let cleanup = |tx: &Transaction<'_>,
                   item: &str,
                   mode: &str,
                   step: &str,
                   extra: &[(&'static str, String)]|
     -> rusqlite::Result<OperationId> {
        let mut cells: Vec<(&'static str, String)> = vec![
            ("integration_mode", format!("'{mode}'")),
            ("action", s("'cleanup'")),
            ("context_oid", o1.clone()),
            ("unheld_digest", blob("0a")),
            ("primary_branch", s("'main'")),
            ("endpoint_digest", blob("0b")),
        ];
        for (name, value) in extra {
            match cells.iter().position(|(n, _)| n == name) {
                Some(at) => cells[at].1 = value.clone(),
                None => cells.push((name, value.clone())),
            }
        }
        let columns: Vec<(&str, &str)> = cells.iter().map(|(n, v)| (*n, v.as_str())).collect();
        try_row(tx, item, step, &columns)
    };
    let remoteless = [("remote_name", none()), ("endpoint_digest", none())];
    let with_r = [("remote_context_oid", o5.clone())];
    let deleted = [
        ("remote_context_oid", o5.clone()),
        ("remote_branch", s("'deleted'")),
    ];
    for mode in ["direct", "pull_request"] {
        let accepted: Vec<Case<'_>> = vec![
            ("with a remote and R", "confirmed", with_r.to_vec()),
            ("with a remote and no R", "confirmed", vec![]),
            ("without a remote", "confirmed", remoteless.to_vec()),
            ("deleting", "remote_deleting", with_r.to_vec()),
            ("deleted", "remote_deleted", deleted.to_vec()),
            (
                "absent",
                "remote_deleted",
                vec![
                    ("remote_context_oid", o5.clone()),
                    ("remote_branch", s("'absent'")),
                ],
            ),
            (
                "nothing to delete",
                "remote_deleted",
                vec![("remote_branch", s("'none'"))],
            ),
            ("worktree removing", "worktree_removing", deleted.to_vec()),
            ("worktree removed", "worktree_removed", deleted.to_vec()),
            ("branch deleted", "branch_deleted", deleted.to_vec()),
            ("complete", "complete", deleted.to_vec()),
            (
                "no remote at the worktree",
                "worktree_removing",
                [&remoteless[..], &[("remote_branch", s("'none'"))]].concat(),
            ),
            (
                "a stop at the worktree",
                "worktree_removing",
                [
                    &deleted[..],
                    &[("cleanup_category", s("'context_changed'"))],
                ]
                .concat(),
            ),
            (
                "a stop while deleting",
                "remote_deleting",
                [
                    &with_r[..],
                    &[("cleanup_category", s("'remote_deletion_transport'"))],
                ]
                .concat(),
            ),
            (
                "abandoned",
                "abandoned",
                vec![("refusal_category", s("'external_change'"))],
            ),
        ];
        for (label, step, extra) in &accepted {
            let op = cleanup(&tx, &next(), mode, step, extra)
                .unwrap_or_else(|e| panic!("{mode} {label}: {e}"));
            let r = read_record(&tx, 1, op).unwrap().unwrap();
            assert_eq!(r.path(), RecordPath::Cleanup, "{mode} {label}");
            assert_eq!(r.action, PromotionAction::Cleanup, "{mode} {label}");
            assert_eq!(r.integration_mode.as_str(), mode, "{mode} {label}");
            assert_eq!(r.step.name(), *step, "{mode} {label}");
            assert_eq!(r.merged_context(), Some(oid()), "{mode} {label}");
            assert_eq!(r.unheld_digest, Some([0x0a; 32]));
            assert_eq!(r.primary_branch.as_deref(), Some("main"));
            assert_eq!(r.endpoint_digest.is_some(), r.remote_name.is_some());
            assert_eq!(r.consented_context_oid, None);
            assert!(
                !ParentRecord::supersedable(&r),
                "{mode} {label}: never superseded"
            );
            assert!(
                !DocumentPromotionLifecycle::continues(&r),
                "{mode} {label}: a promote does not continue it"
            );
            // A promote that meets the record by its ID and digest is refused.
            assert!(
                DocumentPromotionLifecycle::begin_record(
                    &tx,
                    1,
                    op,
                    &r.item_id,
                    PromotionExpectation::from_bytes([7; 32]),
                    Some("origin"),
                    None,
                )
                .is_err(),
                "{mode} {label}: a promote's begin finds a cleanup record"
            );
            assert_eq!(
                r.remote_deletion_required(),
                r.remote_name.is_some(),
                "{mode} {label}"
            );
        }
        let sync = s("'01ARZ3NDEKTSV4RRFFQ69G5FB0'");
        let refused: Vec<Case<'_>> = vec![
            ("a base", "confirmed", vec![("base_oid", o2.clone())]),
            (
                "an integration",
                "confirmed",
                vec![
                    ("integration_oid", o3.clone()),
                    ("integration_kind", s("'merged'")),
                ],
            ),
            (
                "a publication",
                "confirmed",
                vec![("publication_category", s("'published'"))],
            ),
            (
                "a context attempt",
                "confirmed",
                vec![
                    ("context_attempt", s("1")),
                    ("context_sync_id", sync.clone()),
                ],
            ),
            (
                "a primary attempt",
                "confirmed",
                vec![
                    ("primary_attempt", s("1")),
                    ("primary_sync_id", sync.clone()),
                ],
            ),
            (
                "an answer",
                "complete",
                [&deleted[..], &[("answer_code", s("'promoted'"))]].concat(),
            ),
            (
                "a consented head",
                "confirmed",
                vec![("consented_context_oid", o1.clone())],
            ),
            (
                "a situation",
                "confirmed",
                vec![("situation", s("'first'"))],
            ),
            (
                "an accepted head",
                "confirmed",
                vec![("accepted_context_oid", o1.clone())],
            ),
            (
                "a pushed context",
                "confirmed",
                vec![("context_pushed", s("1"))],
            ),
            ("no L", "confirmed", vec![("context_oid", none())]),
            (
                "no unheld digest",
                "confirmed",
                vec![("unheld_digest", none())],
            ),
            (
                "a short unheld digest",
                "confirmed",
                vec![("unheld_digest", s("x'31'"))],
            ),
            (
                "no primary branch",
                "confirmed",
                vec![("primary_branch", none())],
            ),
            (
                "an empty primary branch",
                "confirmed",
                vec![("primary_branch", s("''"))],
            ),
            (
                "a remote and no endpoint",
                "confirmed",
                vec![("endpoint_digest", none())],
            ),
            (
                "an endpoint and no remote",
                "confirmed",
                vec![("remote_name", none())],
            ),
            ("context_synchronized", "context_synchronized", vec![]),
            ("integrating", "integrating", vec![]),
            ("integrated", "integrated", vec![]),
            ("published", "published", vec![]),
            ("publication_pending", "publication_pending", vec![]),
            ("superseded", "superseded", vec![]),
            (
                "an outcome at confirmed",
                "confirmed",
                vec![("remote_branch", s("'none'"))],
            ),
            (
                "an outcome while deleting",
                "remote_deleting",
                [&with_r[..], &[("remote_branch", s("'deleted'"))]].concat(),
            ),
            ("no outcome once deleted", "remote_deleted", with_r.to_vec()),
            (
                "no outcome at the worktree",
                "worktree_removing",
                with_r.to_vec(),
            ),
            ("no outcome at complete", "complete", with_r.to_vec()),
            (
                "none with an R",
                "remote_deleted",
                vec![
                    ("remote_context_oid", o5.clone()),
                    ("remote_branch", s("'none'")),
                ],
            ),
            (
                "deleted with no R",
                "remote_deleted",
                vec![("remote_branch", s("'deleted'"))],
            ),
            (
                "an unknown outcome",
                "remote_deleted",
                vec![("remote_branch", s("'gone'"))],
            ),
            ("deleting with no R", "remote_deleting", vec![]),
            (
                "a blocked refusal",
                "abandoned",
                vec![
                    ("refusal_category", s("'blocked'")),
                    ("refusal_blockers", s("1")),
                ],
            ),
            (
                "a reconfirmation refusal",
                "abandoned",
                vec![("refusal_category", s("'reconfirmation_required'"))],
            ),
            (
                "a refusal on a live record",
                "confirmed",
                vec![("refusal_category", s("'external_change'"))],
            ),
        ];
        // A table reports every row it gets wrong.
        let accepted_wrongly: Vec<&str> = refused
            .iter()
            .filter(|(_, step, extra)| cleanup(&tx, &next(), mode, step, extra).is_ok())
            .map(|(label, _, _)| *label)
            .collect();
        assert!(
            accepted_wrongly.is_empty(),
            "{mode}: the schema accepts {accepted_wrongly:?}"
        );
    }
    // Only a cleanup row has them: a promote refuses the columns and the step.
    for (step, mode, extra) in [
        ("confirmed", "direct", ("unheld_digest", blob("0a"))),
        ("confirmed", "direct", ("remote_branch", s("'none'"))),
        (
            "remote_deleting",
            "direct",
            ("remote_context_oid", o5.clone()),
        ),
        (
            "remote_deleting",
            "pull_request",
            ("remote_context_oid", o5.clone()),
        ),
    ] {
        let mode = format!("'{mode}'");
        let columns = [
            ("integration_mode", mode.as_str()),
            (extra.0, extra.1.as_str()),
        ];
        assert!(
            try_row(&tx, &next(), step, &columns).is_err(),
            "a promote at {step} with {}",
            extra.0
        );
    }
    // The step graph, as the closure's.
    let steps = [
        "confirmed",
        "context_synchronized",
        "integrating",
        "integrated",
        "published",
        "publication_pending",
        "remote_deleting",
        "remote_deleted",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
        "complete",
        "abandoned",
        "superseded",
    ];
    let mut wrong: Vec<String> = Vec::new();
    for remote in [true, false] {
        let graph: &[(&str, &str)] = if remote {
            &[
                ("confirmed", "remote_deleting"),
                ("confirmed", "remote_deleted"),
                ("remote_deleting", "remote_deleted"),
                ("remote_deleted", "worktree_removing"),
                ("worktree_removing", "worktree_removed"),
                ("worktree_removed", "branch_deleted"),
                ("confirmed", "abandoned"),
                ("remote_deleting", "abandoned"),
            ]
        } else {
            &[
                ("confirmed", "worktree_removing"),
                ("worktree_removing", "worktree_removed"),
                ("worktree_removed", "branch_deleted"),
                ("confirmed", "abandoned"),
            ]
        };
        for from in steps {
            for to in steps {
                let (a, b) = (
                    PromotionStep::parse(from).unwrap(),
                    PromotionStep::parse(to).unwrap(),
                );
                let expected = !a.terminal() && (from == to || graph.contains(&(from, to)));
                if a.cleanup_permits(b, remote) != expected {
                    wrong.push(format!(
                        "{from} -> {to}, remote {remote}: expected {expected}"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "the graph differs at {wrong:#?}");
    // The immutables: a save changes the step and the outcome, nothing else.
    let op = cleanup(&tx, &next(), "pull_request", "confirmed", &with_r).unwrap();
    let mut r = read_record(&tx, 1, op).unwrap().unwrap();
    let moved = |r: &PromotionRecord, change: &dyn Fn(&mut PromotionRecord)| {
        let mut r = r.clone();
        change(&mut r);
        r
    };
    assert!(saves(&tx, &r), "unchanged");
    let changes: [(&str, &Change); 8] = [
        ("L", &|r| {
            r.context_oid = Some(git2::Oid::from_str(&"9".repeat(40)).unwrap())
        }),
        ("R", &|r| r.remote_context_oid = None),
        ("the unheld digest", &|r| r.unheld_digest = Some([1; 32])),
        ("the primary branch", &|r| {
            r.primary_branch = Some("dev".into())
        }),
        ("the endpoint digest", &|r| {
            r.endpoint_digest = Some([1; 32])
        }),
        ("the action", &|r| r.action = PromotionAction::Promote),
        ("the mode", &|r| r.integration_mode = Integration::Direct),
        ("the consented head", &|r| {
            r.consented_context_oid = Some(oid())
        }),
    ];
    for (label, change) in changes {
        assert!(!saves(&tx, &moved(&r, change)), "{label} is immutable");
    }
    let to = |r: &PromotionRecord, step, outcome| {
        moved(r, &|r| {
            r.step = step;
            r.remote_branch = outcome;
        })
    };
    use PromotionStep::*;
    assert!(
        !saves(&tx, &to(&r, WorktreeRemoving, None)),
        "not past the delete"
    );
    assert!(
        !saves(
            &tx,
            &to(&r, RemoteDeleting, Some(RemoteBranchOutcome::Deleted))
        ),
        "no outcome before the delete is known"
    );
    assert!(
        !saves(&tx, &to(&r, RemoteDeleted, None)),
        "an outcome with the delete"
    );
    assert!(
        !saves(
            &tx,
            &to(&r, RemoteDeleted, Some(RemoteBranchOutcome::Unneeded))
        ),
        "none says there was no R"
    );
    assert!(
        !saves(&tx, &to(&r, Complete, Some(RemoteBranchOutcome::Deleted))),
        "complete is its own write"
    );
    assert!(!saves(&tx, &to(&r, Superseded, None)), "never superseded");
    assert!(
        !saves(
            &tx,
            &moved(&r, &|r| r.refusal = Some(Refusal::ExternalChange))
        ),
        "a refusal is for the abandoned"
    );
    r = to(&r, RemoteDeleting, None);
    save_record(&tx, 1, &r).unwrap();
    assert!(!saves(&tx, &to(&r, Confirmed, None)), "no step back");
    assert!(
        !saves(
            &tx,
            &to(&r, WorktreeRemoving, Some(RemoteBranchOutcome::Deleted))
        ),
        "the delete first"
    );
    assert!(
        !saves(
            &tx,
            &moved(&to(&r, Abandoned, None), &|r| r.refusal =
                Some(Refusal::Blocked(vec![PromotionBlocker::ContextDirty])))
        ),
        "only an external change ends a cleanup"
    );
    r = to(&r, RemoteDeleted, Some(RemoteBranchOutcome::Absent));
    save_record(&tx, 1, &r).unwrap();
    assert!(
        !saves(
            &tx,
            &moved(&r, &|r| r.remote_branch =
                Some(RemoteBranchOutcome::Deleted))
        ),
        "the outcome is stored once"
    );
    assert!(!saves(&tx, &moved(&r, &|r| r.remote_branch = None)));
    for step in [WorktreeRemoving, WorktreeRemoved, BranchDeleted] {
        r = to(&r, step, r.remote_branch);
        save_record(&tx, 1, &r).unwrap();
    }
    assert_eq!(
        read_record(&tx, 1, op).unwrap().unwrap().remote_branch,
        Some(RemoteBranchOutcome::Absent)
    );
    // With no remote the outcome is `none`, written with the step to the worktree.
    let op = cleanup(&tx, &next(), "direct", "confirmed", &remoteless).unwrap();
    let r = read_record(&tx, 1, op).unwrap().unwrap();
    assert!(
        !saves(&tx, &to(&r, RemoteDeleting, None)),
        "no delete to make"
    );
    assert!(
        !saves(&tx, &to(&r, WorktreeRemoving, None)),
        "the outcome goes with the step"
    );
    assert!(
        !saves(
            &tx,
            &to(&r, WorktreeRemoving, Some(RemoteBranchOutcome::Deleted))
        ),
        "none says there was no R"
    );
    save_record(
        &tx,
        1,
        &to(&r, WorktreeRemoving, Some(RemoteBranchOutcome::Unneeded)),
    )
    .unwrap();
    // Abandoned from `confirmed`, with the external change.
    let op = cleanup(&tx, &next(), "direct", "confirmed", &[]).unwrap();
    let r = read_record(&tx, 1, op).unwrap().unwrap();
    let ended = moved(&to(&r, Abandoned, None), &|r| {
        r.refusal = Some(Refusal::ExternalChange)
    });
    save_record(&tx, 1, &ended).unwrap();
    assert!(read_record(&tx, 1, op).unwrap().unwrap().step.terminal());
}
