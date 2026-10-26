use super::record_tests::{Fixture, commit};
use crate::repository::{closure::record as closure, remote::state};
use crate::{canonical, repository::*, results::ResultCode};

fn closure_owner(f: &Fixture, written: bool) -> OperationId {
    let operation = OperationId::new();
    let context = f.context();
    let tip = context.head().unwrap().target().unwrap();
    let at = time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    let closure_oid = if written {
        let path = format!(".manyhands/tickets/{}/ticket.md", f.item);
        let file = context.workdir().unwrap().join(&path);
        let mut item = canonical::parse_item(
            std::path::Path::new(&path),
            &std::fs::read_to_string(&file).unwrap(),
        )
        .unwrap();
        if let canonical::CanonicalItem::Ticket(ticket) = &mut item {
            ticket.closed_at = Some(at);
            ticket.closed_by = Some("Fixture <fixture@example.invalid>".into());
        }
        std::fs::write(file, canonical::serialize_item(&item).unwrap()).unwrap();
        commit(&context)
    } else {
        tip
    };
    state::with_transaction(&f.service, f.root.path(), |tx, rid| {
        let mut r = closure::begin_record(
            tx,
            rid,
            operation,
            &f.item,
            ClosureExpectation::from_bytes([7; 32]),
            None,
            closure::ClosureKind::Record,
        )?;
        r.context_oid = Some(tip);
        r.step = closure::ClosureStep::ContextSynchronized;
        closure::save_record(tx, rid, &r)?;
        if written {
            closure::begin_closing(tx, rid, operation, tip, at, closure_oid)?;
        }
        Ok(())
    })
    .unwrap();
    operation
}

#[test]
fn matching_written_closing_is_the_only_lifecycle_exception() {
    for written in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let parent = closure_owner(&f, written);
        let before = f.image();
        let result = f.begin(OperationId::new());
        if written {
            assert_eq!(result.unwrap().closure_operation, Some(parent));
        } else {
            assert_eq!(result.unwrap_err().code(), ResultCode::RecoveryRequired);
        }
        assert_eq!(before, f.image());
    }
}

#[test]
fn wrong_closure_item_stage_and_promotion_refuse() {
    for change in [
        "UPDATE ticket_closures SET item_id='01ARZ3NDEKTSV4RRFFQ69G5FAV'",
        "UPDATE ticket_closures SET step='integrating',integration_kind='merged'",
        "UPDATE ticket_closures SET closure_oid='1111111111111111111111111111111111111111'",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        closure_owner(&f, true);
        f.db().execute_batch(change).unwrap();
        assert_eq!(
            f.begin(OperationId::new()).unwrap_err().code(),
            ResultCode::RecoveryRequired
        );
    }
    let f = Fixture::new(AuthoringKind::Document);
    state::with_transaction(&f.service, f.root.path(), |tx, rid| {
        promotion::record::begin_record(
            tx,
            rid,
            OperationId::new(),
            &f.item,
            PromotionExpectation::from_bytes([9; 32]),
            None,
        )
        .map(|_| ())
    })
    .unwrap();
    assert_eq!(
        f.begin(OperationId::new()).unwrap_err().code(),
        ResultCode::RecoveryRequired
    );
}

#[test]
fn local_owner_reciprocally_blocks_closure_preview_and_adoption() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let parent = closure_owner(&f, true);
    f.begin(OperationId::new()).unwrap();
    let preview = f
        .service
        .preview_ticket_closure(PreviewTicketClosureRequest {
            root: f.root.path().into(),
            item_id: f.item.clone(),
            confirmed_identity: None,
        })
        .unwrap();
    assert!(matches!(preview, TicketClosurePreflight::Blocked { .. }));
    state::with_transaction(&f.service, f.root.path(), |tx, rid| {
        assert!(closure::context_synchronization_unfinished(
            tx, rid, &f.item
        )?);
        assert!(
            closure::adopt_written_closure(
                tx,
                rid,
                parent,
                &f.context(),
                f.context().head().unwrap().target().unwrap(),
                None
            )
            .is_err()
        );
        Ok(())
    })
    .unwrap();
}

fn remote(f: &Fixture, item: canonical::ItemId) -> (OperationId, RemoteOperationTarget) {
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&f.service, f.root.path(), |tx, rid| {
        state::configure(tx, rid, Some(&plan), false)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        item,
    )
    .unwrap();
    let id = OperationId::new();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root.path(), id, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    (id, target)
}

