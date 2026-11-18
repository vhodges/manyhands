//! The refusals of `each_action_runs_its_handler` (P18; T11, T12).

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{repository::AuthoringKind, results::ResultCode};

use super::cases::{TICKET, answered, fabricate, hold_after_acceptance};
use crate::desktop::{
    model::activity::{
        copy::{CANCEL, DISMISS, FINISHING},
        rows::{ActivityAction, RowKey},
    },
    services::{
        requests::{Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{act_refused, offered, request, row},
        harness::{Shell, WAIT},
    },
};

/// T11: a choice made against an older revision of its row runs nothing,
/// though the row still offers the same enabled offer. T12: an offer that is
/// disabled, or gone, is refused. Each is checked against what would happen
/// if it ran.
pub(super) async fn refused(cx: &mut TestAppContext) {
    use crate::desktop::keys::ActivityMenuChoice;

    // T11: the request finishes, is sent again and finishes again; the
    // choice made against its first answer is stale.
    let shell = Shell::open(Fixture::new(), cx);
    let root = Path::new("/fixture/a");
    let id = fabricate(
        &shell,
        7,
        Intent::IndexRefresh {
            root: root.to_owned(),
        },
        answered("index refresh", ResultCode::IndexUnavailable),
        cx,
    );
    let at = RowKey::Request(id);
    let old = row(&shell, &at, cx).expect("its row").revision;
    let mut again = request(&shell, id, cx);
    again.attempt = 2;
    again.state = RequestState::Queued;
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(again));
        operations.apply(WorkerEvent::Started(id));
        operations.apply(WorkerEvent::Finished(
            id,
            Box::new(answered("index refresh", ResultCode::Ok)),
        ));
        cx.notify();
    });
    shell.run(cx);
    let now = shell
        .workspace
        .update(cx, |workspace, cx| {
            workspace.refresh_activity_now(cx);
            workspace.activity().rows().to_vec()
        })
        .into_iter()
        .find(|row| row.row.key == at)
        .expect("its row");
    assert_ne!(now.revision, old, "the row took a new revision");
    assert!(
        offered(&shell, &at, DISMISS, cx).is_some_and(|offer| offer.enabled),
        "it still offers Dismiss, enabled"
    );
    let stale = ActivityMenuChoice {
        key: at.clone(),
        action: ActivityAction::Dismiss(at.clone()),
        shown: DISMISS.to_owned(),
        revision: old,
    };
    choose_directly(&shell, stale, cx);
    assert!(
        matches!(request(&shell, id, cx).state, RequestState::Finished(..)),
        "the stale choice dismissed nothing"
    );
    let current = ActivityMenuChoice {
        key: at.clone(),
        action: ActivityAction::Dismiss(at.clone()),
        shown: DISMISS.to_owned(),
        revision: now.revision,
    };
    choose_directly(&shell, current, cx);
    assert!(
        matches!(request(&shell, id, cx).state, RequestState::Dismissed),
        "the current revision runs"
    );

    // T12: an accepted post with no remote has no safe point to stop at: its
    // Stop reads Finishing, disabled. Choosing it does not stop it.
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Local", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let (reached, release) = hold_after_acceptance(&shell);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let post = worker.submit(
        Intent::CommentAdd {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
            comment_id: manyhands::canonical::ItemId::generate(),
            parent_id: None,
            body: crate::desktop::services::requests::CommentBody::new("A comment.".to_owned()),
        },
        Lane::User,
        None,
        None,
    );
    reached.recv_timeout(WAIT).expect("the post was accepted");
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(post).map(|request| &request.state),
                Some(RequestState::Running { accepted: true, .. })
            )
        })
        .await;
    let at = RowKey::Request(post);
    let finishing = offered(&shell, &at, FINISHING, cx).expect("Finishing is offered");
    assert!(!finishing.enabled);
    act_refused(&shell, &at, FINISHING, cx);
    assert!(
        !worker.running_job().expect("it runs").cancelled,
        "the disabled Stop did not signal it"
    );
    // A Cancel it never offered is absent.
    act_refused(&shell, &at, CANCEL, cx);
    drop(release);
    shell.until_ended(post, cx).await;
}

/// Calls `choose_activity` with `choice` as a menu's handler would, with no
/// re-check of ours first.
fn choose_directly(
    shell: &Shell,
    choice: crate::desktop::keys::ActivityMenuChoice,
    cx: &mut TestAppContext,
) {
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            let origin =
                workspace.region_focus(crate::desktop::view::workspace::Region::Operations);
            workspace.choose_activity(&choice, origin, window, cx)
        })
    });
    shell.run(cx);
}
