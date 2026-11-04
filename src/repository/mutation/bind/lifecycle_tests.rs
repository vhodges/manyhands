use super::super::super::{identity::RequestId, progress::Session};
use super::*;
use crate::repository::{ItemDtoKind, RepositoryError, RepositoryErrorKind, RepositoryOperation};

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
    ) -> Option<Result<Reached, Stop>> {
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
