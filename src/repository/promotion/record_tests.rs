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
            state::configure(tx, id, Some(&plan), false)?;
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
                ""
            )
            .is_err()
        );
        assert!(
            recovery::begin_or_reconcile_operation(
                &mut db,
                root.path(),
                RepositoryOperation::RefreshRepository,
                child,
                "other"
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
        "refusal_blockers=65536",
        "expectation=x'01'",
        "context_attempt=-1",
    ] {
        assert!(
            tx.execute_batch(&format!("UPDATE document_promotions SET {assignment}"))
                .is_err(),
            "{assignment}"
        );
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
                ""
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
            false,
        )
    })
    .unwrap();
    (data, root, service)
}
#[test]
fn promotion_status_reports_parent_step_once_and_never_offers_boundary_resume() {
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
    assert!(shown.next_action.is_none());
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
        state::configure(tx, id, Some(&plan), false)
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
        state::configure(tx, id, Some(&plan), false)
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
        state::configure(tx,id,Some(&plan),false)?;
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
            state::configure(tx, id, plan.as_ref(), false)?;
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
