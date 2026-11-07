//! After a lifecycle (D5 design, A4; decision 14; A10, A20), over real
//! local-only repositories: the item's tab reopens it from main once the
//! tidy-up is complete and the desktop forgets the working copy it made;
//! the closure shows apart from the status; comment drafts stay; a close
//! that failed keeps the working copy and every draft; and an edit of a
//! promoted document starts a fresh working copy.

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    drafts::CommentTarget,
    drafts::{DraftFields, DraftId, DraftTarget, ItemKind},
    repository::{ClosureState, ItemContextKind, ItemDtoKind, ProgressStage},
    results::ResultCode,
};

use super::{
    confirm_tests::worktree,
    outcome::ENDING_DONE,
    tests::{
        DOCUMENT, TICKET, closes, code, edit, open, pane, saved_ticket, shell, shown, ticket,
        until_present,
    },
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{JobKind, created_contexts::ContextKey, requests::Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
    },
};

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KF0";

/// Whether the desktop counts the item's working copy as made here and
/// unused (R-75).
fn made_here(
    shell: &Shell,
    root: &Path,
    kind: ItemDtoKind,
    id: &str,
    cx: &mut TestAppContext,
) -> bool {
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let context = ContextKey::new(&key(root), kind, id);
    services
        .created_contexts()
        .read_with(cx, |created, _| created.contains(&context))
}

/// Whether the shown tab holds an edit session.
fn editing(shell: &Shell, cx: &mut TestAppContext) -> bool {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_some())
}

/// How the shown tab last read its item's copy.
fn shown_copy(shell: &Shell, cx: &mut TestAppContext) -> Option<ItemContextKind> {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.shown_item().map(|item| item.context.kind)
    })
}

/// A comment draft of the ticket, stored and let go, as one an earlier
/// session left.
fn store_comment(fixture: &Fixture, root: &Path) -> DraftId {
    let target = DraftTarget {
        repository: key(root),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TICKET),
        comment: Some(CommentTarget {
            comment_id: item_id(COMMENT),
            parent_id: None,
        }),
    };
    let fields = DraftFields::Comment {
        body: "Not sent yet".to_owned(),
    };
    match fixture.draft_store().create(target, None, fields) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

fn stored(fixture: &Fixture, id: DraftId) -> bool {
    fixture
        .draft_store()
        .list()
        .unwrap()
        .iter()
        .any(|listing| listing.id == id)
}

/// Confirms the open dialog's run of the item and waits until it is done.
async fn run_to_done(shell: &Shell, cx: &mut TestAppContext) {
    until_present(shell, "lifecycle-confirm", cx).await;
    shell.click("lifecycle-confirm", cx);
    let id = shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
        )
    })[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(shell, id, cx), Some(ResultCode::Ok));
    until_present(shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    shell.keys("escape", cx);
    shell.run(cx);
}

/// A4, R-23, real: Edit made the ticket's working copy, and the tab still
/// edits it, unchanged, when Close ticket… runs. Once the close is tidied
/// up, the tab no longer edits, reads the ticket from main, and the
/// desktop no longer counts the working copy as its own.
#[gpui_kit::test]
async fn tabs_reopen_from_primary_after_cleanup(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Reopened", "");
    });
    open(&shell, ticket(&root), cx);
    edit(&shell, cx).await;
    assert!(made_here(&shell, &root, ItemDtoKind::Ticket, TICKET, cx));
    assert!(editing(&shell, cx));

    shell.click(LIFECYCLE_ACTION, cx);
    run_to_done(&shell, cx).await;
    shell
        .until_drawn(cx, "the tab reading main", |shell, cx| {
            shown_copy(shell, cx) == Some(ItemContextKind::Primary)
        })
        .await;
    assert!(!editing(&shell, cx), "the session ended");
    assert!(!shell.present("item-done", cx));
    assert!(!made_here(&shell, &root, ItemDtoKind::Ticket, TICKET, cx));
}

