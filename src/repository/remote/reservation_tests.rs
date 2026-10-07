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

// Catches treating an acknowledged yield as a fresh request forever.
#[test]
fn acknowledged_yield_restarts_unfinished_poll_with_a_new_fenced_owner() {
    let (data, root, service) = fixture();
    let other = RepositoryService::open_at(data.path()).unwrap();
    let background = reserve(&service, root.path());
    let target = RemoteOperationTarget::for_poll(&plan());
    let manual_id = OperationId::new();
    assert!(matches!(
        other
            .reserve_remote_operation_with_priority(
                root.path(),
                manual_id,
                &target,
                RemoteOperationPriority::Manual
            )
            .unwrap(),
        RemoteReservationOutcome::PollYielding
    ));
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &background,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .unwrap(),
        RemoteSafePointOutcome::Interrupted
    );
    let manual = match other
        .reserve_remote_operation_with_priority(
            root.path(),
            manual_id,
            &target,
            RemoteOperationPriority::Manual,
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(token) => token,
        result => panic!("{result:?}"),
    };
    publish(&other, root.path(), &manual);
    other
        .finish_remote_operation(root.path(), &manual, RemoteOutcomeCategory::Completed)
        .unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    let restarted = match service
        .restart_remote_observation(root.path(), background.operation_id(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(token) => token,
        result => panic!("acknowledged yield must restart unfinished polling: {result:?}"),
    };
    let active = service
        .active_remote_operation(root.path())
        .unwrap()
        .unwrap();
    assert_eq!(active.operation_id(), background.operation_id());
    assert_eq!(active.phase(), RemoteOperationPhase::Reserved);
    assert!(!active.yield_requested());
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &background,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &restarted,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [background.operation_id().to_string()],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    service
        .cancel_remote_operation(root.path(), restarted.operation_id())
        .unwrap();
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &restarted,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    assert!(
        matches!(service.restart_remote_observation(root.path(),restarted.operation_id(),&target).unwrap(),RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Cancelled)
    );
}

// Catches retrying a previously committed advertisement after an acknowledged yield.
#[test]
fn acknowledged_yield_after_batch_commit_replays_completed_evidence() {
    let (_data, root, service) = fixture();
    let background = reserve(&service, root.path());
    publish(&service, root.path(), &background);
    let before = service.remote_snapshot(root.path()).unwrap();
    assert!(matches!(
        service
            .reserve_remote_operation_with_priority(
                root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan()),
                RemoteOperationPriority::Manual
            )
            .unwrap(),
        RemoteReservationOutcome::PollYielding
    ));
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &background,
                RemoteOperationSafePoint::AfterBatchCommit
            )
            .unwrap(),
        RemoteSafePointOutcome::Interrupted
    );
    assert!(
        matches!(service.restart_remote_observation(root.path(),background.operation_id(),&RemoteOperationTarget::for_poll(&plan())).unwrap(),RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Completed)
    );
    assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
}

fn sync_target() -> RemoteOperationTarget {
    RemoteOperationTarget::for_primary_synchronization(&plan())
}
fn sync_owner(service: &RepositoryService, root: &Path) -> RemoteReservation {
    match service
        .reserve_remote_operation(root, OperationId::new(), &sync_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    }
}
#[test]
fn integration_steps_require_context_then_primary_ordering() {
    use super::super::merge::IntegrationStage;
    use crate::repository::{AuthoringKind, SynchronizationTarget};

    let (_data, root, service) = fixture();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    }
    .operation_target(&plan());
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    let intent = state::IntegrationStepIntent {
        ordinal: 0,
        stage: IntegrationStage::Context,
        local_oid: git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
        incoming_oid: git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap(),
        baseline_tree_oid: git2::Oid::from_str("3333333333333333333333333333333333333333").unwrap(),
        baseline_index_digest: [7; 32],
    };
    assert!(
        service
            .prepare_synchronization_integration(
                root.path(),
                &owner,
                &state::IntegrationStepIntent {
                    stage: IntegrationStage::Primary,
                    ..intent.clone()
                },
            )
            .is_err()
    );
    service
        .prepare_synchronization_integration(root.path(), &owner, &intent)
        .unwrap();
    assert!(
        service
            .prepare_synchronization_integration(
                root.path(),
                &owner,
                &state::IntegrationStepIntent {
                    ordinal: 1,
                    stage: IntegrationStage::Context,
                    ..intent.clone()
                },
            )
            .is_err()
    );
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 1,
                stage: IntegrationStage::Primary,
                ..intent
            },
        )
        .unwrap();
}

