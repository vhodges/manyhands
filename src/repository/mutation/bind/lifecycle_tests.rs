use super::super::super::{identity::RequestId, progress::Session};
use super::*;
use crate::repository::{
    ConflictNextAction, ItemDtoKind, LifecyclePathChangeDto, LifecyclePathChangeKind,
    RepositoryError, RepositoryErrorKind, RepositoryOperation,
};

fn reached(publication: PromotionPublication, cleanup: PromotionCleanup) -> Reached {
    Reached {
        recording: None,
        integration: PromotionIntegration::Merged {
            oid: git2::Oid::zero(),
        },
        publication,
        cleanup,
        index_pending: false,
    }
}

fn pending_publication(reason: PromotionPublicationPending) -> PromotionPublication {
    PromotionPublication::Pending { reason }
}

fn pending_cleanup(reason: PromotionCleanupPending) -> PromotionCleanup {
    PromotionCleanup::Pending { reason }
}

const AWAITING: PromotionCleanup = PromotionCleanup::Pending {
    reason: PromotionCleanupPending::AwaitingPublication,
};

#[test]
fn a_publication_that_stopped_answers_with_its_category() {
    use PromotionSynchronizationCategory as Category;
    for (category, code) in [
        (Category::Conflict, ResultCode::RecoveryRequired),
        (Category::Rejected, ResultCode::PushRejected),
        (Category::Transport, ResultCode::TransportUnavailable),
        (Category::Identity, ResultCode::IdentityRequired),
        (Category::Busy, ResultCode::Busy),
        (Category::Yielding, ResultCode::PollYielding),
        (Category::RecoveryRequired, ResultCode::RecoveryRequired),
    ] {
        let stopped = reached(
            pending_publication(PromotionPublicationPending::Synchronization(category)),
            AWAITING,
        );
        assert_eq!(reached_code(&stopped), code, "{category:?}");
    }
}

#[test]
fn a_deletion_that_stopped_answers_with_its_category() {
    use PromotionRemoteDeletionCategory as Category;
    for (category, code) in [
        (Category::Transport, ResultCode::TransportUnavailable),
        (Category::Rejected, ResultCode::PushRejected),
        (Category::Configuration, ResultCode::RecoveryRequired),
        (Category::Key, ResultCode::NoSelectedKey),
        (Category::Unlock, ResultCode::UnlockRequired),
        (Category::Host, ResultCode::HostApprovalRequired),
    ] {
        let stopped = reached(
            PromotionPublication::Published {
                oid: git2::Oid::zero(),
            },
            pending_cleanup(PromotionCleanupPending::RemoteDeletionFailed(category)),
        );
        assert_eq!(reached_code(&stopped), code, "{category:?}");
    }
}

#[test]
fn cleanup_stops_that_name_no_category_are_recovery() {
    for reason in [
        PromotionCleanupPending::ContextChanged,
        PromotionCleanupPending::IgnoredFiles,
        PromotionCleanupPending::RemoteContextChanged,
        PromotionCleanupPending::RecoveryRequired,
    ] {
        let stopped = reached(
            PromotionPublication::Published {
                oid: git2::Oid::zero(),
            },
            pending_cleanup(reason),
        );
        assert_eq!(reached_code(&stopped), ResultCode::RecoveryRequired);
    }
}

#[test]
fn cancelled_publication_or_cleanup_is_cancelled() {
    assert_eq!(
        reached_code(&reached(
            pending_publication(PromotionPublicationPending::Cancelled),
            AWAITING
        )),
        ResultCode::Cancelled
    );
    assert_eq!(
        reached_code(&reached(
            PromotionPublication::Published {
                oid: git2::Oid::zero()
            },
            pending_cleanup(PromotionCleanupPending::Cancelled)
        )),
        ResultCode::Cancelled
    );
}

/// No failure point yields this after a checkpoint, so this is the row. A
/// repository error of kind `ExternalChange` is `external_change` by its
/// own mapping, and is not a consent refusal: a standing parent decides
/// what is left, and the domain's half is `closure/run_tests.rs`.
#[test]
fn a_repository_external_change_stays_a_repository_error() {
    let stop = Stop::Repository(RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::ExternalChange,
        "moved",
    ));
    let (code, blockers, offers_nothing) = stop_answer(&stop);
    assert_eq!(code, ResultCode::ExternalChange);
    assert_eq!(blockers, None);
    assert!(!offers_nothing);
}

#[test]
fn a_stop_answers_with_the_tables_code() {
    for (stop, code) in [
        (Stop::Busy, ResultCode::Busy),
        (Stop::PollYielding, ResultCode::PollYielding),
        (Stop::Interrupted, ResultCode::RecoveryRequired),
        (Stop::RecoveryRequired, ResultCode::RecoveryRequired),
        (Stop::IdentityRequired, ResultCode::IdentityRequired),
        (Stop::PrimaryAdvanced, ResultCode::ExternalChange),
        (Stop::ExternalChange, ResultCode::ExternalChange),
        (Stop::ReconfirmationRequired, ResultCode::ExternalChange),
    ] {
        assert_eq!(stop_answer(&stop).0, code);
    }
    let (code, _, offers_nothing) = stop_answer(&Stop::OperationMismatch);
    assert_eq!(code, ResultCode::RequestMismatch);
    assert!(offers_nothing);
}

#[test]
fn with_no_parent_a_code_that_would_resend_prepares_again() {
    for code in [
        ResultCode::Busy,
        ResultCode::PollYielding,
        ResultCode::TransportUnavailable,
        ResultCode::RemoteUnavailable,
    ] {
        let offered = no_parent_recovery(code);
        assert_eq!(offered.len(), 1);
        assert_eq!(
            offered[0].action,
            crate::results::RecoveryActionKind::RequestPrepare
        );
    }
    assert!(no_parent_recovery(ResultCode::RecoveryRequired).is_empty());
    assert!(no_parent_recovery(ResultCode::InternalError).is_empty());
}

/// A verb whose parent is what the test says it is.
struct Fixed;

thread_local! {
    static PARENT: std::cell::RefCell<Option<ParentView<LifecycleBlocker>>> =
        const { std::cell::RefCell::new(None) };
    static KIND: std::cell::Cell<Option<IntegrationKind>> =
        const { std::cell::Cell::new(None) };
}

impl LifecycleVerb for Fixed {
    const COMMAND: Command = Command::TicketClose;
    const KIND: AuthoringKind = AuthoringKind::Ticket;
    const DTO_KIND: ItemDtoKind = ItemDtoKind::Ticket;
    const REPORTS_MADE_READY: bool = true;

    fn observe(
        _service: &RepositoryService,
        _root: &Path,
        _id: &ItemId,
    ) -> Result<Observed, RepositoryError> {
        Ok(Observed::Blocked(Vec::new()))
    }

    fn call(
        _session: &dyn Session,
        _service: &RepositoryService,
        _root: PathBuf,
        _operation_id: OperationId,
        _item_id: ItemId,
        _expected: [u8; 32],
        _request_id: RequestId,
        _progress: &mut dyn RunObserver,
    ) -> Option<Result<Called, Stop>> {
        None
    }

    fn parent(
        _service: &RepositoryService,
        _root: &Path,
        _operation: OperationId,
    ) -> Result<Option<ParentView<LifecycleBlocker>>, RepositoryError> {
        Ok(PARENT.with(|parent| parent.borrow_mut().take()))
    }

    fn holder(
        _db: &rusqlite::Connection,
        _repository_id: i64,
        _id: &ItemId,
    ) -> Result<Option<OperationId>, crate::repository::ReadError> {
        Ok(None)
    }
}

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

