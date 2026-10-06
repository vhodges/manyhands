use super::*;
use crate::repository::{
    REGISTRY_FILE,
    remote::{RemoteRefObservation, RemoteRefPlan},
};
use rusqlite::Connection;

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute(
        "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
        [root.path().to_str().unwrap()]).unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    (data, root, service)
}
fn plan() -> RemoteRefPlan {
    RemoteRefPlan::from_configuration("origin", "main").unwrap()
}
fn reserve(service: &RepositoryService, root: &Path) -> RemoteReservation {
    match service
        .reserve_remote_operation(
            root,
            OperationId::new(),
            &RemoteOperationTarget::for_poll(&plan()),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(token) => token,
        other => panic!("{other:?}"),
    }
}
fn observation() -> RemoteRefObservation {
    RemoteRefObservation::from_advertisement(
        &plan(),
        "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV",
        git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
        None,
    )
    .unwrap()
}
fn publish(service: &RepositoryService, root: &Path, token: &RemoteReservation) {
    service
        .remote_safe_point(root, token, RemoteOperationSafePoint::BeforeBatchCommit)
        .unwrap();
    assert_eq!(
        commit_observation_batch(service, root, token, &plan(), &[observation()], 123).unwrap(),
        RemoteSafePointOutcome::Continue
    );
}

// Catches committing a partial observation/deletion batch on a per-ref SQL fault.
#[test]
fn persistence_fault_rolls_back_batch_and_restart_repeats_only_unfinished_read() {
    let (data, root, service) = fixture();
    let first = reserve(&service, root.path());
    publish(&service, root.path(), &first);
    service
        .finish_remote_operation(root.path(), &first, RemoteOutcomeCategory::Completed)
        .unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    let next = reserve(&service, root.path());
    service
        .remote_safe_point(
            root.path(),
            &next,
            RemoteOperationSafePoint::BeforeBatchCommit,
        )
        .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_ref BEFORE INSERT ON remote_ref_observations BEGIN SELECT RAISE(ABORT,'SECRET_RESPONSE_SENTINEL'); END").unwrap();
    let error =
        commit_observation_batch(&service, root.path(), &next, &plan(), &[observation()], 124)
            .unwrap_err();
    assert!(!format!("{error:?} {error}").contains("SENTINEL"));
    assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
    assert_eq!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap()
            .completed_step(),
        Some(RemoteOperationSafePoint::BeforeBatchCommit)
    );
    connection.execute_batch("DROP TRIGGER fail_ref").unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let restarted = match reopened
        .restart_remote_observation(
            root.path(),
            next.operation_id(),
            &RemoteOperationTarget::for_poll(&plan()),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(token) => token,
        other => panic!("{other:?}"),
    };
    assert!(commit_observation_batch(&service, root.path(), &next, &plan(), &[], 125).is_err());
    publish(&reopened, root.path(), &restarted);
}

// Catches restarting a committed batch as an empty advertisement (false deletion).
#[test]
fn crash_after_batch_commit_completes_without_replacing_evidence() {
    let (data, root, service) = fixture();
    let token = reserve(&service, root.path());
    publish(&service, root.path(), &token);
    let before = service.remote_snapshot(root.path()).unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        matches!(reopened.restart_remote_observation(root.path(),token.operation_id(),&RemoteOperationTarget::for_poll(&plan())).unwrap(), RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Completed)
    );
    assert_eq!(before, reopened.remote_snapshot(root.path()).unwrap());
    assert!(
        reopened
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
}

