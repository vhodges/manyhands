//! The retirement loop's rules, RT-1 to RT-11 (D6 Task 11b), over the pure
//! model: no worker, no draft store, no thread.

use manyhands::{canonical::ItemId, repository::RequestId};

use super::*;
use crate::desktop::model::polling::{AttemptTarget, Disposition};

const IDS: [&str; 4] = [
    "01ARZ3NDEKTSV4RRFFQ69G5FA1",
    "01ARZ3NDEKTSV4RRFFQ69G5FA2",
    "01ARZ3NDEKTSV4RRFFQ69G5FA3",
    "01ARZ3NDEKTSV4RRFFQ69G5FA4",
];

fn item(n: usize) -> ItemId {
    IDS[n].parse().unwrap()
}

fn candidate(n: usize) -> Candidate {
    Candidate {
        kind: TargetKind::Ticket,
        item: item(n),
    }
}

fn clear() -> Facts {
    Facts {
        retired: false,
        in_flight: false,
        refused: false,
        busy: false,
        drafts: Held::No,
    }
}

fn root() -> Root {
    "/repo".to_owned()
}

/// How the request of a submitted item ended.
#[derive(Clone, Copy, Debug)]
enum Reply {
    /// Answered; `goes_on` is the verdict's (RT-7, RT-8).
    Answered(bool),
    /// Removed from the queue or never sent (RT-9).
    Gone,
}

/// One reported item: what the first check reads, and what the fresh
/// listing reads after the item is marked (RT-6).
struct Reported {
    first: Facts,
    fresh: Facts,
    reply: Reply,
}

fn reported(first: Facts) -> Reported {
    Reported {
        first,
        fresh: first,
        reply: Reply::Answered(true),
    }
}

/// How a loop ended.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Exhausted,
    Stopped,
}

/// Runs a loop over `items` the way the scheduler drives it, and says which
/// it submitted, which it skipped and why, and how it ended.
fn run(items: &[Reported]) -> (Vec<usize>, Vec<(usize, Skip)>, Ended) {
    let at = |candidate: &Candidate| {
        IDS.iter()
            .position(|id| *id == candidate.item.to_string())
            .expect("a known item")
    };
    let mut loops = RetireLoops::default();
    let all: Vec<Candidate> = (0..items.len()).map(candidate).collect();
    assert!(loops.begin(&root(), all).is_none());
    let lp = loops.get_mut(&root()).unwrap();
    let mut submitted = Vec::new();
    let ended = loop {
        let next = lp.next(|candidate| items[at(candidate)].first);
        let Next::Check(checking) = next else {
            break Ended::Exhausted;
        };
        assert_eq!(lp.checking(), Some(&checking), "the item in hand");
        let n = at(&checking);
        match lp.recheck(items[n].fresh) {
            Rechecked::Skipped(_) => continue,
            Rechecked::Nothing => panic!("an item was being checked"),
            Rechecked::Submit(sent) => {
                assert_eq!(sent, checking);
                let request = RequestId::new();
                lp.submitted(request);
                assert_eq!(lp.running(), Some(request));
                submitted.push(n);
                match items[n].reply {
                    Reply::Answered(goes_on) => match lp.answered(request, goes_on) {
                        Settled::Next => {}
                        Settled::Stop => break Ended::Stopped,
                        Settled::Unrelated => panic!("its own answer"),
                    },
                    Reply::Gone => {
                        if lp.gone(request) {
                            break Ended::Stopped;
                        }
                        panic!("its own request");
                    }
                }
            }
        }
    };
    let skipped = lp
        .skipped()
        .iter()
        .map(|(candidate, skip)| (at(candidate), *skip))
        .collect();
    (submitted, skipped, ended)
}

