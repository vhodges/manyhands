//! The lifecycle dialog up to its confirm stage, over real local-only
//! repositories (D5 design, A2): the prepare, a blocked or held item, the
//! first run with its confirmation, an already-closed ticket and the
//! dialog's header. One test renders a remote blocker through a canned
//! prepare (decision 9) and says so. The save made first is
//! `lifecycle/save_first_tests.rs`, the header's action
//! `lifecycle/action_tests.rs`; both share this file's helpers.
//!
//! Each test that waits on the worker allows parking. Editing tests set the
//! caret, focus the editor and draw a frame before typing, and type through
//! the window (D3's rules), through `Shell::type_at_end`.

use std::path::{Path, PathBuf};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, FailurePoint, ItemDto, ItemDtoKind, LifecycleBlockedDto,
        LifecycleBlocker, Mutation, MutationCall, MutationDataDto, NoProgress, PrepareDataDto,
        RepositoryService, RequestId, TicketCloseInput, keys::SessionCredentials,
    },
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use super::{
    CONTINUE, HELD_LEAD, LifecycleDialog, OPEN_FOLDER, OpenedLifecycle, SHARE_MAIN, STILL_RUNNING,
    prepare::{PRIMARY_DIRTY, blocker_words},
    short_id,
};
use crate::desktop::{
    model::{
        item::{APPROVE_AND_MERGE, CLOSE_TICKET},
        navigation::OpenTarget,
    },
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, RequestState},
    },
    test_support::{
        fixtures::{Fixture, NoPassphrase, item_id},
        snapshot::RepositorySnapshot,
    },
    view::{
        harness::{Shell, key},
        item::{ItemPane, LIFECYCLE_ACTION},
    },
};

pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KC0";
pub(super) const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KD0";
pub(super) const NEW_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KE0";
pub(super) const SLUG: &str = "ae-c10s3";

/// One repository, `a`, built by `build` and refreshed, in an open window.
pub(super) fn shell(
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// A ticket committed on main and saved once, retitled `title`, so it has
/// a working copy of its own to close.
pub(super) fn saved_ticket(fixture: &Fixture, root: &Path, title: &str, extra: &str) {
    fixture.commit_ticket(root, TICKET, "Untitled", extra);
    fixture.refresh(root);
    fixture.save_ticket(root, TICKET, title);
}

pub(super) fn ticket(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    }
}

pub(super) fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

pub(super) fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

pub(super) fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

/// Commits reachable from `branch` of `root`.
pub(super) fn commits(root: &Path, branch: &str) -> usize {
    let repository = git2::Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip.id()).unwrap();
    walk.count()
}

pub(super) fn is_close(request: &DesktopRequest) -> bool {
    matches!(request.intent, Intent::TicketClose { .. })
}

pub(super) fn closes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, is_close)
}

/// The request `id` as the model holds it now.
pub(super) fn request(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> DesktopRequest {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).cloned())
        .expect("the request is listed")
}