fn made_ready_state_of(ran: &Ran) -> Option<LifecycleMadeReadyState> {
    match &ran.answer.data {
        Some(MutationDataDto::Lifecycle(data)) => {
            assert_eq!(data.made_ready, Some(Vec::new()));
            data.made_ready_state
        }
        _ => panic!("a closure's stop reports"),
    }
}

/// A stop has no outcome to read the recording from, so its report comes
/// from the parent. The read fails here, since the root is nowhere: the
/// request is owed, so it says the next call reads again.
#[test]
fn a_closures_stop_reports_from_its_parent() {
    let before = stopped_with(ParentStep::ContextSynchronized, None, Stop::Busy);
    assert_eq!(
        made_ready_state_of(&before),
        Some(LifecycleMadeReadyState::NotComputed)
    );
    let past = stopped_with(ParentStep::Integrated, None, Stop::Busy);
    assert_eq!(
        made_ready_state_of(&past),
        Some(LifecycleMadeReadyState::IndexPending)
    );
    KIND.with(|kind| kind.set(Some(IntegrationKind::AlreadyContained)));
    let cleanup = stopped_with(ParentStep::Integrated, None, Stop::Busy);
    assert_eq!(
        made_ready_state_of(&cleanup),
        Some(LifecycleMadeReadyState::NotApplicable)
    );
    // A parent that says it is complete, under a stop of this call's.
    let complete = stopped_with(ParentStep::Complete, None, Stop::Busy);
    assert_eq!(
        made_ready_state_of(&complete),
        Some(LifecycleMadeReadyState::NotApplicable)
    );
    KIND.with(|kind| kind.set(None));
}

/// What `stopped` answers `stop` with, the parent being at `step`.
fn stopped_with(step: ParentStep, refusal: Option<Refusal<LifecycleBlocker>>, stop: Stop) -> Ran {
    let id: ItemId = ITEM.parse().unwrap();
    PARENT.with(|parent| {
        *parent.borrow_mut() = Some(ParentView {
            step,
            refusal,
            expectation: [0; 32],
            item_id: id.clone(),
            checkpoint: None,
            integration: None,
            integration_kind: KIND.with(std::cell::Cell::get),
            publication: None,
            cancel_requested: false,
            children: [None, None],
            path: RecordPath::Direct,
            answered: false,
        });
    });
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let binding = LifecycleBinding::<Fixed> {
        root: PathBuf::from("/nowhere"),
        id,
        expected: None,
        closure_exists: false,
        verb: PhantomData,
    };
    binding.stopped(&service, &accepted(), stop, true)
}

fn accepted() -> Accepted {
    Accepted {
        request_id: crate::repository::mutation::identity::RequestId::new(),
        attempt: 1,
        scope: crate::repository::mutation::identity::ScopeKey::application(),
        family: Some(OperationFamily::Remote),
        operation_id: OperationId::new(),
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    }
}

/// A branch answer whose index step is yet to run is owed, and offers the
/// request that gave it, as a direct close's does; one that answered
/// otherwise is final, whatever the index owes.
#[test]
fn an_index_pending_branch_answer_is_owed_and_offers_its_own_request() {
    use crate::repository::{BranchAnswer, BranchClosureOutcome};
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let binding = LifecycleBinding::<Fixed> {
        root: PathBuf::from("/nowhere"),
        id: ITEM.parse().unwrap(),
        expected: None,
        closure_exists: false,
        verb: PhantomData,
    };
    let accepted = accepted();
    let published = PromotionPublication::Published {
        oid: git2::Oid::zero(),
    };
    let outcome = |answer| BranchClosureOutcome::for_testing(answer, None, Some(published));
    let ran = binding.branch_reached(&service, &accepted, &outcome(BranchAnswer::Published), true);
    assert_eq!(ran.answer.code, ResultCode::DiscoveryPending);
    assert!(matches!(ran.standing, Standing::Owed));
    assert_eq!(
        ran.answer.recovery,
        vec![crate::repository::mutation::outcome::same_request(
            accepted.request_id
        )]
    );
    // The same answer with its index done is final, and offers nothing.
    let ran = binding.branch_reached(
        &service,
        &accepted,
        &outcome(BranchAnswer::Published),
        false,
    );
    assert_eq!(ran.answer.code, ResultCode::Ok);
    assert!(matches!(ran.standing, Standing::Final));
    assert!(ran.answer.recovery.is_empty());
    // Another answer keeps its code, and is final: its record is finished.
    let ran = binding.branch_reached(
        &service,
        &accepted,
        &outcome(BranchAnswer::TicketClosed),
        true,
    );
    assert_eq!(ran.answer.code, ResultCode::TicketClosed);
    assert!(matches!(ran.standing, Standing::Final));
    assert!(ran.answer.recovery.is_empty());
}

/// A registered Git repository, and the link row its ticket was given.
struct Registered {
    _data: tempfile::TempDir,
    root: tempfile::TempDir,
    service: RepositoryService,
}

impl Registered {
    fn new(link: bool) -> Self {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        git2::Repository::init(root.path()).unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let scope =
            crate::repository::mutation::identity::ScopeKey::for_repository(root.path()).unwrap();
        let db =
            rusqlite::Connection::open(data.path().join(crate::repository::REGISTRY_FILE)).unwrap();
        db.execute(
            "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
            [scope.as_str()],
        )
        .unwrap();
        if link {
            let id: i64 = db
                .query_row("SELECT id FROM repositories", [], |row| row.get(0))
                .unwrap();
            crate::repository::pull_request_links::upsert_link(
                &db,
                id,
                "ticket",
                ITEM,
                &crate::forge::PullRequestLink {
                    source: format!("manyhands/ticket/{ITEM}"),
                    target: "main".to_owned(),
                    forge: Some(crate::canonical::ForgeKind::Github),
                    url: Some("https://forge.example/new".to_owned()),
                },
                1,
            )
            .unwrap();
        }
        Self {
            _data: data,
            root,
            service,
        }
    }

    fn binding(&self) -> LifecycleBinding<Fixed> {
        LifecycleBinding::<Fixed> {
            root: self.root.path().to_owned(),
            id: ITEM.parse().unwrap(),
            expected: None,
            closure_exists: false,
            verb: PhantomData,
        }
    }

    fn accepted(&self) -> Accepted {
        Accepted {
            scope: crate::repository::mutation::identity::ScopeKey::for_repository(
                self.root.path(),
            )
            .unwrap(),
            ..accepted()
        }
    }
}

