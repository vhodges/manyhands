//! Regressions for Task 4 SPEC ses_e898c0189ffebSYYicNzmsZaXQ.
use super::record_tests::Fixture;
use crate::{repository::*, results::ResultCode};
use rusqlite::params;

#[test]
fn public_create_folder_refuses_missing_companion_before_any_effect() {
    let f = Fixture::new(AuthoringKind::Document);
    let row = f.begin(OperationId::new()).unwrap();
    let db = f.db();
    db.execute("DELETE FROM local_context_sync_records", [])
        .unwrap();
    let registration = || {
        db.query_row(
            "SELECT enabled_at,refresh_required,refreshed_at FROM repositories",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .unwrap()
    };
    let envelope = || {
        db.query_row("SELECT state,completed_step,index_owner_epoch,observed_at FROM operation_records WHERE operation_ulid=?1", [row.operation_id.to_string()], |r| Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?))).unwrap()
    };
    let before = (registration(), envelope(), f.image());
    let result = f.service.create_folder(CreateFolderRequest {
        root: f.root.path().into(),
        path: DocumentFolderPath::parse("docs/spec/no-effect").unwrap(),
        operation_id: row.operation_id,
    });
    assert!(
        !f.root.path().join("docs/spec").exists(),
        "refusal must precede directory creation"
    );
    assert_eq!(before, (registration(), envelope(), f.image()));
    assert!(result.is_err());
}

#[test]
fn malformed_step_oid_or_reference_pair_refuses_read_and_takeover() {
    let good = "1111111111111111111111111111111111111111";
    for (kind, old, new) in [
        ("reference", Some("not-an-oid"), Some(good)),
        ("reference", Some(good), Some("not-an-oid")),
        (
            "reference",
            Some(good),
            Some("0000000000000000000000000000000000000000"),
        ),
        (
            "reference",
            Some(good),
            Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        ),
        ("reference", None, Some(good)),
        ("reference", Some(good), None),
        ("reference", None, None),
        ("checkout", Some("bad"), None),
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        f.db().execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest,expected_old_oid,expected_new_oid) VALUES(?1,0,1,1,?2,'prepared',zeroblob(32),zeroblob(32),?3,?4)", params![row.operation_id.to_string(),kind,old,new]).unwrap();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired
        );
        assert_eq!(
            f.service
                .take_over_local_context_sync(&f.repo, &row)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT owner_epoch FROM local_context_sync_records",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }
}

#[test]
fn valid_reference_pair_and_nonreference_absence_remain_readable() {
    for (kind, old, new) in [
        (
            "reference",
            Some("1111111111111111111111111111111111111111"),
            Some("2222222222222222222222222222222222222222"),
        ),
        ("objects", None, None),
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        f.db().execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest,expected_old_oid,expected_new_oid) VALUES(?1,0,1,1,?2,'prepared',zeroblob(32),zeroblob(32),?3,?4)",params![row.operation_id.to_string(),kind,old,new]).unwrap();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap(),
            row
        );
        assert_eq!(
            f.service
                .take_over_local_context_sync(&f.repo, &row)
                .unwrap()
                .owner_epoch,
            2
        );
    }
}

