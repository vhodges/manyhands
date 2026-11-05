//! Resume on a lifecycle row (D5 decision 10, Part C; R-22, K-20; A15, P5,
//! P22). Real: a close another process began stops after recording its
//! `closing` step (`ClosureAfterClosingRecorded`, as `tests/ticket_closure/
//! exits.rs` stops one), its request left accepted, before the desktop
//! starts. Its row lists `resume`; the resume names the request that holds
//! it (`original_request_required`, as `tests/mutation_remote/c5_resume.rs`
//! `resume_held_by_its_request` answers), and the desktop sends that close
//! again. One answer is canned, and says so.

use std::path::{Path, PathBuf};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, FailurePoint, Mutation, MutationCall, MutationDataDto,
        NoProgress, OperationAction, PrepareDataDto, RepositoryService, RequestId,
        TicketCloseInput, keys::SessionCredentials,
    },
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use crate::desktop::{
    model::lifecycle_lock::{LockSource, LockState},
    services::{
        BUSY_RETRY_DELAYS, JobKind,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        harness::{Shell, key},
        lifecycle::{LifecycleDialog, OpenedLifecycle, STILL_RUNNING},
        operations::RESUME,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KQ0";

/// Prepares and executes a close of the ticket through `service`, under
/// `request_id`: another process's close.
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

/// A ticket of its own working copy in repository `a`, whose close another
/// process stranded at `closing` before the window opened: the shell, the
/// root and the stranded request.
pub(super) fn stranded(cx: &mut TestAppContext) -> (Shell, PathBuf, RequestId) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Stranded");
    fixture.refresh(&root);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::ClosureAfterClosingRecorded,
    )
    .unwrap();
    let request = RequestId::new();
    let stopped = close_through(&failing, &root, request);
    assert_ne!(stopped.code, ResultCode::Ok, "{}", stopped.message);
    drop(failing);
    (Shell::open(fixture, cx), root, request)
}

/// The close's pending row, once listed.
async fn close_row(shell: &Shell, cx: &mut TestAppContext) -> usize {
    let close = OperationAction::Close.as_str().to_owned();
    let wanted = close.clone();
    shell
        .until_drawn(cx, "the close's row", move |shell, cx| {
            shell.rows("pending-command", cx).contains(&wanted)
        })
        .await;
    shell
        .rows("pending-command", cx)
        .iter()
        .position(|listed| *listed == close)
        .expect("listed")
}

/// Where the ticket of `root` stands under the lifecycle lock.
fn lock(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> LockState {
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let root = key(root);
    cx.read(|cx| services.lifecycle_lock(&root, TICKET, cx))
}

async fn until_lock(shell: &Shell, root: &Path, expected: LockState, cx: &mut TestAppContext) {
    let root = root.to_owned();
    shell
        .until_drawn(cx, &format!("the lock {expected:?}"), move |shell, cx| {
            lock(shell, &root, cx) == expected
        })
        .await;
}

fn resumes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { .. })
    })
}

fn closes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketClose { .. })
    })
}

/// The lifecycle dialog last opened, while it is open.
fn dialog(cx: &mut TestAppContext) -> Option<Entity<LifecycleDialog>> {
    cx.update(|cx| {
        cx.try_global::<OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
    })
}

/// The run the open lifecycle dialog shows.
fn shown(cx: &mut TestAppContext) -> Option<DesktopRequestId> {
    dialog(cx).and_then(|dialog| dialog.read_with(cx, |dialog, _| dialog.submitted()))
}

fn code(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Option<ResultCode> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
        })
}

/// A15, K-20, real: after a restart, a close stranded past `closing` locks
/// the ticket, and its row offers Resume, the one way on from here.
#[gpui_kit::test]
async fn a_lifecycle_row_offers_resume_after_restart(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let row = close_row(&shell, cx).await;
    assert_eq!(
        shell.label(("pending-next", row), cx).as_deref(),
        Some("resume")
    );
    assert_eq!(
        shell.label(("pending-resume", row), cx).as_deref(),
        Some(RESUME)
    );
    assert!(!shell.present(("pending-resume-unavailable", row), cx));
    assert!(!shell.present(("pending-still-running", row), cx));
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;
}

