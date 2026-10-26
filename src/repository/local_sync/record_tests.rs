use super::record;
use crate::canonical::ItemId;
use crate::repository::*;
use std::{fs, path::Path};

pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub data: tempfile::TempDir,
    pub service: RepositoryService,
    pub repo: ResolvedRepository,
    pub kind: AuthoringKind,
    pub item: ItemId,
}

pub(super) fn commit(repo: &git2::Repository) -> git2::Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

impl Fixture {
    pub fn new(kind: AuthoringKind) -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let git = git2::Repository::init(root.path()).unwrap();
        git.set_head("refs/heads/main").unwrap();
        let mut config = git.config().unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        fs::write(root.path().join("ordinary"), "base\n").unwrap();
        commit(&git);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let item = ItemId::generate();
        let target = AuthoringTarget {
            root: root.path().into(),
            kind,
            item_id: item.clone(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        };
        match kind {
            AuthoringKind::Document => {
                service
                    .save_document(SaveDocumentRequest {
                        target,
                        source_path: None,
                        destination_path: "docs/local.md".into(),
                        draft: DocumentDraft {
                            title: "Local".into(),
                            body: "body\n".into(),
                        },
                        expected_source: None,
                        expected_destination: ExpectedPathObservation::Missing,
                    })
                    .unwrap();
            }
            AuthoringKind::Ticket => {
                service
                    .save_ticket(SaveTicketRequest {
                        target,
                        draft: TicketDraft {
                            title: "Local".into(),
                            ticket_type: "task".into(),
                            status: "open".into(),
                            project: None,
                            team: None,
                            body: "body\n".into(),
                        },
                        expected_path: ExpectedPathObservation::Missing,
                    })
                    .unwrap();
            }
        }
        let repo = service.resolve_repository(root.path()).unwrap();
        Self {
            root,
            data,
            service,
            repo,
            kind,
            item,
        }
    }
    pub fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    pub fn context(&self) -> git2::Repository {
        git2::Repository::open(
            self.root
                .path()
                .join(".manyhands/worktrees")
                .join(self.item.to_string()),
        )
        .unwrap()
    }
    pub fn observe(&self) -> LocalContextSyncObservationDto {
        self.service
            .observe_local_context_sync(&self.repo, self.kind, &self.item)
            .unwrap()
    }
    pub fn begin(&self, id: OperationId) -> Result<LocalContextSyncRecord, ReadError> {
        let observation = self.observe();
        self.service.begin_local_context_sync(
            &self.repo,
            id,
            self.kind,
            &self.item,
            &observation.observation,
        )
    }
    pub fn image(&self) -> (git2::Oid, Vec<u8>, Vec<u8>) {
        let context = self.context();
        (
            context.head().unwrap().target().unwrap(),
            fs::read(context.path().join("index")).unwrap(),
            fs::read(context.workdir().unwrap().join("ordinary")).unwrap(),
        )
    }
}

#[test]
fn atomic_parent_and_companion_creation_and_reopen() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let id = OperationId::new();
        let before = f.image();
        let row = f.begin(id).unwrap();
        assert_eq!(row.operation_id, id);
        assert_eq!(row.owner_epoch, 1);
        assert_eq!(row.semantic_revision, 1);
        assert_eq!(row.generation, 1);
        assert_eq!(before, f.image());
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        let read = reopened
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        assert_eq!(row, read);
        let db = f.db();
        assert_eq!(db.query_row("SELECT count(*) FROM operation_records o JOIN local_context_sync_records s ON s.local_record_id=o.id WHERE o.operation_ulid=?1 AND o.action='context_sync_local'", [id.to_string()], |r| r.get::<_, i64>(0)).unwrap(), 1);
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

