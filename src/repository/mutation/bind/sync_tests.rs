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

/// What `stopped` is called with for a synchronization whose operation
/// row ended as cancelled, as `ended_if_unresolved` leaves it.
struct Ended {
    _data: tempfile::TempDir,
    _root: tempfile::TempDir,
    service: RepositoryService,
    scope: ScopeKey,
    accepted: Accepted,
    binding: SyncBinding,
}

impl Ended {
    fn new() -> Self {
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
        let binding = SyncBinding {
            root: root.path().to_owned(),
            target,
            republish: None,
        };
        Self {
            _data: data,
            _root: root,
            service,
            scope,
            accepted,
            binding,
        }
    }

    /// What the binding answers `error` over the ended row.
    fn stopped(&self, error: &SynchronizationError) -> Ran {
        let call = CallContext {
            service: &self.service,
            session: &NoSession,
            confirmation: None,
        };
        let watch = Watch::new(
            &self.accepted,
            &call,
            Subject::of(&self.binding, &self.accepted),
            None,
            None,
        );
        self.binding
            .stopped(&self.service, &self.accepted, &watch, error)
    }
}

/// A finished (cancelled) context sync, as `ended_if_unresolved` leaves it,
/// is answered by the binding with the repair, and settles as Finish.
#[test]
fn unresolved_history_is_answered_with_the_repair_and_no_retry() {
    let ended = Ended::new();
    let scope = &ended.scope;
    let ran = ended.stopped(&SynchronizationError::HistoryUnresolved);
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
    let ran = ended.stopped(&SynchronizationError::RecoveryRequired);
    assert_eq!(ran.answer.code, ResultCode::Cancelled);
}

/// A pull-only refusal ends its operation as a cancelled one (design Part
/// D), and is answered as itself: its own code, blocked, with no recovery
/// action, every effect `not_requested` (`effects_so_far(Unknown)` would
/// say the publication is pending), and its data. It is not the caller's
/// cancellation.
#[test]
fn local_commits_refusal_is_answered_as_itself() {
    let ended = Ended::new();
    let first = git2::Oid::from_str("0123456789abcdef0123456789abcdef01234567").unwrap();
    let second = git2::Oid::from_str("fedcba9876543210fedcba9876543210fedcba98").unwrap();
    for (commits, complete) in [
        (
            vec![
                (first, "Add a thing".to_owned()),
                (second, "Fix\tit".to_owned()),
            ],
            false,
        ),
        // A known limit: the push endpoint ahead of the fetched primary
        // refuses with nothing to list.
        (Vec::new(), true),
    ] {
        let ran = ended.stopped(&SynchronizationError::PrimaryHasLocalCommits {
            commits: commits.clone(),
            complete,
        });
        let answer = &ran.answer;
        assert_eq!(answer.code, ResultCode::PrimaryHasLocalCommits);
        assert_ne!(answer.code, ResultCode::Cancelled);
        assert!(answer.recovery.is_empty());
        assert_eq!(answer.effects, Effects::not_requested());
        assert!(!answer.local_change);
        let Some(MutationDataDto::PrimaryLocalCommits(data)) = &answer.data else {
            panic!("{:?}", answer.data)
        };
        assert_eq!(
            data.r#type,
            crate::repository::mutation::dto::PrimaryLocalCommitsType::PrimaryLocalCommits
        );
        assert_eq!(data.complete, complete);
        assert_eq!(
            data.local_commits
                .iter()
                .map(|commit| (commit.oid.clone(), commit.summary.clone()))
                .collect::<Vec<_>>(),
            commits
                .iter()
                .map(|(oid, summary)| (oid.to_string(), summary.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            data.local_commits
                .iter()
                .all(|commit| commit.oid.len() == 40),
            "{data:?}"
        );
        // The envelope the answer makes is blocked.
        assert_eq!(
            answer.code.failure_class(),
            Some(crate::results::FailureClass::Blocked)
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
    }
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