#[test]
fn closing_admission_requires_genuine_checkpoint_authority() {
    for variant in [
        "wrong_subject",
        "wrong_author",
        "two_parents",
        "closed_parent",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        super::admission_tests::closure_owner_with(&f, true, variant);
        let before = f.image();
        let result = f.begin(OperationId::new());
        assert_eq!(
            result.unwrap_err().code(),
            ResultCode::RecoveryRequired,
            "{variant}"
        );
        assert_eq!(before, f.image());
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM local_context_sync_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn valid_checkpoint_descendant_admits_but_changed_closure_does_not() {
    for changed in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let parent = super::admission_tests::closure_owner_with(&f, true, "valid");
        let context = f.context();
        let checkpoint = context.head().unwrap().target().unwrap();
        crate::repository::closure::checkpoint::checkpoint_authority(&context, &f.item, checkpoint)
            .unwrap();
        if changed {
            let path = format!(".manyhands/tickets/{}/ticket.md", f.item);
            let full = context.workdir().unwrap().join(&path);
            let source = std::fs::read_to_string(&full).unwrap();
            std::fs::write(
                full,
                source.replace(
                    "Fixture <fixture@example.invalid>",
                    "Other <other@example.invalid>",
                ),
            )
            .unwrap();
        } else {
            std::fs::write(
                context.workdir().unwrap().join("ordinary"),
                "valid descendant\n",
            )
            .unwrap();
        }
        let descendant = super::record_tests::commit(&context);
        assert_ne!(checkpoint, descendant);
        let before = f.image();
        let result = f.begin(OperationId::new());
        if changed {
            assert_eq!(result.unwrap_err().code(), ResultCode::RecoveryRequired);
        } else {
            let record = result.unwrap();
            assert_eq!(record.closure_operation, Some(parent));
            assert_eq!(record.closure_checkpoint, Some(checkpoint));
            assert_eq!(record.context_parent, descendant);
        }
        assert_eq!(before, f.image());
    }
}

fn generated_key(f: &Fixture, store: &keys::KeyStore) -> SharedKeyRegistration {
    match f
        .service
        .generate_shared_key(
            store,
            keys::GenerateSharedKeyRequest {
                operation_id: OperationId::new(),
                label: "fixture".into(),
                protection: keys::KeyProtection::Unencrypted,
            },
        )
        .unwrap()
    {
        keys::GenerateSharedKeyOutcome::Created(key) => key,
        _ => panic!("fixture generation must complete"),
    }
}

#[test]
fn local_first_public_key_generation_refuses_before_journal_or_key_files() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    let home = tempfile::tempdir().unwrap();
    let store = keys::KeyStore::for_home(home.path()).unwrap();
    let result = f.service.generate_shared_key(
        &store,
        keys::GenerateSharedKeyRequest {
            operation_id: row.operation_id,
            label: "fixture".into(),
            protection: keys::KeyProtection::Unencrypted,
        },
    );
    assert!(matches!(result,Err(e) if e.kind==keys::KeyMaterialErrorKind::OperationMismatch));
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM key_material_operations", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
    assert_eq!(
        row,
        f.service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap()
    );
}

#[test]
fn local_first_public_key_deletion_preserves_key_files_and_registration() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let home = tempfile::tempdir().unwrap();
    let store = keys::KeyStore::for_home(home.path()).unwrap();
    let key = generated_key(&f, &store);
    let review = f
        .service
        .review_generated_key_deletion(&store, key.id)
        .unwrap();
    let row = f.begin(OperationId::new()).unwrap();
    let private = std::fs::read(&key.private_key_path).unwrap();
    let public = std::fs::read(key.public_key_path.as_ref().unwrap()).unwrap();
    let result = f
        .service
        .delete_generated_key(&store, row.operation_id, Some(review), true);
    assert!(matches!(result,Err(e) if e.kind==keys::KeyMaterialErrorKind::OperationMismatch));
    assert!(private == std::fs::read(&key.private_key_path).unwrap());
    assert!(public == std::fs::read(key.public_key_path.as_ref().unwrap()).unwrap());
    assert_eq!(f.service.list_shared_keys().unwrap(), vec![key]);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM key_material_operations WHERE operation_id=?1",
                [row.operation_id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn local_first_marked_key_registry_change_rolls_back_selection() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let home = tempfile::tempdir().unwrap();
    let store = keys::KeyStore::for_home(home.path()).unwrap();
    let key = generated_key(&f, &store);
    f.service.select_shared_key(key.id).unwrap();
    let before = f.service.list_shared_keys().unwrap();
    let row = f.begin(OperationId::new()).unwrap();
    let result = f
        .service
        .clear_shared_key_selection_under(&keys::KeyRegistryMark {
            operation_id: row.operation_id,
            expected: None,
        });
    assert!(result.is_err());
    assert_eq!(before, f.service.list_shared_keys().unwrap());
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM key_registry_changes", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn duplicate_foreign_journal_identity_invalidates_local_read_and_takeover() {
    for table in [
        "key_material_operations",
        "key_registry_changes",
        "remote_operation_records",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        let db = f.db();
        match table {
            "key_material_operations" => {
                db.execute("INSERT INTO key_material_operations(operation_id,key_id,action,generation_label,private_key_path,public_key_path,phase) VALUES(?1,?2,'generate','fixture','/fixture/private','/fixture/public','reserved')",params![row.operation_id.to_string(),OperationId::new().to_string()]).unwrap();
            }
            "key_registry_changes" => {
                db.execute("INSERT INTO key_registry_changes(operation_id,action,changed,recorded_at) VALUES(?1,'clear',0,1)",[row.operation_id.to_string()]).unwrap();
            }
            _ => {
                db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,local_branch,action,priority,phase,created_at,updated_at) SELECT id,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','main','synchronize_primary','manual','interrupted',1,1 FROM repositories",[row.operation_id.to_string()]).unwrap();
            }
        }
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired,
            "{table}"
        );
        assert_eq!(
            f.service
                .take_over_local_context_sync(&f.repo, &row)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired,
            "{table}"
        );
    }
}