/// A20, P13, characterization: once a ticket whose status says `open` is
/// closed, its details say Closed, with when and by whom, apart from its
/// status, which stays as written.
#[gpui_kit::test]
async fn the_closure_display_stays_apart_from_status(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Shown", ""));
    let status = shown(&shell, &root, TICKET).status;
    open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    run_to_done(&shell, cx).await;
    let item = shown(&shell, &root, TICKET);
    let closure = item.closure.clone().expect("a ticket's closure");
    assert_eq!(closure.state, ClosureState::Closed);
    assert_eq!(item.status, status, "the status as written");
    shell
        .until_drawn(cx, "the closed ticket", |shell, cx| {
            !shell.present(LIFECYCLE_ACTION, cx)
        })
        .await;

    shell.click("chip-all", cx);
    assert_eq!(shell.label("item-lifecycle", cx).as_deref(), Some("Closed"));
    assert_eq!(shell.label("item-closed-at", cx), closure.closed_at);
    assert_eq!(shell.label("item-closed-by", cx), closure.closed_by);
    assert_eq!(shell.label("item-status", cx), status);
}

/// A10, characterization: a comment draft of the ticket, not sent, stays
/// stored, and on the Drafts page, once the ticket is closed.
#[gpui_kit::test]
async fn comment_drafts_stay_on_the_drafts_page(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Commented", "");
    fixture.refresh(&root);
    let draft = store_comment(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    run_to_done(&shell, cx).await;

    assert!(stored(&shell.fixture, draft), "the draft is kept");
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let listed = services.drafts().read_with(cx, |drafts, _| {
        drafts
            .listing()
            .rows()
            .is_some_and(|rows| rows.iter().any(|listing| listing.id == draft))
    });
    assert!(listed, "the Drafts page lists it");
}

/// A10, A4, real: a close whose tidy-up stops (a file left in the working
/// copy, `context_changed`) keeps the working copy the desktop made and
/// every draft of the ticket.
#[gpui_kit::test]
async fn a_failed_close_keeps_its_context_and_drafts(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Kept", "");
    fixture.refresh(&root);
    let draft = store_comment(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    edit(&shell, cx).await;
    let context = worktree(&shell, &root);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::CleaningUp);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    std::fs::write(context.join("untracked.txt"), b"the owner's\n").unwrap();
    gate.release();
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::RecoveryRequired));
    shell.keys("escape", cx);
    shell.run(cx);

    assert!(context.exists(), "the working copy is kept");
    assert!(made_here(&shell, &root, ItemDtoKind::Ticket, TICKET, cx));
    assert!(stored(&shell.fixture, draft), "the comment draft is kept");
}

/// A4, decision 14, real: a document edited, saved first and approved is
/// read from main afterwards; Edit then makes a fresh working copy.
#[gpui_kit::test]
async fn editing_a_promoted_document_starts_a_fresh_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, "docs/plan.md", "Plan");
    });
    let document = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    };
    open(&shell, document, cx);
    let host = edit(&shell, cx).await;
    shell.type_at_end(&host, "More.", cx);
    shell.click(LIFECYCLE_ACTION, cx);
    run_to_done(&shell, cx).await;
    shell
        .until_drawn(cx, "the tab reading main", |shell, cx| {
            shown_copy(shell, cx) == Some(ItemContextKind::Primary)
        })
        .await;
    assert!(!editing(&shell, cx), "the session ended");

    edit(&shell, cx).await;
    let prepares = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ContextPrepare { .. })
    });
    assert_eq!(prepares.len(), 2, "a second prepare: {prepares:?}");
    assert_eq!(code(&shell, prepares[0], cx), Some(ResultCode::Ok));
    assert!(made_here(
        &shell,
        &root,
        ItemDtoKind::Document,
        DOCUMENT,
        cx
    ));
    let fresh = shown(&shell, &root, DOCUMENT);
    assert_ne!(
        fresh.context.kind,
        ItemContextKind::Primary,
        "a fresh working copy"
    );
}
