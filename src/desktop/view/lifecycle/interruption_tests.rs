//! Interrupted lifecycles and what they commit (D5 design, Testing; P4,
//! P5, A8, A9; R-33, K-25), over real local-only repositories: a close
//! stopped at a safe point before its merge, then Retried; a close another
//! process stranded at `closing`, Resumed from a fresh shell; a close whose
//! answer was lost before its request settled; and a close whose tidy-up
//! was refused after its merge, replayed to tidy up only. Each journey counts
//! the commits on primary (`main`) and on the ticket's context branch
//! before and after every step, against the counts the library's
//! local-only close makes from `saved_ticket`'s ticket: main holds 2 (the
//! repository's first commit and the ticket's), the context 3 (those and
//! the ticket's save); the closure checkpoint adds 1 to the context; the
//! merge brings the context's own commits into main with 1 merge commit;
//! the tidy-up removes the context branch.
//!
//! The stage holds (`hold_at_stage`) park the worker once the stage has
//! been sent; the library may hold the repository's lease meanwhile, so no
//! `RepositoryService` call is made while a hold is parked.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, FailurePoint, LifecycleCleanupPendingReason, Mutation,
        MutationCall, NoProgress, OperationAction, PrepareDataDto, ProgressStage,
        RepositoryService, RequestId, RequestState, TicketCloseInput, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{
    confirm_tests::{confirming, worktree},
    lock_tests::{lock, step, until_lock},
    outcome::{ENDING_DONE, SHARING_PENDING, TIDIED_UP, merged},
    outcome_tests::{at_stage, stopped_at_integrating},
    tests::{
        TICKET, closes, code, open, request, saved_ticket, shell, shown, ticket, until_present,
    },
};
use crate::desktop::{
    model::{
        lifecycle_lock::{LockSource, LockState, Untidied},
        list_state::ListState,
    },
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
        operations::RESUME,
    },
};

/// Main and the context before any close.
const BEFORE: (usize, Option<usize>) = (2, Some(3));
/// After the closure checkpoint and no merge.
const CHECKPOINTED: (usize, Option<usize>) = (2, Some(4));
/// After the merge: main's 2, the context's own 2 and one merge commit,
/// with the context kept.
const MERGED_UNTIDIED: (usize, Option<usize>) = (5, Some(4));
/// After the merge and the tidy-up: the context branch is gone.
const MERGED_TIDIED: (usize, Option<usize>) = (5, None);

/// The tip of `branch` of `root`, while it exists.
fn tip(root: &Path, branch: &str) -> Option<git2::Oid> {
    git2::Repository::open(root)
        .unwrap()
        .refname_to_id(&format!("refs/heads/{branch}"))
        .ok()
}

/// Commits reachable from `branch` of `root`, while it exists.
fn count(root: &Path, branch: &str) -> Option<usize> {
    let repository = git2::Repository::open(root).unwrap();
    let tip = tip(root, branch)?;
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip).unwrap();
    Some(walk.count())
}

/// Commits on main and on the context `branch`.
fn counts(root: &Path, branch: &str) -> (usize, Option<usize>) {
    (
        count(root, "main").expect("main exists"),
        count(root, branch),
    )
}

/// The parents of main's tip.
fn main_parents(root: &Path) -> Vec<git2::Oid> {
    let repository = git2::Repository::open(root).unwrap();
    let tip = tip(root, "main").unwrap();
    repository.find_commit(tip).unwrap().parent_ids().collect()
}

/// Where the library's record of `request_id` stands.
fn recorded(service: &RepositoryService, request_id: RequestId) -> RequestState {
    service
        .show_request(request_id)
        .data
        .expect("the request is recorded")
        .state
}

/// A window over `a`, with a ticket of its own working copy: the shell,
/// the root and the ticket's context branch.
fn ticket_shell(cx: &mut TestAppContext, title: &str) -> (Shell, PathBuf, String) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, title, ""));
    let branch = shown(&shell, &root, TICKET)
        .context
        .branch
        .expect("a context of its own");
    (shell, root, branch)
}