/// A completed branch record answers with the branch record's own shape (design
/// Part H): effects with no integration or cleanup, whatever its action; data
/// that is the pull-request closure's, with the closure it recorded or kept or
/// withdrew, the commit it made, how its branch was published, what a new
/// branch retired and the link record's link; and no made-ready report, which
/// a closure that is not closed has none of. An answer that is not one of the
/// design's five carries no data.
#[test]
fn a_branch_record_answers_its_own_shape() {
    use crate::repository::{
        BranchAnswer as A, BranchClosureOutcome, BranchRecording as R, ClosureAction,
    };
    let oid = |text: &str| git2::Oid::from_str(&text.repeat(40)).unwrap();
    let published = PromotionPublication::Published { oid: oid("b") };
    let at = Some("2026-11-25T10:00:00Z");
    let registered = Registered::new(true);
    let binding = registered.binding();
    let accepted = registered.accepted();
    let ran = |outcome: BranchClosureOutcome| {
        binding.branch_reached(&registered.service, &accepted, &outcome, false)
    };
    let data_of = |ran: &Ran| match &ran.answer.data {
        Some(MutationDataDto::PullRequestClosure(data)) => Some(data.as_ref().clone()),
        None => None,
        other => panic!("not a branch record's data: {other:?}"),
    };
    let link = crate::repository::PullRequestDto {
        source: format!("manyhands/ticket/{ITEM}"),
        target: "main".to_owned(),
        forge: Some("github".to_owned()),
        url: Some("https://forge.example/new".to_owned()),
    };

    // A close that recorded its closure and published it.
    let recorded =
        ran(
            BranchClosureOutcome::for_testing(A::Published, Some(oid("a")), Some(published))
                .for_testing_with(ClosureAction::Close, R::Recorded, at, None),
        );
    assert_eq!(recorded.answer.code, ResultCode::Ok);
    assert!(matches!(recorded.standing, Standing::Final));
    let effects = &recorded.answer.effects;
    assert_eq!(effects.write, crate::results::WriteEffect::Written);
    assert_eq!(effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(effects.publication, PublicationEffect::Published);
    assert_eq!(effects.commit_oid, Some(oid("b").to_string()));
    // Nothing reached primary, so nothing integrated or cleaned up.
    assert_eq!(effects.integration, IntegrationEffect::NotRequested);
    assert_eq!(effects.cleanup, CleanupEffect::NotRequested);
    let data = data_of(&recorded).expect("a close that recorded reports");
    assert_eq!(
        data.action,
        crate::repository::PullRequestClosureAction::Close
    );
    assert_eq!(
        data.closure,
        crate::repository::PullRequestClosureDto {
            recording: crate::repository::PullRequestClosureRecording::Recorded,
            closed_at: at.map(str::to_owned),
        }
    );
    assert_eq!(data.checkpoint_oid, Some(oid("a").to_string()));
    assert_eq!(
        data.publication,
        Some(crate::repository::PullRequestClosurePublication::Published)
    );
    assert_eq!(data.retired_context, None);
    assert_eq!(data.pull_request, Some(link.clone()));
    assert!(data.blockers.is_empty());
    // The data serializes with no made-ready report and no `closed_by`.
    let json = serde_json::to_value(&data).unwrap();
    for key in ["made_ready", "made_ready_state", "closed_by", "integration"] {
        assert!(
            json.get(key).is_none() && json["closure"].get(key).is_none(),
            "{key}"
        );
    }

    // A new branch that retired its old context, found the remote current,
    // and kept the closure the old context held.
    let retired = oid("c");
    let new_branch = ran(BranchClosureOutcome::for_testing(
        A::Published,
        Some(oid("a")),
        Some(PromotionPublication::AlreadyCurrent { oid: oid("b") }),
    )
    .for_testing_with(ClosureAction::NewBranch, R::Existing, None, Some(retired)));
    let data = data_of(&new_branch).unwrap();
    // A closure it kept has no time of the record's to give (ruling 60).
    assert_eq!(data.closure.closed_at, None);
    assert_eq!(
        data.action,
        crate::repository::PullRequestClosureAction::NewBranch
    );
    assert_eq!(
        data.closure.recording,
        crate::repository::PullRequestClosureRecording::Existing
    );
    assert_eq!(data.retired_context, Some(retired.to_string()));
    assert_eq!(
        data.publication,
        Some(crate::repository::PullRequestClosurePublication::Current)
    );
    assert_eq!(
        new_branch.answer.effects.publication,
        PublicationEffect::Current
    );

    // A withdraw reports the link record as it stands, and no closure time.
    let withdrawn =
        ran(
            BranchClosureOutcome::for_testing(A::Withdrawn, Some(oid("a")), Some(published))
                .for_testing_with(ClosureAction::Withdraw, R::Withdrawn, None, None),
        );
    assert_eq!(withdrawn.answer.code, ResultCode::Ok);
    let data = data_of(&withdrawn).unwrap();
    assert_eq!(
        data.action,
        crate::repository::PullRequestClosureAction::Withdraw
    );
    assert_eq!(
        data.closure,
        crate::repository::PullRequestClosureDto {
            recording: crate::repository::PullRequestClosureRecording::Withdrawn,
            closed_at: None,
        }
    );
    assert_eq!(data.pull_request, Some(link));

    // Each answer the design gives data to, and the ones it does not.
    for (answer, code, data_named, blockers) in [
        (A::Existing, ResultCode::AlreadyClosing, true, vec![]),
        (A::AlreadyApplied, ResultCode::AlreadyApplied, true, vec![]),
        (
            A::RemoteBranchDeleted,
            ResultCode::RemoteBranchDeleted,
            true,
            vec![],
        ),
        (A::TicketClosed, ResultCode::TicketClosed, true, vec![]),
        (
            A::AlreadyClosed,
            ResultCode::TicketClosed,
            true,
            vec![LifecycleBlocker::AlreadyClosed],
        ),
        (A::ExternalChange, ResultCode::ExternalChange, false, vec![]),
        // E-46 (owner, 2026-11-26, R-35): the one `recovery_required` answer
        // that names what the record did, so `item sync` can be named as the
        // way out. Its sisters, `context_changed` and
        // `context_history_unresolved`, stay without data.
        (A::RemoteChanged, ResultCode::RecoveryRequired, true, vec![]),
        (
            A::ContextChanged,
            ResultCode::RecoveryRequired,
            false,
            vec![],
        ),
        (
            A::ContextHistoryUnresolved,
            ResultCode::RecoveryRequired,
            false,
            vec![],
        ),
    ] {
        let ran = ran(BranchClosureOutcome::for_testing(answer, None, None));
        assert_eq!(ran.answer.code, code, "{answer:?}");
        let data = data_of(&ran);
        assert_eq!(data.is_some(), data_named, "{answer:?}");
        if let Some(data) = data {
            assert_eq!(data.blockers, blockers, "{answer:?}");
            assert_eq!(data.publication, None, "{answer:?}");
            assert_eq!(data.checkpoint_oid, None, "{answer:?}");
        }
    }

    // An item with no link record reports none.
    let bare = Registered::new(false);
    let bare_binding = bare.binding();
    let ran = bare_binding.branch_reached(
        &bare.service,
        &bare.accepted(),
        &BranchClosureOutcome::for_testing(A::Published, Some(oid("a")), Some(published))
            .for_testing_with(ClosureAction::Close, R::Recorded, at, None),
        false,
    );
    assert_eq!(data_of(&ran).unwrap().pull_request, None);
}

/// A repository with a configured remote, and a child synchronization of the
/// ticket whose row says its push was accepted at `pushed`.
fn pushed_child(registered: &Registered, pushed: git2::Oid) -> OperationId {
    use crate::repository::{
        RemoteRefPlan, RemoteReservationOutcome, SynchronizationTarget, remote::state,
    };
    let root = registered.root.path();
    let scope = crate::repository::mutation::identity::ScopeKey::for_repository(root).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&registered.service, Path::new(scope.as_str()), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
    };
    let child = OperationId::new();
    match registered
        .service
        .reserve_remote_operation(
            Path::new(scope.as_str()),
            child,
            &target.operation_target(&plan),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => drop(owner),
        other => panic!("{other:?}"),
    }
    let db = rusqlite::Connection::open(
        registered
            ._data
            .path()
            .join(crate::repository::REGISTRY_FILE),
    )
    .unwrap();
    db.execute(
        "UPDATE remote_operation_records
            SET sync_checkpoint = 'push_verified', push_oid = ?2
          WHERE operation_ulid = ?1",
        [child.to_string(), pushed.to_string()],
    )
    .unwrap();
    child
}