pub(super) fn code(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<ResultCode> {
    match request(shell, id, cx).state {
        RequestState::Finished(_, code, ..) => Some(code),
        _ => None,
    }
}

/// The lifecycle dialog last opened.
pub(super) fn dialog(cx: &mut TestAppContext) -> Entity<LifecycleDialog> {
    cx.update(|cx| cx.global::<OpenedLifecycle>().0.upgrade())
        .expect("a lifecycle dialog is open")
}

/// Clicks Edit and waits for the edit editor.
pub(super) async fn edit(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Entity<crate::desktop::view::editor::host::EditorHost> {
    shell.click("item-edit", cx);
    shell.until_editing(cx).await
}

pub(super) async fn until_present(shell: &Shell, id: &'static str, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, id, |shell, cx| shell.present(id, cx))
        .await;
}

/// P19, real: a dirty main blocks the close at prepare. The dialog says so
/// and offers Open folder; Confirm is not offered, and nothing is merged.
#[gpui_kit::test]
async fn a_dirty_primary_blocks_at_prepare(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Dirty", ""));
    open(&shell, ticket(&root), cx);
    std::fs::write(root.join("dirty-primary.txt"), b"dirty\n").unwrap();
    let main = commits(&root, "main");

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-prepare-again", cx).await;
    assert_eq!(shell.rows("lifecycle-blocker", cx), [PRIMARY_DIRTY]);
    assert_eq!(
        shell.label("lifecycle-open-folder", cx).as_deref(),
        Some(OPEN_FOLDER)
    );
    assert!(!shell.present("lifecycle-confirm", cx));
    assert!(closes(&shell, cx).is_empty());
    assert_eq!(commits(&root, "main"), main, "nothing is merged");
}

/// Canned (decision 9): a repository with a remote, whose main is ahead of
/// it, cannot be reached here. The real answer is the library's
/// (`closure/observe_tests.rs` `primary_ahead_of_remote`;
/// `tests/cli_remote/lifecycle.rs` `c5_lifecycle_walk_primary_ahead_close`).
/// The blocker is shown in words, then L1's way out: the poll as guidance,
/// Share main as a button, and the repository's details, since a blocked
/// answer offers `repo.inspect` (as `tests/cli/lifecycle.rs`
/// `a_dirty_primary_blocks_naming_the_blocker` prints it), in words and as
/// Open repository details (Task 7). Nothing is asserted of the
/// repository.
#[gpui_kit::test]
async fn a_blocked_prepare_lists_blockers_and_way_out(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Ahead", ""));
    let blockers = vec![LifecycleBlocker::PrimaryAheadOfRemote];
    let mut blocked = Envelope::<PrepareDataDto>::failure(
        "ticket close",
        Scope {
            repository: Some(key(&root)),
            item_id: Some(TICKET.to_owned()),
            ..Scope::default()
        },
        ResultCode::RecoveryRequired,
        vec![RecoveryAction::new(
            RecoveryActionKind::RepoInspect,
            [("root", key(&root).into())],
        )],
    );
    blocked.data = Some(PrepareDataDto::LifecycleBlocked(LifecycleBlockedDto {
        blockers: blockers.clone(),
    }));
    shell.backend.answer_next_prepare("ticket close", blocked);
    open(&shell, ticket(&root), cx);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-prepare-again", cx).await;
    let words: Vec<&str> = blockers
        .iter()
        .map(|blocker| blocker_words(*blocker))
        .collect();
    assert_eq!(shell.rows("lifecycle-blocker", cx), words);
    let guidance = shell.rows("lifecycle-way-out", cx);
    assert_eq!(guidance.len(), 2, "the poll and the details: {guidance:?}");
    assert!(guidance[0].contains("Check origin"), "{}", guidance[0]);
    assert!(guidance[1].contains("details"), "{}", guidance[1]);
    assert_eq!(
        shell.label("lifecycle-share-main", cx).as_deref(),
        Some(SHARE_MAIN)
    );
    assert_eq!(
        shell.label("lifecycle-inspect", cx).as_deref(),
        Some(super::outcome::OPEN_REPOSITORY_DETAILS)
    );
    assert!(!shell.present("lifecycle-open-folder", cx));
    assert!(!shell.present("lifecycle-confirm", cx));
    assert!(closes(&shell, cx).is_empty());
}

/// Prepares and executes a close of the ticket through `service`, under
/// `request_id`: another actor's close.
fn close_through(
    service: &RepositoryService,
    root: &Path,
    request_id: RequestId,
) -> Envelope<MutationDataDto> {
    let mutation = || {
        Mutation::TicketClose(TicketCloseInput {
            root: root.to_owned(),
            id: item_id(TICKET),
        })
    };
    let prepared = service.prepare(&mutation());
    let preview = prepared
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .expect("the close is ready");
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    service.execute(
        MutationCall {
            request_id,
            confirmation: Some(confirmation),
            mutation: mutation(),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// An earlier close through a service of its own that stops at
/// `ClosureAfterClosingRecorded`: its `closing` step and checkpoint are
/// recorded, its context untouched, and its request left accepted (the
/// recipe of `closure/run_tests.rs`). Gives its request ID.
fn strand_close(fixture: &Fixture, root: &Path) -> RequestId {
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::ClosureAfterClosingRecorded,
    )
    .unwrap();
    let request_id = RequestId::new();
    let stopped = close_through(&failing, root, request_id);
    assert_ne!(stopped.code, ResultCode::Ok, "{}", stopped.message);
    request_id
}

/// P12, real: an earlier close that stopped after recording its `closing`
/// step holds the ticket. The prepare says so and offers Continue, which
/// sends that request again under its own request ID with no
/// confirmation, and the close finishes.
#[gpui_kit::test]
async fn a_held_item_offers_continue(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Held", "");
    fixture.refresh(&root);
    let holder = strand_close(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-continue", cx).await;
    assert_eq!(
        shell.label("lifecycle-lead", cx).as_deref(),
        Some(HELD_LEAD)
    );
    assert_eq!(
        shell.label("lifecycle-continue", cx).as_deref(),
        Some(CONTINUE)
    );
    assert!(!shell.present("lifecycle-confirm", cx));
    assert!(closes(&shell, cx).is_empty());

    shell.click("lifecycle-continue", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 1, "Continue sent one close");
    let continued = request(&shell, sent[0], cx);
    assert_eq!(continued.request_id, holder, "the holding request");
    assert!(continued.confirmation.is_none(), "no confirmation");
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
}

/// Review M-3 (P22): while the held close is running again, Continue is
/// not offered; the dialog says it is still running.
#[gpui_kit::test]
async fn continue_is_not_offered_while_the_close_runs(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Held", "");
    fixture.refresh(&root);
    strand_close(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    let held = shell.backend.hold_job(JobKind::TicketClose);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-continue", cx).await;
    shell.click("lifecycle-continue", cx);
    held.wait_reached();
    shell.keys("escape", cx);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-still-running", cx).await;
    assert_eq!(
        shell.label("lifecycle-still-running", cx).as_deref(),
        Some(STILL_RUNNING)
    );
    assert!(!shell.present("lifecycle-continue", cx));
    held.release();
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 1, "one close");
    shell.until_ended(sent[0], cx).await;
}

/// The first run is sent once, with the confirmation of the preview the
/// dialog held, and the close finishes.
#[gpui_kit::test]
async fn the_first_run_submits_with_its_confirmation(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "First", ""));
    open(&shell, ticket(&root), cx);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(
        shell.label("lifecycle-confirm", cx).as_deref(),
        Some("Close ticket")
    );
    let held = dialog(cx)
        .read_with(cx, |dialog, _| dialog.confirmation())
        .expect("the preview's confirmation is held");
    assert!(closes(&shell, cx).is_empty(), "nothing runs before Confirm");

    shell.click("lifecycle-confirm", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 1, "one close");
    assert_eq!(
        dialog(cx).read_with(cx, |dialog, _| dialog.submitted()),
        Some(sent[0])
    );
    let first = request(&shell, sent[0], cx);
    assert_eq!(first.confirmation, Some(held), "with its confirmation");
    assert!(!shell.present("lifecycle-confirm", cx), "sent once");
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(closes(&shell, cx), sent);
    let ran = shell
        .backend
        .jobs_seen()
        .into_iter()
        .filter(|kind| *kind == JobKind::TicketClose)
        .count();
    assert_eq!(ran, 1);
}

/// P14: a ticket another actor has closed since the tab read it is
/// answered `ticket_closed` at prepare, blocked as already closed. The
/// dialog says so, offers neither Confirm nor Prepare again, and nothing
/// runs or changes.
#[gpui_kit::test]
async fn already_closed_runs_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Closed", "")
    });
    open(&shell, ticket(&root), cx);
    assert_eq!(
        shell.label(LIFECYCLE_ACTION, cx).as_deref(),
        Some(CLOSE_TICKET)
    );
    let closed = close_through(shell.fixture.service(), &root, RequestId::new());
    assert_eq!(closed.code, ResultCode::Ok, "{}", closed.message);
    let before = RepositorySnapshot::take(&root);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-code", cx).await;
    assert_eq!(
        shell.label("lifecycle-code", cx).as_deref(),
        Some(ResultCode::TicketClosed.as_str())
    );
    assert_eq!(
        shell.rows("lifecycle-blocker", cx),
        [blocker_words(LifecycleBlocker::AlreadyClosed)]
    );
    assert!(!shell.present("lifecycle-confirm", cx));
    assert!(!shell.present("lifecycle-prepare-again", cx));
    assert!(closes(&shell, cx).is_empty());
    assert_eq!(RepositorySnapshot::take(&root), before, "nothing changed");
}

