//! The read-only item view, over real repositories: provenance, closure
//! and status, unknown metadata, source, discussion, a nonconforming
//! entry, a rejected context, a vanished item, and that opening writes
//! nothing.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, VisualTestContext};
use manyhands::{
    repository::{ItemDto, ItemDtoKind, TicketFilter},
    results::{ProblemCode, ResultCode},
};

use super::{NONCONFORMING_FILE, SOURCE_NOT_UTF8};
use crate::desktop::{
    model::{
        item::{
            KIND_ACTIVE, KIND_PRIMARY, LATER, LATER_NOTE, NOT_REPRESENTABLE, UNKNOWN_AUTHOR,
            provenance_label,
        },
        navigation::OpenTarget,
    },
    services::requests::{Intent, RequestState},
    test_support::{
        fixtures::{Fixture, item_id, ticket_path},
        snapshot::RepositorySnapshot,
    },
    view::harness::{Shell, key},
};

const PRIMARY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const SAVED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
const SAYS_CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD5";
const COMMENT_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
const COMMENT_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
const COMMENT_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
const ADA: &str = "Ada Lovelace <ada@example.invalid>";

/// One repository, `a`, built by `build` and refreshed, in an open
/// window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn present(shell: &Shell, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        gpui_kit::test::TestWindowExt::try_find(window, id).is_some()
    })
}

/// A ticket whose only copy is the primary one says Primary; one saved
/// through the library, which provisions its own item context, says
/// Active item worktree with that context's branch and worktree.
#[gpui_kit::test]
fn the_provenance_says_primary_or_active_with_its_branch_and_worktree(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "In primary", "");
        fixture.commit_ticket(root, SAVED, "To be saved", "");
    });
    shell.fixture.save_ticket(&root, SAVED, "Saved");

    open(&shell, ticket(&root, PRIMARY), cx);
    let primary = shown(&shell, &root, PRIMARY);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("In primary"));
    assert_eq!(shell.label("item-kind", cx).as_deref(), Some("Ticket"));
    assert_eq!(shell.label("item-id", cx).as_deref(), Some(PRIMARY));
    assert_eq!(
        shell.label("provenance-kind", cx).as_deref(),
        Some(KIND_PRIMARY)
    );
    assert_eq!(
        shell.label("provenance-branch", cx).as_deref(),
        Some("main")
    );
    assert_eq!(shell.label("provenance-worktree", cx), Some(key(&root)));
    assert_eq!(shell.label("provenance-head", cx), primary.context.head_oid);
    assert_eq!(
        shell.label("provenance", cx),
        Some(provenance_label(&primary.context))
    );

    open(&shell, ticket(&root, SAVED), cx);
    let saved = shown(&shell, &root, SAVED);
    let worktree = root.join(".manyhands/worktrees").join(SAVED);
    assert_eq!(saved.context.worktree, key(&worktree));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Saved"));
    assert_eq!(
        shell.label("provenance-kind", cx).as_deref(),
        Some(KIND_ACTIVE)
    );
    assert_eq!(
        shell.label("provenance-branch", cx),
        Some(format!("manyhands/ticket/{SAVED}"))
    );
    assert_eq!(shell.label("provenance-worktree", cx), Some(key(&worktree)));
    assert_eq!(
        shell.label("provenance", cx),
        Some(provenance_label(&saved.context))
    );
}

/// Lifecycle closure and project status are separate fields: a closed
/// ticket says Closed, with when and by whom, and its status as written;
/// an open ticket whose status says closed is Open.
#[gpui_kit::test]
fn lifecycle_closure_and_project_status_are_separate_fields(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_closed_ticket(root, CLOSED, "Closed for good");
        fixture.create_ticket(root, SAYS_CLOSED, "Says closed", "closed");
    });

    open(&shell, ticket(&root, CLOSED), cx);
    let closed = shown(&shell, &root, CLOSED);
    let closure = closed.closure.clone().unwrap();
    assert_eq!(shell.label("item-lifecycle", cx).as_deref(), Some("Closed"));
    assert_eq!(shell.label("item-closed-at", cx), closure.closed_at);
    assert_eq!(shell.label("item-closed-by", cx), closure.closed_by);
    assert_eq!(shell.label("item-status", cx).as_deref(), Some("done"));

    open(&shell, ticket(&root, SAYS_CLOSED), cx);
    assert_eq!(shell.label("item-lifecycle", cx).as_deref(), Some("Open"));
    assert_eq!(shell.label("item-status", cx).as_deref(), Some("closed"));
    assert!(!present(&shell, "item-closed-at", cx));
}

