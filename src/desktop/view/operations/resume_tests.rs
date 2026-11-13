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
        CancellationToken, ConfirmationId, FailurePoint, ItemDtoKind, LifecycleBlocker,
        LifecycleMutationDto, LifecycleMutationType, Mutation, MutationCall, MutationDataDto,
        NoProgress, OperationAction, OperationDto, PrepareDataDto, RepositoryService, RequestId,
        TicketCloseInput, keys::SessionCredentials,
    },
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use crate::desktop::{
    model::{
        activity::rows::{ActivityAction, RowKey},
        lifecycle_lock::{LockSource, LockState},
    },
    services::{
        BUSY_RETRY_DELAYS, JobKind,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        activity_support::{act, offered, pending_key, refresh, row},
        harness::{Shell, key},
        lifecycle::{LifecycleDialog, OpenedLifecycle, STILL_RUNNING},
        operations::RESUME,
    },
};

pub(in crate::desktop::view::operations) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KQ0";

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
pub(in crate::desktop::view::operations) fn stranded(
    cx: &mut TestAppContext,
) -> (Shell, PathBuf, RequestId) {
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

/// The close's pending operation in `root`'s list, when listed.
fn listed_close(shell: &Shell, root: &str, cx: &mut TestAppContext) -> Option<OperationDto> {
    shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .pending_shown()
            .into_iter()
            .filter(|(listed, _)| listed == root)
            .filter_map(|(_, state)| state.rows().map(|list| list.items.clone()))
            .flatten()
            .find(|operation| operation.action == OperationAction::Close)
    })
}

/// The close's pending row, once listed: its key, for the seam.
pub(in crate::desktop::view::operations) async fn close_row(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> RowKey {
    let root = key(root);
    shell
        .until_drawn(cx, "the close's row", |shell, cx| {
            listed_close(shell, &root, cx).is_some()
        })
        .await;
    pending_key(shell, &root, OperationAction::Close.as_str(), cx)
}

/// Whether the log offers `label` on the pending row `entry`: the offer's
/// action and whether it can be pressed. It panics unless the row is there.
fn offer(
    shell: &Shell,
    entry: &RowKey,
    label: &str,
    cx: &mut TestAppContext,
) -> Option<(ActivityAction, bool)> {
    refresh(shell, cx);
    row(shell, entry, cx).expect("the pending row stays");
    offered(shell, entry, label, cx).map(|offer| (offer.action, offer.enabled))
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

/// Draws frames until `done` holds, for up to thirty seconds: for answers whose
/// retries outlast the harness's own wait under the whole suite.
pub(in crate::desktop::view::operations) async fn until_long(
    shell: &Shell,
    what: &str,
    cx: &mut TestAppContext,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if done(shell, cx) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{what} did not happen"
        );
        cx.executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
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
    let entry = close_row(&shell, &root, cx).await;
    let operation = listed_close(&shell, &key(&root), cx).expect("listed");
    assert_eq!(
        operation.next_action.map(|next| next.as_str()),
        Some("resume")
    );
    let (action, enabled) = offer(&shell, &entry, RESUME, cx).expect("Resume");
    assert!(enabled, "Resume can be pressed");
    assert!(matches!(action, ActivityAction::ResumeLifecycle { .. }));
    assert!(
        offer(&shell, &entry, STILL_RUNNING, cx).is_none(),
        "not still running"
    );
    assert!(
        row(&shell, &entry, cx)
            .expect("its row")
            .row
            .menu
            .iter()
            .all(|offer| offer.enabled),
        "no Resume that waits"
    );
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
    let entry = close_row(&shell, &root, cx).await;
    let held = shell.backend.hold_job(JobKind::TicketClose);
    act(&shell, &entry, RESUME, cx);
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
            refresh(shell, cx);
            row(shell, &entry, cx).is_none()
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
    let entry = close_row(&shell, &root, cx).await;
    let held = shell.backend.hold_job(JobKind::TicketClose);
    act(&shell, &entry, RESUME, cx);
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
        .until_drawn(cx, "the row waits", |shell, cx| {
            refresh(shell, cx);
            row(shell, &entry, cx).is_some() && offered(shell, &entry, STILL_RUNNING, cx).is_some()
        })
        .await;
    let (_, enabled) = offer(&shell, &entry, STILL_RUNNING, cx).expect("it says why");
    assert!(!enabled, "no Resume while it runs");
    assert!(offer(&shell, &entry, RESUME, cx).is_none());
    shell.keys("escape", cx);
    held.release();
    let sent = closes(&shell, cx);
    shell.until_ended(sent[0], cx).await;
    until_lock(&shell, &root, LockState::Open, cx).await;

    // A second stranded close, whose resume is answered `busy`.
    let (shell, root, _) = stranded(cx);
    let entry = close_row(&shell, &root, cx).await;
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
    act(&shell, &entry, RESUME, cx);
    let resume = resumes(&shell, cx)[0];
    until_long(&shell, "the resume's busy answer", cx, |shell, cx| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations
                .request(resume)
                .is_some_and(|request| matches!(request.state, RequestState::Finished(..)))
        })
    })
    .await;
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
    let entry = close_row(&shell, &root, cx).await;
    let held = shell.backend.hold_job(JobKind::OperationResume);
    act(&shell, &entry, RESUME, cx);
    held.wait_reached();
    let resume = resumes(&shell, cx);
    assert_eq!(resume.len(), 1);
    until_lock(&shell, &root, LockState::Locked(LockSource::Worker), cx).await;
    assert_eq!(shown(cx), Some(resume[0]));
    assert!(shell.present("lifecycle-stop", cx), "it shows running");
    shell.keys("escape", cx);
    shell
        .until_drawn(cx, "the row waits", |shell, cx| {
            refresh(shell, cx);
            row(shell, &entry, cx).is_some() && offered(shell, &entry, STILL_RUNNING, cx).is_some()
        })
        .await;
    assert!(
        offer(&shell, &entry, RESUME, cx).is_none(),
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

/// P22, real then canned: Resume's resend of the stranded close is answered
/// `busy` each time the worker takes it (as `tests/mutation_remote/
/// c5_lifecycle.rs` `busy_with_a_standing_parent` answers while another
/// call holds the parent): the dialog reads "Still running…", with no
/// failure.
#[gpui_kit::test]
async fn a_resent_close_answered_busy_reads_still_running(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let entry = close_row(&shell, &root, cx).await;
    let busy = Envelope::<MutationDataDto>::failure(
        "ticket close",
        Scope::default(),
        ResultCode::Busy,
        Vec::new(),
    );
    for _ in 0..=BUSY_RETRY_DELAYS.len() {
        shell
            .backend
            .hold_job_return_answering(JobKind::TicketClose, busy.clone())
            .release();
    }
    act(&shell, &entry, RESUME, cx);
    // Each busy answer still runs the real call, then waits out its delay:
    // longer than the harness's wait under the whole suite.
    until_long(&shell, "the close's busy answer", cx, |shell, cx| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::TicketClose { .. })
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
    })
    .await;
    let close = closes(&shell, cx)[0];
    shell
        .until_drawn(cx, "the dialog follows the close", |shell, cx| {
            shown(cx) == Some(close) && shell.present("lifecycle-ending", cx)
        })
        .await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(STILL_RUNNING)
    );
}

