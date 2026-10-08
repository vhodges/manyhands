//! The comment threads of one item: their order, their authors, the files
//! that are not comments, and the copy they are read from.

// A read error carries its whole scope by value, as the contract has it.
#![allow(clippy::result_large_err)]

use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use manyhands::{
    repository::{
        CommentDto, CommentListDto, IndexState, ItemContextKind, ReadError, RepositoryService,
        ResolvedRepository,
    },
    results::{Outcome, ProblemCode, ResultCode},
};
use serde_json::{Value, json};
use support::items::{
    COMMENT_A, COMMENT_B, COMMENT_C, COMMENT_D, COMMENT_E, DOCUMENT_A, DOCUMENT_B, TICKET_A,
    TICKET_B, comment_path, comment_source, commit, context_worktree, create_document_context,
    document_source, index, item_id, refresh_completely, ticket_path, ticket_source, write,
    write_comment,
};

mod support;

const COMMENT_F: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE5";

const ADA: &str = "Ada Lovelace <ada@example.invalid>";

/// Planted where it must not be published: in YAML that fails to parse,
/// where the parser's message repeats it, and in a file a link points at.
const SENTINEL: &str = "SENTINEL-71c3";

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

/// A repository with one ticket, `TICKET_A`, not yet refreshed.
fn repository_with_ticket() -> (support::TestRepository, support::EnabledRepository) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    write(
        &fixture.root,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "Commented", ""),
    );
    (fixture, enabled)
}

fn time(second: u8) -> String {
    format!("2026-09-30T12:00:{second:02}Z")
}

fn comments(service: &RepositoryService, repo: &ResolvedRepository, item: &str) -> CommentListDto {
    let list = service.list_comments(repo, &item_id(item)).unwrap();
    assert_threaded(&list.items);
    list
}

/// What rebuilding the threads from the flat list relies on: an entry has
/// no parent exactly when its depth is 0, and otherwise its parent is the
/// nearest earlier entry one level up.
fn assert_threaded(comments: &[CommentDto]) {
    for (index, comment) in comments.iter().enumerate() {
        let above = comment.depth.checked_sub(1).and_then(|depth| {
            comments[..index]
                .iter()
                .rev()
                .find(|earlier| earlier.depth == depth)
        });
        match above {
            None => assert_eq!((comment.depth, &comment.parent_id), (0, &None), "{index}"),
            Some(parent) => {
                assert!(parent.id.is_some(), "{index}");
                assert_eq!(comment.parent_id, parent.id, "{index}");
            }
        }
    }
}

fn depths(comments: &[CommentDto]) -> Vec<u32> {
    comments.iter().map(|comment| comment.depth).collect()
}

fn ids(comments: &[CommentDto]) -> Vec<Option<&str>> {
    comments
        .iter()
        .map(|comment| comment.id.as_deref())
        .collect()
}

fn codes(comment: &CommentDto) -> Vec<ProblemCode> {
    comment
        .problems
        .iter()
        .map(|problem| problem.code)
        .collect()
}

fn recovery(error: &ReadError) -> Value {
    serde_json::to_value(&error.to_envelope::<Value>("comment list").recovery).unwrap()
}

/// What every nonconforming entry is: where the file is, why it is not a
/// comment, and nothing read from it.
fn assert_nonconforming(comment: &CommentDto, item: &str, path: &str, expected: &[ProblemCode]) {
    assert_eq!(comment.id, None, "{comment:?}");
    assert_eq!(comment.item_id, item);
    assert_eq!(comment.parent_id, None);
    assert_eq!(comment.author, None);
    assert_eq!(comment.created_at, None);
    assert_eq!(comment.body, None);
    assert_eq!(comment.path, path);
    assert!(comment.unknown_metadata.is_empty());
    assert_eq!(comment.depth, 0);
    assert_eq!(codes(comment), expected, "{path}");
    for problem in &comment.problems {
        assert_eq!(problem.path.as_deref(), Some(path));
    }
}

