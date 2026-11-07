//! Give up and Resume in the operation area (Task 10b; design, Part 0
//! Give up, Lane B Resume; decisions 8 and 11; R-1, R-15, P14, U-5).
//!
//! A share of main over SSH in the test binary is accepted, reserves the
//! repository and then fails its transport, which is never initialized
//! (fact 17): its record stays accepted and its remote operation is listed
//! pending, holding the reservation (fact 14). Rows whose controls depend
//! on the record run such a share, held once its library call returned and
//! answered with a constructed envelope; a share whose publication
//! selection is left uncommitted is refused before acceptance, so its
//! record stands finished.

use std::{path::Path, sync::mpsc};

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationDto, OperationFamily, OperationId, RequestState as RecordDtoState,
    },
    results::{Effects, Envelope, PublicationEffect, ResultCode, Scope},
};

use super::{
    ALREADY_FINISHED, offer_part,
    tests::{TICKET, answer, assert_row, open, real_share, shared, submit_main},
};
use crate::desktop::{
    model::{
        operations::{
            CHOOSE_A_KEY, KEY_LOCKED, NEEDS_IDENTITY, STOPPED_AFTER_PART, STOPPED_BEFORE_SENDING,
            check_host_text, main_branch_deleted,
        },
        publication::SyncOffer,
    },
    services::{
        JobKind,
        requests::{CommentBody, DesktopRequestId, Intent, Lane, RequestState},
        worker::{GaveUp, RecordState, WorkerHandle},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, WAIT},
        operations::{CANCELLED, STOP, STOPPING, request_id},
    },
};

fn worker(shell: &Shell, cx: &mut TestAppContext) -> WorkerHandle {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone())
}

/// What the library lists pending in `root`, as it reads now.
fn pending(shell: &Shell, root: &Path) -> Vec<OperationDto> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.list_operations(&repo).unwrap().items
}

/// The remote operation `root` lists pending in `phase`: its ID.
fn remote_row(shell: &Shell, root: &Path, phase: &str) -> String {
    let rows: Vec<String> = pending(shell, root)
        .into_iter()
        .filter(|row| row.family == OperationFamily::Remote && row.state == phase)
        .filter_map(|row| row.operation_id)
        .collect();
    assert_eq!(rows.len(), 1, "one remote operation is {phase}");
    rows[0].clone()
}

/// The pending row of `operation` in `root`.
fn row(shell: &Shell, root: &Path, operation: &str) -> OperationDto {
    pending(shell, root)
        .into_iter()
        .find(|row| row.operation_id.as_deref() == Some(operation))
        .expect("the operation is listed")
}

/// The remote operation that holds `root`'s reservation now, if any.
fn reservation(shell: &Shell, root: &Path) -> Option<String> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.polling_status(&repo).unwrap().active_operation_id
}

/// Where the library record of the request `id` stands now.
fn record(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Option<RecordDtoState> {
    let request_id = shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(id).unwrap().request_id
    });
    shell
        .fixture
        .service()
        .show_request(request_id)
        .data
        .map(|request| request.state)
}

/// The finished request `id`'s code.
fn code(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Option<String> {
    shell.label(request_id(id, "code"), cx)
}

/// A share of main that reserved the repository and failed its
/// transport: its record is accepted and its remote operation holds the
/// reservation.
async fn accepted_share(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> DesktopRequestId {
    let id = submit_main(shell, root, cx);
    shell.until_ended(id, cx).await;
    assert_eq!(code(shell, id, cx).as_deref(), Some("internal_error"));
    assert_eq!(
        worker(shell, cx).record_state(id),
        Some(RecordState::Accepted)
    );
    id
}

/// Holds the next request after its acceptance, before its domain call
/// (fact 16): the receiver hears it is held, and dropping the sender lets
/// it go on.
fn hold_after_acceptance(shell: &Shell) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (reached_tx, reached) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    shell
        .fixture
        .service()
        .set_request_hook_for_testing(move || {
            let _ = reached_tx.send(());
            let _ = released.recv_timeout(WAIT);
        });
    (reached, release)
}

/// Waits until the request model has heard the request `id` accepted.
async fn until_accepted(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Running { accepted: true, .. })
            )
        })
        .await;
}

fn resume_of(root: &Path, operation: &str) -> Intent {
    Intent::OperationResume {
        root: root.to_owned(),
        operation_id: OperationId::parse(operation).unwrap(),
    }
}

/// Resumes `operation` of `root` as Resume does, a user job, held after
/// its acceptance until the sender is dropped. The pending list offers no
/// Resume for a remote operation that holds the reservation, so it is
/// sent as Resume would send it.
async fn held_resume(
    shell: &Shell,
    root: &Path,
    operation: &str,
    cx: &mut TestAppContext,
) -> (DesktopRequestId, mpsc::Sender<()>) {
    let (reached, release) = hold_after_acceptance(shell);
    let id = worker(shell, cx).submit(resume_of(root, operation), Lane::User, None, None);
    reached.recv_timeout(WAIT).expect("the resume was accepted");
    until_accepted(shell, id, cx).await;
    (id, release)
}