/// A commit in the registered repository.
fn a_commit(registered: &Registered) -> git2::Oid {
    let repository = git2::Repository::open(registered.root.path()).unwrap();
    let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
    let tree = repository
        .find_tree(repository.index().unwrap().write_tree().unwrap())
        .unwrap();
    repository
        .commit(None, &signature, &signature, "child", &tree, &[])
        .unwrap()
}

/// R-32: a finished branch record that has no publication child's result of
/// its own still shows what its context child published. A withdraw that
/// finds, at step 3, another clone's withdraw already merged in
/// (`already_applied`) wrote nothing and ran no publication child, but its
/// context child pushed the pending work, and its effects say so.
#[test]
fn a_branch_record_s_answer_shows_what_its_context_child_published() {
    use crate::repository::{BranchAnswer as A, BranchClosureOutcome, ClosureAction};
    let registered = Registered::new(false);
    let pushed = a_commit(&registered);
    let child = pushed_child(&registered, pushed);
    let binding = registered.binding();
    let accepted = registered.accepted();
    let nothing_published = BranchClosureOutcome::for_testing(A::AlreadyApplied, None, None)
        .for_testing_with(
            ClosureAction::Withdraw,
            crate::repository::BranchRecording::Nothing,
            None,
            None,
        );
    let parent = |child| {
        PARENT.with(|parent| {
            *parent.borrow_mut() = Some(parent_of(
                RecordPath::Branch,
                ParentStep::Abandoned,
                None,
                [Some(child), None],
            ));
        })
    };
    parent(child);
    let ran = binding.branch_reached(&registered.service, &accepted, &nothing_published, false);
    // The code is still `already_applied`, and nothing of its own was
    // written; what the child pushed is shown.
    assert_eq!(ran.answer.code, ResultCode::AlreadyApplied);
    let effects = &ran.answer.effects;
    assert_eq!(effects.publication, PublicationEffect::Published);
    assert_eq!(effects.commit_oid, Some(pushed.to_string()));
    assert_eq!(effects.write, crate::results::WriteEffect::NotRequested);
    assert_eq!(effects.checkpoint, CheckpointEffect::NotRequested);
    // A child that pushed nothing shows nothing.
    parent(OperationId::new());
    let ran = binding.branch_reached(&registered.service, &accepted, &nothing_published, false);
    assert_eq!(
        ran.answer.effects.publication,
        PublicationEffect::NotRequested
    );
    assert_eq!(ran.answer.effects.commit_oid, None);
}

/// A parent of the given path whose children are the given operations.
fn parent_of(
    path: RecordPath,
    step: ParentStep,
    checkpoint: Option<git2::Oid>,
    children: [Option<OperationId>; 2],
) -> ParentView<LifecycleBlocker> {
    ParentView {
        step,
        refusal: None,
        expectation: [0; 32],
        item_id: ITEM.parse().unwrap(),
        checkpoint,
        integration: None,
        integration_kind: None,
        publication: None,
        cancel_requested: false,
        children,
        path,
        answered: false,
    }
}

/// What a stopped branch record shows of its effects (design R-5): the
/// closure commit it made on the context's branch is written and committed
/// once the context holds it, and what its publication child pushed is
/// published; nothing integrated or cleaned up. A direct close's stop shows
/// neither the write nor a second child's push, which it has no use for.
#[test]
fn a_stopped_branch_record_shows_what_it_wrote_and_its_publication_pushed() {
    use crate::repository::{
        RemoteRefPlan, RemoteReservationOutcome, SynchronizationTarget, remote::state,
    };
    let registered = Registered::new(false);
    let root = registered.root.path();
    let repository = git2::Repository::open(root).unwrap();
    let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
    let tree = repository
        .find_tree(repository.index().unwrap().write_tree().unwrap())
        .unwrap();
    let first = repository
        .commit(Some("HEAD"), &signature, &signature, "first", &tree, &[])
        .unwrap();
    let closure = repository
        .commit(
            None,
            &signature,
            &signature,
            "closure",
            &tree,
            &[&repository.find_commit(first).unwrap()],
        )
        .unwrap();
    repository
        .reference(
            &format!("refs/heads/manyhands/ticket/{ITEM}"),
            closure,
            false,
            "context",
        )
        .unwrap();
    // The publication child's row says its push was accepted at the commit.
    let scope = crate::repository::mutation::identity::ScopeKey::for_repository(root).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&registered.service, Path::new(scope.as_str()), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
    };
    let publication = OperationId::new();
    let owner = match registered
        .service
        .reserve_remote_operation(
            Path::new(scope.as_str()),
            publication,
            &target.operation_target(&plan),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    drop(owner);
    let db = rusqlite::Connection::open(
        registered
            ._data
            .path()
            .join(crate::repository::REGISTRY_FILE),
    )
    .unwrap();
    db.execute(
        "UPDATE remote_operation_records
            SET sync_checkpoint = 'push_verified', push_oid = ?2
          WHERE operation_ulid = ?1",
        [publication.to_string(), closure.to_string()],
    )
    .unwrap();
    let effects = |path, step, children| {
        parent_effects(
            &AuthoringKind::Ticket,
            &registered.service,
            root,
            Some("refs/heads/main"),
            &parent_of(path, step, Some(closure), children),
        )
    };
    let stopped = effects(
        RecordPath::Branch,
        ParentStep::PublicationPending,
        [None, Some(publication)],
    );
    assert_eq!(stopped.write, crate::results::WriteEffect::Written);
    assert_eq!(stopped.checkpoint, CheckpointEffect::Committed);
    assert_eq!(stopped.publication, PublicationEffect::Published);
    assert_eq!(stopped.commit_oid, Some(closure.to_string()));
    assert_eq!(stopped.integration, IntegrationEffect::NotRequested);
    assert_eq!(stopped.cleanup, CleanupEffect::NotRequested);
    // The same, with its closure written and nothing pushed yet.
    let unpushed = effects(
        RecordPath::Branch,
        ParentStep::PublicationPending,
        [None, Some(OperationId::new())],
    );
    assert_eq!(unpushed.write, crate::results::WriteEffect::Written);
    assert_eq!(unpushed.checkpoint, CheckpointEffect::Committed);
    assert_eq!(unpushed.publication, PublicationEffect::NotRequested);
    // A record that has written nothing says nothing of its own.
    let nothing = parent_effects(
        &AuthoringKind::Ticket,
        &registered.service,
        root,
        Some("refs/heads/main"),
        &parent_of(
            RecordPath::Branch,
            ParentStep::ContextSynchronized,
            None,
            [None, None],
        ),
    );
    assert_eq!(nothing.write, crate::results::WriteEffect::NotRequested);
    assert_eq!(nothing.checkpoint, CheckpointEffect::NotRequested);
    // A direct close, before it integrates, shows its checkpoint and not a
    // write, and does not read a second child.
    let direct = effects(
        RecordPath::Direct,
        ParentStep::Confirmed,
        [None, Some(publication)],
    );
    assert_eq!(direct.write, crate::results::WriteEffect::NotRequested);
    assert_eq!(direct.checkpoint, CheckpointEffect::Committed);
    assert_eq!(direct.publication, PublicationEffect::NotRequested);
}

/// What `stopped` answers `stop` with, the parent being a branch record at
/// `step` that has written nothing.
fn branch_stopped_with(step: ParentStep, stop: Stop) -> Ran {
    let id: ItemId = ITEM.parse().unwrap();
    PARENT.with(|parent| {
        *parent.borrow_mut() = Some(parent_of(RecordPath::Branch, step, None, [None, None]));
    });
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let binding = LifecycleBinding::<Fixed> {
        root: PathBuf::from("/nowhere"),
        id,
        expected: None,
        closure_exists: false,
        verb: PhantomData,
    };
    binding.stopped(&service, &accepted(), stop, true)
}

