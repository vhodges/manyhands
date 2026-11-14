//! P2, rows N5 to N12 of A3's table: what a repository's state, its unshared list and its pending list make.

use super::{
    attention::{cant_reach, idle, locked_by, repository_of, sorted, then, waiting},
    scene::{Row, pending_row, repo, req, row},
    states::world::{OPERATION, OTHER, key_id, target},
    support::ROOT,
};
use crate::desktop::model::{
    activity::rows::{RowKey, Target},
    polling::{Answer, Disposition, Reason, TargetKind, TransportFailure, attention::Action},
    repository_state::{AttentionCause as Cause, RowAction, StateKind as K},
};

/// N5 to N12: what a repository's state, its unshared list and its pending
/// list make.
pub(super) fn repositories() -> Vec<Row> {
    let locked = || RowKey::Hold(format!("KeyLocked:{}", key_id()));
    let key = RowKey::Hold(format!("KeyProblem:{}", key_id()));
    vec![
        row(
            "N5",
            vec![
                then(
                    "one hold for the key, however many repositories it holds",
                    vec![locked()],
                    |s| {
                        for root in [ROOT, OTHER, "/fixture/third"] {
                            s.answered(root, locked_by(key_id()));
                        }
                        s.world.repositories = [ROOT, OTHER, "/fixture/third"]
                            .into_iter()
                            .map(|root| {
                                repository_of(
                                    root,
                                    K::Locked,
                                    Some(RowAction::Unlock(key_id())),
                                    None,
                                )
                            })
                            .collect();
                    },
                )
                .check(move |s| {
                    assert_eq!(s.shown(), vec![locked()], "no repository row");
                    assert_eq!(s.row(&locked()).row.group.roots.len(), 3);
                }),
            ],
        ),
        row(
            "N6",
            vec![
                then(
                    "one hold for the reason, and no repository row",
                    vec![key.clone()],
                    |s| {
                        for root in [ROOT, OTHER] {
                            s.answered(
                                root,
                                Answer::Transport(TransportFailure::KeyProblem {
                                    key: Some(key_id()),
                                }),
                            );
                        }
                        let reason = Reason::KeyProblem(key_id());
                        s.world.repositories = [ROOT, OTHER]
                            .into_iter()
                            .map(|root| {
                                repository_of(
                                    root,
                                    K::NeedsAttention,
                                    Some(RowAction::Attend(Action::ChooseKey, reason.clone())),
                                    Some(Cause::Phase(reason.clone())),
                                )
                            })
                            .collect();
                    },
                )
                .check(|s| assert_eq!(s.shown().len(), 1)),
            ],
        ),
        row(
            "N7",
            vec![
                then(
                    "a repository row with Resume checking",
                    vec![repo(ROOT)],
                    |s| {
                        s.world.repositories = vec![repository_of(
                            ROOT,
                            K::NeedsAttention,
                            Some(RowAction::Attend(
                                Action::ResumeChecking,
                                Reason::RecoveryHeld(ROOT.to_owned()),
                            )),
                            Some(Cause::Held),
                        )];
                    },
                )
                .check(|s| {
                    let offer = s.row(&repo(ROOT)).row.button.clone().unwrap();
                    assert_eq!(
                        (offer.label.as_str(), offer.enabled),
                        ("Resume checking", true)
                    );
                }),
            ],
        ),
        row(
            "N8",
            vec![then(
                "each of the seven states and the stored Locked is one row",
                sorted((1..=8).map(|n| repo(&format!("/fixture/r{n}"))).collect()),
                |s| {
                    let attend = |cause, action| (K::NeedsAttention, Some(action), Some(cause));
                    let states = [
                        (K::CantOpen, Some(RowAction::Retry), None),
                        (K::NeedsFixing, Some(RowAction::Settings), None),
                        (K::CantReach, Some(RowAction::RetryNow), None),
                        attend(Cause::WaitingFailed, RowAction::RetryNow),
                        attend(Cause::Unreadable, RowAction::Reread),
                        attend(Cause::Transport, RowAction::RetryNow),
                        attend(Cause::Stored, RowAction::CheckNow),
                        (K::Locked, Some(RowAction::CheckNow), None),
                    ];
                    s.world.repositories = states
                        .into_iter()
                        .enumerate()
                        .map(|(at, (kind, action, cause))| {
                            repository_of(&format!("/fixture/r{}", at + 1), kind, action, cause)
                        })
                        .collect();
                },
            )],
        ),
        // N9: a problem row for each item, none for the repository's own
        // problems; a row for another state stands beside them.
        row(
            "N9",
            vec![
                then(
                    "a stored Locked row beside two problems: three",
                    sorted(vec![repo(ROOT), problem("a"), problem("b")]),
                    |s| {
                        s.world.repositories = vec![repository_of(
                            ROOT,
                            K::Locked,
                            Some(RowAction::CheckNow),
                            None,
                        )];
                        s.world.repositories[0].unshared.problems = two_problems();
                    },
                ),
                then(
                    "the repository's own problems make no row: two",
                    sorted(vec![problem("a"), problem("b")]),
                    |s| {
                        s.world.repositories = vec![repository_of(
                            ROOT,
                            K::NeedsAttention,
                            None,
                            Some(Cause::Problems),
                        )];
                        s.world.repositories[0].unshared.problems = two_problems();
                    },
                ),
            ],
        ),
        // N10: a pending conflict makes no repository row; the conflict's
        // own pending row, or the request that stopped on it, counts.
        row(
            "N10 the pending row",
            vec![
                then(
                    "the pending row offers Review changes, once",
                    vec![pending_row(ROOT, OPERATION)],
                    |s| {
                        s.world.repositories = vec![repository_of(
                            ROOT,
                            K::NeedsAttention,
                            None,
                            Some(Cause::Conflict),
                        )];
                        s.listed(ROOT, vec![waiting()]);
                        s.world.waits.push((ROOT.to_owned(), OPERATION.to_owned()));
                    },
                )
                .check(|s| {
                    let row = s.row(&pending_row(ROOT, OPERATION));
                    assert!(
                        row.row
                            .menu
                            .iter()
                            .any(|offer| offer.label == "Review changes")
                    );
                    assert!(!s.has(&repo(ROOT)), "no repository row");
                }),
            ],
        ),
        row(
            "N10 the request",
            vec![
                then(
                    "the request that stopped on it counts, once",
                    vec![req(1)],
                    |s| {
                        s.world.repositories = vec![repository_of(
                            ROOT,
                            K::NeedsAttention,
                            None,
                            Some(Cause::Conflict),
                        )];
                        s.fail_op(1, "IndexRefresh", OPERATION);
                        s.facts_of(1).review = true;
                    },
                )
                .check(|s| assert!(!s.has(&repo(ROOT)))),
            ],
        ),
        // N11: a repository starting Checking… keeps its row, counted.
        row(
            "N11",
            vec![
                then("a Can't reach row", vec![repo(ROOT)], |s| {
                    s.world.repositories = vec![cant_reach(ROOT)];
                }),
                then(
                    "the check starts: the row stays, still once",
                    vec![repo(ROOT)],
                    |s| {
                        s.world.repositories = vec![repository_of(ROOT, K::Checking, None, None)];
                        s.world.repositories[0].checking = true;
                    },
                )
                .check(|s| {
                    let button = s.row(&repo(ROOT)).row.button.clone().unwrap();
                    assert_eq!(
                        (button.label.as_str(), button.enabled),
                        ("Checking\u{2026}", false)
                    );
                }),
                then("the check answers: the row goes", vec![], |s| {
                    s.world.repositories = vec![repository_of(ROOT, K::UpToDate, None, None)];
                }),
            ],
        ),
        // N12: a pending row with nothing to do does not count.
        row(
            "N12",
            vec![
                then("the row is shown and not counted", vec![], |s| {
                    s.listed(ROOT, vec![idle()]);
                })
                .check(|s| assert!(s.has(&pending_row(ROOT, OPERATION)))),
            ],
        ),
    ]
}

fn problem(which: &str) -> RowKey {
    let item = match which {
        "a" => super::scene::item(),
        _ => "01ARZ3NDEKTSV4RRFFQ69G5FD0".parse().unwrap(),
    };
    RowKey::Problem {
        root: ROOT.to_owned(),
        target: Target::Item(item),
    }
}

fn two_problems() -> Vec<crate::desktop::model::polling::AttemptTarget> {
    vec![
        target(Disposition::Dirty),
        crate::desktop::model::polling::AttemptTarget {
            item_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FD0".parse().unwrap()),
            kind: Some(TargetKind::Document),
            disposition: Disposition::Malformed,
            main: false,
        },
    ]
}
