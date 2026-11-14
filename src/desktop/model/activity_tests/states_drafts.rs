//! P1 for S10 to S14: a pending list that could not be read, and the draft
//! and record rows.

use manyhands::{
    canonical::ItemId,
    drafts::{DraftErrorKind, DraftId},
    repository::AuthoringKind,
    results::ResultCode,
};

use super::{
    super::support::{ITEM, ROOT},
    world::{Want, World, assert_rows, counted, quiet, want},
};
use crate::desktop::{
    model::activity::rows::{ActivityAction as Act, Group, ItemRef, Kind, RowKey},
    services::{DraftStoreUnavailable, ReadFailure, ReadResult, inflight::Report},
};

fn item() -> ItemId {
    ITEM.parse().unwrap()
}

/// S10: a pending list that is Failed or Busy; Loading, Ready and a list
/// removed by an unavailable code make none.
pub(super) fn pending_lists() {
    let key = RowKey::PendingList(ROOT.to_owned());
    let retry = Act::RetryPendingList(ROOT.to_owned());
    for (name, read, row) in [
        (
            "failed",
            Some(ReadResult::Failed(ReadFailure {
                code: ResultCode::InternalError,
                recovery: Vec::new(),
            })),
            Some(
                want(
                    key.clone(),
                    Kind::Failure,
                    "Couldn't read the unfinished operations of demo",
                )
                .repo("demo")
                .button(retry.clone(), "Retry", true)
                .group(counted(&[ROOT])),
            ),
        ),
        (
            "busy",
            Some(ReadResult::Busy),
            Some(
                want(
                    key.clone(),
                    Kind::Info,
                    "Manyhands was busy reading the unfinished operations of demo",
                )
                .repo("demo")
                .button(retry, "Retry", true)
                .group(quiet(&[ROOT])),
            ),
        ),
        (
            "unavailable",
            Some(ReadResult::Failed(ReadFailure {
                code: ResultCode::NotRepository,
                recovery: Vec::new(),
            })),
            None,
        ),
        ("loading", None, None),
    ] {
        let mut world = World::new();
        let stamp = world.operations.load_pending(ROOT.to_owned());
        if let Some(read) = read {
            assert!(world.operations.apply_pending(stamp, read));
        }
        let wanted: Vec<Want> = row.into_iter().collect();
        assert_rows(&world.rows(), &wanted, &format!("S10 {name}"));
    }
    let mut world = World::new();
    world.pending(ROOT, Vec::new());
    assert!(world.rows().is_empty(), "S10 ready, no rows");
}

/// S11 to S13: a recovery copy not saved, the draft store unavailable and
/// the drafts recovered from an earlier session.
pub(super) fn drafts() {
    let draft = DraftId::generate();
    let other = DraftId::generate();
    let flush = |draft: DraftId, kind, id: &str, title: &str, reason: DraftErrorKind| {
        let item = ItemRef {
            root: ROOT.to_owned(),
            kind,
            id: id.parse().unwrap(),
        };
        (draft, item, title.to_owned(), reason)
    };
    let mut world = World::new();
    world.drafts.flush_failures = vec![
        flush(
            draft,
            AuthoringKind::Ticket,
            ITEM,
            "Plan",
            DraftErrorKind::NoSpace,
        ),
        flush(other, AuthoringKind::Ticket, ITEM, ITEM, DraftErrorKind::Io),
        flush(
            DraftId::generate(),
            AuthoringKind::Document,
            "01ARZ3NDEKTSV4RRFFQ69G5FD0",
            "01ARZ3NDEKTSV4RRFFQ69G5FD0",
            DraftErrorKind::Access,
        ),
    ];
    let third = world.drafts.flush_failures[2].0;
    let failure = |id: DraftId, title: &str, description: &str, item: ItemRef| {
        want(RowKey::Flush(id), Kind::Failure, title)
            .desc(description)
            .repo("demo")
            .button(Act::RetryFlush(id), "Retry", true)
            .detail(&["Recovery copy"])
            .group(counted(&[ROOT]))
            .item_of(item.id, item.kind)
    };
    let ticket = || ItemRef {
        root: ROOT.to_owned(),
        kind: AuthoringKind::Ticket,
        id: item(),
    };
    let wanted = [
        failure(
            draft,
            "The recovery copy of “Plan” wasn't saved",
            "There is no space left to keep the draft.",
            ticket(),
        ),
        failure(
            other,
            "The recovery copy of the ticket Q69G5FG0 wasn't saved",
            "The draft could not be read or written.",
            ticket(),
        ),
        failure(
            third,
            "The recovery copy of the document Q69G5FD0 wasn't saved",
            "Access to the drafts folder was denied.",
            ItemRef {
                root: ROOT.to_owned(),
                kind: AuthoringKind::Document,
                id: "01ARZ3NDEKTSV4RRFFQ69G5FD0".parse().unwrap(),
            },
        ),
    ];
    assert_rows(&world.rows(), &wanted, "S11");

    let mut world = World::new();
    world.drafts.unavailable = Some(DraftStoreUnavailable(DraftErrorKind::Access));
    world.drafts.recovered = Some(1);
    let unavailable = want(
        RowKey::DraftStore,
        Kind::Failure,
        "Recovery copies are unavailable",
    )
    .desc("Access to the drafts folder was denied.")
    .detail(&["Recovery store"])
    .group(Group {
        candidate: true,
        ..Group::default()
    });
    let recovered = |title: &str| {
        want(RowKey::Recovered, Kind::Info, title)
            .button(Act::ReviewDrafts, "Review", true)
            .then(Act::DismissRecovered, "Dismiss", true)
            .detail(&["Recovered drafts"])
    };
    assert_rows(
        &world.rows(),
        &[
            unavailable,
            recovered("1 draft recovered from an earlier session"),
        ],
        "S12, S13",
    );
    world.drafts.recovered = Some(3);
    let rows = world.rows();
    assert_eq!(rows[1].title, "3 drafts recovered from an earlier session");
    assert!(
        !rows[1].clearable && !rows[1].group.candidate,
        "S13: never counted"
    );
}

/// S14: each record note, with Dismiss as `DismissNote`.
pub(super) fn notes() {
    let mut world = World::new();
    assert!(world.operations.note_record(Report::NotSaved));
    assert!(world.operations.note_record(Report::Unreadable));
    let note = |report: Report, title: &str| {
        want(RowKey::Note(report), Kind::Info, title)
            .then(Act::DismissNote(report), "Dismiss", true)
            .detail(&["Record note"])
    };
    assert_rows(
        &world.rows(),
        &[
            note(
                Report::NotSaved,
                "Unfinished checks could not be saved to disk. They are kept until you quit",
            ),
            note(
                Report::Unreadable,
                "Unfinished checks from an earlier session could not be read, so they were set aside",
            ),
        ],
        "S14",
    );
}