/// A branch record's busy stop names the operation it was refused for as
/// `item sync`'s does (ruling 61): the one a repository error names, and the
/// one that holds the reservation of a child that was refused it.
#[test]
fn a_branch_record_s_busy_stop_names_its_blocker() {
    use crate::repository::{
        Blocker, OperationFamily, RemoteRefPlan, RemoteReservationOutcome, SynchronizationError,
        SynchronizationTarget, remote::state,
    };
    let registered = Registered::new(false);
    let root = registered.root.path();
    let scope = crate::repository::mutation::identity::ScopeKey::for_repository(root).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&registered.service, Path::new(scope.as_str()), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
    };
    // Another operation holds the repository's reservation.
    let holder = OperationId::new();
    let _held = match registered
        .service
        .reserve_remote_operation(
            Path::new(scope.as_str()),
            holder,
            &target.operation_target(&plan),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    let binding = registered.binding();
    let stopped = |stop| {
        PARENT.with(|parent| {
            *parent.borrow_mut() = Some(parent_of(
                RecordPath::Branch,
                ParentStep::PublicationPending,
                None,
                [None, None],
            ));
        });
        binding.stopped(&registered.service, &registered.accepted(), stop, true)
    };
    let named = |ran: &Ran| {
        assert_eq!(ran.answer.code, ResultCode::Busy);
        assert!(matches!(ran.standing, Standing::Owed));
        let blocker = ran
            .answer
            .blocker
            .as_ref()
            .expect("the operation in its way");
        assert_eq!(blocker.operation_id, holder.to_string());
        assert_eq!(blocker.family, OperationFamily::Remote);
    };
    // A refusal the repository gave, naming its holder.
    let refused = RepositoryError::new(
        RepositoryOperation::Read,
        Some(root.to_owned()),
        RepositoryErrorKind::RepositoryBusy,
        "busy",
    )
    .with_blocker(Some(Blocker {
        family: OperationFamily::Remote,
        operation_id: holder,
        live: None,
        seen: None,
    }));
    named(&stopped(Stop::Repository(refused)));
    // A child refused the reservation: the holder is read after the refusal.
    named(&stopped(Stop::ContextSynchronization(Box::new(
        SynchronizationError::Busy,
    ))));
}

/// A child of a branch record that stops answers as `item sync` answers
/// that stop (design R-5): its code, and its data, which is the host that is
/// not trusted, the conflict's next step, or none. A closure's report of the
/// tickets it made ready is not made, since the record closed nothing. A
/// direct close's child that stops keeps the lifecycle's own data.
#[test]
fn a_branch_record_s_child_stop_carries_the_synchronization_s_data() {
    use crate::repository::{
        SynchronizationError, SynchronizationStage, SynchronizationTarget,
        transport::{
            HostKeyIdentity, SshAuthority, SshDirection, SshTransportError, SshTransportErrorKind,
        },
    };
    let host = || {
        Stop::ContextSynchronization(Box::new(SynchronizationError::Transport(
            SshTransportError {
                root: PathBuf::from("/nowhere"),
                remote_name: "origin".into(),
                direction: SshDirection::Fetch,
                selected_key_id: None,
                authority: Some(SshAuthority {
                    host: "example.invalid".into(),
                    port: 22,
                }),
                kind: SshTransportErrorKind::HostApprovalRequired {
                    presented: HostKeyIdentity {
                        algorithm: "ssh-ed25519".into(),
                        sha256: "SHA256:abc".into(),
                    },
                },
            },
        )))
    };
    let conflicted = OperationId::new();
    let conflict = || {
        Stop::ContextSynchronization(Box::new(SynchronizationError::ConflictPending {
            target: SynchronizationTarget::Context {
                kind: AuthoringKind::Ticket,
                item_id: ITEM.parse().unwrap(),
            },
            operation_id: conflicted,
            stage: SynchronizationStage::Context,
        }))
    };

    let ran = branch_stopped_with(ParentStep::PublicationPending, host());
    assert_eq!(ran.answer.code, ResultCode::HostApprovalRequired);
    assert!(matches!(ran.standing, Standing::Owed));
    let Some(MutationDataDto::Synchronization(data)) = &ran.answer.data else {
        panic!("{:?}", ran.answer.data)
    };
    assert_eq!(data.authority(), "example.invalid:22");
    assert_eq!(data.presented_fingerprint, "SHA256:abc");

    let ran = branch_stopped_with(ParentStep::ContextSynchronized, conflict());
    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    let Some(MutationDataDto::SynchronizationConflict(data)) = &ran.answer.data else {
        panic!("{:?}", ran.answer.data)
    };
    assert_eq!(data.conflict.operation_id, conflicted.to_string());
    // A conflict that cannot be inspected is repaired outside Manyhands.
    assert_eq!(
        data.conflict.next_action,
        ConflictNextAction::ExternalRepair
    );

    // A stop that names nothing carries nothing: no made-ready report either.
    let ran = branch_stopped_with(ParentStep::PublicationPending, Stop::Busy);
    assert_eq!(ran.answer.code, ResultCode::Busy);
    assert!(ran.answer.data.is_none());
    assert!(matches!(ran.standing, Standing::Owed));

    // What stands in the way of a branch record's step is named as a blocked
    // prepare names it.
    let ran = branch_stopped_with(
        ParentStep::ContextSynchronized,
        Stop::Blocked(vec![LifecycleBlocker::ContextDirty]),
    );
    let Some(MutationDataDto::Lifecycle(data)) = &ran.answer.data else {
        panic!("{:?}", ran.answer.data)
    };
    assert_eq!(data.blockers, [LifecycleBlocker::ContextDirty]);

    // A direct close's child that stops keeps the lifecycle's data.
    let ran = stopped_with(ParentStep::Confirmed, None, host());
    assert!(matches!(
        ran.answer.data,
        Some(MutationDataDto::Lifecycle(_))
    ));
}