#[test]
fn local_resolution_id_is_reserved_across_repository_boundaries() {
    for action in ["folder", "remote", "key", "promotion", "closure"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        let child = OperationId::new();
        f.db().execute("INSERT INTO local_context_sync_resolutions(attempt_ulid,operation_ulid,request_ulid,generation,owner_epoch,observation,input_digest,state) VALUES(?1,?2,?3,1,1,?4,zeroblob(32),'prepared')",params![child.to_string(),row.operation_id.to_string(),OperationId::new().to_string(),row.observation]).unwrap();
        let other = Fixture::new(AuthoringKind::Document);
        f.service
            .enable(EnableRepositoryRequest {
                root: other.root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        match action {
            "folder" => {
                let result = f.service.create_folder(CreateFolderRequest {
                    root: other.root.path().into(),
                    path: DocumentFolderPath::parse("docs/foreign-id").unwrap(),
                    operation_id: child,
                });
                assert!(!other.root.path().join("docs/foreign-id").exists());
                assert!(result.is_err());
            }
            "remote" => {
                let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
                remote::state::with_transaction(&f.service, other.root.path(), |tx, id| {
                    remote::state::configure(tx, id, Some(&plan), false)
                })
                .unwrap();
                let result = f.service.reserve_remote_operation(
                    other.root.path(),
                    child,
                    &RemoteOperationTarget::for_primary_synchronization(&plan),
                );
                assert!(result.is_err());
            }
            "promotion" | "closure" => {
                remote::state::with_transaction(&f.service, other.root.path(), |tx, id| {
                    if action == "promotion" {
                        assert!(
                            promotion::record::begin_record(
                                tx,
                                id,
                                child,
                                &other.item,
                                PromotionExpectation::from_bytes([1; 32]),
                                None
                            )
                            .is_err()
                        );
                    } else {
                        assert!(
                            closure::record::begin_record(
                                tx,
                                id,
                                child,
                                &other.item,
                                ClosureExpectation::from_bytes([1; 32]),
                                None,
                                closure::record::ClosureKind::Record
                            )
                            .is_err()
                        );
                    }
                    Ok(())
                })
                .unwrap();
            }
            _ => {
                let home = tempfile::tempdir().unwrap();
                let store = keys::KeyStore::for_home(home.path()).unwrap();
                assert!(
                    f.service
                        .generate_shared_key(
                            &store,
                            keys::GenerateSharedKeyRequest {
                                operation_id: child,
                                label: "fixture".into(),
                                protection: keys::KeyProtection::Unencrypted
                            }
                        )
                        .is_err()
                );
                assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
            }
        }
        assert_eq!(
            row,
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap()
        );
    }
}

#[test]
fn remote_child_id_guard_refuses_local_resolution_identity() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    let child = OperationId::new();
    f.db().execute("INSERT INTO local_context_sync_resolutions(attempt_ulid,operation_ulid,request_ulid,generation,owner_epoch,observation,input_digest,state) VALUES(?1,?2,?3,1,1,?4,zeroblob(32),'prepared')",params![child.to_string(),row.operation_id.to_string(),OperationId::new().to_string(),row.observation]).unwrap();
    let mut db = f.db();
    let tx = db.transaction().unwrap();
    assert!(remote::state::validate_child_id(&tx, child).is_err());
    assert!(remote::state::validate_child_id(&tx, row.operation_id).is_err());
    assert!(remote::state::validate_child_id(&tx, OperationId::new()).is_ok());
}
