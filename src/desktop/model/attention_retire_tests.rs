//! The retirement rows of `counts_what_awaits_the_person` and the rule that
//! dismisses a retirement's request (D6 design E3, R-23, W-1).
//!
//! A retirement is a request in Activity. One answered in an attention or
//! retry row, or retired, is dismissed, whichever lane sent it; refused,
//! preserved, failed and pending ones stay and count. A request and the
//! pending row of its operation count once, and the worker's running
//! operation, a retirement's or an automatic poll's, is no pending row.

use std::{collections::HashSet, path::PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        ContextRetirementState as State, OperationDto, OperationId, OperationListDto,
        ProgressEvent, ProgressStage, RequestId, RetirementRemoval as Removal,
    },
    results::{
        Effects, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode as Code, Scope,
    },
};

use super::{failing, needs_attention, pending, row};
use crate::desktop::{
    model::{operations::Operations, polling::PollSchedule},
    services::{
        ReadResult,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{RecordState, WorkerEvent},
    },
    test_support::retire::{ITEM, OPERATION, answer, answer_of, no_data},
};

/// Another operation, for a pending row that is not the retirement's.
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";

fn retire_intent() -> Intent {
    Intent::ContextRetire {
        root: PathBuf::from("/fixture/a"),
        kind: manyhands::repository::AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
    }
}

fn queue(operations: &mut Operations, id: u64, intent: Intent, lane: Lane) {
    operations.apply(WorkerEvent::Queued(DesktopRequest {
        id: DesktopRequestId(id),
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane,
        attempt: 1,
        state: RequestState::Queued,
    }));
}

/// The first progress event of a request: the stage and the operation it
/// holds.
fn progress(operations: &mut Operations, id: u64, operation: &str) {
    operations.apply(WorkerEvent::Started(DesktopRequestId(id)));
    operations.apply(WorkerEvent::Progress(
        DesktopRequestId(id),
        ProgressEvent {
            request_id: RequestId::new().to_string(),
            operation_id: Some(operation.to_owned()),
            command: "context retire".to_owned(),
            scope: Scope::default(),
            stage: ProgressStage::Observing,
            effects: Effects::not_requested(),
        },
    ));
}

/// A retirement request answered `answer`, the worker's record at `record`.
fn retired(
    operations: &mut Operations,
    id: u64,
    lane: Lane,
    record: RecordState,
    answer: Envelope<manyhands::repository::MutationDataDto>,
) {
    queue(operations, id, retire_intent(), lane);
    progress(operations, id, OPERATION);
    operations.record_stood(DesktopRequestId(id), record);
    operations.apply(WorkerEvent::Finished(
        DesktopRequestId(id),
        Box::new(answer),
    ));
}

fn listed(operations: &mut Operations, operation: &[&str]) {
    let items = operation
        .iter()
        .map(|operation| OperationDto {
            operation_id: Some((*operation).to_owned()),
            ..row()
        })
        .collect();
    pending(
        operations,
        "/fixture/a",
        ReadResult::Ok(OperationListDto {
            items,
            complete: true,
        }),
    );
}

/// A failed request of another command, whose answer offers to abandon the
/// pending operation `operation` it left.
fn failed_with_operation(operations: &mut Operations, id: u64, operation: &str) {
    queue(
        operations,
        id,
        Intent::IndexRefresh {
            root: PathBuf::from("/fixture/a"),
        },
        Lane::User,
    );
    let mut envelope = Envelope::mutation(
        "operation resume",
        Scope::default(),
        Outcome::Error,
        Code::Busy,
    );
    envelope.recovery = vec![RecoveryAction {
        action: RecoveryActionKind::OperationAbandon,
        operation_id: Some(operation.to_owned()),
        arguments: Default::default(),
    }];
    operations.apply(WorkerEvent::Finished(
        DesktopRequestId(id),
        Box::new(envelope),
    ));
}

use RecordState::{Accepted, Finished};

