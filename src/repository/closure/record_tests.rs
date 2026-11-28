use super::*;
use crate::repository::{
    AuthoringKind, REGISTRY_FILE, RemoteOperationAction, RemoteOperationTarget, RemoteRefPlan,
    RepositoryErrorKind, RepositoryOperation, RepositoryService,
    discovery::migrate_registry,
    promotion::{
        PromotionExpectation,
        lifecycle::{LifecycleRecord, ParentStep, RecordPath},
        record as promotion,
    },
    recovery,
};
use rusqlite::{Connection, params};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
/// A `closed_by` value a closure would carry: its `@` and `<` mark it.
const CLOSED_BY: &str = "Closer Sentinel <closer-sentinel@example.invalid>";

fn db() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    migrate_registry(&mut db).unwrap();
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,'/fixture',123,'accessible',0)", []).unwrap();
    db
}
fn item(text: &str) -> ItemId {
    text.parse().unwrap()
}
fn digest(byte: u8) -> ClosureExpectation {
    ClosureExpectation::from_bytes([byte; 32])
}
fn oid(digit: char) -> Oid {
    Oid::from_str(&digit.to_string().repeat(40)).unwrap()
}
fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
}
fn begin(tx: &Transaction<'_>, op: OperationId, kind: ClosureKind) -> ClosureRecord {
    begin_record(tx, 1, op, &item(ITEM), digest(7), None, kind).unwrap()
}
fn read(tx: &Transaction<'_>, op: OperationId) -> ClosureRecord {
    read_record(tx, 1, op).unwrap().unwrap()
}
fn kind_of<T>(result: Result<T, RepositoryError>) -> RepositoryErrorKind {
    match result {
        Ok(_) => panic!("expected an error"),
        Err(error) => error.kind,
    }
}
fn synchronize(tx: &Transaction<'_>, r: &mut ClosureRecord) {
    r.context_oid = Some(oid('1'));
    r.step = ClosureStep::ContextSynchronized;
    save_record(tx, 1, r).unwrap();
}
/// `Record`: confirmed → context_synchronized → closing.
fn close(tx: &Transaction<'_>, r: &mut ClosureRecord) {
    synchronize(tx, r);
    begin_closing(tx, 1, r.operation_id, oid('2'), at(), oid('3')).unwrap();
    *r = read(tx, r.operation_id);
}
/// From `closing` (`Record`) or `context_synchronized` (`Existing`) to
/// `integrated`.
fn integrate(tx: &Transaction<'_>, r: &mut ClosureRecord) {
    if r.base_oid.is_none() {
        r.base_oid = Some(oid('2'));
    }
    r.integration_kind = Some(IntegrationKind::Merged);
    r.step = ClosureStep::Integrating;
    save_record(tx, 1, r).unwrap();
    r.integration_oid = Some(oid('4'));
    r.step = ClosureStep::Integrated;
    save_record(tx, 1, r).unwrap();
}
/// Whether a direct change to a stored row is refused by the schema or by
/// the record's read validation. The change is always rolled back.
fn rejected(tx: &Transaction<'_>, op: OperationId, assignment: &str) -> bool {
    tx.execute_batch("SAVEPOINT probe").unwrap();
    let refused = tx
        .execute(
            &format!("UPDATE ticket_closures SET {assignment} WHERE operation_id=?1"),
            [op.to_string()],
        )
        .is_err()
        || read_record(tx, 1, op).is_err();
    tx.execute_batch("ROLLBACK TO probe; RELEASE probe")
        .unwrap();
    refused
}

/// How many of the closure table's four migration markers are set.
fn marker(db: &Connection) -> i64 {
    db.query_row(
        "SELECT count(*) FROM registry_migrations WHERE name IN ('cycle_10_ticket_closures','closure_refusal_blockers_v2','wave_06_closure_modes_v1','wave_06_closure_cleanup_v1')",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn migration_is_all_or_nothing_and_idempotent() {
    let mut db = db();
    db.execute_batch("DROP TABLE ticket_closures; DELETE FROM registry_migrations WHERE name IN ('cycle_10_ticket_closures','closure_refusal_blockers_v2','wave_06_closure_modes_v1','wave_06_closure_cleanup_v1');").unwrap();
    // A failure anywhere in the migration leaves neither table, index nor marker.
    db.execute_batch("CREATE TRIGGER marker_fault BEFORE INSERT ON registry_migrations WHEN NEW.name='wave_06_closure_cleanup_v1' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;").unwrap();
    assert!(migrate_registry(&mut db).is_err());
    let leftovers: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('ticket_closures','ticket_closures_unfinished')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leftovers, 0);
    assert_eq!(marker(&db), 0);
    db.execute_batch("DROP TRIGGER marker_fault").unwrap();
    migrate_registry(&mut db).unwrap();
    // A fresh registry is created at the newest layout and carries all four markers.
    assert_eq!(marker(&db), 4);
    // A second run changes nothing.
    let changes = db.total_changes();
    let tx = db.transaction().unwrap();
    migrate(&tx).unwrap();
    tx.commit().unwrap();
    migrate_registry(&mut db).unwrap();
    assert_eq!(db.total_changes(), changes);
    assert_eq!(marker(&db), 4);
    audit(&db).unwrap();
    // A half-created table, or a marked but missing one, fails closed.
    for marked in [false, true] {
        let mut db = self::db();
        db.execute_batch("DROP TABLE ticket_closures;").unwrap();
        if !marked {
            db.execute_batch("DELETE FROM registry_migrations WHERE name='cycle_10_ticket_closures'; CREATE TABLE ticket_closures(operation_id TEXT);").unwrap();
        }
        assert_eq!(
            migrate_registry(&mut db).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired
        );
    }
}

#[test]
fn steps_permit_only_designed_transitions() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    // Record: context_synchronized → closing → integrating; never straight to
    // integrating, and closing only through begin_closing.
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    synchronize(&tx, &mut r);
    let mut skip = r.clone();
    skip.base_oid = Some(oid('2'));
    skip.integration_kind = Some(IntegrationKind::Merged);
    skip.step = ClosureStep::Integrating;
    assert_eq!(
        kind_of(save_record(&tx, 1, &skip)),
        RepositoryErrorKind::RecoveryRequired
    );
    let mut saved_closing = r.clone();
    saved_closing.base_oid = Some(oid('2'));
    saved_closing.step = ClosureStep::Closing;
    assert!(save_record(&tx, 1, &saved_closing).is_err());
    begin_closing(&tx, 1, r.operation_id, oid('2'), at(), oid('3')).unwrap();
    r = read(&tx, r.operation_id);
    assert_eq!(r.step, ClosureStep::Closing);
    for backwards in [
        ClosureStep::Confirmed,
        ClosureStep::ContextSynchronized,
        ClosureStep::Integrated,
        ClosureStep::Superseded,
        ClosureStep::Complete,
    ] {
        let mut wrong = r.clone();
        wrong.step = backwards;
        wrong.integration_kind = Some(IntegrationKind::Merged);
        wrong.integration_oid = Some(oid('4'));
        assert!(save_record(&tx, 1, &wrong).is_err(), "{backwards:?}");
    }
    // The refusal above is the transition itself, not another invalid field:
    // closing → superseded is never permitted, with or without a remote,
    // for either closure kind (R-P2).
    for remote in [false, true] {
        for kind in [Some(ClosureKind::Record), Some(ClosureKind::Existing)] {
            assert!(
                !ClosureStep::Closing.permits(ClosureStep::Superseded, remote, kind),
                "closing → superseded ({remote}, {kind:?})"
            );
        }
    }
    assert!(ClosureStep::Closing.permits(ClosureStep::Abandoned, false, Some(ClosureKind::Record)));
    assert_eq!(
        kind_of(begin_closing(
            &tx,
            1,
            r.operation_id,
            oid('2'),
            at(),
            oid('3')
        )),
        RepositoryErrorKind::RecoveryRequired
    );
    integrate(&tx, &mut r);
    assert_eq!(read(&tx, r.operation_id).step, ClosureStep::Integrated);

    // Closing → abandoned is a record-level transition; the runner alone
    // decides when the checkpoint ref has not moved (R-P2).
    let mut a = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Record,
    )
    .unwrap();
    close(&tx, &mut a);
    a.step = ClosureStep::Abandoned;
    a.refusal = Some(Refusal::ReconfirmationRequired);
    save_record(&tx, 1, &a).unwrap();
    let a = read(&tx, a.operation_id);
    assert_eq!(a.step, ClosureStep::Abandoned);
    assert_eq!(a.closure_oid, Some(oid('3')));
    let mut after = a.clone();
    after.step = ClosureStep::Integrating;
    after.refusal = None;
    assert!(
        save_record(&tx, 1, &after).is_err(),
        "abandoned is terminal"
    );
    drop(tx);

    // Existing: context_synchronized → integrating directly; never closing.
    let mut db = self::db();
    let tx = db.transaction().unwrap();
    let mut e = begin(&tx, OperationId::new(), ClosureKind::Existing);
    synchronize(&tx, &mut e);
    assert_eq!(
        kind_of(begin_closing(
            &tx,
            1,
            e.operation_id,
            oid('2'),
            at(),
            oid('3')
        )),
        RepositoryErrorKind::RecoveryRequired
    );
    integrate(&tx, &mut e);
    assert_eq!(read(&tx, e.operation_id).step, ClosureStep::Integrated);

    // A branch record has its own graph, per action (design Part D).
    use ClosureStep::*;
    let graph = |action: ClosureAction| -> Vec<(ClosureStep, ClosureStep)> {
        let mut all = Vec::new();
        for from in ALL_STEPS {
            for to in ALL_STEPS {
                if from.branch_permits(to, action) {
                    all.push((from, to));
                }
            }
        }
        all
    };
    let steps_of = |action: ClosureAction, from: ClosureStep| -> Vec<ClosureStep> {
        graph(action)
            .into_iter()
            .filter(|(f, t)| *f == from && *t != from)
            .map(|(_, t)| t)
            .collect()
    };
    for action in [ClosureAction::Close, ClosureAction::Withdraw] {
        assert_eq!(
            steps_of(action, Confirmed),
            [ContextSynchronized, Abandoned]
        );
        assert_eq!(steps_of(action, ContextSynchronized), [Closing, Abandoned]);
        assert_eq!(steps_of(action, Closing), [PublicationPending, Abandoned]);
        assert_eq!(steps_of(action, PublicationPending), [Published, Complete]);
        assert_eq!(steps_of(action, Published), [Complete]);
        for terminal in [Complete, Abandoned, Superseded] {
            assert!(steps_of(action, terminal).is_empty(), "{terminal:?}");
            assert!(!terminal.branch_permits(terminal, action));
        }
        // No integration, no remote delete, no cleanup, never superseded.
        for refused in [
            (Closing, Integrating),
            (Closing, Superseded),
            (Closing, Published),
            (Closing, Retired),
            (ContextSynchronized, Integrating),
            (PublicationPending, RemoteDeleted),
            (PublicationPending, WorktreeRemoving),
            (Published, RemoteDeleted),
            (Confirmed, Superseded),
            (BranchDeleted, Retired),
        ] {
            assert!(!refused.0.branch_permits(refused.1, action), "{refused:?}");
        }
    }
    assert_eq!(
        steps_of(ClosureAction::NewBranch, Confirmed),
        [Closing, WorktreeRemoving, Abandoned]
    );
    assert_eq!(
        steps_of(ClosureAction::NewBranch, WorktreeRemoving),
        [WorktreeRemoved]
    );
    assert_eq!(
        steps_of(ClosureAction::NewBranch, WorktreeRemoved),
        [BranchDeleted]
    );
    // Q5: a retired branch's re-check may abandon from `branch_deleted`.
    assert_eq!(
        steps_of(ClosureAction::NewBranch, BranchDeleted),
        [Retired, Abandoned]
    );
    assert_eq!(
        steps_of(ClosureAction::NewBranch, Retired),
        [Closing, Abandoned]
    );
    assert_eq!(
        steps_of(ClosureAction::NewBranch, Closing),
        [PublicationPending]
    );
    assert_eq!(
        steps_of(ClosureAction::NewBranch, PublicationPending),
        [Published, Complete]
    );
    assert_eq!(steps_of(ClosureAction::NewBranch, Published), [Complete]);
    // A self loop is allowed on a live step.
    assert!(Closing.branch_permits(Closing, ClosureAction::Close));
    // `permits` itself is the direct graph's and never takes `retired`.
    for step in ALL_STEPS {
        assert!(
            !ClosureStep::Retired.permits(step, true, Some(ClosureKind::Record)) || step == Retired
        );
        assert!(!step.permits(Retired, true, Some(ClosureKind::Record)) || step == Retired);
    }
    // `remote_deleting` is a cleanup record's step: no direct or branch graph
    // enters it or leaves it, and the cleanup action has no branch graph.
    for step in ALL_STEPS {
        for remote in [true, false] {
            for kind in [None, Some(ClosureKind::Record), Some(ClosureKind::Existing)] {
                assert!(!step.permits(RemoteDeleting, remote, kind) || step == RemoteDeleting);
                assert!(!RemoteDeleting.permits(step, remote, kind) || step == RemoteDeleting);
            }
        }
        for action in [
            ClosureAction::Close,
            ClosureAction::Withdraw,
            ClosureAction::NewBranch,
        ] {
            assert!(!step.branch_permits(RemoteDeleting, action) || step == RemoteDeleting);
            assert!(!RemoteDeleting.branch_permits(step, action) || step == RemoteDeleting);
        }
    }
}

const ALL_STEPS: [ClosureStep; 16] = [
    ClosureStep::Confirmed,
    ClosureStep::ContextSynchronized,
    ClosureStep::Closing,
    ClosureStep::Integrating,
    ClosureStep::Integrated,
    ClosureStep::Published,
    ClosureStep::PublicationPending,
    ClosureStep::RemoteDeleting,
    ClosureStep::RemoteDeleted,
    ClosureStep::WorktreeRemoving,
    ClosureStep::WorktreeRemoved,
    ClosureStep::BranchDeleted,
    ClosureStep::Retired,
    ClosureStep::Complete,
    ClosureStep::Abandoned,
    ClosureStep::Superseded,
];

