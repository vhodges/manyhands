use super::*;
use crate::repository::{
    AuthoringKind, REGISTRY_FILE, RemoteOperationAction, RemoteOperationTarget, RemoteRefPlan,
    RepositoryErrorKind, RepositoryOperation, RepositoryService,
    discovery::migrate_registry,
    promotion::{
        PromotionExpectation,
        lifecycle::{LifecycleRecord, ParentStep},
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

#[test]
fn migration_is_all_or_nothing_and_idempotent() {
    let mut db = db();
    db.execute_batch("DROP TABLE ticket_closures; DELETE FROM registry_migrations WHERE name='cycle_10_ticket_closures';").unwrap();
    // A failure anywhere in the migration leaves neither table, index nor marker.
    db.execute_batch("CREATE TRIGGER marker_fault BEFORE INSERT ON registry_migrations WHEN NEW.name='cycle_10_ticket_closures' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;").unwrap();
    assert!(migrate_registry(&mut db).is_err());
    let leftovers: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('ticket_closures','ticket_closures_unfinished')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leftovers, 0);
    db.execute_batch("DROP TRIGGER marker_fault").unwrap();
    migrate_registry(&mut db).unwrap();
    let marker = |db: &Connection| -> i64 {
        db.query_row(
            "SELECT count(*) FROM registry_migrations WHERE name='cycle_10_ticket_closures'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(marker(&db), 1);
    // A second run changes nothing.
    let changes = db.total_changes();
    let tx = db.transaction().unwrap();
    migrate(&tx).unwrap();
    tx.commit().unwrap();
    migrate_registry(&mut db).unwrap();
    assert_eq!(db.total_changes(), changes);
    assert_eq!(marker(&db), 1);
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
}

#[test]
fn closure_step_round_trips_through_parent_view() {
    for step in [
        ParentStep::Confirmed,
        ParentStep::ContextSynchronized,
        ParentStep::Integrating,
        ParentStep::Integrated,
        ParentStep::Published,
        ParentStep::PublicationPending,
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
        false,
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
fn blocker_mask_round_trips_all_21() {
    assert_eq!(CLOSURE_BLOCKERS.len(), 21);
    assert!(CLOSURE_BLOCKERS.windows(2).all(|p| p[0] < p[1]));
    assert_eq!(CLOSURE_BLOCKERS[0], ClosureBlocker::NotATicket);
    assert_eq!(CLOSURE_BLOCKERS[20], ClosureBlocker::IdentityRequired);
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
    assert!(!rejected(&tx, r.operation_id, "refusal_blockers=2097151"));
    for wrong in ["refusal_blockers=0", "refusal_blockers=2097152"] {
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
                ""
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
            false,
        )
    })
    .unwrap();
    (data, root, service)
}
