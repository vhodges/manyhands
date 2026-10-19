use std::path::PathBuf;

use manyhands::{
    repository::{
        OperationAction, OperationDto, OperationFamily, OperationListDto, OperationNextAction,
        OperationOwner, ProgressEvent, ProgressStage, RequestId,
    },
    results::{Effects, Envelope, RecoveryActionKind, ResultCode, Scope},
};

use super::{
    Operations, RecoveryControl, ResumeOffer, SYNCHRONIZATION_UNAVAILABLE, abandon_offered,
    recovery_control, recovery_label, resume_offer,
};
use crate::desktop::{
    model::list_state::LABEL_EMPTY,
    services::{
        ReadFailure, ReadResult,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{NotSent, WorkerEvent},
    },
};

fn request(id: u64) -> DesktopRequest {
    DesktopRequest {
        id: DesktopRequestId(id),
        request_id: RequestId::new(),
        intent: Intent::IndexRefresh {
            root: PathBuf::from("/fixture/a"),
        },
        confirmation: None,
        lane: Lane::User,
        state: RequestState::Queued,
    }
}

fn progress(request_id: RequestId, stage: ProgressStage) -> ProgressEvent {
    ProgressEvent {
        request_id: request_id.to_string(),
        operation_id: None,
        command: "index refresh".to_owned(),
        scope: Scope::default(),
        stage,
        effects: Effects::not_requested(),
    }
}

fn ids(operations: &Operations) -> Vec<u64> {
    operations
        .requests()
        .iter()
        .map(|request| request.id.0)
        .collect()
}

#[test]
fn requests_are_listed_newest_first() {
    let mut operations = Operations::default();
    operations.apply(WorkerEvent::Queued(request(1)));
    operations.apply(WorkerEvent::Queued(request(2)));
    assert_eq!(ids(&operations), [2, 1]);
    assert!(matches!(
        operations.request(DesktopRequestId(1)).unwrap().state,
        RequestState::Queued
    ));
}

#[test]
fn a_request_runs_is_accepted_and_finishes() {
    let mut operations = Operations::default();
    let queued = request(1);
    let request_id = queued.request_id;
    let id = queued.id;
    operations.apply(WorkerEvent::Queued(queued));
    operations.apply(WorkerEvent::Started(id));
    assert!(matches!(
        operations.request(id).unwrap().state,
        RequestState::Running {
            stage: None,
            accepted: false
        }
    ));

    operations.apply(WorkerEvent::Progress(
        id,
        progress(request_id, ProgressStage::Accepted),
    ));
    operations.apply(WorkerEvent::Progress(
        id,
        progress(request_id, ProgressStage::Indexing),
    ));
    assert!(matches!(
        operations.request(id).unwrap().state,
        RequestState::Running {
            stage: Some(ProgressStage::Indexing),
            accepted: true
        }
    ));

    let envelope = Envelope::failure("index refresh", Scope::default(), ResultCode::Busy, vec![]);
    let message = envelope.message.clone();
    let expected = envelope.outcome;
    operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
    let RequestState::Finished(outcome, code, _, recovery, shown) =
        &operations.request(id).unwrap().state
    else {
        panic!("finished");
    };
    assert_eq!(*outcome, expected);
    assert_eq!(*code, ResultCode::Busy);
    assert!(recovery.is_empty());
    assert_eq!(*shown, message);
}

#[test]
fn a_retry_keeps_the_request_and_its_place() {
    let mut operations = Operations::default();
    let first = request(1);
    let request_id = first.request_id;
    operations.apply(WorkerEvent::Queued(first));
    operations.apply(WorkerEvent::Queued(request(2)));
    operations.apply(WorkerEvent::Started(DesktopRequestId(1)));
    let envelope = Envelope::failure("index refresh", Scope::default(), ResultCode::Busy, vec![]);
    operations.apply(WorkerEvent::Finished(
        DesktopRequestId(1),
        Box::new(envelope),
    ));

    let mut retried = request(1);
    retried.request_id = request_id;
    operations.apply(WorkerEvent::Queued(retried));
    assert_eq!(ids(&operations), [2, 1]);
    let request = operations.request(DesktopRequestId(1)).unwrap();
    assert_eq!(request.request_id, request_id);
    assert!(matches!(request.state, RequestState::Queued));
}