#[test]
fn failed_companion_insert_rolls_back_envelope() {
    let f = Fixture::new(AuthoringKind::Ticket);
    f.db().execute_batch("CREATE TRIGGER fail_local_companion BEFORE INSERT ON local_context_sync_records BEGIN SELECT RAISE(ABORT,'fixture'); END").unwrap();
    let id = OperationId::new();
    let image = f.image();
    assert!(f.begin(id).is_err());
    assert_eq!(image, f.image());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn missing_and_malformed_companion_are_not_reconstructed() {
    for change in [
        "DELETE FROM local_context_sync_records",
        "PRAGMA ignore_check_constraints=ON; UPDATE local_context_sync_records SET format_version=99",
        "ALTER TABLE local_context_sync_records ADD COLUMN surprise TEXT",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let id = OperationId::new();
        f.begin(id).unwrap();
        let before = f.image();
        f.db().execute_batch(change).unwrap();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap_err()
                .code(),
            crate::results::ResultCode::RecoveryRequired
        );
        assert!(RepositoryService::open_at(f.data.path()).is_err());
        assert_eq!(before, f.image());
    }
}

#[test]
fn journal_directory_has_no_unjournaled_fallback() {
    let f = Fixture::new(AuthoringKind::Document);
    let observation = f.observe();
    let before = f.image();
    let registry = f.data.path().join(REGISTRY_FILE);
    fs::remove_file(&registry).unwrap();
    fs::create_dir(&registry).unwrap();
    let result = f.service.begin_local_context_sync(
        &f.repo,
        OperationId::new(),
        f.kind,
        &f.item,
        &observation.observation,
    );
    assert_eq!(
        result.unwrap_err().code(),
        crate::results::ResultCode::IndexUnavailable
    );
    assert_eq!(before, f.image());
    assert!(registry.is_dir());
}

#[test]
fn journal_permission_failure_precedes_git_effects() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let before = f.image();
    let id = OperationId::new();
    record::WRITE_FAULT.with(|fault| fault.set(Some(std::io::ErrorKind::PermissionDenied)));
    let result = f.begin(id);
    assert_eq!(
        result.unwrap_err().code(),
        crate::results::ResultCode::IndexUnavailable
    );
    assert_eq!(before, f.image());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn unfinished_local_cannot_become_an_ordinary_refresh_handoff() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let id = OperationId::new();
    f.begin(id).unwrap();
    let row = recovery::local_row(&f.db(), f.root.path().to_str().unwrap(), id)
        .unwrap()
        .unwrap();
    assert!(!row.owes_only_its_hand_off());
    assert!(
        recovery::require_refresh_can_begin(
            &f.db(),
            f.root.path(),
            RepositoryOperation::RefreshRepository,
            id
        )
        .is_err()
    );
    assert!(
        f.service
            .remove_registration(RemoveRegistrationRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new()
            })
            .is_err()
    );
    assert!(Path::new(f.context().workdir().unwrap()).exists());
}

#[test]
fn local_action_and_prepared_step_have_exact_store_and_read_names() {
    assert_eq!(
        read::local_action("context_sync_local").map(|a| a.as_str()),
        Some("context_sync_local")
    );
    let step = recovery::LocalStep::parse("local_sync_prepared").unwrap();
    assert!(!step.is_checkpoint());
}

#[test]
fn additive_migration_does_not_repair_a_missing_migrated_table() {
    let f = Fixture::new(AuthoringKind::Ticket);
    f.db().execute_batch("DROP TABLE local_context_sync_resolutions; DROP TABLE local_context_sync_steps; DROP TABLE local_context_sync_records; DELETE FROM registry_migrations WHERE name='local_context_sync_v1'").unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        f.observe(),
        reopened
            .observe_local_context_sync(&f.repo, f.kind, &f.item)
            .unwrap()
    );
    f.db()
        .execute_batch("DROP TABLE local_context_sync_steps")
        .unwrap();
    assert!(RepositoryService::open_at(f.data.path()).is_err());
}

#[test]
fn semantic_revision_changes_but_owner_takeover_does_not_invalidate_observation() {
    let f = Fixture::new(AuthoringKind::Ticket);
    f.begin(OperationId::new()).unwrap();
    let old = f.observe();
    let before = f.image();
    f.db()
        .execute(
            "UPDATE local_context_sync_records SET owner_epoch=owner_epoch+1",
            [],
        )
        .unwrap();
    assert_eq!(old, f.observe());
    let error = super::observe::observe_with(&f.service, &f.repo, f.kind, &f.item, || {
        f.db()
            .execute(
                "UPDATE local_context_sync_records SET semantic_revision=semantic_revision+1",
                [],
            )
            .unwrap();
    })
    .unwrap_err();
    assert_eq!(error.code(), crate::results::ResultCode::ExternalChange);
    assert_ne!(old.observation, f.observe().observation);
    assert_eq!(before, f.image());
}

