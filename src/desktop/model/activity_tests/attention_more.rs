//! P2, rows N13 to N25 of A3's table (N20 withdrawn), N22's clauses and the
//! precedence of N23. The first rows are in `attention.rs`; the runner and
//! the fixture are in `scene.rs`.

use std::collections::HashSet;

use manyhands::{
    drafts::{DraftErrorKind, DraftId},
    repository::{
        AuthoringKind, ContextRetirementState as State, OperationAction, OperationDto,
        OperationFamily, OperationId, ProgressStage, RetirementRemoval as Removal,
    },
    results::ResultCode,
};

use super::{
    attention::{cant_reach, locked_by, repository_of, sorted, waiting},
    scene::{OTHER_OPERATION, Row, key_hold, pending_row, repo, req, row, step},
    states::world::{OPERATION, OTHER, key_id, operation, target},
    support::{ITEM, ROOT},
};
use crate::desktop::{
    model::{
        activity::rows::{ItemRef, RowKey, Target},
        polling::{Answer, Disposition, TargetKind, TransportFailure, entry::RetireOffer},
        repository_state::{RowAction, StateKind as K},
    },
    services::{
        DraftStoreUnavailable, ReadFailure, ReadResult, inflight::Report, worker::RecordState,
    },
    test_support::retire::answer as retirement,
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";

fn locked() -> RowKey {
    key_hold(key_id())
}

fn retired_answer() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    retirement(State::Retired, ResultCode::Ok, Removal::Observed)
}

/// A lifecycle's pending parent, waiting to be resumed.
fn closing() -> OperationDto {
    OperationDto {
        item_id: Some(ITEM.to_owned()),
        ..operation(OperationAction::Close, OperationFamily::Local)
    }
}

pub(super) fn more_rows() -> Vec<Row> {
    let mut rows = lists();
    rows.extend(never_and_dismissed());
    rows.extend(covering());
    rows.extend(running());
    rows.extend(super::attention_settled::settled());
    rows.extend(followed());
    rows
}

