//! P1 for S9 to S14: pending operations and their lists, and the draft
//! and record rows.

use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ItemDtoKind, OperationAction as Action, OperationDto,
        OperationFamily as Family, OperationId, OperationNextAction as Next, ProgressStage,
    },
};

use super::{
    super::support::{ITEM, ROOT, queue, stage, start},
    world::{OPERATION, Want, World, assert_rows, operation, want},
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction as Act, Group, Kind, RowKey},
        operations::Entered,
        polling::TargetKind,
    },
    services::requests::Intent,
};

fn item() -> ItemId {
    ITEM.parse().unwrap()
}

fn key(operation: &str) -> RowKey {
    RowKey::Pending {
        root: ROOT.to_owned(),
        operation: operation.to_owned(),
    }
}

/// A pending operation's group: it counts when the row has a next action.
fn pending_group(candidate: bool, operation: Option<&str>) -> Group {
    Group {
        candidate,
        operation: operation.map(str::to_owned),
        roots: vec![ROOT.to_owned()],
        ..Group::default()
    }
}

/// The raw lines of a pending row's tooltip: its action, state, step,
/// operation, next action and time.
fn raw(action: &str, state: &str, next: &str) -> Vec<String> {
    vec![
        format!("Action: {action}"),
        format!("State: {state}"),
        format!("Operation {OPERATION}"),
        format!("Next action: {next}"),
    ]
}

fn lines(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).collect()
}

fn resume() -> Act {
    Act::Resume {
        root: ROOT.to_owned(),
        operation: OPERATION.to_owned(),
    }
}

fn abandon() -> Act {
    Act::Abandon {
        root: ROOT.to_owned(),
        operation: OPERATION.to_owned(),
    }
}

fn review() -> Act {
    Act::ReviewPending {
        root: ROOT.to_owned(),
        operation: OPERATION.to_owned(),
    }
}

fn lifecycle(kind: ItemDtoKind) -> Act {
    Act::ResumeLifecycle {
        root: ROOT.to_owned(),
        kind,
        item: ITEM.to_owned(),
        operation: OPERATION.to_owned(),
    }
}

/// One S9 case: what is pending, what else is true, and the rows.
struct Case {
    name: &'static str,
    pending: Vec<OperationDto>,
    waits: bool,
    running: bool,
    titled: bool,
    wanted: Vec<Want>,
}

fn case(name: &'static str, pending: Vec<OperationDto>, wanted: Vec<Want>) -> Case {
    Case {
        name,
        pending,
        waits: false,
        running: false,
        titled: true,
        wanted,
    }
}