fn sync_evidence() -> state::SynchronizationEvidence {
    state::SynchronizationEvidence {
        expected_oid: Some(
            git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
        ),
        local_oid: Some(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap()),
        tracking_oid: Some(
            git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
        ),
        primary_tracking_oid: Some(
            git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
        ),
        push_oid: Some(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap()),
        push_advertised_oid: None,
    }
}
fn sync_fetch(service: &RepositoryService, root: &Path, owner: &RemoteReservation) {
    service
        .checkpoint_synchronization(
            root,
            owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &state::SynchronizationEvidence::default(),
        )
        .unwrap();
    service
        .remote_safe_point(root, owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    commit_observation_batch(service, root, owner, &plan(), &[observation()], 123).unwrap();
}
fn sync_push_prepared(service: &RepositoryService, root: &Path, owner: &RemoteReservation) {
    sync_fetch(service, root, owner);
    service
        .checkpoint_synchronization(
            root,
            owner,
            state::SynchronizationCheckpoint::PushPrepared,
            &sync_evidence(),
        )
        .unwrap();
}

#[test]
fn candidate_reconciliation_advances_fetch_observed_once_after_child_observation() {
    use super::super::merge::IntegrationStage;
    let (_data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    let local = git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap();
    let incoming = git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap();
    let fetch_evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        tracking_oid: Some(incoming),
        primary_tracking_oid: Some(incoming),
        ..state::SynchronizationEvidence::default()
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &fetch_evidence,
        )
        .unwrap();
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    commit_observation_batch(
        &service,
        root.path(),
        &owner,
        &plan(),
        &[observation()],
        123,
    )
    .unwrap();
    let candidate = git2::Oid::from_str("4444444444444444444444444444444444444444").unwrap();
    let tree = git2::Oid::from_str("5555555555555555555555555555555555555555").unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: IntegrationStage::Primary,
                local_oid: local,
                incoming_oid: incoming,
                baseline_tree_oid: local,
                baseline_index_digest: [0; 32],
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, Some(candidate))
        .unwrap();
    let resumed = match service
        .restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!(),
    };
    let step = service
        .applying_synchronization_candidate(root.path(), &resumed)
        .unwrap()
        .unwrap();
    service
        .observe_synchronization_integration_effect(root.path(), &resumed, 0, candidate, tree)
        .unwrap();
    let record = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    assert!(record.reconciliation_required);
    assert_eq!(
        record.sync_checkpoint,
        Some(state::SynchronizationCheckpoint::FetchObserved)
    );
    let mut evidence = record.sync_evidence;
    evidence.local_oid = Some(candidate);
    evidence.primary_tracking_oid = Some(local);
    evidence.tracking_oid = Some(local);
    assert_eq!(step.candidate_oid, Some(candidate));
    service
        .reconcile_synchronization_candidate_applied(
            root.path(),
            &resumed,
            0,
            candidate,
            tree,
            &evidence,
        )
        .unwrap();
    let completed = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    assert_eq!(
        completed.sync_checkpoint,
        Some(state::SynchronizationCheckpoint::LocalFastForwarded)
    );
    assert!(!completed.reconciliation_required);
    // A distinct candidate cannot be adopted after completion; the durable
    // child evidence remains the exact recorded candidate.
    let mismatched = git2::Oid::from_str("6666666666666666666666666666666666666666").unwrap();
    assert!(
        service
            .reconcile_synchronization_candidate_applied(
                root.path(),
                &resumed,
                0,
                mismatched,
                tree,
                &evidence,
            )
            .is_err()
    );
    let unchanged = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    assert_eq!(
        unchanged.sync_checkpoint,
        Some(state::SynchronizationCheckpoint::LocalFastForwarded)
    );
}

