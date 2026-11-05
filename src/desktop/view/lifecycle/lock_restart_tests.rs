//! The lifecycle lock from what the library recorded (D5 design, A2; P20,
//! P25, P26; rulings R-21, R-22): a closure at `closing` after a restart
//! (L-2), a not-tidied close after a restart (L-4), each answer before
//! acceptance (L-6), the CLI finishing or starting a close while the
//! desktop is open, and an unreadable repository. A close "of another
//! process" is the library's, through a service of its own over the same
//! data directory, as `lifecycle_tests.rs` `strand_close` makes one.

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::drafts::Held;
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, DocumentPromoteInput, FailurePoint, ItemDtoKind,
        LifecycleCleanupPendingReason, Mutation, MutationCall, MutationDataDto, NoProgress,
        PrepareDataDto, RepositoryService, RequestId, TicketCloseInput, keys::SessionCredentials,
    },
    results::{Envelope, RecoveryAction, ResultCode, Scope},
};

use super::{
    confirm_tests::worktree,
    lock_tests::{
        CLOSING, LOCKED_NOTE, MERGING, NOT_TIDIED, assert_edits_live, assert_edits_wait, lock,
        step, until_lock,
    },
    outcome_canned_tests::{finished, intent},
    tests::{
        DOCUMENT, TICKET, closes, code, open, saved_ticket, shell, shown, strand_close, ticket,
        until_present,
    },
};
use crate::desktop::{
    model::{
        lifecycle_lock::{LockSource, LockState, Untidied},
        navigation::OpenTarget,
    },
    services::requests::Intent,
    test_support::{
        drafts::{held, store_draft},
        fixtures::{Fixture, NoPassphrase, item_id},
    },
    view::{
        drafts::row_id,
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
    },
};

/// The window loses focus, then takes it again.
fn focus_again(shell: &Shell, cx: &mut TestAppContext) {
    let mut visual = shell.visual(cx);
    visual.deactivate_window();
    visual.update(|window, _| window.activate_window());
    visual.run_until_parked();
}

/// Sends the close `request_id` of the ticket again through `service`,
/// with no confirmation: the CLI's same-request resend.
fn resend(
    service: &RepositoryService,
    root: &Path,
    request_id: RequestId,
) -> Envelope<MutationDataDto> {
    service.execute(
        MutationCall {
            request_id,
            confirmation: None,
            mutation: Mutation::TicketClose(TicketCloseInput {
                root: root.to_owned(),
                id: item_id(TICKET),
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// One repository with a ticket of its own working copy, a close of which
/// another process stranded first.
fn stranded(cx: &mut TestAppContext) -> (Shell, std::path::PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Stranded", "");
    fixture.refresh(&root);
    strand_close(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    (shell, root)
}

/// L-2, decision 20, real: a close that recorded its closure checkpoint
/// (`ClosureAfterClosingRecorded`, as `tests/ticket_closure/exits.rs`
/// stops one) before the desktop started is read at launch: the ticket is
/// read-only, saying "Closing…".
#[gpui_kit::test]
async fn a_closure_at_closing_stays_locked_after_restart(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = stranded(cx);
    assert_eq!(step(&shell, &root).as_deref(), Some("closing"));
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;
    assert_edits_wait(&shell, CLOSING, cx);
    focus_again(&shell, cx);
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;
    assert_edits_wait(&shell, CLOSING, cx);
}

/// L-4 after a restart (R-22), real: a merge stopped after it reached main
/// (`AfterPublication`) with a file left in its working copy is read at
/// launch as locked, "Merging…": its row carries no reason. Run again from
/// Approve and merge…, the answer says the working copy changed, and the
/// document unlocks with "Merged, but not tidied up".
///
/// A ticket's close at this point offers no Close ticket…: the ticket reads
/// closed. Its way on after a restart is Resume on its row (Task 14).
#[gpui_kit::test]
async fn a_changed_context_after_restart_unlocks_once_answered(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, "docs/plan.md", "Plan");
    fixture.refresh(&root);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::AfterPublication,
    )
    .unwrap();
    let stopped = promote_through(&failing, &root);
    assert_ne!(stopped.code, ResultCode::Ok, "{}", stopped.message);
    drop(failing);
    let shell = Shell::open(fixture, cx);
    let context = shown(&shell, &root, DOCUMENT).context.worktree;
    std::fs::write(Path::new(&context).join("untracked.txt"), b"the owner's\n").unwrap();
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    let locked = LockState::Locked(LockSource::Record);
    until_lock_of(&shell, &root, DOCUMENT, locked, cx).await;
    assert_edits_wait(&shell, MERGING, cx);

    shell.click(LIFECYCLE_ACTION, cx);
    shell
        .until_drawn(cx, "a way on", |shell, cx| {
            shell.present("lifecycle-continue", cx) || shell.present("lifecycle-confirm", cx)
        })
        .await;
    if shell.present("lifecycle-continue", cx) {
        shell.click("lifecycle-continue", cx);
    } else {
        shell.click("lifecycle-confirm", cx);
    }
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentPromote { .. })
    })[0];
    shell.until_ended(id, cx).await;
    let untidied = LockState::Untidied(Untidied {
        reason: LifecycleCleanupPendingReason::ContextChanged,
        request: id,
    });
    until_lock_of(&shell, &root, DOCUMENT, untidied, cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present(LOCKED_NOTE, cx));
    assert_eq!(
        shell.label("item-not-tidied", cx).as_deref(),
        Some(NOT_TIDIED)
    );
}

/// Prepares and executes a promotion of the document through `service`.
fn promote_through(service: &RepositoryService, root: &Path) -> Envelope<MutationDataDto> {
    let mutation = || {
        Mutation::DocumentPromote(DocumentPromoteInput {
            root: root.to_owned(),
            id: item_id(DOCUMENT),
        })
    };
    let prepared = service.prepare(&mutation());
    let preview = prepared
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .expect("the merge is ready");
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: Some(confirmation),
            mutation: mutation(),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// Where the item `id` of `root` stands under the lock.
fn lock_of(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) -> LockState {
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let root = key(root);
    cx.read(|cx| services.lifecycle_lock(&root, id, cx))
}

async fn until_lock_of(
    shell: &Shell,
    root: &Path,
    id: &'static str,
    expected: LockState,
    cx: &mut TestAppContext,
) {
    let root = root.to_owned();
    shell
        .until_drawn(cx, &format!("the lock {expected:?}"), move |shell, cx| {
            lock_of(shell, &root, id, cx) == expected
        })
        .await;
}

/// A canned answer to the close before it was accepted, `code`, with
/// `recovery`.
fn refused(
    root: &Path,
    code: ResultCode,
    recovery: Vec<RecoveryAction>,
) -> Envelope<MutationDataDto> {
    Envelope::<MutationDataDto>::failure(
        "ticket close",
        Scope {
            repository: Some(key(root)),
            item_id: Some(TICKET.to_owned()),
            ..Scope::default()
        },
        code,
        recovery,
    )
}

/// L-6, P25: every answer before acceptance leaves no lock once its dialog
/// closes. Real: a dirty main blocks the prepare, a path added after the
/// preview refuses the consent (`external_change`), and Cancel. Canned, as
/// answers no local run gives here, with nothing asserted of the
/// repository: `busy` (`tests/mutation_remote/c5_lifecycle.rs`
/// `c5_lifecycle_busy_with_no_parent_close`) and `request_mismatch`
/// (`c5_lifecycle_parent_unreadable_on_mismatch_close`).
#[gpui_kit::test]
async fn each_pre_acceptance_answer_unlocks(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Refusals", "")
    });
    open(&shell, ticket(&root), cx);
    let dialog = LockState::Locked(LockSource::Dialog);

    let dirty = root.join("dirty-primary.txt");
    std::fs::write(&dirty, b"dirty\n").unwrap();
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-prepare-again", cx).await;
    assert_eq!(lock(&shell, &root, cx), dialog, "blocked");
    shell.keys("escape", cx);
    until_lock(&shell, &root, LockState::Open, cx).await;
    std::fs::remove_file(&dirty).unwrap();

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(lock(&shell, &root, cx), dialog, "confirm");
    shell.keys("escape", cx);
    until_lock(&shell, &root, LockState::Open, cx).await;

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell
        .fixture
        .commit_file(&worktree(&shell, &root), "src/late.rs", b"// late\n");
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::ExternalChange));
    until_present(&shell, "lifecycle-stale", cx).await;
    assert_eq!(lock(&shell, &root, cx), dialog, "stale");
    shell.keys("escape", cx);
    until_lock(&shell, &root, LockState::Open, cx).await;
    assert_eq!(step(&shell, &root), None);

    for (n, code) in [ResultCode::Busy, ResultCode::RequestMismatch]
        .into_iter()
        .enumerate()
    {
        let answer = refused(&root, code, Vec::new());
        let request = RequestId::new();
        finished(
            &shell,
            n as u64 + 1,
            intent(&root, ItemDtoKind::Ticket),
            request,
            answer,
            cx,
        );
        until_lock(&shell, &root, LockState::Open, cx).await;
    }
    assert_edits_live(&shell, cx);
}

