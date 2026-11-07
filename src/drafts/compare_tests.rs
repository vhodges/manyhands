//! The pure comparison, target state and restore policy, over items built
//! here. Draft types derive no `Debug`, so these read them through `match`.

use super::compare::*;
use super::schema::*;
use super::store::Held;
use crate::canonical::ItemId;
use crate::repository::{
    ClosureDto, ClosureState, CommentDto, CommentListDto, DependencyDto, DependencyState,
    IndexState, IndexStateDto, ItemContextDto, ItemContextKind, ItemDto, ItemDtoKind,
};
use crate::results::ResultCode;
use time::OffsetDateTime;

const ITEM: &str = "01M5CS4MJ0G7B0MPJD0KN0QMYA";
const DEP: &str = "01M5B0Z6PNP7X4WH4JV3N0VQJ6";
const PARENT: &str = "01M57SKVF6TQ3QM6AXJ35JGGSY";
const COMMENT: &str = "01M4CC0VMZ21Z79B09YQRA0P7J";
const ROOT: &str = "/home/u/work/project";

fn id(text: &str) -> ItemId {
    text.parse().expect("item id")
}

fn ticket(title: &str, body: &str) -> DraftFields {
    DraftFields::Ticket {
        title: title.to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: Some("core".to_owned()),
        deps: vec![DEP.to_owned()],
        parent: Some(PARENT.to_owned()),
        body: body.to_owned(),
    }
}

fn context(kind: ContextKind, branch: &str) -> BaseContext {
    BaseContext {
        kind,
        branch: Some(branch.to_owned()),
        worktree: ROOT.to_owned(),
        head_oid: Some("abc".to_owned()),
    }
}

fn base(observation: Option<&str>, fields: DraftFields) -> DraftBase {
    DraftBase {
        observation: observation.map(str::to_owned),
        path: format!(".manyhands/tickets/{ITEM}/ticket.md"),
        context: context(ContextKind::Primary, "main"),
        source: Some("---\n---\n".to_owned()),
        fields,
    }
}

fn draft(base: Option<DraftBase>, current: DraftFields) -> Draft {
    let now = OffsetDateTime::from_unix_timestamp(1_792_000_000).unwrap();
    Draft {
        id: "01J9ZQ6V0Y8K3N4M5P6Q7R8S9T".parse().expect("draft id"),
        revision: 3,
        created_at: now,
        flushed_at: now,
        target: DraftTarget {
            repository: ROOT.to_owned(),
            item_kind: ItemKind::Ticket,
            item_id: id(ITEM),
            comment: None,
        },
        base,
        current,
    }
}

/// A ticket draft whose base is the "Base" ticket at observation `obs-1`
/// on `main`, and whose text renames it "Mine".
fn ticket_draft() -> Draft {
    draft(
        Some(base(Some("obs-1"), ticket("Base", "Body\n"))),
        ticket("Mine", "Body\n"),
    )
}

fn comment_draft(parent: Option<&str>) -> Draft {
    let mut draft = draft(
        None,
        DraftFields::Comment {
            body: "Hi\n".into(),
        },
    );
    draft.target.comment = Some(CommentTarget {
        comment_id: id(COMMENT),
        parent_id: parent.map(id),
    });
    draft
}