#[test]
fn conflict_release_fences_stale_owner_and_requires_explicit_matching_reacquisition() {
    use super::super::merge::IntegrationStage;
    let (data, root, service) = fixture();
    let other = RepositoryService::open_at(data.path()).unwrap();
    let owner = sync_owner(&service, root.path());
    let intent = state::IntegrationStepIntent {
        ordinal: 0,
        stage: IntegrationStage::Primary,
        local_oid: git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
        incoming_oid: git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap(),
        baseline_tree_oid: git2::Oid::from_str("3333333333333333333333333333333333333333").unwrap(),
        baseline_index_digest: [7; 32],
    };
    service
        .prepare_synchronization_integration(root.path(), &owner, &intent)
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, None)
        .unwrap();
    let conflict = [9; 32];
    service
        .release_synchronization_conflict(root.path(), &owner, 0, conflict)
        .unwrap();
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        other
            .reserve_remote_operation(root.path(), OperationId::new(), &sync_target())
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    assert!(
        service
            .restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target())
            .is_err()
    );
    assert!(
        other
            .reacquire_synchronization_conflict(
                root.path(),
                owner.operation_id(),
                &sync_target(),
                0,
                [8; 32]
            )
            .is_err()
    );
    let reacquired = match other
        .reacquire_synchronization_conflict(
            root.path(),
            owner.operation_id(),
            &sync_target(),
            0,
            conflict,
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    assert!(
        service
            .begin_synchronization_integration_effect(root.path(), &owner, 0, None)
            .is_err()
    );
    let confirmation = state::IdentityConfirmationIntent {
        confirmation_id: OperationId::new(),
        input_digest: [4; 32],
        configuration_digest: [5; 32],
    };
    other
        .prepare_synchronization_identity_confirmation(root.path(), &reacquired, &confirmation)
        .unwrap();
    let attempt = state::ResolutionAttemptIntent {
        attempt_id: OperationId::new(),
        step_ordinal: 0,
        observation_digest: conflict,
        input_digest: [3; 32],
        identity_confirmation_id: Some(confirmation.confirmation_id),
    };
    let path = state::ResolutionPathIntent {
        ordinal: 0,
        path_digest: [1; 32],
        expected_digest: [2; 32],
        result_digest: [3; 32],
        base_blob_oid: None,
        local_blob_oid: None,
        incoming_blob_oid: None,
        mode: 33188,
    };
    other
        .prepare_synchronization_resolution_attempt(
            root.path(),
            &reacquired,
            &attempt,
            std::slice::from_ref(&path),
        )
        .unwrap();
    other
        .prepare_synchronization_resolution_attempt(
            root.path(),
            &reacquired,
            &attempt,
            std::slice::from_ref(&path),
        )
        .unwrap();
    let mismatched = state::ResolutionAttemptIntent {
        input_digest: [4; 32],
        ..attempt
    };
    assert!(
        other
            .prepare_synchronization_resolution_attempt(
                root.path(),
                &reacquired,
                &mismatched,
                std::slice::from_ref(&path)
            )
            .is_err()
    );
    let changed_confirmation = state::IdentityConfirmationIntent {
        confirmation_id: OperationId::new(),
        input_digest: [4; 32],
        configuration_digest: [5; 32],
    };
    other
        .prepare_synchronization_identity_confirmation(
            root.path(),
            &reacquired,
            &changed_confirmation,
        )
        .unwrap();
    assert!(
        other
            .prepare_synchronization_resolution_attempt(
                root.path(),
                &reacquired,
                &state::ResolutionAttemptIntent {
                    identity_confirmation_id: Some(changed_confirmation.confirmation_id),
                    ..attempt.clone()
                },
                std::slice::from_ref(&path),
            )
            .is_err()
    );
    other
        .begin_synchronization_identity_confirmation_effect(
            root.path(),
            &reacquired,
            confirmation.confirmation_id,
        )
        .unwrap();
    other
        .observe_synchronization_identity_confirmation_effect(
            root.path(),
            &reacquired,
            confirmation.confirmation_id,
            [6; 32],
        )
        .unwrap();
    other
        .begin_synchronization_resolution_path_effects(root.path(), &reacquired, attempt.attempt_id)
        .unwrap();
    other
        .observe_synchronization_resolution_path_effect(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            0,
        )
        .unwrap();
    let checkpoint = git2::Oid::from_str("4444444444444444444444444444444444444444").unwrap();
    let tree = git2::Oid::from_str("5555555555555555555555555555555555555555").unwrap();
    other
        .prepare_synchronization_resolution_candidate(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            checkpoint,
        )
        .unwrap();
    other
        .observe_synchronization_resolution_checkpoint(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            checkpoint,
            tree,
        )
        .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT phase FROM remote_integration_steps", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
        "applied"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM remote_resolution_paths WHERE applied=1",
                [],
                |row| { row.get::<_, i64>(0) }
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT phase FROM remote_identity_confirmations",
                [],
                |row| { row.get::<_, String>(0) }
            )
            .unwrap(),
        "applied"
    );
}