/// S9: Resume when offered; Review changes when a conflict waits; a
/// lifecycle's parent resumes through its dialog, or is still running (N25);
/// Abandon only from the menu; a row with no next action counts for nothing.
pub(super) fn pending_rows() {
    let local = |action| operation(action, Family::Local);
    let remote = |action| operation(action, Family::Remote);
    let with_next = |mut dto: OperationDto, next: Option<Next>| {
        dto.next_action = next;
        dto
    };
    let on_item = |mut dto: OperationDto| {
        dto.item_id = Some(ITEM.to_owned());
        dto
    };
    let lifecycle_row = |title: &str, kind: ItemDtoKind, authoring: AuthoringKind| {
        want(key(OPERATION), Kind::Failure, title)
            .desc("Resume finishes it.")
            .repo("demo")
            .item(authoring)
            .detail(&lines(&raw(
                if kind == ItemDtoKind::Ticket {
                    "close"
                } else {
                    "promote"
                },
                "prepared",
                "resume",
            )))
            .group(pending_group(true, Some(OPERATION)))
    };
    let mut cases = vec![
        case(
            "a local operation to resume",
            vec![local(Action::Refresh)],
            vec![
                want(
                    key(OPERATION),
                    Kind::Failure,
                    "Refreshing didn't finish in demo",
                )
                .desc("Resume finishes it.")
                .repo("demo")
                .button(resume(), "Resume", true)
                .then(abandon(), "Abandon", true)
                .detail(&lines(&raw("refresh", "prepared", "resume")))
                .group(pending_group(true, Some(OPERATION))),
            ],
        ),
        Case {
            waits: true,
            ..case(
                "a conflict waits beside Resume",
                vec![local(Action::Refresh)],
                vec![
                    want(
                        key(OPERATION),
                        Kind::Failure,
                        "Refreshing didn't finish in demo",
                    )
                    .desc("Resume finishes it.")
                    .repo("demo")
                    .button(resume(), "Resume", true)
                    .then(review(), "Review changes", true)
                    .then(abandon(), "Abandon", true)
                    .detail(&lines(&raw("refresh", "prepared", "resume")))
                    .group(pending_group(true, Some(OPERATION))),
                ],
            )
        },
        Case {
            waits: true,
            ..case(
                "Review changes is the button when Resume is not offered",
                vec![with_next(
                    local(Action::Refresh),
                    Some(Next::ReviewDeletionAgain),
                )],
                vec![
                    want(
                        key(OPERATION),
                        Kind::Failure,
                        "Refreshing didn't finish in demo",
                    )
                    .desc("Review it again.")
                    .repo("demo")
                    .button(review(), "Review changes", true)
                    .then(abandon(), "Abandon", true)
                    .detail(&lines(&raw("refresh", "prepared", "review_deletion_again")))
                    .group(pending_group(true, Some(OPERATION))),
                ],
            )
        },
        case(
            "a synchronization Resume cannot take up",
            vec![{
                let mut dto = remote(Action::Poll);
                dto.state = "interrupted".to_owned();
                dto
            }],
            vec![
                want(
                    key(OPERATION),
                    Kind::Failure,
                    "Checking for updates didn't finish in demo",
                )
                .desc("Resume finishes it.")
                .repo("demo")
                .button(resume(), "Resume", false)
                .detail(&[
                    "Action: poll",
                    "State: interrupted",
                    &format!("Operation {OPERATION}"),
                    "Next action: resume",
                    "Synchronization is not available yet",
                ])
                .group(pending_group(true, Some(OPERATION))),
            ],
        ),
        case(
            "a synchronization of main",
            vec![remote(Action::SynchronizePrimary)],
            vec![
                want(
                    key(OPERATION),
                    Kind::Failure,
                    "Sharing trunk didn't finish in demo",
                )
                .desc("Resume finishes it.")
                .repo("demo")
                .button(resume(), "Resume", true)
                .detail(&lines(&raw("synchronize_primary", "prepared", "resume")))
                .group(pending_group(true, Some(OPERATION))),
            ],
        ),
        case(
            "nothing to do",
            vec![with_next(local(Action::Refresh), None)],
            vec![
                want(
                    key(OPERATION),
                    Kind::Info,
                    "Refreshing didn't finish in demo",
                )
                .desc("Nothing to do.")
                .repo("demo")
                .then(abandon(), "Abandon", true)
                .detail(&lines(&raw("refresh", "prepared", "none")))
                .group(pending_group(false, Some(OPERATION))),
            ],
        ),
        case(
            "no operation ID",
            vec![{
                let mut dto = local(Action::Refresh);
                dto.operation_id = None;
                dto
            }],
            vec![
                want(key("#0"), Kind::Failure, "Refreshing didn't finish in demo")
                    .desc("Resume finishes it.")
                    .repo("demo")
                    .detail(&[
                        "Action: refresh",
                        "State: prepared",
                        "No operation ID",
                        "Next action: resume",
                    ])
                    .group(pending_group(true, None)),
            ],
        ),
        case(
            "an item known by its title",
            vec![on_item(local(Action::SaveTicket))],
            vec![
                want(
                    key(OPERATION),
                    Kind::Failure,
                    "Saving a ticket “Plan” didn't finish",
                )
                .desc("Resume finishes it.")
                .repo("demo")
                .button(resume(), "Resume", true)
                .then(abandon(), "Abandon", true)
                .item(AuthoringKind::Ticket)
                .detail(&lines(&raw("save_ticket", "prepared", "resume")))
                .group(pending_group(true, Some(OPERATION))),
            ],
        ),
        case(
            "a closure's parent",
            vec![on_item(remote(Action::Close))],
            vec![
                lifecycle_row(
                    "Closing “Plan” didn't finish",
                    ItemDtoKind::Ticket,
                    AuthoringKind::Ticket,
                )
                .button(lifecycle(ItemDtoKind::Ticket), "Resume", true),
            ],
        ),
        case(
            "a promotion's parent",
            vec![on_item(remote(Action::Promote))],
            vec![
                lifecycle_row(
                    "Approving and merging “Plan” didn't finish",
                    ItemDtoKind::Document,
                    AuthoringKind::Document,
                )
                .button(lifecycle(ItemDtoKind::Document), "Resume", true),
            ],
        ),
        Case {
            running: true,
            ..case(
                "a closure still running elsewhere",
                vec![on_item(remote(Action::Close))],
                vec![
                    lifecycle_row(
                        "Closing “Plan” didn't finish",
                        ItemDtoKind::Ticket,
                        AuthoringKind::Ticket,
                    )
                    .button(lifecycle(ItemDtoKind::Ticket), "Still running…", false)
                    .group(pending_group(false, Some(OPERATION))),
                ],
            )
        },
    ];
    cases.push(Case {
        titled: false,
        ..case(
            "an item with no title",
            vec![on_item(local(Action::SaveTicket))],
            vec![
                want(
                    key(OPERATION),
                    Kind::Failure,
                    "Saving a ticket didn't finish in demo",
                )
                .desc("Resume finishes it.")
                .repo("demo")
                .button(resume(), "Resume", true)
                .then(abandon(), "Abandon", true)
                .detail(&lines(&raw("save_ticket", "prepared", "resume")))
                .group(pending_group(true, Some(OPERATION))),
            ],
        )
    });
    for case in cases {
        let mut world = World::new();
        world.titled = case.titled;
        if case.waits {
            world.waits.push((ROOT.to_owned(), OPERATION.to_owned()));
        }
        if case.running {
            world.running.push((ROOT.to_owned(), item()));
        }
        world.pending(ROOT, case.pending);
        assert_rows(&world.rows(), &case.wanted, &format!("S9 {}", case.name));
    }

    // A retirement's row is offered Resume only once the in-flight record
    // holds its entry (K-27); an automatic poll's row never is.
    let retirement = || {
        let mut dto = local(Action::RetireContext);
        dto.item_id = Some(ITEM.to_owned());
        dto
    };
    let mut world = World::new();
    world.pending(ROOT, vec![retirement()]);
    let rows = world.rows();
    assert_eq!(
        rows[0]
            .menu
            .iter()
            .map(|offer| offer.label.as_str())
            .collect::<Vec<_>>(),
        ["Abandon"],
        "S9 a retirement with no entry: Abandon only"
    );
    world.operations.set_retirement_entries(vec![Entered {
        root: ROOT.to_owned(),
        kind: TargetKind::Ticket,
        item: item(),
        request: None,
        operation: Some(OperationId::parse(OPERATION).unwrap()),
    }]);
    let rows = world.rows();
    assert_eq!(
        rows[0]
            .menu
            .iter()
            .map(|offer| offer.label.as_str())
            .collect::<Vec<_>>(),
        ["Resume", "Abandon"],
        "S9 a retirement the record holds"
    );
    let mut world = World::new();
    let mut poll = remote(Action::Poll);
    poll.next_action = None;
    poll.state = "reserved".to_owned();
    world
        .operations
        .set_automatic_polls([OperationId::parse(OPERATION).unwrap()].into());
    world.pending(ROOT, vec![poll]);
    let rows = world.rows();
    assert_eq!(
        (
            rows[0].button.clone(),
            rows[0].kind,
            rows[0].group.candidate
        ),
        (None, Kind::Info, false)
    );

    // The worker's own running operation is no pending row (N21).
    let mut world = World::new();
    queue(
        &mut world.operations,
        1,
        Intent::OperationResume {
            root: ROOT.into(),
            operation_id: OperationId::parse(OPERATION).unwrap(),
        },
    );
    start(&mut world.operations, 1);
    stage(
        &mut world.operations,
        1,
        ProgressStage::Indexing,
        Some(OPERATION),
    );
    world.pending(ROOT, vec![local(Action::Refresh)]);
    assert!(
        world.rows().is_empty(),
        "S9 a running operation is not pending"
    );

    // Two repositories: each repository's rows, by root; a row with no ID is
    // numbered by its place in its repository's list.
    let mut world = World::new();
    let anonymous = || {
        let mut dto = local(Action::Rebuild);
        dto.operation_id = None;
        dto
    };
    world.pending(ROOT, vec![local(Action::Refresh), anonymous(), anonymous()]);
    let keys: Vec<RowKey> = world.rows().into_iter().map(|row| row.key).collect();
    assert_eq!(keys, [key(OPERATION), key("#1"), key("#2")]);
}
