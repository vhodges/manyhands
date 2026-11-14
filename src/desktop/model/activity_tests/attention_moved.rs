//! P2, the rows moved from the old count's tests (E-3): `needs_attention`'s
//! `counts_what_awaits_the_person`, its polling and retirement rows, and
//! `retire_rows_dismissed`, each case a row here, under the N-row that now
//! says its count. A case whose count changed says so in its name.

use manyhands::{
    drafts::{DraftErrorKind, DraftId},
    repository::OperationDto,
    results::{Outcome, ResultCode as Code},
};

use super::{
    attention::{idle, sorted, waiting},
    scene::{OTHER_OPERATION, Row, Scene, pending_row, req, row, step},
    states::world::{OPERATION, OTHER, key_id},
    support::{ROOT, id},
};
use crate::desktop::{
    model::{
        activity::rows::{ItemRef, RowKey},
        polling::{Answer, Authority, CancelCause, NotEligible, Policy, TransportFailure},
    },
    services::DraftStoreUnavailable,
};

/// A pending row of `operation` that waits to be resumed.
pub(super) fn waiting_of(operation: &str) -> OperationDto {
    OperationDto {
        operation_id: Some(operation.to_owned()),
        ..waiting()
    }
}

pub(super) fn idle_of(operation: &str) -> OperationDto {
    OperationDto {
        operation_id: Some(operation.to_owned()),
        ..idle()
    }
}

pub(super) fn moved_rows() -> Vec<Row> {
    let mut rows = requests();
    rows.extend(polling());
    rows.extend(super::attention_retired::retirements());
    rows
}