/// Unknown metadata is listed by key, in the library's order, as the
/// library's own JSON or text; a value JSON cannot express is labeled not
/// representable, and its problem is listed.
#[gpui_kit::test]
fn unknown_metadata_is_listed_and_a_null_value_is_not_representable(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(
            root,
            PRIMARY,
            "Odd",
            "zeta: last\ntagged: !secret hidden\nnested:\n  deep: [1, 2.5, true]\n",
        );
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    let item = shown(&shell, &root, PRIMARY);
    let keys: Vec<&str> = item.unknown_metadata.keys().map(String::as_str).collect();
    assert_eq!(keys, ["nested", "tagged", "zeta"]);

    assert_eq!(shell.rows("meta-key", cx), keys);
    assert_eq!(
        shell.rows("meta-value", cx),
        [r#"{"deep":[1,2.5,true]}"#, NOT_REPRESENTABLE, "last"]
    );
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::MetadataNotRepresentable.as_str()]
    );
    assert_eq!(
        shell.rows("item-problem-guidance", cx),
        [ProblemCode::MetadataNotRepresentable.guidance()]
    );
}

/// The source is the file's text exactly; a file that is not UTF-8 says
/// it has none to show.
#[gpui_kit::test]
fn the_source_is_shown_verbatim_and_a_non_utf8_file_says_so(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Plain", "extra: kept\n");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    let item = shown(&shell, &root, PRIMARY);
    assert!(item.source.as_deref().unwrap().contains("extra: kept\n"));
    assert_eq!(shell.label("item-source", cx), item.source);
    assert!(!present(&shell, "item-source-missing", cx));

    shell.fixture.write_non_utf8(&root, &ticket_path(PRIMARY));
    open(&shell, ticket(&root, PRIMARY), cx);
    assert_eq!(shown(&shell, &root, PRIMARY).source, None);
    assert_eq!(
        shell.label("item-source-missing", cx).as_deref(),
        Some(SOURCE_NOT_UTF8)
    );
    assert!(!present(&shell, "item-source", cx));
}

/// Comments are in the library's order, each indented by its depth, with
/// its author, or "unknown author" when it has none.
#[gpui_kit::test]
fn comments_are_indented_by_depth_in_list_order(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Discussed", "");
        let by_ada = format!("created_by: {ADA}\n");
        fixture.commit_comment(
            root,
            PRIMARY,
            COMMENT_A,
            None,
            "2026-09-30T12:00:01Z",
            &by_ada,
        );
        fixture.commit_comment(
            root,
            PRIMARY,
            COMMENT_B,
            Some(COMMENT_A),
            "2026-09-30T12:00:02Z",
            "",
        );
        fixture.commit_comment(
            root,
            PRIMARY,
            COMMENT_C,
            None,
            "2026-09-30T12:00:03Z",
            &by_ada,
        );
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let list = service.list_comments(&repo, &item_id(PRIMARY)).unwrap();
    let order: Vec<Option<&str>> = list.items.iter().map(|c| c.id.as_deref()).collect();
    assert_eq!(order, [Some(COMMENT_A), Some(COMMENT_B), Some(COMMENT_C)]);
    let depths: Vec<u32> = list.items.iter().map(|c| c.depth).collect();
    assert_eq!(depths, [0, 1, 0]);

    let bodies: Vec<String> = list.items.iter().map(|c| c.body.clone().unwrap()).collect();
    assert_eq!(shell.rows("comment-body", cx), bodies);
    assert_eq!(shell.rows("comment-author", cx), [ADA, UNKNOWN_AUTHOR, ADA]);
    let times: Vec<String> = list
        .items
        .iter()
        .map(|c| c.created_at.clone().unwrap())
        .collect();
    assert_eq!(shell.rows("comment-time", cx), times);

    let x = |index: usize, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            gpui_kit::test::TestWindowExt::find(window, ("comment", index))
                .bounds()
                .origin
                .x
        })
    };
    let (a, b, c) = (x(0, cx), x(1, cx), x(2, cx));
    assert!(b > a, "the reply is indented: {a:?} {b:?}");
    assert_eq!(a, c);
}

/// A nonconforming entry opens by its path: it says so, shows its
/// problem code with its guidance, and Repair is visibly unavailable.
#[gpui_kit::test]
fn a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_unavailable(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_ticket(root, BROKEN);
    });
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let list = service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let entry = list.items.iter().find(|item| item.id.is_none()).unwrap();
    let target = OpenTarget::for_item(&key(&root), entry);
    assert!(matches!(target, OpenTarget::Inspect { .. }));
    open(&shell, target, cx);

    assert_eq!(
        shell.label("item-nonconforming", cx).as_deref(),
        Some(NONCONFORMING_FILE)
    );
    assert_eq!(shell.label("item-path", cx), Some(entry.path.clone()));
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::MalformedFrontMatter.as_str()]
    );
    assert_eq!(
        shell.rows("item-problem-guidance", cx),
        [ProblemCode::MalformedFrontMatter.guidance()]
    );
    // Its source is still shown, and it has no discussion.
    assert!(present(&shell, "item-source", cx));
    assert!(!present(&shell, "comments-state", cx));

    // Every later action is shown, and says it is not available yet.
    assert_eq!(shell.rows("later-action", cx), LATER);
    let notes = shell.rows("later-note", cx);
    assert_eq!(notes.len(), LATER.len());
    assert!(notes.iter().all(|note| note == LATER_NOTE));
    let repair = LATER.iter().position(|action| *action == "Repair").unwrap();
    assert_eq!(
        shell.label(("later-action", repair), cx).as_deref(),
        Some("Repair")
    );
    // The control does nothing: the view is as it was.
    let before = shell.rows("item-problem", cx);
    shell.click(("later-button", repair), cx);
    assert_eq!(shell.rows("item-problem", cx), before);
}