// Catches the race between a successful before-commit hook and an interruption request.
#[test]
fn batch_commit_rechecks_requests_and_generation_atomically() {
    for cancel in [false, true] {
        let (_data, root, service) = fixture();
        let token = reserve(&service, root.path());
        service
            .remote_safe_point(
                root.path(),
                &token,
                RemoteOperationSafePoint::BeforeBatchCommit,
            )
            .unwrap();
        if cancel {
            service
                .cancel_remote_operation(root.path(), token.operation_id())
                .unwrap();
        } else {
            service
                .reserve_remote_operation(
                    root.path(),
                    OperationId::new(),
                    &RemoteOperationTarget::for_primary_synchronization(&plan()),
                )
                .unwrap();
        }
        assert_eq!(
            commit_observation_batch(
                &service,
                root.path(),
                &token,
                &plan(),
                &[observation()],
                123
            )
            .unwrap(),
            if cancel {
                RemoteSafePointOutcome::Cancelled
            } else {
                RemoteSafePointOutcome::Interrupted
            }
        );
        assert!(
            service
                .remote_snapshot(root.path())
                .unwrap()
                .observations()
                .is_empty()
        );
    }
    let (_data, root, service) = fixture();
    let token = reserve(&service, root.path());
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan()), true)
    })
    .unwrap();
    assert!(
        service
            .finish_remote_operation(
                root.path(),
                &token,
                RemoteOutcomeCategory::TransportUnavailable
            )
            .is_err()
    );
    assert!(
        commit_observation_batch(
            &service,
            root.path(),
            &token,
            &plan(),
            &[observation()],
            123
        )
        .is_err()
    );
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .latest_outcome(),
        None
    );
}

// Catches cancellation being lost on restart and failures releasing unacknowledged requests.
#[test]
fn terminal_outcomes_preserve_batch_and_restart_honors_pending_cancel() {
    let (data, root, service) = fixture();
    let first = reserve(&service, root.path());
    publish(&service, root.path(), &first);
    service
        .finish_remote_operation(root.path(), &first, RemoteOutcomeCategory::Completed)
        .unwrap();
    let next = reserve(&service, root.path());
    service
        .remote_safe_point(
            root.path(),
            &next,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    service
        .finish_remote_operation(
            root.path(),
            &next,
            RemoteOutcomeCategory::TransportUnavailable,
        )
        .unwrap();
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .len(),
        1
    );
    let next = reserve(&service, root.path());
    service
        .cancel_remote_operation(root.path(), next.operation_id())
        .unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        matches!(reopened.restart_remote_observation(root.path(),next.operation_id(),&RemoteOperationTarget::for_poll(&plan())).unwrap(), RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Cancelled)
    );
    assert_eq!(
        reopened
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .len(),
        1
    );
}

// Catches fabricated successful completion and moving a committed checkpoint backwards.
#[test]
fn safe_points_cannot_claim_a_batch_that_was_never_committed_or_rewind_one() {
    let (_data, root, service) = fixture();
    let token = reserve(&service, root.path());
    assert!(
        service
            .remote_safe_point(
                root.path(),
                &token,
                RemoteOperationSafePoint::AfterBatchCommit
            )
            .is_err()
    );
    assert!(
        service
            .finish_remote_operation(root.path(), &token, RemoteOutcomeCategory::Completed)
            .is_err()
    );
    publish(&service, root.path(), &token);
    assert!(
        service
            .remote_safe_point(
                root.path(),
                &token,
                RemoteOperationSafePoint::BeforeTransport
            )
            .is_err()
    );
    assert!(commit_observation_batch(&service, root.path(), &token, &plan(), &[], 124).is_err());
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .len(),
        1
    );
}

