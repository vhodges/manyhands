//! Nothing lifecycle without consent (A17; Cycle contract 1), over real
//! local-only repositories: navigation, Save, Done, closing a tab or the
//! window, and quit never start a close, an approve and merge, a resume or
//! a republish. Each test asserts that no `TicketClose`,
//! `DocumentPromote`, `OperationResume` or `ItemRepublish` request was
//! ever made, and, on the repositories, that no lifecycle commit appears:
//! main is unchanged, the items' context branches gain only what a Save
//! commits, the ticket is not closed and the library lists no new
//! lifecycle row.

use std::path::Path;

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    repository::{ClosureState, ItemDtoKind, OperationAction, RequestState as LibraryState},
    results::ResultCode,
};

use super::{
    lock_tests::step,
    tests::{
        DOCUMENT, TICKET, closes, code, commits, edit, open, saved_ticket, shown, strand_close,
        ticket, until_present,
    },
};
use crate::desktop::{
    model::{navigation::OpenTarget, operations::Operations},
    services::requests::Intent,
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, is_save, key},
        item::LIFECYCLE_ACTION,
        workspace::QuitPhase,
    },
};

/// Whether `intent` starts or continues a lifecycle, or republishes.
fn is_lifecycle(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::TicketClose { .. }
            | Intent::DocumentPromote { .. }
            | Intent::OperationResume { .. }
            | Intent::ItemRepublish { .. }
    )
}

/// The lifecycle requests `operations` ever held, by label.
fn lifecycles(operations: &Entity<Operations>, cx: &mut TestAppContext) -> Vec<&'static str> {
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| is_lifecycle(&request.intent))
            .map(|request| request.intent.label())
            .collect()
    })
}

/// The lifecycle rows the library lists for `root`: closes and promotions.
fn lifecycle_rows(fixture: &Fixture, root: &Path) -> usize {
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .filter(|row| {
            matches!(
                row.action,
                OperationAction::Close | OperationAction::Promote
            )
        })
        .count()
}

fn document(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    }
}

/// Commits on main and on the ticket's and the document's context
/// branches.
fn counts(shell: &Shell, root: &Path) -> (usize, usize, usize) {
    let branch = |id: &str| shown(shell, root, id).context.branch.expect("its own");
    (
        commits(root, "main"),
        commits(root, &branch(TICKET)),
        commits(root, &branch(DOCUMENT)),
    )
}

/// One repository with a ticket and a document, each with a working copy
/// of its own, in an open window.
fn two_items(cx: &mut TestAppContext) -> (Shell, std::path::PathBuf) {
    cx.executor().allow_parking();
    super::tests::shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Consent", "");
        fixture.create_document(root, DOCUMENT, "docs/plan.md", "Plan");
    })
}

/// A17, real: opening the ticket and the document, selecting the
/// repository, moving focus about, Edit, a Save, Done, closing both tabs,
/// quit and closing the window send no lifecycle request; main and the
/// document's branch are unchanged, the ticket's branch gains the Save's
/// one commit, the ticket stays open and no lifecycle row is listed.
#[gpui_kit::test]
async fn a17_navigation_save_done_close_and_quit_start_no_lifecycle(cx: &mut TestAppContext) {
    let (shell, root) = two_items(cx);
    let (main, ticket_branch, document_branch) = counts(&shell, &root);
    let operations = shell.operations(cx);

    // Navigation.
    open(&shell, ticket(&root), cx);
    open(&shell, document(&root), cx);
    shell.select(&root, cx);
    open(&shell, ticket(&root), cx);
    shell.keys("tab tab tab escape", cx);
    assert!(shell.present(LIFECYCLE_ACTION, cx), "the action is offered");

    // Edit, Save, then Done.
    let host = edit(&shell, cx).await;
    shell.type_at_end(&host, " more", cx);
    shell.click("item-save", cx);
    let save = shell.requests(cx, |request| is_save(&request.intent))[0];
    shell.until_ended(save, cx).await;
    assert_eq!(code(&shell, save, cx), Some(ResultCode::Ok));
    shell.click("item-done", cx);
    assert!(lifecycles(&operations, cx).is_empty());

    // Closing both tabs.
    shell.keys("secondary-w", cx);
    open(&shell, document(&root), cx);
    shell.keys("secondary-w", cx);
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len());
    assert_eq!(tabs, 0, "both tabs closed");

    // Quit, then the window's close.
    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Quit);
    assert!(shell.visual(cx).simulate_close(), "the window may close");
    shell.run(cx);

    assert!(lifecycles(&operations, cx).is_empty());
    assert_eq!(
        counts(&shell, &root),
        (main, ticket_branch + 1, document_branch),
        "only the Save's commit"
    );
    let closure = shown(&shell, &root, TICKET)
        .closure
        .expect("a ticket's closure");
    assert_ne!(closure.state, ClosureState::Closed, "the ticket stays open");
    assert_eq!(lifecycle_rows(&shell.fixture, &root), 0);
}

/// A17, real: the lifecycle dialog at its confirm stage runs nothing until
/// Confirm. Escape cancels it; opened again and left at Confirm, the
/// window's close request and quit send no lifecycle request, and nothing
/// is committed or listed.
#[gpui_kit::test]
async fn a17_a_dialog_left_at_confirm_starts_nothing(cx: &mut TestAppContext) {
    let (shell, root) = two_items(cx);
    let before = counts(&shell, &root);
    let operations = shell.operations(cx);

    open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert!(closes(&shell, cx).is_empty());

    open(&shell, document(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.visual(cx).simulate_close();
    shell.run(cx);
    shell.keys("secondary-q", cx);
    shell.run(cx);

    assert!(lifecycles(&operations, cx).is_empty());
    assert_eq!(counts(&shell, &root), before, "nothing committed");
    assert_eq!(lifecycle_rows(&shell.fixture, &root), 0);
}

/// A17, real: a close another process stranded at `closing` (its request
/// accepted, its row offering Resume) is not resumed by a launch that
/// refreshes, by opening the ticket and moving about, by closing its tab or
/// by quitting: no request is made, its row stays at `closing`, its
/// request stays accepted, and main is unchanged.
#[gpui_kit::test]
async fn a17_a_stranded_close_is_not_resumed_without_consent(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Stranded", "");
    fixture.refresh(&root);
    let holder = strand_close(&fixture, &root);
    let main = commits(&root, "main");
    let shell = Shell::launch(fixture, |_| {}, cx);
    let operations = shell.operations(cx);
    assert_eq!(step(&shell, &root).as_deref(), Some("closing"));

    open(&shell, ticket(&root), cx);
    shell
        .until_drawn(cx, "the close's row", |shell, cx| {
            shell
                .rows("pending-command", cx)
                .contains(&OperationAction::Close.as_str().to_owned())
        })
        .await;
    shell.select(&root, cx);
    shell.keys("tab tab tab escape", cx);
    shell.keys("secondary-w", cx);
    shell.keys("secondary-q", cx);
    assert!(shell.visual(cx).simulate_close(), "the window may close");
    shell.run(cx);

    assert!(lifecycles(&operations, cx).is_empty());
    assert_eq!(step(&shell, &root).as_deref(), Some("closing"));
    let recorded = shell
        .fixture
        .service()
        .show_request(holder)
        .data
        .expect("the request is recorded")
        .state;
    assert_eq!(recorded, LibraryState::Accepted);
    assert_eq!(commits(&root, "main"), main, "nothing merged");
}