#[test]
fn sync_ambiguous_push_restart_is_reconciliation_not_a_fresh_push() {
    let (data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
        .unwrap();
    service
        .finish_remote_operation(
            root.path(),
            &owner,
            RemoteOutcomeCategory::TransportUnavailable,
        )
        .unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let restarted = match reopened
        .restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    let record = reopened
        .active_remote_operation(root.path())
        .unwrap()
        .unwrap();
    assert_eq!(record.phase(), RemoteOperationPhase::Reconciling);
    assert_eq!(
        record.sync_checkpoint(),
        Some(state::SynchronizationCheckpoint::PushPrepared)
    );
    assert_eq!(record.sync_evidence().push_oid, sync_evidence().push_oid);
    assert!(
        reopened
            .checkpoint_synchronization(
                root.path(),
                &restarted,
                state::SynchronizationCheckpoint::PushPrepared,
                &sync_evidence()
            )
            .is_err()
    );
    reopened
        .remote_safe_point(
            root.path(),
            &restarted,
            RemoteOperationSafePoint::BeforeFetch,
        )
        .unwrap();
    commit_observation_batch(&reopened, root.path(), &restarted, &plan(), &[], 124).unwrap();
    assert!(
        reopened
            .checkpoint_synchronization(
                root.path(),
                &restarted,
                state::SynchronizationCheckpoint::PushPrepared,
                &sync_evidence()
            )
            .is_err()
    );
    let candidate = sync_evidence().push_oid.unwrap();
    assert!(
        reopened
            .reconcile_synchronization(
                root.path(),
                &restarted,
                candidate,
                candidate,
                Some(sync_evidence().expected_oid.unwrap()),
                false
            )
            .is_err()
    );
    assert!(
        reopened
            .reconcile_synchronization(
                root.path(),
                &restarted,
                candidate,
                sync_evidence().expected_oid.unwrap(),
                Some(candidate),
                false
            )
            .is_err()
    );
    assert!(
        service
            .reconcile_synchronization(
                root.path(),
                &owner,
                candidate,
                candidate,
                Some(candidate),
                false
            )
            .is_err()
    );
    reopened
        .reconcile_synchronization(
            root.path(),
            &restarted,
            candidate,
            candidate,
            Some(candidate),
            false,
        )
        .unwrap();
    assert_eq!(
        reopened
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap()
            .sync_checkpoint(),
        Some(state::SynchronizationCheckpoint::PushVerified)
    );
    assert!(
        reopened
            .checkpoint_synchronization(
                root.path(),
                &restarted,
                state::SynchronizationCheckpoint::PushPrepared,
                &sync_evidence()
            )
            .is_err()
    );
}

#[test]
fn sync_authoritative_replay_is_exact_and_index_only_even_after_cancel() {
    let (_data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    let mut evidence = sync_evidence();
    evidence.push_advertised_oid = evidence.push_oid;
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::PushVerified,
            &evidence,
        )
        .unwrap();
    let outcome = state::SynchronizationAuthority::Published(evidence.push_oid.unwrap());
    service
        .classify_synchronization(root.path(), &owner, outcome)
        .unwrap();
    service
        .cancel_remote_operation(root.path(), owner.operation_id())
        .unwrap();
    assert!(
        matches!(service.restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target()).unwrap(), RemoteReservationOutcome::Replay(record) if record.authority()==Some(outcome) && record.index_pending())
    );
    assert!(
        service
            .reserve_remote_operation(
                root.path(),
                owner.operation_id(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .is_err()
    );
    service
        .finish_synchronization_index(root.path(), owner.operation_id(), &sync_target())
        .unwrap();
    assert!(
        matches!(service.reserve_remote_operation(root.path(), owner.operation_id(), &sync_target()).unwrap(), RemoteReservationOutcome::Replay(record) if record.authority()==Some(outcome) && !record.index_pending() && record.phase()==RemoteOperationPhase::Completed)
    );
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
}

#[test]
fn sync_every_checkpoint_database_fault_preserves_previous_boundary_and_candidate() {
    use state::SynchronizationCheckpoint as C;
    for checkpoint in [
        C::FetchPrepared,
        C::LocalPrepared,
        C::LocalFastForwarded,
        C::PushPrepared,
        C::PushReturned,
        C::PushVerified,
    ] {
        let (data, root, service) = fixture();
        let owner = sync_owner(&service, root.path());
        if checkpoint != C::FetchPrepared {
            sync_fetch(&service, root.path(), &owner);
        }
        let mut evidence = if checkpoint == C::FetchPrepared {
            state::SynchronizationEvidence::default()
        } else {
            sync_evidence()
        };
        if checkpoint == C::LocalFastForwarded {
            service
                .checkpoint_synchronization(root.path(), &owner, C::LocalPrepared, &evidence)
                .unwrap();
            service
                .remote_safe_point(
                    root.path(),
                    &owner,
                    RemoteOperationSafePoint::BeforeLocalUpdate,
                )
                .unwrap();
        }
        if matches!(checkpoint, C::PushReturned | C::PushVerified) {
            service
                .checkpoint_synchronization(root.path(), &owner, C::PushPrepared, &evidence)
                .unwrap();
            service
                .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
                .unwrap();
        }
        if checkpoint == C::PushVerified {
            evidence.push_advertised_oid = evidence.push_oid;
        }
        let before = service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection.execute_batch("CREATE TRIGGER sync_fault BEFORE UPDATE OF sync_checkpoint ON remote_operation_records BEGIN SELECT RAISE(ABORT,'HOSTILE_SERVER_TEXT'); END").unwrap();
        let error = service
            .checkpoint_synchronization(root.path(), &owner, checkpoint, &evidence)
            .unwrap_err();
        assert!(!format!("{error} {error:?}").contains("HOSTILE"));
        let after = service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap();
        assert_eq!(after.sync_checkpoint(), before.sync_checkpoint());
        assert_eq!(after.sync_evidence(), before.sync_evidence());
        assert!(matches!(
            service
                .reserve_remote_operation(root.path(), OperationId::new(), &sync_target())
                .unwrap(),
            RemoteReservationOutcome::Busy
        ));
        connection.execute_batch("DROP TRIGGER sync_fault").unwrap();
        service
            .checkpoint_synchronization(root.path(), &owner, checkpoint, &evidence)
            .unwrap();
    }
}

#[test]
fn sync_batch_and_discovery_faults_roll_back_atomically() {
    let (data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
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
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_sync_batch BEFORE UPDATE OF sync_checkpoint ON remote_operation_records BEGIN SELECT RAISE(ABORT,'HOSTILE_SERVER_TEXT'); END").unwrap();
    assert!(
        commit_observation_batch(
            &service,
            root.path(),
            &owner,
            &plan(),
            &[observation()],
            123
        )
        .is_err()
    );
    assert!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .is_empty()
    );
    connection
        .execute_batch("DROP TRIGGER fail_sync_batch")
        .unwrap();
    commit_observation_batch(
        &service,
        root.path(),
        &owner,
        &plan(),
        &[observation()],
        123,
    )
    .unwrap();
    let mut evidence = sync_evidence();
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::PushPrepared,
            &evidence,
        )
        .unwrap();
    evidence.push_advertised_oid = evidence.push_oid;
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::PushVerified,
            &evidence,
        )
        .unwrap();
    connection.execute_batch("CREATE TRIGGER fail_authority BEFORE UPDATE OF authoritative_kind ON remote_operation_records BEGIN SELECT RAISE(ABORT,'HOSTILE_SERVER_TEXT'); END").unwrap();
    let authority = state::SynchronizationAuthority::Published(evidence.push_oid.unwrap());
    assert!(
        service
            .classify_synchronization(root.path(), &owner, authority)
            .is_err()
    );
    assert_eq!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap()
            .sync_checkpoint(),
        Some(state::SynchronizationCheckpoint::PushVerified)
    );
    connection
        .execute_batch("DROP TRIGGER fail_authority")
        .unwrap();
    service
        .classify_synchronization(root.path(), &owner, authority)
        .unwrap();
    connection.execute_batch("CREATE TRIGGER fail_index BEFORE UPDATE OF index_pending ON remote_operation_records BEGIN SELECT RAISE(ABORT,'HOSTILE_SERVER_TEXT'); END").unwrap();
    assert!(
        service
            .finish_synchronization_index(root.path(), owner.operation_id(), &sync_target())
            .is_err()
    );
    assert!(
        matches!(service.reserve_remote_operation(root.path(), owner.operation_id(), &sync_target()).unwrap(), RemoteReservationOutcome::Replay(record) if record.authority()==Some(authority) && record.index_pending())
    );
}

