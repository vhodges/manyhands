use std::path::Path;

use super::{
    super::super::{
        super::{
            REGISTRY_FILE, RemoteRefPlan, RemoteReservationOutcome, SynchronizationError,
            SynchronizationTarget, remote::state,
        },
        identity::ScopeKey,
        progress::{NoSession, Subject},
    },
    *,
};

/// A finished (cancelled) context sync, as `ended_if_unresolved` leaves it,
/// is answered by the binding with the repair, and settles as Finish.
#[test]
fn unresolved_history_is_answered_with_the_repair_and_no_retry() {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let scope = ScopeKey::for_repository(root.path()).unwrap();
    rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .execute(
            "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
            [scope.as_str()],
        )
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, Path::new(scope.as_str()), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };
    let operation_id = OperationId::new();
    let owner = match service
        .reserve_remote_operation(
            Path::new(scope.as_str()),
            operation_id,
            &target.operation_target(&plan),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    service
        .request_synchronization_cancellation(Path::new(scope.as_str()), &owner)
        .unwrap();
    service
        .check_synchronization_requests(Path::new(scope.as_str()), &owner)
        .unwrap();

    let accepted = Accepted {
        request_id: RequestId::new(),
        attempt: 1,
        scope: scope.clone(),
        family: Some(OperationFamily::Remote),
        operation_id,
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    };
    let call = CallContext {
        service: &service,
        session: &NoSession,
        confirmation: None,
    };
    let binding = SyncBinding {
        root: root.path().to_owned(),
        target,
        republish: None,
    };
    let watch = Watch::new(
        &accepted,
        &call,
        Subject::of(&binding, &accepted),
        None,
        None,
    );
    let ran = binding.stopped(
        &service,
        &accepted,
        &watch,
        &SynchronizationError::HistoryUnresolved,
    );
    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    let actions: Vec<_> = ran
        .answer
        .recovery
        .iter()
        .map(|action| (action.action, action.arguments.clone()))
        .collect();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].0, RecoveryActionKind::RemoteHistory);
    assert_eq!(
        actions[0].1.get("root").and_then(|root| root.as_str()),
        Some(scope.as_str())
    );
    assert!(matches!(
        ran.standing,
        Standing::Stopped {
            journal: Some(JournalRow::Final {
                kind: FinalKind::Cancelled,
                ..
            })
        }
    ));
    // The same row under any other error is the caller's own cancellation.
    let ran = binding.stopped(
        &service,
        &accepted,
        &watch,
        &SynchronizationError::RecoveryRequired,
    );
    assert_eq!(ran.answer.code, ResultCode::Cancelled);
}

/// A synchronization refused because another operation's push is one
/// nothing answered, as `push_absence` names it (the error its boundary
/// gives, `remote::sync`), is answered `recovery_required` naming that
/// operation: the envelope carries it as `data.blocker` and offers
/// `operation.show`, and its abandon where it closes it
/// (`outcome_tests`). Here the operation is a synchronization a
/// cancellation ended as its push returned, which nothing else names.
#[test]
fn a_synchronization_refused_for_another_s_unanswered_push_names_it() {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let scope = ScopeKey::for_repository(root.path()).unwrap();
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute(
        "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
        [scope.as_str()],
    )
    .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, Path::new(scope.as_str()), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };
    let reserve = |operation_id| match service
        .reserve_remote_operation(
            Path::new(scope.as_str()),
            operation_id,
            &target.operation_target(&plan),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    let stranded = OperationId::new();
    drop(reserve(stranded));
    db.execute(
        "UPDATE remote_operation_records
            SET phase = 'cancelled', outcome = 'cancelled', sync_checkpoint = 'push_returned',
                completed_step = 'before_push', expected_oid = ?2, local_oid = ?2,
                primary_tracking_oid = ?2, push_oid = ?2
          WHERE operation_ulid = ?1",
        [
            stranded.to_string(),
            "1111111111111111111111111111111111111111".to_owned(),
        ],
    )
    .unwrap();
    let operation_id = OperationId::new();
    drop(reserve(operation_id));
    let accepted = Accepted {
        request_id: RequestId::new(),
        attempt: 1,
        scope: scope.clone(),
        family: Some(OperationFamily::Remote),
        operation_id,
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    };
    let call = CallContext {
        service: &service,
        session: &NoSession,
        confirmation: None,
    };
    let binding = SyncBinding {
        root: root.path().to_owned(),
        target,
        republish: None,
    };
    let watch = Watch::new(
        &accepted,
        &call,
        Subject::of(&binding, &accepted),
        None,
        None,
    );
    let refused = SynchronizationError::Repository(state::recovery_required().with_blocker(Some(
        crate::repository::Blocker {
            family: OperationFamily::Remote,
            operation_id: stranded,
            live: None,
            seen: None,
        },
    )));

    let ran = binding.stopped(&service, &accepted, &watch, &refused);

    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    let blocker = ran.answer.blocker.expect("the operation in its way");
    assert_eq!(blocker.operation_id, stranded.to_string());
    assert_eq!(blocker.family, OperationFamily::Remote);
    // It holds no reservation: no call works under it.
    assert_eq!(blocker.live, Some(false));
}
