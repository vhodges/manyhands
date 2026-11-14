//! P3, T9 and T11 to T18: "Clear finished", a choice made against a row that
//! changed (`still_offered`), and the latches. The fixture is `scene.rs`; T1
//! to T7 and T13 are in `log.rs`.

use manyhands::{
    repository::{ContextRetirementState as State, RetirementRemoval as Removal},
    results::ResultCode,
};

use super::{
    attention::{repository_of, sorted},
    scene::{Row, Scene, after, repo, req, row, step},
    support::{ITEM, ROOT},
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction, RowKey},
        polling::retire::Verdict,
        publication::SyncOffer,
        repository_state::{RowAction, StateKind as K},
    },
    services::worker::RecordState,
    test_support::retire::answer as retirement,
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
pub(super) const STOP: &str = "Stop at a safe point";

fn note() -> RowKey {
    RowKey::RetireNote {
        root: ROOT.to_owned(),
        item: ITEM.parse().unwrap(),
    }
}

fn retired() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    retirement(State::Retired, ResultCode::Ok, Removal::Observed)
}

/// A retirement refused with `code`: an information event for a code the
/// next attempt covers.
fn refused(
    code: ResultCode,
) -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    retirement(State::Refused, code, Removal::None)
}

/// A request's Dismiss, as its menu carries it.
pub(super) fn dismiss(n: u64) -> ActivityAction {
    ActivityAction::Dismiss(req(n))
}

/// The revision of request `n`'s row, kept under `name` for a later step.
pub(super) fn keep(s: &mut Scene, name: &'static str, n: u64) {
    let revision = s.row(&req(n)).revision;
    s.remember(name, revision);
}

pub(super) fn more_rows() -> Vec<Row> {
    let mut rows = vec![clearing()];
    rows.extend(super::log_choices::choices());
    rows.extend(latches());
    rows.extend(replaced());
    rows
}

/// T9: "Clear finished" takes the success, the failure settled under N22 that
/// still offers Retry, the two S8 events and the cancelled close, and nothing
/// else.
fn clearing() -> Row {
    row(
        "T9",
        vec![
            step(
                "a failure that offers Retry and the success that settles it; a stopped share whose record is Accepted; a stopped republish a lifecycle waits on; a failure; a cancelled close; an Owed retirement; the two S8 events; a state",
                |s| {
                    s.fail(1, "RepoSync");
                    s.facts_of(1).shown_offers = vec![SyncOffer::Retry];
                    s.succeed(2, "RepoSync");
                    s.stop_share(3, "ItemSync", RecordState::Accepted);
                    s.facts_of(3).shown_offers = vec![SyncOffer::GiveUp];
                    s.stop_share(4, "ItemRepublish", RecordState::Finished);
                    s.facts_of(4).resend_waits = true;
                    s.fail(5, "IndexRebuild");
                    s.cancel(6, "TicketClose");
                    s.retire(7, ITEM.parse().unwrap(), RecordState::Accepted, retired());
                    s.retire(
                        8,
                        ITEM.parse().unwrap(),
                        RecordState::Finished,
                        refused(ResultCode::UnlockRequired),
                    );
                    s.retire(9, DOCUMENT.parse().unwrap(), RecordState::Finished, retired());
                    s.world.repositories = vec![repository_of(
                        ROOT,
                        K::ThisComputer,
                        Some(RowAction::SetUpSharing),
                        None,
                    )];
                },
                vec![req(5)],
            )
            .check(|s| {
                let mut taken = s.log.clearable();
                taken.sort();
                assert_eq!(
                    taken,
                    sorted(vec![req(1), req(2), req(6), RowKey::Retired(1), note()]),
                    "only these go"
                );
                let settled = s.row(&req(1));
                assert!(
                    settled
                        .row
                        .menu
                        .iter()
                        .any(|offer| offer.label == "Retry" && offer.enabled),
                    "the settled failure still offers Retry"
                );
            }),
            step(
                "Clear finished: requests through dismiss_request, events through the log",
                |s| {
                    for key in s.log.clearable() {
                        match key {
                            RowKey::Request(request) => s.dismiss(request.0),
                            other => s.log.dismiss(&other),
                        }
                    }
                },
                vec![req(5)],
            )
            .check(|s| {
                assert_eq!(
                    {
                        let mut shown = s.shown();
                        shown.sort();
                        shown
                    },
                    sorted(vec![req(3), req(4), req(5), req(7), repo(ROOT)]),
                    "the stopped share, the republish, the failure, the Owed retirement and the state stay"
                );
                assert!(s.log.clearable().is_empty(), "nothing is left to clear");
            }),
        ],
    )
}

