use std::path::PathBuf;

use manyhands::{
    repository::{ProgressEvent, ProgressStage, RequestId},
    results::{Effects, Envelope, ResultCode, Scope},
};

use super::Operations;
use crate::desktop::services::{
    requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
    worker::{NotSent, WorkerEvent},
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
        RequestState::Dismissed
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