/// P4, A8, real: a close held at `integrating` by the progress-sink seam
/// and stopped there ends before the merge: the context gains the closure
/// checkpoint only, main nothing, and the library's record of the request
/// stays accepted. Retry sends the same request, which merges once, adds
/// no second checkpoint (the merge's second parent is the context as the
/// stop left it) and tidies up: Done, the context branch and working copy
/// gone.
#[gpui_kit::test]
async fn p4_a_stopped_close_retries_with_one_checkpoint_and_one_merge(cx: &mut TestAppContext) {
    let (shell, root, branch) = ticket_shell(cx, "Interrupted");
    let context = worktree(&shell, &root);
    assert!(context.exists(), "a working copy of its own");
    assert_eq!(counts(&root, &branch), BEFORE, "before");

    let stopped = stopped_at_integrating(&shell, &root, cx).await;
    assert_eq!(
        counts(&root, &branch),
        CHECKPOINTED,
        "stopped before the merge"
    );
    let request_id = request(&shell, stopped, cx).request_id;
    assert_eq!(
        recorded(shell.fixture.service(), request_id),
        RequestState::Accepted
    );
    let (main, checkpoint) = (tip(&root, "main"), tip(&root, &branch));

    shell.click("lifecycle-retry", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Retry sent the close again");
    assert_eq!(sent[1], stopped, "after the stopped run");
    assert_eq!(request(&shell, sent[0], cx).request_id, request_id);
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));

    assert_eq!(
        counts(&root, &branch),
        MERGED_TIDIED,
        "one merge, no checkpoint"
    );
    assert_eq!(
        main_parents(&root),
        [main.unwrap(), checkpoint.unwrap()],
        "merged from the context as the stop left it"
    );
    assert_eq!(
        recorded(shell.fixture.service(), request_id),
        RequestState::Finished
    );
    assert!(!context.exists(), "its working copy is tidied up");
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
}

/// A9, real: a cleanup-only replay. A file left in the working copy while
/// the close tidies up stops the tidy-up after the merge (`context_changed`,
/// the recipe of `tests/mutation_remote/c5_cancel.rs`
/// `stopped_on_context_changed`): main is merged and the context kept. With
/// the file gone, Retry sends the same request, which only tidies up: held
/// at `cleaning_up`, main is exactly as the first run left it, and once
/// done main's tip is the same merge commit, with no checkpoint or merge
/// repeated, and the context branch and working copy are gone. The replay's
/// stages are not recorded by the desktop (the request model keeps only the
/// latest), so no `recording` or `integrating` is shown by main's unchanged
/// tip and counts; the library pins the stage order (`tests/mutation_remote/
/// d5_progress.rs`).
#[gpui_kit::test]
async fn a_cleanup_only_replay_repeats_no_merge_or_checkpoint(cx: &mut TestAppContext) {
    let (shell, root, branch) = ticket_shell(cx, "Untidied");
    let context = worktree(&shell, &root);
    assert!(context.exists(), "a working copy of its own");
    assert_eq!(counts(&root, &branch), BEFORE, "before");

    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::CleaningUp);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let left = context.join("untracked.txt");
    std::fs::write(&left, b"the owner's\n").unwrap();
    gate.release();
    let first = closes(&shell, cx)[0];
    shell.until_ended(first, cx).await;
    assert_eq!(code(&shell, first, cx), Some(ResultCode::RecoveryRequired));
    // Why it stopped: its answer's pending tidy-up is `context_changed`.
    let untidied = LockState::Untidied(Untidied {
        reason: LifecycleCleanupPendingReason::ContextChanged,
        request: first,
    });
    until_lock(&shell, &root, untidied, cx).await;
    assert_eq!(
        counts(&root, &branch),
        MERGED_UNTIDIED,
        "merged, not tidied"
    );
    let merge = tip(&root, "main");
    let request_id = request(&shell, first, cx).request_id;

    std::fs::remove_file(&left).unwrap();
    until_present(&shell, "lifecycle-retry", cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::CleaningUp);
    shell.click("lifecycle-retry", cx);
    gate.wait_reached();
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Retry sent the close again");
    assert_eq!(request(&shell, sent[0], cx).request_id, request_id);
    at_stage(&shell, sent[0], ProgressStage::CleaningUp, cx).await;
    assert_eq!(
        counts(&root, &branch),
        MERGED_UNTIDIED,
        "nothing merged again"
    );
    gate.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));

    assert_eq!(counts(&root, &branch), MERGED_TIDIED, "tidied only");
    assert_eq!(tip(&root, "main"), merge, "the same merge");
    assert!(!context.exists(), "its working copy is tidied up");
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    assert_eq!(
        shell.rows("lifecycle-outcome", cx),
        [merged("main").as_str(), SHARING_PENDING, TIDIED_UP]
    );
}