/// Presses Stop trying to share on the finished share `id` and waits for
/// the Give up's answer: held once its call returned, so the wait is for
/// that attempt.
async fn give_up(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    let returned = shell.backend.hold_job_return(JobKind::SynchronizeRepo);
    shell.click(request_id(id, offer_part(SyncOffer::GiveUp)), cx);
    returned.wait_reached();
    returned.release();
    shell.until_ended(id, cx).await;
}

/// The fixed words of the rows the Give up tests answer with; a failure
/// shows the library's message instead.
fn words_of(code: ResultCode) -> Option<String> {
    match code {
        // These shares are of main (E-6; review M3).
        ResultCode::RemoteBranchDeleted => Some(main_branch_deleted("main")),
        ResultCode::UnlockRequired => Some(KEY_LOCKED.to_owned()),
        ResultCode::NoSelectedKey => Some(CHOOSE_A_KEY.to_owned()),
        ResultCode::HostApprovalRequired => Some(check_host_text(None)),
        ResultCode::IdentityRequired => Some(NEEDS_IDENTITY.to_owned()),
        _ => None,
    }
}

/// Decision 8: Give up is offered exactly while the share's record is
/// accepted: after `unlock_required`, which Not now answers (R-13), it is
/// offered beside Unlock…; the same answer to a share refused before its
/// acceptance, whose record is finished, offers Share and no Give up; and
/// one whose record the worker never read offers none either.
#[gpui_kit::test]
async fn give_up_is_offered_exactly_when_the_record_is_accepted(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let locked = || answer(ResultCode::UnlockRequired, Effects::not_requested());

    let (shell, root) = open(true, cx);
    let accepted = real_share(&shell, &root, locked(), cx).await;
    assert_eq!(
        worker(&shell, cx).record_state(accepted),
        Some(RecordState::Accepted)
    );
    assert_row(
        &shell,
        accepted,
        Some(KEY_LOCKED),
        &[SyncOffer::GiveUp, SyncOffer::Unlock],
        cx,
    );

    let (shell, root) = open(false, cx);
    let finished = real_share(&shell, &root, locked(), cx).await;
    assert_eq!(
        worker(&shell, cx).record_state(finished),
        Some(RecordState::Finished)
    );
    assert_row(&shell, finished, Some(KEY_LOCKED), &[SyncOffer::Share], cx);

    let unknown = shared(&shell, &root, 1, locked(), cx);
    assert_row(&shell, unknown, Some(KEY_LOCKED), &[SyncOffer::Unlock], cx);
}

/// Decision 8: every row a share can stop on with its record accepted
/// offers Give up beside its own step, and the same rows with the record
/// finished never do.
#[gpui_kit::test]
async fn give_up_is_offered_on_exactly_the_rows_that_name_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    use SyncOffer as O;
    let none = Effects::not_requested;
    // Each answer, and what it offers under an accepted and under a
    // finished record, in the entry's order.
    let rows: [(ResultCode, &[O], &[O]); 6] = [
        (ResultCode::RemoteBranchDeleted, &[O::GiveUp], &[]),
        (
            ResultCode::UnlockRequired,
            &[O::GiveUp, O::Unlock],
            &[O::Share],
        ),
        (
            ResultCode::NoSelectedKey,
            &[O::GiveUp, O::OpenSharingAndKeys],
            &[O::Share, O::OpenSharingAndKeys],
        ),
        (
            ResultCode::HostApprovalRequired,
            &[O::GiveUp, O::CheckHost],
            &[O::Share, O::CheckHost],
        ),
        (
            ResultCode::IdentityRequired,
            &[O::Retry, O::GiveUp, O::SetIdentity],
            &[O::Share, O::SetIdentity],
        ),
        (
            ResultCode::PushRejected,
            &[O::Retry, O::GiveUp],
            &[O::Share],
        ),
    ];

    // One accepted share, answered each in turn: a retry is the same
    // request, and its record stays accepted.
    let (shell, root) = open(true, cx);
    let handle = worker(&shell, cx);
    let mut accepted = None;
    for (code, offers, _) in rows {
        let id = match accepted {
            None => real_share(&shell, &root, answer(code, none()), cx).await,
            Some(id) => {
                let held = shell
                    .backend
                    .hold_job_return_answering(JobKind::SynchronizeRepo, answer(code, none()));
                assert!(handle.retry(id, None));
                held.wait_reached();
                held.release();
                shell.until_ended(id, cx).await;
                id
            }
        };
        accepted = Some(id);
        assert_eq!(
            handle.record_state(id),
            Some(RecordState::Accepted),
            "{code:?}"
        );
        assert_row(&shell, id, words_of(code).as_deref(), offers, cx);
    }

    let (shell, root) = open(false, cx);
    for (code, _, offers) in rows {
        let id = real_share(&shell, &root, answer(code, none()), cx).await;
        assert_eq!(
            worker(&shell, cx).record_state(id),
            Some(RecordState::Finished),
            "{code:?}"
        );
        assert_row(&shell, id, words_of(code).as_deref(), offers, cx);
    }
}