#[test]
fn sync_reconciled_ancestor_requires_explicit_restart_and_keeps_recorded_candidate() {
    let (_data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    assert!(
        matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&sync_target()).unwrap(),RemoteReservationOutcome::Replay(record) if record.sync_checkpoint()==Some(state::SynchronizationCheckpoint::PushPrepared))
    );
    let restarted = match service
        .restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    service
        .remote_safe_point(
            root.path(),
            &restarted,
            RemoteOperationSafePoint::BeforeFetch,
        )
        .unwrap();
    commit_observation_batch(&service, root.path(), &restarted, &plan(), &[], 124).unwrap();
    let evidence = sync_evidence();
    let candidate = evidence.push_oid.unwrap();
    let ancestor = evidence.expected_oid.unwrap();
    service
        .reconcile_synchronization(
            root.path(),
            &restarted,
            candidate,
            candidate,
            Some(ancestor),
            true,
        )
        .unwrap();
    let record = service
        .active_remote_operation(root.path())
        .unwrap()
        .unwrap();
    assert_eq!(record.sync_evidence().push_oid, Some(candidate));
    assert_eq!(
        record.sync_checkpoint(),
        Some(state::SynchronizationCheckpoint::PushPrepared)
    );
    service
        .remote_safe_point(
            root.path(),
            &restarted,
            RemoteOperationSafePoint::BeforePush,
        )
        .unwrap();
    assert!(
        service
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
            .is_err()
    );
}

