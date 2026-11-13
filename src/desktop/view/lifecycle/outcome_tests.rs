//! The lifecycle dialog once a run is sent (D5 design, A2 Running, A3
//! Outcomes), over real local-only repositories: the steps lit from L3's
//! stages, Stop at a safe point and what it names (P4, A22), Done for a
//! local-only run (R-7), Retry under the same request ID with no repeated
//! checkpoint or merge (P7), a run stopped with no record (P18), a dirty
//! main met at execute (P19), and the operation area's entry, which reopens
//! the dialog. The canned rows (P6, the way out, credentials) are
//! `outcome_canned_tests.rs`.
//!
//! The stage holds (`hold_at_stage`) park the worker once the stage has
//! been sent: the library may hold the repository's lease meanwhile, so no
//! test opens a second repository operation while one is parked.

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{FailurePoint, ProgressStage, RepositoryService},
    results::ResultCode,
};

use super::{
    confirm::{KEEP_LOCAL, MARK_CLOSED, SAVE_STEP, TIDY_UP},
    confirm_tests::{confirming, paths, row, worktree},
    outcome::{
        CANCELLATION_FINISHED, ENDING_DONE, ENDING_FAILED, ENDING_STOPPED, NO_RECORD,
        OPEN_REPOSITORY_DETAILS, SHARING_PENDING, STATE_DONE, STATE_FAILED, STATE_NOT_DONE,
        STATE_NOT_STARTED, STATE_RUNNING, STOP, STOPPING, TIDIED_UP, merged, not_merged,
    },
    prepare::primary_dirty,
    short_id,
    tests::{
        TICKET, closes, code, commits, dialog, request, saved_ticket, shell, shown, until_present,
    },
};
use crate::desktop::{
    services::{JobKind, requests::DesktopRequestId},
    view::{
        harness::Shell,
        operations::{OPEN_LIFECYCLE, recovery_id, request_id},
    },
};

/// A local-only close's steps, as its preview plans them.
fn local_steps() -> [&'static str; 5] {
    [
        SAVE_STEP,
        KEEP_LOCAL,
        MARK_CLOSED,
        "Merge into main",
        TIDY_UP,
    ]
}

/// Waits until the request model has heard the request `id` report
/// `stage`.
pub(super) async fn at_stage(
    shell: &Shell,
    id: DesktopRequestId,
    stage: ProgressStage,
    cx: &mut TestAppContext,
) {
    use crate::desktop::services::requests::RequestState;
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                matches!(
                    request.state,
                    RequestState::Running { stage: Some(at), .. } if at == stage
                )
            })
        })
        .await;
}

/// Commits reachable from any of `branches` of `root`: what a merge of
/// them holds before its own commit.
fn reachable(root: &Path, branches: &[&str]) -> usize {
    let repository = git2::Repository::open(root).unwrap();
    let mut walk = repository.revwalk().unwrap();
    for branch in branches {
        let tip = repository
            .revparse_single(&format!("refs/heads/{branch}"))
            .unwrap()
            .peel_to_commit()
            .unwrap();
        walk.push(tip.id()).unwrap();
    }
    walk.count()
}

/// The tip of `branch` of `root`.
fn tip(root: &Path, branch: &str) -> git2::Oid {
    git2::Repository::open(root)
        .unwrap()
        .refname_to_id(&format!("refs/heads/{branch}"))
        .unwrap()
}

/// Opens the dialog on the ticket, confirms, holds the close once it
/// reports `integrating`, presses Stop at a safe point there, and lets it
/// go on: it stops before the merge (design, Testing, P4). Gives the
/// close's request.
pub(super) async fn stopped_at_integrating(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    confirming(shell, root, cx).await;
    let main = commits(root, "main");
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(shell, cx)[0];
    at_stage(shell, id, ProgressStage::Integrating, cx).await;
    until_present(shell, "lifecycle-stop", cx).await;
    assert_eq!(shell.label("lifecycle-stop", cx).as_deref(), Some(STOP));
    shell.click("lifecycle-stop", cx);
    assert_eq!(
        shell.label("lifecycle-stopping", cx).as_deref(),
        Some(STOPPING)
    );
    assert!(!shell.present("lifecycle-stop", cx), "pressed once");
    gate.release();
    shell.until_ended(id, cx).await;
    assert_eq!(code(shell, id, cx), Some(ResultCode::Cancelled));
    assert_eq!(commits(root, "main"), main, "stopped before the merge");
    until_present(shell, "lifecycle-ending", cx).await;
    id
}

