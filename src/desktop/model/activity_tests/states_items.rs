//! P1 for S5 to S8: the unshared list's problems and items, the Retire…
//! offers and the retirement events.

use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, PollDisposition},
};

use super::{
    super::support::{ITEM, ROOT},
    world::{
        DOCUMENT, DOCUMENT_SHORT, SHORT, Want, World, assert_rows, counted, quiet, target, want,
    },
};
use crate::desktop::model::{
    activity::{
        rows::{ActivityAction as Act, ItemRef, Kind, RowKey, Target},
        states::ShareState,
    },
    polling::{AttemptTarget, Disposition, TargetKind, Unshared},
};

/// A case: its name, whether items are titled, what the list says, how each
/// share stands and the rows it must make.
type Reminder = (
    &'static str,
    bool,
    Vec<AttemptTarget>,
    Vec<(Target, ShareState)>,
    Vec<Want>,
);

fn id() -> ItemId {
    ITEM.parse().unwrap()
}

fn document() -> ItemId {
    DOCUMENT.parse().unwrap()
}

fn item_ref(kind: AuthoringKind, id: &ItemId) -> ItemRef {
    ItemRef {
        root: ROOT.to_owned(),
        kind,
        id: id.clone(),
    }
}

fn found(target: AttemptTarget, id: Option<ItemId>, kind: Option<TargetKind>) -> AttemptTarget {
    AttemptTarget {
        item_id: id,
        kind,
        ..target
    }
}

fn main_target(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: None,
        kind: None,
        disposition,
        main: true,
    }
}

/// S5: a problem row for each problem of the unshared list, keyed by its
/// target (R-17); Open only when the target has an ID and a kind.
pub(super) fn problems() {
    let guidance = |disposition: PollDisposition| disposition.guidance();
    let dirty = format!("Dirty: {}", guidance(PollDisposition::Dirty));
    let malformed = format!("Malformed: {}", guidance(PollDisposition::Malformed));
    let not_reached = format!("Not reached: {}", guidance(PollDisposition::NotReached));
    let key = |target| RowKey::Problem {
        root: ROOT.to_owned(),
        target,
    };
    let ticket = AuthoringKind::Ticket;
    let open = |kind: AuthoringKind, id: &ItemId| Act::OpenItem(item_ref(kind, id));
    let cases: Vec<(&str, bool, Vec<AttemptTarget>, Vec<Want>)> = vec![
        (
            "a titled ticket",
            true,
            vec![target(Disposition::Dirty)],
            vec![
                want(
                    key(Target::Item(id())),
                    Kind::Failure,
                    "“Plan” needs a look",
                )
                .desc("It has changes on this computer that aren't saved yet.")
                .repo("demo")
                .button(open(ticket, &id()), "Open", true)
                .detail(&[&dirty])
                .item(ticket)
                .group(counted(&[ROOT])),
            ],
        ),
        (
            "an untitled ticket, and a document",
            false,
            vec![
                target(Disposition::Dirty),
                found(
                    target(Disposition::Malformed),
                    Some(document()),
                    Some(TargetKind::Document),
                ),
            ],
            vec![
                want(
                    key(Target::Item(id())),
                    Kind::Failure,
                    &format!("The ticket {SHORT} needs a look"),
                )
                .desc("It has changes on this computer that aren't saved yet.")
                .repo("demo")
                .button(open(ticket, &id()), "Open", true)
                .detail(&[&dirty])
                .item(ticket)
                .group(counted(&[ROOT])),
                want(
                    key(Target::Item(document())),
                    Kind::Failure,
                    &format!("The document {DOCUMENT_SHORT} needs a look"),
                )
                .desc("Its file can't be read as a ticket or document.")
                .repo("demo")
                .button(open(AuthoringKind::Document, &document()), "Open", true)
                .detail(&[&malformed])
                .item_of(document(), AuthoringKind::Document)
                .group(counted(&[ROOT])),
            ],
        ),
        (
            "an ID with no kind",
            true,
            vec![found(target(Disposition::NotReached), Some(id()), None)],
            vec![
                want(
                    key(Target::Item(id())),
                    Kind::Failure,
                    &format!("The item {SHORT} needs a look"),
                )
                .desc("The check didn't reach it this time.")
                .repo("demo")
                .detail(&[&not_reached])
                .group(counted(&[ROOT])),
            ],
        ),
        (
            "two targets with no ID, and main",
            true,
            vec![
                found(target(Disposition::Dirty), None, None),
                main_target(Disposition::Conflicted),
                found(target(Disposition::Dirty), None, Some(TargetKind::Ticket)),
            ],
            vec![
                want(
                    key(Target::Unnamed(0)),
                    Kind::Failure,
                    "An item needs a look",
                )
                .desc("It has changes on this computer that aren't saved yet.")
                .repo("demo")
                .detail(&[&dirty])
                .group(counted(&[ROOT])),
                want(
                    key(Target::Main),
                    Kind::Failure,
                    "trunk in demo needs a look",
                )
                .desc("It has a conflict to resolve.")
                .repo("demo")
                .detail(&[&format!(
                    "Conflicted: {}",
                    guidance(PollDisposition::Conflicted)
                )])
                .group(counted(&[ROOT])),
                want(
                    key(Target::Unnamed(1)),
                    Kind::Failure,
                    "An item needs a look",
                )
                .desc("It has changes on this computer that aren't saved yet.")
                .repo("demo")
                .detail(&[&dirty])
                .group(counted(&[ROOT])),
            ],
        ),
    ];
    for (name, titled, problems, wanted) in cases {
        let mut world = World::new();
        world.titled = titled;
        world.repositories[0].unshared = Unshared {
            items: Vec::new(),
            problems,
            as_of: None,
        };
        assert_rows(&world.rows(), &wanted, &format!("S5 {name}"));
    }
}

