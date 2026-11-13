//! The fixtures and helpers of the list tests (`list.rs`): the window with a
//! row of each kind Enter and a click treat differently, and the steps the
//! tests share.

use gpui_kit::{
    App, Entity, InputEvent, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, Window,
};
use manyhands::{
    repository::{
        AuthoringKind, OperationAction, OperationDto, ProgressEvent, ProgressStage, RequestId,
    },
    results::{Effects, ResultCode, Scope},
};

use super::{
    cases::{OPERATION, TICKET, answered, fabricate, sent},
    pending_cases::{PENDING_ROOT, listing, read_pending, waiting},
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityRow, RowKey},
        operations::Operations,
    },
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::harness::Shell,
};

/// The four pending operations: each test step presses its own, since a
/// press sends a request that changes its row.
pub(super) const OP_A: &str = OPERATION;
pub(super) const OP_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
pub(super) const OP_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
pub(super) const OP_D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";

pub(super) fn save(operation: &str) -> OperationDto {
    waiting(OperationAction::SaveTicket, operation, None)
}

pub(super) fn pending_key(operation: &str) -> RowKey {
    RowKey::Pending {
        root: PENDING_ROOT.to_owned(),
        operation: operation.to_owned(),
    }
}

/// The window the list tests share: repository `a` with the ticket
/// `TICKET`, and rows of each kind Enter and a click treat differently:
///
/// - four pending operations of `/fixture/a`, each offering Resume (a
///   button);
/// - a finished refresh, which has neither a button nor an item;
/// - a finished comment, which has an item and no button;
/// - a refresh that was accepted and cannot be stopped: a disabled button.
pub(super) struct Rows {
    pub(super) shell: Shell,
    pub(super) pending: [RowKey; 4],
    pub(super) plain: RowKey,
    pub(super) item: RowKey,
    pub(super) disabled: RowKey,
}

pub(super) fn rows(cx: &mut TestAppContext) -> Rows {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Local ticket", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let refresh = || Intent::IndexRefresh { root: root.clone() };
    let plain = fabricate(
        &shell,
        1,
        refresh(),
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    let item = fabricate(
        &shell,
        2,
        Intent::CommentAdd {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
            comment_id: manyhands::canonical::ItemId::generate(),
            parent_id: None,
            body: crate::desktop::services::requests::CommentBody::new("A comment.".to_owned()),
        },
        answered("comment add", ResultCode::Ok),
        cx,
    );
    let disabled = running_accepted(&shell, 3, refresh(), cx);
    read_pending(
        &shell,
        PENDING_ROOT,
        listing([OP_A, OP_B, OP_C, OP_D].map(save).to_vec()),
        cx,
    );
    Rows {
        shell,
        pending: [OP_A, OP_B, OP_C, OP_D].map(pending_key),
        plain: RowKey::Request(plain),
        item: RowKey::Request(item),
        disabled: RowKey::Request(disabled),
    }
}

/// A request `n` of `intent` that has reached a stage, so it is accepted:
/// a refresh offers no Stop, and reads a disabled "Finishing".
pub(super) fn running_accepted(
    shell: &Shell,
    n: u64,
    intent: Intent,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(5_000_000 + n);
    let request = DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    let event = ProgressEvent {
        request_id: request.request_id.to_string(),
        operation_id: None,
        command: "index refresh".to_owned(),
        scope: Scope::default(),
        stage: ProgressStage::Writing,
        effects: Effects::not_requested(),
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        operations.apply(WorkerEvent::Started(id));
        operations.apply(WorkerEvent::Progress(id, event));
        cx.notify();
    });
    shell.run(cx);
    id
}

/// Draws until the pane shows what the log holds.
pub(super) fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    let (log, pane) = shell.workspace.read_with(cx, |workspace, _| {
        (
            workspace.activity().rows().to_vec(),
            workspace.activity_pane().rows().to_vec(),
        )
    });
    assert_eq!(pane, log, "the pane shows the log's rows");
}

pub(super) fn pressed(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_pressed())
}

pub(super) fn resumes(shell: &Shell, operation: &str, cx: &mut TestAppContext) -> usize {
    sent(shell, cx, |intent| {
        matches!(intent, Intent::OperationResume { operation_id, .. }
            if operation_id.to_string() == operation)
    })
    .len()
}

/// Presses and releases `key` inside one update, as a keyboard does.
pub(super) fn press_in(window: &mut Window, key: &str, cx: &mut App) {
    let keystroke = Keystroke::parse(key).unwrap();
    let down = KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    };
    window.dispatch_event(down.to_platform_input(), cx);
    window.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
}

/// The pending list read again with only `listed`.
pub(super) fn relist(shell: &Shell, listed: &[&str], cx: &mut TestAppContext) {
    read_pending(
        shell,
        PENDING_ROOT,
        listing(listed.iter().map(|operation| save(operation)).collect()),
        cx,
    );
}

/// The log's row for `key` takes a newer revision while the pane still has
/// the one it drew: the pending operation has nothing to do, so its
/// Resume goes. Run inside the update that then presses, so no frame
/// shows the new row before the press. Returns the revision the pane
/// drew and the log's.
pub(super) fn change_row(
    shell: &Shell,
    operations: &Entity<Operations>,
    operation: &str,
    cx: &mut App,
) -> (Option<u64>, Option<u64>) {
    let key = pending_key(operation);
    operations.update(cx, |operations, cx| {
        let stamp = operations.load_pending(PENDING_ROOT.to_owned());
        let mut next = save(operation);
        next.next_action = None;
        assert!(operations.apply_pending(stamp, listing(vec![next])));
        cx.notify();
    });
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.refresh_activity_now(cx));
    shell.workspace.read_with(cx, |workspace, _| {
        let revision = |rows: &[ActivityRow]| {
            rows.iter()
                .find(|row| row.row.key == key)
                .map(|row| row.revision)
        };
        (
            revision(workspace.activity_pane().rows()),
            revision(workspace.activity().rows()),
        )
    })
}