/// `counts_what_awaits_the_person`: requests, drafts and lists (N3, N12 to
/// N15). The old pending rows had no next action, which N12 no longer
/// counts: each case that counted them says what counts now.
fn requests() -> Vec<Row> {
    let (first, second) = (DraftId::generate(), DraftId::generate());
    let ticket = || ItemRef {
        root: ROOT.to_owned(),
        kind: manyhands::repository::AuthoringKind::Ticket,
        id: super::scene::item(),
    };
    vec![
        row("E-3 nothing (N15)", vec![step("nothing", |_| {}, vec![])]),
        row(
            "E-3 finished: partial, blocked, error count; success, noop, cancelled do not (N3, N15)",
            vec![
                step(
                    "success, noop and cancelled",
                    |s| {
                        s.succeed(1, "IndexRefresh");
                        s.end(2, "IndexRebuild", Outcome::Noop, Code::AlreadyApplied);
                        s.cancel(3, "PollPause");
                    },
                    vec![],
                ),
                step(
                    "partial, blocked and error, three commands",
                    |s| {
                        s.end(4, "IndexRefresh", Outcome::Partial, Code::Busy);
                        s.end(5, "IndexRebuild", Outcome::Blocked, Code::Busy);
                        s.end(6, "PollPause", Outcome::Error, Code::Busy);
                    },
                    vec![req(4), req(5), req(6)],
                ),
            ],
        ),
        row(
            "E-3 queued and running (N15)",
            vec![step(
                "a queued request and a running one",
                |s| {
                    s.queue(1, "IndexRefresh");
                    s.queue(2, "IndexRebuild");
                    s.run(2);
                },
                vec![],
            )],
        ),
        row(
            "E-3 pending rows, one each; N12: rows with no next action count nothing now (was 3)",
            vec![
                step(
                    "three rows that wait to be resumed, two repositories",
                    |s| {
                        s.listed(
                            ROOT,
                            vec![waiting_of(OPERATION), waiting_of(OTHER_OPERATION)],
                        );
                        s.listed(OTHER, vec![waiting_of(OPERATION)]);
                    },
                    sorted(vec![
                        pending_row(ROOT, OPERATION),
                        pending_row(ROOT, OTHER_OPERATION),
                        pending_row(OTHER, OPERATION),
                    ]),
                ),
                step(
                    "the same rows with no next action",
                    |s| {
                        s.listed(ROOT, vec![idle_of(OPERATION), idle_of(OTHER_OPERATION)]);
                        s.listed(OTHER, vec![idle_of(OPERATION)]);
                    },
                    vec![],
                ),
            ],
        ),
        row(
            "E-3 pending, none listed (N12)",
            vec![step("an empty list", |s| s.listed(ROOT, vec![]), vec![])],
        ),
        row(
            "E-3 pending read failed, one each; busy and loading do not (N13)",
            vec![step(
                "two failed reads, a busy one and a loading one",
                |s| {
                    for root in [ROOT, OTHER] {
                        let failure = crate::desktop::services::ReadFailure {
                            code: Code::IndexUnavailable,
                            recovery: Vec::new(),
                        };
                        s.read(root, crate::desktop::services::ReadResult::Failed(failure));
                    }
                    s.read("/fixture/c", crate::desktop::services::ReadResult::Busy);
                    let _ = s.world.operations.load_pending("/fixture/d".to_owned());
                },
                sorted(vec![
                    RowKey::PendingList(ROOT.to_owned()),
                    RowKey::PendingList(OTHER.to_owned()),
                ]),
            )],
        ),
        row(
            "E-3 flush failures, one each; drafts unavailable (N14)",
            vec![step(
                "two recovery copies not saved and a store that did not open",
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
        row(
            "E-3 the sum (N3, N12 to N15)",
            vec![step(
                "a failed request, a success, a request not sent, drafts, rows and lists",
                move |s| {
                    s.fail(1, "IndexRefresh");
                    s.succeed(2, "IndexRebuild");
                    s.queue(3, "KeyClear");
                    s.world.operations.apply(
                        crate::desktop::services::worker::WorkerEvent::NotSent(
                            id(3),
                            crate::desktop::services::worker::NotSent::KeyStoreUnavailable,
                        ),
                    );
                    let flush =
                        |draft| (draft, ticket(), "Plan".to_owned(), DraftErrorKind::NoSpace);
                    s.world.drafts.flush_failures = vec![flush(first), flush(second)];
                    s.listed(
                        ROOT,
                        vec![waiting_of(OPERATION), waiting_of(OTHER_OPERATION)],
                    );
                    let failure = crate::desktop::services::ReadFailure {
                        code: Code::IndexUnavailable,
                        recovery: Vec::new(),
                    };
                    s.read(OTHER, crate::desktop::services::ReadResult::Failed(failure));
                    s.read("/fixture/c", crate::desktop::services::ReadResult::Busy);
                },
                sorted(vec![
                    req(1),
                    RowKey::Flush(first),
                    RowKey::Flush(second),
                    pending_row(ROOT, OPERATION),
                    pending_row(ROOT, OTHER_OPERATION),
                    RowKey::PendingList(OTHER.to_owned()),
                ]),
            )],
        ),
    ]
}

fn hold(reason: &str) -> RowKey {
    RowKey::Hold(reason.to_owned())
}

fn on(root: &str) -> impl Fn(&mut Scene, Answer) + '_ {
    move |s, answer| s.answered(root, answer)
}

fn host(name: Option<&str>) -> Answer {
    Answer::Transport(TransportFailure::HostUnapproved {
        authority: name.map(|host| Authority {
            host: host.to_owned(),
            port: 22,
        }),
    })
}

/// The polling rows: one hold for each key, host or repository the schedule
/// holds back (N5, N6, N18's holds), and nothing for what waits for itself.
fn polling() -> Vec<Row> {
    let (one, two) = (
        key_id(),
        manyhands::repository::SharedKeyId::parse("01ARZ3NDEKTSV4RRFFQ69G5FB2").unwrap(),
    );
    let locked = |key| Answer::Transport(TransportFailure::KeyLocked { key: Some(key) });
    let problem = |key| Answer::Transport(TransportFailure::KeyProblem { key });
    vec![
        row(
            "E-3 polling: two locked keys, one each (N5)",
            vec![step(
                "two repositories, two keys",
                move |s| {
                    on("/a")(s, locked(one));
                    on("/b")(s, locked(two));
                },
                sorted(vec![
                    hold(&format!("KeyLocked:{one}")),
                    hold(&format!("KeyLocked:{two}")),
                ]),
            )],
        ),
        row(
            "E-3 polling: no key, no host, no configuration and held until recovery, one per repository (N6)",
            vec![
                step(
                    "no key selected",
                    move |s| {
                        on("/a")(s, problem(None));
                        on("/b")(
                            s,
                            Answer::Transport(TransportFailure::KeyLocked { key: None }),
                        );
                    },
                    sorted(vec![hold("NoKeySelected:/a"), hold("NoKeySelected:/b")]),
                ),
                step(
                    "a host answer naming no host",
                    |s| {
                        s.world.schedule = Default::default();
                        on("/a")(s, host(None));
                        on("/b")(s, host(None));
                    },
                    sorted(vec![
                        hold("HostUnidentified:/a"),
                        hold("HostUnidentified:/b"),
                    ]),
                ),
                step(
                    "configuration required",
                    |s| {
                        s.world.schedule = Default::default();
                        for root in ["/a", "/b"] {
                            on(root)(
                                s,
                                Answer::Transport(TransportFailure::ConfigurationRequired),
                            );
                        }
                    },
                    sorted(vec![
                        hold("ConfigurationRequired:/a"),
                        hold("ConfigurationRequired:/b"),
                    ]),
                ),
                step(
                    "held until recovery",
                    |s| {
                        s.world.schedule = Default::default();
                        for root in ["/a", "/b"] {
                            on(root)(s, Answer::NotEligible(NotEligible::RecoverySuspended));
                        }
                    },
                    sorted(vec![hold("RecoveryHeld:/a"), hold("RecoveryHeld:/b")]),
                ),
            ],
        ),
        row(
            "E-3 polling: every kind once, and a key problem and a host held back two repositories (N5, N6)",
            vec![step(
                "seven kinds, one repository each, and one key problem and one host over two",
                move |s| {
                    on("/a")(s, locked(one));
                    on("/b")(s, problem(Some(two)));
                    on("/c")(s, problem(None));
                    on("/d")(s, host(Some("x")));
                    on("/e")(s, host(None));
                    on("/f")(
                        s,
                        Answer::Transport(TransportFailure::ConfigurationRequired),
                    );
                    on("/g")(s, Answer::NotEligible(NotEligible::RecoverySuspended));
                    on("/h")(s, host(Some("x")));
                    on("/i")(s, problem(Some(two)));
                },
                sorted(vec![
                    hold(&format!("KeyLocked:{one}")),
                    hold(&format!("KeyProblem:{two}")),
                    hold("NoKeySelected:/c"),
                    hold("HostUnapproved:x:22"),
                    hold("HostUnidentified:/e"),
                    hold("ConfigurationRequired:/f"),
                    hold("RecoveryHeld:/g"),
                ]),
            )],
        ),
        row(
            "E-3 polling: what waits for itself or is the person's own choice is not counted (N15)",
            vec![
                step(
                    "paused, off, not due, unreachable, waiting and quiet answers",
                    |s| {
                        let answers = [
                            Answer::NotEligible(NotEligible::Paused),
                            Answer::NotEligible(NotEligible::Disabled),
                            Answer::NotEligible(NotEligible::NotDue),
                            Answer::Transport(TransportFailure::Unreachable),
                            Answer::Transport(TransportFailure::Changed),
                            Answer::Transport(TransportFailure::Unavailable),
                            Answer::Busy,
                            Answer::Interrupted,
                            Answer::Repository {
                                recovery_required: true,
                            },
                            Answer::Repository {
                                recovery_required: false,
                            },
                            Answer::ConflictPending,
                            Answer::Cancelled(CancelCause::Preemption),
                            Answer::Cancelled(CancelCause::Quit),
                            Answer::Completed,
                        ];
                        for (at, answer) in answers.into_iter().enumerate() {
                            on(&format!("/r{at}"))(s, answer);
                        }
                    },
                    vec![],
                ),
                step(
                    "a paused policy, a policy that is off and a retirement loop",
                    |s| {
                        s.world.schedule = Default::default();
                        let policy = |paused, enabled| Policy {
                            paused,
                            enabled,
                            ..super::states::world::policy()
                        };
                        s.world
                            .schedule
                            .add("/p".to_owned(), policy(true, true), None);
                        s.world
                            .schedule
                            .add("/o".to_owned(), policy(false, false), None);
                        s.world
                            .schedule
                            .add("/l".to_owned(), policy(false, true), None);
                        assert!(s.world.schedule.begin_retiring(&"/l".to_owned()));
                    },
                    vec![],
                ),
                step(
                    "a repository the library holds back until recovery counts once",
                    |s| {
                        s.world.schedule = Default::default();
                        let suspended = Policy {
                            recovery_suspended: true,
                            ..super::states::world::policy()
                        };
                        let root = "/h".to_owned();
                        s.world.schedule.add(root.clone(), suspended.clone(), None);
                        s.world.schedule.sync_recovery(&root);
                        s.world.schedule.set_policy(&root, suspended);
                        s.world.schedule.sync_recovery(&root);
                    },
                    vec![hold("RecoveryHeld:/h")],
                ),
            ],
        ),
    ]
}