#[test]
fn retained_remote_and_local_admission_never_deadlock() {
    for same in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let item = if same {
            f.item.clone()
        } else {
            canonical::ItemId::generate()
        };
        let (id, target) = remote(&f, item);
        f.db().execute("UPDATE remote_operation_records SET phase='interrupted',reconciliation_required=1 WHERE operation_ulid=?1",[id.to_string()]).unwrap();
        let before: (String, i64) = f
            .db()
            .query_row(
                "SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            f.begin(OperationId::new()).unwrap_err().code(),
            ResultCode::RecoveryRequired
        );
        assert_eq!(before,f.db().query_row("SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).unwrap());
        // A genuinely released old owner may coexist with the new local
        // admission, but may not reacquire its remote phase afterward.
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM local_context_sync_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(matches!(
            f.service
                .restart_remote_synchronization(f.root.path(), id, &target)
                .unwrap(),
            RemoteReservationOutcome::Reserved(_)
        ));
        f.db().execute("UPDATE remote_operation_records SET phase='interrupted',reconciliation_required=0 WHERE operation_ulid=?1",[id.to_string()]).unwrap();
        let before: (String, i64) = f
            .db()
            .query_row(
                "SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let local = f.begin(OperationId::new()).unwrap();
        assert!(
            f.service
                .restart_remote_synchronization(f.root.path(), id, &target)
                .is_err()
        );
        assert_eq!(before,f.db().query_row("SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).unwrap());
        assert_eq!(
            local,
            f.service
                .read_local_context_sync_record(&f.repo, local.operation_id)
                .unwrap()
        );
    }
}

#[test]
fn retained_conflict_reacquisition_checks_local_before_phase_or_epoch() {
    for same in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let item = if same {
            f.item.clone()
        } else {
            canonical::ItemId::generate()
        };
        let (id, target) = remote(&f, item);
        let db = f.db();
        db.execute(
            "UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1",
            [id.to_string()],
        )
        .unwrap();
        let local = f.begin(OperationId::new()).unwrap();
        // Model retained evidence from an older executor/version. The new
        // local operation itself is always admitted through the real store.
        let oid = f.context().head().unwrap().target().unwrap().to_string();
        db.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase,window_number,conflict_digest) SELECT id,configuration_generation,owner_epoch,0,'context',?2,?2,?2,zeroblob(32),'conflict_pending',0,zeroblob(32) FROM remote_operation_records WHERE operation_ulid=?1",rusqlite::params![id.to_string(),oid]).unwrap();
        let before: (String, i64) = db
            .query_row(
                "SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(
            f.service
                .reacquire_synchronization_conflict_in_window(
                    f.root.path(),
                    id,
                    &target,
                    0,
                    0,
                    [0; 32]
                )
                .is_err()
        );
        assert_eq!(
            before,
            db.query_row(
                "SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?))
            )
            .unwrap()
        );
        assert_eq!(
            local,
            f.service
                .read_local_context_sync_record(&f.repo, local.operation_id)
                .unwrap()
        );
    }
}

#[test]
fn retained_remote_metadata_blocks_local_even_after_step_applied() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let (id, _) = remote(&f, canonical::ItemId::generate());
    let db = f.db();
    db.execute(
        "UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1",
        [id.to_string()],
    )
    .unwrap();
    let oid = f.context().head().unwrap().target().unwrap().to_string();
    db.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase,window_number,result_oid,observed_tree_oid) SELECT id,configuration_generation,owner_epoch,0,'context',?2,?2,?2,zeroblob(32),'applied',0,?2,?2 FROM remote_operation_records WHERE operation_ulid=?1",rusqlite::params![id.to_string(),oid]).unwrap();
    db.execute("INSERT INTO remote_integration_merge_metadata(integration_step_id,merge_head_digest,phase) VALUES(?1,zeroblob(32),'recorded')",[db.last_insert_rowid()]).unwrap();
    assert_eq!(
        f.begin(OperationId::new()).unwrap_err().code(),
        ResultCode::RecoveryRequired
    );
}

#[test]
fn pending_local_owner_refuses_new_lifecycle_parent_and_closure_integration() {
    let f = Fixture::new(AuthoringKind::Document);
    f.begin(OperationId::new()).unwrap();
    state::with_transaction(&f.service, f.root.path(), |tx, rid| {
        assert!(
            promotion::record::begin_record(
                tx,
                rid,
                OperationId::new(),
                &f.item,
                PromotionExpectation::from_bytes([9; 32]),
                None
            )
            .is_err()
        );
        Ok(())
    })
    .unwrap();
    let f = Fixture::new(AuthoringKind::Ticket);
    let parent = closure_owner(&f, true);
    f.begin(OperationId::new()).unwrap();
    state::with_transaction(&f.service, f.root.path(), |tx, rid| {
        let mut r = closure::read_record(tx, rid, parent)?.unwrap();
        r.step = closure::ClosureStep::Integrating;
        r.integration_kind = Some(promotion::record::IntegrationKind::Merged);
        assert!(closure::save_record(tx, rid, &r).is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn every_later_closure_stage_refuses_and_terminal_history_grants_no_exception() {
    for stage in [
        "integrating",
        "integrated",
        "published",
        "publication_pending",
        "remote_deleted",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
        "complete",
        "abandoned",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let parent = closure_owner(&f, stage != "abandoned");
        let db = f.db();
        if stage == "abandoned" {
            db.execute(
                "UPDATE ticket_closures SET step='abandoned',refusal_category='external_change'",
                [],
            )
            .unwrap();
        } else {
            let publication = if stage == "publication_pending" {
                "transport"
            } else {
                "published"
            };
            db.execute("UPDATE ticket_closures SET step=?1,remote_name='origin',integration_kind='merged',integration_oid=context_oid,publication_category=?2",rusqlite::params![stage,publication]).unwrap();
        }
        state::with_transaction(&f.service, f.root.path(), |tx, rid| {
            closure::read_record(tx, rid, parent).map(|r| assert!(r.is_some()))
        })
        .unwrap();
        let result = f.begin(OperationId::new());
        if matches!(stage, "complete" | "abandoned") {
            assert_eq!(result.unwrap().closure_operation, None, "{stage}");
        } else {
            assert_eq!(
                result.unwrap_err().code(),
                ResultCode::RecoveryRequired,
                "{stage}"
            );
        }
    }
}