/// T14 to T17: the latch (R-56).
fn latches() -> Vec<Row> {
    vec![
        row(
            "T14",
            vec![
                step("a failed share", |s| s.fail(1, "ItemSync"), vec![req(1)]),
                step(
                    "a later share succeeds: the failure is settled",
                    |s| s.succeed(2, "ItemSync"),
                    vec![],
                ),
                step(
                    "the success is dismissed, or cleared: the failure stays uncounted",
                    |s| s.dismiss(2),
                    vec![],
                )
                .check(|s| {
                    assert!(s.has(&req(1)) && !s.has(&req(2)));
                    assert!(
                        s.row(&req(1)).row.clearable,
                        "it stays as history, and can be cleared"
                    );
                }),
            ],
        ),
        row(
            "T14 a request with no attempt number",
            vec![
                step(
                    "a failed share the model never saw queued",
                    |s| {
                        s.unsent.insert(1);
                        s.fail(1, "ItemSync");
                    },
                    vec![req(1)],
                ),
                step(
                    "a later share succeeds: the failure is settled",
                    |s| s.succeed(2, "ItemSync"),
                    vec![],
                ),
                step(
                    "the success is dismissed: the failure stays uncounted",
                    |s| s.dismiss(2),
                    vec![],
                )
                .check(|s| assert!(s.has(&req(1)) && !s.has(&req(2)))),
            ],
        ),
        row(
            "T15",
            vec![
                step("a failed share", |s| s.fail(1, "ItemSync"), vec![req(1)]),
                step(
                    "a later share succeeds",
                    |s| s.succeed(2, "ItemSync"),
                    vec![],
                ),
                step(
                    "the failed one is retried and fails again: counted again",
                    |s| {
                        s.retry(1);
                        s.run(1);
                        s.finish(
                            1,
                            super::support::answer(
                                manyhands::results::Outcome::Error,
                                ResultCode::InternalError,
                            ),
                        );
                    },
                    vec![req(1)],
                ),
                step(
                    "another share succeeds: settled again",
                    |s| s.succeed(3, "ItemSync"),
                    vec![],
                ),
            ],
        ),
        row(
            "T16",
            vec![
                step(
                    "a Resume answered original_request_required, and the original sent again",
                    |s| {
                        s.fail_code(1, "OperationResume", ResultCode::OriginalRequestRequired);
                        s.succeed(2, "IndexRefresh");
                        s.facts_of(1).followed = true;
                    },
                    vec![],
                ),
                step(
                    "the follow-up is dismissed after it succeeded: the Resume stays uncounted",
                    |s| {
                        s.dismiss(2);
                        s.facts_of(1).followed = false;
                    },
                    vec![],
                )
                .check(|s| assert!(s.has(&req(1)))),
            ],
        ),
        row(
            "T17",
            vec![
                step(
                    "a Resume answered original_request_required, and the original queued",
                    |s| {
                        s.fail_code(1, "OperationResume", ResultCode::OriginalRequestRequired);
                        s.queue(2, "IndexRefresh");
                        s.facts_of(1).followed = true;
                    },
                    vec![],
                ),
                step(
                    "the follow-up is cancelled while queued: the Resume counts",
                    |s| {
                        s.remove(2);
                        s.facts_of(1).follow_void = true;
                    },
                    vec![req(1)],
                ),
            ],
        ),
    ]
}

/// T18: a later information event for the same item replaces the earlier
/// (R-59).
fn replaced() -> Vec<Row> {
    let unreachable = || refused(ResultCode::TransportUnavailable);
    let retry = || refused(ResultCode::IndexUnavailable);
    let _ = Verdict::Retired;
    vec![
        row(
            "T18 the earlier one is shown",
            vec![
                step(
                    "an information event",
                    move |s| {
                        s.retire(
                            1,
                            ITEM.parse().unwrap(),
                            RecordState::Finished,
                            unreachable(),
                        )
                    },
                    vec![],
                )
                .check(|s| assert_eq!(s.row(&note()).at, after(0))),
                step(
                    "a later one for the same item: one row, with the time now",
                    move |s| {
                        s.at(1);
                        s.retire(2, ITEM.parse().unwrap(), RecordState::Finished, retry());
                    },
                    vec![],
                )
                .check(|s| {
                    let notes = s
                        .shown()
                        .into_iter()
                        .filter(|key| matches!(key, RowKey::RetireNote { .. }))
                        .count();
                    assert_eq!(notes, 1, "one RetireNote row for the item");
                    assert_eq!(s.row(&note()).at, after(1));
                }),
            ],
        ),
        row(
            "T18 the earlier one is dismissed",
            vec![
                step(
                    "an information event",
                    move |s| {
                        s.retire(
                            1,
                            ITEM.parse().unwrap(),
                            RecordState::Finished,
                            unreachable(),
                        )
                    },
                    vec![],
                ),
                step(
                    "it is dismissed",
                    |s| {
                        s.at(1);
                        s.log.dismiss(&note());
                    },
                    vec![],
                )
                .check(|s| assert!(!s.has(&note()))),
                step(
                    "a later one for the same item: shown, with the time now",
                    move |s| {
                        s.at(2);
                        s.retire(2, ITEM.parse().unwrap(), RecordState::Finished, retry());
                    },
                    vec![],
                )
                .check(|s| assert_eq!(s.row(&note()).at, after(2))),
            ],
        ),
    ]
}
