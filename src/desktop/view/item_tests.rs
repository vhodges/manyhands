//! The read-only item view, over real repositories: provenance, closure
//! and status, unknown metadata, source, discussion, a nonconforming
//! entry, a rejected context, a vanished item, and that opening writes
//! nothing.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, px};
use manyhands::{
    repository::{ItemDto, ItemDtoKind, TicketFilter},
    results::{ProblemCode, ResultCode},
};
use time::OffsetDateTime;

use super::{
    NONCONFORMING_FILE, SOURCE_NOT_UTF8,
    discussion::{REPLY_INDENT, offset_at, shown_time},
};
use crate::desktop::{
    model::{
        item::{
            KIND_ACTIVE, KIND_PRIMARY, LATER, LATER_NOTE, NOT_REPRESENTABLE, UNKNOWN_AUTHOR,
            provenance_label,
        },
        navigation::OpenTarget,
    },
    services::{
        JobKind,
        requests::{Intent, Lane, RequestState},
    },
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

/// Opens `target` in a new tab, as after closing it: an item already open
/// would only have its tab shown, not be read again.
fn reopen(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    let repo = match &target {
        OpenTarget::Item { repo, .. } | OpenTarget::Inspect { repo, .. } => repo.clone(),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.close_repository(&repo, cx));
    open(shell, target, cx);
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

/// E-23: opens the shown tab's details sheet with All details, by click.
fn details(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("chip-all", cx);
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
    details(&shell, cx);
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
    details(&shell, cx);
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
    details(&shell, cx);
    let closed = shown(&shell, &root, CLOSED);
    let closure = closed.closure.clone().unwrap();
    assert_eq!(shell.label("item-lifecycle", cx).as_deref(), Some("Closed"));
    assert_eq!(shell.label("item-closed-at", cx), closure.closed_at);
    assert_eq!(shell.label("item-closed-by", cx), closure.closed_by);
    assert_eq!(shell.label("item-status", cx).as_deref(), Some("done"));

    open(&shell, ticket(&root, SAYS_CLOSED), cx);
    details(&shell, cx);
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
    details(&shell, cx);
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
    // E-1: the assertions run in the item's Inspect tab (R-64).
    inspect_from_sheet(&shell, cx);
    let item = shown(&shell, &root, PRIMARY);
    assert!(item.source.as_deref().unwrap().contains("extra: kept\n"));
    assert!(present(&shell, "item-source", cx));
    assert_eq!(source_value(&shell, cx), item.source);
    assert!(!present(&shell, "item-source-missing", cx));

    shell.fixture.write_non_utf8(&root, &ticket_path(PRIMARY));
    reopen(&shell, ticket(&root, PRIMARY), cx);
    assert_eq!(shown(&shell, &root, PRIMARY).source, None);
    assert_eq!(
        shell.label("item-source-missing", cx).as_deref(),
        Some(SOURCE_NOT_UTF8)
    );
    assert!(!present(&shell, "item-source", cx));
}

/// E-1, E-2: opens the shown item's Inspect tab from its details sheet,
/// with All details and then the sheet's Inspect link (R-64). The link
/// sits below the sheet's fold, so it is reached by Tab and pressed by
/// Enter.
fn inspect_from_sheet(shell: &Shell, cx: &mut TestAppContext) {
    details(shell, cx);
    let mut presses = 0;
    while !shell.frame(cx, |window, _| {
        gpui_kit::test::TestWindowExt::try_find(window, "sheet-inspect")
            .is_some_and(|link| link.focused() == Some(true))
    }) {
        presses += 1;
        assert!(presses < 40, "Tab never reached the Inspect link");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);
}

/// The text the item view shows the source in, after a frame.
fn source_value(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.frame(cx, |_, _| ());
    shell.workspace.read_with(cx, |workspace, cx| {
        let pane = workspace.item_pane()?;
        let state = pane.read(cx).source_state()?.clone();
        Some(state.read(cx).value().to_string())
    })
}

/// The source is read-only: it can take focus, but typing, deleting,
/// cutting and pasting into it change neither the text shown nor the
/// file, and nothing in the repository.
#[gpui_kit::test]
fn the_source_rejects_every_edit(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Kept", "");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    // E-2: in the item's Inspect tab (R-64).
    inspect_from_sheet(&shell, cx);
    let source = shown(&shell, &root, PRIMARY).source.unwrap();
    assert_eq!(source_value(&shell, cx).as_deref(), Some(source.as_str()));
    let before = RepositorySnapshot::take(&root);
    let file = std::fs::read(root.join(ticket_path(PRIMARY))).unwrap();

    let state = shell.workspace.read_with(cx, |workspace, cx| {
        workspace
            .item_pane()
            .unwrap()
            .read(cx)
            .source_state()
            .unwrap()
            .clone()
    });
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| state.update(cx, |state, cx| state.focus(window, cx)));
    visual.run_until_parked();
    assert!(visual.update(|window, cx| {
        gpui_kit::Focusable::focus_handle(state.read(cx), cx).is_focused(window)
    }));
    assert!(!visual.update(|_, cx| state.read(cx).is_editable()));
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("pasted".to_owned()));
    shell.keys(
        "x y z enter backspace delete secondary-a secondary-x secondary-v",
        cx,
    );

    assert_eq!(source_value(&shell, cx).as_deref(), Some(source.as_str()));
    assert_eq!(
        std::fs::read(root.join(ticket_path(PRIMARY))).unwrap(),
        file
    );
    assert_eq!(RepositorySnapshot::take(&root), before);
    // Read again, the item is as it was.
    assert_eq!(
        shown(&shell, &root, PRIMARY).source.as_deref(),
        Some(source.as_str())
    );
}

