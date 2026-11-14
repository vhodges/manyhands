//! D5's privacy sweep (A16) over the surfaces only a remote reaches: a
//! shared conflict reviewed and resolved on the sheet, and a close stopped
//! on its deleted shared branch, stopped and republished from the
//! lifecycle dialog. `privacy_tests.rs` holds the sentinels and the sweep
//! (`Seen`) and drives the real local surfaces.
//!
//! A shared conflict, a deleted shared branch, a republish's preview and
//! their answers come only from a remote, which the SSH harness does not
//! bring to desktop tests: each is canned (decision 9), built from library
//! types by the helpers of the conflict and republish tests, which name the
//! library test that produces the real one. The item is real, with its
//! title, body and comment; a canned answer carries none of them, so these
//! tests show the desktop adds none to what it shows. Nothing is asserted of
//! the repository.

use std::path::PathBuf;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        AuthoringKind, ConflictPathDto, ConflictPathKind, ConflictPathStructure, FailurePoint,
        ItemDtoKind, OperationId, RepositoryService, RequestId,
    },
    results::{Outcome, ResultCode},
};

use super::{
    privacy_tests::{
        BASE, BODY, MENU_ITEM, POPOVER, ROW_MENU, Seen, THEIRS, TITLE, commit_sentinel_comment,
        shows_body_and_comment, ticket_with_body,
    },
    tests::{closes, open, shell, until_present},
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{JobKind, requests::Intent},
    test_support::fixtures::{item_id, ticket_path},
    view::{
        activity_support::to_row,
        conflict::{answer_tests::applied, entry_tests, tests as conflict},
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
        operations::republish::tests as republish,
        privacy_layers_tests::{is_open, open_popover},
    },
};

/// The synchronization `entry_tests::share_conflict` names.
const SYNCHRONIZATION: &str = "01M4CC0VN1KTQFSVR99378TDHV";

/// The ticket `id` committed on main as "Untitled", with the sentinel
/// comment, then saved with `TITLE` and `BODY`: a working copy of its own.
/// Its tab is open and shows them.
fn ticket_shell(cx: &mut TestAppContext, id: &str) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, id, "Untitled", "");
        commit_sentinel_comment(fixture, root, id);
        fixture.refresh(root);
        fixture.save_ticket_with_body(root, id, TITLE, &format!("{BODY} is the body.\n"));
        fixture.refresh(root);
    });
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    };
    open(&shell, target, cx);
    shows_body_and_comment(&shell, cx);
    (shell, root)
}

/// The ticket's file in conflict, of the shape `conflict::canned_path`
/// gives (`tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_canonical`): the base titled `BASE`, mine `TITLE` with
/// `BODY`, and theirs `THEIRS`.
fn sentinel_path() -> ConflictPathDto {
    let id = conflict::TICKET;
    ConflictPathDto {
        display_path: ticket_path(id),
        token: format!("conflict-path-v1:{}", "1".repeat(64)),
        kind: ConflictPathKind::Ticket,
        structure: ConflictPathStructure::Regular,
        content_available: true,
        base: Some(ticket_with_body(id, BASE, "Body.\n")),
        local: Some(ticket_with_body(id, TITLE, &format!("{BODY} is mine.\n"))),
        incoming: Some(ticket_with_body(id, THEIRS, "Body.\n")),
    }
}

/// The Needs attention popover and the menu of its row, open and swept
/// (Task 12, E-14). The item's own row needs attention, so both name it by
/// its title, which is allowed there, and by nothing else of it: the body and
/// the comment, planted in the same item, are on screen in its tab.
fn layers(shell: &Shell, seen: &mut Seen, cx: &mut TestAppContext) {
    open_popover(shell, cx);
    seen.window(shell, cx);
    assert!(
        seen.allowed_holding(POPOVER, TITLE),
        "the popover's row names the item by its title"
    );
    let key = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_rows())
        .first()
        .map(|row| row.row.key.clone())
        .expect("a row that needs attention");
    shell.keys("escape", cx);
    assert!(!is_open(shell, cx), "Escape closes the popover");
    to_row(shell, &key, cx);
    shell.keys("shift-f10", cx);
    shell.frame(cx, |_, _| ());
    seen.window(shell, cx);
    for at in [ROW_MENU, MENU_ITEM] {
        assert!(
            seen.allowed_holding(at, TITLE),
            "the row's menu names the item by its title ({at})"
        );
    }
    shell.keys("escape", cx);
}