/// A repository `a` with a ticket of its own working copy, whose close
/// another process broke at `point` before any window opened, through a
/// service of its own over the same data directory (as `lifecycle_tests.rs`
/// `strand_close` breaks one): the fixture, the root, the context branch,
/// the commits before the break and the broken request.
fn broken_at(point: FailurePoint) -> (Fixture, PathBuf, String, (usize, Option<usize>), RequestId) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Broken", "");
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let branch = service
        .show_item(&repo, &item_id(TICKET))
        .unwrap()
        .context
        .branch
        .expect("a context of its own");
    let before = counts(&root, &branch);

    let failing =
        RepositoryService::open_at_with_failure_point_for_testing(fixture.data_dir(), point)
            .unwrap();
    let mutation = || {
        Mutation::TicketClose(TicketCloseInput {
            root: root.clone(),
            id: item_id(TICKET),
        })
    };
    let prepared = failing.prepare(&mutation());
    let preview = prepared
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .expect("the close is ready");
    let request_id = RequestId::new();
    let broken = failing.execute(
        MutationCall {
            request_id,
            confirmation: Some(ConfirmationId::parse(&preview.confirmation_id).unwrap()),
            mutation: mutation(),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_ne!(broken.code, ResultCode::Ok, "{}", broken.message);
    drop(failing);
    (fixture, root, branch, before, request_id)
}

/// The requests of this session whose intent `matches`, newest first.
fn sent(
    shell: &Shell,
    cx: &mut TestAppContext,
    matches: fn(&Intent) -> bool,
) -> Vec<DesktopRequestId> {
    shell.requests(cx, move |request| matches(&request.intent))
}

/// P5, A8, R-33, real: a close another process broke after recording
/// `closing` (`ClosureAfterClosingRecorded`, as `lock_restart_tests.rs` and
/// `operations/resume_tests.rs` strand one) left its request accepted, its
/// `closing` step recorded, and the context and main untouched. A fresh shell
/// over the same data directory (as `item/posting_tests/relaunch.rs` opens
/// one) reads the ticket locked, and its Close row offers Resume. Resume
/// answers `original_request_required`; the desktop sends the named
/// request again, and the dialog follows it to Done. One checkpoint, one
/// merge: main's merge has for its second parent one commit, the closure
/// checkpoint, over the context as the break left it, and the context is
/// tidied up.
#[gpui_kit::test]
async fn p5_a_close_stranded_at_closing_resumes_after_relaunch(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (fixture, root, branch, before, holder) =
        broken_at(FailurePoint::ClosureAfterClosingRecorded);
    assert_eq!(before, BEFORE, "before the break");
    assert_eq!(counts(&root, &branch), BEFORE, "nothing committed");
    assert_eq!(recorded(fixture.service(), holder), RequestState::Accepted);
    let (main, context) = (tip(&root, "main"), tip(&root, &branch));

    let shell = Shell::open(fixture, cx);
    let working_copy = worktree(&shell, &root);
    assert!(working_copy.exists(), "the context's working copy is kept");
    assert_eq!(step(&shell, &root).as_deref(), Some("closing"));
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;
    let close = OperationAction::Close.as_str().to_owned();
    let wanted = close.clone();
    shell
        .until_drawn(cx, "the close's row", move |shell, cx| {
            shell.rows("pending-command", cx).contains(&wanted)
        })
        .await;
    let row = shell
        .rows("pending-command", cx)
        .iter()
        .position(|listed| *listed == close)
        .expect("listed");
    assert_eq!(
        shell.label(("pending-resume", row), cx).as_deref(),
        Some(RESUME)
    );

    shell.click(("pending-resume", row), cx);
    let resumes = sent(&shell, cx, |intent| {
        matches!(intent, Intent::OperationResume { .. })
    });
    assert_eq!(resumes.len(), 1, "one resume of the row");
    shell.until_ended(resumes[0], cx).await;
    assert_eq!(
        code(&shell, resumes[0], cx),
        Some(ResultCode::OriginalRequestRequired)
    );
    shell
        .until_drawn(cx, "the named close sent again", |shell, cx| {
            !closes(shell, cx).is_empty()
        })
        .await;
    let again = closes(&shell, cx);
    assert_eq!(again.len(), 1, "sent once");
    assert_eq!(request(&shell, again[0], cx).request_id, holder);
    shell.until_ended(again[0], cx).await;
    assert_eq!(code(&shell, again[0], cx), Some(ResultCode::Ok));
    shell
        .until_drawn(cx, "the dialog at Done", |shell, cx| {
            shell.label("lifecycle-ending", cx).as_deref() == Some(ENDING_DONE)
        })
        .await;

    assert_eq!(
        counts(&root, &branch),
        MERGED_TIDIED,
        "one checkpoint, one merge"
    );
    let parents = main_parents(&root);
    assert_eq!(parents.len(), 2, "a merge commit");
    assert_eq!(parents[0], main.unwrap(), "over main as it was");
    let repository = git2::Repository::open(&root).unwrap();
    let checkpoint = repository.find_commit(parents[1]).unwrap();
    assert_eq!(
        checkpoint.parent_ids().collect::<Vec<_>>(),
        [context.unwrap()],
        "one checkpoint over the context as the break left it"
    );
    assert_eq!(
        recorded(shell.fixture.service(), holder),
        RequestState::Finished
    );
    assert!(!working_copy.exists(), "its working copy is tidied up");
    until_lock(&shell, &root, LockState::Open, cx).await;
}

/// K-25, R-33, real: a close whose answer was lost before its request
/// settled (`BeforeRequestSettlement`) had already run to its end: merged,
/// closed and tidied up. A fresh shell over the same data directory, once
/// it has read the repository's pending operations, lists no lifecycle row
/// and holds no lock; the ticket reads closed and offers no Close ticket….
/// The request stays accepted: nothing in the desktop settles it.
#[gpui_kit::test]
async fn a_close_lost_before_settlement_is_complete_and_stays_accepted(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (fixture, root, branch, before, lost) = broken_at(FailurePoint::BeforeRequestSettlement);
    assert_eq!(before, BEFORE, "before the break");
    assert_eq!(counts(&root, &branch), MERGED_TIDIED, "it ran to its end");
    assert_eq!(recorded(fixture.service(), lost), RequestState::Accepted);

    let shell = Shell::open(fixture, cx);
    let closure = shown(&shell, &root, TICKET).closure;
    assert!(
        closure.and_then(|closure| closure.closed_at).is_some(),
        "the ticket reads closed"
    );
    assert_eq!(step(&shell, &root), None, "no lifecycle row");
    let listed = key(&root);
    shell
        .until(cx, move |operations| {
            matches!(
                operations.pending().get(&listed),
                Some(ListState::Ready { .. })
            )
        })
        .await;
    shell.frame(cx, |_, _| ());
    assert!(
        !shell
            .rows("pending-command", cx)
            .contains(&OperationAction::Close.as_str().to_owned()),
        "no Close row"
    );
    assert_eq!(lock(&shell, &root, cx), LockState::Open, "no lock");
    open(&shell, ticket(&root), cx);
    shell
        .until_drawn(cx, "the ticket's details", |shell, cx| {
            shell.present("chip-all", cx)
        })
        .await;
    shell.click("chip-all", cx);
    until_present(&shell, "item-lifecycle", cx).await;
    assert_eq!(shell.label("item-lifecycle", cx).as_deref(), Some("Closed"));
    assert!(
        !shell.present(LIFECYCLE_ACTION, cx),
        "no Close ticket\u{2026}"
    );
    assert!(closes(&shell, cx).is_empty(), "nothing is sent");
    assert_eq!(
        recorded(shell.fixture.service(), lost),
        RequestState::Accepted
    );
}
