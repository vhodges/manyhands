//! P23, S1: each button a failed or stopped request's row can carry, run by
//! keyboard from the popover. The rows are P18's: fabricated through the
//! request model (`fabricate`, no worker), or, for the buttons that send the
//! same request again or check a host, real requests through the worker
//! whose library answer is scripted (`real_share`; no SSH). Each press
//! asserts what the action did, as P18 does, that the popover closed, and,
//! where a dialog or sheet opens, that closing it gives focus back to the
//! footer item.

use gpui_kit::TestAppContext;
use manyhands::results::{Effects, RecoveryAction, RecoveryActionKind, ResultCode};

use super::{
    actions::{dialog_and_back, press_from_popover},
    support::{Spot, spot},
};
use crate::desktop::{
    model::{
        activity::{
            copy::{
                OPEN_LIFECYCLE, OPEN_SHARING_AND_KEYS, REPUBLISH, REVIEW_CHANGES, SET_UP_SHARING,
                SHOW_IT, offer_label,
            },
            rows::{ActivityAction, RowKey},
        },
        publication::SyncOffer,
        stamp::SettingsPage,
    },
    services::{JobKind, requests::Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity::tests::cases::{
            OPERATION, TICKET, answered, dialog_open, fabricate, item_sync, sent, settings,
        },
        activity_support::{label_of, refresh, request},
        harness::Shell,
        operations::{
            outcomes::tests::{answer, host_problem, open, real_share},
            republish::tests::{finished_deleted_share, republish_preview, ticket_shell},
        },
        settings::Shown,
    },
};

/// P23, S1, fabricated rows: Open Sharing & keys, Set up sharing, Show it,
/// a recovery that sends a request and one that
/// confirms, Republish…, Open… and Review changes.
#[gpui_kit::test]
async fn popover_actions_requests(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    shares(cx);
    recovery(cx);
    dialogs(cx).await;
}

/// P23, S1, real requests: Unlock… and Retry send the same request again,
/// Share sends a new one,
/// Share, on a failure the same request cannot get past once its record is
/// finished (R-17): a new request of the same share.
async fn share(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let id = real_share(
        &shell,
        &root,
        answer(ResultCode::PushRejected, Effects::not_requested()),
        cx,
    )
    .await;
    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    press_from_popover(
        &shell,
        &RowKey::Request(id),
        offer_label(SyncOffer::Share),
        cx,
    );
    held.wait_reached();
    let mains = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    assert_eq!(mains.len(), 2, "Share sent a new request");
    held.release();
    shell.until_ended(mains[0], cx).await;
}

/// Check host checks the host the answer names, Set identity opens its page.
#[gpui_kit::test]
async fn popover_actions_real_requests(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    again(cx).await;
    share(cx).await;
    routes(cx).await;
}

/// What a share offer's button did.
enum Effect {
    /// A Settings page opened.
    Page(SettingsPage),
    /// Activity's list took focus (Show it).
    Holder,
}

/// S1: a failed share of each kind of answer, whose first offer is its
/// button: what the answer's code makes of it, and what the button does.
fn shares(cx: &mut TestAppContext) {
    let table = [
        (
            ResultCode::NoSelectedKey,
            OPEN_SHARING_AND_KEYS,
            Effect::Page(SettingsPage::Keys),
        ),
        (
            ResultCode::PublicationRemoteRequired,
            SET_UP_SHARING,
            Effect::Page(SettingsPage::Remotes),
        ),
        (ResultCode::Busy, SHOW_IT, Effect::Holder),
    ];
    for (code, label, effect) in table {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        fixture.commit_ticket(&root, TICKET, "Ticket", "");
        fixture.refresh(&root);
        let shell = Shell::open(fixture, cx);
        let id = fabricate(&shell, 1, item_sync(&root), answered("item sync", code), cx);
        let row = RowKey::Request(id);
        press_from_popover(&shell, &row, label, cx);
        match effect {
            Effect::Page(page) => {
                assert_eq!(settings(&shell, cx), Shown::Settings(page), "{label}");
            }
            Effect::Holder => {
                shell.frame(cx, |_, _| ());
                assert!(
                    crate::desktop::view::activity_support::list_focused(&shell, cx),
                    "{label}: Activity's list took focus: the action placed it"
                );
            }
        }
    }
}