/// Comments are in the library's order. Roots are not indented, every
/// reply sits at the one reply indent, and only a reply deeper than one
/// level says whom it answers (R-53). Each shows its author without the
/// email, or "unknown author" when it has none, its time at the offset
/// passed, and its body in its read editor.
#[gpui_kit::test]
fn comments_keep_list_order_and_replies_share_one_indent(cx: &mut TestAppContext) {
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
    shell.frame(cx, |_, _| ());
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .unwrap();
    let read: Vec<String> = list
        .items
        .iter()
        .map(|c| {
            let view = pane
                .read_with(cx, |pane, _| pane.comment_view(c.id.as_deref().unwrap()))
                .unwrap();
            view.read_with(cx, |view, cx| view.editor().read(cx).text().to_owned())
        })
        .collect();
    assert_eq!(read, bodies);
    let ada = "Ada Lovelace";
    assert_eq!(shell.rows("comment-author", cx), [ada, UNKNOWN_AUTHOR, ada]);
    let times: Vec<String> = list
        .items
        .iter()
        .map(|c| c.created_at.clone().unwrap())
        .collect();
    let now = OffsetDateTime::now_utc();
    let shown: Vec<String> = times
        .iter()
        .map(|at| shown_time(at, now, offset_at(at)))
        .collect();
    assert_eq!(shell.rows("comment-time", cx), shown);

    let x = |index: usize, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            gpui_kit::test::TestWindowExt::find(window, ("comment", index))
                .bounds()
                .origin
                .x
        })
    };
    let root = x(0, cx);
    for (ix, comment) in list.items.iter().enumerate() {
        let indent = if comment.depth == 0 { 0. } else { REPLY_INDENT };
        assert_eq!(x(ix, cx) - root, px(indent), "{ix}: its indent");
        assert_eq!(
            present(&shell, ("comment-reply-to", ix), cx),
            comment.depth > 1,
            "{ix}: only a reply deeper than one says whom it answers"
        );
    }
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

    // The header's later actions and Repair in the "Needs fixing" banner
    // are shown, and each says it is not available yet. An Inspect tab has
    // no discussion, so Comment is not among them, and the header's
    // primary action is the ticket's, Close ticket….
    let later: Vec<&str> = LATER
        .into_iter()
        .filter(|action| !["Comment", "Approve and merge…"].contains(action))
        .collect();
    assert_eq!(shell.rows("later-action", cx), later);
    let notes = shell.rows("later-note", cx);
    assert_eq!(notes.len(), later.len());
    assert!(notes.iter().all(|note| note == LATER_NOTE));
    let repair = later.iter().position(|action| *action == "Repair").unwrap();
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
    details(&shell, cx);
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

    reopen(&shell, target, cx);
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
    reopen(&shell, ticket(&root, PRIMARY), cx);
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
    // The refresh Refresh submitted: the user's, of this root. A
    // background refresh at launch is not it.
    let submitted = operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .find(|request| {
                request.lane == Lane::User
                    && matches!(&request.intent, Intent::IndexRefresh { root: r } if *r == root)
            })
            .map(|request| request.id)
            .expect("Refresh submitted a refresh")
    });
    shell.until_ended(submitted, cx).await;
    let refreshed = operations.read_with(cx, |operations, _| {
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
    details(&shell, cx);
    shell.click("copy-id", cx);
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some(PRIMARY)
    );
}