/// S6: a reminder for each unshared item, main included, never counted; its
/// Share… is disabled while the item's share runs or while it is locked, and
/// main's is "Share main".
pub(super) fn unshared() {
    let key = |target| RowKey::Unshared {
        root: ROOT.to_owned(),
        target,
    };
    let share = |kind: AuthoringKind| Act::Share {
        root: ROOT.to_owned(),
        item: Some((kind, id())),
    };
    let ahead = format!("Not shared yet: {}", "Local ahead");
    let unpublished = "Not shared yet: Unpublished";
    let ticket = AuthoringKind::Ticket;
    let base = |title: &str| {
        want(key(Target::Item(id())), Kind::ToShare, title)
            .repo("demo")
            .group(quiet(&[ROOT]))
    };
    let ready = |title: &str, detail: &str| {
        base(title)
            .button(share(ticket), "Share…", true)
            .detail(&[detail])
            .item(ticket)
    };
    let main_share = Act::Share {
        root: ROOT.to_owned(),
        item: None,
    };
    let cases: Vec<Reminder> = vec![
        (
            "ready",
            true,
            vec![target(Disposition::LocalAhead)],
            vec![(Target::Item(id()), ShareState::Ready)],
            vec![ready("“Plan” isn't shared yet", &ahead)],
        ),
        (
            "no share state means ready",
            true,
            vec![target(Disposition::Unpublished)],
            vec![],
            vec![ready("“Plan” isn't shared yet", unpublished)],
        ),
        (
            "untitled, diverged",
            false,
            vec![target(Disposition::Diverged)],
            vec![],
            vec![ready(
                &format!("The ticket {SHORT} isn't shared yet"),
                "Not shared yet: Diverged",
            )],
        ),
        (
            "its share runs",
            true,
            vec![target(Disposition::LocalAhead)],
            vec![(Target::Item(id()), ShareState::Sharing)],
            vec![
                base("“Plan” isn't shared yet")
                    .button(share(ticket), "Sharing…", false)
                    .detail(&[&ahead])
                    .item(ticket),
            ],
        ),
        (
            "locked",
            true,
            vec![target(Disposition::LocalAhead)],
            vec![(
                Target::Item(id()),
                ShareState::Locked("Its close is not finished.".to_owned()),
            )],
            vec![
                base("“Plan” isn't shared yet")
                    .button(share(ticket), "Share…", false)
                    .detail(&[&ahead, "Its close is not finished."])
                    .item(ticket),
            ],
        ),
        (
            "main",
            true,
            vec![main_target(Disposition::LocalAhead)],
            vec![],
            vec![
                want(
                    key(Target::Main),
                    Kind::ToShare,
                    "trunk in demo isn't shared yet",
                )
                .repo("demo")
                .button(main_share.clone(), "Share main", true)
                .detail(&[&ahead])
                .group(quiet(&[ROOT])),
            ],
        ),
        (
            "main, its share runs",
            true,
            vec![main_target(Disposition::LocalAhead)],
            vec![(Target::Main, ShareState::Sharing)],
            vec![
                want(
                    key(Target::Main),
                    Kind::ToShare,
                    "trunk in demo isn't shared yet",
                )
                .repo("demo")
                .button(main_share, "Sharing…", false)
                .detail(&[&ahead])
                .group(quiet(&[ROOT])),
            ],
        ),
        (
            "no kind, and no ID",
            true,
            vec![
                found(target(Disposition::LocalAhead), Some(id()), None),
                found(target(Disposition::LocalAhead), None, None),
            ],
            vec![],
            vec![
                base(&format!("The item {SHORT} isn't shared yet")).detail(&[&ahead]),
                want(
                    key(Target::Unnamed(0)),
                    Kind::ToShare,
                    "An item isn't shared yet",
                )
                .repo("demo")
                .detail(&[&ahead])
                .group(quiet(&[ROOT])),
            ],
        ),
    ];
    for (name, titled, items, sharing, wanted) in cases {
        let mut world = World::new();
        world.titled = titled;
        world.repositories[0].unshared = Unshared {
            items,
            problems: Vec::new(),
            as_of: None,
        };
        world.repositories[0].sharing = sharing.into_iter().collect();
        assert_rows(&world.rows(), &wanted, &format!("S6 {name}"));
    }
}
