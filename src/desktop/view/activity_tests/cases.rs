//! The fixtures of `each_action_runs_its_handler` (P18): one small window
//! for each family of rows, and for each `ActivityAction` the one row that
//! offers it. Each action is chosen through the seam (`act`) and what it
//! sent is read from the request model, the recording backend, or the
//! dialog, sheet or page it opened. Nothing here draws a control.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, component::WindowExt};
use manyhands::{
    canonical::CONFIG_PATH,
    repository::{AuthoringKind, MutationDataDto, RequestId},
    results::{Effects, Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use crate::desktop::{
    model::{
        activity::{
            copy::{CANCEL, DISMISS, OPEN_LIFECYCLE, REVIEW_CHANGES, STOP, STOPPING, offer_label},
            rows::{ActivityAction, RowKey},
        },
        publication::SyncOffer,
        stamp::SettingsPage,
    },
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{act, offered, request, row},
        harness::{Shell, WAIT, key},
        lifecycle::OpenedLifecycle,
        settings::Shown,
    },
};

pub(in crate::desktop::view) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA0";
pub(in crate::desktop::view) const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 5_000_000;

pub(in crate::desktop::view) fn answered(
    command: &str,
    code: ResultCode,
) -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        command,
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        Effects::not_requested(),
        false,
    )
}

/// The request `n` of `intent`, fabricated through the model, finished with
/// `envelope`.
pub(in crate::desktop::view) fn fabricate(
    shell: &Shell,
    n: u64,
    intent: Intent,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(FABRICATED + n);
    let request = DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        operations.apply(WorkerEvent::Started(id));
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
        cx.notify();
    });
    shell.run(cx);
    id
}

/// The label of the one offer of the row `at` whose action `pick` picks.
pub(in crate::desktop::view) fn label_of(
    shell: &Shell,
    at: &RowKey,
    pick: impl Fn(&ActivityAction) -> bool,
    cx: &mut TestAppContext,
) -> String {
    let row = row(shell, at, cx).unwrap_or_else(|| panic!("no row {at:?}"));
    let picked: Vec<_> = row
        .row
        .menu
        .iter()
        .filter(|offer| pick(&offer.action))
        .collect();
    assert_eq!(picked.len(), 1, "one offer of {at:?}: {:?}", row.row.menu);
    picked[0].label.clone()
}

pub(in crate::desktop::view) fn sent(
    shell: &Shell,
    cx: &mut TestAppContext,
    pick: impl Fn(&Intent) -> bool,
) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| pick(&request.intent))
}

pub(in crate::desktop::view) fn dialog_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .visual(cx)
        .update(|window, cx| window.has_active_dialog(cx))
}