/// A pull-request close's preview (design Part H): the effects of each
/// action in the order the record makes them, and the details a branch record
/// shows: its action, the commits it starts from, the closure with its
/// `closed_by`, what a new branch retires and the unheld paths, and the link
/// a close gives (a withdraw gives none).
#[test]
fn a_branch_record_previews_its_own_effects_and_details() {
    use crate::repository::{ClosureAction as Action, PlannedCleanup, PreviewEffect as E};
    let oid = |text: &str| git2::Oid::from_str(&text.repeat(40)).unwrap();
    let record = |closed_by: &str| PlannedClosure::Record {
        closed_by: closed_by.to_owned(),
    };
    let kept = PlannedClosure::Existing {
        closed_at: "2026-11-25T10:00:00Z".to_owned(),
        closed_by: "A <a@example.invalid>".to_owned(),
    };
    let ready = |action, context_child, cleanup: (bool, bool), retired, unheld| Ready {
        expected: [0; 32],
        branch: format!("refs/heads/manyhands/ticket/{ITEM}"),
        primary_branch: "main".to_owned(),
        remote: Some("origin".to_owned()),
        context_oid: oid("1"),
        primary_oid: oid("2"),
        context_child,
        changes: Vec::new(),
        integration: PlannedIntegration::MergeCommit,
        publication: PlannedPublication::Publish,
        cleanup: PlannedCleanup {
            worktree: cleanup.0,
            local_branch: cleanup.1,
            remote_branch: false,
        },
        closure: None,
        record: Some(BranchReady {
            action,
            context_oid: Some(oid("1")),
            tracking_primary: Some(oid("3")),
            retired_oid: retired,
            unheld,
        }),
    };
    for (action, closure, context_child, cleanup, expected) in [
        (
            Action::Close,
            record("A <a@example.invalid>"),
            true,
            (false, false),
            vec![
                E::ContextSynchronized,
                E::ClosureRecorded,
                E::ContextPublished,
                E::PullRequestLink,
            ],
        ),
        (
            Action::Close,
            kept.clone(),
            true,
            (false, false),
            vec![E::ContextSynchronized, E::PullRequestLink],
        ),
        (
            Action::Withdraw,
            kept.clone(),
            true,
            (false, false),
            vec![
                E::ContextSynchronized,
                E::ClosureWithdrawn,
                E::ContextPublished,
            ],
        ),
        (
            Action::NewBranch,
            record("A <a@example.invalid>"),
            false,
            (true, true),
            vec![
                E::WorktreeRemoved,
                E::BranchDeleted,
                E::ContextCreated,
                E::ClosureRecorded,
                E::ContextPublished,
                E::PullRequestLink,
            ],
        ),
        (
            Action::NewBranch,
            kept.clone(),
            false,
            (false, false),
            vec![E::ContextCreated, E::ContextPublished, E::PullRequestLink],
        ),
    ] {
        let ready = ready(action, context_child, cleanup, None, Vec::new());
        assert_eq!(
            branch_effects(&ready, action, &closure),
            expected,
            "{action:?}"
        );
    }

    let item: ItemId = ITEM.parse().unwrap();
    let link = crate::forge::PullRequestLink {
        source: format!("manyhands/ticket/{ITEM}"),
        target: "main".to_owned(),
        forge: None,
        url: Some("https://forge.example/new".to_owned()),
    };
    let unheld = vec![LifecyclePathChangeDto {
        path: "docs/notes.md".to_owned(),
        change: LifecyclePathChangeKind::Added,
    }];
    let new_branch = ready(
        Action::NewBranch,
        false,
        (true, true),
        Some(oid("1")),
        unheld.clone(),
    );
    let details = branch_preview(
        "/repository",
        &item,
        Some("mh-1".to_owned()),
        &new_branch,
        new_branch.record.as_ref().unwrap(),
        &record("A <a@example.invalid>"),
        Some(link.clone()),
    );
    assert_eq!(details.action, PullRequestClosureAction::NewBranch);
    assert_eq!(details.slug.as_deref(), Some("mh-1"));
    assert_eq!(details.branch, format!("manyhands/ticket/{ITEM}"));
    assert_eq!(details.context_oid, Some(oid("1").to_string()));
    assert_eq!(details.tracking_primary_oid, Some(oid("3").to_string()));
    assert_eq!(details.retired_context, Some(oid("1").to_string()));
    assert_eq!(details.unheld, unheld);
    assert_eq!(
        details.closure,
        LifecyclePlannedClosureDto {
            recording: LifecycleRecording::Record,
            closed_by: "A <a@example.invalid>".to_owned()
        }
    );
    assert_eq!(
        details.pull_request,
        Some(crate::repository::PullRequestDto::from(&link))
    );
    // A kept closure is shown as existing, with its `closed_by`; a withdraw
    // gives no link.
    let withdraw = ready(Action::Withdraw, true, (false, false), None, Vec::new());
    let details = branch_preview(
        "/repository",
        &item,
        None,
        &withdraw,
        withdraw.record.as_ref().unwrap(),
        &kept,
        Some(link),
    );
    assert_eq!(details.action, PullRequestClosureAction::Withdraw);
    assert_eq!(details.closure.recording, LifecycleRecording::Existing);
    assert_eq!(details.closure.closed_by, "A <a@example.invalid>");
    assert_eq!(details.pull_request, None);
    assert_eq!(details.retired_context, None);
    let json = serde_json::to_value(&details).unwrap();
    assert_eq!(json["type"], "pull_request_closure_preview");
}

/// A branch record's answer gives the code of the design's table, and the
/// stored name of each reads back as the same answer.
#[test]
fn a_branch_answer_gives_its_code() {
    use crate::repository::BranchAnswer as A;
    for (answer, code, name) in [
        (A::Merged, ResultCode::Ok, "merged"),
        (A::Published, ResultCode::Ok, "published"),
        (A::WithdrawnSince, ResultCode::Ok, "withdrawn_since"),
        (A::Existing, ResultCode::AlreadyClosing, "existing"),
        (
            A::KeptPublished,
            ResultCode::AlreadyClosing,
            "kept_published",
        ),
        (
            A::AlreadyClosing,
            ResultCode::AlreadyClosing,
            "already_closing",
        ),
        (
            A::ClosedElsewhere,
            ResultCode::TicketClosed,
            "closed_elsewhere",
        ),
        (A::TicketClosed, ResultCode::TicketClosed, "ticket_closed"),
        (A::AlreadyClosed, ResultCode::TicketClosed, "already_closed"),
        (
            A::ContextTicketClosed,
            ResultCode::TicketClosed,
            "context_ticket_closed",
        ),
        (
            A::RemoteBranchDeleted,
            ResultCode::RemoteBranchDeleted,
            "remote_branch_deleted",
        ),
        (
            A::ContextRemoteBranchDeleted,
            ResultCode::RemoteBranchDeleted,
            "context_remote_branch_deleted",
        ),
        (
            A::RemoteChanged,
            ResultCode::RecoveryRequired,
            "remote_changed",
        ),
        (
            A::ContextHistoryUnresolved,
            ResultCode::RecoveryRequired,
            "context_history_unresolved",
        ),
        (
            A::ContextChanged,
            ResultCode::RecoveryRequired,
            "context_changed",
        ),
        (
            A::ExternalChange,
            ResultCode::ExternalChange,
            "external_change",
        ),
        (
            A::ReconfirmationRequired,
            ResultCode::ExternalChange,
            "reconfirmation_required",
        ),
        (A::Withdrawn, ResultCode::Ok, "withdrawn"),
        (
            A::WithdrawClosed,
            ResultCode::TicketClosed,
            "withdraw_closed",
        ),
        (A::ClosedSince, ResultCode::TicketClosed, "closed_since"),
        (
            A::AlreadyApplied,
            ResultCode::AlreadyApplied,
            "already_applied",
        ),
    ] {
        assert_eq!(branch_code(answer), code, "{name}");
        assert_eq!(answer.name(), name);
        assert_eq!(A::parse(name).unwrap(), answer);
    }
}

/// A branch record's view: it is never integrated, whatever its step, and
/// once it has answered, its request answers from the record, not from what
/// ended it.
#[test]
fn a_branch_parent_view_is_never_integrated() {
    let view = |step, path, answered, refusal| ParentView {
        step,
        refusal,
        expectation: [0; 32],
        item_id: ITEM.parse().unwrap(),
        checkpoint: None,
        integration: None,
        integration_kind: None,
        publication: None,
        cancel_requested: false,
        children: [None, None],
        path,
        answered,
    };
    let integrated =
        |view: &ParentView<LifecycleBlocker>| parent_integrated(Path::new("/nowhere"), None, view);
    for step in [ParentStep::PublicationPending, ParentStep::Complete] {
        assert!(
            integrated(&view(step, RecordPath::Direct, false, None)),
            "{step:?}"
        );
        assert!(
            !integrated(&view(step, RecordPath::Branch, false, None)),
            "{step:?}"
        );
    }
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let binding = LifecycleBinding::<Fixed> {
        root: PathBuf::from("/nowhere"),
        id: ITEM.parse().unwrap(),
        expected: None,
        closure_exists: false,
        verb: PhantomData,
    };
    let ended = |answered| {
        binding.ended_answer(
            &service,
            Path::new("/nowhere"),
            None,
            &view(
                ParentStep::Abandoned,
                RecordPath::Branch,
                answered,
                Some(Refusal::ExternalChange),
            ),
        )
    };
    // The refusal alone says external change; an answered record's own
    // answer is the domain's to give.
    assert_eq!(
        ended(false).map(|answer| answer.code),
        Some(ResultCode::ExternalChange)
    );
    assert!(ended(true).is_none());
    // A stop of an answered record is not its last word, as a complete
    // parent's is not.
    PARENT.with(|parent| {
        *parent.borrow_mut() = Some(view(
            ParentStep::Abandoned,
            RecordPath::Branch,
            true,
            Some(Refusal::ExternalChange),
        ));
    });
    let ran = binding.stopped(&service, &accepted(), Stop::Busy, true);
    assert_eq!(ran.answer.code, ResultCode::Busy);
    assert!(matches!(ran.standing, Standing::Owed));
    assert_eq!(ran.answer.recovery.len(), 1);
}