/// The buttons that open a dialog, each in a window of its own: Open…,
/// Republish…, a recovery that confirms and a share's Review changes. The
/// dialog's close gives focus back to the footer item.
async fn dialogs(cx: &mut TestAppContext) {
    use manyhands::results::{RecoveryAction, RecoveryActionKind};

    use crate::desktop::model::activity::rows::ActivityAction;
    let (shell, root) = ticket_shell(cx);
    let deleted = RowKey::Request(finished_deleted_share(&shell, &root, cx).await);
    let close = RowKey::Request(fabricate(
        &shell,
        1,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        answered("ticket close", ResultCode::PushRejected),
        cx,
    ));
    let prepare = RowKey::Request(fabricate(
        &shell,
        2,
        Intent::OperationAbandon {
            root: root.clone(),
            operation_id: manyhands::repository::OperationId::parse(OPERATION).unwrap(),
        },
        answered("operation abandon", ResultCode::ExternalChange).with_recovery(vec![
            RecoveryAction::new(RecoveryActionKind::RequestPrepare, []),
        ]),
        cx,
    ));
    refresh(&shell, cx);
    let prepare_again = crate::desktop::view::activity_support::label_of(
        &shell,
        &prepare,
        &ActivityAction::Recovery(
            match &prepare {
                RowKey::Request(id) => *id,
                _ => unreachable!(),
            },
            0,
        ),
        cx,
    );
    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    dialog_and_back(&shell, &deleted, REPUBLISH, cx).await;
    dialog_and_back(&shell, &close, OPEN_LIFECYCLE, cx).await;
    dialog_and_back(&shell, &prepare, &prepare_again, cx).await;

    // Review changes, on a share stopped on a conflict.
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Mine");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let share = RowKey::Request(crate::desktop::view::conflict::entry_tests::fabricate(
        &shell,
        6,
        item_sync(&root),
        crate::desktop::view::conflict::entry_tests::share_conflict(),
        cx,
    ));
    press_from_popover(&shell, &share, REVIEW_CHANGES, cx);
    shell
        .until_drawn(cx, "the sheet", |shell, cx| {
            shell.present("conflict-sheet", cx)
        })
        .await;
    // Closing the sheet gives focus back to the footer item.
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("conflict-sheet", cx), "the sheet closed");
    assert_eq!(spot(&shell, cx), Spot::Item, "focus is on the item");
}

/// A recovery that is the row's button sends the request it names (P18's
/// `Recovery(_, 0)`).
fn recovery(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Ticket", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let failed = fabricate(
        &shell,
        2,
        Intent::IndexRefresh { root: root.clone() },
        answered("index refresh", ResultCode::IndexUnavailable).with_recovery(vec![
            RecoveryAction::new(
                RecoveryActionKind::IndexRefresh,
                [("root", crate::desktop::view::harness::key(&root).into())],
            ),
        ]),
        cx,
    );
    let row = RowKey::Request(failed);
    refresh(&shell, cx);
    let first = label_of(&shell, &row, &ActivityAction::Recovery(failed, 0), cx);
    let refreshes = |shell: &Shell, cx: &mut TestAppContext| {
        sent(shell, cx, |intent| {
            matches!(intent, Intent::IndexRefresh { .. })
        })
        .len()
    };
    let before = refreshes(&shell, cx);
    press_from_popover(&shell, &row, &first, cx);
    assert_eq!(
        refreshes(&shell, cx),
        before + 1,
        "the recovery sent a refresh"
    );
    assert!(!dialog_open(&shell, cx), "and asked nothing");
}

/// The same request sent again, through the real worker: its attempt count
/// rises, under the same request ID.
async fn again(cx: &mut TestAppContext) {
    // Unlock… on a share whose key was not unlocked, and Retry on a failure
    // the library calls transient: both leave the record accepted, so the
    // row also offers Stop trying to share, after its button.
    for (code, label) in [
        (ResultCode::UnlockRequired, offer_label(SyncOffer::Unlock)),
        (ResultCode::RemoteUnavailable, offer_label(SyncOffer::Retry)),
    ] {
        let (shell, root) = open(true, cx);
        let id = real_share(&shell, &root, answer(code, Effects::not_requested()), cx).await;
        let first = request(&shell, id, cx);
        let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
        press_from_popover(&shell, &RowKey::Request(id), label, cx);
        held.wait_reached();
        let retried = request(&shell, id, cx);
        assert_eq!(
            retried.request_id, first.request_id,
            "{label}: the same request"
        );
        assert_eq!(retried.attempt, first.attempt + 1, "{label}: sent again");
        let mains = shell.requests(cx, |request| {
            matches!(request.intent, Intent::RepoSync { .. })
        });
        assert_eq!(mains, [id], "{label}: no new request");
        held.release();
        shell.until_ended(id, cx).await;
    }
}

/// Check host checks the host the answer names, on the Hosts page of the
/// share's repository; Set identity opens Settings.
async fn routes(cx: &mut TestAppContext) {
    let (shell, root) = open(true, cx);
    let id = real_share(
        &shell,
        &root,
        host_problem(ResultCode::HostApprovalRequired, true),
        cx,
    )
    .await;
    let held = shell.backend.hold_job(JobKind::ObserveHost);
    press_from_popover(
        &shell,
        &RowKey::Request(id),
        offer_label(SyncOffer::CheckHost),
        cx,
    );
    held.wait_reached();
    assert!(shell.present("settings-hosts", cx), "the Hosts page opened");
    held.release();

    let (shell, root) = open(true, cx);
    let id = real_share(
        &shell,
        &root,
        answer(ResultCode::IdentityRequired, Effects::not_requested()),
        cx,
    )
    .await;
    assert_eq!(settings(&shell, cx), Shown::Tabs);
    press_from_popover(
        &shell,
        &RowKey::Request(id),
        offer_label(SyncOffer::SetIdentity),
        cx,
    );
    assert!(
        matches!(settings(&shell, cx), Shown::Settings(_)),
        "Settings opened"
    );
}