/// Each copy action leaves exactly its own value on the clipboard: the
/// short code, then the ULID, then the short code again.
#[gpui_kit::test]
fn each_copy_leaves_exactly_the_short_code_or_the_ulid(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Copied", "slug: ae-k9x2b\n");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    details(&shell, cx);
    let clipboard = |cx: &mut TestAppContext| cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(shell.label("item-slug", cx).as_deref(), Some("ae-k9x2b"));
    for (button, wanted) in [
        ("copy-slug", "ae-k9x2b"),
        ("copy-id", PRIMARY),
        ("copy-slug", "ae-k9x2b"),
    ] {
        shell.click(button, cx);
        assert_eq!(clipboard(cx).as_deref(), Some(wanted), "{button}");
    }
}

/// A ticket with no short code offers no short code to copy.
#[gpui_kit::test]
fn a_ticket_without_a_short_code_offers_none_to_copy(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Uncoded", "");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    details(&shell, cx);
    assert!(!present(&shell, "copy-slug", cx));
    assert!(present(&shell, "copy-id", cx));
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

/// Refresh that is emptied from the queue on quit has ended: the item
/// is read again and Refresh is offered, not "Refreshing" for ever.
#[gpui_kit::test]
async fn a_refresh_removed_on_quit_ends_refreshing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Going", "");
    });
    shell.fixture.commit_removal(&root, &ticket_path(PRIMARY));
    shell.fixture.refresh(&root);
    open(&shell, ticket(&root, PRIMARY), cx);
    assert!(present(&shell, "item-refresh", cx));

    // The worker is busy with another job, so Refresh's waits.
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    let (services, busy) = shell.workspace.read_with(cx, |workspace, _| {
        let services = workspace.services().clone();
        let busy = services.worker().submit(
            Intent::IndexRefresh { root: root.clone() },
            Lane::Background,
            None,
            None,
        );
        (services, busy)
    });
    held.wait_reached();
    shell.click("item-refresh", cx);
    assert!(present(&shell, "item-refreshing", cx));

    let _ = services.worker().quit();
    shell.run(cx);
    assert!(!present(&shell, "item-refreshing", cx));
    assert!(present(&shell, "item-refresh", cx));

    // The worker must not outlive the test: let the held job end.
    held.release();
    shell
        .until(cx, move |operations| {
            operations
                .request(busy)
                .is_some_and(|request| matches!(request.state, RequestState::Finished(..)))
        })
        .await;
}

/// Ruling R-B5: an item shown, then deleted and the index refreshed, is
/// read again in place: the reload shows `item_not_found` with Refresh,
/// as a first load does, not the item it showed.
#[gpui_kit::test]
fn a_reload_after_the_item_is_deleted_shows_item_not_found_with_refresh(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "Going", "");
    });
    open(&shell, ticket(&root, PRIMARY), cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Going"));

    shell.fixture.commit_removal(&root, &ticket_path(PRIMARY));
    shell.fixture.refresh(&root);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .unwrap();
    pane.update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    assert_eq!(
        shell.label("item-failure", cx).as_deref(),
        Some(ResultCode::ItemNotFound.message())
    );
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::ItemNotFound.as_str())
    );
    assert!(present(&shell, "item-refresh", cx));
    assert!(!present(&shell, "item-title", cx));
    assert!(!present(&shell, "item-not-refreshed", cx));
    assert_eq!(
        shell.label(("tab", 0usize), cx).as_deref(),
        Some("Going"),
        "the tab keeps the last title read"
    );
}
