//! P1 for S7 and S8: the Retire… offers and the retirement events.

use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ContextRetirementState as State, MutationDataDto, RequestId,
        RetirementRemoval as Removal,
    },
    results::{Envelope, Outcome, ResultCode as Code},
};

use super::{
    super::support::{ITEM, ROOT},
    world::{DOCUMENT, DOCUMENT_SHORT, SHORT, Want, World, assert_rows, quiet, want},
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction as Act, Family, Group, Kind, RowKey, Subject},
        polling::{
            TargetKind,
            entry::{RETIRE, RetireOffer},
            retire::Verdict,
        },
    },
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{RecordState, WorkerEvent},
    },
    test_support::retire::{answer, no_data},
};

/// A verdict's case: its name, the record's state, the answer and the event
/// it appends, with whether it changed the copy.
type Answered = (
    &'static str,
    RecordState,
    Envelope<MutationDataDto>,
    Option<(Verdict, bool)>,
);

fn id() -> ItemId {
    ITEM.parse().unwrap()
}

fn document() -> ItemId {
    DOCUMENT.parse().unwrap()
}

/// S7: a Retire… offer for each item whose copy is left; disabled while the
/// item retires; a promoted document reads "is approved".
pub(super) fn retire_offers() {
    let key = |item: ItemId| RowKey::RetireOffer {
        root: ROOT.to_owned(),
        item,
    };
    let retire = |kind, item: ItemId| Act::Retire {
        root: ROOT.to_owned(),
        kind,
        item,
    };
    let cases: Vec<(&str, bool, bool, Vec<Want>)> = vec![
        (
            "a closed ticket and a promoted document",
            true,
            false,
            vec![
                want(
                    key(id()),
                    Kind::Info,
                    "“Plan” is closed, but its copy is still on this computer",
                )
                .repo("demo")
                .button(retire(TargetKind::Ticket, id()), RETIRE, true)
                .item(AuthoringKind::Ticket)
                .group(quiet(&[ROOT])),
                want(
                    key(document()),
                    Kind::Info,
                    "“Plan” is approved, but its copy is still on this computer",
                )
                .repo("demo")
                .button(retire(TargetKind::Document, document()), RETIRE, true)
                .group(quiet(&[ROOT]))
                .item_of(document(), AuthoringKind::Document),
            ],
        ),
        (
            "untitled, and retiring",
            false,
            true,
            vec![
                want(
                    key(id()),
                    Kind::Info,
                    &format!(
                        "The ticket {SHORT} is closed, but its copy is still on this computer"
                    ),
                )
                .repo("demo")
                .button(retire(TargetKind::Ticket, id()), RETIRE, false)
                .item(AuthoringKind::Ticket)
                .group(quiet(&[ROOT])),
                want(
                    key(document()),
                    Kind::Info,
                    &format!(
                        "The document {} is approved, but its copy is still on this computer",
                        DOCUMENT_SHORT
                    ),
                )
                .repo("demo")
                .button(retire(TargetKind::Document, document()), RETIRE, false)
                .group(quiet(&[ROOT]))
                .item_of(document(), AuthoringKind::Document),
            ],
        ),
    ];
    for (name, titled, retiring, wanted) in cases {
        let mut world = World::new();
        world.titled = titled;
        let input = &mut world.repositories[0];
        input.retire_offers = vec![
            RetireOffer {
                kind: TargetKind::Ticket,
                item: id(),
            },
            RetireOffer {
                kind: TargetKind::Document,
                item: document(),
            },
        ];
        if retiring {
            input.retiring = [id(), document()].into_iter().collect();
        }
        assert_rows(&world.rows(), &wanted, &format!("S7 {name}"));
    }
}

fn retire_intent(item: ItemId) -> Intent {
    let kind = if item == document() {
        AuthoringKind::Document
    } else {
        AuthoringKind::Ticket
    };
    Intent::ContextRetire {
        root: ROOT.into(),
        kind,
        id: item,
    }
}

/// Request `n` retires `item`, and the worker answers `envelope`, its
/// record at `record`.
fn retire(
    world: &mut World,
    n: u64,
    item: ItemId,
    record: RecordState,
    envelope: manyhands::results::Envelope<manyhands::repository::MutationDataDto>,
) {
    let id = DesktopRequestId(n);
    world.operations.apply(WorkerEvent::Queued(DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent: retire_intent(item),
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    }));
    world.operations.apply(WorkerEvent::Started(id));
    world.operations.record_stood(id, record);
    world
        .operations
        .apply(WorkerEvent::Finished(id, Box::new(envelope)));
}