/// RT-1 to RT-5: each rule holds an item back, in that order, and only its
/// own item.
fn rules() {
    let cases = [
        (
            "RT-1 a retirement of it is queued, running or pending",
            Facts {
                in_flight: true,
                ..clear()
            },
            Some(Skip::InFlight),
        ),
        (
            "RT-2 it is in the refused set",
            Facts {
                refused: true,
                ..clear()
            },
            Some(Skip::Refused),
        ),
        (
            "RT-3 it is being edited",
            Facts {
                busy: true,
                ..clear()
            },
            Some(Skip::Busy),
        ),
        (
            "RT-4 the store holds a draft of it",
            Facts {
                drafts: Held::Yes,
                ..clear()
            },
            Some(Skip::Draft),
        ),
        (
            "RT-5 the listing is unknown",
            Facts {
                drafts: Held::Unknown,
                ..clear()
            },
            Some(Skip::Unknown),
        ),
        (
            "an earlier loop or a Retire… already retired it",
            Facts {
                retired: true,
                ..clear()
            },
            Some(Skip::Retired),
        ),
        (
            "retired before RT-1: not asked twice",
            Facts {
                retired: true,
                in_flight: true,
                refused: true,
                busy: true,
                drafts: Held::Yes,
            },
            Some(Skip::Retired),
        ),
        ("none of them", clear(), None),
        (
            "RT-1 before RT-2",
            Facts {
                in_flight: true,
                refused: true,
                busy: true,
                drafts: Held::Yes,
                ..clear()
            },
            Some(Skip::InFlight),
        ),
        (
            "RT-3 before RT-4",
            Facts {
                busy: true,
                drafts: Held::Unknown,
                ..clear()
            },
            Some(Skip::Busy),
        ),
        (
            "RT-4 before RT-5 only by the draft: a draft is a draft",
            Facts {
                drafts: Held::Yes,
                ..clear()
            },
            Some(Skip::Draft),
        ),
    ];
    for (what, facts, skip) in cases {
        assert_eq!(facts.skip(), skip, "{what}");
        let (submitted, skipped, ended) = run(&[reported(facts)]);
        match skip {
            Some(skip) => {
                assert!(submitted.is_empty(), "{what}: submitted");
                assert_eq!(skipped, vec![(0, skip)], "{what}");
            }
            None => {
                assert_eq!(submitted, vec![0], "{what}: submitted");
                assert!(skipped.is_empty(), "{what}");
            }
        }
        assert_eq!(ended, Ended::Exhausted, "{what}: a skip does not stop");
    }
}

/// What a whole run was given, and what it came to.
type Case = (
    &'static str,
    Vec<Reported>,
    Vec<usize>,
    Vec<(usize, Skip)>,
    Ended,
);

/// The loops of a table of whole runs.
fn runs() {
    let edited = Facts {
        busy: true,
        ..clear()
    };
    let drafted = Facts {
        drafts: Held::Yes,
        ..clear()
    };
    let unknown = Facts {
        drafts: Held::Unknown,
        ..clear()
    };
    let answered = |goes_on| Reply::Answered(goes_on);
    let item = |first: Facts, reply| Reported {
        first,
        fresh: first,
        reply,
    };
    // (what, items, submitted, skipped, ended)
    let cases: Vec<Case> = vec![
        (
            "four reported items, one edited, one with a draft, one while the listing is unknown: only the fourth",
            vec![
                item(edited, answered(true)),
                item(drafted, answered(true)),
                item(unknown, answered(true)),
                item(clear(), answered(true)),
            ],
            vec![3],
            vec![(0, Skip::Busy), (1, Skip::Draft), (2, Skip::Unknown)],
            Ended::Exhausted,
        ),
        (
            "RT-6 a draft stored after the first check stops the item, and the loop goes on",
            vec![
                Reported {
                    first: clear(),
                    fresh: drafted,
                    reply: answered(true),
                },
                Reported {
                    first: clear(),
                    fresh: Facts {
                        drafts: Held::Unknown,
                        ..clear()
                    },
                    reply: answered(true),
                },
                item(clear(), answered(true)),
            ],
            vec![2],
            vec![(0, Skip::Draft), (1, Skip::Unknown)],
            Ended::Exhausted,
        ),
        (
            "RT-6 an edit begun after the first check stops the item",
            vec![
                Reported {
                    first: clear(),
                    fresh: edited,
                    reply: answered(true),
                },
                item(clear(), answered(true)),
            ],
            vec![1],
            vec![(0, Skip::Busy)],
            Ended::Exhausted,
        ),
        (
            "RT-7 retired, refused and retried answers go on to every item",
            vec![
                item(clear(), answered(true)),
                item(clear(), answered(true)),
                item(clear(), answered(true)),
                item(clear(), answered(true)),
            ],
            vec![0, 1, 2, 3],
            vec![],
            Ended::Exhausted,
        ),
        (
            "RT-8 a pending, attention, transport or no-data answer stops the loop",
            vec![
                item(clear(), answered(true)),
                item(clear(), answered(false)),
                item(clear(), answered(true)),
                item(clear(), answered(true)),
            ],
            vec![0, 1],
            vec![],
            Ended::Stopped,
        ),
        (
            "RT-9 a request removed from the queue or never sent stops the loop",
            vec![
                item(clear(), answered(true)),
                item(clear(), Reply::Gone),
                item(clear(), answered(true)),
            ],
            vec![0, 1],
            vec![],
            Ended::Stopped,
        ),
        (
            "an item skipped before the wall is still skipped, and none after it is reached",
            vec![
                item(edited, answered(true)),
                item(clear(), answered(false)),
                item(drafted, answered(true)),
            ],
            vec![1],
            vec![(0, Skip::Busy)],
            Ended::Stopped,
        ),
        ("no item reported", vec![], vec![], vec![], Ended::Exhausted),
    ];
    for (what, items, submitted, skipped, ended) in cases {
        let seen = run(&items);
        assert_eq!(seen.0, submitted, "{what}: submitted");
        assert_eq!(seen.1, skipped, "{what}: skipped");
        assert_eq!(seen.2, ended, "{what}: ended");
    }
}

