//! Q1: real original A3 and foreign owned integration journals, not live SSH/Git
//! resolution. Only the resolution envelope is explicitly compatibility-projected
//! after real journal APIs, to isolate retained debt from active envelope flags.
use super::*;
use crate::repository::remote::retirement_batch_test_support::ResolutionStage;

fn foreign_conflict(s: &SourceFixture, window: u32) -> OperationId {
    let before: i64 =
        s.f.db()
            .query_row(
                "SELECT id FROM remote_observation_batches WHERE is_current=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
    let operation = OperationId::new();
    let RemoteReservationOutcome::Reserved(owner) =
        s.f.service
            .reserve_remote_operation(
                s.f.root.path(),
                operation,
                &RemoteOperationTarget::for_primary_synchronization(&s.plan),
            )
            .unwrap()
    else {
        panic!("foreign primary synchronization reserves");
    };
    s.f.service
        .retirement_test_conflict(s.f.root.path(), &owner, window)
        .unwrap();
    let db = s.f.db();
    let row: (String, bool, bool, String) = db.query_row("SELECT o.phase,o.index_pending,o.reconciliation_required,i.phase FROM remote_operation_records o JOIN remote_integration_steps i ON i.operation_record_id=o.id WHERE o.operation_ulid=?1", [operation.to_string()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
    assert_eq!(
        row,
        (
            "interrupted".into(),
            false,
            false,
            "conflict_pending".into()
        )
    );
    assert_eq!(
        before,
        db.query_row(
            "SELECT id FROM remote_observation_batches WHERE is_current=1",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap()
    );
    assert!(state::has_pending_conflict(&db, 1).unwrap());
    assert!(matches!(
        s.f.service
            .reserve_remote_poll(
                s.f.root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&s.plan),
                RemoteOperationPriority::Manual
            )
            .unwrap(),
        RemotePollReservationOutcome::ConflictPending
    ));
    operation
}

fn change_last(s: &SourceFixture) {
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let other = git
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .parent_id(0)
        .unwrap();
    git.reference(
        "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW",
        other,
        true,
        "positive frozen-tip change",
    )
    .unwrap();
}

fn assert_positive_git_change(s: &SourceFixture, ordinal: usize) {
    use crate::repository::context_retirement::instance;
    let image: String =
        s.f.db()
            .query_row("SELECT image FROM poll_retirement_freezes", [], |r| {
                r.get(0)
            })
            .unwrap();
    let entries: serde_json::Value = serde_json::from_str(&image).unwrap();
    let k = &entries[ordinal];
    let request = RetireContextRequest {
        root: s.f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: k["item_id"].as_str().unwrap().parse().unwrap(),
        operation_id: OperationId::parse(k["child_operation_id"].as_str().unwrap()).unwrap(),
    };
    assert!(matches!(
        instance::compare_frozen(
            &git2::Repository::open(s.f.root.path()).unwrap(),
            &request,
            git2::Oid::from_str(k["expected_tip"].as_str().unwrap()).unwrap(),
            k["instance_nonce"].as_str()
        ),
        Ok(instance::FrozenComparison::Changed)
    ));
}

fn project_inactive_resolution_envelope(s: &SourceFixture, operation: OperationId) {
    // Explicit compatibility projection ONLY: real resolution reacquisition sets
    // reconciliation_required. Zeroing it isolates child-journal exclusion; this
    // is not represented as the real resolve/re-fetch/finalization journey.
    s.f.db().execute("UPDATE remote_operation_records SET phase='interrupted',reconciliation_required=0 WHERE operation_ulid=?1", [operation.to_string()]).unwrap();
}

fn assert_resolution_stage(s: &SourceFixture, operation: OperationId, stage: ResolutionStage) {
    let db = s.f.db();
    let row: (String, String, String, String) = db.query_row("SELECT step.phase,attempt.phase,artifact.phase,artifact.ref_phase FROM remote_operation_records operation JOIN remote_integration_steps step ON step.operation_record_id=operation.id JOIN remote_resolution_attempts attempt ON attempt.integration_step_id=step.id JOIN remote_resolution_index_artifacts artifact ON artifact.attempt_id=attempt.id WHERE operation.operation_ulid=?1", [operation.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
    let expected = match stage {
        ResolutionStage::Intent => (
            "resolution_prepared",
            "paths_applying",
            "intent",
            "not_started",
        ),
        ResolutionStage::Paths => (
            "resolution_prepared",
            "paths_applying",
            "published",
            "not_started",
        ),
        ResolutionStage::Candidate => (
            "commit_prepared",
            "candidate_prepared",
            "published",
            "not_started",
        ),
        ResolutionStage::Applied => ("applied", "applied", "published", "observed"),
        ResolutionStage::Releasing => ("applied", "applied", "release_intent", "observed"),
        ResolutionStage::Released => ("applied", "applied", "released", "observed"),
    };
    assert_eq!(
        (
            row.0.as_str(),
            row.1.as_str(),
            row.2.as_str(),
            row.3.as_str()
        ),
        expected
    );
    let envelope: (String, bool, bool) = db.query_row("SELECT phase,index_pending,reconciliation_required FROM remote_operation_records WHERE operation_ulid=?1", [operation.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(envelope, ("interrupted".into(), false, false));
}

#[test]
fn retirement_batch_qualityfix_new_handle_refuses_owned_conflict_debt() {
    for window in [0, 1] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        foreign_conflict(&s, window);
        assert_positive_git_change(&s, 0);
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(
            live.next(&s.f.service, s.f.root.path()).is_err(),
            "window={window}"
        );
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(decision(&db), ("eligible".into(), None));
    }
}

#[test]
fn retirement_batch_qualityfix_retained_handle_and_last_settlement_refuse_owned_conflict() {
    for (window, last) in [(0, false), (1, false), (0, true), (1, true)] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        if last {
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .refuse_if_changed(&s.f.service)
                .unwrap();
            change_last(&s);
        }
        let held = live.next(&s.f.service, s.f.root.path()).unwrap();
        foreign_conflict(&s, window);
        assert_positive_git_change(&s, usize::from(last));
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        let next = live.next(&s.f.service, s.f.root.path());
        let result = held.refuse_if_changed(&s.f.service);
        assert!(
            next.is_err() && result.is_err(),
            "window={window}, last={last}"
        );
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(decision(&db), ("eligible".into(), None));
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_qualityfix_resolution_steps_and_unreleased_artifacts_remain_debt() {
    for window in [0, 1] {
        for stage in [
            ResolutionStage::Intent,
            ResolutionStage::Paths,
            ResolutionStage::Candidate,
            ResolutionStage::Applied,
            ResolutionStage::Releasing,
        ] {
            let s = prepared_batch();
            let live = run_original(&s);
            change_first(&s);
            let held = live.next(&s.f.service, s.f.root.path()).unwrap();
            let operation = foreign_conflict(&s, window);
            s.f.service
                .retirement_test_resolution(s.f.root.path(), operation, &s.plan, window, stage)
                .unwrap();
            project_inactive_resolution_envelope(&s, operation);
            assert_resolution_stage(&s, operation, stage);
            assert_positive_git_change(&s, 0);
            let db = s.f.db();
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            let next = live.next(&s.f.service, s.f.root.path());
            let result = held.refuse_if_changed(&s.f.service);
            assert!(
                next.is_err() && result.is_err(),
                "window={window}, stage={stage:?}"
            );
            assert_eq!(before, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert_eq!(counts(&db), (1, 1, 1));
            record::audit_connection(&db).unwrap();
        }
    }
}

#[test]
fn retirement_batch_qualityfix_applied_released_history_and_pinned_windows_do_not_block_c() {
    for window in [0, 1] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        let held = live.next(&s.f.service, s.f.root.path()).unwrap();
        let operation = foreign_conflict(&s, window);
        s.f.service
            .retirement_test_resolution(
                s.f.root.path(),
                operation,
                &s.plan,
                window,
                ResolutionStage::Released,
            )
            .unwrap();
        project_inactive_resolution_envelope(&s, operation);
        assert_resolution_stage(&s, operation, ResolutionStage::Released);
        let db = s.f.db();
        let history = [
            "remote_integration_steps",
            "remote_integration_windows",
            "remote_resolution_attempts",
            "remote_resolution_index_artifacts",
        ]
        .map(|table| row_image(&db, table));
        held.refuse_if_changed(&s.f.service).unwrap();
        change_last(&s);
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .refuse_if_changed(&s.f.service)
            .unwrap();
        assert_eq!(
            history,
            [
                "remote_integration_steps",
                "remote_integration_windows",
                "remote_resolution_attempts",
                "remote_resolution_index_artifacts"
            ]
            .map(|table| row_image(&db, table))
        );
        assert_eq!(counts(&db), (1, 0, 2));
        assert_eq!(decision(&db), ("eligible".into(), Some("ok".into())));
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_batch_qualityfix_historical_final_read_survives_later_foreign_conflict() {
    for complete in [false, true] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        let report = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .refuse_if_changed(&s.f.service)
            .unwrap();
        if complete {
            change_last(&s);
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .refuse_if_changed(&s.f.service)
                .unwrap();
        }
        foreign_conflict(&s, 1);
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert_eq!(
            batch::read_refusal(&s.f.service, s.f.root.path(), s.operation, 0).unwrap(),
            Some(report)
        );
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_batch_qualityfix_missing_integration_evidence_fails_closed_without_writes() {
    for table in [
        "remote_integration_steps",
        "remote_resolution_attempts",
        "remote_resolution_index_artifacts",
    ] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        let held = live.next(&s.f.service, s.f.root.path()).unwrap();
        let db = s.f.db();
        // Isolated missing-table corruption, not a supported journal transition.
        db.execute_batch(&format!("DROP TABLE {table}")).unwrap();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        let next = live.next(&s.f.service, s.f.root.path());
        let result = held.refuse_if_changed(&s.f.service);
        assert!(next.is_err() && result.is_err(), "{table}");
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(counts(&db), (1, 1, 1));
    }
}
