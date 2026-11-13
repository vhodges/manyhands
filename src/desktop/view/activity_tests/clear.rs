//! "Clear finished" (Wave 04 Cycle 03b, C1, L8): the ⋯ menu's one item takes
//! the clearable rows and nothing else (`clear_finished_keeps_failures_and_
//! states`, P19; R-47).

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ContextRetirementState as Retirement, MutationDataDto,
        RetirementRemoval as Removal,
    },
    results::{Effects, Envelope, PublicationEffect, ResultCode, Scope},
};

use super::{
    cases::{answered, fabricate},
    list_support::*,
    pending_cases::{PENDING_ROOT, listing, read_pending},
    toolbar::more_open,
};
use crate::desktop::{
    model::{
        activity::{
            copy::{CLEAR_FINISHED, GIVE_UP},
            rows::RowKey,
        },
        polling::{Answer, TransportFailure},
    },
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        resends::AfterRepublish,
        worker::{RecordState, WorkerEvent},
    },
    test_support::retire::answer as retirement,
    view::{
        activity_support::{offered, refresh, rows as log_rows},
        harness::Shell,
        repositories::tests::support::{choose, labels},
        settings::polling_support::Rig,
        workspace::ToolbarButton,
    },
};

/// A request `n` of `intent` that ends with `envelope` after the record of
/// its share stood at `record`, as the worker's drain notes it.
fn finished_after(
    shell: &Shell,
    n: u64,
    intent: Intent,
    record: RecordState,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(5_000_000 + n);
    let request = DesktopRequest {
        id,
        request_id: manyhands::repository::RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        operations.apply(WorkerEvent::Started(id));
        operations.note_share_attempt(id, record, false);
        operations.record_stood(id, record);
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
        cx.notify();
    });
    shell.run(cx);
    id
}

/// A share that published.
fn published() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "repo sync",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            publication: PublicationEffect::Published,
            ..Effects::not_requested()
        },
        false,
    )
}