/// A22: the header names the command, the item by its short code (its
/// short ID when it has none) and title, the repository, and the item's
/// context branch.
#[gpui_kit::test]
async fn the_header_names_item_repository_and_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Coded", &format!("slug: {SLUG}\n"));
        fixture.create_document(root, DOCUMENT, "docs/plan.md", "Plan");
    });
    open(&shell, ticket(&root), cx);
    let branch = shown(&shell, &root, TICKET)
        .context
        .branch
        .expect("the ticket's own branch");
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(shell.label("lifecycle-kind", cx).as_deref(), Some("ticket"));
    assert_eq!(
        shell.label("lifecycle-title", cx).as_deref(),
        Some("Close ticket")
    );
    assert_eq!(
        shell.label("lifecycle-subtitle", cx),
        Some(format!("{SLUG} \u{b7} Coded \u{b7} a"))
    );
    assert_eq!(shell.label("lifecycle-branch", cx), Some(branch));
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert!(closes(&shell, cx).is_empty(), "Escape runs nothing");

    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    assert_eq!(
        shell.label(LIFECYCLE_ACTION, cx).as_deref(),
        Some(APPROVE_AND_MERGE)
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-title", cx).await;
    assert_eq!(
        shell.label("lifecycle-kind", cx).as_deref(),
        Some("document")
    );
    assert_eq!(
        shell.label("lifecycle-title", cx).as_deref(),
        Some("Approve and merge")
    );
    assert_eq!(
        shell.label("lifecycle-subtitle", cx),
        Some(format!("{} \u{b7} Plan \u{b7} a", short_id(DOCUMENT)))
    );
}