fn set_modified(path: &Path, modified: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

#[test]
fn roots_and_replies_are_ordered_by_time_then_id_to_depth_three_with_bodies() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    // Written in an order that is neither the expected one nor ID order.
    write_comment(root, TICKET_A, COMMENT_F, None, &time(2), "");
    write_comment(root, TICKET_A, COMMENT_A, None, &time(2), "");
    write_comment(root, TICKET_A, COMMENT_B, None, &time(1), "");
    write_comment(root, TICKET_A, COMMENT_C, Some(COMMENT_B), &time(5), "");
    write_comment(root, TICKET_A, COMMENT_D, Some(COMMENT_B), &time(3), "");
    write_comment(root, TICKET_A, COMMENT_E, Some(COMMENT_D), &time(4), "");
    // Another item's comment is not this item's.
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "Other", ""),
    );
    write_comment(
        root,
        TICKET_B,
        "01ARZ3NDEKTSV4RRFFQ69G5FE9",
        None,
        &time(0),
        "",
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert!(list.complete);
    assert_eq!(list.index.state, IndexState::Current);
    assert_eq!(list.context.kind, ItemContextKind::Primary);
    assert_eq!(Path::new(&list.context.worktree), repo.root());
    // The earlier root first; the two created in one second in ID order.
    // Each comment is followed at once by its replies, the earlier first,
    // and each of those by its own.
    assert_eq!(
        ids(&list.items),
        [
            Some(COMMENT_B),
            Some(COMMENT_D),
            Some(COMMENT_E),
            Some(COMMENT_C),
            Some(COMMENT_A),
            Some(COMMENT_F)
        ]
    );
    assert_eq!(depths(&list.items), [0, 1, 2, 1, 0, 0]);
    let first = &list.items[0];
    let second_level = &list.items[1];
    let third_level = &list.items[2];
    assert_eq!(list.items[3].parent_id.as_deref(), Some(COMMENT_B));
    assert_eq!(list.items[4].parent_id, None);
    assert_eq!(list.items[5].parent_id, None);

    assert_eq!(first.item_id, TICKET_A);
    assert_eq!(first.parent_id, None);
    assert_eq!(first.created_at.as_deref(), Some("2026-09-30T12:00:01Z"));
    assert_eq!(first.path, comment_path(TICKET_A, COMMENT_B));
    assert_eq!(
        first.body.as_deref(),
        Some(format!("Body of {COMMENT_B}.\n").as_str())
    );
    assert_eq!(second_level.parent_id.as_deref(), Some(COMMENT_B));
    assert_eq!(
        second_level.body.as_deref(),
        Some(format!("Body of {COMMENT_D}.\n").as_str())
    );
    assert_eq!(third_level.parent_id.as_deref(), Some(COMMENT_D));
    assert_eq!(third_level.item_id, TICKET_A);
    assert_eq!(third_level.path, comment_path(TICKET_A, COMMENT_E));
    assert_eq!(
        third_level.created_at.as_deref(),
        Some("2026-09-30T12:00:04Z")
    );
    assert_eq!(
        third_level.body.as_deref(),
        Some(format!("Body of {COMMENT_E}.\n").as_str())
    );
    for comment in &list.items {
        assert_eq!(comment.author, None);
        assert!(comment.problems.is_empty());
        assert!(comment.unknown_metadata.is_empty());
    }
    assert_git_transport_uninitialized();
}

/// How deeply the arrays and objects of `value` are nested.
fn nesting(value: &Value) -> usize {
    match value {
        Value::Array(values) => 1 + values.iter().map(nesting).max().unwrap_or(0),
        Value::Object(fields) => 1 + fields.values().map(nesting).max().unwrap_or(0),
        _ => 0,
    }
}

#[test]
fn a_long_chain_of_replies_is_listed_flat_and_nests_no_deeper_than_one_comment() {
    const LENGTH: usize = 200;
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    let chain: Vec<String> = (0..LENGTH)
        .map(|index| format!("01ARZ3NDEKTSV4RRFFQ69G{index:04}"))
        .collect();
    // Every comment is a reply to the one before it, all made in the same
    // second.
    for (index, id) in chain.iter().enumerate() {
        let parent = index.checked_sub(1).map(|parent| chain[parent].as_str());
        write_comment(root, TICKET_A, id, parent, &time(1), "");
    }
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(list.index.state, IndexState::Current);
    assert_eq!(
        ids(&list.items),
        chain.iter().map(|id| Some(id.as_str())).collect::<Vec<_>>()
    );
    assert_eq!(
        depths(&list.items),
        (0..u32::try_from(LENGTH).unwrap()).collect::<Vec<_>>()
    );
    // A parser with the default recursion limit reads it, as it reads the
    // list of a single comment: the list, its items, a comment, and what a
    // comment holds.
    let json = serde_json::to_string(&list).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["items"].as_array().unwrap().len(), LENGTH);
    assert_eq!(nesting(&parsed), 4);
    assert_git_transport_uninitialized();
}

#[test]
fn author_is_created_by_which_is_never_unknown_metadata() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(
        root,
        TICKET_A,
        COMMENT_A,
        None,
        &time(1),
        &format!("created_by: {ADA}\nmood: calm\n"),
    );
    write_comment(root, TICKET_A, COMMENT_B, None, &time(2), "mood: quiet\n");
    // Neither is an identity: one is not a string, one is empty.
    write_comment(root, TICKET_A, COMMENT_C, None, &time(3), "created_by: 7\n");
    write_comment(
        root,
        TICKET_A,
        COMMENT_D,
        None,
        &time(4),
        "created_by: \"\"\n",
    );
    write_comment(
        root,
        TICKET_A,
        COMMENT_E,
        None,
        &time(5),
        &format!("created_by: {ADA}\ntagged: !secret hidden\n"),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(
        ids(&list.items),
        [
            Some(COMMENT_A),
            Some(COMMENT_B),
            Some(COMMENT_C),
            Some(COMMENT_D),
            Some(COMMENT_E)
        ]
    );
    let [recorded, absent, number, empty, tagged] = list.items.as_slice() else {
        unreachable!()
    };
    assert_eq!(recorded.author.as_deref(), Some(ADA));
    assert_eq!(
        Value::Object(recorded.unknown_metadata.clone()),
        json!({"mood": "calm"})
    );
    assert!(recorded.problems.is_empty());

    assert_eq!(absent.author, None);
    assert_eq!(
        Value::Object(absent.unknown_metadata.clone()),
        json!({"mood": "quiet"})
    );
    assert!(absent.problems.is_empty());

    // Still a comment, with no author, and the bad value reported.
    for (comment, id) in [(number, COMMENT_C), (empty, COMMENT_D)] {
        assert_eq!(comment.id.as_deref(), Some(id));
        assert_eq!(comment.author, None);
        assert!(comment.unknown_metadata.is_empty(), "{comment:?}");
        assert_eq!(codes(comment), [ProblemCode::InvalidField]);
        assert_eq!(
            comment.problems[0].path.as_deref(),
            Some(comment_path(TICKET_A, id).as_str())
        );
        assert!(comment.body.is_some());
    }

    assert_eq!(tagged.author.as_deref(), Some(ADA));
    assert_eq!(
        Value::Object(tagged.unknown_metadata.clone()),
        json!({"tagged": null})
    );
    assert_eq!(codes(tagged), [ProblemCode::MetadataNotRepresentable]);

    // The key is published nowhere, under any name but `author`.
    let serialized = serde_json::to_string(&list).unwrap();
    assert!(!serialized.contains("created_by"), "{serialized}");
    assert_eq!(list.index.state, IndexState::Current);
    assert_git_transport_uninitialized();
}