/// P14: the answer of a Give up never offers Give up again. A Give up
/// whose re-send answers `unlock_required` offers the step that lets the
/// share finish under its record: Unlock… while the record is still
/// accepted, and here, where the re-send really ended it (fact 17: the
/// test binary reaches no prompt that could keep it open), Share.
#[gpui_kit::test]
async fn a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let locked = || answer(ResultCode::UnlockRequired, Effects::not_requested());
    let (shell, root) = open(true, cx);
    let id = real_share(&shell, &root, locked(), cx).await;
    assert_row(
        &shell,
        id,
        Some(KEY_LOCKED),
        &[SyncOffer::GiveUp, SyncOffer::Unlock],
        cx,
    );

    let held = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeRepo, locked());
    shell.click(request_id(id, offer_part(SyncOffer::GiveUp)), cx);
    held.wait_reached();
    held.release();
    shell.until_ended(id, cx).await;
    assert_eq!(worker(&shell, cx).gave_up(id), Some(GaveUp::Resent));
    assert!(!shell.present(request_id(id, "gave-up"), cx));
    assert_row(&shell, id, Some(KEY_LOCKED), &[SyncOffer::Share], cx);

    // The same answer of a Give up, with the record still accepted.
    let offers = shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .share_result(id, RecordState::Accepted, true)
            .unwrap()
            .offers()
    });
    assert_eq!(offers, [SyncOffer::Unlock]);
}

/// Decision 8: a Give up that finds the record already finished cancels
/// nothing, and the entry says "Already finished" above the stored answer
/// it replays, with no Give up. Here a stopped resume of the share's
/// operation finished its record behind the entry (`bind/resume.rs`: a
/// resume answers to the request that owns the operation it takes up).
#[gpui_kit::test]
async fn giving_up_a_finished_record_says_already_finished(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(true, cx);
    let share = accepted_share(&shell, &root, cx).await;
    assert!(shell.present(request_id(share, offer_part(SyncOffer::GiveUp)), cx));
    let operation = remote_row(&shell, &root, "fetch_prepared");
    let (resume, release) = held_resume(&shell, &root, &operation, cx).await;
    shell.click(request_id(resume, "stop"), cx);
    drop(release);
    shell.until_ended(resume, cx).await;
    assert_eq!(record(&shell, share, cx), Some(RecordDtoState::Finished));

    give_up(&shell, share, cx).await;
    assert_eq!(
        worker(&shell, cx).gave_up(share),
        Some(GaveUp::AlreadyFinished)
    );
    assert_eq!(
        shell.label(request_id(share, "gave-up"), cx).as_deref(),
        Some(ALREADY_FINISHED)
    );
    assert_eq!(code(&shell, share, cx).as_deref(), Some("cancelled"));
    assert_row(
        &shell,
        share,
        Some(STOPPED_BEFORE_SENDING),
        &[SyncOffer::Share],
        cx,
    );
}

/// R-15: with no publication remote an accepted post has no safe point to
/// stop at, so it reads Finishing and offers no Stop.
#[gpui_kit::test]
async fn stop_is_not_offered_without_a_remote(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Local", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let (reached, release) = hold_after_acceptance(&shell);
    let id = worker(&shell, cx).submit(
        Intent::CommentAdd {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
            comment_id: ItemId::generate(),
            parent_id: None,
            body: CommentBody::new("A comment.".to_owned()),
        },
        Lane::User,
        None,
        None,
    );
    reached.recv_timeout(WAIT).expect("the post was accepted");
    until_accepted(&shell, id, cx).await;
    assert!(!shell.present(request_id(id, "stop"), cx));
    assert!(!shell.present(request_id(id, "cancel"), cx));
    assert!(shell.present(request_id(id, "finishing"), cx));
    drop(release);
    shell.until_ended(id, cx).await;
}