#[test]
fn removed_and_unsent_requests_say_so() {
    let mut operations = Operations::default();
    operations.apply(WorkerEvent::Queued(request(1)));
    operations.apply(WorkerEvent::Queued(request(2)));
    operations.apply(WorkerEvent::Removed(DesktopRequestId(1)));
    operations.apply(WorkerEvent::NotSent(
        DesktopRequestId(2),
        NotSent::SecretRequired,
    ));
    assert!(matches!(
        operations.request(DesktopRequestId(1)).unwrap().state,
        RequestState::Removed
    ));
    assert!(matches!(
        operations.request(DesktopRequestId(2)).unwrap().state,
        RequestState::NotSent(NotSent::SecretRequired)
    ));
}

#[test]
fn an_event_for_an_unknown_request_changes_nothing() {
    let mut operations = Operations::default();
    operations.apply(WorkerEvent::Started(DesktopRequestId(7)));
    assert!(operations.requests().is_empty());
}

#[test]
fn a_finished_request_can_be_dismissed() {
    let mut operations = Operations::default();
    operations.apply(WorkerEvent::Queued(request(1)));
    let envelope = Envelope::failure("index refresh", Scope::default(), ResultCode::Busy, vec![]);
    operations.apply(WorkerEvent::Finished(
        DesktopRequestId(1),
        Box::new(envelope),
    ));
    operations.dismiss(DesktopRequestId(1));
    assert!(matches!(
        operations.request(DesktopRequestId(1)).unwrap().state,
        RequestState::Dismissed
    ));

    // A running request is not dismissed.
    operations.apply(WorkerEvent::Queued(request(2)));
    operations.apply(WorkerEvent::Started(DesktopRequestId(2)));
    operations.dismiss(DesktopRequestId(2));
    assert!(matches!(
        operations.request(DesktopRequestId(2)).unwrap().state,
        RequestState::Running { .. }
    ));
}

#[test]
fn a_removed_request_is_not_a_dismissed_one_and_can_be_dismissed() {
    let mut operations = Operations::default();
    operations.apply(WorkerEvent::Queued(request(1)));
    operations.apply(WorkerEvent::Removed(DesktopRequestId(1)));
    assert!(matches!(
        operations.request(DesktopRequestId(1)).unwrap().state,
        RequestState::Removed
    ));
    operations.dismiss(DesktopRequestId(1));
    assert!(matches!(
        operations.request(DesktopRequestId(1)).unwrap().state,
        RequestState::Dismissed
    ));
}

/// A finished request says its repository may have changed, once; a
/// request that never ran says nothing.
#[test]
fn a_finished_request_marks_its_repository_changed() {
    let mut operations = Operations::default();
    operations.apply(WorkerEvent::Queued(request(1)));
    operations.apply(WorkerEvent::Queued(request(2)));
    operations.apply(WorkerEvent::Removed(DesktopRequestId(2)));
    assert!(operations.take_changed().is_empty());
    let envelope = Envelope::failure("index refresh", Scope::default(), ResultCode::Busy, vec![]);
    operations.apply(WorkerEvent::Finished(
        DesktopRequestId(1),
        Box::new(envelope),
    ));
    assert_eq!(
        operations.take_changed().into_iter().collect::<Vec<_>>(),
        ["/fixture/a".to_owned()]
    );
    assert!(operations.take_changed().is_empty());
    operations.mark_changed("/fixture/b".to_owned());
    assert_eq!(
        operations.take_changed().into_iter().collect::<Vec<_>>(),
        ["/fixture/b".to_owned()]
    );
}