/// P19: "Clear finished" takes the success, the settled failure that still
/// offers Retry, the two S8 events and the cancelled close, and nothing
/// else: the stopped share whose record is Accepted (it offers Give up), the
/// stopped republish a lifecycle waits on, a failure, an attention row and a
/// state stay (T9; R-47). Its item is disabled when no row is clearable,
/// whatever becomes clearable while the menu is open.
#[gpui_kit::test]
async fn clear_finished_keeps_failures_and_states(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    let shell = &rig.shell;
    let root = rig.roots[0].clone();
    let item = ItemId::generate();
    let ticket = |root: &std::path::Path| Intent::ItemSync {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: item.clone(),
    };
    let sync = || Intent::RepoSync { root: root.clone() };

    // Rows the first menu clears.
    let success = fabricate(
        shell,
        1,
        Intent::IndexRefresh { root: root.clone() },
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    let settled = fabricate(
        shell,
        2,
        sync(),
        answered("repo sync", ResultCode::InternalError),
        cx,
    );
    let settling = fabricate(shell, 3, sync(), published(), cx);
    let close = fabricate(
        shell,
        4,
        Intent::TicketClose {
            root: root.clone(),
            id: item.clone(),
        },
        answered("ticket close", ResultCode::Cancelled),
        cx,
    );
    finished_after(
        shell,
        5,
        Intent::ContextRetire {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: crate::desktop::test_support::retire::ITEM.parse().unwrap(),
        },
        RecordState::Finished,
        retirement(Retirement::Retired, ResultCode::Ok, Removal::Observed),
        cx,
    );
    finished_after(
        shell,
        6,
        Intent::ContextRetire {
            root: root.clone(),
            kind: AuthoringKind::Document,
            id: ItemId::generate(),
        },
        RecordState::Finished,
        retirement(
            Retirement::Refused,
            ResultCode::UnlockRequired,
            Removal::None,
        ),
        cx,
    );
    // Rows it keeps.
    let stopped = finished_after(
        shell,
        7,
        ticket(&root),
        RecordState::Accepted,
        answered("item sync", ResultCode::Cancelled),
        cx,
    );
    let republish = fabricate(
        shell,
        8,
        Intent::ItemRepublish {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item.clone(),
        },
        answered("item republish", ResultCode::Cancelled),
        cx,
    );
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    services.resends().after_republish(
        republish,
        AfterRepublish::Retry(close),
        root.clone(),
        item.clone(),
    );
    let failed = fabricate(
        shell,
        9,
        Intent::IndexRebuild { root: root.clone() },
        answered("index rebuild", ResultCode::IndexUnavailable),
        cx,
    );
    read_pending(shell, PENDING_ROOT, listing(vec![save(OP_A)]), cx);
    rig.schedule(0, cx);
    rig.answer(0, Answer::Transport(TransportFailure::Unreachable), cx);
    refresh(shell, cx);

    let request = RowKey::Request;
    let state = RowKey::Repository(rig.key(0));
    let goes = [
        request(success),
        request(settled),
        request(settling),
        request(close),
    ];
    let stays = [
        request(stopped),
        request(republish),
        request(failed),
        pending_key(OP_A),
        state,
    ];
    let present = |shell: &Shell, cx: &mut TestAppContext| -> Vec<RowKey> {
        log_rows(shell, cx)
            .into_iter()
            .map(|row| row.row.key)
            .collect()
    };
    let before = present(shell, cx);
    for key in goes.iter().chain(&stays) {
        assert!(before.contains(key), "{key:?} is a row: {before:?}");
    }
    let (retired_key, note_key) = {
        let events: Vec<RowKey> = before
            .iter()
            .filter(|key| matches!(key, RowKey::Retired(_) | RowKey::RetireNote { .. }))
            .cloned()
            .collect();
        assert_eq!(events.len(), 2, "the two S8 events: {events:?}");
        (events[0].clone(), events[1].clone())
    };
    // What the log calls clearable is exactly what goes.
    let mut expected: Vec<RowKey> = goes.to_vec();
    expected.extend([retired_key, note_key]);
    expected.sort();
    let mut clearable = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity().clearable());
    clearable.sort();
    assert_eq!(clearable, expected, "the clearable rows");
    // The settled failure still offers Retry, and the stopped share Give up.
    let retry = offered(shell, &request(settled), "Retry", cx);
    assert!(retry.is_some_and(|offer| offer.enabled), "Retry stays");
    let give_up = offered(shell, &request(stopped), GIVE_UP, cx);
    assert!(give_up.is_some(), "the stopped share offers Give up");
    let attention_before: Vec<RowKey> = log_rows(shell, cx)
        .into_iter()
        .filter(|row| row.row.attention)
        .map(|row| row.row.key)
        .collect();

    // ⋯ opens with its one item, enabled: a click on the button, a click on
    // the item.
    shell.click(ToolbarButton::ActivityMore.id(), cx);
    assert!(more_open(shell, cx), "⋯ opens");
    assert_eq!(labels(shell, cx), [CLEAR_FINISHED]);
    choose(shell, CLEAR_FINISHED, cx);
    assert!(!more_open(shell, cx), "the choice closes the menu");
    refresh(shell, cx);
    let after = present(shell, cx);
    for key in &expected {
        assert!(!after.contains(key), "{key:?} went");
    }
    for key in &stays {
        assert!(after.contains(key), "{key:?} stays");
    }
    assert_eq!(after.len(), stays.len(), "nothing else: {after:?}");
    for key in &goes {
        let RowKey::Request(id) = key else {
            unreachable!()
        };
        assert!(
            matches!(
                shell
                    .operations(cx)
                    .read_with(cx, |operations, _| operations.request(*id).cloned())
                    .map(|request| request.state),
                Some(RequestState::Dismissed)
            ),
            "{id:?} was dismissed through the request model"
        );
    }
    let attention_after: Vec<RowKey> = log_rows(shell, cx)
        .into_iter()
        .filter(|row| row.row.attention)
        .map(|row| row.row.key)
        .collect();
    assert_eq!(attention_after, attention_before, "attention is untouched");

    // Nothing is clearable now: the item is disabled. A row that becomes
    // clearable while the menu is open is not taken by a click on it.
    shell.click(ToolbarButton::ActivityMore.id(), cx);
    assert!(more_open(shell, cx));
    let late = fabricate(
        shell,
        10,
        Intent::IndexRefresh { root: root.clone() },
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    refresh(shell, cx);
    assert!(
        present(shell, cx).contains(&RowKey::Request(late)),
        "the new row is clearable"
    );
    choose(shell, CLEAR_FINISHED, cx);
    refresh(shell, cx);
    assert!(
        present(shell, cx).contains(&RowKey::Request(late)),
        "the disabled item took nothing"
    );
    // Closed, opened again with a clearable row: it takes it.
    if more_open(shell, cx) {
        shell.keys("escape", cx);
    }
    shell.click(ToolbarButton::ActivityMore.id(), cx);
    choose(shell, CLEAR_FINISHED, cx);
    refresh(shell, cx);
    assert!(
        !present(shell, cx).contains(&RowKey::Request(late)),
        "enabled once a row can go"
    );
}