/// L3, real: each stage of a local-only close lights its step; the steps
/// before it are done and those after it not started, and Stop at a safe
/// point is offered throughout. Once it finishes, every step is done.
#[gpui_kit::test]
async fn running_lights_each_step_from_its_stage(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Lit", ""));
    confirming(&shell, &root, cx).await;
    let stages = [
        ProgressStage::Recording,
        ProgressStage::Integrating,
        ProgressStage::CleaningUp,
    ];
    let gates = stages.map(|stage| shell.backend.hold_at_stage(JobKind::TicketClose, stage));
    shell.click("lifecycle-confirm", cx);
    let (done, running, waiting) = (STATE_DONE, STATE_RUNNING, STATE_NOT_STARTED);
    let lit = [
        [done, done, running, waiting, waiting],
        [done, done, done, running, waiting],
        [done, done, done, done, running],
    ];
    for ((stage, states), gate) in stages.into_iter().zip(lit).zip(&gates) {
        gate.wait_reached();
        let id = closes(&shell, cx)[0];
        at_stage(&shell, id, stage, cx).await;
        shell.run(cx);
        assert_eq!(shell.rows("lifecycle-step", cx), local_steps());
        assert_eq!(shell.rows("lifecycle-step-state", cx), states, "{stage:?}");
        assert_eq!(shell.label("lifecycle-stop", cx).as_deref(), Some(STOP));
        gate.release();
    }
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    assert_eq!(shell.rows("lifecycle-step-state", cx), [done; 5]);
    assert!(!shell.present("lifecycle-stop", cx));
}

/// P4, A22, real: Stop at a safe point while the close is at
/// `integrating` stops it before the merge. The dialog says it stopped,
/// that the cancellation finished, and which steps completed; the merge
/// and the tidy-up are not done, Retry is offered, and the header still
/// names the item, its repository and its context.
#[gpui_kit::test]
async fn stop_at_a_safe_point_names_completed_steps(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Stopped", "")
    });
    let branch = shown(&shell, &root, TICKET).context.branch.unwrap();
    stopped_at_integrating(&shell, &root, cx).await;

    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_STOPPED)
    );
    assert_eq!(
        shell.label("lifecycle-cancellation", cx).as_deref(),
        Some(CANCELLATION_FINISHED)
    );
    assert_eq!(
        shell.label("lifecycle-completed", cx),
        Some(format!(
            "Completed: {SAVE_STEP}, {KEEP_LOCAL}, {MARK_CLOSED}."
        ))
    );
    assert_eq!(
        shell.rows("lifecycle-step-state", cx),
        [
            STATE_DONE,
            STATE_DONE,
            STATE_DONE,
            STATE_NOT_DONE,
            STATE_NOT_DONE
        ]
    );
    assert_eq!(shell.rows("lifecycle-outcome", cx), [not_merged("main")]);
    assert!(shell.present("lifecycle-retry", cx));
    assert_eq!(
        shell.label("lifecycle-subtitle", cx),
        Some(format!("{} \u{b7} Stopped \u{b7} a", short_id(TICKET)))
    );
    assert_eq!(shell.label("lifecycle-branch", cx), Some(branch));
}

/// R-7, real: a local-only close whose tidy-up completes is Done: merged,
/// sharing pending with no remote, tidied up; every step done and nothing
/// to retry. Enter closes the dialog.
#[gpui_kit::test]
async fn local_only_with_cleanup_complete_is_done(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Done", ""));
    confirming(&shell, &root, cx).await;
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
    until_present(&shell, "lifecycle-ending", cx).await;

    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    assert_eq!(
        shell.rows("lifecycle-outcome", cx),
        [merged("main").as_str(), SHARING_PENDING, TIDIED_UP]
    );
    assert_eq!(shell.rows("lifecycle-step", cx), local_steps());
    assert_eq!(shell.rows("lifecycle-step-state", cx), [STATE_DONE; 5]);
    assert!(!shell.present("lifecycle-retry", cx));
    shell.keys("enter", cx);
    assert!(!shell.present("lifecycle-dialog", cx), "Enter closes");
    assert_eq!(closes(&shell, cx), [id], "and sends nothing");
}

