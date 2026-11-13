//! P3: the log keeps times and events. A4's T-table, less T8 and T10 (window
//! tests), one row each over `ActivityLog::refresh` with an injected clock:
//! no sleeps, no real time. The fixture is `scene.rs`; the rows after T9 are
//! in `log_more.rs`.

use manyhands::results::ResultCode;

use super::{
    attention::{cant_reach, locked_by, repository_of, sorted, waiting},
    scene::{Row, after, key_hold, pending_row, repo, req, row, step},
    states::world::{OPERATION, key_id},
    support::ROOT,
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction, RowKey},
        polling::{Answer, Disposition},
        repository_state::{RowAction, StateKind as K},
    },
    services::{ReadFailure, ReadResult},
};

fn hold() -> RowKey {
    key_hold(key_id())
}

fn brought() -> RowKey {
    RowKey::Brought {
        root: ROOT.to_owned(),
        seq: 0,
    }
}

/// P3: each row of A4's table that is the log's to test.
#[test]
fn the_log_keeps_times_and_events() {
    let mut rows = times();
    rows.extend(kept());
    rows.extend(super::log_more::more_rows());
    super::scene::play("the_log_keeps_times_and_events", rows);
}

/// T1 to T4: a key's time, and its life.
fn times() -> Vec<Row> {
    vec![
        row(
            "T1",
            vec![
                step(
                    "a repository row and a brought-in row, first seen",
                    |s| {
                        s.at(0);
                        s.world.repositories = vec![cant_reach(ROOT)];
                        s.world.schedule_root(ROOT);
                        s.world
                            .automatic(ROOT, Answer::Completed, &[Disposition::FastForwarded]);
                    },
                    vec![repo(ROOT)],
                )
                .check(|s| {
                    assert!(s.changed, "the log reports a change");
                    assert_eq!(s.row(&repo(ROOT)).at, after(0), "first seen now");
                    assert_ne!(s.world.utc(), after(0), "the checks' own time differs");
                    assert_eq!(
                        s.row(&brought()).at,
                        s.world.utc(),
                        "Brought.at wins over now"
                    );
                }),
                step("seen again later", |s| s.at(5), vec![repo(ROOT)]).check(|s| {
                    assert!(!s.changed, "nothing changed: no change is reported");
                    assert_eq!(s.row(&repo(ROOT)).at, after(0));
                    assert_eq!(s.row(&brought()).at, s.world.utc());
                }),
            ],
        ),
        row(
            "T2",
            vec![
                step(
                    "a failed request",
                    |s| s.fail(1, "IndexRefresh"),
                    vec![req(1)],
                ),
                step(
                    "a second one, later",
                    |s| {
                        s.at(1);
                        s.fail(2, "IndexRebuild");
                    },
                    vec![req(1), req(2)],
                )
                .check(|s| assert_eq!(s.shown(), vec![req(2), req(1)], "newest first")),
                step(
                    "the first is retried: same time, same place",
                    |s| {
                        s.at(2);
                        s.retry(1);
                    },
                    vec![req(2)],
                )
                .check(|s| {
                    assert_eq!(s.row(&req(1)).at, after(0));
                    assert_eq!(s.shown(), vec![req(2), req(1)]);
                }),
            ],
        ),
        row(
            "T3",
            vec![
                step(
                    "two failed requests",
                    |s| {
                        s.fail(1, "IndexRefresh");
                        s.fail(2, "IndexRebuild");
                    },
                    vec![req(1), req(2)],
                ),
                step(
                    "one is dismissed",
                    |s| {
                        s.at(1);
                        s.dismiss(1);
                    },
                    vec![req(2)],
                )
                .check(|s| assert!(!s.has(&req(1)))),
                step(
                    "it runs again: a new key life, now, at the top",
                    |s| {
                        s.at(2);
                        s.retry(1);
                    },
                    vec![req(2)],
                )
                .check(|s| {
                    assert_eq!(s.row(&req(1)).at, after(2));
                    assert_eq!(s.shown()[0], req(1));
                }),
            ],
        ),
        row(
            "T4",
            vec![
                step(
                    "a hold",
                    |s| s.answered(ROOT, locked_by(key_id())),
                    vec![hold()],
                )
                .check(|s| assert_eq!(s.row(&hold()).at, after(0))),
                step(
                    "it is no longer derived",
                    |s| {
                        s.at(1);
                        s.world.schedule_root(ROOT);
                    },
                    vec![],
                )
                .check(|s| assert!(!s.has(&hold()))),
                step(
                    "derived again: a new time",
                    |s| {
                        s.at(2);
                        s.answered(ROOT, locked_by(key_id()));
                    },
                    vec![hold()],
                )
                .check(|s| assert_eq!(s.row(&hold()).at, after(2))),
            ],
        ),
    ]
}