#[test]
fn operation_id_collides_with_every_journal_and_child_namespace() {
    use crate::results::ResultCode;
    use rusqlite::params;
    for family in [
        "local",
        "key_material",
        "key_registry",
        "promotion",
        "promotion_context",
        "promotion_primary",
        "closure",
        "closure_context",
        "closure_primary",
        "comment",
        "comment_child",
        "remote",
        "remote_resolution",
        "remote_identity",
        "local_resolution",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let id = OperationId::new();
        let other = OperationId::new();
        let db = f.db();
        match family {
            "local" => {
                db.execute("UPDATE operation_records SET operation_ulid=?1 WHERE id=(SELECT min(id) FROM operation_records)",[id.to_string()]).unwrap();
            }
            "key_material" => {
                db.execute("INSERT INTO key_material_operations(operation_id,key_id,action,generation_label,private_key_path,public_key_path,phase) VALUES(?1,?2,'generate','fixture','/fixture/private','/fixture/public','reserved')",params![id.to_string(),other.to_string()]).unwrap();
            }
            "key_registry" => {
                db.execute("INSERT INTO key_registry_changes(operation_id,action,changed,recorded_at) VALUES(?1,'clear',0,1)",[id.to_string()]).unwrap();
            }
            "promotion" | "promotion_context" | "promotion_primary" | "closure"
            | "closure_context" | "closure_primary" => {
                let table = if family.starts_with("promotion") {
                    "document_promotions"
                } else {
                    "ticket_closures"
                };
                let suffix = if table == "ticket_closures" {
                    ",closure_kind"
                } else {
                    ""
                };
                let value = if table == "ticket_closures" {
                    ",'record'"
                } else {
                    ""
                };
                let child = if family.ends_with("context") {
                    Some("context")
                } else if family.ends_with("primary") {
                    Some("primary")
                } else {
                    None
                };
                let op = if child.is_some() { other } else { id };
                db.execute(&format!("INSERT INTO {table}(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at{suffix}) SELECT id,?1,?2,zeroblob(32),'origin','confirmed',1{value} FROM repositories"),params![op.to_string(),f.item.to_string()]).unwrap();
                if let Some(role) = child {
                    db.execute(
                        &format!("UPDATE {table} SET {role}_attempt=1,{role}_sync_id=?1"),
                        [id.to_string()],
                    )
                    .unwrap();
                }
            }
            "comment" | "comment_child" => {
                let parent = if family == "comment" { id } else { other };
                let child = if family == "comment_child" { id } else { other };
                let comment = ItemId::generate();
                let target = format!(
                    "authoring-context-v1/ticket/{}/Edit\0.manyhands/comments\0{comment}",
                    f.item
                );
                db.execute("INSERT INTO operation_records(root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,'submit_comment',?3,'completed',1)",params![f.root.path().to_str(),parent.to_string(),target]).unwrap();
                db.execute("INSERT INTO comment_publication_bindings(operation_ulid,local_record_id,root_digest,kind,item_ulid,comment_ulid,synchronization_ulid,pre_checkpoint_oid) VALUES(?1,?2,?3,'ticket',?4,?5,?6,?7)",params![parent.to_string(),db.last_insert_rowid(),blake3::hash(f.root.path().to_str().unwrap().as_bytes()).as_bytes().as_slice(),f.item.to_string(),comment.to_string(),child.to_string(),f.context().head().unwrap().target().unwrap().to_string()]).unwrap();
            }
            "local_resolution" => {
                let parent = f.begin(other).unwrap();
                db.execute("INSERT INTO local_context_sync_resolutions(attempt_ulid,operation_ulid,request_ulid,generation,owner_epoch,observation,input_digest,state) VALUES(?1,?2,?3,1,1,?4,zeroblob(32),'prepared')",params![id.to_string(),parent.operation_id.to_string(),OperationId::new().to_string(),parent.observation]).unwrap();
            }
            _ => {
                let parent = if family == "remote" { id } else { other };
                db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,local_branch,action,priority,phase,created_at,updated_at) SELECT id,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','main','synchronize_primary','manual','interrupted',1,1 FROM repositories",[parent.to_string()]).unwrap();
                let remote_id = db.last_insert_rowid();
                if family == "remote_identity" {
                    db.execute("INSERT INTO remote_identity_confirmations(confirmation_ulid,operation_record_id,configuration_generation,owner_epoch,input_digest,configuration_digest,phase) VALUES(?1,?2,0,0,zeroblob(32),zeroblob(32),'prepared')",params![id.to_string(),remote_id]).unwrap();
                } else if family == "remote_resolution" {
                    db.execute("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind) VALUES(?1,0,0,0,'legacy')",[remote_id]).unwrap();
                    db.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase,window_number) VALUES(?1,0,0,0,'primary',?2,?2,?2,zeroblob(32),'prepared',0)",params![remote_id,f.context().head().unwrap().target().unwrap().to_string()]).unwrap();
                    db.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,phase,commit_time,commit_offset_minutes) VALUES(?1,?2,?3,0,0,zeroblob(32),zeroblob(32),zeroblob(32),'prepared',1,0)",params![id.to_string(),remote_id,db.last_insert_rowid()]).unwrap();
                }
            }
        }
        let before = f.image();
        let result = f.begin(id);
        assert_eq!(
            result.unwrap_err().code(),
            ResultCode::RequestMismatch,
            "{family}"
        );
        assert_eq!(before, f.image(), "{family}");
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM local_context_sync_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn companion_audit_rejects_invalid_evidence_not_only_column_shape() {
    for assignment in [
        "candidate_oid='bad'",
        "active_resolution='bad'",
        "cancel_acknowledged=2",
        "integration_proven=7",
        "generation=0",
        "owner_epoch=0",
        "semantic_revision=0",
        "state='unknown'",
        "index_complete=1",
        "artifacts_released=1",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let id = OperationId::new();
        f.begin(id).unwrap();
        f.db().execute_batch(&format!("PRAGMA ignore_check_constraints=ON; UPDATE local_context_sync_records SET {assignment}")).unwrap();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap_err()
                .code(),
            crate::results::ResultCode::RecoveryRequired,
            "{assignment}"
        );
    }
}

