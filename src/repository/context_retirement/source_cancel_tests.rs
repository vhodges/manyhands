//! CURRENT-F2 cancellation regressions. INERT canonical A0/A1 projections and
//! synthetic advertisements, REAL owned SQL; not source SSH or Task 3/4 journeys.
use super::*;
use crate::repository::remote::reservation;
use rusqlite::types::Value;

#[derive(Clone, Copy, Debug)]
enum Signal {
    Request,
    Remote,
    Both,
}
const SIGNALS: [Signal; 3] = [Signal::Request, Signal::Remote, Signal::Both];

fn setup(captured: bool, acknowledged: bool) -> SourceFixture {
    let mut s = SourceFixture::new();
    s.seal().unwrap();
    if captured {
        s.before_batch();
        s.capture().unwrap();
    } else if acknowledged {
        assert_eq!(
            s.f.service
                .remote_safe_point(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    RemoteOperationSafePoint::BeforeTransport
                )
                .unwrap(),
            RemoteSafePointOutcome::Continue
        );
    }
    s
}
fn signal(s: &SourceFixture, signal: Signal) {
    if matches!(signal, Signal::Request | Signal::Both) {
        assert_eq!(
            s.f.db()
                .execute(
                    "UPDATE request_records SET cancel_requested=1 WHERE request_ulid=?1",
                    [s.request_id.to_string()]
                )
                .unwrap(),
            1
        );
    }
    if matches!(signal, Signal::Remote | Signal::Both) {
        s.f.service
            .cancel_remote_operation(s.f.root.path(), s.operation)
            .unwrap();
    }
}
fn point(captured: bool) -> RemoteOperationSafePoint {
    if captured {
        RemoteOperationSafePoint::BeforeDiscovery
    } else {
        RemoteOperationSafePoint::BeforeTransport
    }
}
type Image = BTreeMap<String, Vec<Vec<Value>>>;
fn value(db: &Connection, table: &str, column: &str) -> Value {
    db.query_row(&format!("SELECT {column} FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

// Compare the ENTIRE registry/schema/cache image. Only the normal cancellation
// metadata, exact source phase and exact copied pin/release may change. In
// particular request flags/history, batches, observations, configuration, local
// debt, provenance and every unrelated row/column must remain byte-for-byte equal.
fn assert_stop_image(s: &SourceFixture, mut before: Image, captured: bool) {
    let db = s.f.db();
    let after = registry_image(&db);
    for (table, allowed) in [
        (
            "remote_operation_records",
            &[
                "phase",
                "completed_step",
                "outcome",
                "updated_at",
                "poll_completed_at",
                "poll_next_eligible_at",
            ][..],
        ),
        (
            "remote_polling_state",
            &["latest_outcome", "latest_attempt_at"][..],
        ),
        ("poll_retirement_attempts", &["phase"][..]),
    ] {
        let columns: Vec<String> = db
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let old = before.get_mut(table).unwrap();
        let new = &after[table];
        assert_eq!(old.len(), new.len());
        for (old, new) in old.iter_mut().zip(new) {
            for (index, name) in columns.iter().enumerate() {
                if allowed.contains(&name.as_str()) {
                    old[index] = new[index].clone();
                }
            }
        }
    }
    let pins = before.get_mut("retirement_observation_pins").unwrap();
    assert_eq!(pins.len(), usize::from(captured));
    let releases = if captured {
        let mut release = pins[0].clone();
        release.push(Value::Text("source_terminal".into()));
        vec![release]
    } else {
        vec![]
    };
    pins.clear();
    assert!(before["retirement_observation_pin_releases"].is_empty());
    before.insert("retirement_observation_pin_releases".into(), releases);
    assert_eq!(before, after, "only exact STOP/Z changes may commit");
    assert_eq!(
        value(&db, "poll_retirement_attempts", "phase"),
        Value::Text("ended".into())
    );
    assert_eq!(
        value(&db, "remote_operation_records", "phase"),
        Value::Text("cancelled".into())
    );
    assert_eq!(
        value(&db, "remote_operation_records", "outcome"),
        Value::Text("cancelled".into())
    );
    assert_eq!(
        value(&db, "remote_polling_state", "latest_outcome"),
        Value::Text("cancelled".into())
    );
    assert_ne!(
        value(&db, "remote_operation_records", "poll_completed_at"),
        Value::Null
    );
    assert!(
        !db.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    assert_eq!(
        after,
        registry_image(&db),
        "reopen must not repair anything"
    );
    assert!(
        s.f.service
            .remote_safe_point(s.f.root.path(), s.owner.as_ref().unwrap(), point(captured))
            .is_err(),
        "ended LIVE cap never acknowledges twice"
    );
    assert!(
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Cancelled
            )
            .is_err()
    );
    assert_eq!(after, registry_image(&db));
    // Existing stopped-source metadata replay remains idempotent and performs no
    // fresh collection, while the old LIVE capability stays fenced above.
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        let stopped = state::read_operation(tx, id, s.operation)?.unwrap();
        crate::repository::context_retirement::source::close_stopped(tx, &stopped)
    })
    .unwrap();
    assert_eq!(after, registry_image(&db));
}

#[test]
fn retirement_source_cancelfix_current_request_remote_and_both_stop_at_safe_points() {
    let mut rejected = Vec::new();
    for captured in [false, true] {
        for which in SIGNALS {
            let s = setup(captured, false);
            signal(&s, which);
            let before = registry_image(&s.f.db());
            let disk = disk_image(s.f.root.path());
            let result = s.f.service.remote_safe_point(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                point(captured),
            );
            if matches!(result, Ok(RemoteSafePointOutcome::Cancelled)) {
                assert_stop_image(&s, before, captured);
            } else {
                assert_eq!(before, registry_image(&s.f.db()));
                rejected.push((captured, which));
            }
            assert_eq!(disk, disk_image(s.f.root.path()));
        }
    }
    assert!(
        rejected.is_empty(),
        "CURRENT cancellation lost STOP ownership: {rejected:?}"
    );
}

#[test]
fn retirement_source_cancelfix_supported_cancel_finish_honors_all_current_signals() {
    let mut rejected = Vec::new();
    for captured in [false, true] {
        for which in SIGNALS {
            // The before-ad fixture first acknowledges BeforeTransport while live.
            // Finish does not acquire an unconditional right to stop an unacked P.
            let s = setup(captured, true);
            signal(&s, which);
            let before = registry_image(&s.f.db());
            let disk = disk_image(s.f.root.path());
            let result = s.f.service.finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Cancelled,
            );
            if matches!(result, Ok(RemoteSafePointOutcome::Cancelled)) {
                assert_stop_image(&s, before, captured);
            } else {
                assert_eq!(before, registry_image(&s.f.db()));
                rejected.push((captured, which));
            }
            assert_eq!(disk, disk_image(s.f.root.path()));
        }
    }
    assert!(
        rejected.is_empty(),
        "CURRENT cancellation finish lost ownership: {rejected:?}"
    );
}