#[test]
fn closure_step_round_trips_through_parent_view() {
    for step in [
        ParentStep::Confirmed,
        ParentStep::ContextSynchronized,
        ParentStep::Integrating,
        ParentStep::Integrated,
        ParentStep::Published,
        ParentStep::PublicationPending,
        ParentStep::RemoteDeleting,
        ParentStep::RemoteDeleted,
        ParentStep::WorktreeRemoving,
        ParentStep::WorktreeRemoved,
        ParentStep::BranchDeleted,
        ParentStep::Complete,
        ParentStep::Abandoned,
        ParentStep::Superseded,
    ] {
        assert_eq!(ParentStep::from(ClosureStep::from(step)), step);
    }
    assert_eq!(
        ParentStep::from(ClosureStep::Closing),
        ParentStep::ContextSynchronized
    );
    // `retired` reads as `branch_deleted`, and writing that view back keeps it.
    assert_eq!(
        ParentStep::from(ClosureStep::Retired),
        ParentStep::BranchDeleted
    );
    let mut retired = branch_record(ClosureAction::NewBranch, ClosureStep::Retired);
    retired.set_step(ParentStep::BranchDeleted);
    assert_eq!(retired.step, ClosureStep::Retired);
    retired.step = ClosureStep::BranchDeleted;
    retired.set_step(ParentStep::BranchDeleted);
    assert_eq!(retired.step, ClosureStep::BranchDeleted);

    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    close(&tx, &mut r);
    // The runner sees closing as context_synchronized; its own column stays
    // authoritative and survives a save/load round trip unchanged.
    assert_eq!(LifecycleRecord::step(&r), ParentStep::ContextSynchronized);
    let mut viewed = r.clone();
    viewed.set_step(ParentStep::ContextSynchronized);
    assert_eq!(viewed.step, ClosureStep::Closing);
    save_record(&tx, 1, &viewed).unwrap();
    let mut loaded = read(&tx, r.operation_id);
    loaded.updated_at = r.updated_at;
    assert_eq!(loaded, r);
    // From closing the view accepts only integrating and abandoned.
    let mut wrong = loaded.clone();
    wrong.set_step(ParentStep::Integrated);
    wrong.set_integration_kind(Some(IntegrationKind::Merged));
    wrong.set_integration_oid(Some(oid('4')));
    assert!(save_record(&tx, 1, &wrong).is_err());
    let mut next = loaded.clone();
    next.set_step(ParentStep::Integrating);
    next.set_integration_kind(Some(IntegrationKind::Merged));
    assert_eq!(next.step, ClosureStep::Integrating);
    save_record(&tx, 1, &next).unwrap();
    let stored: String = tx
        .query_row(
            "SELECT step FROM ticket_closures WHERE operation_id=?1",
            [r.operation_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, "integrating");
    let mut other = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Record,
    )
    .unwrap();
    close(&tx, &mut other);
    other.set_step(ParentStep::Abandoned);
    other.set_refusal(Some(Refusal::ReconfirmationRequired));
    save_record(&tx, 1, &other).unwrap();
    assert_eq!(read(&tx, other.operation_id).step, ClosureStep::Abandoned);
}

#[test]
fn step_invariants_enforced() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    // confirmed: identity columns, expectation, closure_kind.
    let confirmed = begin(&tx, OperationId::new(), ClosureKind::Record).operation_id;
    assert!(!rejected(&tx, confirmed, "updated_at=updated_at"));
    for missing in [
        "closure_kind=NULL",
        "expectation=NULL",
        "item_id=NULL",
        "repository_id=NULL",
        "closed_at='2023-11-14T22:13:20Z'",
        "closure_oid='3333333333333333333333333333333333333333'",
    ] {
        assert!(rejected(&tx, confirmed, missing), "confirmed: {missing}");
    }
    // A refusal recorded at insertion may not know the closure effect.
    let refused = refuse_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(8),
        None,
        None,
        &Refusal::Blocked(vec![ClosureBlocker::NotATicket]),
    )
    .unwrap();
    assert_eq!(refused.closure_kind, None);
    assert_eq!(refused.step, ClosureStep::Abandoned);
    drop(tx);

    // context_synchronized: + context_oid; remote_context_oid only with a remote.
    let mut db = self::db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    synchronize(&tx, &mut r);
    let op = r.operation_id;
    assert!(!rejected(&tx, op, "updated_at=updated_at"));
    assert!(rejected(&tx, op, "context_oid=NULL"));
    assert!(rejected(&tx, op, "closure_kind=NULL"));
    assert!(rejected(
        &tx,
        op,
        "remote_context_oid='5555555555555555555555555555555555555555'"
    ));
    // closing: + base_oid, closed_at, closure_oid; record closures only.
    begin_closing(&tx, 1, op, oid('2'), at(), oid('3')).unwrap();
    assert!(!rejected(&tx, op, "updated_at=updated_at"));
    for missing in [
        "base_oid=NULL",
        "closed_at=NULL",
        "closure_oid=NULL",
        "context_oid=NULL",
        "closure_kind='existing'",
        "closure_kind=NULL",
    ] {
        assert!(rejected(&tx, op, missing), "closing: {missing}");
    }
    // integrating: + base_oid (all kinds).
    r = read(&tx, op);
    r.integration_kind = Some(IntegrationKind::Merged);
    r.step = ClosureStep::Integrating;
    save_record(&tx, 1, &r).unwrap();
    assert!(!rejected(&tx, op, "updated_at=updated_at"));
    for missing in ["base_oid=NULL", "closed_at=NULL", "closure_oid=NULL"] {
        assert!(rejected(&tx, op, missing), "integrating: {missing}");
    }
    let mut e = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Existing,
    )
    .unwrap();
    synchronize(&tx, &mut e);
    e.base_oid = Some(oid('2'));
    e.integration_kind = Some(IntegrationKind::AlreadyContained);
    e.step = ClosureStep::Integrating;
    save_record(&tx, 1, &e).unwrap();
    assert!(!rejected(&tx, e.operation_id, "updated_at=updated_at"));
    for wrong in [
        "base_oid=NULL",
        "closed_at='2023-11-14T22:13:20Z',closure_oid='3333333333333333333333333333333333333333'",
    ] {
        assert!(
            rejected(&tx, e.operation_id, wrong),
            "existing integrating: {wrong}"
        );
    }
    // integrated onward: + integration_oid, integration_kind.
    r.integration_oid = Some(oid('4'));
    r.step = ClosureStep::Integrated;
    save_record(&tx, 1, &r).unwrap();
    assert!(!rejected(&tx, op, "updated_at=updated_at"));
    for missing in ["integration_oid=NULL", "integration_kind=NULL"] {
        assert!(rejected(&tx, op, missing), "integrated: {missing}");
    }
    r.publication = Some(PublicationCategory::NoPublicationRemote);
    r.step = ClosureStep::PublicationPending;
    save_record(&tx, 1, &r).unwrap();
    for missing in [
        "integration_oid=NULL",
        "integration_kind=NULL",
        "publication_category=NULL",
    ] {
        assert!(rejected(&tx, op, missing), "publication_pending: {missing}");
    }
    drop(tx);

    // The path columns: each CHECK of the v3 table refuses its bad row and
    // accepts the twin that differs by the one thing it checks.
    let mut db = self::db();
    let tx = db.transaction().unwrap();
    let ts = "'2023-11-14T22:13:20Z'";
    let (o1, o2, o3, o5, o6) = (q('1'), q('2'), q('3'), q('5'), q('6'));
    let written = |action: &str, step: &str, n: u8, extra: &[(&str, &str)]| {
        let mut columns = vec![
            ("closure_kind", "'record'"),
            ("context_oid", &o1[..]),
            ("base_oid", &o2[..]),
            ("closed_at", ts),
            ("closure_oid", &o3[..]),
        ];
        columns.extend_from_slice(extra);
        branch_row(&tx, &item_n(n), action, step, &columns)
    };
    let existing = |action: &str, step: &str, n: u8, extra: &[(&str, &str)]| {
        let mut columns = vec![
            ("closure_kind", "'existing'"),
            ("context_oid", &o1[..]),
            ("base_oid", &o2[..]),
        ];
        columns.extend_from_slice(extra);
        branch_row(&tx, &item_n(n), action, step, &columns)
    };
    let close_record = written("close", "closing", 1, &[]);
    let close_existing = existing("close", "closing", 2, &[]);
    let pending = existing(
        "close",
        "publication_pending",
        3,
        &[("publication_category", "'transport'")],
    );
    let withdraw = existing("withdraw", "closing", 4, &[("withdraw_oid", &o5[..])]);
    let new_branch = written(
        "new_branch",
        "closing",
        5,
        &[
            ("retired_oid", &o6[..]),
            ("pending_ref", "'refs/manyhands/closures/{op}'"),
        ],
    );
    let synchronized = existing("close", "context_synchronized", 6, &[]);
    let complete = existing("close", "complete", 7, &[("answer_code", "'published'")]);
    let direct = {
        let mut r = begin_record(
            &tx,
            1,
            OperationId::new(),
            &item(&item_n(8)),
            digest(7),
            Some("origin"),
            ClosureKind::Record,
        )
        .unwrap();
        close(&tx, &mut r);
        r.operation_id
    };
    // A direct row at `complete`: a branch record's answer is not its.
    let direct_complete = {
        let op = OperationId::new();
        tx.execute_batch(&format!(
            "INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,closure_kind,context_oid,base_oid,closed_at,closure_oid,integration_kind,integration_oid,publication_category) VALUES(1,'{op}','{}',x'{}','origin','complete',1,'record',{o1},{o2},{ts},{o3},'merged',{o1},'published')",
            item_n(9),
            "07".repeat(32),
        ))
        .unwrap();
        op
    };
    // A refusal stored with no remote is still a readable row.
    let abandoned_without_remote = branch_row(
        &tx,
        &item_n(10),
        "close",
        "abandoned",
        &[
            ("remote_name", "NULL"),
            ("closure_kind", "'existing'"),
            ("refusal_category", "'external_change'"),
            ("answer_code", "'external_change'"),
        ],
    );
    // Every row above is accepted as it stands.
    for op in [
        direct_complete,
        abandoned_without_remote,
        close_record,
        close_existing,
        pending,
        withdraw,
        new_branch,
        synchronized,
        complete,
        direct,
    ] {
        assert!(!rejected(&tx, op, "updated_at=updated_at"), "{op}");
    }
    // (row, assignment the schema refuses, twin the schema and the read accept)
    let checks: Vec<(OperationId, &str, &str)> = vec![
        // The mode and the action are closed sets.
        (
            close_record,
            "integration_mode='merge'",
            "integration_mode='direct'",
        ),
        (close_record, "action='retire'", "action='close'"),
        // Only a pull-request mode has a new branch.
        (
            new_branch,
            "integration_mode='direct'",
            "closure_kind='existing'",
        ),
        // A branch record never integrates.
        (
            close_record,
            "integration_kind='merged'",
            "integration_mode='direct',integration_kind='merged'",
        ),
        (
            close_record,
            "integration_oid='4444444444444444444444444444444444444444'",
            "integration_mode='direct',integration_oid='4444444444444444444444444444444444444444'",
        ),
        (
            close_record,
            "step='integrating',integration_kind='merged'",
            "integration_mode='direct',step='integrating',integration_kind='merged'",
        ),
        // A direct row carries no branch-only column and never `retired`.
        (
            direct,
            "withdraw_oid='5555555555555555555555555555555555555555'",
            "updated_at=updated_at",
        ),
        (
            direct,
            "retired_oid='6666666666666666666666666666666666666666'",
            "updated_at=updated_at",
        ),
        (
            direct,
            "pending_ref='refs/manyhands/closures/x'",
            "updated_at=updated_at",
        ),
        (direct, "answer_code='published'", "updated_at=updated_at"),
        (
            direct,
            "consented_context_oid='1111111111111111111111111111111111111111'",
            "integration_mode='pull_request',consented_context_oid='1111111111111111111111111111111111111111'",
        ),
        (direct, "step='retired'", "updated_at=updated_at"),
        (
            direct_complete,
            "answer_code='published'",
            "updated_at=updated_at",
        ),
        (
            abandoned_without_remote,
            "step='complete'",
            "answer_code='merged'",
        ),
        // Cleanup steps belong to a direct close and a new branch.
        (
            close_record,
            "step='worktree_removing'",
            "action='new_branch',step='worktree_removing'",
        ),
        (
            close_existing,
            "step='branch_deleted'",
            "updated_at=updated_at",
        ),
        // A new branch has no synchronized-context step.
        (synchronized, "action='new_branch'", "action='withdraw'"),
        // `closing` per action.
        (
            close_existing,
            "integration_mode='direct'",
            "closed_at=NULL",
        ),
        (
            close_existing,
            "closure_kind='record'",
            "closure_kind='existing'",
        ),
        (
            close_record,
            "closed_at=NULL,closure_oid=NULL",
            "closure_kind='record'",
        ),
        (
            withdraw,
            "withdraw_oid=NULL",
            "withdraw_oid='5555555555555555555555555555555555555555'",
        ),
        (
            withdraw,
            "closed_at='2023-11-14T22:13:20Z',closure_oid='3333333333333333333333333333333333333333'",
            "updated_at=updated_at",
        ),
        (withdraw, "closure_kind='record'", "updated_at=updated_at"),
        // `closed_at` without a record closure: only a new branch.
        (
            close_existing,
            "closed_at='2023-11-14T22:13:20Z',closure_oid='3333333333333333333333333333333333333333'",
            "updated_at=updated_at",
        ),
        (
            new_branch,
            "closed_at=NULL,closure_oid=NULL",
            "closure_kind='existing'",
        ),
        // An adopted context is a direct record's.
        (
            close_record,
            "adopted_context_oid='7777777777777777777777777777777777777777'",
            "integration_mode='direct',adopted_context_oid='7777777777777777777777777777777777777777'",
        ),
        // `withdraw_oid`: only a withdraw, and not before `closing`.
        (
            close_existing,
            "withdraw_oid='5555555555555555555555555555555555555555'",
            "action='withdraw',withdraw_oid='5555555555555555555555555555555555555555'",
        ),
        (
            withdraw,
            "step='confirmed'",
            "step='publication_pending',publication_category='transport'",
        ),
        // `retired_oid` and `pending_ref`: only a new branch.
        (
            close_record,
            "retired_oid='6666666666666666666666666666666666666666'",
            "updated_at=updated_at",
        ),
        (
            close_record,
            "pending_ref='refs/manyhands/closures/'||operation_id",
            "updated_at=updated_at",
        ),
        (
            new_branch,
            "pending_ref='refs/manyhands/closures/other'",
            "updated_at=updated_at",
        ),
        (
            new_branch,
            "step='complete',answer_code='published'",
            "step='publication_pending',publication_category='transport'",
        ),
        // `answer_code`: its shape, and only at `complete` or `abandoned`.
        (
            complete,
            "answer_code='Published'",
            "answer_code='kept_published'",
        ),
        (complete, "answer_code='publish ed'", "answer_code='merged'"),
        (complete, "answer_code=''", "answer_code='closed_elsewhere'"),
        (complete, "answer_code=1", "answer_code='remote_changed'"),
        (
            complete,
            "answer_code='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'",
            "answer_code='ticket_closed'",
        ),
        (
            pending,
            "answer_code='published'",
            "step='complete',answer_code='published'",
        ),
        (
            pending,
            "step='complete'",
            "step='complete',answer_code='existing'",
        ),
        // `consented_context_oid` is an OID.
        (
            close_record,
            "consented_context_oid='nope'",
            "consented_context_oid='1111111111111111111111111111111111111111'",
        ),
        (
            close_record,
            "consented_context_oid='0000000000000000000000000000000000000000'",
            "consented_context_oid='1111111111111111111111111111111111111111'",
        ),
    ];
    for (op, bad, twin) in checks {
        assert!(schema_refuses(&tx, op, bad), "the schema accepts: {bad}");
        assert!(!schema_refuses(&tx, op, twin), "the schema refuses: {twin}");
        assert!(!rejected(&tx, op, twin), "the read refuses: {twin}");
    }
    // Rules the read enforces as well: the stored values, and a branch
    // record's shape that no CHECK can see.
    for (op, bad) in [
        (complete, "answer_code='unknown_answer'"),
        (pending, "publication_category='published'"),
        (pending, "remote_name=NULL"),
        (close_record, "step='retired'"),
        (close_record, "context_oid=NULL"),
        (close_record, "base_oid=NULL"),
        (close_existing, "integration_kind='merged'"),
        (new_branch, "cleanup_category='context_changed'"),
    ] {
        assert!(rejected(&tx, op, bad), "{bad}");
    }
}