#[test]
fn files_that_are_not_comments_end_the_root_list_with_null_ids_paths_and_codes() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    // Its parent is no comment at all.
    write_comment(root, TICKET_A, COMMENT_B, Some(COMMENT_F), &time(2), "");
    // Its parent is a file that is not a comment.
    write_comment(root, TICKET_A, COMMENT_C, Some(COMMENT_B), &time(3), "");
    let malformed = comment_path(TICKET_A, COMMENT_D);
    write(
        root,
        &malformed,
        &format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\n\
             {SENTINEL}: 1\n{SENTINEL}: 2\n---\nBody with {SENTINEL}.\n"
        ),
    );
    // Filed under this item and naming another.
    let misfiled = comment_path(TICKET_A, COMMENT_E);
    write(
        root,
        &misfiled,
        &comment_source(TICKET_B, COMMENT_E, None, &time(4), ""),
    );
    let misnamed = format!(".manyhands/comments/{TICKET_A}/notes.md");
    write(root, &misnamed, "---\nmanyhands_managed: true\n---\n");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // The sentinel really is in the index, so the scan below can fail.
    let stored: String = index(enabled.data_directory.path())
        .query_row(
            "SELECT guidance FROM problems WHERE code = 'malformed-front-matter'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(stored.contains(SENTINEL), "{stored}");

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(list.index.state, IndexState::Current);
    assert_eq!(
        ids(&list.items),
        [Some(COMMENT_A), None, None, None, None, None]
    );
    assert_eq!(depths(&list.items), [0; 6]);
    // In path order after every comment.
    let reply_to_nothing = comment_path(TICKET_A, COMMENT_B);
    let reply_to_that = comment_path(TICKET_A, COMMENT_C);
    assert_nonconforming(
        &list.items[1],
        TICKET_A,
        &reply_to_nothing,
        &[ProblemCode::MissingParent],
    );
    assert_nonconforming(
        &list.items[2],
        TICKET_A,
        &reply_to_that,
        &[ProblemCode::MissingParent],
    );
    assert_nonconforming(
        &list.items[3],
        TICKET_A,
        &malformed,
        &[ProblemCode::MalformedFrontMatter],
    );
    assert_nonconforming(
        &list.items[4],
        TICKET_A,
        &misfiled,
        &[ProblemCode::InvalidField],
    );
    assert_nonconforming(
        &list.items[5],
        TICKET_A,
        &misnamed,
        &[ProblemCode::InvalidPath],
    );
    // Fixed guidance, and nothing of the parser's or the file's.
    let serialized = serde_json::to_string(&list).unwrap();
    assert!(!serialized.contains(SENTINEL), "{serialized}");
    assert_eq!(
        serde_json::to_value(&list.items[3].problems[0]).unwrap(),
        json!({
            "code": "malformed_front_matter",
            "path": malformed,
            "target_id": null,
            "guidance": ProblemCode::MalformedFrontMatter.guidance(),
        })
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_parent_cycle_is_reported_on_each_comment_in_it() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, Some(COMMENT_B), &time(1), "");
    write_comment(root, TICKET_A, COMMENT_B, Some(COMMENT_A), &time(2), "");
    write_comment(root, TICKET_A, COMMENT_C, None, &time(3), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&list.items), [Some(COMMENT_C), None, None]);
    for (entry, id) in [(&list.items[1], COMMENT_A), (&list.items[2], COMMENT_B)] {
        assert_nonconforming(
            entry,
            TICKET_A,
            &comment_path(TICKET_A, id),
            &[ProblemCode::CommentCycle],
        );
    }
    assert_eq!(list.index.state, IndexState::Current);
    assert_git_transport_uninitialized();
}