/// P7, Review Focus 1, real: Retry after a stop at a safe point sends the
/// same request ID again, with no confirmation, and the close finishes. It
/// adds no checkpoint and one merge only: main then holds exactly what it
/// and the stopped branch held, and one merge commit whose second parent is
/// the branch as the stop left it. A local-only run publishes nothing; that
/// a Retry repeats no publication is the library's to prove, over a real
/// remote (`tests/mutation_remote/c5_lifecycle.rs` `push_rejected_continues`
/// and `deletion_rejected`).
#[gpui_kit::test]
async fn retry_sends_the_same_request_id(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Again", ""));
    let branch = shown(&shell, &root, TICKET).context.branch.unwrap();
    let stopped = stopped_at_integrating(&shell, &root, cx).await;
    let held = reachable(&root, &["main", &branch]);
    let checkpoint = tip(&root, &branch);

    shell.click("lifecycle-retry", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Retry sent the close again");
    let (first, again) = (request(&shell, stopped, cx), request(&shell, sent[0], cx));
    assert_eq!(again.request_id, first.request_id, "the same request");
    assert!(again.confirmation.is_none(), "no confirmation");
    assert_eq!(
        dialog(cx).read_with(cx, |dialog, _| dialog.submitted()),
        Some(sent[0])
    );
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));

    assert_eq!(commits(&root, "main"), held + 1, "one merge, no checkpoint");
    let repository = git2::Repository::open(&root).unwrap();
    let merge = repository.find_commit(tip(&root, "main")).unwrap();
    let parents: Vec<git2::Oid> = merge.parent_ids().collect();
    assert_eq!(parents.len(), 2, "a merge commit");
    assert_eq!(parents[1], checkpoint, "the branch as the stop left it");
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
}

/// P18, real: a run that stopped between its acceptance and its record
/// (the library's failure point `BeforeRequestDomainCall`) offers Retry.
/// With the branch moved meanwhile, the same request is refused
/// `external_change` and its record is deleted: the dialog says nothing of
/// it is kept and offers Prepare again, not Retry. Prepare again shows the
/// new summary, and Confirm then sends a new request, which closes the
/// ticket.
#[gpui_kit::test]
async fn a_stopped_run_with_no_record_offers_prepare(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Gone", ""));
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeRequestDomainCall,
        )
        .unwrap(),
    );
    confirming(&shell, &root, cx).await;
    let main = commits(&root, "main");
    shell.click("lifecycle-confirm", cx);
    let first = closes(&shell, cx)[0];
    shell.until_ended(first, cx).await;
    assert_eq!(code(&shell, first, cx), Some(ResultCode::InternalError));
    until_present(&shell, "lifecycle-retry", cx).await;

    let context = worktree(&shell, &root);
    shell
        .fixture
        .commit_file(&context, "src/moved.rs", b"// moved\n");
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    shell.click("lifecycle-retry", cx);
    let second = closes(&shell, cx)[0];
    assert_ne!(second, first);
    shell.until_ended(second, cx).await;
    assert_eq!(code(&shell, second, cx), Some(ResultCode::ExternalChange));
    until_present(&shell, "lifecycle-no-record", cx).await;
    assert_eq!(
        shell.label("lifecycle-no-record", cx).as_deref(),
        Some(NO_RECORD)
    );
    assert!(shell.present("lifecycle-prepare-again", cx));
    assert!(!shell.present("lifecycle-retry", cx));
    assert_eq!(commits(&root, "main"), main, "nothing ran");

    shell.click("lifecycle-prepare-again", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert!(
        paths(&shell, cx).contains(&row("src/moved.rs", "code", "added")),
        "{:?}",
        paths(&shell, cx)
    );
    shell.click("lifecycle-confirm", cx);
    let third = closes(&shell, cx)[0];
    assert_ne!(
        request(&shell, third, cx).request_id,
        request(&shell, first, cx).request_id,
        "a new request"
    );
    shell.until_ended(third, cx).await;
    assert_eq!(code(&shell, third, cx), Some(ResultCode::Ok));
    assert!(commits(&root, "main") > main);
}