#[test]
fn retirement_source_cancelfix_commit_stops_before_advertisement_or_capture_writes() {
    let mut rejected = Vec::new();
    for which in SIGNALS {
        let s = setup(false, false);
        s.before_batch();
        signal(&s, which);
        let before = registry_image(&s.f.db());
        let disk = disk_image(s.f.root.path());
        // Cancellation arrives after the caller's acknowledged BeforeBatchCommit
        // boundary, before the actual owned write transaction. Not a new permit.
        let result = s.capture();
        if matches!(result, Ok(RemoteSafePointOutcome::Cancelled)) {
            assert_stop_image(&s, before, false);
        } else {
            assert_eq!(before, registry_image(&s.f.db()));
            rejected.push(which);
        }
        assert_eq!(disk, disk_image(s.f.root.path()));
    }
    assert!(
        rejected.is_empty(),
        "cancelled commit did not execute STOP-only path: {rejected:?}"
    );
}

#[test]
fn retirement_source_cancelfix_cancel_never_relaxes_stale_finished_or_foreign_ownership() {
    for captured in [false, true] {
        for case in [
            "attempt",
            "finished",
            "map",
            "source_epoch",
            "batch_epoch",
            "generation",
            "endpoint",
            "local_debt",
            "foreign_service",
        ] {
            let s = setup(captured, true);
            if !captured {
                s.before_batch();
            }
            signal(&s, Signal::Both);
            let db = s.f.db();
            match case {
                "attempt" => {
                    s.f.service.enter_request(s.request_id).unwrap().unwrap();
                }
                "finished" => {
                    assert!(
                        s.f.service
                            .finish_request(
                                s.request_id,
                                1,
                                &RequestResult {
                                    outcome: crate::results::Outcome::Cancelled,
                                    code: crate::results::ResultCode::Cancelled,
                                    effects: crate::results::Effects::not_requested(),
                                    data: None
                                }
                            )
                            .unwrap()
                    );
                }
                "map" => {
                    db.execute("UPDATE request_operations SET family='local'", [])
                        .unwrap();
                }
                "source_epoch" => {
                    db.execute(
                        "UPDATE remote_operation_records SET owner_epoch=owner_epoch+1",
                        [],
                    )
                    .unwrap();
                }
                "batch_epoch" => {
                    db.execute(
                        "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1",
                        [],
                    )
                    .unwrap();
                }
                "generation" => {
                    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                        state::configure(tx, id, None, true)
                    })
                    .unwrap();
                }
                "endpoint" => {
                    db.execute(
                        "UPDATE poll_retirement_batches SET endpoint_digest=zeroblob(32)",
                        [],
                    )
                    .unwrap();
                }
                "local_debt" => {
                    db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,observed_at) SELECT id,root_path,?1,'refresh','authoritative_observed',1 FROM repositories",[OperationId::new().to_string()]).unwrap();
                }
                "foreign_service" => {}
                _ => unreachable!(),
            }
            // A new request attempt may reset its cancellation flag. Keep the
            // matched stop signals present without invoking APIs on corrupt rows.
            db.execute("UPDATE request_records SET cancel_requested=1", [])
                .unwrap();
            let foreign = if case == "foreign_service" {
                Some(RepositoryService::open_at(s.f.data.path()).unwrap())
            } else {
                None
            };
            let service = foreign.as_ref().unwrap_or(&s.f.service);
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            let boundary = if captured {
                point(true)
            } else {
                RemoteOperationSafePoint::BeforeBatchCommit
            };
            assert!(
                service
                    .remote_safe_point(s.f.root.path(), s.owner.as_ref().unwrap(), boundary)
                    .is_err(),
                "{case}/{captured}"
            );
            assert!(
                service
                    .finish_remote_operation(
                        s.f.root.path(),
                        s.owner.as_ref().unwrap(),
                        RemoteOutcomeCategory::Cancelled
                    )
                    .is_err(),
                "{case}/{captured}"
            );
            assert!(
                reservation::commit_observation_batch(
                    service,
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    &s.plan,
                    &[],
                    43
                )
                .is_err(),
                "{case}/{captured}"
            );
            assert_eq!(before, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
        }
    }
}