#[test]
fn context_oid_immutable_once_set() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    synchronize(&tx, &mut r);
    let mut moved = r.clone();
    moved.context_oid = Some(oid('9'));
    assert!(save_record(&tx, 1, &moved).is_err());
    let mut cleared = r.clone();
    cleared.context_oid = None;
    assert!(save_record(&tx, 1, &cleared).is_err());
    close(&tx, &mut r);
    // The checkpoint replaces nothing but closure_oid.
    let mut moved = r.clone();
    moved.context_oid = Some(oid('9'));
    assert!(save_record(&tx, 1, &moved).is_err());
    replace_unmoved_closure(&tx, 1, r.operation_id, oid('8')).unwrap();
    let replaced = read(&tx, r.operation_id);
    assert_eq!(replaced.context_oid, Some(oid('1')));
    assert_eq!(replaced.closure_oid, Some(oid('8')));
    assert_eq!(replaced.closed_at, r.closed_at);
    assert_eq!(replaced.base_oid, r.base_oid);
    // closure_oid and closed_at never change through an ordinary save.
    let mut sneaked = replaced.clone();
    sneaked.closure_oid = Some(oid('7'));
    assert!(save_record(&tx, 1, &sneaked).is_err());
    let mut sneaked = replaced.clone();
    sneaked.closed_at = Some(at() + time::Duration::seconds(1));
    assert!(save_record(&tx, 1, &sneaked).is_err());
    // Replacement only while closing.
    let mut r = replaced;
    integrate(&tx, &mut r);
    assert!(replace_unmoved_closure(&tx, 1, r.operation_id, oid('6')).is_err());
    // The sole back-transition is the primary-advance reset before closing.
    let mut db = self::db();
    let tx = db.transaction().unwrap();
    state::configure(
        &tx,
        1,
        Some(&RemoteRefPlan::from_configuration("origin", "main").unwrap()),
        state::ConfigurationChange::Plan,
    )
    .unwrap();
    let mut r = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(ITEM),
        digest(7),
        Some("origin"),
        ClosureKind::Record,
    )
    .unwrap();
    synchronize(&tx, &mut r);
    let reset = reset_context_after_primary_advance(&tx, 1, r.operation_id).unwrap();
    assert_eq!(reset.step, ClosureStep::Confirmed);
    assert_eq!(reset.context_oid, None);
    assert_eq!(reset.context_attempt, 1);
    let mut r = reset;
    synchronize(&tx, &mut r);
    begin_closing(&tx, 1, r.operation_id, oid('2'), at(), oid('3')).unwrap();
    assert!(reset_context_after_primary_advance(&tx, 1, r.operation_id).is_err());
    assert_eq!(read(&tx, r.operation_id).context_oid, Some(oid('1')));
}

#[test]
fn merged_context_prefers_closure_oid() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    assert_eq!(r.merged_context(), None);
    synchronize(&tx, &mut r);
    assert_eq!(r.merged_context(), Some(oid('1')));
    close(&tx, &mut r);
    assert_eq!(r.merged_context(), Some(oid('3')));
    let mut e = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Existing,
    )
    .unwrap();
    synchronize(&tx, &mut e);
    integrate(&tx, &mut e);
    assert_eq!(e.merged_context(), Some(oid('1')));
}

#[test]
fn one_unfinished_row_per_item() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let first = begin(&tx, OperationId::new(), ClosureKind::Record);
    for kind in [ClosureKind::Record, ClosureKind::Existing] {
        assert_eq!(
            kind_of(begin_record(
                &tx,
                1,
                OperationId::new(),
                &item(ITEM),
                digest(9),
                None,
                kind
            )),
            RepositoryErrorKind::RepositoryBusy
        );
    }
    // Another item is independent.
    begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Record,
    )
    .unwrap();
    // The schema refuses a second unfinished row for the item outright.
    assert!(tx.execute("INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,step,updated_at,closure_kind) VALUES(1,?1,?2,zeroblob(32),'confirmed',1,'record')", params![OperationId::new().to_string(), ITEM]).is_err());
    // Once the first ends, a new ID may begin.
    let mut ended = first.clone();
    ended.step = ClosureStep::Abandoned;
    ended.refusal = Some(Refusal::ExternalChange);
    save_record(&tx, 1, &ended).unwrap();
    begin(&tx, OperationId::new(), ClosureKind::Existing);
}

#[test]
fn closing_row_makes_new_id_busy() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    close(&tx, &mut r);
    let busy = |tx: &Transaction<'_>| {
        assert_eq!(
            kind_of(begin_record(
                tx,
                1,
                OperationId::new(),
                &item(ITEM),
                digest(9),
                None,
                ClosureKind::Record
            )),
            RepositoryErrorKind::RepositoryBusy
        );
        assert_eq!(
            kind_of(refuse_record(
                tx,
                1,
                OperationId::new(),
                &item(ITEM),
                digest(9),
                None,
                Some(ClosureKind::Record),
                &Refusal::ExternalChange
            )),
            RepositoryErrorKind::RepositoryBusy
        );
    };
    busy(&tx);
    // Even a supersedable cleanup category never supersedes a written closure.
    tx.execute(
        "UPDATE ticket_closures SET cleanup_category='context_changed' WHERE operation_id=?1",
        [r.operation_id.to_string()],
    )
    .unwrap();
    busy(&tx);
    assert_eq!(read(&tx, r.operation_id).step, ClosureStep::Closing);
    // The same ID replays it.
    assert_eq!(
        begin(&tx, r.operation_id, ClosureKind::Record).step,
        ClosureStep::Closing
    );
}

#[test]
fn supersession_only_from_context_changed_ignored_remote_changed() {
    for category in [
        CleanupCategory::ContextChanged,
        CleanupCategory::IgnoredFiles,
        CleanupCategory::RemoteContextChanged,
        CleanupCategory::RecoveryRequired,
        CleanupCategory::Cancelled,
        CleanupCategory::AwaitingPublication,
    ] {
        let mut db = db();
        let tx = db.transaction().unwrap();
        let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
        close(&tx, &mut r);
        integrate(&tx, &mut r);
        r.step = ClosureStep::PublicationPending;
        r.publication = Some(PublicationCategory::NoPublicationRemote);
        r.cleanup = Some(category);
        save_record(&tx, 1, &r).unwrap();
        let next = OperationId::new();
        let result = begin_record(
            &tx,
            1,
            next,
            &r.item_id,
            digest(9),
            None,
            ClosureKind::Existing,
        );
        if category.supersedable() {
            assert_eq!(result.unwrap().step, ClosureStep::Confirmed);
            let old = read(&tx, r.operation_id);
            assert_eq!(old.step, ClosureStep::Superseded);
            assert_eq!(old.closure_oid, Some(oid('3')));
            assert!(save_record(&tx, 1, &r).is_err());
        } else {
            assert_eq!(
                kind_of(result),
                RepositoryErrorKind::RepositoryBusy,
                "{category:?}"
            );
            assert_eq!(
                read(&tx, r.operation_id).step,
                ClosureStep::PublicationPending
            );
        }
    }
}

#[test]
fn abandoned_refusal_replays() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    for refusal in [
        Refusal::ExternalChange,
        Refusal::ReconfirmationRequired,
        Refusal::Blocked(vec![
            ClosureBlocker::ContextDirty,
            ClosureBlocker::CommentPublicationPending,
            ClosureBlocker::IdentityRequired,
        ]),
    ] {
        let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
        r.refusal = Some(refusal.clone());
        r.step = ClosureStep::Abandoned;
        save_record(&tx, 1, &r).unwrap();
        let replay = begin(&tx, r.operation_id, ClosureKind::Record);
        assert_eq!(replay.step, ClosureStep::Abandoned);
        assert_eq!(replay.refusal, Some(refusal.clone()));
        for (other_item, other_digest) in [(OTHER, 7), (ITEM, 8)] {
            assert_eq!(
                kind_of(begin_record(
                    &tx,
                    1,
                    r.operation_id,
                    &item(other_item),
                    digest(other_digest),
                    None,
                    ClosureKind::Record
                )),
                RepositoryErrorKind::OperationMismatch
            );
        }
        // A refusal inserted as such replays the same way.
        let op = OperationId::new();
        let inserted = refuse_record(
            &tx,
            1,
            op,
            &item(ITEM),
            digest(7),
            None,
            Some(ClosureKind::Existing),
            &refusal,
        )
        .unwrap();
        assert_eq!(inserted.refusal, Some(refusal.clone()));
        assert_eq!(begin(&tx, op, ClosureKind::Existing).refusal, Some(refusal));
    }
}