/// P19, real (the library's `blocked_at_run`, through its request hook): main
/// becomes dirty between Confirm and the merge. The run stops at execute
/// naming the dirty main, with Open folder and the repository's details;
/// nothing is merged, the stored answer offers no Retry, and Prepare
/// again is offered.
#[gpui_kit::test]
async fn a_dirty_primary_stop_at_execute_merges_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Dirty", ""));
    confirming(&shell, &root, cx).await;
    let main = commits(&root, "main");
    let dirty = root.join("dirty-primary.txt");
    shell
        .fixture
        .service()
        .set_request_hook_for_testing(move || std::fs::write(dirty, b"dirty\n").unwrap());

    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::RecoveryRequired));
    until_present(&shell, "lifecycle-ending", cx).await;

    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_FAILED)
    );
    assert_eq!(
        shell.rows("lifecycle-blocker", cx),
        [primary_dirty(Some("main"))]
    );
    // Review M-1: a dirty main stands in the merge's way.
    assert_eq!(
        shell.rows("lifecycle-step-state", cx),
        [
            STATE_DONE,
            STATE_NOT_DONE,
            STATE_NOT_DONE,
            STATE_FAILED,
            STATE_NOT_DONE
        ]
    );
    assert!(shell.present("lifecycle-open-folder", cx));
    assert_eq!(
        shell.label("lifecycle-inspect", cx).as_deref(),
        Some(OPEN_REPOSITORY_DETAILS)
    );
    assert_eq!(shell.rows("lifecycle-outcome", cx), [not_merged("main")]);
    assert!(!shell.present("lifecycle-retry", cx));
    assert!(shell.present("lifecycle-prepare-again", cx));
    assert_eq!(commits(&root, "main"), main, "nothing is merged");
}

/// A3, real: the operation area's entry names the command and the item by
/// its short ID, and shows the latest stage while it runs. Once stopped it
/// offers Open…, which reopens the dialog on that request, closed meanwhile;
/// its Retry there finishes the close.
#[gpui_kit::test]
async fn the_operation_entry_reopens_the_dialog(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Entry", ""));
    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(&shell, cx)[0];
    at_stage(&shell, id, ProgressStage::Integrating, cx).await;
    shell.run(cx);
    assert_eq!(
        shell.label(request_id(id, "command"), cx).as_deref(),
        Some("Close ticket")
    );
    assert_eq!(
        shell.label(request_id(id, "item"), cx),
        Some(short_id(TICKET))
    );
    assert_eq!(
        shell.label(request_id(id, "state"), cx).as_deref(),
        Some("Closing \u{b7} Merging into main\u{2026}")
    );
    assert!(
        !shell.present(request_id(id, "open"), cx),
        "not while running"
    );
    shell.click("lifecycle-stop", cx);
    gate.release();
    shell.until_ended(id, cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));

    assert_eq!(
        shell.label(request_id(id, "open"), cx).as_deref(),
        Some(OPEN_LIFECYCLE)
    );
    // R-1: a lifecycle's recovery is its dialog's, never the raw recovery
    // actions: Prepare again here would be the generic ConfirmDialog.
    for n in 0..4 {
        assert!(
            !shell.present(recovery_id(id, n), cx),
            "the entry has no raw recovery action {n}"
        );
    }
    shell.click(request_id(id, "open"), cx);
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        dialog(cx).read_with(cx, |dialog, _| dialog.submitted()),
        Some(id)
    );
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_STOPPED)
    );
    assert_eq!(
        shell.label("lifecycle-title", cx).as_deref(),
        Some("Close ticket")
    );
    // Review M-5: the workspace knows the repository has no remote, and its
    // main branch, so the steps are the local-only ones.
    assert_eq!(shell.rows("lifecycle-step", cx), local_steps());
    shell.click("lifecycle-retry", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2);
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
}

/// Unresolved remote history is guidance in the CLI, in the dialog's
/// blocker, its way out and a finished run's way out alike, and is told
/// apart from push-proof unknown, which polls.
#[test]
fn unresolved_history_guides_to_the_command_line() {
    use crate::desktop::model::operations::{
        HISTORY_UNRESOLVED_BLOCKER, HISTORY_UNRESOLVED_GUIDANCE,
    };
    use manyhands::repository::{LifecycleBlocker, LifecycleWayOut};
    assert_eq!(
        super::prepare::blocker_words(LifecycleBlocker::HistoryUnresolved, None),
        HISTORY_UNRESOLVED_BLOCKER
    );
    assert_ne!(
        super::prepare::blocker_words(LifecycleBlocker::HistoryUnknown, None),
        HISTORY_UNRESOLVED_BLOCKER
    );
    let way = LifecycleWayOut::ResolveHistory;
    let expected = Some(HISTORY_UNRESOLVED_GUIDANCE.to_owned());
    assert_eq!(super::prepare::guidance(&way, "ticket", "main"), expected);
    assert_eq!(super::outcome::guidance(&way, "ticket", "main"), expected);
    assert_ne!(
        super::prepare::guidance(&LifecycleWayOut::PollOnce, "ticket", "main"),
        expected
    );
}
