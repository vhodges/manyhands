//! What each Drafts page row offers: the restore policy for a draft no one
//! holds, Show for one restored here, Inspect only for one open
//! elsewhere, Inspect and Discard while its item is read, Discard only
//! for one that cannot be read, and Retry exactly for a failed read.
//!
//! Rows hold draft text and derive no `Debug`; they are read through
//! `match` and their fields.

use manyhands::{
    drafts::{
        CommentTarget, Comparison, Draft, DraftFields, DraftId, DraftTarget, Held, ItemKind,
        TargetState, UnreadableReason, restore_policy,
    },
    results::ResultCode,
};
use time::OffsetDateTime;

use super::{COMMENT_ON, DOCUMENT, DraftRow, RowActions, RowState, TICKET, row_actions};
use crate::desktop::model::read_state::Read;

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";

fn draft(kind: ItemKind, comment: bool) -> Draft {
    let current = match (comment, kind) {
        (true, _) => DraftFields::Comment {
            body: "A reply".to_owned(),
        },
        (false, ItemKind::Document) => DraftFields::Document {
            title: "Draft title".to_owned(),
            body: "Body".to_owned(),
        },
        (false, ItemKind::Ticket) => DraftFields::Ticket {
            title: "Draft title".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            deps: Vec::new(),
            parent: None,
            body: "Body".to_owned(),
        },
    };
    let now = OffsetDateTime::now_utc();
    Draft {
        id: DraftId::generate(),
        revision: 1,
        created_at: now,
        flushed_at: now,
        target: DraftTarget {
            repository: "/a/repository".to_owned(),
            item_kind: kind,
            item_id: ITEM.parse().unwrap(),
            comment: comment.then(|| CommentTarget {
                comment_id: COMMENT.parse().unwrap(),
                parent_id: None,
            }),
        },
        base: None,
        current,
    }
}

fn theirs() -> DraftFields {
    DraftFields::Document {
        title: "Theirs".to_owned(),
        body: "Body".to_owned(),
    }
}

/// Every target state, with whether the restore policy allows a restore
/// of an item draft no one holds.
fn targets() -> Vec<(TargetState, bool)> {
    let comparison = || Comparison { fields: Vec::new() };
    vec![
        (TargetState::Current, true),
        (
            TargetState::Changed {
                theirs: theirs(),
                comparison: comparison(),
                context_changed: false,
            },
            true,
        ),
        (TargetState::AlreadySaved, true),
        (
            TargetState::Unverified {
                theirs: theirs(),
                comparison: comparison(),
            },
            true,
        ),
        (TargetState::TicketClosed, false),
        (TargetState::ItemNotFound, false),
        (TargetState::Nonconforming, false),
        (
            TargetState::RepositoryUnavailable(ResultCode::RepositoryNotRegistered),
            false,
        ),
        (TargetState::ReadFailed(ResultCode::Busy), false),
        (TargetState::ParentNotFound, false),
    ]
}

fn ready(held: Held, target: TargetState) -> RowState {
    RowState::Readable {
        held,
        target: Read::Ready(target),
    }
}

fn actions(draft: &Draft, held: Held, target: TargetState) -> RowActions {
    let restore = restore_policy(draft, &target, held);
    row_actions(&ready(held, target), Some(restore))
}

#[test]
fn a_draft_no_one_holds_follows_the_restore_policy() {
    let draft = draft(ItemKind::Ticket, false);
    for (target, allowed) in targets() {
        let failed = matches!(target, TargetState::ReadFailed(_));
        assert_eq!(
            actions(&draft, Held::No, target),
            RowActions {
                restore: allowed,
                show: false,
                inspect: true,
                discard: true,
                retry: failed,
            }
        );
    }
}

#[test]
fn a_comment_draft_is_never_restored_in_d2() {
    let draft = draft(ItemKind::Ticket, true);
    for (target, _) in targets() {
        let failed = matches!(target, TargetState::ReadFailed(_));
        let offered = actions(&draft, Held::No, target);
        assert!(!offered.restore);
        assert_eq!(offered.retry, failed);
        assert!(offered.inspect && offered.discard && !offered.show);
    }
}