#[test]
fn partial_unmarked_schema_is_not_silently_completed() {
    let f = Fixture::new(AuthoringKind::Ticket);
    f.db().execute_batch("DELETE FROM registry_migrations WHERE name='local_context_sync_v1'; DROP TABLE local_context_sync_steps").unwrap();
    assert!(RepositoryService::open_at(f.data.path()).is_err());
}

#[test]
fn journal_owner_takeover_requires_exact_epoch_and_generation() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let first = f.begin(OperationId::new()).unwrap();
    let observation = f.observe();
    let next = f
        .service
        .take_over_local_context_sync(&f.repo, &first)
        .unwrap();
    assert_eq!(next.owner_epoch, first.owner_epoch + 1);
    assert_eq!(next.semantic_revision, first.semantic_revision);
    assert_eq!(observation, f.observe());
    assert_eq!(
        f.service
            .take_over_local_context_sync(&f.repo, &first)
            .unwrap_err()
            .code(),
        crate::results::ResultCode::ExternalChange
    );
    assert_eq!(
        next,
        f.service
            .read_local_context_sync_record(&f.repo, next.operation_id)
            .unwrap()
    );
}

#[test]
fn enumeration_and_rebuild_preserve_local_authority() {
    let f = Fixture::new(AuthoringKind::Document);
    let row = f.begin(OperationId::new()).unwrap();
    let operation = f.service.show_operation(&f.repo, row.operation_id).unwrap();
    assert_eq!(operation.family, OperationFamily::Local);
    assert_eq!(operation.action, OperationAction::ContextSyncLocal);
    assert_eq!(operation.item_id, Some(f.item.to_string()));
    assert_eq!(f.service.list_operations(&f.repo).unwrap().items.len(), 1);
    assert!(
        f.service
            .rebuild_repository(RebuildRepositoryRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new()
            })
            .is_err()
    );
    assert_eq!(
        row,
        f.service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap()
    );
}

