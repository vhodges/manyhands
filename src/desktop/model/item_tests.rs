//! The item view's state: provenance in text, which problems make the
//! recovery view, how unknown metadata is shown, and stale reads dropped.

use manyhands::{
    repository::{
        CommentListDto, IndexState, ItemContextDto, ItemContextKind, ItemDto, ItemDtoKind,
        ProblemDto,
    },
    results::{ProblemCode, ResultCode},
};
use serde_json::json;

use super::{
    ItemReads, ItemView, KIND_ACTIVE, KIND_PRIMARY, KIND_UNVERIFIED, LATER, NO_BRANCH,
    comment_author, context_problems, metadata_value, provenance_label,
};
use crate::desktop::{
    model::{
        list_state::ListState,
        navigation::{NotReloaded, OpenTarget},
        stamp::{TabId, ViewScope},
    },
    services::{ReadFailure, ReadResult},
    test_support::fixtures::{Fixture, item_id},
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";

fn context(kind: ItemContextKind, branch: Option<&str>) -> ItemContextDto {
    ItemContextDto {
        kind,
        branch: branch.map(str::to_owned),
        worktree: "/work/a".to_owned(),
        head_oid: Some("0123abcd".to_owned()),
    }
}

#[test]
fn provenance_names_the_context_kind_its_branch_and_worktree() {
    assert_eq!(
        provenance_label(&context(ItemContextKind::Primary, Some("main"))),
        format!("{KIND_PRIMARY}, branch main, worktree /work/a")
    );
    assert_eq!(
        provenance_label(&context(
            ItemContextKind::Active,
            Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FD0")
        )),
        format!(
            "{KIND_ACTIVE}, branch manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FD0, worktree /work/a"
        )
    );
    assert_eq!(
        provenance_label(&context(ItemContextKind::Unverified, None)),
        format!("{KIND_UNVERIFIED}, {NO_BRANCH}, worktree /work/a")
    );
    assert_eq!(KIND_PRIMARY, "Primary");
    assert_eq!(KIND_ACTIVE, "Active item worktree");
    assert_eq!(KIND_UNVERIFIED, "Unverified");
}

#[test]
fn the_unavailable_actions_are_the_designs_six() {
    assert_eq!(
        LATER,
        [
            "Edit",
            "Comment",
            "Sync",
            "Approve and merge",
            "Close ticket",
            "Repair"
        ]
    );
}

#[test]
fn a_null_metadata_value_is_not_representable_and_others_are_the_librarys_json() {
    assert_eq!(metadata_value(&json!(null)), None);
    // Text as itself, not quoted.
    assert_eq!(metadata_value(&json!("last")).as_deref(), Some("last"));
    assert_eq!(
        metadata_value(&json!({"deep": [1, 2.5, true]})).as_deref(),
        Some(r#"{"deep":[1,2.5,true]}"#)
    );
    assert_eq!(metadata_value(&json!(7)).as_deref(), Some("7"));
}

fn problem(code: ProblemCode) -> ProblemDto {
    ProblemDto {
        code,
        path: Some(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FD0".to_owned()),
        target_id: None,
    }
}

/// A real ticket, read through the library, from a fixture with one
/// comment on it.
fn shown_ticket() -> (Fixture, manyhands::repository::ItemDto) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shown", "");
    fixture.commit_comment(&root, TICKET, COMMENT, None, "2026-09-30T12:00:01Z", "");
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let item = service.show_item(&repo, &item_id(TICKET)).unwrap();
    (fixture, item)
}

/// Only a context or branch problem makes the recovery view; other
/// problems are listed as problems.
#[test]
fn only_context_and_branch_problems_make_the_recovery_view() {
    let (_fixture, mut item) = shown_ticket();
    assert!(context_problems(&item).is_empty());
    item.problems
        .push(problem(ProblemCode::MetadataNotRepresentable));
    assert!(context_problems(&item).is_empty());
    item.problems.push(problem(ProblemCode::ContextProblem));
    item.problems.push(problem(ProblemCode::BranchProblem));
    let codes: Vec<ProblemCode> = context_problems(&item)
        .iter()
        .map(|problem| problem.code)
        .collect();
    assert_eq!(
        codes,
        [ProblemCode::ContextProblem, ProblemCode::BranchProblem]
    );
}

#[test]
fn a_comment_without_an_author_is_by_an_unknown_author() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shown", "");
    fixture.commit_comment(&root, TICKET, COMMENT, None, "2026-09-30T12:00:01Z", "");
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let list = service.list_comments(&repo, &item_id(TICKET)).unwrap();
    assert_eq!(list.items[0].author, None);
    assert_eq!(comment_author(&list.items[0]), "unknown author");
}

fn ticket_target() -> OpenTarget {
    OpenTarget::Item {
        repo: "/work/a".to_owned(),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    }
}

/// An item target loads its comments too; a nonconforming entry has no ID
/// to list comments of.
#[test]
fn only_an_item_by_id_has_a_discussion() {
    let mut view = ItemView::new(ticket_target(), ViewScope::Tab(TabId(0)));
    view.load();
    assert!(matches!(view.item(), ListState::Loading));
    assert!(matches!(view.comments(), Some(ListState::Loading)));

    let mut inspect = ItemView::new(
        OpenTarget::Inspect {
            repo: "/work/a".to_owned(),
            worktree: "/work/a".to_owned(),
            path: ".manyhands/tickets/x/ticket.md".to_owned(),
        },
        ViewScope::Tab(TabId(0)),
    );
    inspect.load();
    assert!(inspect.comments().is_none());
}

/// A load of the item and its comments only, as a test gives it.
fn item_reads(
    item: ReadResult<ItemDto>,
    comments: Option<ReadResult<CommentListDto>>,
) -> ItemReads {
    ItemReads {
        item,
        comments,
        dependents: None,
        children: None,
        parent: None,
    }
}

#[test]
fn a_finished_read_is_shown_with_its_index_state() {
    let (_fixture, item) = shown_ticket();
    let mut view = ItemView::new(ticket_target(), ViewScope::Tab(TabId(3)));
    let stamp = view.load();
    assert_eq!(stamp.scope, ViewScope::Tab(TabId(3)));
    let failure = ReadFailure {
        code: ResultCode::ItemNotFound,
        recovery: Vec::new(),
    };
    assert!(view.apply_reads(
        stamp,
        item_reads(
            ReadResult::Ok(item.clone()),
            Some(ReadResult::Failed(failure))
        ),
    ));
    match view.item() {
        ListState::Ready {
            rows,
            complete,
            index,
        } => {
            assert_eq!(rows.id, item.id);
            assert!(*complete);
            assert_eq!(
                index.as_ref().map(|index| &index.state),
                Some(&IndexState::Current)
            );
        }
        _ => panic!("not ready"),
    }
    assert!(matches!(
        view.comments(),
        Some(ListState::Failed(failure)) if failure.code == ResultCode::ItemNotFound
    ));
    // A first load shows its answer: nothing was shown to keep.
    assert!(view.not_refreshed().is_none());
}

/// Review Focus 1: a load asked before the view loaded again never
/// replaces what the later load shows.
#[test]
fn a_read_from_before_a_reload_is_dropped() {
    let (_fixture, item) = shown_ticket();
    let mut view = ItemView::new(ticket_target(), ViewScope::Tab(TabId(0)));
    let first = view.load();
    let second = view.load();
    assert!(!view.apply_reads(
        first,
        item_reads(ReadResult::Ok(item.clone()), Some(ReadResult::Busy))
    ));
    assert!(matches!(view.item(), ListState::Loading));
    assert!(matches!(view.comments(), Some(ListState::Loading)));
    assert!(view.apply_reads(second, item_reads(ReadResult::Busy, Some(ReadResult::Busy))));
    assert!(matches!(view.item(), ListState::Busy));
    assert!(view.not_refreshed().is_none());
}

/// Ruling R-B5: a reload keeps what is shown until its load lands; a load
/// from before it is still dropped. One that stays busy keeps the rows
/// shown and says so, since busy passes; one that fails shows its failure,
/// as a first load does. The next load that gives the item replaces it
/// and clears the reason. The last title read stays either way.
#[test]
fn a_busy_reload_keeps_what_is_shown_and_a_failed_one_shows_its_failure() {
    let (_fixture, item) = shown_ticket();
    let mut view = ItemView::new(ticket_target(), ViewScope::Tab(TabId(1)));
    let first = view.load();
    assert!(view.apply_reads(
        first.clone(),
        item_reads(
            ReadResult::Ok(item.clone()),
            Some(ReadResult::Ok(comments(&item)))
        )
    ));

    let second = view.reload();
    assert_eq!(second.scope, ViewScope::Tab(TabId(1)));
    assert!(matches!(view.item(), ListState::Ready { rows, .. } if rows.id == item.id));
    assert!(!view.apply_reads(first, item_reads(ReadResult::Busy, None)));
    assert!(matches!(view.item(), ListState::Ready { .. }));

    assert!(view.apply_reads(second, item_reads(ReadResult::Busy, Some(ReadResult::Busy))));
    assert!(matches!(view.item(), ListState::Ready { rows, .. } if rows.id == item.id));
    assert!(matches!(view.comments(), Some(ListState::Ready { .. })));
    assert!(matches!(view.not_refreshed(), Some(NotReloaded::Busy)));

    let failure = ReadFailure {
        code: ResultCode::ItemNotFound,
        recovery: Vec::new(),
    };
    let third = view.reload();
    assert!(view.apply_reads(
        third,
        item_reads(
            ReadResult::Failed(failure.clone()),
            Some(ReadResult::Failed(failure))
        ),
    ));
    assert!(matches!(
        view.item(),
        ListState::Failed(why) if why.code == ResultCode::ItemNotFound
    ));
    assert!(matches!(
        view.comments(),
        Some(ListState::Failed(why)) if why.code == ResultCode::ItemNotFound
    ));
    assert!(view.not_refreshed().is_none());
    assert_eq!(view.last_title(), Some("Shown"));

    let fourth = view.reload();
    assert!(view.apply_reads(fourth, item_reads(ReadResult::Ok(item.clone()), None)));
    assert!(matches!(view.item(), ListState::Ready { .. }));
    assert!(view.not_refreshed().is_none());
}

/// The comments of `item`'s read, as the library lists them.
fn comments(item: &ItemDto) -> CommentListDto {
    CommentListDto {
        items: Vec::new(),
        complete: true,
        context: item.context.clone(),
        index: item.index.clone(),
    }
}