pub(in crate::desktop::view) fn settings(shell: &Shell, cx: &mut TestAppContext) -> Shown {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

fn finished_state(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> bool {
    matches!(request(shell, id, cx).state, RequestState::Finished(..))
}

// --- Requests ---

/// Repository `a`, published over SSH with its selection committed, in an
/// open window.
fn remote_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let config = std::fs::read(root.join(CONFIG_PATH)).unwrap();
    fixture.commit_file(&root, CONFIG_PATH, &config);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// Cancel, Stop, Open, Offer, Recovery and Dismiss of a request's row.
pub(in crate::desktop::view) async fn requests(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let refresh = || Intent::IndexRefresh { root: root.clone() };

    // Cancel: a queued request is removed.
    let gate = shell.backend.hold_job(JobKind::IndexRefresh);
    let first = worker.submit(refresh(), Lane::User, None, None);
    gate.wait_reached();
    let queued = worker.submit(refresh(), Lane::User, None, None);
    shell.run(cx);
    assert!(matches!(
        request(&shell, queued, cx).state,
        RequestState::Queued
    ));
    act(&shell, &RowKey::Request(queued), CANCEL, cx);
    assert!(matches!(
        request(&shell, queued, cx).state,
        RequestState::Removed
    ));
    gate.release();
    shell.until_ended(first, cx).await;

    // Stop: an accepted share, held after its acceptance, is signalled.
    let (reached, release) = hold_after_acceptance(&shell);
    let share = worker.submit(
        Intent::RepoSync { root: root.clone() },
        Lane::User,
        None,
        None,
    );
    reached.recv_timeout(WAIT).expect("the share was accepted");
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(share).map(|request| &request.state),
                Some(RequestState::Running { accepted: true, .. })
            )
        })
        .await;
    act(&shell, &RowKey::Request(share), STOP, cx);
    let job = worker.running_job().expect("the share runs");
    assert_eq!(job.id, share);
    assert!(job.cancelled, "Stop cancelled its token");
    let stopping =
        offered(&shell, &RowKey::Request(share), STOPPING, cx).expect("Stop now reads Stopping");
    assert!(!stopping.enabled);
    drop(release);
    shell.until_ended(share, cx).await;

    // Offer: a stopped share's Share sends a new request of the same share.
    let stopped = fabricate(
        &shell,
        1,
        item_sync(&root),
        answered("item sync", ResultCode::Cancelled),
        cx,
    );
    let before = sent(&shell, cx, |intent| {
        matches!(intent, Intent::ItemSync { .. })
    })
    .len();
    act(
        &shell,
        &RowKey::Request(stopped),
        offer_label(SyncOffer::Share),
        cx,
    );
    assert_eq!(
        sent(&shell, cx, |intent| matches!(
            intent,
            Intent::ItemSync { .. }
        ))
        .len(),
        before + 1,
        "Share sent a new request"
    );
    // A route offer opens its place.
    let keys = fabricate(
        &shell,
        2,
        item_sync(&root),
        answered("item sync", ResultCode::NoSelectedKey),
        cx,
    );
    act(
        &shell,
        &RowKey::Request(keys),
        offer_label(SyncOffer::OpenSharingAndKeys),
        cx,
    );
    assert_eq!(settings(&shell, cx), Shown::Settings(SettingsPage::Keys));

    // Recovery: a failed refresh offers its own recovery actions, in order.
    let mut abandon = RecoveryAction::new(
        RecoveryActionKind::OperationAbandon,
        [("root", key(&root).into())],
    );
    abandon.operation_id = Some(OPERATION.to_owned());
    let failed = fabricate(
        &shell,
        3,
        refresh(),
        answered("index refresh", ResultCode::IndexUnavailable).with_recovery(vec![
            RecoveryAction::new(
                RecoveryActionKind::IndexRefresh,
                [("root", key(&root).into())],
            ),
            abandon,
        ]),
        cx,
    );
    let failed_key = RowKey::Request(failed);
    let (again, confirm) = (
        label_of(
            &shell,
            &failed_key,
            |a| matches!(a, ActivityAction::Recovery(_, 0)),
            cx,
        ),
        label_of(
            &shell,
            &failed_key,
            |a| matches!(a, ActivityAction::Recovery(_, 1)),
            cx,
        ),
    );
    let before = sent(&shell, cx, |intent| {
        matches!(intent, Intent::IndexRefresh { .. })
    })
    .len();
    act(&shell, &failed_key, &again, cx);
    assert_eq!(
        sent(&shell, cx, |intent| matches!(
            intent,
            Intent::IndexRefresh { .. }
        ))
        .len(),
        before + 1,
        "the recovery sent a refresh"
    );
    assert!(!dialog_open(&shell, cx));
    act(&shell, &failed_key, &confirm, cx);
    assert!(dialog_open(&shell, cx), "the recovery confirms first");
    assert!(
        shell.present("confirm-dialog", cx),
        "the confirmation dialog"
    );
    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx));
    assert!(
        sent(&shell, cx, |intent| matches!(
            intent,
            Intent::OperationAbandon { .. }
        ))
        .is_empty(),
        "the confirmation sent nothing"
    );

    // Open: a stopped close's dialog again, on that request.
    let close = fabricate(
        &shell,
        4,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        answered("ticket close", ResultCode::PushRejected),
        cx,
    );
    act(&shell, &RowKey::Request(close), OPEN_LIFECYCLE, cx);
    assert!(dialog_open(&shell, cx));
    let shown = cx.update(|cx| {
        cx.try_global::<OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
    });
    let shown = shown.expect("the lifecycle dialog opened");
    assert_eq!(
        shown.read_with(cx, |dialog, _| dialog.submitted()),
        Some(close)
    );
    shell.keys("escape", cx);

    // Dismiss: a finished request goes.
    assert!(finished_state(&shell, failed, cx));
    act(&shell, &failed_key, DISMISS, cx);
    assert!(matches!(
        request(&shell, failed, cx).state,
        RequestState::Dismissed
    ));
}

pub(in crate::desktop::view) fn item_sync(root: &Path) -> Intent {
    Intent::ItemSync {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    }
}

/// Holds the next request after its acceptance, before its domain call; the
/// receiver hears it is held, and dropping the sender lets it go on.
pub(in crate::desktop::view) fn hold_after_acceptance(
    shell: &Shell,
) -> (std::sync::mpsc::Receiver<()>, std::sync::mpsc::Sender<()>) {
    let (reached_tx, reached) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel::<()>();
    shell
        .fixture
        .service()
        .set_request_hook_for_testing(move || {
            let _ = reached_tx.send(());
            let _ = released.recv_timeout(WAIT);
        });
    (reached, release)
}

/// Review changes, on a share stopped on a conflict, opens the sheet.
pub(in crate::desktop::view) async fn review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Mine");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let share = crate::desktop::view::conflict::entry_tests::fabricate(
        &shell,
        6,
        item_sync(&root),
        crate::desktop::view::conflict::entry_tests::share_conflict(),
        cx,
    );
    assert!(!shell.present("conflict-sheet", cx));
    act(&shell, &RowKey::Request(share), REVIEW_CHANGES, cx);
    shell
        .until_drawn(cx, "the sheet", |shell, cx| {
            shell.present("conflict-sheet", cx)
        })
        .await;
}