#[test]
fn generic_abandon_cannot_split_local_envelope_and_companion_authority() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    let result = f.service.abandon_operation(AbandonOperationRequest {
        root: f.root.path().into(),
        operation_id: row.operation_id,
        expected: None,
    });
    assert!(matches!(
        result,
        Ok(AbandonOperationOutcome::AnotherJournal)
    ));
    assert_eq!(
        row,
        f.service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap()
    );
}

#[test]
fn admission_rechecks_authority_in_its_creation_transaction() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let id = OperationId::new();
    let observation = f.observe();
    let before = f.image();
    let result = f.service.begin_local_context_sync_with(
        &f.repo,
        id,
        f.kind,
        &f.item,
        &observation.observation,
        || {
            f.db()
                .execute("UPDATE repositories SET enabled_at=enabled_at+1", [])
                .unwrap();
        },
    );
    assert_eq!(
        result.unwrap_err().code(),
        crate::results::ResultCode::ExternalChange
    );
    assert_eq!(before, f.image());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn accepted_journal_reentry_reads_frozen_parents_before_live_observation() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let original = f.observe();
    let row = f.begin(OperationId::new()).unwrap();
    let primary = git2::Repository::open(f.root.path()).unwrap();
    fs::write(f.root.path().join("ordinary"), "new primary").unwrap();
    commit(&primary);
    let retry = f
        .service
        .begin_local_context_sync(
            &f.repo,
            row.operation_id,
            f.kind,
            &f.item,
            &original.observation,
        )
        .unwrap();
    assert_eq!(row, retry);
    assert_eq!(
        f.service
            .begin_local_context_sync(
                &f.repo,
                row.operation_id,
                f.kind,
                &f.item,
                &f.observe().observation
            )
            .unwrap_err()
            .code(),
        crate::results::ResultCode::RequestMismatch
    );
}

#[test]
fn companion_audit_requires_marker_and_foreign_key_integrity() {
    for corruption in [
        "DELETE FROM registry_migrations WHERE name='local_context_sync_v1'",
        "INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest) VALUES('01ARZ3NDEKTSV4RRFFQ69G5FAV',0,1,1,'checkout','prepared',zeroblob(32),zeroblob(32))",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        let db = f.db();
        db.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        db.execute_batch(corruption).unwrap();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap_err()
                .code(),
            crate::results::ResultCode::RecoveryRequired
        );
    }
}

#[test]
fn refused_epoch_update_cannot_return_a_fabricated_owner() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    f.db().execute_batch("CREATE TRIGGER refuse_owner BEFORE UPDATE OF owner_epoch ON local_context_sync_records BEGIN SELECT RAISE(IGNORE); END").unwrap();
    assert!(
        f.service
            .take_over_local_context_sync(&f.repo, &row)
            .is_err()
    );
    assert_eq!(
        row,
        f.service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap()
    );
}

#[test]
fn accepted_reentry_cannot_borrow_a_different_registered_root() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let observation = f.observe();
    let row = f.begin(OperationId::new()).unwrap();
    let other = tempfile::tempdir().unwrap();
    let db = f.db();
    db.execute(
        "UPDATE repositories SET root_path=?1",
        [other.path().to_str()],
    )
    .unwrap();
    db.execute(
        "UPDATE operation_records SET root_path=?1",
        [other.path().to_str()],
    )
    .unwrap();
    assert_eq!(
        f.service
            .begin_local_context_sync(
                &f.repo,
                row.operation_id,
                f.kind,
                &f.item,
                &observation.observation
            )
            .unwrap_err()
            .code(),
        crate::results::ResultCode::RepositoryNotRegistered
    );
}