/// P5, decision 10, K-20, real: Resume sends `operation resume` of the row,
/// and opens the lifecycle dialog on it. The resume names the stranded
/// request; the desktop sends the close again under that request ID, with
/// no confirmation, and the dialog follows it, Running, then Done. The
/// ticket is no longer locked.
#[gpui_kit::test]
async fn resume_resends_the_named_request_and_opens_running(cx: &mut TestAppContext) {
    let (shell, root, holder) = stranded(cx);
    let row = close_row(&shell, cx).await;
    let held = shell.backend.hold_job(JobKind::TicketClose);
    shell.click(("pending-resume", row), cx);
    let resume = resumes(&shell, cx);
    assert_eq!(resume.len(), 1, "one resume of the row");
    assert!(dialog(cx).is_some(), "the dialog opened on it");
    shell.until_ended(resume[0], cx).await;
    assert_eq!(
        code(&shell, resume[0], cx),
        Some(ResultCode::OriginalRequestRequired)
    );

    held.wait_reached();
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 1, "the close sent again");
    let close = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(sent[0]).cloned())
        .unwrap();
    assert_eq!(close.request_id, holder, "the named request");
    assert!(close.confirmation.is_none(), "no confirmation");
    shell
        .until_drawn(cx, "the dialog follows the close", |shell, cx| {
            shown(cx) == Some(sent[0]) && shell.present("lifecycle-stop", cx)
        })
        .await;
    assert_eq!(
        lock(&shell, &root, cx),
        LockState::Locked(LockSource::Worker)
    );

    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    shell
        .until_drawn(cx, "the close is done", |shell, cx| {
            shell.present("lifecycle-ending", cx)
        })
        .await;
    assert_eq!(shell.label("lifecycle-ending", cx).as_deref(), Some("Done"));
    until_lock(&shell, &root, LockState::Open, cx).await;
    shell
        .until_drawn(cx, "the row is gone", |shell, cx| {
            !shell
                .rows("pending-command", cx)
                .contains(&"close".to_owned())
        })
        .await;
}

/// P22, real then canned: while the ticket's close is in flight, its row
/// offers no Resume and says it is still running. A resume answered `busy`
/// reads "Still running…" in the dialog, with no failure.
///
/// Canned: the `busy` of `tests/mutation_remote/c5_resume.rs`
/// `resume_running_elsewhere` (another call holds the lease; `operation.
/// resume` offered, no data). The resume's real call still runs, and is
/// answered from the stranded record; nothing is asserted of the repository
/// after it.
#[gpui_kit::test]
async fn resume_is_not_offered_while_in_flight_and_busy_reads_still_running(
    cx: &mut TestAppContext,
) {
    let (shell, root, _) = stranded(cx);
    let row = close_row(&shell, cx).await;
    let held = shell.backend.hold_job(JobKind::TicketClose);
    shell.click(("pending-resume", row), cx);
    // The drain sends the close once the resume's answer lands.
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::TicketClose { .. }))
        })
        .await;
    held.wait_reached();
    shell
        .until_drawn(cx, "the row waits", move |shell, cx| {
            shell.present(("pending-still-running", row), cx)
        })
        .await;
    assert_eq!(
        shell.label(("pending-still-running", row), cx).as_deref(),
        Some(STILL_RUNNING)
    );
    assert!(!shell.present(("pending-resume", row), cx));
    shell.keys("escape", cx);
    held.release();
    let sent = closes(&shell, cx);
    shell.until_ended(sent[0], cx).await;
    until_lock(&shell, &root, LockState::Open, cx).await;

    // A second stranded close, whose resume is answered `busy`.
    let (shell, _, _) = stranded(cx);
    let row = close_row(&shell, cx).await;
    let busy = Envelope::<MutationDataDto>::failure(
        "operation resume",
        Scope::default(),
        ResultCode::Busy,
        vec![RecoveryAction::new(RecoveryActionKind::OperationResume, [])],
    );
    // The worker takes a `busy` job again before its answer stands.
    for _ in 0..=BUSY_RETRY_DELAYS.len() {
        shell
            .backend
            .hold_job_return_answering(JobKind::OperationResume, busy.clone())
            .release();
    }
    shell.click(("pending-resume", row), cx);
    let resume = resumes(&shell, cx)[0];
    shell.until_ended(resume, cx).await;
    shell
        .until_drawn(cx, "the dialog says it", |shell, cx| {
            shell.present("lifecycle-still-running", cx)
        })
        .await;
    assert_eq!(
        shell.label("lifecycle-still-running", cx).as_deref(),
        Some(STILL_RUNNING)
    );
    assert!(!shell.present("lifecycle-ending", cx), "no failure");
    assert!(!shell.present("lifecycle-code", cx));
    assert!(!shell.present("lifecycle-stop", cx));
    assert!(closes(&shell, cx).is_empty(), "nothing sent again");
}

/// R-21, P22, real: a resume of the row, while it runs, is the ticket's
/// lifecycle in flight: the ticket is locked by the worker, the row offers
/// no second Resume, and the dialog on it shows it running.
#[gpui_kit::test]
async fn a_running_resume_counts_as_in_flight(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let row = close_row(&shell, cx).await;
    let held = shell.backend.hold_job(JobKind::OperationResume);
    shell.click(("pending-resume", row), cx);
    held.wait_reached();
    let resume = resumes(&shell, cx);
    assert_eq!(resume.len(), 1);
    until_lock(&shell, &root, LockState::Locked(LockSource::Worker), cx).await;
    assert_eq!(shown(cx), Some(resume[0]));
    assert!(shell.present("lifecycle-stop", cx), "it shows running");
    shell.keys("escape", cx);
    shell
        .until_drawn(cx, "the row waits", move |shell, cx| {
            shell.present(("pending-still-running", row), cx)
        })
        .await;
    assert!(
        !shell.present(("pending-resume", row), cx),
        "no second resume"
    );

    held.release();
    shell.until_ended(resume[0], cx).await;
    shell
        .until(cx, |operations| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::TicketClose { .. })
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    until_lock(&shell, &root, LockState::Open, cx).await;
}