#[test]
fn comments_of_an_item_with_an_active_worktree_are_read_from_that_worktree() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let root = &fixture.root;
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/active.md");
    let worktree = context_worktree(root, DOCUMENT_A);
    let file = write_comment(
        &worktree,
        DOCUMENT_A,
        COMMENT_A,
        None,
        &time(1),
        &format!("created_by: {ADA}\n"),
    );
    // A copy of the item's comments in the root is not the effective one.
    write_comment(root, DOCUMENT_A, COMMENT_B, None, &time(2), "");
    // An item with no worktree keeps its comments in the root, and a copy
    // of them in another item's worktree is not the effective one either.
    write(root, "docs/b.md", &document_source(DOCUMENT_B, "B", ""));
    write_comment(root, DOCUMENT_B, COMMENT_C, None, &time(3), "");
    write_comment(&worktree, DOCUMENT_B, COMMENT_D, None, &time(4), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let active = comments(&enabled.service, &repo, DOCUMENT_A);
    let primary = comments(&enabled.service, &repo, DOCUMENT_B);

    assert_eq!(ids(&active.items), [Some(COMMENT_A)]);
    assert_eq!(active.items[0].author.as_deref(), Some(ADA));
    assert_eq!(active.context.kind, ItemContextKind::Active);
    assert_eq!(Path::new(&active.context.worktree), worktree);
    assert_eq!(
        active.context.branch,
        Some(format!("manyhands/document/{DOCUMENT_A}"))
    );
    assert_eq!(active.index.state, IndexState::Current);
    assert_eq!(ids(&primary.items), [Some(COMMENT_C)]);
    assert_eq!(primary.context.kind, ItemContextKind::Primary);
    assert_eq!(Path::new(&primary.context.worktree), repo.root());
    assert_eq!(primary.index.state, IndexState::Current);

    // Edited in the worktree and not refreshed: the read returns what is
    // there now and says the index is behind.
    fs::write(
        &file,
        comment_source(DOCUMENT_A, COMMENT_A, None, &time(1), "")
            .replace("Body of", "Edited body of"),
    )
    .unwrap();
    set_modified(&file, SystemTime::now() + Duration::from_secs(30));
    let edited = comments(&enabled.service, &repo, DOCUMENT_A);

    assert_eq!(
        edited.items[0].body.as_deref(),
        Some(format!("Edited body of {COMMENT_A}.\n").as_str())
    );
    assert_eq!(edited.items[0].author, None);
    assert_eq!(edited.index.state, IndexState::Stale);
    assert_eq!(edited.context, active.context);
    assert_git_transport_uninitialized();
}

#[test]
fn an_uncommitted_comment_is_read_and_unrelated_commits_change_nothing() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    commit(&fixture, &[&ticket_path(TICKET_A)], 1_700_000_000);
    write_comment(
        root,
        TICKET_A,
        COMMENT_A,
        None,
        &time(1),
        &format!("created_by: {ADA}\n"),
    );
    write_comment(root, TICKET_A, COMMENT_B, Some(COMMENT_A), &time(2), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let status = fixture
        .repository
        .status_file(Path::new(&comment_path(TICKET_A, COMMENT_A)))
        .unwrap();
    assert!(status.contains(git2::Status::WT_NEW), "{status:?}");

    let before = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&before.items), [Some(COMMENT_A), Some(COMMENT_B)]);
    assert_eq!(depths(&before.items), [0, 1]);
    assert_eq!(before.items[0].author.as_deref(), Some(ADA));
    assert_eq!(before.index.state, IndexState::Current);

    // Commits by somebody else, to other files. The author is not theirs,
    // and nothing about the comments moves.
    for (name, seconds) in [("one.txt", 1_700_000_100), ("two.txt", 1_700_000_200)] {
        write(root, name, "unrelated\n");
        commit(&fixture, &[name], seconds);
    }
    let after = comments(&enabled.service, &repo, TICKET_A);

    assert!(after == before, "{after:?}");

    // Nor after the index has seen those commits.
    refresh_completely(&enabled.service, root);
    let refreshed = comments(&enabled.service, &repo, TICKET_A);

    assert!(refreshed.items == before.items, "{refreshed:?}");
    assert_ne!(refreshed.context.head_oid, before.context.head_oid);
    assert_eq!(refreshed.index.state, IndexState::Current);
    let status = fixture
        .repository
        .status_file(Path::new(&comment_path(TICKET_A, COMMENT_A)))
        .unwrap();
    assert!(status.contains(git2::Status::WT_NEW), "{status:?}");
    assert_git_transport_uninitialized();
}

#[test]
fn an_item_without_comments_has_none_and_an_unknown_item_is_not_found() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write_comment(root, DOCUMENT_A, COMMENT_A, None, &time(1), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let none = comments(&enabled.service, &repo, TICKET_A);

    assert!(none.items.is_empty());
    assert!(none.complete);
    assert_eq!(none.index.state, IndexState::Current);
    assert_eq!(none.context.kind, ItemContextKind::Primary);
    assert_eq!(
        ids(&comments(&enabled.service, &repo, DOCUMENT_A).items),
        [Some(COMMENT_A)]
    );

    // No item has this ID, and a comment is not an item.
    for id in [TICKET_B, COMMENT_A] {
        let error = enabled
            .service
            .list_comments(&repo, &item_id(id))
            .unwrap_err();
        assert_eq!(error.code(), ResultCode::ItemNotFound, "{id}");
        assert_eq!(error.scope.item_id.as_deref(), Some(id));
        assert_eq!(error.scope.repository.as_deref(), repo.root().to_str());
        // The index is current, so a refresh would find nothing more.
        assert_eq!(recovery(&error), json!([]));
        assert_eq!(
            error.to_envelope::<Value>("comment list").outcome,
            Outcome::Error
        );
    }

    // The item's file is gone: a refresh is the recovery, as it is for the
    // item itself.
    fs::remove_file(root.join(ticket_path(TICKET_A))).unwrap();
    let error = enabled
        .service
        .list_comments(&repo, &item_id(TICKET_A))
        .unwrap_err();
    assert_eq!(error.code(), ResultCode::ItemNotFound);
    assert_eq!(
        recovery(&error),
        json!([{
            "action": "index.refresh",
            "operation_id": null,
            "arguments": {"root": repo.root().to_str().unwrap()},
        }])
    );
    assert_git_transport_uninitialized();
}