#[test]
fn sync_owner_target_root_generation_and_active_index_are_fenced() {
    use state::SynchronizationCheckpoint as C;
    let (data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let second = tempfile::tempdir().unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)",[second.path().to_str().unwrap()]).unwrap();
    state::with_transaction(&service, second.path(), |tx, id| {
        state::configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    assert!(
        matches!(service.reserve_remote_operation(second.path(),owner.operation_id(),&sync_target()),Err(error) if error.kind==RepositoryErrorKind::OperationMismatch)
    );
    assert!(
        service
            .checkpoint_synchronization(second.path(), &owner, C::PushReturned, &sync_evidence())
            .is_err()
    );
    assert!(
        service
            .restart_remote_synchronization(
                root.path(),
                owner.operation_id(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .is_err()
    );
    // Bypass API arbitration to prove the SQL index covers the new active phases.
    assert!(
        state::with_transaction(&service, root.path(), |tx, id| state::insert_operation(
            tx,
            id,
            OperationId::new(),
            &sync_target(),
            RemoteOperationPriority::Manual,
            123
        ))
        .is_err()
    );
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan()), true)
    })
    .unwrap();
    assert!(
        service
            .checkpoint_synchronization(root.path(), &owner, C::PushReturned, &sync_evidence())
            .is_err()
    );
    assert!(
        service
            .restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target())
            .is_err()
    );
    assert!(
        matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&sync_target()).unwrap(),RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Interrupted && record.sync_evidence().push_oid==sync_evidence().push_oid)
    );
}

#[test]
fn sync_safe_point_failures_and_cancellation_never_erase_effect_evidence() {
    use state::SynchronizationCheckpoint as C;
    for point in [
        RemoteOperationSafePoint::BeforeFetch,
        RemoteOperationSafePoint::AfterFetch,
        RemoteOperationSafePoint::BeforeLocalUpdate,
        RemoteOperationSafePoint::AfterLocalUpdate,
        RemoteOperationSafePoint::BeforePush,
        RemoteOperationSafePoint::AfterPushReturn,
        RemoteOperationSafePoint::AfterPushVerification,
    ] {
        let (data, root, service) = fixture();
        let owner = sync_owner(&service, root.path());
        if point == RemoteOperationSafePoint::BeforeFetch {
            service
                .checkpoint_synchronization(
                    root.path(),
                    &owner,
                    C::FetchPrepared,
                    &state::SynchronizationEvidence::default(),
                )
                .unwrap();
        } else {
            sync_fetch(&service, root.path(), &owner);
        }
        let mut evidence = sync_evidence();
        if matches!(
            point,
            RemoteOperationSafePoint::BeforeLocalUpdate
                | RemoteOperationSafePoint::AfterLocalUpdate
        ) {
            service
                .checkpoint_synchronization(root.path(), &owner, C::LocalPrepared, &evidence)
                .unwrap();
            if point == RemoteOperationSafePoint::AfterLocalUpdate {
                service
                    .remote_safe_point(
                        root.path(),
                        &owner,
                        RemoteOperationSafePoint::BeforeLocalUpdate,
                    )
                    .unwrap();
                service
                    .checkpoint_synchronization(
                        root.path(),
                        &owner,
                        C::LocalFastForwarded,
                        &evidence,
                    )
                    .unwrap();
            }
        }
        if matches!(
            point,
            RemoteOperationSafePoint::BeforePush
                | RemoteOperationSafePoint::AfterPushReturn
                | RemoteOperationSafePoint::AfterPushVerification
        ) {
            service
                .checkpoint_synchronization(root.path(), &owner, C::PushPrepared, &evidence)
                .unwrap();
            if point == RemoteOperationSafePoint::AfterPushReturn {
                service
                    .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
                    .unwrap();
                service
                    .checkpoint_synchronization(root.path(), &owner, C::PushReturned, &evidence)
                    .unwrap();
            }
            if point == RemoteOperationSafePoint::AfterPushVerification {
                evidence.push_advertised_oid = evidence.push_oid;
                service
                    .checkpoint_synchronization(root.path(), &owner, C::PushVerified, &evidence)
                    .unwrap();
            }
        }
        let before = service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap();
        service
            .cancel_remote_operation(root.path(), owner.operation_id())
            .unwrap();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection.execute_batch("CREATE TRIGGER fault_sync_safe BEFORE UPDATE OF completed_step ON remote_operation_records BEGIN SELECT RAISE(ABORT,'HOSTILE_SERVER_TEXT'); END").unwrap();
        assert!(
            service
                .remote_safe_point(root.path(), &owner, point)
                .is_err()
        );
        let after = service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap();
        assert!(after.cancel_requested());
        assert_eq!(after.sync_checkpoint(), before.sync_checkpoint());
        assert_eq!(after.sync_evidence(), before.sync_evidence());
        connection
            .execute_batch("DROP TRIGGER fault_sync_safe")
            .unwrap();
        assert_eq!(
            service
                .remote_safe_point(root.path(), &owner, point)
                .unwrap(),
            RemoteSafePointOutcome::Cancelled
        );
        assert!(
            matches!(service.restart_remote_synchronization(root.path(),owner.operation_id(),&sync_target()).unwrap(),RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Cancelled && record.sync_evidence()==before.sync_evidence())
        );
    }
}