/// T5 to T7, T13: the rows the log keeps (L3, L3b).
fn kept() -> Vec<Row> {
    let checking = |s: &mut super::scene::Scene| {
        s.world.repositories = vec![repository_of(ROOT, K::Checking, None, None)];
        s.world.repositories[0].checking = true;
    };
    let can_open = |s: &mut super::scene::Scene| {
        s.world.repositories = vec![repository_of(
            ROOT,
            K::CantOpen,
            Some(RowAction::Retry),
            None,
        )];
        s.world.repositories[0].checking = true;
    };
    vec![
        row(
            "T5",
            vec![
                step("a Can't reach row", |s| s.world.repositories = vec![cant_reach(ROOT)], vec![repo(ROOT)])
                    .check(|s| {
                        let revision = s.row(&repo(ROOT)).revision;
                        s.remember("revision", revision);
                    }),
                step("its repository checks: no row is derived, the row is kept", move |s| {
                    s.at(1);
                    checking(s);
                }, vec![repo(ROOT)])
                .check(|s| {
                    let kept = s.row(&repo(ROOT));
                    assert_eq!(kept.at, after(0), "its time does not change");
                    let button = kept.row.button.clone().expect("a button");
                    assert_eq!(
                        (button.label.as_str(), button.enabled),
                        ("Checking\u{2026}", false)
                    );
                    assert_eq!(
                        button.action,
                        ActivityAction::Repository(ROOT.to_owned(), RowAction::RetryNow)
                    );
                    assert_eq!(kept.row.menu.len(), 1, "the menu holds the same offers");
                    assert!(kept.row.menu.iter().all(|offer| !offer.enabled), "every offer");
                    assert_ne!(kept.revision, s.recall("revision"), "its offers changed");
                }),
                step("the repository leaves the list: the row leaves with it", |s| {
                    s.at(2);
                    s.world.repositories.clear();
                }, vec![])
                .check(|s| assert!(!s.has(&repo(ROOT)))),
            ],
        ),
        row(
            "T6",
            vec![
                step("a Can't reach row", |s| s.world.repositories = vec![cant_reach(ROOT)], vec![repo(ROOT)]),
                step("it checks, and the registration is damaged: the derived row wins", move |s| {
                    s.at(1);
                    can_open(s);
                }, vec![repo(ROOT)])
                .check(|s| {
                    let row = s.row(&repo(ROOT));
                    assert_eq!(row.row.title, "Can't open demo");
                    let button = row.row.button.clone().expect("a button");
                    assert_eq!((button.label.as_str(), button.enabled), ("Retry", true));
                    assert_eq!(row.at, after(0));
                }),
            ],
        ),
        row(
            "T7",
            vec![
                step("a pending row", |s| s.listed(ROOT, vec![waiting()]), vec![pending_row(ROOT, OPERATION)])
                    .check(|s| {
                        let revision = s.row(&pending_row(ROOT, OPERATION)).revision;
                        s.remember("revision", revision);
                    }),
                step("its list goes Busy: the row is kept", |s| {
                    s.at(1);
                    s.read(ROOT, ReadResult::Busy);
                }, vec![pending_row(ROOT, OPERATION)])
                .check(|s| {
                    let kept = s.row(&pending_row(ROOT, OPERATION));
                    assert_eq!(kept.at, after(0), "the same time");
                    assert!(!kept.row.menu.is_empty());
                    assert!(kept.row.menu.iter().all(|offer| !offer.enabled), "every offer");
                    assert!(kept.row.button.as_ref().is_some_and(|button| !button.enabled));
                    assert_ne!(kept.revision, s.recall("revision"));
                }),
                step("a Ready read: derived again", |s| {
                    s.at(2);
                    s.listed(ROOT, vec![waiting()]);
                }, vec![pending_row(ROOT, OPERATION)])
                .check(|s| {
                    let row = s.row(&pending_row(ROOT, OPERATION));
                    assert_eq!(row.at, after(0));
                    assert!(row.row.menu.iter().any(|offer| offer.enabled));
                }),
                step("a Failed read with an unavailable code removes the list: kept", |s| {
                    s.at(3);
                    s.read(
                        ROOT,
                        ReadResult::Failed(ReadFailure {
                            code: ResultCode::InvalidPath,
                            recovery: Vec::new(),
                        }),
                    );
                }, vec![pending_row(ROOT, OPERATION)])
                .check(|s| {
                    let kept = s.row(&pending_row(ROOT, OPERATION));
                    assert_eq!(kept.at, after(0));
                    assert!(kept.row.menu.iter().all(|offer| !offer.enabled));
                }),
                step("a Failed read S10 counts: the row leaves, the list's row counts", |s| {
                    s.at(4);
                    s.read(
                        ROOT,
                        ReadResult::Failed(ReadFailure {
                            code: ResultCode::InternalError,
                            recovery: Vec::new(),
                        }),
                    );
                }, vec![RowKey::PendingList(ROOT.to_owned())])
                .check(|s| assert!(!s.has(&pending_row(ROOT, OPERATION)))),
                step("read again", |s| {
                    s.at(5);
                    s.listed(ROOT, vec![waiting()]);
                }, vec![pending_row(ROOT, OPERATION)]),
                step("Busy once more, and the repository leaves the list: it leaves with it", |s| {
                    s.at(6);
                    s.read(ROOT, ReadResult::Busy);
                    s.refresh();
                    assert!(s.has(&pending_row(ROOT, OPERATION)), "kept while it is listed");
                    s.world.repositories.clear();
                    s.world.operations.retain_pending(&[]);
                }, vec![])
                .check(|s| assert!(!s.has(&pending_row(ROOT, OPERATION)))),
            ],
        ),
        // T13: A3's counts run over the rows after L1 to L3b, the kept rows
        // included: a kept row still absorbs.
        row(
            "T13",
            vec![
                step(
                    "a Can't reach row, a pending row and a request of its operation",
                    |s| {
                        s.world.repositories = vec![cant_reach(ROOT)];
                        s.fail_op(2, "IndexRefresh", OPERATION);
                        s.listed(ROOT, vec![waiting()]);
                    },
                    sorted(vec![repo(ROOT), pending_row(ROOT, OPERATION)]),
                ),
                step(
                    "the repository checks and the list is busy: both rows are kept, and each absorbs a failure",
                    move |s| {
                        s.at(1);
                        checking(s);
                        s.read(ROOT, ReadResult::Busy);
                            s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                    },
                    sorted(vec![repo(ROOT), pending_row(ROOT, OPERATION)]),
                )
                .check(|s| assert!(s.has(&req(1)) && s.has(&req(2)))),
            ],
        ),
    ]
}