fn operation(
    family: OperationFamily,
    action: OperationAction,
    next: Option<OperationNextAction>,
) -> OperationDto {
    OperationDto {
        operation_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned()),
        family,
        owner: OperationOwner::Repository,
        action,
        state: "pending".to_owned(),
        completed_step: None,
        next_action: next,
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

fn list(items: Vec<OperationDto>) -> OperationListDto {
    OperationListDto {
        items,
        complete: true,
    }
}

/// Each repository's pending operations load under their own stamp; a
/// reload's earlier read is dropped, and a repository that is no longer
/// listed is let go.
#[test]
fn pending_operations_load_per_repository_and_drop_stale_reads() {
    let mut operations = Operations::default();
    let first = operations.load_pending("/fixture/a".to_owned());
    let second = operations.load_pending("/fixture/a".to_owned());
    let other = operations.load_pending("/fixture/b".to_owned());
    let row = operation(
        OperationFamily::Local,
        OperationAction::SaveTicket,
        Some(OperationNextAction::Resume),
    );
    assert!(!operations.apply_pending(first, ReadResult::Ok(list(vec![row.clone()]))));
    assert!(operations.apply_pending(second, ReadResult::Ok(list(vec![row]))));
    assert!(operations.apply_pending(other, ReadResult::Ok(list(vec![]))));
    assert_eq!(
        operations.pending()["/fixture/a"]
            .rows()
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(operations.pending()["/fixture/b"].label(), LABEL_EMPTY);

    // A repository that cannot be found is not resolved, and lists nothing.
    let gone = operations.load_pending("/fixture/b".to_owned());
    assert!(operations.apply_pending(
        gone,
        ReadResult::Failed(ReadFailure {
            code: ResultCode::InvalidPath,
            recovery: vec![],
        })
    ));
    assert!(!operations.pending().contains_key("/fixture/b"));

    operations.retain_pending(&[]);
    assert!(operations.pending().is_empty());
}

/// Resume is offered for a local operation waiting to be resumed; a
/// synchronization waiting for it shows Resume unavailable, with why.
/// No local recipe leaves a pending synchronization, so the row is built
/// here; production code never builds one.
#[test]
fn resume_is_offered_for_a_local_operation_and_not_for_a_synchronization() {
    let local = operation(
        OperationFamily::Local,
        OperationAction::SaveTicket,
        Some(OperationNextAction::Resume),
    );
    assert_eq!(resume_offer(&local), ResumeOffer::Offered);
    assert!(abandon_offered(&local));

    for (family, action) in [
        (OperationFamily::Remote, OperationAction::SynchronizeContext),
        (OperationFamily::Remote, OperationAction::SynchronizePrimary),
        (OperationFamily::Local, OperationAction::SynchronizeContext),
    ] {
        let synchronization = operation(family, action, Some(OperationNextAction::Resume));
        assert_eq!(
            resume_offer(&synchronization),
            ResumeOffer::Unavailable(SYNCHRONIZATION_UNAVAILABLE)
        );
        assert_eq!(
            SYNCHRONIZATION_UNAVAILABLE,
            "Synchronization is not available yet"
        );
        assert!(!abandon_offered(&synchronization));
    }

    let waiting = operation(OperationFamily::Local, OperationAction::SaveTicket, None);
    assert_eq!(resume_offer(&waiting), ResumeOffer::NotOffered);
    let key = operation(
        OperationFamily::KeyMaterial,
        OperationAction::GenerateKey,
        Some(OperationNextAction::RetryGeneration),
    );
    assert_eq!(resume_offer(&key), ResumeOffer::NotOffered);
    assert!(!abandon_offered(&key));
}

/// Every recovery action maps to a control: the ones D1 offers run, the
/// rest show as text, unavailable.
#[test]
fn recovery_actions_map_to_controls() {
    use RecoveryActionKind as Kind;
    let expected = [
        (Kind::IndexRefresh, RecoveryControl::IndexRefresh),
        (Kind::IndexRebuild, RecoveryControl::IndexRebuild),
        (Kind::RequestRetry, RecoveryControl::Retry),
        (Kind::RequestPrepare, RecoveryControl::Prepare),
        (Kind::RequestCancel, RecoveryControl::Cancel),
        (Kind::OperationResume, RecoveryControl::Resume),
        (Kind::OperationAbandon, RecoveryControl::Abandon),
        (Kind::RepoIdentitySet, RecoveryControl::Unavailable),
        (Kind::HostApprove, RecoveryControl::Unavailable),
        (Kind::HostReplace, RecoveryControl::Unavailable),
        (Kind::RepoInspect, RecoveryControl::Unavailable),
    ];
    assert_eq!(expected.len(), Kind::ALL.len());
    for (kind, control) in expected {
        assert_eq!(recovery_control(kind), control, "{}", kind.as_str());
        assert!(!recovery_label(kind).is_empty());
    }
}