#[test]
fn retirement_source_cancelfix_after_capture_stop_and_z_roll_back_together() {
    for finish in [false, true] {
        for which in SIGNALS {
            let s = setup(true, true);
            signal(&s, which);
            let before = registry_image(&s.f.db());
            let disk = disk_image(s.f.root.path());
            record::set_release_failure(true);
            let result = if finish {
                s.f.service.finish_remote_operation(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    RemoteOutcomeCategory::Cancelled,
                )
            } else {
                s.f.service.remote_safe_point(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    point(true),
                )
            };
            assert!(result.is_err());
            let unconsumed = record::set_release_failure(false);
            assert_eq!(before, registry_image(&s.f.db()));
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert!(
                !unconsumed,
                "current owner must reach the injected Z boundary: {finish}/{which:?}"
            );
            let retry = if finish {
                s.f.service.finish_remote_operation(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    RemoteOutcomeCategory::Cancelled,
                )
            } else {
                s.f.service.remote_safe_point(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    point(true),
                )
            };
            assert_eq!(retry.unwrap(), RemoteSafePointOutcome::Cancelled);
            assert_stop_image(&s, before, true);
            assert_eq!(disk, disk_image(s.f.root.path()));
        }
    }
}

#[test]
fn retirement_source_cancelfix_cancel_does_not_grant_unacked_finish_or_normal_writes() {
    for captured in [false, true] {
        let s = setup(captured, false);
        signal(&s, Signal::Request);
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        if !captured {
            assert!(
                s.f.service
                    .finish_remote_operation(
                        s.f.root.path(),
                        s.owner.as_ref().unwrap(),
                        RemoteOutcomeCategory::Cancelled
                    )
                    .is_err(),
                "no preceding acknowledgement"
            );
        }
        assert!(
            s.f.service
                .finish_remote_operation(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    RemoteOutcomeCategory::Completed
                )
                .is_err()
        );
        let write = state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
            let record = reservation::owned(&s.f.service, tx, id, s.owner.as_ref().unwrap())?;
            tx.execute(
                "UPDATE remote_operation_records SET updated_at=updated_at+1 WHERE id=?1",
                [record.id],
            )
            .map_err(RepositoryError::sqlite)?;
            Ok(())
        });
        assert!(
            write.is_err(),
            "general owned authority must stay continuation-strict"
        );
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
    }
}

#[test]
fn retirement_source_cancelfix_configuration_stopped_metadata_release_stays_separate() {
    let s = setup(true, true);
    signal(&s, Signal::Request);
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        state::configure(tx, id, None, true)
    })
    .unwrap();
    let db = s.f.db();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(
        s.f.service
            .remote_safe_point(s.f.root.path(), s.owner.as_ref().unwrap(), point(true))
            .is_err()
    );
    assert!(
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Cancelled
            )
            .is_err()
    );
    assert_eq!(before, registry_image(&db));
    for replay in [false, true] {
        let before = registry_image(&db);
        state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
            let record = state::read_operation(tx, id, s.operation)?.unwrap();
            crate::repository::context_retirement::source::close_stopped(tx, &record)
        })
        .unwrap();
        if replay {
            assert_eq!(before, registry_image(&db));
        }
    }
    assert_eq!(
        value(&db, "poll_retirement_attempts", "phase"),
        Value::Text("ended".into())
    );
    assert_eq!(
        value(&db, "request_records", "cancel_requested"),
        Value::Integer(1)
    );
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    assert_eq!(disk, disk_image(s.f.root.path()));
}
