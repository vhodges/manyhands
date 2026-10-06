use manyhands::repository::{
    LeaseKind, OperationId, REGISTRY_FILE, RemoteOperationPhase, RemoteOperationSafePoint,
    RemoteOperationTarget, RemoteRefPlan, RemoteReservation, RemoteReservationOutcome,
    RemoteSafePointOutcome, RepositoryErrorKind, RepositoryService,
};
use rusqlite::Connection;

mod support;

fn configure(enabled: &support::EnabledRepository) {
    Connection::open(enabled.data_directory.path().join(REGISTRY_FILE))
        .unwrap()
        .execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main'")
        .unwrap();
}

fn poll() -> RemoteOperationTarget {
    RemoteOperationTarget::for_poll(&RemoteRefPlan::from_configuration("origin", "main").unwrap())
}

fn manual() -> RemoteOperationTarget {
    RemoteOperationTarget::for_primary_synchronization(
        &RemoteRefPlan::from_configuration("origin", "main").unwrap(),
    )
}

fn reserved(outcome: RemoteReservationOutcome) -> RemoteReservation {
    match outcome {
        RemoteReservationOutcome::Reserved(token) => token,
        other => panic!("expected reservation, got {other:?}"),
    }
}

// Catches premature release/stealing and non-durable yield requests.
#[test]
fn two_services_yield_poll_only_at_acknowledged_safe_point() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    configure(&enabled);
    let other = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let id = OperationId::new();
    let token = reserved(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, id, &poll())
            .unwrap(),
    );
    let manual_id = OperationId::new();
    assert!(matches!(
        other
            .reserve_remote_operation(&fixture.root, manual_id, &manual())
            .unwrap(),
        RemoteReservationOutcome::PollYielding
    ));
    let active = other
        .active_remote_operation(&fixture.root)
        .unwrap()
        .unwrap();
    assert_eq!(active.operation_id(), id);
    assert!(active.yield_requested());
    assert_eq!(
        enabled
            .service
            .remote_safe_point(
                &fixture.root,
                &token,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .unwrap(),
        RemoteSafePointOutcome::Interrupted
    );
    assert!(
        other
            .active_remote_operation(&fixture.root)
            .unwrap()
            .is_none()
    );
    reserved(
        other
            .reserve_remote_operation(&fixture.root, manual_id, &manual())
            .unwrap(),
    );
    assert!(matches!(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, OperationId::new(), &manual())
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    assert_eq!(
        enabled
            .service
            .remote_safe_point(
                &fixture.root,
                &token,
                RemoteOperationSafePoint::BeforeBatchCommit
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
}

// Catches granting a second executor on replay and mismatched ID reuse.
#[test]
fn stable_id_replays_without_a_second_owner_and_mismatch_is_fixed() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    configure(&enabled);
    let id = OperationId::new();
    reserved(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, id, &poll())
            .unwrap(),
    );
    assert!(
        matches!(enabled.service.reserve_remote_operation(&fixture.root, id, &poll()).unwrap(), RemoteReservationOutcome::Replay(record) if record.phase() == RemoteOperationPhase::Reserved)
    );
    let error = enabled
        .service
        .reserve_remote_operation(&fixture.root, id, &manual())
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
    assert!(!format!("{error:?} {error}").contains(fixture.root.to_str().unwrap()));
}

// Catches holding the repository lease or acknowledging cancel at request time.
#[test]
fn cancellation_waits_for_each_named_safe_point_without_git_lease() {
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
        RemoteOperationSafePoint::BetweenObservations,
        RemoteOperationSafePoint::BeforeBatchCommit,
        RemoteOperationSafePoint::AfterBatchCommit,
        RemoteOperationSafePoint::BeforeLocalMutation,
    ] {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        configure(&enabled);
        let before = support::repository_and_worktree_snapshot(&fixture);
        let id = OperationId::new();
        let token = reserved(
            enabled
                .service
                .reserve_remote_operation(&fixture.root, id, &poll())
                .unwrap(),
        );
        let lease = RepositoryService::hold_lease_for_testing(
            &fixture.root,
            enabled.data_directory.path(),
            LeaseKind::Repository,
        )
        .unwrap();
        enabled
            .service
            .cancel_remote_operation(&fixture.root, id)
            .unwrap();
        assert!(
            enabled
                .service
                .active_remote_operation(&fixture.root)
                .unwrap()
                .unwrap()
                .cancel_requested()
        );
        assert_eq!(
            enabled
                .service
                .remote_safe_point(&fixture.root, &token, point)
                .unwrap(),
            RemoteSafePointOutcome::Cancelled
        );
        assert!(
            enabled
                .service
                .active_remote_operation(&fixture.root)
                .unwrap()
                .is_none()
        );
        drop(lease);
        assert_eq!(before, support::repository_and_worktree_snapshot(&fixture));
    }
}

// Catches local recovery being replaced or bypassed by a remote reservation.
#[test]
fn legacy_local_recovery_blocks_remote_without_changing_its_record() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    configure(&enabled);
    let connection = Connection::open(enabled.data_directory.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO operation_records(repository_id,root_path,action,state,observed_at) SELECT id,root_path,'refresh','failed',123 FROM repositories", []).unwrap();
    assert_eq!(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, OperationId::new(), &poll())
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM operation_records WHERE action='refresh' AND state='failed'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
}