/// P26, real: the CLI strands a close of the ticket while the desktop is
/// open; once the window takes focus again, the ticket is read-only. The
/// CLI then finishes it under the same request; the next focus unlocks it.
#[gpui_kit::test]
async fn a_cli_finish_unlocks_on_refresh_or_focus(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Outside", "")
    });
    open(&shell, ticket(&root), cx);
    assert_eq!(lock(&shell, &root, cx), LockState::Open);

    let request = strand_close(&shell.fixture, &root);
    focus_again(&shell, cx);
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;
    assert_edits_wait(&shell, CLOSING, cx);

    let service = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    let done = resend(&service, &root, request);
    assert_eq!(done.code, ResultCode::Ok, "{}", done.message);
    focus_again(&shell, cx);
    until_lock(&shell, &root, LockState::Open, cx).await;
    assert!(!shell.present(LOCKED_NOTE, cx));
}

/// R-21: rows that cannot be read give no record lock. A close stranded at
/// `closing` locks the ticket; once its repository's folder is gone, the
/// next read fails, and nothing locks it.
#[gpui_kit::test]
async fn an_unreadable_repository_gives_no_record_lock(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = stranded(cx);
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;

    std::fs::rename(&root, root.with_file_name("gone")).unwrap();
    focus_again(&shell, cx);
    until_lock(&shell, &root, LockState::Open, cx).await;
}

/// R-23, real: while a close stranded at `closing` locks the ticket, the
/// Drafts page does not restore a draft of it into its tab: the row says
/// "Closing…", and the draft stays stored and unclaimed.
#[gpui_kit::test]
async fn the_drafts_page_does_not_restore_a_locked_items_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Drafted", "");
    fixture.refresh(&root);
    let draft = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Mine");
    strand_close(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;
    shell.keys("secondary-shift-d", cx);

    shell.click(row_id(draft, "restore"), cx);
    assert_eq!(
        shell.label(row_id(draft, "notice"), cx).as_deref(),
        Some(CLOSING)
    );
    assert!(matches!(
        held(&shell.fixture.draft_store(), draft),
        Some(Held::No)
    ));
}
