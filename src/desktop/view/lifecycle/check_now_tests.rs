//! D5's `PollOnce` way out gains a Check now button (D6 design C2, Document
//! Amendment 3, superseding D5's K-12): a blocked prepare whose main is
//! ahead of its remote says "Check origin for others' changes", and the
//! button sends `poll once` for the item's repository on the user lane. The
//! prepare is canned (decision 9), as in `a_blocked_prepare_lists_blockers_
//! and_way_out`; the poll itself fails closed here for want of a key and
//! reaches no network.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{LifecycleBlockedDto, LifecycleBlocker, PrepareDataDto},
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use super::tests::{TICKET, open, saved_ticket, shell, ticket, until_present};
use crate::desktop::{
    services::{
        JobKind,
        requests::{Intent, Lane},
    },
    view::{harness::key, item::LIFECYCLE_ACTION},
};

const CHECK_NOW: &str = "lifecycle-check-now";

/// The button is offered beside the guidance; pressed, it sends one `poll
/// once` of the item's repository and the dialog says the check runs, in
/// place of the button, until it ended; the button is then offered again.
#[gpui_kit::test]
async fn way_out_check_now(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Ahead", "");
        fixture.publish_over_ssh(root);
    });
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
        blockers: vec![LifecycleBlocker::PrimaryAheadOfRemote],
    }));
    shell.backend.answer_next_prepare("ticket close", blocked);
    open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-prepare-again", cx).await;

    let polls = |cx: &mut TestAppContext| {
        shell.requests(
            cx,
            |request| matches!(&request.intent, Intent::PollOnce { root: asked } if *asked == root),
        )
    };
    assert_eq!(
        shell.label(CHECK_NOW, cx).as_deref(),
        Some("Check now"),
        "beside the guidance"
    );
    assert!(shell.rows("lifecycle-way-out", cx)[0].contains("Check origin"));
    assert!(polls(cx).is_empty());

    let gate = shell.backend.hold_job(JobKind::PollOnce);
    shell.click(CHECK_NOW, cx);
    let sent = polls(cx);
    assert_eq!(sent.len(), 1);
    let operations = shell.operations(cx);
    let lane = operations.read_with(cx, |operations, _| {
        operations.request(sent[0]).unwrap().lane
    });
    assert_eq!(lane, Lane::User);
    gate.wait_reached();
    // While it runs the dialog says so, and a second press is not offered.
    assert!(shell.present("lifecycle-checking", cx));
    assert!(!shell.present(CHECK_NOW, cx));
    assert_eq!(polls(cx).len(), 1);
    gate.release();
    shell.until_ended(sent[0], cx).await;
    until_present(&shell, CHECK_NOW, cx).await;
    assert!(!shell.present("lifecycle-checking", cx));
    assert_eq!(polls(cx).len(), 1, "it is not sent again by itself");
}