/// Decision 10, canned: a resume answered `original_request_required` for
/// an operation the lifecycle rows do not name sends nothing again; the
/// dialog shows the answer's message and code, with no way on of its own.
/// The answer is `tests/mutation_remote/c5_resume.rs`
/// `resume_held_by_its_request`'s, naming a request this session does not
/// hold.
#[gpui_kit::test]
async fn a_resume_naming_an_unknown_lifecycle_sends_nothing(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let entry = close_row(&shell, &root, cx).await;
    let named = RequestId::new();
    let refused = Envelope::<MutationDataDto>::failure(
        "operation resume",
        Scope::default(),
        ResultCode::OriginalRequestRequired,
        vec![RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", named.to_string().into())],
        )],
    );
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    // The lock's rows are forgotten: they name no lifecycle for the resume.
    shell
        .backend
        .hold_job_return_answering(JobKind::OperationResume, refused)
        .release();
    shell.operations(cx).update(cx, |operations, _| {
        operations.locks_mut().forget(&key(&root));
    });
    act(&shell, &entry, RESUME, cx);
    let resume = resumes(&shell, cx)[0];
    shell.until_ended(resume, cx).await;
    shell
        .until_drawn(cx, "the answer", |shell, cx| {
            shell.present("lifecycle-code", cx)
        })
        .await;
    assert_eq!(
        shell.label("lifecycle-code", cx).as_deref(),
        Some("original_request_required")
    );
    assert!(closes(&shell, cx).is_empty(), "nothing sent again");
    assert!(services.resends().follow(resume).is_none());
    for button in [
        "lifecycle-retry",
        "lifecycle-resume",
        "lifecycle-republish",
        "lifecycle-stop-and-republish",
        "lifecycle-prepare-again",
    ] {
        assert!(!shell.present(button, cx), "{button}");
    }
}

/// V17, canned: a resume's answer has no operation ID of its own, and its
/// child's conflict is named by the parent the resume action names. The
/// dialog on the resume offers Resolve conflict… and Resume, not Inspect
/// alone. The answer is `tests/mutation_remote/c5_child_conflict.rs`'s under
/// a resume (`operation resume`, `recovery_required`, `conflict_pending` and
/// `context_conflicted`, `operation.resume` naming the parent).
#[gpui_kit::test]
async fn a_resumed_child_conflict_offers_resolve_and_resume(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let entry = close_row(&shell, &root, cx).await;
    let mut conflicted = Envelope::<MutationDataDto>::failure(
        "operation resume",
        Scope::default(),
        ResultCode::RecoveryRequired,
        vec![RecoveryAction::for_operation(
            RecoveryActionKind::OperationResume,
            "01ARZ3NDEKTSV4RRFFQ69G5FY1",
            [],
        )],
    );
    conflicted.operation_id = None;
    conflicted.data = Some(MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind: ItemDtoKind::Ticket,
        item_id: TICKET.to_owned(),
        recording: None,
        checkpoint_oid: None,
        integration: None,
        integration_oid: None,
        publication: None,
        publication_oid: None,
        publication_pending: None,
        cleanup: None,
        cleanup_pending: None,
        blockers: vec![
            LifecycleBlocker::ContextConflicted,
            LifecycleBlocker::ConflictPending,
        ],
        made_ready: None,
        made_ready_state: None,
    })));
    shell
        .backend
        .hold_job_return_answering(JobKind::OperationResume, conflicted)
        .release();
    act(&shell, &entry, RESUME, cx);
    shell
        .until_drawn(
            cx,
            "the dialog shows the conflict's way out",
            |shell, cx| shell.present("lifecycle-ending", cx),
        )
        .await;
    assert!(shell.present("lifecycle-resolve-conflict", cx));
    assert!(shell.present("lifecycle-resume", cx));
}
