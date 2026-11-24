//! A stop that leaves no record (review I-1): the library answers
//! `cancelled` with no data and no recovery and deletes the request, so
//! nothing of it can be continued, and a new prepare is what follows.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::MutationDataDto,
    results::{Effects, Envelope, ResultCode, Scope},
};

use super::{
    confirm_tests::{confirming, worktree},
    outcome::{ENDING_STOPPED, NO_RECORD},
    tests::{closes, code, commits, request, saved_ticket, shell, until_present},
};
use crate::desktop::{services::JobKind, view::harness::key};

/// Canned (decision 9): a Stop pressed before the parent record exists
/// cannot be reached for real. A real Stop held at `accepted` already
/// finds the parent standing, and answers with its data and
/// `request.retry`: `tests/mutation_remote/c5_cancel.rs` `no_parent_yet`
/// shows that the flag stops the run "by when the parent stands". The answer
/// is the one `src/repository/mutation/bind/lifecycle.rs` `stopped` builds
/// with no parent: `cancelled`, no data, and `no_parent_recovery(cancelled)`,
/// which is empty, since `cancelled` does not pass by itself
/// (`bind/lifecycle_tests.rs`
/// `with_no_parent_a_code_that_would_resend_prepares_again`). The real call
/// under it is refused before acceptance: the ticket's branch moved after
/// the preview, so the library keeps no record, as it does after deleting
/// one. The dialog says Stopped and that nothing of it is kept. It offers
/// Prepare again, not Retry; that shows the new summary, and Confirm sends
/// a new request.
#[gpui_kit::test]
async fn a_stop_before_any_record_offers_prepare(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Early", ""));
    confirming(&shell, &root, cx).await;
    let context = worktree(&shell, &root);
    shell
        .fixture
        .commit_file(&context, "src/early.rs", b"// early\n");
    let stopped = Envelope::<MutationDataDto>::classified_mutation(
        "ticket close",
        Scope {
            repository: Some(key(&root)),
            ..Scope::default()
        },
        true,
        ResultCode::Cancelled,
        Effects::not_requested(),
        false,
    );
    let gate = shell
        .backend
        .hold_job_return_answering(JobKind::TicketClose, stopped);
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    gate.release();
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Cancelled));
    until_present(&shell, "lifecycle-ending", cx).await;

    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_STOPPED)
    );
    assert_eq!(
        shell.label("lifecycle-no-record", cx).as_deref(),
        Some(NO_RECORD)
    );
    assert!(shell.present("lifecycle-prepare-again", cx));
    assert!(!shell.present("lifecycle-retry", cx));

    let main = commits(&root, "main");
    shell.click("lifecycle-prepare-again", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.click("lifecycle-confirm", cx);
    let again = closes(&shell, cx)[0];
    assert_ne!(
        request(&shell, again, cx).request_id,
        request(&shell, id, cx).request_id,
        "a new request"
    );
    shell.until_ended(again, cx).await;
    assert_eq!(code(&shell, again, cx), Some(ResultCode::Ok));
    assert!(commits(&root, "main") > main);
}
