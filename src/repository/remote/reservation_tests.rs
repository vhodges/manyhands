use super::*;

fn promote_target() -> RemoteOperationTarget {
    RemoteOperationTarget::for_context(
        &plan(),
        RemoteOperationAction::Promote,
        crate::repository::AuthoringKind::Document,
        "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    )
    .unwrap()
}
#[test]
fn promotion_manual_reservation_holds_across_cleanup_and_only_final_sql_finishes() {
    let (_data, root, service) = fixture();
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &promote_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
    ] {
        assert_eq!(
            service
                .remote_safe_point(root.path(), &owner, point)
                .unwrap(),
            RemoteSafePointOutcome::Continue
        );
    }
    assert!(
        service
            .finish_remote_operation(root.path(), &owner, RemoteOutcomeCategory::Completed)
            .is_err()
    );
    assert!(matches!(
        service
            .reserve_remote_operation(
                root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    // A local cleanup checkpoint does not release or rewrite the remote owner.
    for _ in 0..3 {
        state::with_transaction(&service, root.path(), |tx, id| {
            let r = owned(&service, tx, id, &owner)?;
            assert_eq!(r.target.action(), RemoteOperationAction::Promote);
            Ok(())
        })
        .unwrap();
    }
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_some()
    );
}
#[test]
fn promotion_cancellation_keeps_the_slot_and_does_not_write_poll_policy() {
    let (data, root, service) = fixture();
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &promote_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    service
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch(
        "UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=1234",
    )
    .unwrap();
    service
        .cancel_remote_operation(root.path(), owner.operation_id())
        .unwrap();
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        service
            .reserve_remote_operation(
                root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    let policy: (Option<String>, Option<i64>, Option<i64>) = db
        .query_row(
            "SELECT latest_outcome,latest_attempt_at,next_eligible_at FROM remote_polling_state",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(policy, (None, None, Some(1234)));
}
#[test]
fn promotion_restart_fences_old_epoch_preserves_step_and_poll_policy() {
    let (data, root, service) = fixture();
    let op = OperationId::new();
    let owner = match service
        .reserve_remote_operation(root.path(), op, &promote_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    service
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("UPDATE remote_operation_records SET phase='interrupted'; UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=1234").unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let new = match reopened
        .restart_remote_promotion(root.path(), op, &promote_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    assert!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .is_err()
    );
    assert_eq!(new.epoch, owner.epoch + 1);
    reopened
        .remote_safe_point(
            root.path(),
            &new,
            RemoteOperationSafePoint::AfterAdvertisement,
        )
        .unwrap();
    assert_eq!(
        db.query_row(
            "SELECT next_eligible_at FROM remote_polling_state",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1234
    );
}
#[test]
fn promotion_yields_running_poll_and_is_busy_against_sync() {
    for poll in [true, false] {
        let (_data, root, service) = fixture();
        let target = if poll {
            RemoteOperationTarget::for_poll(&plan())
        } else {
            RemoteOperationTarget::for_primary_synchronization(&plan())
        };
        let owner = match service
            .reserve_remote_operation(root.path(), OperationId::new(), &target)
            .unwrap()
        {
            RemoteReservationOutcome::Reserved(o) => o,
            other => panic!("{other:?}"),
        };
        let result = service
            .reserve_remote_operation(root.path(), OperationId::new(), &promote_target())
            .unwrap();
        if poll {
            assert!(matches!(result, RemoteReservationOutcome::PollYielding));
            assert_eq!(
                service
                    .remote_safe_point(
                        root.path(),
                        &owner,
                        RemoteOperationSafePoint::BeforeTransport
                    )
                    .unwrap(),
                RemoteSafePointOutcome::Interrupted
            );
        } else {
            assert!(matches!(result, RemoteReservationOutcome::Busy));
        }
    }
}

#[test]
fn promotion_child_ownership_dominates_interruption_and_authority_never_reexecutes() {
    use crate::repository::promotion::record::{self, ChildRole, ChildState};
    use crate::repository::{AuthoringKind, PromotionExpectation};
    for primary in [false, true] {
        for mode in ["absent", "candidate", "conflict", "complete", "uncertain"] {
            let (data, root, service) = fixture();
            let role = if primary {
                ChildRole::Primary
            } else {
                ChildRole::Context
            };
            let parent = state::with_transaction(&service, root.path(), |tx, id| {
                let mut r = record::begin_record(
                    tx,
                    id,
                    OperationId::new(),
                    &"01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
                    PromotionExpectation::from_bytes([7; 32]),
                    Some("origin"),
                )?;
                if primary {
                    r.context_oid = sync_evidence().expected_oid;
                    r.step = record::PromotionStep::ContextSynchronized;
                    record::save_record(tx, id, &r)?;
                    r.base_oid = r.context_oid;
                    r.integration_kind = Some(record::IntegrationKind::Merged);
                    r.step = record::PromotionStep::Integrating;
                    record::save_record(tx, id, &r)?;
                    r.integration_oid = sync_evidence().local_oid;
                    r.step = record::PromotionStep::Integrated;
                    record::save_record(tx, id, &r)?;
                }
                record::next_attempt(tx, id, r.operation_id, role)
            })
            .unwrap();
            let child = if primary {
                parent.primary_sync_id.unwrap()
            } else {
                parent.context_sync_id.unwrap()
            };
            let target = if primary {
                sync_target()
            } else {
                RemoteOperationTarget::for_context(
                    &plan(),
                    RemoteOperationAction::SynchronizeContext,
                    AuthoringKind::Document,
                    parent.item_id.clone(),
                )
                .unwrap()
            };
            if mode == "absent" {
                state::with_transaction(&service, root.path(), |tx, _| {
                    assert!(matches!(
                        record::child_state(tx, &parent, role)?,
                        ChildState::Absent
                    ));
                    Ok(())
                })
                .unwrap();
                continue;
            }
            let owner = match service
                .reserve_remote_operation(root.path(), child, &target)
                .unwrap()
            {
                RemoteReservationOutcome::Reserved(o) => o,
                other => panic!("{other:?}"),
            };
            if mode == "complete" {
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
                service
                    .classify_synchronization(
                        root.path(),
                        &owner,
                        state::SynchronizationAuthority::Published(evidence.push_oid.unwrap()),
                    )
                    .unwrap();
            } else if mode != "uncertain" {
                service
                    .prepare_synchronization_integration(
                        root.path(),
                        &owner,
                        &state::IntegrationStepIntent {
                            ordinal: 0,
                            stage: if primary {
                                super::super::merge::IntegrationStage::Primary
                            } else {
                                super::super::merge::IntegrationStage::Context
                            },
                            local_oid: sync_evidence().expected_oid.unwrap(),
                            incoming_oid: sync_evidence().local_oid.unwrap(),
                            baseline_tree_oid: sync_evidence().primary_tracking_oid.unwrap(),
                            baseline_index_digest: [7; 32],
                        },
                    )
                    .unwrap();
                service
                    .begin_synchronization_integration_effect(
                        root.path(),
                        &owner,
                        0,
                        if mode == "candidate" {
                            sync_evidence().push_oid
                        } else {
                            None
                        },
                    )
                    .unwrap();
                if mode == "conflict" {
                    service
                        .release_synchronization_conflict(root.path(), &owner, 0, [7; 32])
                        .unwrap();
                }
                Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute("UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1",[child.to_string()]).unwrap();
            } else {
                Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute_batch("PRAGMA ignore_check_constraints=ON;UPDATE remote_operation_records SET owner_epoch=-1").unwrap();
            }
            state::with_transaction(&service, root.path(), |tx, id| {
                let state = record::child_state(tx, &parent, role);
                match mode {
                    "complete" => assert!(matches!(state?, ChildState::Complete(_))),
                    "uncertain" => assert!(state.is_err()),
                    _ => assert!(matches!(state?, ChildState::Retained(_))),
                }
                assert!(record::next_attempt(tx, id, parent.operation_id, role).is_err());
                let actual = record::read_record(tx, id, parent.operation_id)?.unwrap();
                assert_eq!(actual.context_sync_id, parent.context_sync_id);
                assert_eq!(actual.primary_sync_id, parent.primary_sync_id);
                Ok(())
            })
            .unwrap();
        }
    }
}

#[test]
fn poll_progress_probe_is_read_only_and_fences_flags_and_owner_epoch() {
    let (data, root, service) = poll_fixture();
    let owner = match service
        .reserve_remote_poll(
            root.path(),
            OperationId::new(),
            &RemoteOperationTarget::for_poll(&plan()),
            RemoteOperationPriority::Poll,
        )
        .unwrap()
    {
        RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(o)) => o,
        other => panic!("{other:?}"),
    };
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("CREATE TABLE probe_writes(n INTEGER);INSERT INTO probe_writes VALUES(0);CREATE TRIGGER probe_update AFTER UPDATE ON remote_operation_records BEGIN UPDATE probe_writes SET n=n+1;END;").unwrap();
    assert_eq!(
        poll_progress(&service, &owner).unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(
        db.query_row("SELECT n FROM probe_writes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.execute_batch("UPDATE remote_operation_records SET yield_requested=1")
        .unwrap();
    assert_eq!(
        poll_progress(&service, &owner).unwrap(),
        RemoteSafePointOutcome::Interrupted
    );
    db.execute_batch("UPDATE remote_operation_records SET cancel_requested=1")
        .unwrap();
    assert_eq!(
        poll_progress(&service, &owner).unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    let count = db
        .query_row("SELECT n FROM probe_writes", [], |r| r.get::<_, i64>(0))
        .unwrap();
    assert_eq!(
        poll_progress(&service, &owner).unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    assert_eq!(
        db.query_row("SELECT n FROM probe_writes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        count
    );
    let other = RepositoryService::open_at(data.path()).unwrap();
    assert!(poll_progress(&other, &owner).is_err());
    db.execute_batch("UPDATE remote_operation_records SET cancel_requested=0,yield_requested=0,owner_epoch=owner_epoch+1").unwrap();
    assert_eq!(
        poll_progress(&service, &owner).unwrap(),
        RemoteSafePointOutcome::Interrupted
    );
}
use crate::repository::{
    REGISTRY_FILE,
    remote::{RemoteRefObservation, RemoteRefPlan},
};
use rusqlite::Connection;

fn poll_fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let result = fixture();
    let repo = git2::Repository::init(result.1.path()).unwrap();
    let tree = repo
        .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
        .unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.test").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[])
        .unwrap();
    result
}

fn update_owner(service: &RepositoryService, root: &Path) -> RemoteReservation {
    match service
        .reserve_remote_poll(
            root,
            OperationId::new(),
            &RemoteOperationTarget::for_poll(&plan()),
            RemoteOperationPriority::Poll,
        )
        .unwrap()
    {
        RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(owner)) => {
            owner
        }
        other => panic!("{other:?}"),
    }
}

fn update_points() -> [RemoteOperationSafePoint; 8] {
    use RemoteOperationSafePoint::*;
    [
        BeforeTransport,
        AfterAdvertisement,
        BetweenObservations,
        BeforeBatchCommit,
        AfterBatchCommit,
        BeforeLocalMutation,
        BeforeLocalMutation,
        BeforeDiscovery,
    ]
}

fn advance_update(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    count: usize,
) {
    for point in update_points().into_iter().take(count) {
        let result = if point == RemoteOperationSafePoint::AfterBatchCommit {
            commit_observation_batch(service, root, owner, &plan(), &[], 123)
        } else {
            service.remote_safe_point(root, owner, point)
        };
        assert_eq!(result.unwrap(), RemoteSafePointOutcome::Continue);
    }
}

#[test]
fn update_local_stage_is_ordered_repeatable_active_and_completion_requires_discovery() {
    use RemoteOperationSafePoint::*;
    let (_data, root, service) = poll_fixture();
    let owner = update_owner(&service, root.path());
    for point in [BeforeLocalMutation, BeforeDiscovery] {
        assert_eq!(
            service
                .remote_safe_point(root.path(), &owner, point)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::RecoveryRequired
        );
    }
    advance_update(&service, root.path(), &owner, 5);
    assert_eq!(
        service
            .finish_remote_operation(root.path(), &owner, RemoteOutcomeCategory::Completed)
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    for _ in 0..2 {
        assert_eq!(
            service
                .remote_safe_point(root.path(), &owner, BeforeLocalMutation)
                .unwrap(),
            RemoteSafePointOutcome::Continue
        );
        assert_eq!(
            service
                .active_remote_operation(root.path())
                .unwrap()
                .unwrap()
                .phase(),
            RemoteOperationPhase::LocalPrepared
        );
        assert!(matches!(
            service
                .reserve_remote_poll(
                    root.path(),
                    OperationId::new(),
                    &RemoteOperationTarget::for_poll(&plan()),
                    RemoteOperationPriority::Poll
                )
                .unwrap(),
            RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Busy)
        ));
    }
    assert_eq!(
        service
            .finish_remote_operation(root.path(), &owner, RemoteOutcomeCategory::Completed)
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(
        service
            .remote_safe_point(root.path(), &owner, BeforeBatchCommit)
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    service
        .remote_safe_point(root.path(), &owner, BeforeDiscovery)
        .unwrap();
    for point in [BeforeDiscovery, BeforeLocalMutation, AfterAdvertisement] {
        assert_eq!(
            service
                .remote_safe_point(root.path(), &owner, point)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::RecoveryRequired
        );
    }
    assert_eq!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap()
            .phase(),
        RemoteOperationPhase::LocalPrepared
    );
    service
        .finish_remote_operation(root.path(), &owner, RemoteOutcomeCategory::Completed)
        .unwrap();
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
}

#[test]
fn update_requests_at_every_boundary_release_slot_preserve_policy_and_fence_owner() {
    for cancel in [false, true] {
        for boundary in 0..=update_points().len() {
            let (data, root, service) = poll_fixture();
            let owner = update_owner(&service, root.path());
            let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            db.execute_batch("UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=1234").unwrap();
            let before_batch_commit = boundary == update_points().len();
            let count = if before_batch_commit {
                4
            } else if boundary == 4 {
                5
            } else {
                boundary
            };
            advance_update(&service, root.path(), &owner, count);
            if cancel {
                service
                    .cancel_remote_operation(root.path(), owner.operation_id())
                    .unwrap();
            } else {
                assert!(matches!(
                    service
                        .reserve_remote_operation(
                            root.path(),
                            OperationId::new(),
                            &RemoteOperationTarget::for_primary_synchronization(&plan())
                        )
                        .unwrap(),
                    RemoteReservationOutcome::PollYielding
                ));
            }
            let point = if before_batch_commit {
                RemoteOperationSafePoint::BeforeBatchCommit
            } else {
                update_points()[boundary]
            };
            let result = if before_batch_commit {
                commit_observation_batch(&service, root.path(), &owner, &plan(), &[], 124)
            } else {
                service.remote_safe_point(root.path(), &owner, point)
            };
            assert_eq!(
                result.unwrap(),
                if cancel {
                    RemoteSafePointOutcome::Cancelled
                } else {
                    RemoteSafePointOutcome::Interrupted
                }
            );
            assert!(
                service
                    .active_remote_operation(root.path())
                    .unwrap()
                    .is_none()
            );
            let snapshot = service.remote_snapshot(root.path()).unwrap();
            assert_eq!(
                snapshot.polling().automatic_backoff().unwrap().as_secs(),
                120
            );
            assert_eq!(snapshot.next_eligible_at().unwrap().unix_timestamp(), 1234);
            assert_eq!(snapshot.latest_attempt_at().is_some(), cancel);
            assert_eq!(
                service
                    .remote_safe_point(root.path(), &owner, point)
                    .unwrap_err()
                    .kind,
                RepositoryErrorKind::RecoveryRequired
            );
        }
    }
}

#[test]
fn update_restart_from_every_abandoned_or_interrupted_stage_reobserves_with_new_epoch() {
    for interrupted in [false, true] {
        for count in 0..=update_points().len() {
            let (data, root, service) = poll_fixture();
            let owner = update_owner(&service, root.path());
            advance_update(&service, root.path(), &owner, count);
            if interrupted {
                Connection::open(data.path().join(REGISTRY_FILE))
                    .unwrap()
                    .execute_batch(
                        "UPDATE remote_operation_records SET phase='interrupted',yield_requested=1",
                    )
                    .unwrap();
            }
            let reopened = RepositoryService::open_at(data.path()).unwrap();
            let restarted = match reopened
                .restart_remote_poll(
                    root.path(),
                    owner.operation_id(),
                    &RemoteOperationTarget::for_poll(&plan()),
                )
                .unwrap()
            {
                RemoteReservationOutcome::Reserved(owner) => owner,
                other => panic!("{other:?}"),
            };
            assert_eq!(restarted.epoch, owner.epoch + 1);
            let inspection = reopened
                .active_remote_operation(root.path())
                .unwrap()
                .unwrap();
            assert_eq!(inspection.phase(), RemoteOperationPhase::Reserved);
            assert_eq!(inspection.completed_step(), None);
            assert!(!inspection.yield_requested());
            assert_eq!(
                service
                    .remote_safe_point(
                        root.path(),
                        &owner,
                        RemoteOperationSafePoint::BeforeTransport
                    )
                    .unwrap_err()
                    .kind,
                RepositoryErrorKind::RecoveryRequired
            );
            advance_update(&reopened, root.path(), &restarted, 8);
        }
    }
}

#[test]
fn poll_kind_is_stored_identity_and_terminal_update_replays() {
    for cancelled in [false, true] {
        let (data, root, service) = poll_fixture();
        let owner = update_owner(&service, root.path());
        advance_update(&service, root.path(), &owner, 8);
        service
            .finish_remote_operation(
                root.path(),
                &owner,
                if cancelled {
                    RemoteOutcomeCategory::Cancelled
                } else {
                    RemoteOutcomeCategory::Completed
                },
            )
            .unwrap();
        let reopened = RepositoryService::open_at(data.path()).unwrap();
        let target = RemoteOperationTarget::for_poll(&plan());
        assert!(matches!(
            reopened
                .restart_remote_poll(root.path(), owner.operation_id(), &target)
                .unwrap(),
            RemoteReservationOutcome::Replay(_)
        ));
        assert!(matches!(
            reopened
                .reserve_remote_poll(
                    root.path(),
                    owner.operation_id(),
                    &target,
                    RemoteOperationPriority::Poll
                )
                .unwrap(),
            RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Replay(_))
        ));
        assert_eq!(
            reopened
                .restart_remote_observation(root.path(), owner.operation_id(), &target)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::OperationMismatch
        );
        assert_eq!(
            reopened
                .reserve_remote_operation(root.path(), owner.operation_id(), &target)
                .unwrap_err()
                .kind,
            RepositoryErrorKind::OperationMismatch
        );
    }
    let (_data, root, service) = poll_fixture();
    let observe = reserve(&service, root.path());
    assert_eq!(
        service
            .reserve_remote_poll(
                root.path(),
                observe.operation_id(),
                &RemoteOperationTarget::for_poll(&plan()),
                RemoteOperationPriority::Poll
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    assert_eq!(
        service
            .restart_remote_poll(
                root.path(),
                observe.operation_id(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
}

#[test]
fn observe_legacy_migration_freezes_discovery_completion_and_restart() {
    let (data, root, service) = poll_fixture();
    let owner = reserve(&service, root.path());
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    // Remove just the additive column, representing a pre-Cycle-08 envelope.
    db.execute_batch("DROP TRIGGER IF EXISTS remote_poll_kind_immutable; ALTER TABLE remote_operation_records DROP COLUMN poll_kind").unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT poll_kind FROM remote_operation_records",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "observe"
    );
    let owner = match reopened
        .restart_remote_observation(
            root.path(),
            owner.operation_id(),
            &RemoteOperationTarget::for_poll(&plan()),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    reopened
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::BeforeBatchCommit,
        )
        .unwrap();
    assert_eq!(
        reopened
            .finish_remote_operation(root.path(), &owner, RemoteOutcomeCategory::Completed)
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    commit_observation_batch(&reopened, root.path(), &owner, &plan(), &[], 123).unwrap();
    assert_eq!(
        reopened
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::BeforeDiscovery
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert!(
        matches!(reopened.restart_remote_observation(root.path(), owner.operation_id(), &RemoteOperationTarget::for_poll(&plan())).unwrap(), RemoteReservationOutcome::Replay(record) if record.phase()==RemoteOperationPhase::Completed)
    );
}

#[test]
fn finish_updates_attempt_deadline_and_phase_atomically_from_pre_finish_policy() {
    use RemoteOutcomeCategory::*;
    for priority in [
        RemoteOperationPriority::Poll,
        RemoteOperationPriority::Manual,
    ] {
        for category in [
            Completed,
            ConfigurationRequired,
            SelectedKeyUnavailable,
            UnlockRequired,
            HostApprovalRequired,
            TransportUnavailable,
            ProtocolRejected,
            Cancelled,
            RepositoryUnavailable,
        ] {
            let (data, root, service) = poll_fixture();
            let owner = match service
                .reserve_remote_poll(
                    root.path(),
                    OperationId::new(),
                    &RemoteOperationTarget::for_poll(&plan()),
                    priority,
                )
                .unwrap()
            {
                RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(
                    owner,
                )) => owner,
                other => panic!("{other:?}"),
            };
            let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            db.execute_batch("UPDATE remote_polling_state SET interval_seconds=300,automatic_backoff_seconds=120,latest_attempt_at=100,next_eligible_at=1234").unwrap();
            let policy_before_batch = service.remote_snapshot(root.path()).unwrap();
            advance_update(&service, root.path(), &owner, 8);
            let policy_after_batch = service.remote_snapshot(root.path()).unwrap();
            assert_eq!(policy_before_batch.polling(), policy_after_batch.polling());
            assert_eq!(
                policy_before_batch.latest_outcome(),
                policy_after_batch.latest_outcome()
            );
            assert_eq!(
                policy_before_batch.latest_attempt_at(),
                policy_after_batch.latest_attempt_at()
            );
            assert_eq!(
                policy_before_batch.next_eligible_at(),
                policy_after_batch.next_eligible_at()
            );
            db.execute_batch("CREATE TRIGGER fail_finish BEFORE UPDATE OF phase ON remote_operation_records WHEN NEW.phase IN ('completed','failed','cancelled') BEGIN SELECT RAISE(ABORT,'fault'); END").unwrap();
            let before = service.remote_snapshot(root.path()).unwrap();
            assert_eq!(
                service
                    .finish_remote_operation(root.path(), &owner, category)
                    .unwrap_err()
                    .kind,
                RepositoryErrorKind::RecoveryRequired
            );
            assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
            assert_eq!(
                service
                    .active_remote_operation(root.path())
                    .unwrap()
                    .unwrap()
                    .phase(),
                RemoteOperationPhase::LocalPrepared
            );
            db.execute_batch("DROP TRIGGER fail_finish").unwrap();
            service
                .finish_remote_operation(root.path(), &owner, category)
                .unwrap();
            let snapshot = service.remote_snapshot(root.path()).unwrap();
            let attempt = snapshot.latest_attempt_at().unwrap().unix_timestamp();
            assert!(attempt >= 100);
            let delay = match category {
                Cancelled => None,
                Completed => Some(300),
                _ if priority == RemoteOperationPriority::Manual => None,
                TransportUnavailable | ProtocolRejected => Some(240),
                _ => Some(300),
            };
            assert_eq!(
                snapshot.next_eligible_at().unwrap().unix_timestamp(),
                delay.map_or(1234, |delay| attempt + delay)
            );
            assert_eq!(
                snapshot
                    .polling()
                    .automatic_backoff()
                    .map(|value| value.as_secs()),
                match category {
                    Completed => None,
                    TransportUnavailable | ProtocolRejected
                        if priority == RemoteOperationPriority::Poll =>
                        Some(240),
                    _ => Some(120),
                }
            );
            assert_eq!(snapshot.latest_outcome(), Some(category));
            assert!(
                service
                    .active_remote_operation(root.path())
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn pending_conflict_blocks_only_update_without_record_or_policy_change() {
    let (data, root, service) = poll_fixture();
    let sync = sync_owner(&service, root.path());
    let repo = git2::Repository::open(root.path()).unwrap();
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &sync,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: super::super::merge::IntegrationStage::Primary,
                local_oid: commit.id(),
                incoming_oid: commit.id(),
                baseline_tree_oid: commit.tree_id(),
                baseline_index_digest: [0; 32],
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &sync, 0, None)
        .unwrap();
    service
        .release_synchronization_conflict(root.path(), &sync, 0, [1; 32])
        .unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let rows = || {
        db.query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap()
    };
    assert_eq!(rows(), 1);
    let result = service
        .reserve_remote_poll(
            root.path(),
            OperationId::new(),
            &RemoteOperationTarget::for_poll(&plan()),
            RemoteOperationPriority::Poll,
        )
        .unwrap();
    assert!(matches!(
        result,
        RemotePollReservationOutcome::ConflictPending
    ));
    assert_eq!(rows(), 1);
    assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
    reserve(&service, root.path());
}

#[test]
fn update_local_prepared_journal_guard_yields_without_creating_local_rows() {
    for priority in [
        RemoteOperationPriority::Poll,
        RemoteOperationPriority::Manual,
    ] {
        for action in [
            RepositoryOperation::SaveDocument,
            RepositoryOperation::PrepareContext,
            RepositoryOperation::SubmitComment,
        ] {
            let (data, root, service) = poll_fixture();
            let owner = match service
                .reserve_remote_poll(
                    root.path(),
                    OperationId::new(),
                    &RemoteOperationTarget::for_poll(&plan()),
                    priority,
                )
                .unwrap()
            {
                RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(
                    owner,
                )) => owner,
                other => panic!("{other:?}"),
            };
            advance_update(&service, root.path(), &owner, 6);
            let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            let local_id = OperationId::new();
            let error = crate::repository::recovery::begin_or_reconcile_operation(
                &mut db,
                root.path(),
                action,
                local_id,
                "local-fixture",
            )
            .err()
            .expect("active poll must reject local journal begin");
            assert_eq!(
                error.kind,
                if priority == RemoteOperationPriority::Poll {
                    RepositoryErrorKind::RepositoryBusy
                } else {
                    RepositoryErrorKind::RecoveryRequired
                }
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM operation_records", [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                service
                    .active_remote_operation(root.path())
                    .unwrap()
                    .unwrap()
                    .yield_requested(),
                priority == RemoteOperationPriority::Poll
            );
            if priority == RemoteOperationPriority::Manual {
                service
                    .cancel_remote_operation(root.path(), owner.operation_id())
                    .unwrap();
            }
            service
                .remote_safe_point(
                    root.path(),
                    &owner,
                    RemoteOperationSafePoint::BeforeLocalMutation,
                )
                .unwrap();
            crate::repository::recovery::begin_or_reconcile_operation(
                &mut db,
                root.path(),
                action,
                local_id,
                "local-fixture",
            )
            .unwrap();
        }
    }
}

#[test]
fn cancellation_attempt_writer_fault_rolls_back_phase_outcome_and_retains_request() {
    let (data, root, service) = poll_fixture();
    let owner = update_owner(&service, root.path());
    advance_update(&service, root.path(), &owner, 6);
    service
        .cancel_remote_operation(root.path(), owner.operation_id())
        .unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("CREATE TRIGGER fail_attempt BEFORE UPDATE OF latest_attempt_at ON remote_polling_state BEGIN SELECT RAISE(ABORT,'fault'); END").unwrap();
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::BeforeLocalMutation
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(before, service.remote_snapshot(root.path()).unwrap());
    let active = service
        .active_remote_operation(root.path())
        .unwrap()
        .unwrap();
    assert_eq!(active.phase(), RemoteOperationPhase::LocalPrepared);
    assert!(active.cancel_requested());
    db.execute_batch("DROP TRIGGER fail_attempt").unwrap();
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::BeforeLocalMutation
            )
            .unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    assert!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .latest_attempt_at()
            .is_some()
    );
}

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    fixture_in(&std::env::temp_dir())
}

fn fixture_in(parent: &Path) -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    // Match canonical registry lookups, including Windows verbatim prefixes and
    // macOS /var aliases, before creating either fixture directory.
    let parent = parent.canonicalize().unwrap();
    let data = tempfile::tempdir_in(&parent).unwrap();
    let root = tempfile::tempdir_in(&parent).unwrap();
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

#[cfg(unix)]
#[test]
fn reservation_fixture_symlink_parent_reserves_and_reads_registered_root() {
    let temporary = tempfile::tempdir().unwrap();
    let parent = temporary.path().canonicalize().unwrap();
    let real = parent.join("real");
    let alias = parent.join("alias");
    std::fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    let (data, root, service) = fixture_in(&alias);

    let token = reserve(&service, root.path());
    assert_eq!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .unwrap()
            .operation_id(),
        token.operation_id()
    );
    assert_eq!(data.path(), data.path().canonicalize().unwrap());
    assert_eq!(root.path(), root.path().canonicalize().unwrap());
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
    let after = reopened.remote_snapshot(root.path()).unwrap();
    assert_eq!(before.observations(), after.observations());
    assert_eq!(before.contexts(), after.contexts());
    assert_eq!(before.polling(), after.polling());
    assert_eq!(before.latest_outcome(), after.latest_outcome());
    let absent = "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap();
    assert_eq!(
        before.publication_evidence_for(crate::repository::AuthoringKind::Document, &absent),
        after.publication_evidence_for(crate::repository::AuthoringKind::Document, &absent)
    );
    assert_eq!(
        after.next_eligible_at().unwrap().unix_timestamp(),
        after.latest_attempt_at().unwrap().unix_timestamp() + 300
    );
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
    let after = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(before.observations(), after.observations());
    assert_eq!(before.contexts(), after.contexts());
    assert_eq!(before.polling(), after.polling());
    assert_eq!(before.latest_outcome(), after.latest_outcome());
    let absent = "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap();
    assert_eq!(
        before.publication_evidence_for(crate::repository::AuthoringKind::Document, &absent),
        after.publication_evidence_for(crate::repository::AuthoringKind::Document, &absent)
    );
    assert_eq!(
        after.next_eligible_at().unwrap().unix_timestamp(),
        after.latest_attempt_at().unwrap().unix_timestamp() + 300
    );
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

#[test]
fn window_intent_and_local_effect_boundaries_fence_stale_services_and_cancellation() {
    let (data, root, service) = fixture();
    let old_owner = sync_owner(&service, root.path());
    sync_fetch(&service, root.path(), &old_owner);
    let primary = git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap();
    let local = git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap();
    let intent = state::with_transaction(&service, root.path(), |tx, id| {
        let record = state::read_operation(tx, id, old_owner.operation_id())?.unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &plan(),
            "refs/heads/main",
            primary,
            Some(primary),
        )
        .unwrap();
        let batch = state::complete_batch(tx, id, &plan(), record.generation, &[observation], 124)?;
        Ok(state::IntegrationWindowIntent {
            observation_batch_id: batch,
            local_oid: local,
            primary_oid: primary,
            context_oid: None,
        })
    })
    .unwrap();
    let second = RepositoryService::open_at(data.path()).unwrap();
    let owner = match second
        .restart_remote_synchronization(root.path(), old_owner.operation_id(), &sync_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    assert!(
        service
            .prepare_synchronization_window(root.path(), &old_owner, 1, &intent)
            .is_err()
    );
    second
        .prepare_synchronization_window(root.path(), &owner, 1, &intent)
        .unwrap();
    let step = state::IntegrationStepIntent {
        ordinal: 0,
        stage: super::super::merge::IntegrationStage::Primary,
        local_oid: local,
        incoming_oid: primary,
        baseline_tree_oid: local,
        baseline_index_digest: [7; 32],
    };
    second
        .prepare_synchronization_integration_in_window(root.path(), &owner, 1, &step)
        .unwrap();
    second
        .cancel_remote_operation(root.path(), owner.operation_id())
        .unwrap();
    assert!(
        second
            .begin_synchronization_integration_effect_in_window(root.path(), &owner, 1, 0, None)
            .is_err()
    );
    state::with_transaction(&second, root.path(), |tx, id| {
        let record = state::read_operation(tx, id, owner.operation_id())?.unwrap();
        assert!(record.cancel_requested);
        assert_eq!(
            state::integration_step_in_window(tx, record.id, 1, 0)?
                .unwrap()
                .phase,
            state::IntegrationStepPhase::Prepared
        );
        assert!(state::integration_step(tx, record.id, 0)?.is_none());
        Ok(())
    })
    .unwrap();
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
    // Finalization requires the restarted Fetch to have completed first.
    service
        .remote_safe_point(root.path(), &resumed, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    commit_observation_batch(
        &service,
        root.path(),
        &resumed,
        &plan(),
        &[observation()],
        124,
    )
    .unwrap();
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
    // The same operation may restart for offline inspection of its own
    // conflict; that token is fenced again by explicit reacquisition.
    let inspecting = match service
        .restart_remote_synchronization(root.path(), owner.operation_id(), &sync_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
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
            .applying_synchronization_candidate(root.path(), &inspecting)
            .is_err()
    );
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
        preflight_digest: [6; 32],
        identity_confirmation_id: Some(confirmation.confirmation_id),
        commit_time: (1_700_000_000, 0),
    };
    let path = state::ResolutionPathIntent {
        ordinal: 0,
        path_digest: [1; 32],
        expected_digest: [2; 32],
        result_digest: [3; 32],
        prewrite_digest: [4; 32],
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
    let artifact = state::ResolutionIndexArtifact {
        device: 1,
        inode: 2,
        sentinel_digest: [7; 32],
        baseline_digest: [8; 32],
        baseline_identity: (1, 3),
        metadata: [Some([9; 32]), None, Some([10; 32])],
        output: None,
        ref_phase: "not_started".into(),
        phase: "intent".into(),
    };
    assert!(
        service
            .prepare_synchronization_index_artifact(
                root.path(),
                &owner,
                attempt.attempt_id,
                &artifact
            )
            .is_err(),
        "old owner is fenced"
    );
    other
        .prepare_synchronization_index_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            &artifact,
        )
        .unwrap();
    other
        .prepare_synchronization_index_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            &artifact,
        )
        .unwrap();
    let changed_artifact = state::ResolutionIndexArtifact {
        inode: 4,
        ..artifact.clone()
    };
    assert!(
        other
            .prepare_synchronization_index_artifact(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                &changed_artifact
            )
            .is_err()
    );
    assert!(
        other
            .advance_synchronization_index_artifact(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                "release_intent"
            )
            .is_err()
    );
    other
        .advance_synchronization_index_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "published",
        )
        .unwrap();
    assert!(
        other
            .prepare_synchronization_index_output(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                (1, 5, [11; 32])
            )
            .is_err(),
        "candidate must precede install intent"
    );
    other
        .observe_synchronization_resolution_path_effect(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            0,
        )
        .unwrap();
    let ref_artifact = state::ResolutionRefLogArtifact {
        device: 1,
        inode: 17,
        digest: [13; 32],
    };
    assert!(
        other
            .prepare_synchronization_ref_log_artifact(
                root.path(),
                &owner,
                attempt.attempt_id,
                "baseline",
                &ref_artifact
            )
            .is_err(),
        "stale owner cannot bind proof"
    );
    other
        .prepare_synchronization_ref_log_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "baseline",
            &ref_artifact,
        )
        .unwrap();
    assert!(
        other
            .prepare_synchronization_ref_log_artifact(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                "transition",
                &ref_artifact
            )
            .is_err(),
        "candidate must precede result proof"
    );
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
        .prepare_synchronization_index_output(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            (1, 5, [11; 32]),
        )
        .unwrap();
    other
        .prepare_synchronization_index_output(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            (1, 5, [11; 32]),
        )
        .unwrap();
    assert!(
        other
            .prepare_synchronization_index_output(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                (1, 6, [11; 32])
            )
            .is_err()
    );
    assert!(
        other
            .advance_synchronization_resolution_ref_effect(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                "intent"
            )
            .is_err(),
        "result proof must precede invocation intent"
    );
    other
        .prepare_synchronization_ref_log_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "transition",
            &ref_artifact,
        )
        .unwrap();
    assert!(
        other
            .prepare_synchronization_ref_log_artifact(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                "transition",
                &state::ResolutionRefLogArtifact {
                    inode: 18,
                    ..ref_artifact.clone()
                }
            )
            .is_err()
    );
    other
        .advance_synchronization_resolution_ref_effect(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "intent",
        )
        .unwrap();
    // Repeating the durable marker is not permission to repeat a native effect:
    // the service additionally proves exact actual ref/log images under exclusion.
    other
        .advance_synchronization_resolution_ref_effect(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "intent",
        )
        .unwrap();
    other
        .advance_synchronization_resolution_ref_effect(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "observed",
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
    assert!(
        other
            .finalize_synchronization_resolution(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                checkpoint
            )
            .is_err(),
        "metadata/sentinel retirement must precede release"
    );
    other
        .advance_synchronization_index_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "release_intent",
        )
        .unwrap();
    other
        .advance_synchronization_index_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "released",
        )
        .unwrap();
    other
        .advance_synchronization_index_artifact(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            "released",
        )
        .unwrap();
    assert!(
        other
            .advance_synchronization_index_artifact(
                root.path(),
                &reacquired,
                attempt.attempt_id,
                "published"
            )
            .is_err()
    );
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
    assert!(
        connection
            .query_row("SELECT refresh_required FROM repositories", [], |row| row
                .get::<_, bool>(
                0
            ))
            .unwrap()
    );
    assert!(
        other
            .finalize_synchronization_resolution(
                root.path(),
                &owner,
                attempt.attempt_id,
                checkpoint
            )
            .is_err(),
        "stale owner is fenced"
    );
    assert!(
        other
            .finalize_synchronization_resolution(
                root.path(),
                &reacquired,
                OperationId::new(),
                checkpoint
            )
            .is_err(),
        "only the bound attempt can release ownership"
    );
    assert!(
        other
            .finalize_synchronization_resolution(root.path(), &reacquired, attempt.attempt_id, tree)
            .is_err(),
        "only the exact checkpoint can release ownership"
    );
    let before = state::with_transaction(&other, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    other
        .finalize_synchronization_resolution(
            root.path(),
            &reacquired,
            attempt.attempt_id,
            checkpoint,
        )
        .unwrap();
    assert!(
        other
            .active_remote_operation(root.path())
            .unwrap()
            .is_none()
    );
    let after = state::with_transaction(&other, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    assert_eq!(after.phase, RemoteOperationPhase::Interrupted);
    assert!(after.reconciliation_required);
    assert_eq!(after.sync_evidence, before.sync_evidence);
    assert_eq!(after.sync_checkpoint, before.sync_checkpoint);
    assert!(after.authority.is_none());
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
    sync_owner_target_root_generation_and_active_index_are_fenced_in(&std::env::temp_dir());
}

#[cfg(unix)]
#[test]
fn sync_fencing_fixture_symlink_secondary_parent_preserves_root_fencing() {
    let temporary = tempfile::tempdir().unwrap();
    let parent = temporary.path().canonicalize().unwrap();
    let real = parent.join("real");
    let alias = parent.join("alias");
    std::fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();

    sync_owner_target_root_generation_and_active_index_are_fenced_in(&alias);
}

fn sync_owner_target_root_generation_and_active_index_are_fenced_in(parent: &Path) {
    use state::SynchronizationCheckpoint as C;
    let (data, root, service) = fixture_in(parent);
    let owner = sync_owner(&service, root.path());
    sync_push_prepared(&service, root.path(), &owner);
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    // Like the primary fixture, register the canonical spelling of this root.
    let second = tempfile::tempdir_in(parent.canonicalize().unwrap()).unwrap();
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

// Cycle 10 Task 5: the `Close` arms. A closure parent holds its manual slot
// exactly as a promotion parent does, and only its final SQL finishes it.
const CLOSE_ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
fn close_item() -> crate::canonical::ItemId {
    CLOSE_ITEM.parse().unwrap()
}
fn close_target() -> RemoteOperationTarget {
    RemoteOperationTarget::for_context(
        &plan(),
        RemoteOperationAction::Close,
        crate::repository::AuthoringKind::Ticket,
        close_item(),
    )
    .unwrap()
}
fn close_oid(digit: char) -> git2::Oid {
    git2::Oid::from_str(&digit.to_string().repeat(40)).unwrap()
}
/// A `record` closure parent with the fixture's remote, at `confirmed`.
fn close_parent(
    service: &RepositoryService,
    root: &Path,
) -> crate::repository::closure::record::ClosureRecord {
    use crate::repository::closure::{ClosureExpectation, record as closure};
    state::with_transaction(service, root, |tx, id| {
        closure::begin_record(
            tx,
            id,
            OperationId::new(),
            &close_item(),
            ClosureExpectation::from_bytes([7; 32]),
            Some("origin"),
            closure::ClosureKind::Record,
        )
    })
    .unwrap()
}
/// Walks a closure parent through its checkpoint to `branch_deleted`. The
/// pre-closure context is `1…`, the base `2…`, the checkpoint `3…`.
fn close_parent_at_branch_deleted(
    service: &RepositoryService,
    root: &Path,
) -> crate::repository::closure::record::ClosureRecord {
    use crate::repository::{
        closure::record::{self as closure, ClosureStep},
        promotion::record::{IntegrationKind, PublicationCategory},
    };
    let parent = close_parent(service, root);
    state::with_transaction(service, root, |tx, id| {
        let op = parent.operation_id;
        let mut r = closure::read_record(tx, id, op)?.unwrap();
        r.context_oid = Some(close_oid('1'));
        r.remote_context_oid = Some(close_oid('1'));
        r.step = ClosureStep::ContextSynchronized;
        closure::save_record(tx, id, &r)?;
        closure::begin_closing(
            tx,
            id,
            op,
            close_oid('2'),
            time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            close_oid('3'),
        )?;
        let mut r = closure::read_record(tx, id, op)?.unwrap();
        r.integration_kind = Some(IntegrationKind::Merged);
        r.step = ClosureStep::Integrating;
        closure::save_record(tx, id, &r)?;
        r.integration_oid = Some(close_oid('4'));
        r.step = ClosureStep::Integrated;
        closure::save_record(tx, id, &r)?;
        r.publication = Some(PublicationCategory::Published);
        for step in [
            ClosureStep::Published,
            ClosureStep::RemoteDeleted,
            ClosureStep::WorktreeRemoving,
            ClosureStep::WorktreeRemoved,
            ClosureStep::BranchDeleted,
        ] {
            r.step = step;
            closure::save_record(tx, id, &r)?;
        }
        closure::read_record(tx, id, op).map(Option::unwrap)
    })
    .unwrap()
}
fn reserve_close(service: &RepositoryService, root: &Path, op: OperationId) -> RemoteReservation {
    match service
        .reserve_remote_operation(root, op, &close_target())
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    }
}

#[test]
fn close_reservation_holds_slot_like_promote() {
    let (data, root, service) = fixture();
    let owner = reserve_close(&service, root.path(), OperationId::new());
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
    ] {
        assert_eq!(
            service
                .remote_safe_point(root.path(), &owner, point)
                .unwrap(),
            RemoteSafePointOutcome::Continue
        );
    }
    // Only the parent's final SQL completes a close slot.
    assert!(
        service
            .finish_remote_operation(root.path(), &owner, RemoteOutcomeCategory::Completed)
            .is_err()
    );
    assert!(matches!(
        service
            .reserve_remote_operation(
                root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    state::with_transaction(&service, root.path(), |tx, id| {
        let r = owned(&service, tx, id, &owner)?;
        assert_eq!(r.target.action(), RemoteOperationAction::Close);
        Ok(())
    })
    .unwrap();
    // A yield is a recoverable stop: the slot stays reserved, no poll
    // bookkeeping is written, and the parent records the stop between steps.
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch(
        "UPDATE remote_operation_records SET yield_requested=1; UPDATE remote_polling_state SET next_eligible_at=1234",
    )
    .unwrap();
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .unwrap(),
        RemoteSafePointOutcome::Interrupted
    );
    let row: (String, bool) = db
        .query_row(
            "SELECT phase,yield_requested FROM remote_operation_records",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(row, ("reserved".to_owned(), false));
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_some()
    );
    let policy: (Option<String>, Option<i64>, Option<i64>) = db
        .query_row(
            "SELECT latest_outcome,latest_attempt_at,next_eligible_at FROM remote_polling_state",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(policy, (None, None, Some(1234)));
}

#[test]
fn close_restart_fences_owner_epoch() {
    use crate::repository::promotion::lifecycle::LifecycleKind;
    let (data, root, service) = fixture();
    let parent = close_parent(&service, root.path());
    let op = parent.operation_id;
    let owner = reserve_close(&service, root.path(), op);
    service
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("UPDATE remote_operation_records SET phase='interrupted'; UPDATE remote_polling_state SET next_eligible_at=1234").unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let new = match reopened
        .restart_remote_lifecycle(
            LifecycleKind::TicketClosure,
            root.path(),
            op,
            &close_target(),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    assert!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .is_err()
    );
    assert_eq!(new.epoch, owner.epoch + 1);
    reopened
        .remote_safe_point(
            root.path(),
            &new,
            RemoteOperationSafePoint::AfterAdvertisement,
        )
        .unwrap();
    assert_eq!(
        db.query_row(
            "SELECT next_eligible_at FROM remote_polling_state",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1234
    );
    // The close restart is its own kind: a promotion restart of a close
    // target is a mismatch, and an abandoned parent never restarts.
    assert_eq!(
        reopened
            .restart_remote_lifecycle(
                LifecycleKind::DocumentPromotion,
                root.path(),
                op,
                &close_target()
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    db.execute_batch("UPDATE remote_operation_records SET phase='interrupted'; UPDATE ticket_closures SET step='abandoned',refusal_category='external_change'").unwrap();
    assert_eq!(
        reopened
            .restart_remote_lifecycle(
                LifecycleKind::TicketClosure,
                root.path(),
                op,
                &close_target()
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::RecoveryRequired
    );
}

#[test]
fn close_cancel_flags_parent_and_reservation() {
    use crate::repository::closure::record as closure;
    let (data, root, service) = fixture();
    let parent = close_parent(&service, root.path());
    let op = parent.operation_id;
    let owner = reserve_close(&service, root.path(), op);
    service
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch(
        "UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=1234",
    )
    .unwrap();
    assert!(
        state::with_transaction(&service, root.path(), |tx, id| {
            closure::request_cancellation(tx, id, op)
        })
        .unwrap()
    );
    let flags: (bool, bool) = db
        .query_row(
            "SELECT c.cancel_requested,r.cancel_requested FROM ticket_closures c JOIN remote_operation_records r ON r.operation_ulid=c.operation_id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(flags, (true, true));
    assert_eq!(
        service
            .remote_safe_point(
                root.path(),
                &owner,
                RemoteOperationSafePoint::AfterAdvertisement
            )
            .unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    // The close slot is kept; the parent records the cancellation between
    // whole steps, and no poll policy is written.
    assert!(
        service
            .active_remote_operation(root.path())
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        service
            .reserve_remote_operation(
                root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan())
            )
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    let policy: (Option<String>, Option<i64>, Option<i64>) = db
        .query_row(
            "SELECT latest_outcome,latest_attempt_at,next_eligible_at FROM remote_polling_state",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(policy, (None, None, Some(1234)));
    assert!(
        state::with_transaction(&service, root.path(), |tx, id| Ok(closure::read_record(
            tx, id, op
        )?
        .unwrap()
        .cancel_requested))
        .unwrap()
    );
}

/// Runs `finish_parent(TicketClosure)` against a closure parent at
/// `branch_deleted` after `setup` adjusts its rows. Returns the result and
/// the close reservation's phase afterwards.
fn finish_close_case(setup: impl FnOnce(&Connection, OperationId)) -> (bool, String) {
    use crate::repository::promotion::lifecycle::LifecycleKind;
    let (data, root, service) = fixture();
    let parent = close_parent_at_branch_deleted(&service, root.path());
    let op = parent.operation_id;
    let owner = reserve_close(&service, root.path(), op);
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
    ] {
        service
            .remote_safe_point(root.path(), &owner, point)
            .unwrap();
    }
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    setup(&db, op);
    let finished = state::with_transaction(&service, root.path(), |tx, id| {
        finish_parent(LifecycleKind::TicketClosure, &service, tx, id, &owner)
    })
    .is_ok();
    let phase = db
        .query_row("SELECT phase FROM remote_operation_records", [], |r| {
            r.get(0)
        })
        .unwrap();
    (finished, phase)
}
fn retire(db: &Connection, op: OperationId, kind: &str, oid: char) {
    db.execute(
        "INSERT INTO retired_contexts(repository_id,kind,item_id,operation_id,retired_oid) SELECT id,?1,?2,?3,?4 FROM repositories",
        rusqlite::params![kind, CLOSE_ITEM, op.to_string(), close_oid(oid).to_string()],
    )
    .unwrap();
}
fn complete(db: &Connection) {
    db.execute_batch("UPDATE ticket_closures SET step='complete'")
        .unwrap();
}

#[test]
fn finish_close_requires_complete_closure_and_ticket_retirement_at_merged_context() {
    // The checkpoint `3…` is the merged context.
    assert_eq!(
        finish_close_case(|db, op| {
            complete(db);
            retire(db, op, "ticket", '3');
        }),
        (true, "completed".to_owned())
    );
    // An adopted synchronization over the checkpoint is the merged context.
    assert_eq!(
        finish_close_case(|db, op| {
            db.execute_batch(&format!(
                "UPDATE ticket_closures SET adopted_context_oid='{}'",
                close_oid('5')
            ))
            .unwrap();
            complete(db);
            retire(db, op, "ticket", '5');
        }),
        (true, "completed".to_owned())
    );
    for (name, setup) in [
        (
            "not complete",
            Box::new(|db: &Connection, op| retire(db, op, "ticket", '3'))
                as Box<dyn FnOnce(&Connection, OperationId)>,
        ),
        ("no retirement", Box::new(|db: &Connection, _| complete(db))),
        (
            "pre-closure context",
            Box::new(|db: &Connection, op| {
                complete(db);
                retire(db, op, "ticket", '1');
            }),
        ),
        (
            "checkpoint, not adopted context",
            Box::new(|db: &Connection, op| {
                db.execute_batch(&format!(
                    "UPDATE ticket_closures SET adopted_context_oid='{}'",
                    close_oid('5')
                ))
                .unwrap();
                complete(db);
                retire(db, op, "ticket", '3');
            }),
        ),
        (
            "another operation",
            Box::new(|db: &Connection, _| {
                complete(db);
                retire(db, OperationId::new(), "ticket", '3');
            }),
        ),
    ] {
        let (finished, phase) = finish_close_case(setup);
        assert!(!finished, "{name}");
        assert_ne!(phase, "completed", "{name}");
    }
}

#[test]
fn finish_close_refuses_document_retirement_row() {
    let (finished, phase) = finish_close_case(|db, op| {
        complete(db);
        retire(db, op, "document", '3');
    });
    assert!(!finished);
    assert_ne!(phase, "completed");
}

/// A closure parent with a minted context attempt, and its child execution.
fn close_child(
    service: &RepositoryService,
    root: &Path,
) -> (
    crate::repository::closure::record::ClosureRecord,
    crate::repository::promotion::record::ChildExecution,
) {
    use crate::repository::{closure::record as closure, promotion::record::ChildRole};
    let parent = close_parent(service, root);
    let parent = state::with_transaction(service, root, |tx, id| {
        closure::next_attempt(tx, id, parent.operation_id, ChildRole::Context)
    })
    .unwrap();
    let execution = closure::child_execution(&parent, ChildRole::Context).unwrap();
    (parent, execution)
}
fn context_request(
    root: &Path,
    child: OperationId,
    kind: crate::repository::AuthoringKind,
) -> crate::repository::SynchronizeRemoteRequest {
    crate::repository::SynchronizeRemoteRequest {
        root: root.into(),
        operation_id: child,
        target: crate::repository::SynchronizationTarget::Context {
            kind,
            item_id: close_item(),
        },
        approval: None,
        confirmed_identity: None,
        restart: false,
    }
}
fn context_sync_target(kind: crate::repository::AuthoringKind) -> RemoteOperationTarget {
    RemoteOperationTarget::for_context(
        &plan(),
        RemoteOperationAction::SynchronizeContext,
        kind,
        close_item(),
    )
    .unwrap()
}

#[test]
fn child_fence_accepts_ticket_target_only_for_closure_parent() {
    use crate::repository::{
        AuthoringKind, PromotionExpectation,
        promotion::record::{self as promotion, ChildRole},
    };
    let (data, root, service) = fixture();
    let (parent, execution) = close_child(&service, root.path());
    let child = parent.context_sync_id.unwrap();
    execution
        .validate_request(&context_request(root.path(), child, AuthoringKind::Ticket))
        .unwrap();
    // The fence reads the closure parent in the reservation transaction.
    let owner = match service
        .reserve_promotion_child(
            root.path(),
            child,
            &context_sync_target(AuthoringKind::Ticket),
            &execution,
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    assert_eq!(owner.operation_id(), child);
    // Its explicit restart passes the same fence.
    Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .execute_batch("UPDATE remote_operation_records SET phase='interrupted'")
        .unwrap();
    assert!(matches!(
        service
            .restart_promotion_child(
                root.path(),
                child,
                &context_sync_target(AuthoringKind::Ticket),
                &execution,
            )
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    // A promotion parent's child is never admitted for a ticket target.
    let (_data, root, service) = fixture();
    let parent = state::with_transaction(&service, root.path(), |tx, id| {
        let r = promotion::begin_record(
            tx,
            id,
            OperationId::new(),
            &close_item(),
            PromotionExpectation::from_bytes([7; 32]),
            Some("origin"),
        )?;
        promotion::next_attempt(tx, id, r.operation_id, ChildRole::Context)
    })
    .unwrap();
    let execution = promotion::child_execution(&parent, ChildRole::Context).unwrap();
    let child = parent.context_sync_id.unwrap();
    assert_eq!(
        execution
            .validate_request(&context_request(root.path(), child, AuthoringKind::Ticket))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    execution
        .validate_request(&context_request(
            root.path(),
            child,
            AuthoringKind::Document,
        ))
        .unwrap();
    assert!(
        service
            .reserve_promotion_child(
                root.path(),
                child,
                &context_sync_target(AuthoringKind::Ticket),
                &execution,
            )
            .is_err()
    );
}

#[test]
fn child_fence_refuses_document_target_for_closure_parent() {
    use crate::repository::AuthoringKind;
    let (data, root, service) = fixture();
    let (parent, execution) = close_child(&service, root.path());
    let child = parent.context_sync_id.unwrap();
    assert_eq!(
        execution
            .validate_request(&context_request(
                root.path(),
                child,
                AuthoringKind::Document
            ))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    // Another ID than the minted child is refused too.
    assert_eq!(
        execution
            .validate_request(&context_request(
                root.path(),
                OperationId::new(),
                AuthoringKind::Ticket
            ))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    assert_eq!(
        service
            .reserve_promotion_child(
                root.path(),
                child,
                &context_sync_target(AuthoringKind::Document),
                &execution,
            )
            .unwrap_err()
            .kind,
        RepositoryErrorKind::OperationMismatch
    );
    assert_eq!(
        Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
}