// Catches races where two immediate transactions both claim an idle repository.
#[test]
fn simultaneous_manual_requests_have_exactly_one_owner() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    configure(&enabled);
    let other = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let barrier = std::sync::Barrier::new(2);
    let outcomes = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            enabled
                .service
                .reserve_remote_operation(&fixture.root, OperationId::new(), &manual())
                .unwrap()
        });
        let second = scope.spawn(|| {
            barrier.wait();
            other
                .reserve_remote_operation(&fixture.root, OperationId::new(), &manual())
                .unwrap()
        });
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(o, RemoteReservationOutcome::Reserved(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(o, RemoteReservationOutcome::Busy))
            .count(),
        1
    );
}

// Catches stale owners publishing after an explicit restart, or replay silently restarting work.
#[test]
fn restart_fences_old_poll_owner_and_refuses_manual_or_mutation_work() {
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
        RemoteOperationSafePoint::BetweenObservations,
        RemoteOperationSafePoint::BeforeBatchCommit,
    ] {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        configure(&enabled);
        let id = OperationId::new();
        let old = reserved(
            enabled
                .service
                .reserve_remote_operation(&fixture.root, id, &poll())
                .unwrap(),
        );
        enabled
            .service
            .remote_safe_point(&fixture.root, &old, point)
            .unwrap();
        let reopened = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
        assert_eq!(
            reopened
                .active_remote_operation(&fixture.root)
                .unwrap()
                .unwrap()
                .completed_step(),
            Some(point)
        );
        let new = reserved(
            reopened
                .restart_remote_observation(&fixture.root, id, &poll())
                .unwrap(),
        );
        assert_eq!(
            enabled
                .service
                .remote_safe_point(&fixture.root, &old, point)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::RecoveryRequired
        );
        assert_eq!(
            reopened
                .remote_safe_point(
                    &fixture.root,
                    &new,
                    RemoteOperationSafePoint::BeforeTransport
                )
                .unwrap(),
            RemoteSafePointOutcome::Continue
        );
    }
    for target in [poll(), manual()] {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        configure(&enabled);
        let id = OperationId::new();
        let token = reserved(
            enabled
                .service
                .reserve_remote_operation(&fixture.root, id, &target)
                .unwrap(),
        );
        enabled
            .service
            .remote_safe_point(
                &fixture.root,
                &token,
                RemoteOperationSafePoint::BeforeLocalMutation,
            )
            .unwrap();
        assert_eq!(
            enabled
                .service
                .restart_remote_observation(&fixture.root, id, &target)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::RecoveryRequired
        );
    }
}

// Catches a new local lifecycle mutation racing an existing remote reservation.
#[test]
fn remote_reservation_blocks_new_local_lifecycle_without_taking_git_lease() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    configure(&enabled);
    reserved(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, OperationId::new(), &poll())
            .unwrap(),
    );
    let before = support::repository_and_worktree_snapshot(&fixture);
    let error = enabled
        .service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
            name: "blocked".into(),
            url: "https://example.invalid/repo".into(),
        })
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert_eq!(before, support::repository_and_worktree_snapshot(&fixture));
}

// Catches treating explicit observation as background work, and changing priority on replay.
#[test]
fn explicit_observation_has_manual_priority_and_yields_background_poll() {
    use manyhands::repository::RemoteOperationPriority;
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    configure(&enabled);
    let token = reserved(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, OperationId::new(), &poll())
            .unwrap(),
    );
    let manual_id = OperationId::new();
    assert!(matches!(
        enabled
            .service
            .reserve_remote_operation_with_priority(
                &fixture.root,
                manual_id,
                &poll(),
                RemoteOperationPriority::Manual
            )
            .unwrap(),
        RemoteReservationOutcome::PollYielding
    ));
    enabled
        .service
        .remote_safe_point(
            &fixture.root,
            &token,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    reserved(
        enabled
            .service
            .reserve_remote_operation_with_priority(
                &fixture.root,
                manual_id,
                &poll(),
                RemoteOperationPriority::Manual,
            )
            .unwrap(),
    );
    assert!(matches!(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, OperationId::new(), &poll())
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    assert_eq!(
        enabled
            .service
            .reserve_remote_operation(&fixture.root, manual_id, &poll())
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
}

// Catches accepting a token against coincident IDs in another registry/service.
#[test]
fn reservation_tokens_are_bound_to_the_issuing_service_without_paths() {
    let first_fixture = support::born_repository();
    let first = support::enabled_repository(&first_fixture);
    configure(&first);
    let second_fixture = support::born_repository();
    let second = support::enabled_repository(&second_fixture);
    configure(&second);
    let id = OperationId::new();
    let first_token = reserved(
        first
            .service
            .reserve_remote_operation(&first_fixture.root, id, &poll())
            .unwrap(),
    );
    let second_token = reserved(
        second
            .service
            .reserve_remote_operation(&second_fixture.root, id, &poll())
            .unwrap(),
    );
    assert_eq!(
        second
            .service
            .remote_safe_point(
                &second_fixture.root,
                &first_token,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    let reopened = RepositoryService::open_at(first.data_directory.path()).unwrap();
    assert_eq!(
        reopened
            .remote_safe_point(
                &first_fixture.root,
                &first_token,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(
        first
            .service
            .remote_safe_point(
                &first_fixture.root,
                &first_token,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(
        second
            .service
            .remote_safe_point(
                &second_fixture.root,
                &second_token,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    let debug = format!("{first_token:?}");
    assert!(!debug.contains(first.data_directory.path().to_str().unwrap()));
    assert!(!debug.contains(first_fixture.root.to_str().unwrap()));
}