#[test]
fn blocker_mask_round_trips_all_22() {
    assert_eq!(CLOSURE_BLOCKERS.len(), 22);
    assert!(CLOSURE_BLOCKERS.windows(2).all(|p| p[0] < p[1]));
    assert_eq!(CLOSURE_BLOCKERS[0], ClosureBlocker::NotATicket);
    assert_eq!(CLOSURE_BLOCKERS[20], ClosureBlocker::IdentityRequired);
    assert_eq!(CLOSURE_BLOCKERS[21], ClosureBlocker::HistoryUnresolved);
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut all = Vec::new();
    for (bit, blocker) in CLOSURE_BLOCKERS.iter().enumerate() {
        all.push(*blocker);
        let refusal = Refusal::Blocked(vec![*blocker]);
        let r = refuse_record(
            &tx,
            1,
            OperationId::new(),
            &item(ITEM),
            digest(7),
            None,
            None,
            &refusal,
        )
        .unwrap();
        let mask: i64 = tx
            .query_row(
                "SELECT refusal_blockers FROM ticket_closures WHERE operation_id=?1",
                [r.operation_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mask, 1 << bit);
        assert_eq!(read(&tx, r.operation_id).refusal, Some(refusal));
    }
    let every = Refusal::Blocked(all);
    let r = refuse_record(
        &tx,
        1,
        OperationId::new(),
        &item(ITEM),
        digest(7),
        None,
        None,
        &every,
    )
    .unwrap();
    assert_eq!(read(&tx, r.operation_id).refusal, Some(every));
    assert!(!rejected(&tx, r.operation_id, "refusal_blockers=4194303"));
    for wrong in ["refusal_blockers=0", "refusal_blockers=4194304"] {
        assert!(rejected(&tx, r.operation_id, wrong), "{wrong}");
    }
    // An empty or unordered set is never stored.
    for wrong in [
        vec![],
        vec![ClosureBlocker::IdentityRequired, ClosureBlocker::NotATicket],
    ] {
        assert!(
            refuse_record(
                &tx,
                1,
                OperationId::new(),
                &item(ITEM),
                digest(7),
                None,
                None,
                &Refusal::Blocked(wrong)
            )
            .is_err()
        );
    }
}

#[test]
fn closed_at_stored_as_whole_utc_seconds() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    synchronize(&tx, &mut r);
    // A sub-second time is refused, not rounded.
    let fractional = at() + time::Duration::milliseconds(500);
    assert!(begin_closing(&tx, 1, r.operation_id, oid('2'), fractional, oid('3')).is_err());
    assert_eq!(
        read(&tx, r.operation_id).step,
        ClosureStep::ContextSynchronized
    );
    // Another offset is stored as the same instant in UTC.
    let offset = at().to_offset(time::UtcOffset::from_hms(5, 30, 0).unwrap());
    begin_closing(&tx, 1, r.operation_id, oid('2'), offset, oid('3')).unwrap();
    let text: String = tx
        .query_row(
            "SELECT closed_at FROM ticket_closures WHERE operation_id=?1",
            [r.operation_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(text, "2023-11-14T22:13:20Z");
    let stored = read(&tx, r.operation_id).closed_at.unwrap();
    assert_eq!(stored, at());
    assert_eq!(stored.offset(), time::UtcOffset::UTC);
    for wrong in [
        "closed_at='2023-11-14T22:13:20.5Z'",
        "closed_at='2023-11-14T22:13:20+00:00'",
        "closed_at='2023-11-15T03:43:20+05:30'",
        "closed_at='2023-11-14 22:13:20Z'",
        "closed_at='2023-13-14T22:13:20Z'",
        "closed_at=1700000000",
    ] {
        assert!(rejected(&tx, r.operation_id, wrong), "{wrong}");
    }
}

#[test]
fn no_closed_by_column_or_value() {
    let mut db = db();
    let columns = db
        .prepare("PRAGMA table_info(ticket_closures)")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        columns,
        [
            "repository_id",
            "operation_id",
            "item_id",
            "expectation",
            "remote_name",
            "step",
            "context_attempt",
            "context_sync_id",
            "primary_attempt",
            "primary_sync_id",
            "base_oid",
            "context_oid",
            "integration_oid",
            "integration_kind",
            "remote_context_oid",
            "publication_category",
            "cleanup_category",
            "cancel_requested",
            "updated_at",
            "refusal_category",
            "refusal_blockers",
            "closure_kind",
            "closed_at",
            "closure_oid",
            "adopted_context_oid",
            "integration_mode",
            "action",
            "withdraw_oid",
            "retired_oid",
            "pending_ref",
            "answer_code",
            "consented_context_oid",
            "unheld_digest",
            "primary_branch",
            "endpoint_digest",
            "remote_branch",
        ]
    );
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    close(&tx, &mut r);
    integrate(&tx, &mut r);
    // No stored value and no Debug output carries the closer's identity.
    let mut q = tx.prepare("SELECT * FROM ticket_closures").unwrap();
    let count = q.column_count();
    let values = q
        .query_map([], |row| {
            (0..count)
                .map(|i| row.get::<_, rusqlite::types::Value>(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(!values.is_empty());
    for value in values.iter().flatten() {
        if let rusqlite::types::Value::Text(text) = value {
            assert!(
                !text.contains(CLOSED_BY) && !text.contains('@') && !text.contains('<'),
                "{text}"
            );
        }
    }
    let debug = format!("{:?}", read(&tx, r.operation_id));
    assert!(
        !debug.contains("closed_by") && !debug.contains('@'),
        "{debug}"
    );
    assert!(format!("{:?}", digest(7)).contains("redacted"));
}

/// Promotion and closure parents, their children and every other journal
/// share one operation ID namespace.
#[test]
fn operation_ids_collide_across_both_parent_tables_and_journals() {
    let (data, root, service) = service_fixture();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let (promotion_parent, promotion_child, closure_parent, closure_child) =
        state::with_transaction(&service, root.path(), |tx, id| {
            let p = promotion::begin_record(
                tx,
                id,
                OperationId::new(),
                &item(OTHER),
                PromotionExpectation::from_bytes([7; 32]),
                Some("origin"),
            )?;
            let p = promotion::next_attempt(tx, id, p.operation_id, promotion::ChildRole::Context)?;
            let c = begin_record(
                tx,
                id,
                OperationId::new(),
                &item(ITEM),
                digest(7),
                Some("origin"),
                ClosureKind::Record,
            )?;
            let c = next_attempt(tx, id, c.operation_id, ChildRole::Context)?;
            Ok((
                p.operation_id,
                p.context_sync_id.unwrap(),
                c.operation_id,
                c.context_sync_id.unwrap(),
            ))
        })
        .unwrap();
    let begin_closure = |op| {
        state::with_transaction(&service, root.path(), |tx, id| {
            begin_record(
                tx,
                id,
                op,
                &item(ITEM),
                digest(7),
                Some("origin"),
                ClosureKind::Record,
            )
        })
    };
    let begin_promotion = |op| {
        state::with_transaction(&service, root.path(), |tx, id| {
            promotion::begin_record(
                tx,
                id,
                op,
                &item(OTHER),
                PromotionExpectation::from_bytes([7; 32]),
                Some("origin"),
            )
        })
    };
    // Each parent table refuses the other's parent and child IDs.
    for op in [promotion_parent, promotion_child] {
        assert_eq!(
            kind_of(begin_closure(op)),
            RepositoryErrorKind::OperationMismatch
        );
    }
    for op in [closure_parent, closure_child] {
        assert_eq!(
            kind_of(begin_promotion(op)),
            RepositoryErrorKind::OperationMismatch
        );
    }
    // Both replay their own ID.
    assert_eq!(
        begin_closure(closure_parent).unwrap().operation_id,
        closure_parent
    );
    assert_eq!(
        begin_promotion(promotion_parent).unwrap().operation_id,
        promotion_parent
    );
    // The local journal cannot begin with a closure parent or unrun child ID.
    let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    for op in [closure_parent, closure_child] {
        assert_eq!(
            kind_of(recovery::begin_or_reconcile_operation(
                &mut db,
                root.path(),
                RepositoryOperation::RefreshRepository,
                op,
                "",
                0
            )),
            RepositoryErrorKind::OperationMismatch
        );
        assert_eq!(
            kind_of(recovery::require_refresh_can_begin(
                &db,
                root.path(),
                RepositoryOperation::CreateFolder,
                op
            )),
            RepositoryErrorKind::OperationMismatch
        );
    }
    // The remote journal cannot reserve a closure ID for another target.
    let ticket_sync = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        item(ITEM),
    )
    .unwrap();
    assert_eq!(
        kind_of(service.reserve_remote_operation(root.path(), closure_parent, &ticket_sync)),
        RepositoryErrorKind::OperationMismatch
    );
    let close = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::Close,
        AuthoringKind::Ticket,
        item(ITEM),
    )
    .unwrap();
    assert_eq!(
        kind_of(service.reserve_remote_operation(root.path(), closure_child, &close)),
        RepositoryErrorKind::OperationMismatch
    );
    // Both parents and their children pass the startup audit.
    RepositoryService::open_at(data.path()).unwrap();
    // A closure cannot take an ID another journal already holds.
    let reserved = OperationId::new();
    service
        .reserve_remote_operation(root.path(), reserved, &ticket_sync)
        .unwrap();
    db.execute(
        "UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled' WHERE operation_ulid=?1",
        [reserved.to_string()],
    )
    .unwrap();
    let local = OperationId::new();
    let binding = OperationId::new();
    let synchronization = OperationId::new();
    db.execute("INSERT INTO operation_records(operation_ulid,repository_id,root_path,action,target,state,observed_at) VALUES(?1,1,?2,'refresh','','completed',123)",params![local.to_string(),root.path().to_str().unwrap()]).unwrap();
    db.execute("INSERT INTO comment_publication_bindings(operation_ulid,local_record_id,root_digest,kind,item_ulid,comment_ulid,synchronization_ulid,pre_checkpoint_oid) VALUES(?1,(SELECT id FROM operation_records WHERE operation_ulid=?2),zeroblob(32),'ticket',?3,?4,?5,?6)",params![binding.to_string(),local.to_string(),ITEM,OTHER,synchronization.to_string(),oid('1').to_string()]).unwrap();
    for op in [reserved, local, binding, synchronization] {
        assert_eq!(
            kind_of(begin_closure(op)),
            RepositoryErrorKind::OperationMismatch
        );
    }
    // A stored collision between the two parent tables fails both audits.
    db.execute("INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,step,updated_at,closure_kind,refusal_category) VALUES(1,?1,?2,zeroblob(32),'abandoned',1,'record','external_change')",params![promotion_parent.to_string(),OTHER]).unwrap();
    assert!(audit(&db).is_err());
    assert!(promotion::audit(&db).is_err());
}

#[test]
fn service_start_audits_closure_rows_and_schema() {
    let (data, root, service) = service_fixture();
    state::with_transaction(&service, root.path(), |tx, id| {
        let r = begin_record(
            tx,
            id,
            OperationId::new(),
            &item(ITEM),
            digest(7),
            Some("origin"),
            ClosureKind::Record,
        )?;
        next_attempt(tx, id, r.operation_id, ChildRole::Context)?;
        Ok(())
    })
    .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("DROP INDEX ticket_closures_unfinished")
        .unwrap();
    assert_eq!(
        RepositoryService::open_at(data.path()).err().unwrap().kind,
        RepositoryErrorKind::RecoveryRequired
    );
}

fn service_fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,?1,123,'accessible',0)",[root.path().to_str().unwrap()]).unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(
            tx,
            id,
            Some(&RemoteRefPlan::from_configuration("origin", "main").unwrap()),
            state::ConfigurationChange::Plan,
        )
    })
    .unwrap();
    (data, root, service)
}

#[test]
fn adopted_context_invariants() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin(&tx, OperationId::new(), ClosureKind::Record);
    // Never before closing: there is no checkpoint to descend from.
    synchronize(&tx, &mut r);
    assert!(rejected(
        &tx,
        r.operation_id,
        "adopted_context_oid='5555555555555555555555555555555555555555'"
    ));
    close(&tx, &mut r);
    // While closing: a commit other than the checkpoint (Git ancestry is
    // checked by `adopt_written_closure`, which holds the repository).
    let adopted = "adopted_context_oid='5555555555555555555555555555555555555555'";
    assert!(!rejected(&tx, r.operation_id, adopted));
    for wrong in [
        "adopted_context_oid='3333333333333333333333333333333333333333'",
        "adopted_context_oid='0000000000000000000000000000000000000000'",
        "adopted_context_oid='not an oid'",
    ] {
        assert!(rejected(&tx, r.operation_id, wrong), "{wrong}");
    }
    tx.execute(
        &format!("UPDATE ticket_closures SET {adopted} WHERE operation_id=?1"),
        [r.operation_id.to_string()],
    )
    .unwrap();
    let stored = read(&tx, r.operation_id);
    assert_eq!(stored.adopted_context_oid, Some(oid('5')));
    assert_eq!(stored.closure_oid, Some(oid('3')));
    // The merged context prefers the adopted synchronization.
    assert_eq!(stored.merged_context(), Some(oid('5')));
    // Without the checkpoint it cannot stand.
    assert!(rejected(
        &tx,
        r.operation_id,
        "closed_at=NULL,closure_oid=NULL"
    ));
    // An ordinary save never sets, changes or clears it, at any step.
    for changed in [None, Some(oid('6'))] {
        let mut sneaked = stored.clone();
        sneaked.adopted_context_oid = changed;
        assert!(save_record(&tx, 1, &sneaked).is_err(), "{changed:?}");
    }
    let mut r = stored;
    integrate(&tx, &mut r);
    let mut sneaked = read(&tx, r.operation_id);
    sneaked.adopted_context_oid = None;
    assert!(save_record(&tx, 1, &sneaked).is_err());
    assert_eq!(
        read(&tx, r.operation_id).adopted_context_oid,
        Some(oid('5'))
    );
    // Never for an `existing` closure, which has no checkpoint.
    let mut e = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Existing,
    )
    .unwrap();
    synchronize(&tx, &mut e);
    integrate(&tx, &mut e);
    assert!(rejected(&tx, e.operation_id, adopted));
    tx.commit().unwrap();
    // The startup audit reads the column through the same validation.
    audit(&db).unwrap();
}

/// R-T3-3. Without a context child (a contained closure; the runner passes
/// `None` only for an absent child) the remote context delete is guarded by
/// the pre-checkpoint tracking OID the observation recorded, never by an
/// absent publication; with a child that ran, by its verified publication
/// (T7-1), never by the tracking evidence.
#[test]
fn remote_delete_guard_uses_tracking_without_a_context_child_else_its_publication() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let tracking = oid('1');
    let published = oid('5');
    let mut contained = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(ITEM),
        digest(7),
        Some("origin"),
        ClosureKind::Record,
    )
    .unwrap();
    contained.remote_context_oid = Some(tracking);
    assert_eq!(contained.context_sync_id, None);
    assert_eq!(contained.remote_delete_guard(None), Some(tracking));
    // No child and no tracking context: no guard, so no delete is attempted.
    contained.remote_context_oid = None;
    assert_eq!(contained.remote_delete_guard(None), None);

    let mut with_child = next_attempt(&tx, 1, contained.operation_id, ChildRole::Context).unwrap();
    with_child.remote_context_oid = Some(tracking);
    assert!(with_child.context_sync_id.is_some());
    // The child's publication guards, not the differing tracking evidence.
    assert_eq!(
        with_child.remote_delete_guard(Some(published)),
        Some(published)
    );
    // R-T5-2: a minted attempt that never ran (absent, e.g. skipped after a
    // primary advance made the context contained) has no guard; only a
    // closure with no context attempt falls back to the tracking OID.
    assert_eq!(with_child.remote_delete_guard(None), None);
}

/// R-T5-3: `remote_context_oid` is the tracking OID observed when the
/// context is synchronized. It is set only with `confirmed →
/// context_synchronized`; no later save may add one.
#[test]
fn remote_context_oid_is_set_only_with_context_synchronized() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let begin_remote = |item_text: &str, kind| {
        begin_record(
            &tx,
            1,
            OperationId::new(),
            &item(item_text),
            digest(7),
            Some("origin"),
            kind,
        )
        .unwrap()
    };
    // Allowed: with the synchronization itself.
    let mut r = begin_remote(ITEM, ClosureKind::Record);
    r.remote_context_oid = Some(oid('1'));
    synchronize(&tx, &mut r);
    assert_eq!(read(&tx, r.operation_id).remote_context_oid, Some(oid('1')));
    // Refused: at `confirmed` without the synchronization.
    let mut early = begin_remote(OTHER, ClosureKind::Record);
    early.remote_context_oid = Some(oid('1'));
    assert_eq!(
        kind_of(save_record(&tx, 1, &early)),
        RepositoryErrorKind::RecoveryRequired
    );
    // Refused: after it, at every later step, when none was observed.
    early.remote_context_oid = None;
    close(&tx, &mut early);
    assert_eq!(early.remote_context_oid, None);
    let mut later = early.clone();
    later.remote_context_oid = Some(oid('1'));
    assert_eq!(
        kind_of(save_record(&tx, 1, &later)),
        RepositoryErrorKind::RecoveryRequired
    );
    integrate(&tx, &mut early);
    for step in [ClosureStep::Integrated, ClosureStep::PublicationPending] {
        let mut later = early.clone();
        later.step = step;
        later.publication = Some(PublicationCategory::Transport);
        later.remote_context_oid = Some(oid('1'));
        assert_eq!(
            kind_of(save_record(&tx, 1, &later)),
            RepositoryErrorKind::RecoveryRequired,
            "{step:?}"
        );
    }
    // The same save without it is accepted.
    early.step = ClosureStep::PublicationPending;
    early.publication = Some(PublicationCategory::Transport);
    save_record(&tx, 1, &early).unwrap();
    assert_eq!(read(&tx, early.operation_id).remote_context_oid, None);
}

/// The design's cleanup row: a remote context branch is deleted (and the
/// close reservation required) unless no context child ran and no tracking
/// context was observed. A contained `record` closure integrates as a merge,
/// so promotion's `already_contained` rule does not decide it.
#[test]
fn remote_deletion_required_unless_contained_without_tracking() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut r = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(ITEM),
        digest(7),
        Some("origin"),
        ClosureKind::Record,
    )
    .unwrap();
    r.integration_kind = Some(IntegrationKind::Merged);
    // Contained, no child, no tracking context: nothing to delete remotely.
    assert!(!r.remote_deletion_required());
    r.remote_context_oid = Some(oid('1'));
    assert!(r.remote_deletion_required());
    r.remote_context_oid = None;
    let mut child = next_attempt(&tx, 1, r.operation_id, ChildRole::Context).unwrap();
    child.integration_kind = Some(IntegrationKind::Merged);
    assert!(child.remote_deletion_required());
    let mut local = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Record,
    )
    .unwrap();
    local.integration_kind = Some(IntegrationKind::Merged);
    assert!(!local.remote_deletion_required());
}

/// A registry from before the widened mask CHECK, and before the path
/// columns, is rebuilt twice, keeping its rows, and then takes the new bit.
#[test]
fn closure_refusal_blockers_v2_rebuilds_an_old_bound_table_once() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let refusal = |blocker| Refusal::Blocked(vec![blocker]);
    let refuse = |tx: &Transaction<'_>, refusal: &Refusal| {
        refuse_record(
            tx,
            1,
            OperationId::new(),
            &item(ITEM),
            digest(7),
            None,
            None,
            refusal,
        )
        .unwrap()
    };
    let old = refuse(&tx, &refusal(ClosureBlocker::ContextDirty));
    // The table as v1 left it: the v2 layout with the mask one bit narrower.
    tx.execute_batch(&format!(
        "DROP INDEX ticket_closures_unfinished; ALTER TABLE ticket_closures RENAME TO before; {}; {INDEX}; INSERT INTO ticket_closures({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM before; DROP TABLE before; DELETE FROM registry_migrations WHERE name IN ('closure_refusal_blockers_v2','wave_06_closure_modes_v1','wave_06_closure_cleanup_v1');",
        SCHEMA_V2.replace("4194303", "2097151")
    ))
    .unwrap();
    assert!(
        tx.execute("UPDATE ticket_closures SET refusal_blockers=2097152", [])
            .is_err()
    );
    assert_eq!(marker(&tx), 1);
    for _ in 0..2 {
        migrate(&tx).unwrap();
        let rebuilt = read(&tx, old.operation_id);
        assert_eq!(rebuilt.refusal, old.refusal);
        assert_eq!(rebuilt.path(), RecordPath::Direct);
        // All three rebuilds ran once: all four markers, one each.
        assert_eq!(marker(&tx), 4);
        for name in [
            "closure_refusal_blockers_v2",
            "wave_06_closure_modes_v1",
            "wave_06_closure_cleanup_v1",
        ] {
            let marked: i64 = tx
                .query_row(
                    "SELECT count(*) FROM registry_migrations WHERE name=?1",
                    [name],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(marked, 1, "{name}");
        }
    }
    let new = refuse(&tx, &refusal(ClosureBlocker::HistoryUnresolved));
    assert_eq!(read(&tx, new.operation_id).refusal, new.refusal);
}