// Catches a fault acknowledging interruption releasing the row without a durable checkpoint.
#[test]
fn every_safe_point_fault_retains_requests_reservation_and_prior_batch() {
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
        RemoteOperationSafePoint::BetweenObservations,
        RemoteOperationSafePoint::BeforeBatchCommit,
        RemoteOperationSafePoint::AfterBatchCommit,
        RemoteOperationSafePoint::BeforeLocalMutation,
    ] {
        let (data, root, service) = fixture();
        let token = reserve(&service, root.path());
        publish(&service, root.path(), &token);
        service
            .finish_remote_operation(root.path(), &token, RemoteOutcomeCategory::Completed)
            .unwrap();
        let token = reserve(&service, root.path());
        service
            .cancel_remote_operation(root.path(), token.operation_id())
            .unwrap();
        let before = service.remote_snapshot(root.path()).unwrap();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection.execute_batch("CREATE TRIGGER fault_checkpoint BEFORE UPDATE OF completed_step ON remote_operation_records BEGIN SELECT RAISE(ABORT,'SECRET_SAFE_POINT_SENTINEL'); END").unwrap();
        let error = service
            .remote_safe_point(root.path(), &token, point)
            .unwrap_err();
        assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
        assert!(!format!("{error:?} {error}").contains("SENTINEL"));
        assert_eq!(
            service
                .active_remote_operation(root.path())
                .unwrap()
                .unwrap()
                .phase(),
            RemoteOperationPhase::Reserved
        );
        assert!(
            service
                .active_remote_operation(root.path())
                .unwrap()
                .unwrap()
                .cancel_requested()
        );
        assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
        connection
            .execute_batch("DROP TRIGGER fault_checkpoint")
            .unwrap();
        assert_eq!(
            service
                .remote_safe_point(root.path(), &token, point)
                .unwrap(),
            RemoteSafePointOutcome::Cancelled
        );
        assert_eq!(
            service.remote_snapshot(root.path()).unwrap().observations(),
            before.observations()
        );
        assert_eq!(
            service
                .remote_snapshot(root.path())
                .unwrap()
                .latest_outcome(),
            Some(RemoteOutcomeCategory::Cancelled)
        );
    }
}

// Catches accidental reinterpretation of a completed local operation ID as remote work.
#[test]
fn local_operation_ids_are_not_reused_as_remote_operation_ids() {
    let (data, root, service) = fixture();
    let operation_id = OperationId::new();
    Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute(
        "INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,observed_at) SELECT id,root_path,?1,'refresh','completed',123 FROM repositories",[operation_id.to_string()]).unwrap();
    let error = service
        .reserve_remote_operation(
            root.path(),
            operation_id,
            &RemoteOperationTarget::for_poll(&plan()),
        )
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
}

// Catches a damaged reservation schema silently clearing a durable cancel request on reopen.
#[test]
fn partial_reservation_schema_requires_recovery_instead_of_resetting_requests() {
    let (data, root, service) = fixture();
    let token = reserve(&service, root.path());
    service
        .cancel_remote_operation(root.path(), token.operation_id())
        .unwrap();
    Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .execute_batch("ALTER TABLE remote_operation_records DROP COLUMN cancel_requested")
        .unwrap();
    let error = match RepositoryService::open_at(data.path()) {
        Ok(_) => panic!("partial reservation schema silently repaired"),
        Err(error) => error,
    };
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
}

// Catches periodic remote arbitration walking every completed local operation.
#[test]
fn local_recovery_guard_work_is_bounded_with_retained_history() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("WITH RECURSIVE numbers(n) AS (VALUES(1000) UNION ALL SELECT n+1 FROM numbers WHERE n<4999)
        INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,observed_at)
        SELECT 1,(SELECT root_path FROM repositories WHERE id=1),printf('%026d',n),'refresh','completed',123 FROM numbers").unwrap();
    let work = std::cell::Cell::new(0usize);
    unsafe extern "C" fn count(context: *mut std::ffi::c_void) -> std::ffi::c_int {
        // The callback runs synchronously while the stack counter is alive.
        let work = unsafe { &*context.cast::<std::cell::Cell<usize>>() };
        work.set(work.get() + 1);
        0
    }
    // No operation below unwinds: capture the result, uninstall, then assert.
    unsafe {
        rusqlite::ffi::sqlite3_progress_handler(
            connection.handle(),
            1,
            Some(count),
            std::ptr::from_ref(&work).cast_mut().cast(),
        );
    }
    let result = crate::repository::recovery::require_no_pending_local(&connection, 1);
    unsafe {
        rusqlite::ffi::sqlite3_progress_handler(connection.handle(), 0, None, std::ptr::null_mut());
    }
    result.unwrap();
    assert!(
        work.get() < 500,
        "local recovery guard scanned retained history: {} VM steps",
        work.get()
    );
}