/// The items an attempt reported make one candidate each, in report order:
/// a duplicate target is checked once, and a target with no ID or kind is
/// none.
fn candidates() {
    let target = |n: usize, kind| AttemptTarget {
        item_id: Some(item(n)),
        kind,
        disposition: Disposition::Closed,
        main: false,
    };
    let targets = vec![
        target(2, Some(TargetKind::Ticket)),
        target(0, Some(TargetKind::Ticket)),
        target(2, Some(TargetKind::Ticket)),
        target(1, None),
        AttemptTarget {
            item_id: None,
            kind: Some(TargetKind::Document),
            disposition: Disposition::Promoted,
            main: false,
        },
        target(2, Some(TargetKind::Document)),
    ];
    assert_eq!(
        Candidate::of(&targets),
        vec![
            candidate(2),
            candidate(0),
            Candidate {
                kind: TargetKind::Document,
                item: item(2)
            }
        ],
        "once each, in report order; a ticket and a document of one ID are two items"
    );
}

/// RT-10, RT-11 and the loop's identity: an answer from a request the loop
/// did not send neither continues it nor un-drops it, and one that hits a
/// wall stops it.
fn late_answers() {
    let mut loops = RetireLoops::default();
    // A loop running its first item.
    let first = RequestId::new();
    {
        loops.begin(&root(), vec![candidate(0), candidate(1)]);
        let lp = loops.get_mut(&root()).unwrap();
        assert!(matches!(lp.next(|_| clear()), Next::Check(_)));
        assert!(matches!(lp.recheck(clear()), Rechecked::Submit(_)));
        lp.submitted(first);
    }
    let old = loops.get(&root()).unwrap().generation();
    // RT-11: a Check now's attempt replaces the loop's remaining items.
    let second = RequestId::new();
    let replaced = loops
        .begin(&root(), vec![candidate(2), candidate(3)])
        .expect("the loop it replaced");
    {
        let lp = loops.get_mut(&root()).unwrap();
        assert_ne!(lp.generation(), old, "another loop");
        assert!(matches!(lp.next(|_| clear()), Next::Check(_)));
        assert!(matches!(lp.recheck(clear()), Rechecked::Submit(_)));
        lp.submitted(second);
    }
    assert_eq!(replaced.running(), Some(first), "its running item goes on");
    let lp = loops.get_mut(&root()).unwrap();
    // The replaced loop's item answers, retired: the new loop is not
    // advanced by it, and does not lose its own item.
    assert_eq!(lp.answered(first, true), Settled::Unrelated);
    assert_eq!(lp.running(), Some(second));
    // It answers again as a wall would: the repository's loop stops.
    assert_eq!(lp.answered(first, false), Settled::Stop);
    // A request the loop never sent ended unsent: that is not its end.
    assert!(!lp.gone(RequestId::new()));
    assert_eq!(lp.running(), Some(second), "still waiting on its own");
    // RT-10: the loop is dropped whole. Its running item may still answer;
    // there is no loop left to continue or to un-drop.
    assert!(loops.end(&root()).is_some());
    assert!(loops.get_mut(&root()).is_none());
    assert!(loops.end(&root()).is_none());
    assert!(loops.roots().is_empty());
}

/// RT-1 to RT-11, over the pure model.
#[test]
fn retire_loop_table() {
    rules();
    runs();
    candidates();
    late_answers();
}