/// U-5: a resume over a push whose outcome is not settled, stopped at a
/// safe point, answers `recovery_required`, not `cancelled`
/// (`bind/sync.rs`): the entry reads the library's answer, never that it
/// stopped. The answer is constructed: the test binary pushes nothing.
#[gpui_kit::test]
async fn a_stopped_resume_over_an_unsettled_push_reads_not_confirmed(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(true, cx);
    accepted_share(&shell, &root, cx).await;
    let operation = remote_row(&shell, &root, "fetch_prepared");
    let (resume, release) = held_resume(&shell, &root, &operation, cx).await;
    assert_eq!(
        shell.label(request_id(resume, "stop"), cx).as_deref(),
        Some(STOP)
    );
    shell.click(request_id(resume, "stop"), cx);
    assert_eq!(
        shell.label(request_id(resume, "stop"), cx).as_deref(),
        Some(STOPPING)
    );

    let unsettled = Envelope::classified_mutation(
        "operation resume",
        Scope::default(),
        false,
        ResultCode::RecoveryRequired,
        Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        },
        false,
    );
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::OperationResume, unsettled);
    drop(release);
    returned.wait_reached();
    returned.release();
    shell.until_ended(resume, cx).await;
    assert_eq!(
        code(&shell, resume, cx).as_deref(),
        Some("recovery_required")
    );
    let state = shell.label(request_id(resume, "state"), cx);
    assert_eq!(state.as_deref(), Some("Blocked"));
    assert_ne!(state.as_deref(), Some(CANCELLED));
    let outcome = shell.label(request_id(resume, "outcome"), cx);
    assert!(
        ![Some(STOPPED_BEFORE_SENDING), Some(STOPPED_AFTER_PART)].contains(&outcome.as_deref()),
        "a stop the library did not confirm is not said"
    );
    assert!(!shell.present(request_id(resume, "stop"), cx));
}

/// U-5, and what it leaves of K-5: a resume of a share's operation and
/// the repository's reservation.
///
/// - Stopped once accepted, the resume stops at its first safe point,
///   before the transport: it answers `cancelled`, the operation ends
///   cancelled, the share's record is finished, and the reservation is
///   free, so the next share reserves, reaches the transport and fails it
///   there, not `busy`.
/// - Not stopped, it reaches the transport and fails it: the reservation
///   stays held (the operation is listed `reconciling` with nothing to
///   resume, and still holds it; the share's record stays accepted), so
///   any other share is answered `busy`. That share's own Give up ends
///   it; after a restart no request of the session would hold it (a Known
///   Limit).
#[gpui_kit::test]
async fn a_stopped_resume_that_then_fails_the_transport_reports_the_reservation(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = open(true, cx);
    let first = accepted_share(&shell, &root, cx).await;
    let operation = remote_row(&shell, &root, "fetch_prepared");
    assert_eq!(
        reservation(&shell, &root).as_deref(),
        Some(operation.as_str())
    );
    let (resume, release) = held_resume(&shell, &root, &operation, cx).await;
    shell.click(request_id(resume, "stop"), cx);
    let job = worker(&shell, cx).running_job().expect("the resume runs");
    assert_eq!(job.id, resume);
    assert!(job.cancelled, "Stop cancelled its token");
    drop(release);
    shell.until_ended(resume, cx).await;
    assert_eq!(code(&shell, resume, cx).as_deref(), Some("cancelled"));
    assert_eq!(row(&shell, &root, &operation).state, "cancelled");
    assert_eq!(record(&shell, first, cx), Some(RecordDtoState::Finished));
    assert_eq!(reservation(&shell, &root), None);
    // Freed: the next share reserves and fails the transport, not `busy`.
    let second = accepted_share(&shell, &root, cx).await;

    let operation = remote_row(&shell, &root, "fetch_prepared");
    let resume = worker(&shell, cx).submit(resume_of(&root, &operation), Lane::User, None, None);
    shell.until_ended(resume, cx).await;
    assert_eq!(code(&shell, resume, cx).as_deref(), Some("internal_error"));
    let held = row(&shell, &root, &operation);
    assert_eq!(held.state, "reconciling");
    assert_eq!(held.next_action, None, "nothing to resume");
    assert_eq!(record(&shell, second, cx), Some(RecordDtoState::Accepted));
    assert_eq!(
        reservation(&shell, &root).as_deref(),
        Some(operation.as_str())
    );

    // The share's own Give up.
    // Pressed as the entry's control runs it: the area has scrolled it
    // out of view.
    let returned = shell.backend.hold_job_return(JobKind::SynchronizeRepo);
    let workspace = shell.workspace.clone();
    shell
        .window
        .update(cx, |_, window, cx| {
            workspace.update(cx, |workspace, cx| {
                workspace.run_share_offer(second, SyncOffer::GiveUp, window, cx)
            })
        })
        .unwrap();
    returned.wait_reached();
    returned.release();
    shell.until_ended(second, cx).await;
    assert_eq!(code(&shell, second, cx).as_deref(), Some("cancelled"));
    assert_eq!(record(&shell, second, cx), Some(RecordDtoState::Finished));
    assert_eq!(row(&shell, &root, &operation).state, "cancelled");
    assert_eq!(reservation(&shell, &root), None);
}
