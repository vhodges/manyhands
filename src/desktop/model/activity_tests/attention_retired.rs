//! P2, the retirement rows moved from the old count's tests (E-3):
//! `retire_rows_dismissed` and the R-23 rows.

use manyhands::{
    repository::{ContextRetirementState as State, RetirementRemoval as Removal},
    results::ResultCode as Code,
};

use super::{
    attention::sorted,
    attention_moved::{idle_of, waiting_of},
    scene::{OTHER_OPERATION, Row, Scene, pending_row, req, row, step},
    states::world::OPERATION,
    support::{ROOT, id},
};
use crate::desktop::{
    model::activity::rows::RowKey,
    services::{requests::RequestState, worker::RecordState},
    test_support::retire::{answer, answer_of, no_data},
};

/// A retirement of the item answered `envelope` with its record at `record`.
fn retired(
    s: &mut Scene,
    n: u64,
    record: RecordState,
    envelope: manyhands::results::Envelope<manyhands::repository::MutationDataDto>,
) {
    s.retire(n, super::scene::item(), record, envelope);
}

/// `retire_rows_dismissed` and the retirement rows of the old count (N1, N3,
/// N12, N21, N24; R-23). Each case: whether its request is dismissed and what
/// counts. An answer that is not dismissed has an operation, which makes it N24
/// or, with its pending row, N1; the old pending rows had no next action, so
/// the cases with a row count the request (N12), and the ones where a row
/// counts itself have a row that waits.
pub(super) fn retirements() -> Vec<Row> {
    use RecordState::{Accepted, Finished};
    let refused = |code| answer(State::Refused, code, Removal::None);
    let mut rows = Vec::new();
    // E3's table: whether the request is dismissed, and what counts.
    let table: Vec<(
        &str,
        RecordState,
        manyhands::results::Envelope<_>,
        bool,
        usize,
    )> = vec![
        (
            "retired",
            Finished,
            answer(State::Retired, Code::Ok, Removal::Observed),
            true,
            0,
        ),
        (
            "already absent",
            Finished,
            answer(State::AlreadyAbsent, Code::AlreadyApplied, Removal::None),
            true,
            0,
        ),
        (
            "attention",
            Finished,
            refused(Code::UnlockRequired),
            true,
            0,
        ),
        (
            "attention, a host",
            Finished,
            refused(Code::HostApprovalRequired),
            true,
            0,
        ),
        (
            "configuration",
            Finished,
            refused(Code::InvalidRemote),
            true,
            0,
        ),
        (
            "transport",
            Finished,
            refused(Code::TransportUnavailable),
            true,
            0,
        ),
        ("retry", Finished, refused(Code::IndexUnavailable), true, 0),
        (
            "recovery required",
            Finished,
            refused(Code::RecoveryRequired),
            true,
            0,
        ),
        (
            "refused",
            Finished,
            refused(Code::WorktreeNotClean),
            false,
            1,
        ),
        (
            "refused, another code",
            Finished,
            refused(Code::InvalidInput),
            false,
            1,
        ),
        (
            "preserved",
            Finished,
            answer(State::Preserved, Code::ExternalChange, Removal::Observed),
            false,
            1,
        ),
        ("no data", Finished, no_data(Code::InvalidInput), false, 1),
        (
            "pending, no row yet",
            Finished,
            answer(State::Pending, Code::RecoveryRequired, Removal::None),
            false,
            1,
        ),
    ];
    for (what, record, envelope, dismissed, counted) in table {
        let keys = if counted == 1 { vec![req(1)] } else { vec![] };
        rows.push(row(
            Box::leak(format!("E-3 retire: {what} (N24, N16)").into_boxed_str()),
            vec![
                step(
                    "the request answered",
                    move |s| retired(s, 1, record, envelope.clone()),
                    keys,
                )
                .check(move |s| {
                    let state = &s
                        .world
                        .operations
                        .request(id(1))
                        .expect("the request")
                        .state;
                    assert_eq!(
                        matches!(state, RequestState::Dismissed),
                        dismissed,
                        "dismissed"
                    );
                }),
            ],
        ));
    }
    let case = |what: &'static str, counted: Vec<RowKey>, change: fn(&mut Scene)| {
        row(
            what,
            vec![step("the requests and the rows", change, counted)],
        )
    };
    let op = |operation| pending_row(ROOT, operation);
    rows.extend([
        case(
            "E-3 retire: pending, with its row (N1)",
            vec![req(1)],
            |s| {
                retired(
                    s,
                    1,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                s.listed(ROOT, vec![idle_of(OPERATION)]);
            },
        ),
        case(
            "E-3 retire: left accepted, with its row (N1)",
            vec![req(1)],
            |s| {
                retired(
                    s,
                    1,
                    Accepted,
                    answer(State::Refused, Code::Busy, Removal::None),
                );
                s.listed(ROOT, vec![idle_of(OPERATION)]);
            },
        ),
        case(
            "E-3 retire: pending with an attention code, with its row (N1)",
            vec![req(1)],
            |s| {
                retired(
                    s,
                    1,
                    Finished,
                    answer(State::Pending, Code::UnlockRequired, Removal::None),
                );
                s.listed(ROOT, vec![idle_of(OPERATION)]);
            },
        ),
        // R-23's rules.
        case(
            "E-3 R-23: a failed request and its pending row, one operation (N1)",
            vec![op(OTHER_OPERATION)],
            |s| {
                s.fail_op(1, "IndexRefresh", OTHER_OPERATION);
                s.listed(ROOT, vec![waiting_of(OTHER_OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: a failed request and a row of another operation",
            sorted(vec![req(1), op(OPERATION)]),
            |s| {
                s.fail_op(1, "IndexRefresh", OTHER_OPERATION);
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: a failed request, its row and another row",
            sorted(vec![op(OTHER_OPERATION), op(OPERATION)]),
            |s| {
                s.fail_op(1, "IndexRefresh", OTHER_OPERATION);
                s.listed(
                    ROOT,
                    vec![waiting_of(OTHER_OPERATION), waiting_of(OPERATION)],
                );
            },
        ),
        case(
            "E-3 R-23: a pending retirement and its row",
            vec![op(OPERATION)],
            |s| {
                retired(
                    s,
                    1,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: two requests share a pending retirement's operation, with its row",
            vec![op(OPERATION)],
            |s| {
                retired(
                    s,
                    1,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                retired(
                    s,
                    2,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: left accepted, no data, with its row",
            vec![op(OPERATION)],
            |s| {
                retired(
                    s,
                    1,
                    Accepted,
                    no_data(Code::Busy).with_operation_id(OPERATION),
                );
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: two requests share an operation, no row listed",
            vec![req(2)],
            |s| {
                retired(
                    s,
                    1,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                retired(
                    s,
                    2,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
            },
        ),
        case(
            "E-3 R-23: a request of its own operation and a retirement's row",
            sorted(vec![op(OTHER_OPERATION), op(OPERATION)]),
            |s| {
                s.fail_op(1, "IndexRefresh", OTHER_OPERATION);
                retired(
                    s,
                    2,
                    Finished,
                    answer(State::Pending, Code::RecoveryRequired, Removal::None),
                );
                s.listed(
                    ROOT,
                    vec![waiting_of(OTHER_OPERATION), waiting_of(OPERATION)],
                );
            },
        ),
        case(
            "E-3 R-23: a refused retirement of another operation beside a row",
            sorted(vec![req(1), op(OPERATION)]),
            |s| {
                retired(
                    s,
                    1,
                    Finished,
                    answer_of(
                        OTHER_OPERATION,
                        State::Refused,
                        Code::WorktreeNotClean,
                        Removal::None,
                    ),
                );
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        // Rule 3: the worker's running operation is not pending (N21).
        case(
            "E-3 R-23: a running retirement's own row (N21)",
            vec![],
            |s| {
                s.queue_retire(1);
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: a running retirement and another row (N21)",
            vec![op(OTHER_OPERATION)],
            |s| {
                s.queue_retire(1);
                s.listed(
                    ROOT,
                    vec![waiting_of(OPERATION), waiting_of(OTHER_OPERATION)],
                );
            },
        ),
        case(
            "E-3 R-23: a queued retirement has no operation yet",
            vec![op(OPERATION)],
            |s| {
                s.queue_intent(1, retire_intent());
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: a retirement that ended: its row is pending again",
            vec![op(OPERATION)],
            |s| {
                s.queue_retire(1);
                s.remove(1);
                s.listed(ROOT, vec![waiting_of(OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: D6's own automatic poll in flight (N21)",
            vec![],
            |s| {
                s.poll_runs(OTHER_OPERATION);
                s.listed(ROOT, vec![waiting_of(OTHER_OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: an automatic poll's row, the poll over",
            vec![op(OTHER_OPERATION)],
            |s| {
                s.poll_runs(OTHER_OPERATION);
                s.poll_runs_none();
                s.listed(ROOT, vec![waiting_of(OTHER_OPERATION)]);
            },
        ),
        case(
            "E-3 R-23: an automatic poll in flight and another row",
            vec![op(OPERATION)],
            |s| {
                s.poll_runs(OTHER_OPERATION);
                s.listed(
                    ROOT,
                    vec![waiting_of(OTHER_OPERATION), waiting_of(OPERATION)],
                );
            },
        ),
    ]);
    rows
}

fn retire_intent() -> crate::desktop::services::requests::Intent {
    crate::desktop::services::requests::Intent::ContextRetire {
        root: ROOT.into(),
        kind: manyhands::repository::AuthoringKind::Ticket,
        id: super::scene::item(),
    }
}
