use super::{
    super::super::{
        super::{
            Blocker, OperationFamily, REGISTRY_FILE, RemoteOperationAction, RemoteOperationTarget,
            RemoteRefPlan, RemoteReservationOutcome, SynchronizationError, remote::state,
        },
        identity::ScopeKey,
        outcome::CallKind,
        reply::{Kept, Reply, RequestIdentity},
    },
    *,
};

/// A `comment add` whose publication was refused because another
/// synchronization's push is one nothing answered, as `push_absence` names
/// it, is that synchronization's answer: `recovery_required` naming the
/// operation in its way. The envelope carries it as `data.blocker` in place
/// of the comment's, offers `operation.show` for it, and its abandon, since
/// the abandon closes it: a synchronization a cancellation ended as its
/// push returned, which nothing else names. The comment is saved and
/// committed all the same, as the effects say.
///
/// The synchronization's own answer is made as its binding makes it for
/// that refusal (`sync_tests`): no server runs here.
#[test]
fn a_comment_whose_publication_meets_another_s_unanswered_push_names_it() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let root = parent.path().to_owned();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let scope = ScopeKey::for_repository(&root).unwrap();
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
    let item: ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        item.clone(),
    )
    .unwrap();
    let stranded = OperationId::new();
    assert!(matches!(
        service
            .reserve_remote_operation(Path::new(scope.as_str()), stranded, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
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
    let blocker = Blocker {
        family: OperationFamily::Remote,
        operation_id: stranded,
        live: None,
        seen: None,
    };
    // The synchronization's binding, refused so, names the operation.
    let child = Child {
        ran: Ran {
            answer: Answer::stopped(ResultCode::RecoveryRequired).naming(
                &service,
                Path::new(scope.as_str()),
                Some(&blocker),
            ),
            standing: Standing::Stopped { journal: None },
        },
        succeeded: false,
    };
    let operation_id = OperationId::new();
    let synchronization_id = OperationId::new();
    let receipt = CommentReceipt {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        item_id: item.clone(),
        comment_id: "01ARZ3NDEKTSV4RRFFQ69G5FC1".parse().unwrap(),
        parent_id: None,
        operation_id,
        synchronization_id,
        comment_path: PathBuf::from("comment.md"),
        context_branch: format!("manyhands/ticket/{item}"),
        checkpoint_oid: git2::Oid::from_bytes(&[2; 20]).unwrap(),
    };
    let publication = crate::repository::comment_publication::pending_synchronization(
        SynchronizationError::Repository(
            state::recovery_required().with_blocker(Some(blocker.clone())),
        ),
    );

    let ran = saved(
        &receipt,
        publication,
        CommentIndexingState::default(),
        Some(child),
    );

    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    assert_eq!(ran.answer.effects.checkpoint, CheckpointEffect::Committed);
    let binding = CommentAddBinding {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item,
        comment_id: receipt.comment_id.clone(),
        parent_id: None,
        body: "A comment.\n".to_owned(),
        created_by: None,
    };
    let accepted = Accepted {
        request_id: RequestId::new(),
        attempt: 1,
        scope: scope.clone(),
        family: Some(OperationFamily::Local),
        operation_id,
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    };
    let identity = RequestIdentity::of(accepted.request_id, &binding);
    let envelope = Reply {
        binding: &binding,
        identity: &identity,
    }
    .envelope(ran.answer, Some(&accepted), Kept::Accepted);
    let envelope = service.offering(envelope, CallKind::Execute);
    let value = serde_json::to_value(&envelope).unwrap();
    assert_eq!(value["code"], "recovery_required");
    assert_eq!(
        value["data"]["blocker"]["operation_id"],
        stranded.to_string()
    );
    assert_eq!(value["data"]["blocker"]["family"], "remote");
    let named = |action: &str| {
        envelope.recovery.iter().any(|offered| {
            offered.action.as_str() == action
                && offered.operation_id.as_deref() == Some(stranded.to_string().as_str())
        })
    };
    assert!(named("operation.show"), "{:?}", envelope.recovery);
    assert!(named("operation.abandon"), "{:?}", envelope.recovery);
}