/// N13 and N14: the lists and the drafts.
fn lists() -> Vec<Row> {
    let (first, second) = (DraftId::generate(), DraftId::generate());
    let ticket = || ItemRef {
        root: ROOT.to_owned(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
    };
    vec![
        row(
            "N13",
            vec![
                step(
                    "Failed counts, Busy does not, Loading makes no row",
                    |s| {
                        let failed = ReadFailure {
                            code: ResultCode::InternalError,
                            recovery: Vec::new(),
                        };
                        s.read(ROOT, ReadResult::Failed(failed));
                        s.read(OTHER, ReadResult::Busy);
                        let _ = s.world.operations.load_pending("/fixture/third".to_owned());
                    },
                    vec![RowKey::PendingList(ROOT.to_owned())],
                )
                .check(|s| {
                    assert!(s.has(&RowKey::PendingList(OTHER.to_owned())));
                    assert!(!s.has(&RowKey::PendingList("/fixture/third".to_owned())));
                }),
            ],
        ),
        row(
            "N14",
            vec![step(
                "each recovery copy not saved, and the store unavailable, once",
                move |s| {
                    let flush =
                        |draft| (draft, ticket(), "Plan".to_owned(), DraftErrorKind::NoSpace);
                    s.world.drafts.flush_failures = vec![flush(first), flush(second)];
                    s.world.drafts.unavailable =
                        Some(DraftStoreUnavailable(DraftErrorKind::Access));
                },
                sorted(vec![
                    RowKey::Flush(first),
                    RowKey::Flush(second),
                    RowKey::DraftStore,
                ]),
            )],
        ),
    ]
}

/// N15 and N16, N17, N18: what never counts, what was dismissed, the problems
/// of an unreachable repository, a hold over two repositories.
fn never_and_dismissed() -> Vec<Row> {
    let brought = RowKey::Brought {
        root: ROOT.to_owned(),
        seq: 0,
    };
    let problem = |item: &str| RowKey::Problem {
        root: ROOT.to_owned(),
        target: Target::Item(item.parse().unwrap()),
    };
    vec![
        row(
            "N15",
            vec![step(
                "a success, a cancelled and a removed request, one never sent, a brought-in row, a reminder, a Retire… offer, S8's events, recovered drafts, a record note, This computer and a remote that is not SSH",
                |s| {
                    s.succeed(1, "IndexRefresh");
                    s.cancel(2, "IndexRebuild");
                    s.queue(3, "KeyClear");
                    s.world.operations.apply(crate::desktop::services::worker::WorkerEvent::NotSent(
                        super::support::id(3),
                        crate::desktop::services::worker::NotSent::SecretRequired,
                    ));
                    s.queue(4, "PollPause");
                    s.remove(4);
                    s.world.schedule_root(ROOT);
                    s.world.automatic(ROOT, Answer::Completed, &[Disposition::FastForwarded]);
                    s.world.repositories = vec![
                        repository_of(ROOT, K::ThisComputer, Some(RowAction::SetUpSharing), None),
                        repository_of(OTHER, K::NotSsh, Some(RowAction::SetUpSharing), None),
                    ];
                    s.world.repositories[0].unshared.items = vec![target(Disposition::Unpublished)];
                    s.world.repositories[0].retire_offers = vec![RetireOffer {
                        kind: TargetKind::Ticket,
                        item: ITEM.parse().unwrap(),
                    }];
                    s.retire(5, ITEM.parse().unwrap(), RecordState::Finished, retired_answer());
                    s.retire(
                        6,
                        DOCUMENT.parse().unwrap(),
                        RecordState::Finished,
                        retirement(State::Refused, ResultCode::TransportUnavailable, Removal::None),
                    );
                    s.world.drafts.recovered = Some(2);
                    assert!(s.world.operations.note_record(Report::NotSaved));
                },
                vec![],
            )
            .check({
                let brought = brought.clone();
                move |s| {
                for key in [
                    req(1),
                    req(2),
                    req(3),
                    req(4),
                    brought.clone(),
                    RowKey::Unshared {
                        root: ROOT.to_owned(),
                        target: Target::Item(ITEM.parse().unwrap()),
                    },
                    RowKey::RetireOffer {
                        root: ROOT.to_owned(),
                        item: ITEM.parse().unwrap(),
                    },
                    RowKey::Retired(0),
                    RowKey::RetireNote {
                        root: ROOT.to_owned(),
                        item: DOCUMENT.parse().unwrap(),
                    },
                    RowKey::Recovered,
                    RowKey::Note(Report::NotSaved),
                ] {
                    assert!(s.has(&key), "{key:?} is shown");
                }
                }
            })],
        ),
        row(
            "N16",
            vec![
                step("a failed request", |s| s.fail(1, "IndexRefresh"), vec![req(1)]),
                step("dismissed: not shown, not counted", |s| s.dismiss(1), vec![])
                    .check(|s| assert!(!s.has(&req(1)))),
                step(
                    "a brought-in row, dismissed through the log",
                    |s| {
                        s.world.schedule_root(ROOT);
                        s.world.automatic(ROOT, Answer::Completed, &[Disposition::FastForwarded]);
                    },
                    vec![],
                )
                .check({
                    let brought = brought.clone();
                    move |s| assert!(s.has(&brought))
                }),
                step(
                    "dismissed",
                    {
                        let brought = brought.clone();
                        move |s| s.log.dismiss(&brought)
                    },
                    vec![],
                )
                .check(move |s| assert!(!s.has(&brought))),
            ],
        ),
        row(
            "N17",
            vec![step(
                "unreachable, with kept problems: problem rows, no Can't reach row",
                |s| {
                    s.answered(ROOT, Answer::Transport(TransportFailure::Unreachable));
                    s.world.repositories = vec![repository_of(
                        ROOT,
                        K::NeedsAttention,
                        None,
                        Some(crate::desktop::model::repository_state::AttentionCause::Problems),
                    )];
                    s.world.repositories[0].unshared.problems = vec![
                        target(Disposition::Dirty),
                        crate::desktop::model::polling::AttemptTarget {
                            item_id: Some(DOCUMENT.parse().unwrap()),
                            kind: Some(TargetKind::Document),
                            disposition: Disposition::Malformed,
                            main: false,
                        },
                    ];
                },
                sorted(vec![problem(ITEM), problem(DOCUMENT)]),
            )
            .check(|s| assert!(!s.has(&repo(ROOT))))],
        ),
        row(
            "N18",
            vec![step(
                "a hold over two repositories, and a Check now of one fails unlock_required: once, as the hold",
                |s| {
                    s.answered(ROOT, locked_by(key_id()));
                    s.answered(OTHER, locked_by(key_id()));
                    s.world.repositories = [ROOT, OTHER]
                        .into_iter()
                        .map(|root| {
                            repository_of(root, K::Locked, Some(RowAction::Unlock(key_id())), None)
                        })
                        .collect();
                    s.fail_code(1, "PollOnce", ResultCode::UnlockRequired);
                    s.facts_of(1).check_applied = true;
                },
                vec![locked()],
            )
            .check(|s| {
                assert!(s.has(&req(1)));
                assert_eq!(s.row(&locked()).row.repo.as_deref(), Some("demo +1"));
            })],
        ),
    ]
}

/// N19: a row kept by L3 while a poll runs absorbs a failed Check now.
fn covering() -> Vec<Row> {
    vec![
        row(
            "N19 after the poll answers",
            vec![
                step(
                    "a Can't reach row",
                    |s| s.world.repositories = vec![cant_reach(ROOT)],
                    vec![repo(ROOT)],
                ),
                step(
                    "a poll runs, the row is kept, and a Check now fails",
                    |s| {
                        s.world.repositories = vec![repository_of(ROOT, K::Checking, None, None)];
                        s.world.repositories[0].checking = true;
                        s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                    },
                    vec![repo(ROOT)],
                ),
                step(
                    "the poll answers and the repository is still unreachable: still one thing",
                    |s| {
                        s.world.repositories = vec![cant_reach(ROOT)];
                    },
                    vec![repo(ROOT)],
                ),
            ],
        ),
        row(
            "N19",
            vec![
                step(
                    "a Can't reach row",
                    |s| s.world.repositories = vec![cant_reach(ROOT)],
                    vec![repo(ROOT)],
                ),
                step(
                    "a poll runs, the row is kept, and a Check now fails: once, as the row",
                    |s| {
                        s.world.repositories = vec![repository_of(ROOT, K::Checking, None, None)];
                        s.world.repositories[0].checking = true;
                        s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                    },
                    vec![repo(ROOT)],
                )
                .check(|s| assert!(s.has(&req(1)))),
            ],
        ),
    ]
}

/// N21 and N25: work that runs, and a lifecycle that runs elsewhere.
fn running() -> Vec<Row> {
    let operation_id = || OperationId::parse(OPERATION).unwrap();
    vec![
        row(
            "N21 a request runs the operation",
            vec![
                step(
                    "a failed request and its pending row: the row counts",
                    |s| {
                        s.fail_op(1, "IndexRefresh", OPERATION);
                        s.listed(ROOT, vec![waiting()]);
                    },
                    vec![pending_row(ROOT, OPERATION)],
                ),
                step(
                    "a Resume of it runs: neither the request nor the pending row counts",
                    |s| {
                        s.queue(2, "IndexRebuild");
                        s.run(2);
                        s.stage(2, ProgressStage::Fetching, Some(OPERATION));
                    },
                    vec![],
                )
                .check(|s| {
                    assert!(
                        !s.has(&pending_row(ROOT, OPERATION)),
                        "pending_shown hides it"
                    );
                    assert!(s.has(&req(1)) && s.has(&req(2)));
                }),
            ],
        ),
        row(
            "N21 an automatic poll runs the operation",
            vec![
                step(
                    "the poll runs: its request-less rows do not count",
                    move |s| {
                        s.world
                            .operations
                            .set_running_polls(HashSet::from([operation_id()]));
                        s.fail_op(1, "IndexRefresh", OPERATION);
                        s.listed(ROOT, vec![waiting()]);
                    },
                    vec![],
                ),
                step(
                    "the poll ends: the row counts again",
                    |s| {
                        s.world.operations.set_running_polls(HashSet::new());
                    },
                    vec![pending_row(ROOT, OPERATION)],
                ),
            ],
        ),
        row(
            "N25",
            vec![
                step(
                    "its close runs elsewhere: Still running…, not counted",
                    |s| {
                        s.listed(ROOT, vec![closing()]);
                        s.world.running = vec![(ROOT.to_owned(), ITEM.parse().unwrap())];
                    },
                    vec![],
                )
                .check(|s| {
                    let button = s
                        .row(&pending_row(ROOT, OPERATION))
                        .row
                        .button
                        .clone()
                        .unwrap();
                    assert_eq!(
                        (button.label.as_str(), button.enabled),
                        ("Still running\u{2026}", false)
                    );
                }),
                step(
                    "it ends: the row counts",
                    |s| s.world.running.clear(),
                    vec![pending_row(ROOT, OPERATION)],
                ),
            ],
        ),
        // N25 beside a failed request of the same operation: the table says
        // only that the row does not count, and N1's fallback names N12.
        row(
            "N25 with a failed request of its operation",
            vec![step(
                "its close runs elsewhere, and an earlier failure names the operation",
                |s| {
                    s.fail_op(1, "IndexRefresh", OPERATION);
                    s.listed(ROOT, vec![closing()]);
                    s.world.running = vec![(ROOT.to_owned(), ITEM.parse().unwrap())];
                },
                vec![],
            )],
        ),
    ]
}

/// N23, N24 and the precedence of N23 over N2.
fn followed() -> Vec<Row> {
    vec![
        row(
            "N23",
            vec![
                step(
                    "a Resume answered original_request_required, and the original sent again: not counted",
                    |s| {
                        s.fail_code(1, "OperationResume", ResultCode::OriginalRequestRequired);
                        s.queue(2, "IndexRefresh");
                        s.facts_of(1).followed = true;
                    },
                    vec![],
                )
                .check(|s| assert!(s.has(&req(1)) && s.has(&req(2)))),
                step(
                    "the follow-up is cancelled while queued: the Resume counts again",
                    |s| {
                        s.remove(2);
                        s.facts_of(1).follow_void = true;
                    },
                    vec![req(1)],
                ),
                step(
                    "the follow-up is dismissed: the Resume still counts",
                    |s| {
                        s.dismiss(2);
                        s.facts_of(1).followed = false;
                        s.facts_of(1).follow_void = false;
                    },
                    vec![req(1)],
                ),
            ],
        ),
        row(
            "N24",
            vec![step(
                "a failure names an operation no pending row shows: once",
                |s| s.fail_op(1, "IndexRefresh", OPERATION),
                vec![req(1)],
            )],
        ),
        // The precedence: N23 excludes a request before N2 groups the rest.
        row(
            "N23 before N2",
            vec![step(
                "two failures of one operation, the newer one followed: the older counts",
                |s| {
                    s.fail_op(1, "IndexRefresh", OTHER_OPERATION);
                    s.fail_op(2, "IndexRebuild", OTHER_OPERATION);
                    s.facts_of(2).followed = true;
                },
                vec![req(1)],
            )],
        ),
    ]
}