#[test]
fn sync_fetch_preserves_polling_policy_and_honors_cancel_atomically() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("UPDATE remote_polling_state SET paused=1,enabled=0,automatic_backoff_seconds=300,latest_outcome='transport_unavailable'").unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    let owner = sync_owner(&service, root.path());
    sync_fetch(&service, root.path(), &owner);
    let after = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(before.polling(), after.polling());
    assert_eq!(before.latest_outcome(), after.latest_outcome());
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::AfterFetch)
        .unwrap();
    service
        .finish_remote_operation(
            root.path(),
            &owner,
            RemoteOutcomeCategory::TransportUnavailable,
        )
        .unwrap();
    let owner = sync_owner(&service, root.path());
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
    service
        .cancel_remote_operation(root.path(), owner.operation_id())
        .unwrap();
    assert_eq!(
        commit_observation_batch(&service, root.path(), &owner, &plan(), &[], 124).unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    assert_eq!(service.remote_snapshot(root.path()).unwrap(), after);
}

#[test]
fn sync_privacy_covers_live_wal_backup_rows_and_formatted_replay() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("PRAGMA journal_mode=WAL; BEGIN; SELECT count(*) FROM repositories;")
        .unwrap();
    let owner = sync_owner(&service, root.path());
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
    let sentinels = [
        "HOSTILE_RAW_URL",
        "HOSTILE_SERVER_TEXT",
        "HOSTILE_CREDENTIAL",
        "HOSTILE_KEY",
        "HOSTILE_PASSPHRASE",
        "HOSTILE_MARKDOWN",
        "HOSTILE_WORKTREE_PATH",
    ];
    let observations: Vec<_> = sentinels
        .iter()
        .map(|sentinel| {
            RemoteRefObservation::from_advertisement(
                &plan(),
                &format!("refs/heads/manyhands/ticket/{sentinel}"),
                sync_evidence().local_oid.unwrap(),
                None,
            )
            .unwrap()
        })
        .collect();
    commit_observation_batch(&service, root.path(), &owner, &plan(), &observations, 123).unwrap();
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::PushPrepared,
            &sync_evidence(),
        )
        .unwrap();
    let formatted = format!(
        "{:?} {:?} {:?}",
        owner,
        service
            .reserve_remote_operation(root.path(), owner.operation_id(), &sync_target())
            .unwrap(),
        service.remote_snapshot(root.path()).unwrap()
    );
    let backup = data.path().join("sync-backup.sqlite3");
    let backup_connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    backup_connection
        .execute("VACUUM INTO ?1", [backup.to_str().unwrap()])
        .unwrap();
    assert!(data.path().join(format!("{REGISTRY_FILE}-wal")).is_file());
    for entry in std::fs::read_dir(data.path()).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(path).unwrap();
        for sentinel in sentinels {
            assert!(
                !bytes
                    .windows(sentinel.len())
                    .any(|value| value == sentinel.as_bytes())
            );
            assert!(!formatted.contains(sentinel));
        }
    }
}