/// Each answer of E3's table: whether its request is dismissed (the lane
/// does not matter) and what `needs_attention` counts for it. A pending or
/// `Accepted` retirement, with its pending row listed, is counted once.
#[gpui_kit::test]
async fn retire_rows_dismissed(cx: &mut TestAppContext) {
    let (_fixture, healthy) = failing(0, cx).await;
    type Fill = Box<dyn Fn(&mut Operations)>;
    let one = |lane, record, envelope: Envelope<_>| -> Fill {
        Box::new(move |operations| retired(operations, 1, lane, record, envelope.clone()))
    };
    let cases: Vec<(&str, Fill, bool, usize)> = vec![
        // Dismissed: retired, an attention row, a retry row.
        (
            "retired",
            one(
                Lane::Background,
                Finished,
                answer(State::Retired, Code::Ok, Removal::Observed),
            ),
            true,
            0,
        ),
        (
            "already absent",
            one(
                Lane::Background,
                Finished,
                answer(State::AlreadyAbsent, Code::AlreadyApplied, Removal::None),
            ),
            true,
            0,
        ),
        (
            "attention, background",
            one(
                Lane::Background,
                Finished,
                answer(State::Refused, Code::UnlockRequired, Removal::None),
            ),
            true,
            0,
        ),
        (
            "attention, user lane",
            one(
                Lane::User,
                Finished,
                answer(State::Refused, Code::HostApprovalRequired, Removal::None),
            ),
            true,
            0,
        ),
        (
            "configuration",
            one(
                Lane::User,
                Finished,
                answer(State::Refused, Code::InvalidRemote, Removal::None),
            ),
            true,
            0,
        ),
        (
            "transport",
            one(
                Lane::Background,
                Finished,
                answer(State::Refused, Code::TransportUnavailable, Removal::None),
            ),
            true,
            0,
        ),
        (
            "retry, user lane",
            one(
                Lane::User,
                Finished,
                answer(State::Refused, Code::IndexUnavailable, Removal::None),
            ),
            true,
            0,
        ),
        (
            "recovery required",
            one(
                Lane::Background,
                Finished,
                answer(State::Refused, Code::RecoveryRequired, Removal::None),
            ),
            true,
            0,
        ),
        // Stays and counts: the item's own answer, and an ordinary failure.
        (
            "refused",
            one(
                Lane::Background,
                Finished,
                answer(State::Refused, Code::WorktreeNotClean, Removal::None),
            ),
            false,
            1,
        ),
        (
            "refused, user lane",
            one(
                Lane::User,
                Finished,
                answer(State::Refused, Code::InvalidInput, Removal::None),
            ),
            false,
            1,
        ),
        (
            "preserved",
            one(
                Lane::Background,
                Finished,
                answer(State::Preserved, Code::ExternalChange, Removal::Observed),
            ),
            false,
            1,
        ),
        (
            "no data",
            one(Lane::Background, Finished, no_data(Code::InvalidInput)),
            false,
            1,
        ),
        // Pending counts once with its pending row, and once without it.
        (
            "pending, no row yet",
            one(
                Lane::Background,
                Finished,
                answer(State::Pending, Code::RecoveryRequired, Removal::None),
            ),
            false,
            1,
        ),
        (
            "pending, with its row",
            Box::new(|operations| {
                retired(
                    operations,
                    1,
                    Lane::Background,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                listed(operations, &[OPERATION]);
            }),
            false,
            1,
        ),
        (
            "left accepted, with its row",
            Box::new(|operations| {
                retired(
                    operations,
                    1,
                    Lane::Background,
                    Accepted,
                    answer(State::Refused, Code::Busy, Removal::None),
                );
                listed(operations, &[OPERATION]);
            }),
            false,
            1,
        ),
        (
            "pending with an attention code, with its row",
            Box::new(|operations| {
                retired(
                    operations,
                    1,
                    Lane::User,
                    Finished,
                    answer(State::Pending, Code::UnlockRequired, Removal::None),
                );
                listed(operations, &[OPERATION]);
            }),
            false,
            1,
        ),
    ];
    for (what, fill, dismissed, counted) in cases {
        let mut operations = Operations::default();
        fill(&mut operations);
        let request = operations
            .request(DesktopRequestId(1))
            .expect("the request");
        assert_eq!(
            matches!(request.state, RequestState::Dismissed),
            dismissed,
            "{what}: dismissed"
        );
        let seen = healthy.read_with(cx, |registry, _| {
            needs_attention(&operations, registry, &PollSchedule::default())
        });
        assert_eq!(seen, counted, "{what}: counted");
    }
}

/// R-23's three rules for `needs_attention`, as rows of
/// `counts_what_awaits_the_person`: requests and pending rows of one
/// operation count once, however many requests; the worker's running
/// operation is never a pending row. A row the rules do not touch still
/// counts.
pub(super) fn rows() -> Vec<(&'static str, usize, Operations)> {
    let mut rows = Vec::new();
    let mut case = |what, expected, fill: &dyn Fn(&mut Operations)| {
        let mut operations = Operations::default();
        fill(&mut operations);
        rows.push((what, expected, operations));
    };
    let pending_answer = || answer(State::Pending, Code::RecoveryRequired, Removal::None);

    // Rule 1: grouped by operation ID.
    case(
        "a failed request and its pending row, one operation",
        1,
        &|operations| {
            failed_with_operation(operations, 1, OTHER);
            listed(operations, &[OTHER]);
        },
    );
    case(
        "a failed request and a row of another operation",
        2,
        &|operations| {
            failed_with_operation(operations, 1, OTHER);
            listed(operations, &[OPERATION]);
        },
    );
    case(
        "a failed request, its row and another row",
        2,
        &|operations| {
            failed_with_operation(operations, 1, OTHER);
            listed(operations, &[OTHER, OPERATION]);
        },
    );
    case("a pending retirement and its row", 1, &|operations| {
        retired(operations, 1, Lane::Background, Finished, pending_answer());
        listed(operations, &[OPERATION]);
    });
    case(
        "two requests share a pending retirement's operation, with its row",
        1,
        &|operations| {
            // The original and a Resume resend of it.
            retired(operations, 1, Lane::Background, Finished, pending_answer());
            retired(operations, 2, Lane::User, Finished, pending_answer());
            listed(operations, &[OPERATION]);
        },
    );
    case(
        "two requests share an operation, no row listed",
        1,
        &|operations| {
            retired(operations, 1, Lane::Background, Finished, pending_answer());
            retired(operations, 2, Lane::User, Finished, pending_answer());
        },
    );
    case(
        "a request of its own operation and a retirement's row",
        2,
        &|operations| {
            failed_with_operation(operations, 1, OTHER);
            retired(operations, 2, Lane::Background, Finished, pending_answer());
            listed(operations, &[OTHER, OPERATION]);
        },
    );
    case(
        "a refused retirement of another operation beside a row",
        2,
        &|operations| {
            retired(
                operations,
                1,
                Lane::Background,
                Finished,
                answer_of(OTHER, State::Refused, Code::WorktreeNotClean, Removal::None),
            );
            listed(operations, &[OPERATION]);
        },
    );

    // Rule 3: the worker's running operation is not pending.
    case("a running retirement's own row", 0, &|operations| {
        queue(operations, 1, retire_intent(), Lane::Background);
        progress(operations, 1, OPERATION);
        listed(operations, &[OPERATION]);
    });
    case("a running retirement and another row", 1, &|operations| {
        queue(operations, 1, retire_intent(), Lane::Background);
        progress(operations, 1, OPERATION);
        listed(operations, &[OPERATION, OTHER]);
    });
    case(
        "a queued retirement has no operation yet",
        1,
        &|operations| {
            queue(operations, 1, retire_intent(), Lane::Background);
            listed(operations, &[OPERATION]);
        },
    );
    case(
        "a retirement that ended: its row is pending again",
        1,
        &|operations| {
            queue(operations, 1, retire_intent(), Lane::Background);
            progress(operations, 1, OPERATION);
            operations.apply(WorkerEvent::Removed(DesktopRequestId(1)));
            listed(operations, &[OPERATION]);
        },
    );
    case("D6's own automatic poll in flight", 0, &|operations| {
        let poll = OperationId::parse(OTHER).unwrap();
        operations.set_running_polls(HashSet::from([poll]));
        listed(operations, &[OTHER]);
    });
    case("an automatic poll's row, the poll over", 1, &|operations| {
        let poll = OperationId::parse(OTHER).unwrap();
        operations.set_running_polls(HashSet::from([poll]));
        operations.set_running_polls(HashSet::new());
        listed(operations, &[OTHER]);
    });
    case(
        "an automatic poll in flight and another row",
        1,
        &|operations| {
            let poll = OperationId::parse(OTHER).unwrap();
            operations.set_running_polls(HashSet::from([poll]));
            listed(operations, &[OTHER, OPERATION]);
        },
    );
    rows
}