/// `digit` repeated as a quoted SQL OID literal.
fn q(digit: char) -> String {
    format!("'{}'", digit.to_string().repeat(40))
}
/// The `n`th test item: a valid ULID of its own.
fn item_n(n: u8) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    format!(
        "01ARZ3NDEKTSV4RRFFQ69G5F{}{}",
        ALPHABET[usize::from(n / 32)] as char,
        ALPHABET[usize::from(n % 32)] as char
    )
}
/// A pull-request-mode record, straight in SQL: its writers land with the
/// branch runner. `columns` are added to the row's identity columns (or
/// replace one), `{op}` in a value standing for the row's own operation ID.
fn branch_row(
    tx: &Transaction<'_>,
    item: &str,
    action: &str,
    step: &str,
    columns: &[(&str, &str)],
) -> OperationId {
    let op = OperationId::new();
    let identity = [
        ("repository_id", "1".to_owned()),
        ("operation_id", format!("'{op}'")),
        ("item_id", format!("'{item}'")),
        ("expectation", format!("x'{}'", "07".repeat(32))),
        ("remote_name", "'origin'".to_owned()),
        ("step", format!("'{step}'")),
        ("updated_at", "1".to_owned()),
        ("integration_mode", "'pull_request'".to_owned()),
        ("action", format!("'{action}'")),
    ];
    let (mut names, mut values): (Vec<String>, Vec<String>) = identity
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .unzip();
    for (name, value) in columns {
        let value = value.replace("{op}", &op.to_string());
        match names.iter().position(|n| n == name) {
            Some(at) => values[at] = value,
            None => {
                names.push((*name).to_owned());
                values.push(value);
            }
        }
    }
    tx.execute_batch(&format!(
        "INSERT INTO ticket_closures({}) VALUES({})",
        names.join(","),
        values.join(",")
    ))
    .unwrap();
    op
}
/// Whether the schema alone refuses a change to a stored row. It is rolled back.
fn schema_refuses(tx: &Transaction<'_>, op: OperationId, assignment: &str) -> bool {
    tx.execute_batch("SAVEPOINT probe").unwrap();
    let refused = tx
        .execute(
            &format!("UPDATE ticket_closures SET {assignment} WHERE operation_id=?1"),
            [op.to_string()],
        )
        .is_err();
    tx.execute_batch("ROLLBACK TO probe; RELEASE probe")
        .unwrap();
    refused
}
/// A record in memory, as decoded from a pull-request row.
fn branch_record(action: ClosureAction, step: ClosureStep) -> ClosureRecord {
    ClosureRecord {
        repository_id: 1,
        operation_id: OperationId::new(),
        item_id: item(ITEM),
        expectation: digest(7),
        remote_name: Some("origin".to_owned()),
        step,
        context_attempt: 0,
        context_sync_id: None,
        primary_attempt: 0,
        primary_sync_id: None,
        base_oid: None,
        context_oid: None,
        integration_oid: None,
        integration_kind: None,
        remote_context_oid: None,
        publication: None,
        cleanup: None,
        cancel_requested: false,
        updated_at: 1,
        refusal: None,
        closure_kind: Some(ClosureKind::Existing),
        closed_at: None,
        closure_oid: None,
        adopted_context_oid: None,
        integration_mode: Integration::PullRequest,
        action,
        withdraw_oid: None,
        retired_oid: None,
        pending_ref: None,
        answer_code: None,
        consented_context_oid: None,
        unheld_digest: None,
        primary_branch: None,
        endpoint_digest: None,
        remote_branch: None,
    }
}