/// S8: every verdict through `Operations::apply(Finished)`. Retired,
/// Attention, Unreachable and Retry each append one event with its sequence
/// number and its request's queue order, and their requests are dismissed;
/// Owed, refused, preserved and failed append none and keep their S1 row.
/// Nothing is read from the schedule's retirement lines (R-51, R-59).
pub(super) fn retire_events() {
    use RecordState::{Accepted, Finished};
    let refused = |code| answer(State::Refused, code, Removal::None);
    let rows: [Answered; 9] = [
        (
            "retired",
            Finished,
            answer(State::Retired, Code::Ok, Removal::Observed),
            Some((Verdict::Retired, true)),
        ),
        (
            "attention",
            Finished,
            refused(Code::UnlockRequired),
            Some((
                Verdict::Attention(crate::desktop::model::polling::retire::Need::KeyLocked),
                false,
            )),
        ),
        (
            "unreachable",
            Finished,
            refused(Code::TransportUnavailable),
            Some((Verdict::Unreachable, false)),
        ),
        (
            "retry",
            Finished,
            refused(Code::IndexUnavailable),
            Some((
                Verdict::Retry {
                    recovery_required: false,
                },
                false,
            )),
        ),
        (
            "owed, pending",
            Finished,
            answer(State::Pending, Code::DiscoveryPending, Removal::None),
            None,
        ),
        (
            "owed, left accepted",
            Accepted,
            answer(State::Retired, Code::Ok, Removal::None),
            None,
        ),
        ("refused", Finished, refused(Code::WorktreeNotClean), None),
        (
            "preserved",
            Finished,
            answer(State::Preserved, Code::Ok, Removal::None),
            None,
        ),
        ("failed", Finished, no_data(Code::InternalError), None),
    ];
    for (name, record, envelope, event) in rows {
        let mut world = World::new();
        retire(&mut world, 1, id(), record, envelope);
        let events = world.operations.retire_events();
        match event {
            Some((verdict, changed)) => {
                assert_eq!(events.len(), 1, "{name}: one event");
                let kept = &events[0];
                assert_eq!(
                    (
                        kept.seq,
                        kept.sent,
                        kept.kind,
                        &kept.item,
                        kept.verdict,
                        kept.changed
                    ),
                    (0, 1, AuthoringKind::Ticket, &id(), verdict, changed),
                    "{name}: its event"
                );
                assert_eq!(kept.root, ROOT, "{name}: its root");
                assert!(
                    world
                        .operations
                        .request(DesktopRequestId(1))
                        .is_some_and(|request| matches!(request.state, RequestState::Dismissed)),
                    "{name}: dismissed on its answer"
                );
            }
            None => assert!(events.is_empty(), "{name}: no event"),
        }
        if event.is_none() {
            assert!(
                world.rows().is_empty(),
                "{name}: the request's own row says it, not a state row"
            );
        }
    }

    // The rows: a Retired event is a success row keyed by its sequence number,
    // a later retirement of the same item a new one; an information event is
    // the item's one note, replaced by a later one, with its request's queue
    // order to tell them apart.
    let mut world = World::new();
    retire(
        &mut world,
        1,
        id(),
        Finished,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    retire(&mut world, 2, id(), Finished, refused(Code::UnlockRequired));
    retire(
        &mut world,
        3,
        id(),
        Finished,
        refused(Code::IndexUnavailable),
    );
    retire(
        &mut world,
        4,
        document(),
        Finished,
        answer(State::AlreadyAbsent, Code::AlreadyApplied, Removal::None),
    );
    retire(
        &mut world,
        5,
        id(),
        Finished,
        refused(Code::TransportUnavailable),
    );
    let sequence: Vec<(u64, u64)> = world
        .operations
        .retire_events()
        .iter()
        .map(|event| (event.seq, event.sent))
        .collect();
    assert_eq!(sequence, [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);

    let retired = |seq: u64, sent: u64, item: ItemId, name: &str| {
        want(
            RowKey::Retired(seq),
            Kind::Success,
            &format!("Retired the copy of {name} on this computer"),
        )
        .repo("demo")
        .then(Act::Dismiss(RowKey::Retired(seq)), "Dismiss", true)
        .clearable()
        .group(Group {
            subject: Some(Subject::Intent {
                family: Family::Retire,
                root: Some(ROOT.to_owned()),
                item: Some(item),
                target: None,
            }),
            sent: Some(sent),
            outcome: Some(Outcome::Success),
            ..Group::default()
        })
    };
    let note = |item: ItemId, sent: u64, name: &str| {
        let key = RowKey::RetireNote {
            root: ROOT.to_owned(),
            item,
        };
        want(
            key.clone(),
            Kind::Info,
            &format!("Retiring {name} didn't finish"),
        )
        .desc("It is tried again at the next check.")
        .repo("demo")
        .then(Act::Dismiss(key), "Dismiss", true)
        .clearable()
        .group(Group {
            sent: Some(sent),
            ..Group::default()
        })
    };
    world.titled = false;
    let ticket = format!("the ticket {SHORT}");
    let wanted = [
        retired(0, 1, id(), &ticket),
        note(id(), 5, &ticket),
        retired(
            3,
            4,
            document(),
            &format!("the document {}", DOCUMENT_SHORT),
        ),
    ];
    // The note is the item's latest; the rows come in the events' order,
    // each note at its latest event's place.
    let rows = world.rows();
    let keys: Vec<RowKey> = rows.iter().map(|row| row.key.clone()).collect();
    assert_eq!(
        keys,
        [
            RowKey::Retired(0),
            RowKey::Retired(3),
            RowKey::RetireNote {
                root: ROOT.to_owned(),
                item: id()
            },
        ],
        "S8 rows: retired events in order, then each item's latest note"
    );
    let ordered = [&wanted[0], &wanted[2], &wanted[1]];
    for (row, want) in rows.iter().zip(ordered) {
        want.holds(row, "S8");
    }
}