/// A stop is not stored because the parent has since completed.
#[test]
fn a_stop_of_a_complete_parent_leaves_the_record_for_its_stored_outcome() {
    let ran = stopped_with(ParentStep::Complete, None, Stop::Busy);
    assert_eq!(ran.answer.code, ResultCode::Busy);
    assert!(matches!(ran.standing, Standing::Owed));
    assert_eq!(ran.answer.recovery.len(), 1);
    assert_eq!(
        ran.answer.recovery[0].action,
        crate::results::RecoveryActionKind::RequestRetry
    );
}

/// A stop of a superseded parent answers as the parent does.
#[test]
fn a_stop_of_a_superseded_parent_is_its_request_mismatch() {
    let ran = stopped_with(ParentStep::Superseded, None, Stop::Busy);
    assert_eq!(ran.answer.code, ResultCode::RequestMismatch);
    assert!(ran.answer.offers_nothing);
    assert!(matches!(ran.standing, Standing::Final));
}

/// A stop of a refused parent answers with the stored refusal.
#[test]
fn a_stop_of_a_refused_parent_is_its_refusal() {
    let ran = stopped_with(
        ParentStep::Abandoned,
        Some(Refusal::Blocked(vec![LifecycleBlocker::PrimaryDirty])),
        Stop::Busy,
    );
    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    assert!(matches!(ran.standing, Standing::Final));
}

/// `ended_answer` answers whatever `abandoned` holds: it is not run.
#[test]
fn an_abandoned_parent_with_no_refusal_is_answered() {
    let ran = stopped_with(ParentStep::Abandoned, None, Stop::Busy);
    assert_eq!(ran.answer.code, ResultCode::ExternalChange);
    assert!(matches!(ran.standing, Standing::Final));
}

/// `merge_required` is told only to a caller set to refuse
/// divergence, and the bindings merge, so this is the row that can be had.
#[test]
fn a_stop_the_request_cannot_get_past_offers_its_cancellation() {
    use crate::results::RecoveryActionKind as Kind;
    for code in [ResultCode::MergeRequired, ResultCode::RemoteBranchDeleted] {
        let offered = standing_recovery(code, &accepted());
        assert_eq!(offered[0].action, Kind::RequestCancel, "{code:?}");
        assert!(
            offered
                .iter()
                .all(|action| action.action != Kind::RequestRetry),
            "{code:?}"
        );
    }
}

/// With no reservation to let go, or none that can be read, the action
/// that would let it go isn't offered, whatever a stop suggested.
#[test]
fn a_cancellation_is_not_offered_when_nothing_is_held() {
    use crate::results::{Envelope, RecoveryActionKind as Kind};
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let mut envelope: Envelope<MutationDataDto> = Envelope::failure(
        Command::TicketClose.name(),
        Scope {
            repository: Some(data.path().to_string_lossy().into_owned()),
            ..Scope::default()
        },
        ResultCode::RemoteBranchDeleted,
        standing_recovery(ResultCode::RemoteBranchDeleted, &accepted()),
    );
    envelope.operation_id = Some(OperationId::new().to_string());
    let offered = service.offering(envelope, outcome::CallKind::Execute);
    assert!(
        offered
            .recovery
            .iter()
            .all(|action| action.action != Kind::RequestCancel)
    );
}

/// On a re-entry a transient code with no parent leaves the record.
#[test]
fn a_transient_stop_entered_again_with_no_parent_leaves_the_record() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let binding = LifecycleBinding::<Fixed> {
        root: PathBuf::from("/nowhere"),
        id: ITEM.parse().unwrap(),
        expected: None,
        closure_exists: false,
        verb: PhantomData,
    };
    let again = binding.stopped(&service, &accepted(), Stop::Busy, true);
    assert!(matches!(again.standing, Standing::Owed));
    let first = binding.stopped(&service, &accepted(), Stop::Busy, false);
    assert!(matches!(
        first.standing,
        Standing::Stopped {
            journal: Some(JournalRow::Absent)
        }
    ));
}

/// What a run's observer is told, as the progress stages a request reports
/// (D5 decision 11): a session that only records the stages sent.
struct Recorder(std::cell::RefCell<Vec<ProgressStage>>);

impl Session for Recorder {
    fn cancelled(&self) -> bool {
        false
    }

    fn report(&self, event: &crate::repository::ProgressEvent) {
        assert_eq!(event.request_id, "request");
        self.0.borrow_mut().push(event.stage);
    }

    fn retire_context(
        &self,
        _: &RepositoryService,
        _: crate::repository::RetireContextRequest,
        _: &Accepted,
        _: bool,
        _: Option<crate::repository::context_retirement::handoff::InitialInput>,
    ) -> Option<
        Result<crate::repository::ContextRetirementOutcome, crate::repository::RepositoryError>,
    > {
        None
    }

    fn synchronize(
        &self,
        _: &RepositoryService,
        _: crate::repository::SynchronizeRemoteRequest,
        _: crate::repository::SynchronizationSettings,
        _: &mut dyn crate::repository::SynchronizationObserver,
    ) -> Option<
        Result<crate::repository::SynchronizationResult, crate::repository::SynchronizationError>,
    > {
        None
    }

    fn republish(
        &self,
        _: &RepositoryService,
        _: crate::repository::SynchronizeRemoteRequest,
        _: &crate::repository::remote::sync::RepublishConsent,
        _: &mut dyn crate::repository::SynchronizationObserver,
    ) -> Option<
        Result<crate::repository::SynchronizationResult, crate::repository::SynchronizationError>,
    > {
        None
    }

    fn promote_document(
        &self,
        _: &RepositoryService,
        _: crate::repository::PromoteDocumentRequest,
        _: RequestId,
        _: &mut dyn RunObserver,
    ) -> Option<
        Result<
            crate::repository::DocumentPromotionResult,
            crate::repository::DocumentPromotionError,
        >,
    > {
        None
    }

    fn close_ticket(
        &self,
        _: &RepositoryService,
        _: crate::repository::CloseTicketRequest,
        _: RequestId,
        _: &mut dyn RunObserver,
    ) -> Option<Result<crate::repository::TicketClosureResult, crate::repository::TicketClosureError>>
    {
        None
    }

    fn withdraw_closure(
        &self,
        _: &RepositoryService,
        _: crate::repository::CloseTicketRequest,
        _: RequestId,
        _: &mut dyn RunObserver,
    ) -> Option<Result<crate::repository::TicketClosureResult, crate::repository::TicketClosureError>>
    {
        None
    }