#[test]
fn sync_authoritative_same_id_refresh_replay_allows_only_exact_refresh_identity() {
    for refresh_state in ["created", "indexing", "failed", "completed"] {
        let (data, root, service) = fixture();
        let owner = sync_owner(&service, root.path());
        sync_push_prepared(&service, root.path(), &owner);
        let mut evidence = sync_evidence();
        evidence.push_advertised_oid = evidence.push_oid;
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::PushVerified,
                &evidence,
            )
            .unwrap();
        let authority = state::SynchronizationAuthority::AlreadyCurrent(evidence.push_oid.unwrap());
        service
            .classify_synchronization(root.path(), &owner, authority)
            .unwrap();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT id,root_path,?1,'refresh','',?2,123 FROM repositories",params![owner.operation_id().to_string(),refresh_state]).unwrap();
        assert!(
            matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&sync_target()).unwrap(),RemoteReservationOutcome::Replay(record) if record.authority()==Some(authority) && record.index_pending())
        );
        assert!(
            matches!(service.restart_remote_synchronization(root.path(),owner.operation_id(),&sync_target()).unwrap(),RemoteReservationOutcome::Replay(record) if record.authority()==Some(authority))
        );
        assert!(
            service
                .active_remote_operation(root.path())
                .unwrap()
                .is_none()
        );
        for corruption in [
            "action='close'",
            "target='other'",
            "target=NULL",
            "root_path='other-root'",
            "repository_id=NULL",
        ] {
            connection
                .execute(
                    &format!("UPDATE operation_records SET {corruption} WHERE operation_ulid=?1"),
                    [owner.operation_id().to_string()],
                )
                .unwrap();
            assert!(
                matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&sync_target()),Err(error) if error.kind==RepositoryErrorKind::OperationMismatch)
            );
            assert!(
                service
                    .restart_remote_synchronization(
                        root.path(),
                        owner.operation_id(),
                        &sync_target()
                    )
                    .is_err()
            );
            assert!(
                service
                    .finish_synchronization_index(root.path(), owner.operation_id(), &sync_target())
                    .is_err()
            );
            connection.execute("UPDATE operation_records SET action='refresh',target='',root_path=(SELECT root_path FROM repositories WHERE id=1),repository_id=1 WHERE operation_ulid=?1",[owner.operation_id().to_string()]).unwrap();
        }
        service
            .finish_synchronization_index(root.path(), owner.operation_id(), &sync_target())
            .unwrap();
    }
}

#[test]
fn sync_incomplete_action_cannot_coexist_with_a_same_id_local_refresh() {
    let (data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT id,root_path,?1,'refresh','','completed',123 FROM repositories",[owner.operation_id().to_string()]).unwrap();
    assert!(
        matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&sync_target()),Err(error) if error.kind==RepositoryErrorKind::OperationMismatch)
    );
    assert!(
        matches!(service.restart_remote_synchronization(root.path(),owner.operation_id(),&sync_target()),Err(error) if error.kind==RepositoryErrorKind::OperationMismatch)
    );
}

#[test]
fn sync_context_authority_replay_rejects_a_different_context() {
    let (_data, root, service) = fixture();
    let target = RemoteOperationTarget::for_context(
        &plan(),
        RemoteOperationAction::SynchronizeContext,
        crate::repository::AuthoringKind::Ticket,
        "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    )
    .unwrap();
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    sync_push_prepared(&service, root.path(), &owner);
    let mut evidence = sync_evidence();
    evidence.push_advertised_oid = evidence.push_oid;
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::PushVerified,
            &evidence,
        )
        .unwrap();
    let authority = state::SynchronizationAuthority::Published(evidence.push_oid.unwrap());
    service
        .classify_synchronization(root.path(), &owner, authority)
        .unwrap();
    let different = RemoteOperationTarget::for_context(
        &plan(),
        RemoteOperationAction::SynchronizeContext,
        crate::repository::AuthoringKind::Document,
        "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    )
    .unwrap();
    assert!(
        matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&different),Err(error) if error.kind==RepositoryErrorKind::OperationMismatch)
    );
    assert!(
        service
            .finish_synchronization_index(root.path(), owner.operation_id(), &different)
            .is_err()
    );
    assert!(
        matches!(service.reserve_remote_operation(root.path(),owner.operation_id(),&target).unwrap(),RemoteReservationOutcome::Replay(record) if record.authority()==Some(authority))
    );
    service
        .finish_synchronization_index(root.path(), owner.operation_id(), &target)
        .unwrap();
}

#[test]
fn sync_terminal_slot_release_does_not_classify_or_authorize_a_new_action() {
    let (_data, root, service) = fixture();
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
        .unwrap();
    service
        .cancel_remote_operation(root.path(), owner.operation_id())
        .unwrap();
    assert_eq!(
        service
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
            .unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    assert!(
        matches!(service.restart_remote_synchronization(root.path(),owner.operation_id(),&sync_target()).unwrap(),RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Cancelled && record.sync_checkpoint()==Some(state::SynchronizationCheckpoint::PushPrepared) && record.sync_evidence().push_oid==sync_evidence().push_oid)
    );
    let new_owner = sync_owner(&service, root.path());
    let record = service
        .active_remote_operation(root.path())
        .unwrap()
        .unwrap();
    assert_eq!(record.operation_id(), new_owner.operation_id());
    assert_eq!(record.sync_checkpoint(), None);
    assert_eq!(record.authority(), None);
    assert!(
        service
            .checkpoint_synchronization(
                root.path(),
                &new_owner,
                state::SynchronizationCheckpoint::PushPrepared,
                &sync_evidence()
            )
            .is_err()
    );
}