/// P7. A registry from before the path columns rebuilds once, every row
/// reading as the direct close it always was, and a second run changes
/// nothing; a pull-request row then stores and reads back.
#[test]
fn branch_records_rebuild_and_keep_direct_rows() {
    let mut db = db();
    db.execute_batch(&format!(
        "DROP INDEX ticket_closures_unfinished; DROP TABLE ticket_closures; {SCHEMA_V2}; {INDEX}; DELETE FROM registry_migrations WHERE name IN ('wave_06_closure_modes_v1','wave_06_closure_cleanup_v1');"
    ))
    .unwrap();
    // Direct rows at four steps, as the older layout stored them.
    let (o1, o2, o3, o4) = (q('1'), q('2'), q('3'), q('4'));
    let ts = "'2023-11-14T22:13:20Z'";
    let steps = [
        ("confirmed", String::new()),
        (
            "closing",
            format!(
                ",closure_kind,context_oid,base_oid,closed_at,closure_oid) VALUES(1,'{{op}}','{{item}}',x'{{blob}}','origin','closing',1,'record',{o1},{o2},{ts},{o3}"
            ),
        ),
        (
            "integrating",
            format!(
                ",closure_kind,context_oid,base_oid,closed_at,closure_oid,integration_kind) VALUES(1,'{{op}}','{{item}}',x'{{blob}}','origin','integrating',1,'record',{o1},{o2},{ts},{o3},'merged'"
            ),
        ),
        (
            "complete",
            format!(
                ",closure_kind,context_oid,base_oid,closed_at,closure_oid,integration_kind,integration_oid,publication_category) VALUES(1,'{{op}}','{{item}}',x'{{blob}}','origin','complete',1,'record',{o1},{o2},{ts},{o3},'merged',{o4},'published'"
            ),
        ),
    ];
    let mut ops = Vec::new();
    for (n, (step, tail)) in steps.iter().enumerate() {
        let op = OperationId::new();
        let blob = "07".repeat(32);
        let item = item_n(n as u8);
        let sql = if tail.is_empty() {
            format!(
                "INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,closure_kind) VALUES(1,'{op}','{item}',x'{blob}','origin','{step}',1,'record')"
            )
        } else {
            let tail = tail
                .replace("{op}", &op.to_string())
                .replace("{item}", &item)
                .replace("{blob}", &blob);
            format!(
                "INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at{tail})"
            )
        };
        db.execute_batch(&sql).unwrap();
        ops.push(op);
    }
    let dump = |db: &Connection| -> Vec<Vec<rusqlite::types::Value>> {
        let mut q = db
            .prepare(&format!(
                "SELECT {V2_COLUMNS} FROM ticket_closures ORDER BY operation_id"
            ))
            .unwrap();
        let count = q.column_count();
        q.query_map([], |row| {
            (0..count)
                .map(|i| row.get::<_, rusqlite::types::Value>(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    };
    let before = dump(&db);
    assert_eq!(before.len(), 4);
    migrate_registry(&mut db).unwrap();
    // One rebuild: the same rows, each a direct close, and the marker once.
    assert_eq!(dump(&db), before);
    let modes: Vec<(String, String)> = db
        .prepare("SELECT integration_mode,action FROM ticket_closures")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(modes.len(), 4);
    assert!(
        modes
            .iter()
            .all(|(mode, action)| mode == "direct" && action == "close")
    );
    let marked: i64 = db
        .query_row(
            "SELECT count(*) FROM registry_migrations WHERE name='wave_06_closure_modes_v1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(marked, 1);
    for op in &ops {
        let r = read_record(&db, 1, *op).unwrap().unwrap();
        assert_eq!(r.integration_mode, Integration::Direct);
        assert_eq!(r.action, ClosureAction::Close);
        assert_eq!(r.path(), RecordPath::Direct);
        assert_eq!(
            (
                r.withdraw_oid,
                r.retired_oid,
                r.pending_ref,
                r.answer_code,
                r.consented_context_oid
            ),
            (None, None, None, None, None)
        );
    }
    // A second run changes nothing.
    let changes = db.total_changes();
    migrate_registry(&mut db).unwrap();
    assert_eq!(db.total_changes(), changes);
    assert_eq!(dump(&db), before);
    audit(&db).unwrap();
    // The one-unfinished-row index came with the rebuild.
    let tx = db.transaction().unwrap();
    assert!(
        tx.execute_batch(&format!(
            "INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,step,updated_at,closure_kind) VALUES(1,'{}','{}',x'{}','confirmed',1,'record')",
            OperationId::new(),
            item_n(0),
            "07".repeat(32)
        ))
        .is_err()
    );
    // A pull-request row at `publication_pending` stores and reads back.
    let op = branch_row(
        &tx,
        &item_n(9),
        "close",
        "publication_pending",
        &[
            ("closure_kind", "'existing'"),
            ("context_oid", &o1),
            ("base_oid", &o2),
            ("publication_category", "'transport'"),
            ("consented_context_oid", &o1),
        ],
    );
    let r = read(&tx, op);
    assert_eq!(r.path(), RecordPath::Branch);
    assert_eq!(
        (r.integration_mode, r.action, r.step),
        (
            Integration::PullRequest,
            ClosureAction::Close,
            ClosureStep::PublicationPending
        )
    );
    assert_eq!(r.consented_context_oid, Some(oid('1')));
    assert_eq!(r.written_commit(), None);
    assert!(!r.effect_written());
    assert!(!r.supersedable());
    // `written_commit` is the closure for a close and the withdraw for a withdraw.
    let withdraw = branch_row(
        &tx,
        &item_n(10),
        "withdraw",
        "closing",
        &[
            ("closure_kind", "'existing'"),
            ("context_oid", &o1),
            ("base_oid", &o2),
            ("withdraw_oid", &q('5')),
        ],
    );
    let r = read(&tx, withdraw);
    assert_eq!(r.action, ClosureAction::Withdraw);
    assert_eq!(r.written_commit(), Some(oid('5')));
    assert_eq!(r.merged_context(), Some(oid('5')));
    assert_eq!(r.written_checkpoint(), Some(oid('5')));
    // A direct record's commit is its closure, as ever.
    let direct = read(&tx, ops[1]);
    assert_eq!(direct.written_commit(), Some(oid('3')));
    assert_eq!(direct.merged_context(), Some(oid('3')));
    // A new branch builds its commit C at `confirmed`, before it retires the
    // old branch (tip T): through the retirement the context head is T, from
    // `closing` on it is C.
    let built = [
        ("closure_kind", "'record'"),
        ("closed_at", ts),
        ("closure_oid", &o3[..]),
        ("retired_oid", &o4[..]),
        ("pending_ref", "'refs/manyhands/closures/{op}'"),
    ];
    for (n, step) in [
        "confirmed",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
        "retired",
    ]
    .into_iter()
    .enumerate()
    {
        let op = branch_row(&tx, &item_n(20 + n as u8), "new_branch", step, &built);
        let r = read(&tx, op);
        assert_eq!(r.merged_context(), Some(oid('4')), "new_branch at {step}");
        assert_eq!(r.written_commit(), Some(oid('3')), "new_branch at {step}");
    }
    for (n, step) in ["closing", "publication_pending"].into_iter().enumerate() {
        let mut columns = built.to_vec();
        columns.extend([("context_oid", &o1[..]), ("base_oid", &o2[..])]);
        if step == "publication_pending" {
            columns.push(("publication_category", "'transport'"));
        }
        let op = branch_row(&tx, &item_n(30 + n as u8), "new_branch", step, &columns);
        assert_eq!(
            read(&tx, op).merged_context(),
            Some(oid('3')),
            "new_branch at {step}"
        );
    }
}

/// The fence's range (C1, C9): a direct record holds from `closing`; a
/// branch record holds from `closing` too (from `confirmed` for a new
/// branch with a built commit) until it is finished.
#[test]
fn written_closure_unfinished_covers_branch_records_by_step() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let (o1, o2, o3) = (q('1'), q('2'), q('3'));
    let ts = "'2023-11-14T22:13:20Z'";
    let held = |n: u8| written_closure_unfinished(&tx, 1, &item(&item_n(n))).unwrap();
    let existing = |action: &str, step: &str, n: u8, extra: &[(&str, &str)]| {
        let mut columns = vec![
            ("closure_kind", "'existing'"),
            ("context_oid", &o1[..]),
            ("base_oid", &o2[..]),
        ];
        columns.extend_from_slice(extra);
        branch_row(&tx, &item_n(n), action, step, &columns);
    };
    let mut n = 0;
    let mut row = |action: &str, step: &str, extra: &[(&str, &str)], expected: bool| {
        n += 1;
        existing(action, step, n, extra);
        assert_eq!(held(n), expected, "{action} at {step}");
    };
    row("close", "confirmed", &[], false);
    row("close", "context_synchronized", &[], false);
    row("close", "closing", &[], true);
    row(
        "close",
        "publication_pending",
        &[("publication_category", "'transport'")],
        true,
    );
    row("close", "published", &[], true);
    row(
        "close",
        "complete",
        &[("answer_code", "'published'")],
        false,
    );
    row(
        "close",
        "abandoned",
        &[
            ("refusal_category", "'external_change'"),
            ("answer_code", "'external_change'"),
        ],
        false,
    );
    row("withdraw", "closing", &[("withdraw_oid", &q('5'))], true);
    row(
        "withdraw",
        "publication_pending",
        &[
            ("withdraw_oid", &q('5')),
            ("publication_category", "'transport'"),
        ],
        true,
    );
    // A new branch holds once its commit is built, even before `closing`.
    let built: &[(&str, &str)] = &[("closed_at", ts), ("closure_oid", &o3[..])];
    for (step, columns, expected) in [
        ("confirmed", &[][..], false),
        ("confirmed", built, true),
        ("worktree_removing", built, true),
        ("retired", built, true),
    ] {
        n += 1;
        let mut all = vec![("closure_kind", "'record'")];
        all.extend_from_slice(columns);
        branch_row(&tx, &item_n(n), "new_branch", step, &all);
        assert_eq!(held(n), expected, "new_branch at {step} with {columns:?}");
    }
    // The direct range is unchanged.
    for (step, expected) in [("confirmed", false), ("context_synchronized", false)] {
        n += 1;
        let op = OperationId::new();
        tx.execute_batch(&format!(
            "INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,step,updated_at,closure_kind{}) VALUES(1,'{op}','{}',x'{}','{step}',1,'record'{})",
            if step == "confirmed" { "" } else { ",context_oid" },
            item_n(n),
            "07".repeat(32),
            if step == "confirmed" { String::new() } else { format!(",{o1}") }
        ))
        .unwrap();
        assert_eq!(held(n), expected, "direct at {step}");
    }
    for step in ["integrating", "published", "branch_deleted"] {
        n += 1;
        let op = OperationId::new();
        tx.execute_batch(&format!(
            "INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,step,updated_at,closure_kind,context_oid,base_oid,closed_at,closure_oid,integration_kind,integration_oid,publication_category) VALUES(1,'{op}','{}',x'{}','{step}',1,'record',{o1},{o2},{ts},{o3},'merged',{o1},'published')",
            item_n(n),
            "07".repeat(32),
        ))
        .unwrap();
        assert!(held(n), "direct at {step}");
    }
}

/// A branch record begins with its path and the head its consent bound,
/// replays by its operation, is `busy` to another request while it is
/// unfinished, supersedes a direct record's supersedable cleanup stop, and a
/// refusal is stored with the path it was made in.
#[test]
fn a_branch_record_begins_replays_and_is_refused_on_its_own_path() {
    use crate::canonical::Integration;
    let path = (Integration::PullRequest, ClosureAction::Close);
    let mut db = db();
    let tx = db.transaction().unwrap();
    let branch = |op: OperationId, item_text: &str, expectation: u8, kind| {
        begin_branch_record(
            &tx,
            1,
            op,
            &item(item_text),
            digest(expectation),
            Some("origin"),
            kind,
            path,
            Some(oid('5')),
        )
    };
    let first = OperationId::new();
    let r = branch(first, ITEM, 7, ClosureKind::Record).unwrap();
    assert_eq!(r.step, ClosureStep::Confirmed);
    assert_eq!(r.path(), RecordPath::Branch);
    assert_eq!((r.integration_mode, r.action), path);
    assert_eq!(r.consented_context_oid, Some(oid('5')));
    assert_eq!(r.closure_kind, Some(ClosureKind::Record));
    assert_eq!(r.answer_code, None);
    // The same operation replays what is stored; another is busy.
    assert!(branch(first, ITEM, 7, ClosureKind::Record).unwrap() == r);
    assert_eq!(
        kind_of(branch(OperationId::new(), ITEM, 7, ClosureKind::Record)),
        RepositoryErrorKind::RepositoryBusy
    );
    // A direct record at a supersedable cleanup stop is superseded.
    let mut old = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Record,
    )
    .unwrap();
    close(&tx, &mut old);
    integrate(&tx, &mut old);
    old.step = ClosureStep::PublicationPending;
    old.publication = Some(PublicationCategory::NoPublicationRemote);
    old.cleanup = Some(CleanupCategory::ContextChanged);
    save_record(&tx, 1, &old).unwrap();
    let new = branch(OperationId::new(), OTHER, 9, ClosureKind::Existing).unwrap();
    assert_eq!(new.path(), RecordPath::Branch);
    assert_eq!(read(&tx, old.operation_id).step, ClosureStep::Superseded);
    // A refusal keeps the path of the request it refused, and no answer; a
    // direct refusal keeps the direct path.
    let third = item_n(3);
    let refused = refuse_record_on(
        &tx,
        1,
        OperationId::new(),
        &item(&third),
        digest(7),
        Some("origin"),
        Some(ClosureKind::Record),
        &Refusal::ExternalChange,
        path,
    )
    .unwrap();
    assert_eq!(refused.step, ClosureStep::Abandoned);
    assert_eq!((refused.integration_mode, refused.action), path);
    assert_eq!(refused.answer_code, None);
    assert_eq!(refused.refusal, Some(Refusal::ExternalChange));
    let fourth = item_n(4);
    let direct = refuse_record(
        &tx,
        1,
        OperationId::new(),
        &item(&fourth),
        digest(7),
        None,
        None,
        &Refusal::ExternalChange,
    )
    .unwrap();
    assert_eq!(direct.path(), RecordPath::Direct);
}

/// A branch record's `closing` writes what its action writes: a withdraw its
/// commit, a close its closure or nothing. Another action's write, an
/// effect the record's kind does not have and a zero commit are refused, and
/// leave the record where it was.
#[test]
fn begin_branch_closing_writes_what_the_action_writes() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let mut n = 0;
    let mut row = |action: &str, kind: &str| {
        n += 1;
        branch_row(
            &tx,
            &item_n(n),
            action,
            "context_synchronized",
            &[("closure_kind", kind), ("context_oid", &q('1'))],
        )
    };
    // A withdraw writes its commit, and no closure.
    let withdraw = row("withdraw", "'existing'");
    begin_branch_closing(
        &tx,
        1,
        withdraw,
        oid('2'),
        BranchWrite::Withdraw { commit: oid('5') },
    )
    .unwrap();
    let r = read(&tx, withdraw);
    assert_eq!(r.step, ClosureStep::Closing);
    assert_eq!(r.withdraw_oid, Some(oid('5')));
    assert_eq!(r.base_oid, Some(oid('2')));
    assert_eq!((r.closure_oid, r.closed_at), (None, None));
    assert_eq!(r.written_commit(), Some(oid('5')));
    // A close's own writes are unchanged.
    let existing = row("close", "'existing'");
    begin_branch_closing(&tx, 1, existing, oid('2'), BranchWrite::Existing).unwrap();
    assert_eq!(read(&tx, existing).step, ClosureStep::Closing);
    let recorded = row("close", "'record'");
    begin_branch_closing(
        &tx,
        1,
        recorded,
        oid('2'),
        BranchWrite::Closure {
            closed_at: at(),
            commit: oid('3'),
        },
    )
    .unwrap();
    assert_eq!(read(&tx, recorded).closure_oid, Some(oid('3')));
    // Another action's write, and a zero commit, are refused.
    let refused = [
        (
            "withdraw, a close's nothing",
            row("withdraw", "'existing'"),
            BranchWrite::Existing,
        ),
        (
            "withdraw, a closure",
            row("withdraw", "'existing'"),
            BranchWrite::Closure {
                closed_at: at(),
                commit: oid('3'),
            },
        ),
        (
            "close, a withdraw",
            row("close", "'existing'"),
            BranchWrite::Withdraw { commit: oid('5') },
        ),
        (
            "record close, a withdraw",
            row("close", "'record'"),
            BranchWrite::Withdraw { commit: oid('5') },
        ),
        (
            "withdraw, a zero commit",
            row("withdraw", "'existing'"),
            BranchWrite::Withdraw {
                commit: Oid::zero(),
            },
        ),
    ];
    for (case, op, write) in refused {
        assert_eq!(
            kind_of(begin_branch_closing(&tx, 1, op, oid('2'), write)),
            RepositoryErrorKind::RecoveryRequired,
            "{case}"
        );
        let r = read(&tx, op);
        assert_eq!(r.step, ClosureStep::ContextSynchronized, "{case}");
        assert_eq!(r.withdraw_oid, None, "{case}");
    }
}

/// A new branch with nothing to retire: its commit is recorded at
/// `confirmed` with its pending ref and the tracking tip the consent bound,
/// then it begins `closing` with its context at the commit, and lets the
/// pending ref go. Each write is refused out of its place, and the
/// publication child is minted only once the pending ref is gone.
#[test]
fn a_new_branch_records_its_commit_begins_closing_and_lets_its_ref_go() {
    use crate::canonical::Integration;
    let path = (Integration::PullRequest, ClosureAction::NewBranch);
    let mut db = db();
    let tx = db.transaction().unwrap();
    let begin_new = |item_text: &str, kind| {
        begin_branch_record(
            &tx,
            1,
            OperationId::new(),
            &item(item_text),
            digest(7),
            Some("origin"),
            kind,
            path,
            None,
        )
        .unwrap()
        .operation_id
    };
    let op = begin_new(ITEM, ClosureKind::Record);
    let r = read(&tx, op);
    assert_eq!(
        (r.step, r.closure_oid, r.pending_ref),
        (ClosureStep::Confirmed, None, None)
    );
    // The steps out of their order.
    assert_eq!(
        kind_of(begin_new_branch_closing(&tx, 1, op)),
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(
        kind_of(clear_pending_ref(&tx, 1, op)),
        RepositoryErrorKind::RecoveryRequired
    );
    // A direct record and a zero commit are not a new branch's.
    let direct = begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(OTHER),
        digest(7),
        None,
        ClosureKind::Record,
    )
    .unwrap()
    .operation_id;
    for (case, operation, commit) in [("direct", direct, oid('3')), ("zero", op, Oid::zero())] {
        assert_eq!(
            kind_of(begin_new_branch(
                &tx,
                1,
                operation,
                oid('2'),
                at(),
                commit,
                None
            )),
            RepositoryErrorKind::RecoveryRequired,
            "{case}"
        );
        assert_eq!(read(&tx, operation).closure_oid, None, "{case}");
    }
    begin_new_branch(&tx, 1, op, oid('2'), at(), oid('3'), Some(oid('1'))).unwrap();
    let r = read(&tx, op);
    assert_eq!(r.step, ClosureStep::Confirmed);
    assert_eq!(r.closure_oid, Some(oid('3')));
    assert_eq!(
        (r.base_oid, r.remote_context_oid),
        (Some(oid('2')), Some(oid('1')))
    );
    assert_eq!(r.closed_at, Some(at()));
    assert_eq!(r.pending_ref, Some(pending_ref_name(op)));
    assert_eq!((r.retired_oid, r.context_oid), (None, None));
    // A commit is recorded once.
    assert_eq!(
        kind_of(begin_new_branch(&tx, 1, op, oid('2'), at(), oid('4'), None)),
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(read(&tx, op).closure_oid, Some(oid('3')));
    // From the first write the record holds the ticket.
    assert!(written_closure_unfinished(&tx, 1, &item(ITEM)).unwrap());
    begin_new_branch_closing(&tx, 1, op).unwrap();
    let r = read(&tx, op);
    assert_eq!(r.step, ClosureStep::Closing);
    assert_eq!(r.context_oid, Some(oid('3')));
    assert_eq!(r.pending_ref, Some(pending_ref_name(op)));
    assert_eq!(
        kind_of(begin_new_branch_closing(&tx, 1, op)),
        RepositoryErrorKind::RecoveryRequired
    );
    // No child before the context is made.
    assert_eq!(
        kind_of(begin_publication(&tx, 1, op)),
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(read(&tx, op).step, ClosureStep::Closing);
    clear_pending_ref(&tx, 1, op).unwrap();
    assert_eq!(read(&tx, op).pending_ref, None);
    clear_pending_ref(&tx, 1, op).unwrap();
    let r = begin_publication(&tx, 1, op).unwrap();
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(r.primary_attempt, 1);
    // A kept closure is a new branch's too: its `closed_at` is the kept
    // closure's, and no step differs.
    let kept = begin_new(&item_n(5), ClosureKind::Existing);
    begin_new_branch(&tx, 1, kept, oid('2'), at(), oid('3'), None).unwrap();
    let r = read(&tx, kept);
    assert_eq!(
        (r.closure_kind, r.closed_at, r.closure_oid),
        (Some(ClosureKind::Existing), Some(at()), Some(oid('3')))
    );
    begin_new_branch_closing(&tx, 1, kept).unwrap();
    assert_eq!(read(&tx, kept).step, ClosureStep::Closing);
}

/// A new branch that has an old context to retire is begun with what it
/// retires, and does not skip the retirement: it begins `closing` only from
/// `retired`. It may be abandoned from either side of the retirement, never
/// from its removals, and the abandon lets go of its pending ref.
#[test]
fn a_new_branch_retires_before_it_closes_and_is_abandoned_without_its_ref() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let begin_retiring = |n: u8| {
        begin_new_branch_record(
            &tx,
            1,
            OperationId::new(),
            &item(&item_n(n)),
            digest(7),
            Some("origin"),
            ClosureKind::Record,
            Some(oid('1')),
            Some(oid('1')),
        )
        .unwrap()
        .operation_id
    };
    // (the step it is at, whether an abandon from there is permitted)
    let steps = [
        (ClosureStep::Confirmed, true),
        (ClosureStep::WorktreeRemoving, false),
        (ClosureStep::WorktreeRemoved, false),
        (ClosureStep::BranchDeleted, true),
        (ClosureStep::Retired, true),
    ];
    for (n, (step, abandons)) in steps.into_iter().enumerate() {
        let case = format!("{step:?}");
        let op = begin_retiring(10 + n as u8);
        let r = read(&tx, op);
        assert_eq!(
            (r.retired_oid, r.consented_context_oid),
            (Some(oid('1')), Some(oid('1'))),
            "{case}"
        );
        begin_new_branch(&tx, 1, op, oid('2'), at(), oid('3'), Some(oid('1'))).unwrap();
        // The retirement is not skipped.
        assert_eq!(
            kind_of(begin_new_branch_closing(&tx, 1, op)),
            RepositoryErrorKind::RecoveryRequired,
            "{case}"
        );
        let mut r = read(&tx, op);
        for next in [
            ClosureStep::WorktreeRemoving,
            ClosureStep::WorktreeRemoved,
            ClosureStep::BranchDeleted,
            ClosureStep::Retired,
        ] {
            if r.step == step {
                break;
            }
            r.step = next;
            save_record(&tx, 1, &r).unwrap();
        }
        assert_eq!(read(&tx, op).step, step, "{case}");
        let refusal = Refusal::ExternalChange;
        let abandoned =
            abandon_branch_record(&tx, 1, op, &refusal, BranchAnswer::ExternalChange, None, 0);
        if abandons {
            abandoned.unwrap();
            let r = read(&tx, op);
            assert_eq!(r.step, ClosureStep::Abandoned, "{case}");
            assert_eq!((r.pending_ref, r.cleanup), (None, None), "{case}");
            assert_eq!(r.retired_oid, Some(oid('1')), "{case}");
            assert!(!written_closure_unfinished(&tx, 1, &item(&item_n(10 + n as u8))).unwrap());
        } else {
            assert_eq!(
                kind_of(abandoned),
                RepositoryErrorKind::RecoveryRequired,
                "{case}"
            );
            assert_eq!(read(&tx, op).step, step, "{case}");
        }
    }
    // Only `retired` begins `closing`, and it is not an abandon's step again.
    let op = begin_retiring(20);
    begin_new_branch(&tx, 1, op, oid('2'), at(), oid('3'), None).unwrap();
    let mut r = read(&tx, op);
    for next in [
        ClosureStep::WorktreeRemoving,
        ClosureStep::WorktreeRemoved,
        ClosureStep::BranchDeleted,
        ClosureStep::Retired,
    ] {
        r.step = next;
        save_record(&tx, 1, &r).unwrap();
    }
    begin_new_branch_closing(&tx, 1, op).unwrap();
    let r = read(&tx, op);
    assert_eq!(
        (r.step, r.context_oid),
        (ClosureStep::Closing, Some(oid('3')))
    );
    assert_eq!(
        kind_of(abandon_branch_record(
            &tx,
            1,
            op,
            &Refusal::ExternalChange,
            BranchAnswer::ExternalChange,
            None,
            0,
        )),
        RepositoryErrorKind::RecoveryRequired
    );
}

/// A row of any action, straight in SQL, returning the schema's answer: the
/// identity columns of `branch_row`, with `columns` replacing or adding to
/// them. `NULL` as a value clears an identity column.
fn try_row(
    tx: &Transaction<'_>,
    item: &str,
    action: &str,
    step: &str,
    columns: &[(&str, &str)],
) -> rusqlite::Result<OperationId> {
    let op = OperationId::new();
    let mut cells: Vec<(String, String)> = [
        ("repository_id", "1".to_owned()),
        ("operation_id", format!("'{op}'")),
        ("item_id", format!("'{item}'")),
        ("expectation", format!("x'{}'", "07".repeat(32))),
        ("remote_name", "'origin'".to_owned()),
        ("step", format!("'{step}'")),
        ("updated_at", "1".to_owned()),
        ("integration_mode", "'pull_request'".to_owned()),
        ("action", format!("'{action}'")),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value))
    .collect();
    for (name, value) in columns {
        let value = value.replace("{op}", &op.to_string());
        match cells.iter().position(|(n, _)| n == name) {
            Some(at) => cells[at].1 = value,
            None => cells.push(((*name).to_owned(), value)),
        }
    }
    let (names, values): (Vec<_>, Vec<_>) = cells.into_iter().unzip();
    tx.execute_batch(&format!(
        "INSERT INTO ticket_closures({}) VALUES({})",
        names.join(","),
        values.join(",")
    ))
    .map(|()| op)
}

/// Whether saving `r` is accepted. The save is rolled back.
fn saves(tx: &Transaction<'_>, r: &ClosureRecord) -> bool {
    tx.execute_batch("SAVEPOINT probe").unwrap();
    let saved = save_record(tx, 1, r).is_ok();
    tx.execute_batch("ROLLBACK TO probe; RELEASE probe")
        .unwrap();
    saved
}

/// P7, R-28, R-29, R-33. A registry whose table is today's (`SCHEMA_V3`),
/// holding a row of every step and of every action, is rebuilt once: every row
/// reads its old action and step, byte for byte, with the four new columns
/// null; a second run changes nothing, and a half-done migration fails
/// closed. A cleanup row then keeps to its CHECKs, its step graph and its
/// immutables, in either mode, and holds no context.
#[test]
fn cleanup_records_rebuild_once() {
    use crate::repository::{
        closure::lifecycle::{TicketClosureLifecycle, TicketWithdrawLifecycle},
        promotion::lifecycle::Lifecycle,
    };
    let (o1, o2, o3, o4, o5) = (q('1'), q('2'), q('3'), q('4'), q('5'));
    let ts = "'2023-11-14T22:13:20Z'";
    let blob = format!("x'{}'", "0a".repeat(32));
    let mut db = db();
    db.execute_batch(&format!(
        "DROP INDEX ticket_closures_unfinished; DROP TABLE ticket_closures; {SCHEMA_V3}; {INDEX}; DELETE FROM registry_migrations WHERE name='{CLEANUP_MARKER}';"
    ))
    .unwrap();
    // Rows of every step of a direct close, and one of every branch action.
    let record = [("closure_kind", "'record'")];
    let synchronized = [("closure_kind", "'record'"), ("context_oid", &o1[..])];
    let closing = [
        ("closure_kind", "'record'"),
        ("context_oid", &o1[..]),
        ("base_oid", &o2[..]),
        ("closed_at", ts),
        ("closure_oid", &o3[..]),
    ];
    let integrating = [&closing[..], &[("integration_kind", "'merged'")]].concat();
    let integrated = [&integrating[..], &[("integration_oid", &o4[..])]].concat();
    let published = [&integrated[..], &[("publication_category", "'published'")]].concat();
    let pending = [&integrated[..], &[("publication_category", "'transport'")]].concat();
    let abandoned = [("refusal_category", "'external_change'")];
    let direct: [(&str, &[(&str, &str)]); 14] = [
        ("confirmed", &record),
        ("context_synchronized", &synchronized),
        ("closing", &closing),
        ("integrating", &integrating),
        ("integrated", &integrated),
        ("published", &published),
        ("publication_pending", &pending),
        ("remote_deleted", &published),
        ("worktree_removing", &published),
        ("worktree_removed", &published),
        ("branch_deleted", &published),
        ("complete", &published),
        ("abandoned", &abandoned),
        ("superseded", &record),
    ];
    let existing = [
        ("closure_kind", "'existing'"),
        ("context_oid", &o1[..]),
        ("base_oid", &o2[..]),
    ];
    let withdraw = [&existing[..], &[("withdraw_oid", &o5[..])]].concat();
    // A finished branch record keeps its stored answer and the consented
    // head, as a refused one keeps its answer beside its refusal.
    let answered = [
        &existing[..],
        &[
            ("answer_code", "'existing'"),
            ("consented_context_oid", &o3[..]),
        ],
    ]
    .concat();
    let refused_with_answer = [
        ("refusal_category", "'external_change'"),
        ("answer_code", "'external_change'"),
        ("consented_context_oid", &o4[..]),
    ];
    let built = [
        ("closure_kind", "'record'"),
        ("closed_at", ts),
        ("closure_oid", &o3[..]),
        ("retired_oid", &o4[..]),
        ("pending_ref", "'refs/manyhands/closures/{op}'"),
    ];
    let tx = db.transaction().unwrap();
    let mut old: Vec<(OperationId, ClosureAction, Integration, ClosureStep)> = Vec::new();
    for (n, (step, columns)) in direct.iter().enumerate() {
        let columns = [&[("integration_mode", "'direct'")][..], columns].concat();
        let op = try_row(&tx, &item_n(n as u8), "close", step, &columns).unwrap();
        old.push((
            op,
            ClosureAction::Close,
            Integration::Direct,
            ClosureStep::parse(step).unwrap(),
        ));
    }
    for (n, (action, step, columns)) in [
        ("close", "publication_pending", &existing[..]),
        ("withdraw", "closing", &withdraw[..]),
        ("new_branch", "confirmed", &built[..]),
        ("new_branch", "retired", &built[..]),
        ("close", "complete", &answered[..]),
        ("close", "abandoned", &refused_with_answer[..]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut columns = columns.to_vec();
        if step == "publication_pending" {
            columns.push(("publication_category", "'transport'"));
        }
        let op = try_row(&tx, &item_n(20 + n as u8), action, step, &columns).unwrap();
        old.push((
            op,
            ClosureAction::parse(action).unwrap(),
            Integration::PullRequest,
            ClosureStep::parse(step).unwrap(),
        ));
    }
    assert_eq!(old.len(), 20);
    // Every step is held by an old row, `retired` included.
    for step in ALL_STEPS
        .into_iter()
        .filter(|s| *s != ClosureStep::RemoteDeleting)
    {
        assert!(old.iter().any(|(_, _, _, held)| *held == step), "{step:?}");
    }
    tx.commit().unwrap();
    let dump = |db: &Connection, columns: &str| -> Vec<Vec<rusqlite::types::Value>> {
        let mut q = db
            .prepare(&format!(
                "SELECT {columns} FROM ticket_closures ORDER BY rowid"
            ))
            .unwrap();
        let count = q.column_count();
        q.query_map([], |row| {
            (0..count)
                .map(|i| row.get::<_, rusqlite::types::Value>(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    };
    let before = dump(&db, V3_COLUMNS);
    assert_eq!(before.len(), 20);
    // The two columns that carry values survive the rebuild (byte for byte,
    // by the dump below): they are set on the rows that hold them.
    let held = |db: &Connection, column: &str| -> i64 {
        db.query_row(
            &format!("SELECT count(*) FROM ticket_closures WHERE {column} IS NOT NULL"),
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(held(&db, "answer_code"), 2);
    assert_eq!(held(&db, "consented_context_oid"), 2);
    assert_eq!(dump(&db, "*")[0].len(), 32);
    assert_eq!(V3_COLUMNS.split(',').count(), 32);
    // A V3 table has no `cleanup` action, and no `remote_deleting` step.
    assert!(
        db.execute("UPDATE ticket_closures SET action='cleanup'", [])
            .is_err()
    );
    migrate_registry(&mut db).unwrap();
    // One rebuild: every old row byte for byte, reading its old action, step
    // and mode; the four new columns null; the marker once.
    assert_eq!(dump(&db, V3_COLUMNS), before);
    for (op, action, mode, step) in &old {
        let r = read(&db.unchecked_transaction().unwrap(), *op);
        assert_eq!(
            (r.action, r.integration_mode, r.step),
            (*action, *mode, *step)
        );
        assert_ne!(r.path(), RecordPath::Cleanup);
        // Each verb still continues its own record, and only that.
        assert_eq!(
            TicketClosureLifecycle::continues(&r),
            r.action != ClosureAction::Withdraw
        );
        assert_eq!(
            TicketWithdrawLifecycle::continues(&r),
            r.action == ClosureAction::Withdraw
        );
        assert_eq!(
            (
                r.unheld_digest,
                &r.primary_branch,
                r.endpoint_digest,
                r.remote_branch
            ),
            (None, &None, None, None)
        );
    }
    assert_eq!(marker_of(&db, CLEANUP_MARKER), 1);
    audit(&db).unwrap();
    let changes = db.total_changes();
    migrate_registry(&mut db).unwrap();
    assert_eq!(db.total_changes(), changes, "a second run changes nothing");
    assert_eq!(dump(&db, V3_COLUMNS), before);
    // A half-done migration fails closed: the marker with a table that is not
    // the new one, and a table of the old layout left under the rename's name.
    {
        let mut half = self::db();
        half.execute_batch(&format!(
            "DROP INDEX ticket_closures_unfinished; DROP TABLE ticket_closures; {SCHEMA_V3}; {INDEX};"
        ))
        .unwrap();
        assert_eq!(
            migrate_registry(&mut half).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "marked, but still the old table"
        );
        half.execute_batch(&format!(
            "DELETE FROM registry_migrations WHERE name='{CLEANUP_MARKER}'; CREATE TABLE ticket_closures_before_cleanup(x);"
        ))
        .unwrap();
        assert_eq!(
            migrate_registry(&mut half).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "a leftover table of the rebuild"
        );
        // A fault at the marker undoes the whole rebuild.
        half.execute_batch(&format!(
            "DROP TABLE ticket_closures_before_cleanup; CREATE TRIGGER marker_fault BEFORE INSERT ON registry_migrations WHEN NEW.name='{CLEANUP_MARKER}' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;"
        ))
        .unwrap();
        assert!(migrate_registry(&mut half).is_err());
        assert_eq!(marker_of(&half, CLEANUP_MARKER), 0);
        let stored: String = half
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='ticket_closures'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, SCHEMA_V3, "still the old table");
        half.execute_batch("DROP TRIGGER marker_fault").unwrap();
        migrate_registry(&mut half).unwrap();
        assert_eq!(marker_of(&half, CLEANUP_MARKER), 1);
    }

    // A cleanup row's CHECKs, in either mode.
    let tx = db.transaction().unwrap();
    let mut n = 100u8;
    let mut next = || {
        n += 1;
        item_n(n)
    };
    let base = |extra: &[(&'static str, String)]| -> Vec<(&'static str, String)> {
        let mut cells: Vec<(&'static str, String)> = vec![
            ("context_oid", q('1')),
            ("unheld_digest", format!("x'{}'", "0a".repeat(32))),
            ("primary_branch", "'main'".to_owned()),
            ("endpoint_digest", format!("x'{}'", "0b".repeat(32))),
        ];
        for (name, value) in extra {
            match cells.iter().position(|(n, _)| n == name) {
                Some(at) => cells[at].1 = value.clone(),
                None => cells.push((name, value.clone())),
            }
        }
        cells
    };
    let cleanup = |tx: &Transaction<'_>,
                   item: &str,
                   mode: &str,
                   step: &str,
                   extra: &[(&'static str, String)]|
     -> rusqlite::Result<OperationId> {
        let cells = base(extra);
        let mode_text = format!("'{mode}'");
        let mut columns: Vec<(&str, &str)> = vec![("integration_mode", mode_text.as_str())];
        columns.extend(cells.iter().map(|(n, v)| (*n, v.as_str())));
        try_row(tx, item, "cleanup", step, &columns)
    };
    let s = |text: &str| text.to_owned();
    let none = || s("NULL");
    let remoteless = [("remote_name", none()), ("endpoint_digest", none())];
    let with_r = [("remote_context_oid", o5.clone())];
    let deleted = [
        ("remote_context_oid", o5.clone()),
        ("remote_branch", s("'deleted'")),
    ];
    for mode in ["direct", "pull_request"] {
        let accepted: Vec<Case<'_>> = vec![
            ("with a remote and R", "confirmed", with_r.to_vec()),
            ("with a remote and no R", "confirmed", vec![]),
            ("without a remote", "confirmed", remoteless.to_vec()),
            ("deleting", "remote_deleting", with_r.to_vec()),
            ("deleted", "remote_deleted", deleted.to_vec()),
            (
                "absent",
                "remote_deleted",
                vec![
                    ("remote_context_oid", o5.clone()),
                    ("remote_branch", s("'absent'")),
                ],
            ),
            (
                "nothing to delete",
                "remote_deleted",
                vec![("remote_branch", s("'none'"))],
            ),
            ("worktree removing", "worktree_removing", deleted.to_vec()),
            ("worktree removed", "worktree_removed", deleted.to_vec()),
            ("branch deleted", "branch_deleted", deleted.to_vec()),
            ("complete", "complete", deleted.to_vec()),
            (
                "no remote at the worktree",
                "worktree_removing",
                [&remoteless[..], &[("remote_branch", s("'none'"))]].concat(),
            ),
            (
                "a stop at the worktree",
                "worktree_removing",
                [
                    &deleted[..],
                    &[("cleanup_category", s("'context_changed'"))],
                ]
                .concat(),
            ),
            (
                "a stop while deleting",
                "remote_deleting",
                [
                    &with_r[..],
                    &[("cleanup_category", s("'remote_deletion_transport'"))],
                ]
                .concat(),
            ),
            (
                "abandoned",
                "abandoned",
                vec![("refusal_category", s("'external_change'"))],
            ),
        ];
        for (label, step, extra) in &accepted {
            let op = cleanup(&tx, &next(), mode, step, extra)
                .unwrap_or_else(|e| panic!("{mode} {label}: {e}"));
            let r = read(&tx, op);
            assert_eq!(r.path(), RecordPath::Cleanup, "{mode} {label}");
            assert_eq!(r.action, ClosureAction::Cleanup, "{mode} {label}");
            assert_eq!(r.integration_mode.as_str(), mode, "{mode} {label}");
            assert_eq!(r.step.name(), *step, "{mode} {label}");
            assert_eq!(r.closure_kind, None);
            assert_eq!(r.merged_context(), Some(oid('1')), "{mode} {label}");
            assert_eq!(r.unheld_digest, Some([0x0a; 32]));
            assert_eq!(r.primary_branch.as_deref(), Some("main"));
            assert_eq!(r.endpoint_digest.is_some(), r.remote_name.is_some());
            assert!(!r.supersedable(), "{mode} {label}: never superseded");
            assert!(
                !TicketClosureLifecycle::continues(&r) && !TicketWithdrawLifecycle::continues(&r),
                "{mode} {label}: neither the close nor the withdraw continues it"
            );
            assert_eq!(
                deletion_reservation_required(&r),
                r.remote_name.is_some(),
                "{mode} {label}"
            );
            assert_eq!(
                written_closure_holder(&tx, 1, &r.item_id).unwrap(),
                None,
                "{mode} {label}: a cleanup holds no branch"
            );
        }
        let junk = s("'31'");
        let sync = s("'01ARZ3NDEKTSV4RRFFQ69G5FB0'");
        let refused: Vec<Case<'_>> = vec![
            (
                "a closure effect",
                "confirmed",
                vec![("closure_kind", s("'record'"))],
            ),
            (
                "a closed_at",
                "confirmed",
                vec![("closed_at", s(ts)), ("closure_oid", o3.clone())],
            ),
            ("a base", "confirmed", vec![("base_oid", o2.clone())]),
            (
                "an integration",
                "confirmed",
                vec![
                    ("integration_oid", o4.clone()),
                    ("integration_kind", s("'merged'")),
                ],
            ),
            (
                "a publication",
                "confirmed",
                vec![("publication_category", s("'published'"))],
            ),
            (
                "a context attempt",
                "confirmed",
                vec![
                    ("context_attempt", s("1")),
                    ("context_sync_id", sync.clone()),
                ],
            ),
            (
                "a primary attempt",
                "confirmed",
                vec![
                    ("primary_attempt", s("1")),
                    ("primary_sync_id", sync.clone()),
                ],
            ),
            (
                "a withdraw",
                "confirmed",
                vec![("withdraw_oid", o5.clone())],
            ),
            (
                "a retirement",
                "confirmed",
                vec![("retired_oid", o4.clone())],
            ),
            (
                "an answer",
                "complete",
                [&deleted[..], &[("answer_code", s("'merged'"))]].concat(),
            ),
            (
                "a consented head",
                "confirmed",
                vec![("consented_context_oid", o1.clone())],
            ),
            ("no L", "confirmed", vec![("context_oid", none())]),
            (
                "no unheld digest",
                "confirmed",
                vec![("unheld_digest", none())],
            ),
            (
                "a short unheld digest",
                "confirmed",
                vec![("unheld_digest", junk.clone())],
            ),
            (
                "no primary branch",
                "confirmed",
                vec![("primary_branch", none())],
            ),
            (
                "an empty primary branch",
                "confirmed",
                vec![("primary_branch", s("''"))],
            ),
            (
                "a remote and no endpoint",
                "confirmed",
                vec![("endpoint_digest", none())],
            ),
            (
                "an endpoint and no remote",
                "confirmed",
                vec![("remote_name", none())],
            ),
            ("closing", "closing", vec![]),
            ("context_synchronized", "context_synchronized", vec![]),
            ("integrating", "integrating", vec![]),
            ("integrated", "integrated", vec![]),
            ("published", "published", vec![]),
            ("publication_pending", "publication_pending", vec![]),
            ("retired", "retired", vec![]),
            ("superseded", "superseded", vec![]),
            (
                "an outcome at confirmed",
                "confirmed",
                vec![("remote_branch", s("'none'"))],
            ),
            (
                "an outcome while deleting",
                "remote_deleting",
                [&with_r[..], &[("remote_branch", s("'deleted'"))]].concat(),
            ),
            ("no outcome once deleted", "remote_deleted", with_r.to_vec()),
            (
                "no outcome at the worktree",
                "worktree_removing",
                with_r.to_vec(),
            ),
            ("no outcome at complete", "complete", with_r.to_vec()),
            (
                "none with an R",
                "remote_deleted",
                vec![
                    ("remote_context_oid", o5.clone()),
                    ("remote_branch", s("'none'")),
                ],
            ),
            (
                "deleted with no R",
                "remote_deleted",
                vec![("remote_branch", s("'deleted'"))],
            ),
            (
                "an unknown outcome",
                "remote_deleted",
                vec![("remote_branch", s("'gone'"))],
            ),
            ("deleting with no R", "remote_deleting", vec![]),
            (
                "a blocked refusal",
                "abandoned",
                vec![
                    ("refusal_category", s("'blocked'")),
                    ("refusal_blockers", s("1")),
                ],
            ),
            (
                "a reconfirmation refusal",
                "abandoned",
                vec![("refusal_category", s("'reconfirmation_required'"))],
            ),
            (
                "a refusal on a live record",
                "confirmed",
                vec![("refusal_category", s("'external_change'"))],
            ),
        ];
        // A table reports every row it gets wrong.
        let accepted_wrongly: Vec<&str> = refused
            .iter()
            .filter(|(_, step, extra)| cleanup(&tx, &next(), mode, step, extra).is_ok())
            .map(|(label, _, _)| *label)
            .collect();
        assert!(
            accepted_wrongly.is_empty(),
            "{mode}: the schema accepts {accepted_wrongly:?}"
        );
    }
    // Only a cleanup row has them: every other action refuses the columns and
    // the step.
    for (action, mode, step, extra) in [
        (
            "close",
            "direct",
            "confirmed",
            ("unheld_digest", blob.as_str()),
        ),
        ("close", "direct", "confirmed", ("primary_branch", "'main'")),
        (
            "withdraw",
            "pull_request",
            "closing",
            ("remote_branch", "'none'"),
        ),
        (
            "close",
            "direct",
            "remote_deleting",
            ("remote_context_oid", o5.as_str()),
        ),
        (
            "new_branch",
            "pull_request",
            "remote_deleting",
            ("remote_context_oid", o5.as_str()),
        ),
    ] {
        let mut columns = vec![
            ("integration_mode", format!("'{mode}'")),
            ("closure_kind", "'record'".to_owned()),
            (extra.0, extra.1.to_owned()),
        ];
        if action == "withdraw" {
            columns.extend([
                ("closure_kind", "'existing'".to_owned()),
                ("context_oid", o1.clone()),
                ("base_oid", o2.clone()),
                ("withdraw_oid", o5.clone()),
            ]);
        }
        let columns: Vec<(&str, &str)> = columns.iter().map(|(n, v)| (*n, v.as_str())).collect();
        assert!(
            try_row(&tx, &next(), action, step, &columns).is_err(),
            "{action} {step} {}",
            extra.0
        );
    }
    // The step graph: with a remote, `confirmed` goes to the delete or past it
    // when there is none to make; without one, to the worktree; either may be
    // abandoned before the removal; nothing is superseded or restarted.
    let steps = [
        "confirmed",
        "context_synchronized",
        "closing",
        "integrating",
        "integrated",
        "published",
        "publication_pending",
        "remote_deleting",
        "remote_deleted",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
        "retired",
        "complete",
        "abandoned",
        "superseded",
    ];
    let mut wrong: Vec<String> = Vec::new();
    for remote in [true, false] {
        let graph: &[(&str, &str)] = if remote {
            &[
                ("confirmed", "remote_deleting"),
                ("confirmed", "remote_deleted"),
                ("remote_deleting", "remote_deleted"),
                ("remote_deleted", "worktree_removing"),
                ("worktree_removing", "worktree_removed"),
                ("worktree_removed", "branch_deleted"),
                ("confirmed", "abandoned"),
                ("remote_deleting", "abandoned"),
            ]
        } else {
            &[
                ("confirmed", "worktree_removing"),
                ("worktree_removing", "worktree_removed"),
                ("worktree_removed", "branch_deleted"),
                ("confirmed", "abandoned"),
            ]
        };
        for from in steps {
            for to in steps {
                let (a, b) = (
                    ClosureStep::parse(from).unwrap(),
                    ClosureStep::parse(to).unwrap(),
                );
                let expected = !a.terminal() && (from == to || graph.contains(&(from, to)));
                if a.cleanup_permits(b, remote) != expected {
                    wrong.push(format!(
                        "{from} -> {to}, remote {remote}: expected {expected}"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "the graph differs at {wrong:#?}");
    // The immutables: a save changes the step and the outcome, nothing else.
    let op = cleanup(&tx, &next(), "pull_request", "confirmed", &with_r).unwrap();
    let mut r = read(&tx, op);
    let moved = |r: &ClosureRecord, change: &dyn Fn(&mut ClosureRecord)| {
        let mut r = r.clone();
        change(&mut r);
        r
    };
    assert!(saves(&tx, &r), "unchanged");
    for (label, change) in [
        (
            "L",
            &(|r: &mut ClosureRecord| r.context_oid = Some(oid('9')))
                as &dyn Fn(&mut ClosureRecord),
        ),
        ("R", &|r| r.remote_context_oid = None),
        ("the unheld digest", &|r| r.unheld_digest = Some([1; 32])),
        ("the primary branch", &|r| {
            r.primary_branch = Some("dev".into())
        }),
        ("the endpoint digest", &|r| {
            r.endpoint_digest = Some([1; 32])
        }),
        ("the action", &|r| r.action = ClosureAction::Close),
        ("the mode", &|r| r.integration_mode = Integration::Direct),
    ] {
        assert!(!saves(&tx, &moved(&r, change)), "{label} is immutable");
    }
    let to = |r: &ClosureRecord, step, outcome| {
        moved(r, &|r| {
            r.step = step;
            r.remote_branch = outcome;
        })
    };
    use ClosureStep::*;
    assert!(
        !saves(&tx, &to(&r, WorktreeRemoving, None)),
        "not past the delete"
    );
    assert!(
        !saves(
            &tx,
            &to(&r, RemoteDeleting, Some(RemoteBranchOutcome::Deleted))
        ),
        "no outcome before the delete is known"
    );
    assert!(
        !saves(&tx, &to(&r, RemoteDeleted, None)),
        "an outcome with the delete"
    );
    assert!(
        !saves(
            &tx,
            &to(&r, RemoteDeleted, Some(RemoteBranchOutcome::Unneeded))
        ),
        "none says there was no R"
    );
    assert!(
        !saves(&tx, &to(&r, Complete, Some(RemoteBranchOutcome::Deleted))),
        "complete is its own write"
    );
    assert!(!saves(&tx, &to(&r, Superseded, None)), "never superseded");
    assert!(
        !saves(
            &tx,
            &moved(&r, &|r| r.refusal = Some(Refusal::ExternalChange))
        ),
        "a refusal is for the abandoned"
    );
    r = to(&r, RemoteDeleting, None);
    save_record(&tx, 1, &r).unwrap();
    assert!(!saves(&tx, &to(&r, Confirmed, None)), "no step back");
    assert!(
        !saves(
            &tx,
            &to(&r, WorktreeRemoving, Some(RemoteBranchOutcome::Deleted))
        ),
        "the delete first"
    );
    assert!(
        !saves(
            &tx,
            &moved(&to(&r, Abandoned, None), &|r| r.refusal =
                Some(Refusal::Blocked(vec![ClosureBlocker::ContextDirty])))
        ),
        "only an external change ends a cleanup"
    );
    r = to(&r, RemoteDeleted, Some(RemoteBranchOutcome::Absent));
    save_record(&tx, 1, &r).unwrap();
    assert!(
        !saves(
            &tx,
            &moved(&r, &|r| r.remote_branch =
                Some(RemoteBranchOutcome::Deleted))
        ),
        "the outcome is stored once"
    );
    assert!(!saves(&tx, &moved(&r, &|r| r.remote_branch = None)));
    for step in [WorktreeRemoving, WorktreeRemoved, BranchDeleted] {
        r = to(&r, step, r.remote_branch);
        save_record(&tx, 1, &r).unwrap();
    }
    assert_eq!(
        read(&tx, op).remote_branch,
        Some(RemoteBranchOutcome::Absent)
    );
    // With no remote the outcome is `none`, written with the step to the worktree.
    let op = cleanup(&tx, &next(), "direct", "confirmed", &remoteless).unwrap();
    let r = read(&tx, op);
    assert!(
        !saves(&tx, &to(&r, RemoteDeleting, None)),
        "no delete to make"
    );
    assert!(
        !saves(&tx, &to(&r, WorktreeRemoving, None)),
        "the outcome goes with the step"
    );
    assert!(
        !saves(
            &tx,
            &to(&r, WorktreeRemoving, Some(RemoteBranchOutcome::Deleted))
        ),
        "none says there was no R"
    );
    save_record(
        &tx,
        1,
        &to(&r, WorktreeRemoving, Some(RemoteBranchOutcome::Unneeded)),
    )
    .unwrap();
    // Abandoned from `confirmed`, with the external change.
    let op = cleanup(&tx, &next(), "direct", "confirmed", &[]).unwrap();
    let r = read(&tx, op);
    let ended = moved(&to(&r, Abandoned, None), &|r| {
        r.refusal = Some(Refusal::ExternalChange)
    });
    save_record(&tx, 1, &ended).unwrap();
    assert!(read(&tx, op).step.terminal());
    // A cleanup record holds no branch: the holder reads a branch record at
    // its step and ignores a cleanup one at every step, alone or beside one.
    let other = item_n(250);
    branch_row(
        &tx,
        &other,
        "close",
        "closing",
        &[
            ("closure_kind", "'existing'"),
            ("context_oid", &o1),
            ("base_oid", &o2),
        ],
    );
    assert!(
        written_closure_holder(&tx, 1, &item(&other))
            .unwrap()
            .is_some()
    );
    for step in [
        "confirmed",
        "remote_deleting",
        "remote_deleted",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
    ] {
        let item_text = next();
        let extra = if matches!(step, "confirmed" | "remote_deleting") {
            &with_r[..]
        } else {
            &deleted[..]
        };
        cleanup(&tx, &item_text, "pull_request", step, extra).unwrap();
        let item = item(&item_text);
        assert!(
            !written_closure_unfinished(&tx, 1, &item).unwrap(),
            "{step}"
        );
        assert_eq!(
            written_closure_holder(&tx, 1, &item).unwrap(),
            None,
            "{step}"
        );
    }
}

/// A cleanup test's row: its label, its step and the columns it sets.
type Case<'a> = (&'a str, &'a str, Vec<(&'static str, String)>);
/// How many times the registry has recorded the migration `name`.
fn marker_of(db: &Connection, name: &str) -> i64 {
    db.query_row(
        "SELECT count(*) FROM registry_migrations WHERE name=?1",
        [name],
        |r| r.get(0),
    )
    .unwrap()
}