    fn poll_once(
        &self,
        _: &RepositoryService,
        _: crate::repository::PollPublicationRemoteRequest,
        _: RequestId,
    ) -> Option<Result<crate::repository::RemotePollResult, crate::repository::RemotePollError>>
    {
        None
    }

    fn poll_once_retiring(
        &self,
        _: &RepositoryService,
        _: crate::repository::PollPublicationRemoteRequest,
        _: &Accepted,
    ) -> Option<
        Result<
            (
                crate::repository::RemotePollResult,
                Option<crate::repository::context_retirement::handoff::Live>,
            ),
            crate::repository::RemotePollError,
        >,
    > {
        None
    }

    fn verify_transport(
        &self,
        _: &RepositoryService,
        _: crate::repository::transport::VerifySshTransportRequest,
    ) -> Option<
        Result<
            crate::repository::transport::SshTransportVerified,
            crate::repository::transport::SshTransportError,
        >,
    > {
        None
    }

    fn verify_setup_transport(
        &self,
        _: &RepositoryService,
        _: crate::repository::transport::VerifySshTransportRequest,
        _: &crate::repository::transport::SetupScope,
    ) -> Option<
        Result<
            crate::repository::transport::SshTransportVerified,
            crate::repository::transport::SshTransportError,
        >,
    > {
        None
    }

    fn publish_setup(
        &self,
        _: &RepositoryService,
        _: &std::path::Path,
        _: crate::repository::RequestId,
    ) -> Option<
        Result<
            crate::repository::setup::publish::Published,
            crate::repository::setup::publish::SetupError,
        >,
    > {
        None
    }

    fn poll_setup(
        &self,
        _: &RepositoryService,
        _: &std::path::Path,
        _: crate::repository::RequestId,
    ) -> Option<Result<crate::repository::SetupPollReport, crate::repository::SetupPollError>> {
        None
    }

    fn cancel_setup(
        &self,
        _: &RepositoryService,
        _: &crate::repository::CancelSetupRequest,
    ) -> Option<Result<crate::repository::SetupCancelReport, crate::repository::SetupCancelError>>
    {
        None
    }
}

/// D5 decision 11, L3, R-15: each step a run tells of is a stage. The
/// context's synchronization reports `connecting`, then its fetch, local
/// update and push as a share's do (U-4), and none of its other safe
/// points; the primary's reports `publishing` once and none of its safe
/// points; a closure the ticket already holds is not recorded again.
#[test]
fn a_runs_steps_are_reported_as_stages() {
    use ChildRole::{Context, Primary};
    use ProgressStage as S;
    use RemoteOperationSafePoint as Point;
    use RunEvent as E;
    let stages_of = |closure_exists: bool, events: &[RunEvent]| {
        let session = Recorder(Default::default());
        let mut stages = Stages {
            session: &session,
            subject: Subject::for_testing("request", "ticket close"),
            closure_exists,
        };
        for event in events {
            stages.event(*event);
        }
        session.0.into_inner()
    };
    assert_eq!(
        stages_of(
            false,
            &[
                E::ChildStarting(Context),
                E::ChildSafePoint(Context, Point::BeforeFetch),
                E::ChildSafePoint(Context, Point::AfterFetch),
                E::ChildSafePoint(Context, Point::BeforeLocalUpdate),
                E::ChildSafePoint(Context, Point::BeforePush),
                E::ChildSafePoint(Context, Point::AfterPushReturn),
                E::Recording,
                E::Integrating,
                E::ChildStarting(Primary),
                E::ChildSafePoint(Primary, Point::BeforeFetch),
                E::ChildSafePoint(Primary, Point::BeforePush),
                E::CleaningUp,
            ]
        ),
        [
            S::Connecting,
            S::Fetching,
            S::UpdatingLocal,
            S::Pushing,
            S::Recording,
            S::Integrating,
            S::Publishing,
            S::CleaningUp,
        ]
    );
    // R-15: a closure the ticket already holds is not recorded again.
    assert_eq!(
        stages_of(true, &[E::Recording, E::Integrating, E::CleaningUp]),
        [S::Integrating, S::CleaningUp]
    );
}

/// A withdraw is a closure's twin over the same record, and every site that
/// reads a record's command for a lifecycle parent reaches it: whether a
/// child is the parent's (`held_by_its_child`), what an unsettled request
/// shows (`recorded_effects`), and what `request cancel` asks (`asks_a_parent`
/// and `cancel_parent`, which set the record's own flag).
#[test]
fn a_withdraw_is_a_lifecycle_parent_wherever_its_command_is_read() {
    use crate::repository::{
        mutation::identity::ScopeKey,
        request_store::{DigestSalt, IntentDigest, NewRequest, RequestOperation},
    };
    let registered = Registered::new(false);
    let commit = a_commit(&registered);
    let repository = git2::Repository::open(registered.root.path()).unwrap();
    // The branch that holds the withdraw's commit.
    repository
        .reference(
            &format!("refs/heads/manyhands/ticket/{ITEM}"),
            commit,
            true,
            "",
        )
        .unwrap();
    let scope = ScopeKey::for_repository(registered.root.path()).unwrap();
    let (operation, child, request) = (OperationId::new(), OperationId::new(), RequestId::new());
    let db = rusqlite::Connection::open(
        registered
            ._data
            .path()
            .join(crate::repository::REGISTRY_FILE),
    )
    .unwrap();
    db.execute(
        "INSERT INTO ticket_closures
           (repository_id, operation_id, item_id, expectation, step, updated_at,
            closure_kind, integration_mode, action, withdraw_oid, base_oid, context_oid,
            context_attempt, context_sync_id, remote_name)
         SELECT id, ?1, ?2, zeroblob(32), 'closing', 0,
                'existing', 'pull_request', 'withdraw', ?3, ?3, ?3, 1, ?4, 'origin'
           FROM repositories",
        rusqlite::params![
            operation.to_string(),
            ITEM,
            commit.to_string(),
            child.to_string()
        ],
    )
    .unwrap();
    registered
        .service
        .insert_request(&NewRequest {
            request_id: request,
            scope: scope.clone(),
            command: Command::TicketWithdrawClosure.name().into(),
            target: String::new(),
            intent_digest: IntentDigest::builder(
                DigestSalt::Request(request),
                Command::TicketWithdrawClosure.name(),
                &scope,
                "",
            )
            .finish(),
            confirmation: None,
            base_ref: None,
            base_oid: None,
            expected_digest: None,
            operations: vec![RequestOperation {
                family: crate::repository::OperationFamily::Remote,
                operation_id: operation,
            }],
        })
        .unwrap();

    // The child is the parent's, whichever verb's command asks.
    for command in [Command::TicketClose, Command::TicketWithdrawClosure] {
        assert!(
            held_by_its_child(
                &registered.service,
                Some(command),
                registered.root.path(),
                operation,
                child
            ),
            "{command:?}"
        );
    }
    assert!(!held_by_its_child(
        &registered.service,
        Some(Command::TicketWithdrawClosure),
        registered.root.path(),
        operation,
        OperationId::new()
    ));

    // What an unsettled request shows is what its parent wrote.
    let record = registered.service.request_record(request).unwrap().unwrap();
    let shown = crate::repository::mutation::bind::recorded_effects(&registered.service, &record);
    assert_eq!(shown.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(shown.effects.write, crate::results::WriteEffect::Written);

    // `request cancel` asks the parent as well as the request.
    let cancelled = registered.service.cancel_request(request);
    assert_eq!(cancelled.data.map(|data| data.requested), Some(true));
    let asked: i64 = db
        .query_row(
            "SELECT cancel_requested FROM ticket_closures WHERE operation_id = ?1",
            [operation.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(asked, 1, "the parent's own flag");
}