#[test]
fn a_draft_held_here_offers_show_inspect_and_discard_never_restore() {
    let draft = draft(ItemKind::Document, false);
    let expected = RowActions {
        restore: false,
        show: true,
        inspect: true,
        discard: true,
        retry: false,
    };
    for (target, _) in targets() {
        assert_eq!(actions(&draft, Held::Here, target), expected);
    }
    let loading = RowState::Readable {
        held: Held::Here,
        target: Read::Loading,
    };
    assert_eq!(row_actions(&loading, None), expected);
}

#[test]
fn a_draft_held_elsewhere_offers_inspect_only() {
    let draft = draft(ItemKind::Document, false);
    let expected = RowActions {
        inspect: true,
        ..RowActions::default()
    };
    for (target, _) in targets() {
        assert_eq!(actions(&draft, Held::Elsewhere, target), expected);
    }
    let loading = RowState::Readable {
        held: Held::Elsewhere,
        target: Read::Loading,
    };
    assert_eq!(row_actions(&loading, None), expected);
}

#[test]
fn a_row_being_checked_offers_inspect_and_discard_only() {
    let loading = RowState::Readable {
        held: Held::No,
        target: Read::Loading,
    };
    assert_eq!(
        row_actions(&loading, None),
        RowActions {
            inspect: true,
            discard: true,
            ..RowActions::default()
        }
    );
}

/// Carry note: the policy's `Refused(HeldHere)` is Show.
#[test]
fn a_policy_that_finds_the_draft_held_here_offers_show() {
    let draft = draft(ItemKind::Document, false);
    let restore = restore_policy(&draft, &TargetState::Current, Held::Here);
    let offered = row_actions(&ready(Held::No, TargetState::Current), Some(restore));
    assert!(offered.show && !offered.restore);
}

#[test]
fn an_unreadable_draft_offers_discard_only() {
    for reason in [
        UnreadableReason::NewerFormat { version: 2 },
        UnreadableReason::Damaged,
        UnreadableReason::Access,
        UnreadableReason::NotOwned,
        UnreadableReason::NotAFile,
    ] {
        assert_eq!(
            row_actions(&RowState::Unreadable(reason), None),
            RowActions {
                discard: true,
                ..RowActions::default()
            }
        );
        let row = DraftRow::unreadable(DraftId::generate(), reason);
        assert!(row.actions.discard && !row.actions.inspect);
    }
}

/// A row names its kind, its own title and its repository, is checked
/// until its reads land, and then offers what the policy allows. A
/// comment's kind names its item by the title read, or its ID.
#[test]
fn a_row_is_labeled_by_its_kind_and_lands_its_target() {
    let document = draft(ItemKind::Document, false);
    let mut row = DraftRow::readable(&document, Held::No, "2026-10-24 10:00:00".to_owned());
    assert_eq!(row.kind_label, DOCUMENT);
    assert_eq!(row.title, "Draft title");
    assert_eq!(row.repository, "/a/repository");
    assert!(row.target().is_none());
    assert!(!row.actions.restore && row.actions.inspect);
    row.landed(&document, TargetState::Current, Some("Item title"));
    assert!(matches!(row.target(), Some(TargetState::Current)));
    assert!(row.actions.restore);
    assert_eq!(row.kind_label, DOCUMENT, "an item's kind does not change");
    row.reading();
    assert!(row.target().is_none());
    assert!(!row.actions.restore);

    let ticket = draft(ItemKind::Ticket, false);
    let row = DraftRow::readable(&ticket, Held::No, String::new());
    assert_eq!(row.kind_label, TICKET);

    let comment = draft(ItemKind::Ticket, true);
    let mut row = DraftRow::readable(&comment, Held::No, String::new());
    assert_eq!(row.kind_label, format!("{COMMENT_ON} {ITEM}"));
    assert_eq!(row.title, "", "a comment has no title of its own");
    row.landed(&comment, TargetState::Current, Some("Item title"));
    assert_eq!(row.kind_label, format!("{COMMENT_ON} Item title"));
    row.landed(&comment, TargetState::ItemNotFound, None);
    assert_eq!(row.kind_label, format!("{COMMENT_ON} {ITEM}"));
}