/// The item a read gives: the ticket `title` at `observation` on `branch`.
fn item(title: &str, observation: &str, branch: &str) -> ItemDto {
    ItemDto {
        id: Some(ITEM.to_owned()),
        kind: ItemDtoKind::Ticket,
        path: format!(".manyhands/tickets/{ITEM}/ticket.md"),
        title: Some(title.to_owned()),
        ticket_type: Some("task".to_owned()),
        status: Some("open".to_owned()),
        project: None,
        team: Some("core".to_owned()),
        closure: Some(ClosureDto {
            state: ClosureState::Open,
            closed_at: None,
            closed_by: None,
        }),
        slug: None,
        parent: Some(DependencyDto {
            id: PARENT.to_owned(),
            state: DependencyState::Open,
        }),
        deps: vec![DependencyDto {
            id: DEP.to_owned(),
            state: DependencyState::Unresolved,
        }],
        readiness: None,
        unknown_metadata: serde_json::Map::new(),
        body: Some("Body\n".to_owned()),
        source: Some("---\nsource\n---\nBody\n".to_owned()),
        observation: Some(observation.to_owned()),
        context: ItemContextDto {
            kind: if branch == "main" {
                ItemContextKind::Primary
            } else {
                ItemContextKind::Active
            },
            branch: Some(branch.to_owned()),
            worktree: ROOT.to_owned(),
            head_oid: Some("abc".to_owned()),
        },
        changed_at: None,
        change_source: None,
        problems: Vec::new(),
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

fn closed(mut item: ItemDto) -> ItemDto {
    item.closure = Some(ClosureDto {
        state: ClosureState::Closed,
        closed_at: Some("2026-10-01T00:00:00Z".to_owned()),
        closed_by: Some("A <a@example.invalid>".to_owned()),
    });
    item
}

fn nonconforming(mut item: ItemDto) -> ItemDto {
    item.id = None;
    item.title = None;
    item.ticket_type = None;
    item.status = None;
    item.team = None;
    item.closure = None;
    item.parent = None;
    item.deps = Vec::new();
    item.body = None;
    item
}

fn comments(complete: bool, ids: &[(&str, Option<&str>)]) -> CommentListDto {
    CommentListDto {
        items: ids
            .iter()
            .map(|(comment, parent)| CommentDto {
                id: Some((*comment).to_owned()),
                item_id: ITEM.to_owned(),
                parent_id: parent.map(str::to_owned),
                author: None,
                created_at: None,
                body: Some("text".to_owned()),
                path: format!(".manyhands/comments/{ITEM}/{comment}.md"),
                unknown_metadata: serde_json::Map::new(),
                problems: Vec::new(),
                depth: u32::from(parent.is_some()),
            })
            .collect(),
        complete,
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: ROOT.to_owned(),
            head_oid: None,
        },
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

fn changes(comparison: &Comparison) -> Vec<Change> {
    comparison
        .fields
        .iter()
        .map(|(_, change)| *change)
        .collect()
}

fn names(comparison: &Comparison) -> Vec<Field> {
    comparison.fields.iter().map(|(field, _)| *field).collect()
}

// ---- fields of an item ----

#[test]
fn the_fields_of_a_ticket_are_its_named_fields_with_relationship_ids_in_file_order() {
    let mut read = item("Base", "obs-1", "main");
    read.deps.push(DependencyDto {
        id: PARENT.to_owned(),
        state: DependencyState::Closed,
    });
    let fields = DraftFields::of(&read).expect("a conforming ticket has fields");
    match fields {
        DraftFields::Ticket {
            title,
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            body,
        } => {
            assert_eq!(title, "Base");
            assert_eq!(ticket_type, "task");
            assert_eq!(status, "open");
            assert_eq!(project, None);
            assert_eq!(team.as_deref(), Some("core"));
            assert_eq!(deps, [DEP, PARENT]);
            assert_eq!(parent.as_deref(), Some(PARENT));
            assert_eq!(body, "Body\n");
        }
        _ => panic!("a ticket's fields are a ticket's"),
    }
}

#[test]
fn the_fields_of_a_document_are_its_title_and_body() {
    let mut read = item("Guide", "obs-1", "main");
    read.kind = ItemDtoKind::Document;
    read.ticket_type = None;
    read.status = None;
    read.closure = None;
    read.deps = Vec::new();
    read.parent = None;
    assert!(
        DraftFields::of(&read)
            == Some(DraftFields::Document {
                title: "Guide".to_owned(),
                body: "Body\n".to_owned(),
            })
    );
}

#[test]
fn a_nonconforming_entry_has_no_fields_and_no_base() {
    let read = nonconforming(item("Base", "obs-1", "main"));
    assert!(DraftFields::of(&read).is_none());
    assert!(DraftBase::of(&read).is_none());
}

#[test]
fn the_base_of_an_item_is_its_observation_path_context_source_and_fields() {
    let read = item("Base", "obs-1", "manyhands/ticket/x");
    let base = DraftBase::of(&read).expect("a conforming item has a base");
    assert_eq!(base.observation.as_deref(), Some("obs-1"));
    assert_eq!(base.path, read.path);
    assert_eq!(
        base.context,
        BaseContext {
            kind: ContextKind::Active,
            branch: Some("manyhands/ticket/x".to_owned()),
            worktree: ROOT.to_owned(),
            head_oid: Some("abc".to_owned()),
        }
    );
    assert_eq!(base.source, read.source);
    assert!(base.fields == ticket("Base", "Body\n"));
}

// ---- three-way comparison ----

#[test]
fn every_field_of_a_ticket_is_compared_in_a_fixed_order_with_the_body_last() {
    let fields = ticket("T", "B");
    let comparison = compare(&fields, &fields, &fields);
    assert_eq!(
        names(&comparison),
        [
            Field::Title,
            Field::Type,
            Field::Status,
            Field::Project,
            Field::Team,
            Field::Deps,
            Field::Parent,
            Field::Body,
        ]
    );
    assert!(changes(&comparison).iter().all(|c| *c == Change::None));
}

#[test]
fn a_document_compares_its_title_then_its_body() {
    let doc = |title: &str, body: &str| DraftFields::Document {
        title: title.to_owned(),
        body: body.to_owned(),
    };
    let comparison = compare(&doc("a", "x"), &doc("a", "y"), &doc("b", "x"));
    assert_eq!(
        comparison.fields,
        [(Field::Title, Change::Mine), (Field::Body, Change::Theirs)]
    );
}

#[test]
fn each_change_is_told_apart_per_field() {
    let with = |title: &str, status: &str, team: Option<&str>, deps: &[&str], body: &str| {
        DraftFields::Ticket {
            title: title.to_owned(),
            ticket_type: "task".to_owned(),
            status: status.to_owned(),
            project: None,
            team: team.map(str::to_owned),
            deps: deps.iter().map(|d| (*d).to_owned()).collect(),
            parent: None,
            body: body.to_owned(),
        }
    };
    let base = with("t", "open", None, &[], "b");
    // Title: only mine. Status: only theirs. Team: both, the same.
    // Deps: both, differently. Body: both, differently.
    let theirs = with("t", "done", Some("core"), &[DEP], "their body");
    let mine = with("my t", "open", Some("core"), &[PARENT], "my body");
    let comparison = compare(&base, &theirs, &mine);
    assert_eq!(
        comparison.fields,
        [
            (Field::Title, Change::Mine),
            (Field::Type, Change::None),
            (Field::Status, Change::Theirs),
            (Field::Project, Change::None),
            (Field::Team, Change::BothSame),
            (Field::Deps, Change::Both),
            (Field::Parent, Change::None),
            (Field::Body, Change::Both),
        ]
    );
}

#[test]
fn a_comment_compares_its_body_only() {
    let body = |text: &str| DraftFields::Comment {
        body: text.to_owned(),
    };
    let comparison = compare(&body("a"), &body("b"), &body("b"));
    assert_eq!(comparison.fields, [(Field::Body, Change::BothSame)]);
}

// ---- target state ----

#[test]
fn an_unchanged_item_at_the_base_observation_is_current() {
    let read = item("Base", "obs-1", "main");
    assert!(matches!(
        classify(&ticket_draft(), Ok(&read)),
        TargetState::Current
    ));
}

#[test]
fn read_errors_map_to_their_states_by_code() {
    let draft = ticket_draft();
    for code in [
        ResultCode::RepositoryNotRegistered,
        ResultCode::RepositoryInaccessible,
        ResultCode::NotRepository,
        ResultCode::NotRepositoryRoot,
        ResultCode::RepositoryNotEnabled,
        ResultCode::InvalidPath,
    ] {
        match classify(&draft, Err(code)) {
            TargetState::RepositoryUnavailable(got) => assert_eq!(got, code),
            _ => panic!("{code:?} is RepositoryUnavailable"),
        }
        match classify_comment(&comment_draft(None), Err(code), Err(ResultCode::Busy)) {
            TargetState::RepositoryUnavailable(got) => assert_eq!(got, code),
            _ => panic!("{code:?} is RepositoryUnavailable for a comment draft"),
        }
    }
    assert!(matches!(
        classify(&draft, Err(ResultCode::ItemNotFound)),
        TargetState::ItemNotFound
    ));
    for code in [
        ResultCode::Busy,
        ResultCode::IndexUnavailable,
        ResultCode::InternalError,
    ] {
        match classify(&draft, Err(code)) {
            TargetState::ReadFailed(got) => assert_eq!(got, code),
            _ => panic!("{code:?} is ReadFailed"),
        }
    }
}

#[test]
fn a_closed_ticket_is_closed_whatever_else_holds() {
    // At the base observation: it would otherwise be current.
    let read = closed(item("Base", "obs-1", "main"));
    assert!(matches!(
        classify(&ticket_draft(), Ok(&read)),
        TargetState::TicketClosed
    ));
    // With the draft's own fields: it would otherwise be already saved.
    let read = closed(item("Mine", "obs-2", "main"));
    assert!(matches!(
        classify(&ticket_draft(), Ok(&read)),
        TargetState::TicketClosed
    ));
}

#[test]
fn a_nonconforming_read_is_nonconforming() {
    let read = nonconforming(item("Base", "obs-2", "main"));
    assert!(matches!(
        classify(&ticket_draft(), Ok(&read)),
        TargetState::Nonconforming
    ));
}

#[test]
fn fields_equal_to_the_drafts_at_another_observation_are_already_saved() {
    let read = item("Mine", "obs-2", "main");
    assert!(matches!(
        classify(&ticket_draft(), Ok(&read)),
        TargetState::AlreadySaved
    ));
    // Before the unverified base is considered.
    let mut draft = ticket_draft();
    draft.base.as_mut().unwrap().observation = None;
    assert!(matches!(
        classify(&draft, Ok(&read)),
        TargetState::AlreadySaved
    ));
}

#[test]
fn a_base_with_no_observation_is_unverified_and_compared_with_the_fields_it_holds() {
    let mut draft = ticket_draft();
    draft.base.as_mut().unwrap().observation = None;
    // Even at what would be the same observation.
    let read = item("Theirs", "obs-1", "main");
    match classify(&draft, Ok(&read)) {
        TargetState::Unverified { theirs, comparison } => {
            assert!(theirs == ticket("Theirs", "Body\n"));
            assert_eq!(comparison.fields[0], (Field::Title, Change::Both));
            assert!(
                comparison.fields[1..]
                    .iter()
                    .all(|(_, c)| *c == Change::None)
            );
        }
        _ => panic!("an unverified base is Unverified"),
    }
}

#[test]
fn a_draft_with_no_base_is_unverified() {
    let draft = draft(None, ticket("Mine", "Body\n"));
    let read = item("Theirs", "obs-1", "main");
    assert!(matches!(
        classify(&draft, Ok(&read)),
        TargetState::Unverified { .. }
    ));
}

#[test]
fn another_observation_in_the_same_context_is_changed() {
    let read = item("Theirs", "obs-2", "main");
    match classify(&ticket_draft(), Ok(&read)) {
        TargetState::Changed {
            theirs,
            comparison,
            context_changed,
        } => {
            assert!(!context_changed);
            assert!(theirs == ticket("Theirs", "Body\n"));
            assert_eq!(comparison.fields[0], (Field::Title, Change::Both));
        }
        _ => panic!("another observation is Changed"),
    }
}

#[test]
fn another_branch_or_context_kind_sets_context_changed() {
    let read = item("Base", "obs-2", "manyhands/ticket/x");
    match classify(&ticket_draft(), Ok(&read)) {
        TargetState::Changed {
            context_changed, ..
        } => assert!(context_changed),
        _ => panic!("another branch is Changed"),
    }
    // The same branch name in another kind of context.
    let mut read = item("Base", "obs-2", "main");
    read.context.kind = ItemContextKind::Unverified;
    match classify(&ticket_draft(), Ok(&read)) {
        TargetState::Changed {
            context_changed, ..
        } => assert!(context_changed),
        _ => panic!("another context kind is Changed"),
    }
}

#[test]
fn a_comment_draft_is_current_while_its_item_and_parent_are_there() {
    let read = item("Base", "obs-9", "main");
    let list = comments(true, &[(DEP, None), (PARENT, Some(DEP))]);
    for parent in [None, Some(DEP), Some(PARENT)] {
        assert!(matches!(
            classify_comment(&comment_draft(parent), Ok(&read), Ok(&list)),
            TargetState::Current
        ));
    }
}

#[test]
fn a_reply_whose_parent_is_not_in_the_thread_has_no_parent() {
    let read = item("Base", "obs-9", "main");
    let list = comments(true, &[(DEP, None)]);
    assert!(matches!(
        classify_comment(&comment_draft(Some(PARENT)), Ok(&read), Ok(&list)),
        TargetState::ParentNotFound
    ));
}

#[test]
fn an_incomplete_comment_list_is_a_failed_read_never_a_missing_parent() {
    let read = item("Base", "obs-9", "main");
    let list = comments(false, &[]);
    for parent in [None, Some(PARENT)] {
        assert!(matches!(
            classify_comment(&comment_draft(parent), Ok(&read), Ok(&list)),
            TargetState::ReadFailed(_)
        ));
    }
}

#[test]
fn a_comment_drafts_item_decides_before_its_thread() {
    let list = comments(true, &[]);
    let draft = comment_draft(Some(PARENT));
    assert!(matches!(
        classify_comment(&draft, Err(ResultCode::ItemNotFound), Ok(&list)),
        TargetState::ItemNotFound
    ));
    assert!(matches!(
        classify_comment(&draft, Err(ResultCode::RepositoryNotRegistered), Ok(&list)),
        TargetState::RepositoryUnavailable(ResultCode::RepositoryNotRegistered)
    ));
    let read = closed(item("Base", "obs-9", "main"));
    assert!(matches!(
        classify_comment(&draft, Ok(&read), Ok(&list)),
        TargetState::TicketClosed
    ));
    let read = item("Base", "obs-9", "main");
    assert!(matches!(
        classify_comment(&draft, Ok(&read), Err(ResultCode::Busy)),
        TargetState::ReadFailed(ResultCode::Busy)
    ));
    assert!(matches!(
        classify_comment(&draft, Ok(&read), Err(ResultCode::ItemNotFound)),
        TargetState::ItemNotFound
    ));
}

// ---- restore policy ----

fn state_of(name: &str) -> TargetState {
    let comparison = Comparison { fields: Vec::new() };
    match name {
        "current" => TargetState::Current,
        "changed" => TargetState::Changed {
            theirs: ticket("T", "B"),
            comparison,
            context_changed: false,
        },
        "context_changed" => TargetState::Changed {
            theirs: ticket("T", "B"),
            comparison,
            context_changed: true,
        },
        "unverified" => TargetState::Unverified {
            theirs: ticket("T", "B"),
            comparison,
        },
        "already_saved" => TargetState::AlreadySaved,
        "ticket_closed" => TargetState::TicketClosed,
        "item_not_found" => TargetState::ItemNotFound,
        "nonconforming" => TargetState::Nonconforming,
        "parent_not_found" => TargetState::ParentNotFound,
        "repository_unavailable" => {
            TargetState::RepositoryUnavailable(ResultCode::RepositoryNotRegistered)
        }
        "read_failed" => TargetState::ReadFailed(ResultCode::Busy),
        _ => unreachable!(),
    }
}

const STATES: [&str; 11] = [
    "current",
    "changed",
    "context_changed",
    "unverified",
    "already_saved",
    "ticket_closed",
    "item_not_found",
    "nonconforming",
    "parent_not_found",
    "repository_unavailable",
    "read_failed",
];

#[test]
fn restore_follows_the_policy_table_row_by_row() {
    let draft = ticket_draft();
    let expected = |name: &str| match name {
        "current" | "already_saved" => Restore::Allowed,
        "changed" | "context_changed" | "unverified" => Restore::AllowedFlagged,
        "ticket_closed" => Restore::Refused(RefusalReason::TicketClosed),
        "item_not_found" => Restore::Refused(RefusalReason::ItemNotFound),
        "nonconforming" => Restore::Refused(RefusalReason::Nonconforming),
        "parent_not_found" => Restore::Refused(RefusalReason::ParentNotFound),
        "repository_unavailable" => Restore::Refused(RefusalReason::RepositoryUnavailable),
        "read_failed" => Restore::Refused(RefusalReason::ReadFailed),
        _ => unreachable!(),
    };
    for name in STATES {
        assert_eq!(
            restore_policy(&draft, &state_of(name), Held::No),
            expected(name),
            "{name}"
        );
    }
}

#[test]
fn a_draft_open_elsewhere_is_refused_and_one_held_here_needs_no_restore() {
    let draft = ticket_draft();
    for name in STATES {
        assert_eq!(
            restore_policy(&draft, &state_of(name), Held::Elsewhere),
            Restore::Refused(RefusalReason::OpenElsewhere),
            "{name}"
        );
        assert_eq!(
            restore_policy(&draft, &state_of(name), Held::Here),
            Restore::Refused(RefusalReason::HeldHere),
            "{name}"
        );
    }
}

#[test]
fn a_comment_draft_is_always_refused() {
    let draft = comment_draft(None);
    for name in STATES {
        assert_eq!(
            restore_policy(&draft, &state_of(name), Held::No),
            Restore::Refused(RefusalReason::CommentDraft),
            "{name}"
        );
    }
}

// ---- new-item drafts (R-20) ----

/// A new item's draft: its base has neither observation nor source, as
/// only a draft written before its item's create has (R-20). Its text is
/// the "Mine" ticket.
fn new_item_draft() -> Draft {
    let mut base = base(None, ticket("Mine", "Body\n"));
    base.source = None;
    draft(Some(base), ticket("Mine", "Body\n"))
}

#[test]
fn a_new_item_draft_of_a_missing_item_is_not_created_and_restorable() {
    let draft = new_item_draft();
    assert!(is_new_item_base(draft.base.as_ref().unwrap()));
    let state = classify(&draft, Err(ResultCode::ItemNotFound));
    assert!(matches!(state, TargetState::NotCreated));
    assert_eq!(restore_policy(&draft, &state, Held::No), Restore::Allowed);
    // Who holds it decides first, as for every draft.
    assert_eq!(
        restore_policy(&draft, &state, Held::Elsewhere),
        Restore::Refused(RefusalReason::OpenElsewhere)
    );
    // Any other read error is the read's.
    assert!(matches!(
        classify(&draft, Err(ResultCode::Busy)),
        TargetState::ReadFailed(ResultCode::Busy)
    ));
}

#[test]
fn a_new_item_draft_whose_item_exists_with_other_fields_is_unverified() {
    let draft = new_item_draft();
    let read = item("Theirs", "obs-1", "main");
    match classify(&draft, Ok(&read)) {
        TargetState::Unverified { theirs, comparison } => {
            assert!(theirs == ticket("Theirs", "Body\n"));
            // Compared with the fields the base holds.
            assert_eq!(comparison.fields[0], (Field::Title, Change::Theirs));
        }
        _ => panic!("an existing item with other fields is Unverified"),
    }
}

#[test]
fn a_new_item_draft_whose_item_exists_with_equal_fields_is_already_saved() {
    let draft = new_item_draft();
    let read = item("Mine", "obs-1", "main");
    assert!(matches!(
        classify(&draft, Ok(&read)),
        TargetState::AlreadySaved
    ));
}

#[test]
fn an_unverified_base_with_a_source_is_still_item_not_found_when_missing() {
    // D2's unverified rule: no observation, and the source `""`.
    let mut draft = new_item_draft();
    draft.base.as_mut().unwrap().source = Some(String::new());
    assert!(!is_new_item_base(draft.base.as_ref().unwrap()));
    let state = classify(&draft, Err(ResultCode::ItemNotFound));
    assert!(matches!(state, TargetState::ItemNotFound));
    assert_eq!(
        restore_policy(&draft, &state, Held::No),
        Restore::Refused(RefusalReason::ItemNotFound)
    );
    // Nor is a draft with no base at all, nor one with an observation.
    for draft in [
        draft_with_no_base(),
        draft_with_an_observation_and_no_source(),
    ] {
        assert!(matches!(
            classify(&draft, Err(ResultCode::ItemNotFound)),
            TargetState::ItemNotFound
        ));
    }
}

fn draft_with_no_base() -> Draft {
    draft(None, ticket("Mine", "Body\n"))
}

/// A non-UTF-8 item's base has no source, but it has an observation.
fn draft_with_an_observation_and_no_source() -> Draft {
    let mut draft = ticket_draft();
    draft.base.as_mut().unwrap().source = None;
    assert!(!is_new_item_base(draft.base.as_ref().unwrap()));
    draft
}
