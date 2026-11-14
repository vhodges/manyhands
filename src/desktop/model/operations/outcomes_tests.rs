//! A share's words, and the possible holder of a repository a share was
//! answered `busy` for (design, Part 0, Busy repositories), over requests
//! put in the model through `Operations::apply`.

use std::{collections::HashMap, path::PathBuf};

use manyhands::{
    repository::{AuthoringKind, RequestId},
    results::{Effects, Envelope, ResultCode, Scope},
};

use super::{ANOTHER_SHARE_HOLDS, may_be_waiting_for, sync_text};
use crate::desktop::{
    model::{operations::Operations, publication::SyncOutcome},
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{RecordState, WorkerEvent},
    },
    test_support::fixtures::item_id,
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";

fn root(name: &str) -> PathBuf {
    PathBuf::from(format!("/fixture/{name}"))
}

fn repo_sync(name: &str) -> Intent {
    Intent::RepoSync { root: root(name) }
}

fn item_sync(name: &str) -> Intent {
    Intent::ItemSync {
        root: root(name),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    }
}

/// Queues the request `n` of `intent` and, with `code`, finishes it with
/// that answer; with none it is left running.
fn put(operations: &mut Operations, n: u64, intent: Intent, code: Option<ResultCode>) {
    let id = DesktopRequestId(n);
    operations.apply(WorkerEvent::Queued(DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    }));
    operations.apply(WorkerEvent::Started(id));
    if let Some(code) = code {
        let envelope = Envelope::classified_mutation(
            "item sync",
            Scope::default(),
            false,
            code,
            Effects::not_requested(),
            false,
        );
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
    }
}

fn records(
    states: &[(u64, RecordState)],
) -> impl Fn(DesktopRequestId) -> Option<RecordState> + use<> {
    let states: HashMap<DesktopRequestId, RecordState> = states
        .iter()
        .map(|(n, state)| (DesktopRequestId(*n), *state))
        .collect();
    move |id| states.get(&id).copied()
}

/// A share answered `busy` may be waiting for a request of this session
/// on the same repository that stopped, its record still accepted, with
/// a code given after reserving: the newest such one is named.
#[test]
fn busy_may_name_an_accepted_holder_of_this_session() {
    let mut operations = Operations::default();
    put(
        &mut operations,
        1,
        repo_sync("a"),
        Some(ResultCode::KeyRejected),
    );
    put(
        &mut operations,
        2,
        repo_sync("a"),
        Some(ResultCode::UnlockRequired),
    );
    put(&mut operations, 3, item_sync("a"), Some(ResultCode::Busy));
    let record = records(&[(1, RecordState::Accepted), (2, RecordState::Accepted)]);
    assert_eq!(
        operations.busy_holder(DesktopRequestId(3), record),
        Some(DesktopRequestId(2)),
        "the newest holder"
    );
    assert_eq!(
        sync_text(&SyncOutcome::Busy, Some("Share main")).as_deref(),
        Some(may_be_waiting_for("Share main").as_str())
    );
    assert_eq!(
        may_be_waiting_for("Share main"),
        "May be waiting for Share main"
    );
}

/// A request holds no reservation when its record is finished or not
/// known, when its answer came before reserving, when it is still
/// running, or when it is of another repository or a command that does
/// not stop at safe points: none is named, and the words say another
/// share may hold the repository.
#[test]
fn busy_ignores_requests_that_hold_no_reservation() {
    let mut operations = Operations::default();
    put(
        &mut operations,
        1,
        repo_sync("a"),
        Some(ResultCode::UnlockRequired),
    );
    put(
        &mut operations,
        2,
        repo_sync("a"),
        Some(ResultCode::KeyRejected),
    );
    put(
        &mut operations,
        3,
        item_sync("a"),
        Some(ResultCode::ItemNotFound),
    );
    put(
        &mut operations,
        4,
        repo_sync("b"),
        Some(ResultCode::UnlockRequired),
    );
    put(
        &mut operations,
        5,
        Intent::IndexRefresh { root: root("a") },
        Some(ResultCode::UnlockRequired),
    );
    put(&mut operations, 6, repo_sync("a"), None);
    put(&mut operations, 7, item_sync("a"), Some(ResultCode::Busy));
    let record = records(&[
        (1, RecordState::Finished),
        (2, RecordState::Unknown),
        (3, RecordState::Accepted),
        (4, RecordState::Accepted),
        (6, RecordState::Accepted),
    ]);
    assert_eq!(operations.busy_holder(DesktopRequestId(7), record), None);
    assert_eq!(
        sync_text(&SyncOutcome::Busy, None).as_deref(),
        Some(ANOTHER_SHARE_HOLDS)
    );
}
