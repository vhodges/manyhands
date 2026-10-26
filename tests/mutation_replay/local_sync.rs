//! Acceptance-to-link request authority, through the existing request store API.
use super::support::{operations, records};
use manyhands::repository::request_store::InsertRequestOutcome;

#[test]
fn local_sync_acceptance_records_intended_parent_before_domain_link() {
    let world = records::world();
    for (id, command, role) in [
        (super::REQUEST_A, "context sync-local", "start"),
        (super::REQUEST_B, "context resolve-local", "resolve"),
    ] {
        let mut request = world.request(id);
        request.command = command.into();
        if role == "resolve" {
            request.operations.clear();
            request.target = operations::OPERATION_A.into();
        }
        assert_eq!(
            world.service.insert_request(&request).unwrap(),
            InsertRequestOutcome::Inserted
        );
        let row = world.index().query_row(
            "SELECT intended_parent,scope_key,role,execution_attempt,parent_operation,resolution_attempt,cancel_issued,cancel_acknowledged FROM local_context_sync_request_control WHERE request_ulid=?1",
            [id], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?,r.get::<_, i64>(3)?,r.get::<_, Option<String>>(4)?,r.get::<_, Option<String>>(5)?,r.get::<_, i64>(6)?,r.get::<_, i64>(7)?)),
        ).unwrap();
        assert_eq!(
            row,
            (
                operations::OPERATION_A.into(),
                world.scope.as_str().into(),
                role.into(),
                1,
                None,
                None,
                0,
                0
            )
        );
    }
}

#[test]
fn local_sync_attempt_handover_preserves_unlinked_identity_counters_and_effects() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context resolve-local".into();
    request.target = operations::OPERATION_A.into();
    request.operations.clear();
    world.service.insert_request(&request).unwrap();
    world.index().execute(
        "UPDATE local_context_sync_request_control SET cancel_issued=2,cancel_acknowledged=1,effect_write='written' WHERE request_ulid=?1",
        [super::REQUEST_A],
    ).unwrap();
    for expected in [2, 3] {
        assert_eq!(
            world.service.enter_request(request.request_id).unwrap(),
            Some(expected)
        );
        let row = world.index().query_row(
            "SELECT execution_attempt,intended_parent,cancel_issued,cancel_acknowledged,effect_write FROM local_context_sync_request_control WHERE request_ulid=?1",
            [super::REQUEST_A],|r| Ok((r.get::<_,u64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?)),
        ).unwrap();
        assert_eq!(
            row,
            (
                expected,
                operations::OPERATION_A.into(),
                2,
                1,
                "written".into()
            )
        );
    }
}

#[test]
fn local_sync_missing_control_refuses_handover_without_advancing_request() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    world.service.insert_request(&request).unwrap();
    world
        .index()
        .execute("DELETE FROM local_context_sync_request_control", [])
        .unwrap();
    assert!(world.service.enter_request(request.request_id).is_err());
    assert_eq!(world.record(super::REQUEST_A).unwrap().attempt, 1);
}

#[test]
fn local_sync_ignored_control_insert_rolls_back_acceptance() {
    let world = records::world();
    world.index().execute_batch("CREATE TRIGGER refuse_local_control BEFORE INSERT ON local_context_sync_request_control BEGIN SELECT RAISE(IGNORE); END").unwrap();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    assert!(world.service.insert_request(&request).is_err());
    assert!(world.record(super::REQUEST_A).is_none());
    assert_eq!(world.count("SELECT count(*) FROM request_operations"), 0);
}

#[test]
fn local_sync_stale_control_attempt_cannot_be_repaired_by_handover() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    world.service.insert_request(&request).unwrap();
    world
        .index()
        .execute("UPDATE request_records SET attempt=2", [])
        .unwrap();
    assert!(world.service.enter_request(request.request_id).is_err());
    assert_eq!(world.record(super::REQUEST_A).unwrap().attempt, 2);
    assert_eq!(
        world.count("SELECT execution_attempt FROM local_context_sync_request_control"),
        1
    );
}

#[test]
fn local_sync_ignored_control_handover_rolls_back_both_attempt_fences() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    world.service.insert_request(&request).unwrap();
    world.index().execute_batch("CREATE TRIGGER refuse_control_handover BEFORE UPDATE OF execution_attempt ON local_context_sync_request_control BEGIN SELECT RAISE(IGNORE); END").unwrap();
    assert!(world.service.enter_request(request.request_id).is_err());
    assert_eq!(world.record(super::REQUEST_A).unwrap().attempt, 1);
    assert_eq!(
        world.count("SELECT execution_attempt FROM local_context_sync_request_control"),
        1
    );
}

#[test]
fn local_sync_control_loss_is_not_recreated_on_reopen() {
    for lose_table in [false, true] {
        let world = records::world();
        if lose_table {
            world
                .index()
                .execute_batch("DROP TABLE local_context_sync_request_control")
                .unwrap();
        } else {
            world.index().execute("DELETE FROM registry_migrations WHERE name='local_context_sync_request_control_v1'",[]).unwrap();
        }
        let data = world.data.path().to_owned();
        drop(world.service);
        assert!(manyhands::repository::RepositoryService::open_at(&data).is_err());
    }
}

#[test]
fn local_sync_ordinary_requests_keep_their_existing_acceptance_and_handover() {
    let world = records::world();
    let request = world.request(super::REQUEST_A);
    world.service.insert_request(&request).unwrap();
    assert_eq!(
        world.service.enter_request(request.request_id).unwrap(),
        Some(2)
    );
    assert_eq!(
        world.count("SELECT count(*) FROM local_context_sync_request_control"),
        0
    );
}

#[test]
fn local_sync_unlinked_request_keeps_its_control_after_reopen() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context resolve-local".into();
    request.target = operations::OPERATION_A.into();
    request.operations.clear();
    world.service.insert_request(&request).unwrap();
    let service = manyhands::repository::RepositoryService::open_at(world.data.path()).unwrap();
    assert_eq!(service.enter_request(request.request_id).unwrap(), Some(2));
    assert_eq!(
        world.count("SELECT execution_attempt FROM local_context_sync_request_control"),
        2
    );
    assert_eq!(world.count("SELECT count(*) FROM local_context_sync_request_control WHERE parent_operation IS NULL AND resolution_attempt IS NULL"),1);
}