/// The duplicate-context recipe of the library's discovery tests: the
/// document's own worktree gets a second source with its ID, and a
/// refresh rejects the context. The library reports the context problem
/// in the index status only; the item read answers `item_not_found`. The
/// view shows that answer with Refresh, and offers no choice of context.
#[gpui_kit::test]
fn an_item_whose_context_is_rejected_shows_the_librarys_answer_and_no_context_choice(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, "docs/first.md", "First");
    });
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    };
    open(&shell, target.clone(), cx);
    assert_eq!(
        shell.label("provenance-kind", cx).as_deref(),
        Some(KIND_ACTIVE)
    );

    shell.fixture.commit_duplicate_context(&root, DOCUMENT);
    shell.fixture.refresh(&root);
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let status = service.index_status(&repo).unwrap();
    let context_path = format!(".manyhands/worktrees/{DOCUMENT}");
    assert!(status.problems.iter().any(|problem| {
        problem.code == ProblemCode::ContextProblem
            && problem.path.as_deref() == Some(context_path.as_str())
    }));
    assert_eq!(
        service
            .show_item(&repo, &item_id(DOCUMENT))
            .unwrap_err()
            .code(),
        ResultCode::ItemNotFound
    );

    open(&shell, target, cx);
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::ItemNotFound.as_str())
    );
    assert!(present(&shell, "item-refresh", cx));
    // Nothing offers a context: no provenance and no copy is claimed.
    for id in [
        "provenance",
        "provenance-kind",
        "item-source",
        "item-recovery",
    ] {
        assert!(!present(&shell, id, cx), "{id}");
    }
}

/// An item whose file is deleted, once the index is refreshed, shows
/// `item_not_found` with Refresh; Refresh refreshes the index through the
/// worker and reads the item again.
#[gpui_kit::test]
async fn a_deleted_item_shows_item_not_found_with_refresh(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Going", "");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Going"));

    shell.fixture.commit_removal(&root, &ticket_path(PRIMARY));
    shell.fixture.refresh(&root);
    open(&shell, ticket(&root, PRIMARY), cx);
    assert_eq!(
        shell.label("item-failure", cx).as_deref(),
        Some(ResultCode::ItemNotFound.message())
    );
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::ItemNotFound.as_str())
    );
    assert!(!present(&shell, "item-title", cx));

    shell.click("item-refresh", cx);
    let operations = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().operations().clone());
    let mut visual = VisualTestContext::from_window(shell.window.into(), cx);
    visual
        .condition(&operations, |operations, _| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::IndexRefresh { .. })
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    visual.run_until_parked();
    let refreshed = operations.read_with(&visual, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(
                |request| matches!(&request.intent, Intent::IndexRefresh { root: r } if *r == root),
            )
            .count()
    });
    assert_eq!(refreshed, 1);
    // Read again after the refresh: still gone, still with Refresh.
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::ItemNotFound.as_str())
    );
    assert!(present(&shell, "item-refresh", cx));
}

/// Copy actions write the ID or the short code to the clipboard, each on
/// its own.
#[gpui_kit::test]
fn copy_writes_the_id_to_the_clipboard(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Copied", "");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    shell.click("copy-id", cx);
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some(PRIMARY)
    );
}

/// Opening a ticket with comments, a document in its own worktree and a
/// nonconforming entry changes nothing in the repository.
#[gpui_kit::test]
fn opening_an_item_writes_nothing(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Read", "");
        fixture.commit_comment(root, PRIMARY, COMMENT_A, None, "2026-09-30T12:00:01Z", "");
        fixture.create_document(root, DOCUMENT, "docs/first.md", "First");
        fixture.commit_nonconforming_ticket(root, BROKEN);
    });
    let before = RepositorySnapshot::take(&root);
    let worktree = root.join(".manyhands/worktrees").join(DOCUMENT);
    let worktree_before = RepositorySnapshot::take(&worktree);

    open(&shell, ticket(&root, PRIMARY), cx);
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let list = service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let entry = list.items.iter().find(|item| item.id.is_none()).unwrap();
    open(&shell, OpenTarget::for_item(&key(&root), entry), cx);
    shell.keys("secondary-3", cx);

    assert_eq!(RepositorySnapshot::take(&root), before);
    assert_eq!(RepositorySnapshot::take(&worktree), worktree_before);
}