#[test]
fn comments_changed_since_the_refresh_are_read_as_they_are_and_the_index_is_stale() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    let first = write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    let second = write_comment(root, TICKET_A, COMMENT_B, Some(COMMENT_A), &time(2), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let stored = comments(&enabled.service, &repo, TICKET_A);
    assert_eq!(stored.index.state, IndexState::Current);
    assert_eq!(ids(&stored.items), [Some(COMMENT_A), Some(COMMENT_B)]);
    assert_eq!(depths(&stored.items), [0, 1]);

    // A reply made a root, with a modification time that hides the edit:
    // what the index stored for it is no longer so.
    fs::write(
        &second,
        comment_source(TICKET_A, COMMENT_B, None, &time(2), ""),
    )
    .unwrap();
    set_modified(&second, UNIX_EPOCH + Duration::from_secs(1_000));
    let reparented = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&reparented.items), [Some(COMMENT_A), Some(COMMENT_B)]);
    assert_eq!(depths(&reparented.items), [0, 0]);
    assert_eq!(reparented.index.state, IndexState::Stale);
    assert_eq!(reparented.index.refreshed_at, stored.index.refreshed_at);

    // A file the index has not seen is not listed until it has.
    refresh_completely(&enabled.service, root);
    write_comment(root, TICKET_A, COMMENT_C, None, &time(3), "");
    let unseen = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&unseen.items), [Some(COMMENT_A), Some(COMMENT_B)]);
    assert_eq!(unseen.index.state, IndexState::Current);
    refresh_completely(&enabled.service, root);
    assert_eq!(
        ids(&comments(&enabled.service, &repo, TICKET_A).items),
        [Some(COMMENT_A), Some(COMMENT_B), Some(COMMENT_C)]
    );

    // A file that is gone is left out, and the index is behind.
    fs::remove_file(&first).unwrap();
    let removed = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&removed.items), [Some(COMMENT_B), Some(COMMENT_C)]);
    assert_eq!(removed.index.state, IndexState::Stale);

    // A file that no longer parses is listed as what it now is.
    fs::write(&second, "no front matter\n").unwrap();
    let broken = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&broken.items), [Some(COMMENT_C), None]);
    assert_nonconforming(
        &broken.items[1],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_B),
        &[ProblemCode::MissingFrontMatter],
    );
    assert_eq!(broken.index.state, IndexState::Stale);

    // Once the index holds that problem the list is current again, and
    // behind again when the file it is a problem with is gone.
    refresh_completely(&enabled.service, root);
    let indexed = comments(&enabled.service, &repo, TICKET_A);
    assert!(indexed.items == broken.items, "{indexed:?}");
    assert_eq!(indexed.index.state, IndexState::Current);
    fs::remove_file(&second).unwrap();
    let gone = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&gone.items), [Some(COMMENT_C)]);
    assert_eq!(gone.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn comments_of_an_item_whose_file_changed_follow_what_the_file_now_is() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let ticket = root.join(ticket_path(TICKET_A));

    // The item no longer parses, so in its context no comment has an item:
    // what a refresh would now store.
    fs::write(&ticket, "no front matter\n").unwrap();
    let orphaned = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&orphaned.items), [None]);
    assert_nonconforming(
        &orphaned.items[0],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_A),
        &[ProblemCode::MissingCommentItem],
    );
    assert_eq!(orphaned.index.state, IndexState::Stale);

    // Another item is now where this one was.
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write_comment(root, DOCUMENT_A, COMMENT_B, None, &time(1), "");
    refresh_completely(&enabled.service, root);
    fs::write(root.join("docs/a.md"), document_source(DOCUMENT_B, "B", "")).unwrap();
    let error = enabled
        .service
        .list_comments(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::ItemNotFound);
    assert_ne!(recovery(&error), json!([]));
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn no_comment_is_read_through_a_symbolic_link() {
    use std::os::unix::fs::symlink;

    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("target.md");
    fs::write(
        &target,
        comment_source(TICKET_A, COMMENT_A, None, &time(1), "").replace("Body of", SENTINEL),
    )
    .unwrap();
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    let file = write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    // A link the refresh itself sees and reports.
    let linked = comment_path(TICKET_A, COMMENT_B);
    symlink(&target, root.join(&linked)).unwrap();
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&list.items), [Some(COMMENT_A), None]);
    assert_nonconforming(
        &list.items[1],
        TICKET_A,
        &linked,
        &[ProblemCode::SourceUnreadable],
    );
    assert_eq!(list.index.state, IndexState::Current);
    assert!(!serde_json::to_string(&list).unwrap().contains(SENTINEL));

    // A link that replaced a comment the index holds.
    fs::remove_file(&file).unwrap();
    symlink(&target, &file).unwrap();
    let replaced = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&replaced.items), [None, None]);
    assert_nonconforming(
        &replaced.items[0],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_A),
        &[ProblemCode::SourceUnreadable],
    );
    assert_eq!(replaced.index.state, IndexState::Stale);
    assert!(!serde_json::to_string(&replaced).unwrap().contains(SENTINEL));

    // A link that replaced the item's whole comment directory.
    let directory = root.join(".manyhands/comments").join(TICKET_A);
    let elsewhere = outside.path().join("comments");
    fs::create_dir(&elsewhere).unwrap();
    fs::copy(&target, elsewhere.join(format!("{COMMENT_A}.md"))).unwrap();
    fs::remove_dir_all(&directory).unwrap();
    symlink(&elsewhere, &directory).unwrap();
    let redirected = comments(&enabled.service, &repo, TICKET_A);

    assert!(
        !serde_json::to_string(&redirected)
            .unwrap()
            .contains(SENTINEL)
    );
    // Both files the index knows of are still listed, as files that cannot
    // be reached, and nothing is read through the link.
    assert_eq!(ids(&redirected.items), [None, None]);
    for (entry, id) in [
        (&redirected.items[0], COMMENT_A),
        (&redirected.items[1], COMMENT_B),
    ] {
        assert_nonconforming(
            entry,
            TICKET_A,
            &comment_path(TICKET_A, id),
            &[ProblemCode::SourceUnreadable],
        );
    }
    assert_eq!(redirected.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn an_index_row_that_points_outside_the_items_comments_is_not_read_from() {
    let list = |change: &str| {
        let (fixture, enabled) = repository_with_ticket();
        let root = &fixture.root;
        write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
        write(
            root,
            "docs/a.md",
            &document_source(DOCUMENT_A, SENTINEL, ""),
        );
        write(root, "plain.md", SENTINEL);
        write_comment(root, DOCUMENT_A, COMMENT_B, None, &time(1), SENTINEL);
        refresh_completely(&enabled.service, root);
        let repo = enabled.service.resolve_repository(root).unwrap();
        index(enabled.data_directory.path())
            .execute_batch(change)
            .unwrap();
        enabled.service.list_comments(&repo, &item_id(TICKET_A))
    };

    // Untouched, the fixture reads.
    assert_eq!(ids(&list("SELECT 1").unwrap().items), [Some(COMMENT_A)]);
    // A comment row is trusted only for a file in the item's own comment
    // directory.
    let other_item = comment_path(DOCUMENT_A, COMMENT_B);
    let nested = format!(".manyhands/comments/{TICKET_A}/sub/{COMMENT_A}.md");
    let escaping = format!(".manyhands/comments/{TICKET_A}/../../../plain.md");
    for path in [
        "plain.md",
        "docs/a.md",
        "../plain.md",
        "/etc/hostname",
        other_item.as_str(),
        nested.as_str(),
        escaping.as_str(),
        "",
    ] {
        let error = list(&format!(
            "UPDATE discovered_comments SET canonical_path = '{path}'
              WHERE comment_id = '{COMMENT_A}'"
        ))
        .unwrap_err();
        assert_eq!(error.code(), ResultCode::InternalError, "{path}");
        assert_eq!(recovery(&error), json!([]));
    }
    // A problem row that names a file anywhere else is not this item's to
    // list, whatever it is a problem with.
    for path in [
        "plain.md",
        "docs/a.md",
        other_item.as_str(),
        nested.as_str(),
        escaping.as_str(),
    ] {
        let listed = list(&format!(
            "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
             SELECT repository_id, id, '{path}', 'missing-front-matter', '', 0 FROM contexts"
        ))
        .unwrap();
        assert_eq!(ids(&listed.items), [Some(COMMENT_A)], "{path}");
        assert!(!serde_json::to_string(&listed).unwrap().contains(SENTINEL));
    }
    assert_git_transport_uninitialized();
}

/// Whether permission bits stop this user from reading. They do not for a
/// user that may read anything, and then a test of a refusal has nothing
/// to observe: it says so and stops.
#[cfg(unix)]
fn permissions_bind_or_skip(test: &str, probe: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(probe).unwrap().permissions().mode();
    fs::set_permissions(probe, fs::Permissions::from_mode(0o000)).unwrap();
    let bind = fs::read(probe).is_err();
    fs::set_permissions(probe, fs::Permissions::from_mode(mode)).unwrap();
    if !bind {
        eprintln!(
            "SKIPPED {test}: this user can read a file with mode 000, \
             so no open can be refused here"
        );
    }
    bind
}

#[cfg(unix)]
#[test]
fn a_comment_that_may_not_be_opened_fails_the_read_as_inaccessible() {
    use std::os::unix::fs::PermissionsExt;

    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    let file = write_comment(root, TICKET_A, COMMENT_B, None, &time(2), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    if !permissions_bind_or_skip(
        "a_comment_that_may_not_be_opened_fails_the_read_as_inaccessible",
        &file,
    ) {
        return;
    }

    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    let result = enabled.service.list_comments(&repo, &item_id(TICKET_A));
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();

    // Not a shorter list that says it is complete.
    let error = result.unwrap_err();
    assert_eq!(error.code(), ResultCode::RepositoryInaccessible);
    assert_eq!(error.scope.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(recovery(&error), json!([]));
    assert_eq!(
        error.to_envelope::<Value>("comment list").outcome,
        Outcome::Blocked
    );
    assert_eq!(
        ids(&comments(&enabled.service, &repo, TICKET_A).items),
        [Some(COMMENT_A), Some(COMMENT_B)]
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_degraded_index_and_a_removed_registration_fail_the_read() {
    let (fixture, enabled) = repository_with_ticket();
    refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    index(enabled.data_directory.path())
        .execute("DELETE FROM repositories", [])
        .unwrap();
    let error = enabled
        .service
        .list_comments(&repo, &item_id(TICKET_A))
        .unwrap_err();
    assert_eq!(error.code(), ResultCode::RepositoryNotRegistered);

    let (_data, service) = support::items::degraded_service(enabled);
    let error = service
        .list_comments(&repo, &item_id(TICKET_A))
        .unwrap_err();
    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    assert_eq!(error.scope.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(
        recovery(&error),
        json!([{
            "action": "index.rebuild",
            "operation_id": null,
            "arguments": {"root": repo.root().to_str().unwrap()},
        }])
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_comment_id_two_items_share_is_reported_from_what_the_refresh_stored() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "Other", ""),
    );
    // One ID under two items: each file is a comment of its item by every
    // rule its own item's files can show.
    let shared = write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    write_comment(root, TICKET_B, COMMENT_A, None, &time(1), "");
    write_comment(root, TICKET_A, COMMENT_B, Some(COMMENT_A), &time(2), "");
    write_comment(root, TICKET_A, COMMENT_C, None, &time(3), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&list.items), [Some(COMMENT_C), None, None]);
    assert_nonconforming(
        &list.items[1],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_A),
        &[ProblemCode::DuplicateId],
    );
    // Its parent is not a comment, as the refresh found too.
    assert_nonconforming(
        &list.items[2],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_B),
        &[ProblemCode::MissingParent],
    );
    assert_eq!(list.index.state, IndexState::Current);
    assert!(list.complete);
    let other = comments(&enabled.service, &repo, TICKET_B);
    assert_eq!(ids(&other.items), [None]);
    assert_nonconforming(
        &other.items[0],
        TICKET_B,
        &comment_path(TICKET_B, COMMENT_A),
        &[ProblemCode::DuplicateId],
    );
    assert_eq!(other.index.state, IndexState::Current);

    // Changed since the refresh, the file is checked as it is now, by what
    // this item's files show, and the index is behind.
    set_modified(&shared, SystemTime::now() + Duration::from_secs(30));
    let changed = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(
        ids(&changed.items),
        [Some(COMMENT_A), Some(COMMENT_B), Some(COMMENT_C)]
    );
    assert_eq!(depths(&changed.items), [0, 1, 0]);
    assert_eq!(changed.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn a_reply_to_another_items_comment_is_reported_from_what_the_refresh_stored() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "Other", ""),
    );
    write_comment(root, TICKET_B, COMMENT_A, None, &time(1), "");
    let reply = write_comment(root, TICKET_A, COMMENT_B, Some(COMMENT_A), &time(2), "");
    write_comment(root, TICKET_A, COMMENT_C, Some(COMMENT_B), &time(3), "");
    write_comment(root, TICKET_A, COMMENT_D, None, &time(4), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&list.items), [Some(COMMENT_D), None, None]);
    assert_nonconforming(
        &list.items[1],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_B),
        &[ProblemCode::CrossItemParent],
    );
    assert_nonconforming(
        &list.items[2],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_C),
        &[ProblemCode::MissingParent],
    );
    assert_eq!(list.index.state, IndexState::Current);
    assert_eq!(
        ids(&comments(&enabled.service, &repo, TICKET_B).items),
        [Some(COMMENT_A)]
    );

    // Changed since the refresh: this item's files show only that the
    // parent is no comment of it.
    set_modified(&reply, SystemTime::now() + Duration::from_secs(30));
    let changed = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&changed.items), [Some(COMMENT_D), None, None]);
    assert_nonconforming(
        &changed.items[1],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_B),
        &[ProblemCode::MissingParent],
    );
    assert_eq!(changed.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn a_file_the_refresh_could_not_open_is_listed_as_unreadable_while_it_still_is() {
    use std::os::unix::fs::PermissionsExt;

    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    let file = write_comment(root, TICKET_A, COMMENT_B, None, &time(2), "");
    if !permissions_bind_or_skip(
        "a_file_the_refresh_could_not_open_is_listed_as_unreadable_while_it_still_is",
        &file,
    ) {
        return;
    }
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let result = enabled.service.list_comments(&repo, &item_id(TICKET_A));
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();

    let list = result.unwrap();
    assert_eq!(ids(&list.items), [Some(COMMENT_A), None]);
    assert_nonconforming(
        &list.items[1],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_B),
        &[ProblemCode::SourceUnreadable],
    );
    assert_eq!(list.index.state, IndexState::Current);
    assert!(list.complete);
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn the_list_is_not_complete_when_the_refresh_could_not_read_the_comment_directory() {
    use std::os::unix::fs::symlink;

    let outside = tempfile::tempdir().unwrap();
    fs::write(
        outside.path().join(format!("{COMMENT_A}.md")),
        comment_source(TICKET_A, COMMENT_A, None, &time(1), ""),
    )
    .unwrap();
    for linked in [
        format!(".manyhands/comments/{TICKET_A}"),
        ".manyhands/comments".to_owned(),
    ] {
        let (fixture, enabled) = repository_with_ticket();
        let root = &fixture.root;
        write(
            root,
            &ticket_path(TICKET_B),
            &ticket_source(TICKET_B, "Other", ""),
        );
        if linked.ends_with(TICKET_A) {
            write_comment(root, TICKET_B, COMMENT_B, None, &time(1), "");
        }
        symlink(outside.path(), root.join(&linked)).unwrap();
        refresh_completely(&enabled.service, root);
        let repo = enabled.service.resolve_repository(root).unwrap();

        let list = comments(&enabled.service, &repo, TICKET_A);

        // Nothing is read through the link, and the list says that it may
        // not be everything.
        assert!(list.items.is_empty(), "{linked}: {list:?}");
        assert!(!list.complete, "{linked}");
        assert_eq!(list.index.state, IndexState::Current, "{linked}");
        // An item whose own directory could be read is not affected by
        // another item's.
        let other = comments(&enabled.service, &repo, TICKET_B);
        assert_eq!(other.complete, linked.ends_with(TICKET_A), "{linked}");
    }
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn a_changed_item_file_or_a_problem_file_that_is_no_longer_a_file_is_stale() {
    use std::os::unix::fs::symlink;

    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    let malformed = write(
        root,
        &comment_path(TICKET_A, COMMENT_B),
        "no front matter\n",
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let stored = comments(&enabled.service, &repo, TICKET_A);
    assert_eq!(ids(&stored.items), [Some(COMMENT_A), None]);
    assert_eq!(stored.index.state, IndexState::Current);

    // The item's own file is newer than the refresh.
    let ticket = root.join(ticket_path(TICKET_A));
    let modified = fs::metadata(&ticket).unwrap().modified().unwrap();
    set_modified(&ticket, SystemTime::now() + Duration::from_secs(30));
    let item_changed = comments(&enabled.service, &repo, TICKET_A);

    assert!(item_changed.items == stored.items);
    assert_eq!(item_changed.index.state, IndexState::Stale);
    set_modified(&ticket, modified);
    assert_eq!(
        comments(&enabled.service, &repo, TICKET_A).index.state,
        IndexState::Current
    );

    // A file the index holds a conformity problem for is now a link.
    fs::remove_file(&malformed).unwrap();
    symlink(&ticket, &malformed).unwrap();
    let linked = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&linked.items), [Some(COMMENT_A), None]);
    assert_nonconforming(
        &linked.items[1],
        TICKET_A,
        &comment_path(TICKET_A, COMMENT_B),
        &[ProblemCode::SourceUnreadable],
    );
    assert_eq!(linked.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn comments_of_an_item_the_index_holds_twice_come_from_the_chosen_copy_and_are_stale() {
    let (fixture, enabled) = repository_with_ticket();
    let root = &fixture.root;
    write_comment(root, TICKET_A, COMMENT_A, None, &time(1), "");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // A second row for the item, in its own worktree, as a refresh that has
    // stored one context and not yet removed the other leaves it.
    let worktree = repo.root().join(".manyhands/worktrees").join(TICKET_A);
    let connection = index(enabled.data_directory.path());
    connection
        .execute(
            "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id)
             SELECT id, 'active', ?1, ?2, ?3 FROM repositories",
            rusqlite::params![
                format!("manyhands/ticket/{TICKET_A}"),
                worktree.to_str().unwrap(),
                TICKET_A
            ],
        )
        .unwrap();
    let context_row = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO discovered_items
                (context_id, item_id, kind, canonical_path, title, ticket_type, status,
                 activity_at, activity_source)
             VALUES (?1, ?2, 'ticket', ?3, 'Worktree row', 'task', 'open', 5, 'git')",
            rusqlite::params![context_row, TICKET_A, ticket_path(TICKET_A)],
        )
        .unwrap();
    let item_row = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO discovered_comments
                (item_id, comment_id, canonical_path, created_at)
             VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![item_row, COMMENT_B, comment_path(TICKET_A, COMMENT_B)],
        )
        .unwrap();
    drop(connection);

    // The worktree is not there: the primary row is the one with a file
    // behind it, and its comments are the ones read.
    let from_primary = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&from_primary.items), [Some(COMMENT_A)]);
    assert_eq!(from_primary.context.kind, ItemContextKind::Primary);
    assert_eq!(Path::new(&from_primary.context.worktree), repo.root());
    assert_eq!(from_primary.index.state, IndexState::Stale);

    // The worktree is there: its row is the effective one, and only the
    // comments stored under that row, read from that worktree, are listed.
    write(
        &worktree,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "Commented", ""),
    );
    write_comment(&worktree, TICKET_A, COMMENT_B, None, &time(0), "");
    write_comment(
        &worktree,
        TICKET_A,
        COMMENT_A,
        None,
        &time(9),
        "edited: here\n",
    );
    let from_worktree = comments(&enabled.service, &repo, TICKET_A);

    assert_eq!(ids(&from_worktree.items), [Some(COMMENT_B)]);
    assert_eq!(from_worktree.context.kind, ItemContextKind::Active);
    assert_eq!(Path::new(&from_worktree.context.worktree), worktree);
    assert_eq!(from_worktree.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}
