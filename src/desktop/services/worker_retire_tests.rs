//! The worker's rules for a retirement (D6 Task 11a): one request under one
//! request ID is one (Fact 36), and a graceful quit leaves a running
//! background retirement's token alone (decision 28, R-38). Over the real
//! worker and a held job.

use futures::channel::mpsc::UnboundedReceiver;
use manyhands::repository::{AuthoringKind, RequestId};

use super::{QuitState, WorkerEvent, WorkerHandle};
use crate::desktop::{
    services::{
        JobKind,
        requests::{Intent, Lane},
    },
    test_support::fixtures::Fixture,
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";

fn retire(root: &std::path::Path) -> Intent {
    Intent::ContextRetire {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
    }
}

/// How many retirements the worker announced as queued since the last call.
fn queued_retirements(events: &mut UnboundedReceiver<WorkerEvent>) -> usize {
    let mut queued = 0;
    while let Ok(event) = events.try_recv() {
        if let WorkerEvent::Queued(request) = event
            && matches!(request.intent, Intent::ContextRetire { .. })
        {
            queued += 1;
        }
    }
    queued
}

/// Two retirements sent under one request ID are one request: the second
/// call answers the first's ID and queues nothing, whether the first waits
/// or runs, and whichever lane sent it (Resume's resend is on the user
/// lane, the loop's on the background). Another request ID is another
/// request.
#[test]
fn retire_request_twin() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let backend = fixture.delaying_backend();
    let (worker, mut events) = WorkerHandle::spawn(backend.clone());

    // A refresh holds the worker, so the first retirement waits.
    let refreshing = backend.hold_job(JobKind::IndexRefresh);
    worker.submit(
        Intent::IndexRefresh { root: root.clone() },
        Lane::Background,
        None,
        None,
    );
    refreshing.wait_reached();
    let request = RequestId::new();
    let first = worker.submit_with_request_id(retire(&root), Lane::Background, request);
    let twin = worker.submit_with_request_id(retire(&root), Lane::User, request);
    assert_eq!(twin, first, "a queued twin is the request");
    let other = worker.submit_with_request_id(retire(&root), Lane::Background, RequestId::new());
    assert_ne!(other, first, "another request ID is another request");
    assert_eq!(
        queued_retirements(&mut events),
        2,
        "the twin queued nothing"
    );

    // The first is taken next and held at its start: running, it has a twin
    // as well.
    let running = backend.hold_job(JobKind::ContextRetire);
    refreshing.release();
    running.wait_reached();
    let again = worker.submit_with_request_id(retire(&root), Lane::User, request);
    assert_eq!(again, first, "a running twin is the request");
    assert_eq!(queued_retirements(&mut events), 0);
    running.release();
}

/// Decision 28, R-38: a quit waits for a running background retirement and
/// does not cancel its token, or the item would be left pending and hold
/// the repository's polls at the next launch (Fact 32). Every other job's
/// token is still cancelled, a background refresh's and a user-lane
/// retirement's among them.
#[test]
fn quit_spares_a_retirement_token() {
    let cases = [
        (
            "background retirement",
            JobKind::ContextRetire,
            Lane::Background,
            false,
        ),
        (
            "background refresh",
            JobKind::IndexRefresh,
            Lane::Background,
            true,
        ),
        (
            "user-lane retirement",
            JobKind::ContextRetire,
            Lane::User,
            true,
        ),
    ];
    for (what, kind, lane, cancelled) in cases {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        let backend = fixture.delaying_backend();
        let (worker, _events) = WorkerHandle::spawn(backend.clone());
        let held = backend.hold_job(kind);
        let intent = match kind {
            JobKind::ContextRetire => retire(&root),
            _ => Intent::IndexRefresh { root: root.clone() },
        };
        let job = worker.submit(intent, lane, None, None);
        held.wait_reached();

        assert_eq!(worker.quit(), QuitState::Finishing(job), "{what}");
        let running = worker.running_job().expect("it still runs");
        assert_eq!(running.cancelled, cancelled, "{what}: its token");
        held.release();
    }
}
