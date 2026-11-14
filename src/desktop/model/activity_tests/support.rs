//! Fixtures for Activity's model tests: the worker's events put through a
//! real `Operations`, and the inputs a Workspace would hand `request_rows`.

use std::path::PathBuf;

use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, MutationDataDto, ProgressEvent, ProgressStage, RequestId,
        transport::SshAuthority,
    },
    results::{Effects, Envelope, Outcome, ResultCode, Scope},
};

use crate::desktop::{
    model::{
        activity::{
            requests::{RequestFacts, RequestInputs, request_rows},
            rows::Derived,
        },
        operations::{Operations, StopOffer},
        stamp::RepoKey,
    },
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{NotSent, WorkerEvent},
    },
};

pub(super) const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
pub(super) const ROOT: &str = "/fixture/demo";
/// The main branch the fixtures' repository names.
pub(super) const PRIMARY: &str = "trunk";
/// The title the item has where a row may read it.
pub(super) const TITLE: &str = "Plan";
/// A draft's own title, which no row may show (D2; decision 21).
pub(super) const DRAFT_TITLE: &str = "Secret draft title";

pub(super) fn root() -> PathBuf {
    PathBuf::from(ROOT)
}

pub(super) fn item() -> ItemId {
    ITEM.parse().unwrap()
}

pub(super) fn id(n: u64) -> DesktopRequestId {
    DesktopRequestId(n)
}

/// What the Workspace finds for a request with nothing special about it.
pub(super) fn facts() -> RequestFacts {
    RequestFacts {
        stop_offer: StopOffer::NotOffered,
        shown_offers: Vec::new(),
        share_words: None,
        already_finished: false,
        review: false,
        recoveries: Vec::new(),
        resend_waits: false,
        followed: false,
        follow_void: false,
        check_applied: false,
        superseded: false,
    }
}

pub(super) fn authority() -> SshAuthority {
    SshAuthority {
        host: "git.example.invalid".to_owned(),
        port: 22,
    }
}

/// Queues the request `n` of `intent`.
pub(super) fn queue(operations: &mut Operations, n: u64, intent: Intent) {
    operations.apply(WorkerEvent::Queued(DesktopRequest {
        id: id(n),
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    }));
}

pub(super) fn start(operations: &mut Operations, n: u64) {
    operations.apply(WorkerEvent::Started(id(n)));
}

/// The request `n` reports `stage`, naming `operation` when it has one: it
/// is accepted from then on.
pub(super) fn stage(
    operations: &mut Operations,
    n: u64,
    stage: ProgressStage,
    operation: Option<&str>,
) {
    let request_id = operations.request(id(n)).unwrap().request_id.to_string();
    operations.apply(WorkerEvent::Progress(
        id(n),
        ProgressEvent {
            request_id,
            operation_id: operation.map(str::to_owned),
            command: "fixture".to_owned(),
            scope: Scope::default(),
            stage,
            effects: Effects::not_requested(),
        },
    ));
}

/// An answer of `outcome` with `code` and no data.
pub(super) fn answer(outcome: Outcome, code: ResultCode) -> Envelope<MutationDataDto> {
    Envelope::mutation("fixture", Scope::default(), outcome, code)
}

pub(super) fn finish(operations: &mut Operations, n: u64, envelope: Envelope<MutationDataDto>) {
    operations.apply(WorkerEvent::Finished(id(n), Box::new(envelope)));
}

pub(super) fn not_sent(operations: &mut Operations, n: u64, why: NotSent) {
    operations.apply(WorkerEvent::NotSent(id(n), why));
}

/// The request rows the Workspace would derive: every request's facts from
/// `facts`, the item titled `TITLE` unless `titled` says it has none, and
/// the repository's main branch `PRIMARY`.
pub(super) fn derive(
    operations: &Operations,
    facts: &dyn Fn(DesktopRequestId) -> RequestFacts,
    titled: bool,
) -> Vec<Derived> {
    let title = move |_: &RepoKey, _: AuthoringKind, _: &ItemId| titled.then(|| TITLE.to_owned());
    let primary = |_: &RepoKey| Some(PRIMARY.to_owned());
    request_rows(&RequestInputs {
        operations,
        requests: facts,
        title: &title,
        primary: &primary,
    })
}

/// The row of the request `n`.
pub(super) fn row_of(rows: &[Derived], n: u64) -> &Derived {
    use crate::desktop::model::activity::rows::RowKey;
    rows.iter()
        .find(|row| row.key == RowKey::Request(id(n)))
        .unwrap_or_else(|| panic!("no row for request {n}"))
}