/// A16, canned (decision 9): a share of the ticket stopped on a conflict
/// (`entry_tests::share_conflict`) raises the conflict banner; Review
/// changes opens the sheet on a remote conflict (`conflict::canned`) whose
/// three sides hold sentinels, Before showing `BASE`, Yours `TITLE` and
/// Theirs `THEIRS`; mine is kept and saved, and once applied
/// (`answer_tests::applied`) the banner goes. No sentinel reaches an entry,
/// an answer, the operation area, the banner or the sheet's own words.
#[gpui_kit::test]
async fn a16_a_shared_conflict_and_its_resolution_show_no_item_text(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx, conflict::TICKET);
    let mut seen = Seen::default();
    seen.window(&shell, cx);

    let share = Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item_id(conflict::TICKET),
    };
    entry_tests::fabricate(&shell, 1, share, entry_tests::share_conflict(), cx);
    until_present(&shell, "item-review-changes", cx).await;
    seen.window(&shell, cx);
    layers(&shell, &mut seen, cx);
    let operation = OperationId::parse(SYNCHRONIZATION).unwrap();
    shell
        .backend
        .answer_next_conflict_read(conflict::canned(operation, vec![sentinel_path()]));
    shell.click("item-review-changes", cx);
    until_present(&shell, "conflict-save", cx).await;
    seen.window(&shell, cx);
    assert!(seen.allowed_holding("conflict-title", TITLE));
    for sentinel in [TITLE, THEIRS] {
        assert!(
            seen.allowed_holding("conflict-line", sentinel),
            "{sentinel}"
        );
    }
    assert!(
        seen.allowed_holding("conflict-before", BASE),
        "Before's lines"
    );
    shell.click(("conflict-keep-mine", 0usize), cx);
    seen.window(&shell, cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);
    let resolve = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ConflictResolve { .. })
    });
    shell.until_ended(resolve[0], cx).await;
    shell
        .until_drawn(cx, "the banner's clearing", |shell, cx| {
            !shell.present("item-conflict", cx)
        })
        .await;
    seen.window(&shell, cx);

    seen.holds_none(&[
        "operation entry",
        "operation area",
        "banner",
        "sheet message",
        "popover row",
        "popover tooltip",
        "popover description",
        "popover",
        "popover copy",
        "row menu",
        "menu item",
        "drawn",
    ]);
    assert!(seen.read_at("banner", "safe"));
}

/// A16, canned (decision 9), as `operations/republish_stop_tests.rs`
/// `stop_and_republish_cancels_republishes_then_resends` drives it, with
/// its helpers: a close of the ticket, through a service that stops it at
/// `ClosureAfterClosingRecorded` so its record stays accepted, is answered
/// as stopped on its deleted shared branch (`republish::branch_deleted`);
/// Stop and republish… gives it up (answered `cancelled`), Republish… is
/// confirmed (`republish::republish_preview`) and answered `ok`
/// (`republish::republished`), and the close is sent again. No sentinel
/// reaches an entry, an answer, the operation area, the dialog's messages
/// and outcome, or the republish's confirmation.
#[gpui_kit::test]
async fn a16_stop_and_republish_shows_no_item_text(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx, republish::TICKET);
    let mut seen = Seen::default();
    let preview = republish::republish_preview(&shell, &root);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    seen.window(&shell, cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::ClosureAfterClosingRecorded,
        )
        .unwrap(),
    );
    shell
        .backend
        .hold_job_return_answering(
            JobKind::TicketClose,
            republish::branch_deleted(&root, RequestId::new()),
        )
        .release();
    shell.click("lifecycle-confirm", cx);
    let close = closes(&shell, cx)[0];
    shell.until_ended(close, cx).await;
    until_present(&shell, "lifecycle-stop-and-republish", cx).await;
    seen.window(&shell, cx);
    let asked = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(close).cloned())
        .expect("the close is listed")
        .request_id;

    shell
        .backend
        .hold_job_return_answering(
            JobKind::TicketClose,
            republish::close_answer(
                &root,
                ResultCode::Cancelled,
                Outcome::Cancelled,
                Vec::new(),
                asked,
            ),
        )
        .release();
    shell.backend.answer_next_prepare("item republish", preview);
    shell.click("lifecycle-stop-and-republish", cx);
    until_present(&shell, "confirm-ok", cx).await;
    seen.window(&shell, cx);

    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republish::republished())
        .release();
    shell.click("confirm-ok", cx);
    let republished = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ItemRepublish { .. })
    });
    assert_eq!(republished.len(), 1, "republished once");
    shell.until_ended(republished[0], cx).await;
    shell
        .until(cx, move |operations| {
            operations
                .request(close)
                .is_some_and(|request| request.attempt == 3)
        })
        .await;
    shell.until_ended(close, cx).await;
    shell.run(cx);
    seen.window(&shell, cx);

    seen.holds_none(&[
        "operation entry",
        "operation area",
        "dialog message",
        "confirmation message",
        "drawn",
    ]);
    assert!(seen.read_at("operation entry", "Republish"));
    assert!(seen.allowed_holding("lifecycle-subtitle", TITLE));
}
