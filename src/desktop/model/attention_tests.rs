//! The count is one table (P20): each source, and each state that is not
//! counted. The draft registries are built once, with the fixtures the
//! drafts' own tests use.

use std::{path::PathBuf, sync::Arc};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    drafts::{
        DraftError, DraftErrorKind, DraftFields, DraftTarget, FailureAction, FailurePoint, ItemKind,
    },
    repository::{
        OperationAction, OperationDto, OperationFamily, OperationListDto, OperationOwner, RequestId,
    },
    results::{Envelope, Outcome, ResultCode, Scope},
};

use super::needs_attention;
use crate::desktop::{
    model::operations::Operations,
    services::{
        ReadFailure, ReadResult, Services,
        drafts::DraftRegistry,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{NotSent, WorkerEvent},
    },
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

fn queued(id: u64) -> WorkerEvent {
    WorkerEvent::Queued(DesktopRequest {
        id: DesktopRequestId(id),
        request_id: RequestId::new(),
        intent: Intent::IndexRefresh {
            root: PathBuf::from("/fixture/a"),
        },
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    })
}

/// A request that finished with `outcome`, under a code that agrees with it.
fn finished(operations: &mut Operations, id: u64, outcome: Outcome) {
    let code = match outcome {
        Outcome::Success => ResultCode::Ok,
        Outcome::Noop => ResultCode::AlreadyApplied,
        Outcome::Cancelled => ResultCode::Cancelled,
        Outcome::Partial | Outcome::Blocked | Outcome::Error => ResultCode::Busy,
    };
    operations.apply(queued(id));
    operations.apply(WorkerEvent::Finished(
        DesktopRequestId(id),
        Box::new(Envelope::mutation(
            "index refresh",
            Scope::default(),
            outcome,
            code,
        )),
    ));
}

fn row() -> OperationDto {
    OperationDto {
        operation_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned()),
        family: OperationFamily::Local,
        owner: OperationOwner::Repository,
        action: OperationAction::SaveTicket,
        state: "pending".to_owned(),
        completed_step: None,
        next_action: None,
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

/// `root`'s pending read, answered with `result`.
fn pending(operations: &mut Operations, root: &str, result: ReadResult<OperationListDto>) {
    let stamp = operations.load_pending(root.to_owned());
    assert!(operations.apply_pending(stamp, result));
}

fn rows(count: usize) -> ReadResult<OperationListDto> {
    ReadResult::Ok(OperationListDto {
        items: vec![row(); count],
        complete: true,
    })
}

fn failed() -> ReadResult<OperationListDto> {
    // A code the model keeps as a failed list: an unreadable index.
    ReadResult::Failed(ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: vec![],
    })
}

fn ticket_fields() -> DraftFields {
    DraftFields::Ticket {
        title: "Draft title".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: "Draft body".to_owned(),
    }
}

/// A registry with `sessions` drafts whose last write failed.
async fn failing(sessions: usize, cx: &mut TestAppContext) -> (Fixture, Entity<DraftRegistry>) {
    let fixture = Fixture::new();
    let services = cx.update(|cx| Services::start(fixture.delaying_backend(), cx));
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    for _ in 0..sessions {
        let target = DraftTarget {
            repository: "/a/repository".to_owned(),
            item_kind: ItemKind::Ticket,
            item_id: "01ARZ3NDEKTSV4RRFFQ69G5FG0".parse().unwrap(),
            comment: None,
        };
        let begun = drafts.update(cx, |registry, cx| {
            registry.begin(target, None, ticket_fields(), cx)
        });
        cx.run_until_parked();
        let id = begun.await.expect("the draft begins");
        drafts.update(cx, |registry, cx| registry.edit(id, ticket_fields(), cx));
    }
    let _failure = fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    let all = drafts.update(cx, |registry, cx| registry.flush_all(cx));
    cx.run_until_parked();
    assert_eq!(all.await.len(), sessions);
    (fixture, drafts)
}

/// A registry whose store did not open.
fn unavailable(cx: &mut TestAppContext) -> (Fixture, Entity<DraftRegistry>) {
    let fixture = Fixture::new();
    let backend = Arc::new(DelayingBackend::new(
        Arc::new(manyhands::repository::RepositoryService::open_at(fixture.data_dir()).unwrap()),
        fixture.home().to_owned(),
        Err(DraftError {
            kind: DraftErrorKind::ProtectionUnavailable,
        }),
    ));
    let services = cx.update(|cx| Services::start(backend, cx));
    cx.run_until_parked();
    (fixture, services.drafts().clone())
}

/// Decisions 3 and 14: what awaits the person is counted from the models
/// as they are; what is dismissed, cancelled, never sent or still being
/// read is not.
#[gpui_kit::test]
async fn counts_what_awaits_the_person(cx: &mut TestAppContext) {
    let (_healthy_fixture, healthy) = failing(0, cx).await;
    let (_failing_fixture, failures) = failing(2, cx).await;
    let (_unavailable_fixture, unavailable) = unavailable(cx);
    healthy.read_with(cx, |registry, _| {
        assert!(registry.flush_failures().is_empty() && registry.unavailable().is_none());
    });
    failures.read_with(cx, |registry, _| {
        assert_eq!(registry.flush_failures().len(), 2)
    });
    unavailable.read_with(cx, |registry, _| assert!(registry.unavailable().is_some()));

    struct Case {
        what: &'static str,
        operations: Operations,
        drafts: Entity<DraftRegistry>,
        expected: usize,
    }
    let mut cases = Vec::new();
    let mut case =
        |what, expected, drafts: &Entity<DraftRegistry>, fill: &dyn Fn(&mut Operations)| {
            let mut operations = Operations::default();
            fill(&mut operations);
            cases.push(Case {
                what,
                operations,
                drafts: drafts.clone(),
                expected,
            });
        };
    case("nothing", 0, &healthy, &|_| {});
    for (what, outcome, expected) in [
        ("finished: success", Outcome::Success, 0),
        ("finished: noop", Outcome::Noop, 0),
        ("finished: partial", Outcome::Partial, 1),
        ("finished: blocked", Outcome::Blocked, 1),
        ("finished: cancelled", Outcome::Cancelled, 0),
        ("finished: error", Outcome::Error, 1),
    ] {
        case(what, expected, &healthy, &|operations| {
            finished(operations, 1, outcome)
        });
    }
    case("dismissed", 0, &healthy, &|operations| {
        finished(operations, 1, Outcome::Error);
        operations.dismiss(DesktopRequestId(1));
    });
    case("not sent", 0, &healthy, &|operations| {
        operations.apply(queued(1));
        operations.apply(WorkerEvent::NotSent(
            DesktopRequestId(1),
            NotSent::SecretRequired,
        ));
    });
    case("queued and running", 0, &healthy, &|operations| {
        operations.apply(queued(1));
        operations.apply(queued(2));
        operations.apply(WorkerEvent::Started(DesktopRequestId(2)));
    });
    case("flush failures, one each", 2, &failures, &|_| {});
    case("drafts unavailable", 1, &unavailable, &|_| {});
    case("pending rows, one each", 3, &healthy, &|operations| {
        pending(operations, "/fixture/a", rows(2));
        pending(operations, "/fixture/b", rows(1));
    });
    case("pending, none listed", 0, &healthy, &|operations| {
        pending(operations, "/fixture/a", rows(0));
    });
    case(
        "pending read failed, one each",
        2,
        &healthy,
        &|operations| {
            pending(operations, "/fixture/a", failed());
            pending(operations, "/fixture/b", failed());
        },
    );
    case("pending read busy", 0, &healthy, &|operations| {
        pending(operations, "/fixture/a", ReadResult::Busy);
    });
    case("pending read loading", 0, &healthy, &|operations| {
        operations.load_pending("/fixture/a".to_owned());
    });
    case("the sum", 1 + 2 + 2 + 1, &failures, &|operations| {
        finished(operations, 1, Outcome::Error);
        finished(operations, 2, Outcome::Success);
        operations.apply(queued(3));
        operations.apply(WorkerEvent::NotSent(
            DesktopRequestId(3),
            NotSent::KeyStoreUnavailable,
        ));
        pending(operations, "/fixture/a", rows(2));
        pending(operations, "/fixture/b", failed());
        pending(operations, "/fixture/c", ReadResult::Busy);
    });

    for case in cases {
        let counted = case.drafts.read_with(cx, |registry, _| {
            needs_attention(&case.operations, registry)
        });
        assert_eq!(counted, case.expected, "{}", case.what);
    }
}
