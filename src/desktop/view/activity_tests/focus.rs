//! Where focus goes when a row opens a dialog (Wave 04 Cycle 03b, C3,
//! decision 23): `a_dialog_from_a_row_returns_focus_to_its_row` (P20). An
//! action that opens a dialog from a row, chosen from the row's button, its
//! menu or Enter, passes the list as the origin: cancelled, the dialog gives
//! focus back to the list, with the row still highlighted.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{OperationAction, OperationId},
    results::{RecoveryAction, RecoveryActionKind, ResultCode},
};

use super::{
    cases::{OPERATION, TICKET, answered, dialog_open, fabricate, label_of},
    list_support::{OP_B, save, waiting_close},
    pending_cases::{PENDING_ROOT, listing, read_pending},
};
use crate::desktop::{
    model::activity::{
        copy::{OPEN_LIFECYCLE, REPUBLISH},
        rows::{ActivityAction, RowKey},
    },
    services::{JobKind, requests::Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{highlighted, index_of, list_focus, pending_key, refresh, to_row},
        harness::{Shell, key},
        operations::republish::tests::{finished_deleted_share, republish_preview, ticket_shell},
        repositories::tests::support::{choose, press_and_hold},
    },
};

/// How a person chooses what a row offers.
#[derive(Clone, Copy, Debug)]
enum By {
    /// A click on the row's drawn button.
    Button,
    /// `shift-f10`, then the menu's item.
    Menu,
    /// Enter, with the list focused on the row.
    Enter,
}

/// Chooses `label` on the row `key` by `how`, waits for a dialog, cancels
/// it with Escape and says what focus and the highlight were in that same
/// update.
async fn dialog_and_back(
    shell: &Shell,
    key: &RowKey,
    label: &str,
    how: By,
    cx: &mut TestAppContext,
) {
    to_row(shell, key, cx);
    match how {
        By::Button => {
            let at = index_of(shell, key, cx);
            shell.click(("activity-button", at), cx);
        }
        By::Menu => {
            shell.keys("shift-f10", cx);
            choose(shell, label, cx);
        }
        By::Enter => shell.keys("enter", cx),
    }
    shell.until_drawn(cx, "the dialog", dialog_open).await;
    let list = list_focus(shell, cx);
    assert!(
        press_and_hold(shell, "escape", &list, cx),
        "{label} by {how:?}: Escape gave focus back to the list"
    );
    assert!(!dialog_open(shell, cx), "{label} by {how:?}: it closed");
    assert_eq!(
        highlighted(shell, cx).as_ref(),
        Some(key),
        "{label} by {how:?}: the same row is highlighted"
    );
}

/// P20: Republish…, a recovery that confirms (Prepare again), Open… and a
/// lifecycle's Resume, each from the row's button, its menu and Enter;
/// Abandon…, from the menu; each cancelled: focus is back in the list, the
/// same row highlighted.
#[gpui_kit::test]
async fn a_dialog_from_a_row_returns_focus_to_its_row(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = ticket_shell(cx);

    // Republish…, on a share whose shared branch was deleted.
    let deleted = RowKey::Request(finished_deleted_share(&shell, &root, cx).await);
    // Open…, on a close that stopped.
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
    // A recovery that confirms: Prepare again, on an abandon the library
    // asked to be prepared again.
    let prepare = RowKey::Request(fabricate(
        &shell,
        2,
        Intent::OperationAbandon {
            root: root.clone(),
            operation_id: OperationId::parse(OPERATION).unwrap(),
        },
        answered("operation abandon", ResultCode::ExternalChange).with_recovery(vec![
            RecoveryAction::new(RecoveryActionKind::RequestPrepare, []),
        ]),
        cx,
    ));
    refresh(&shell, cx);
    let prepare_again = label_of(
        &shell,
        &prepare,
        |action| matches!(action, ActivityAction::Recovery(_, 0)),
        cx,
    );
    for how in [By::Button, By::Menu, By::Enter] {
        shell
            .backend
            .answer_next_prepare("item republish", republish_preview(&shell, &root));
        dialog_and_back(&shell, &deleted, REPUBLISH, how, cx).await;
        dialog_and_back(&shell, &close, OPEN_LIFECYCLE, how, cx).await;
        dialog_and_back(&shell, &prepare, &prepare_again, how, cx).await;
    }

    // A lifecycle's Resume, which also resumes its operation: each way in a
    // window of its own, since the resume it sends is running while the
    // lifecycle's row says "Still running…".
    for how in [By::Button, By::Menu, By::Enter] {
        let (shell, _) = ticket_shell(cx);
        read_pending(
            &shell,
            PENDING_ROOT,
            listing(vec![waiting_close(OPERATION, TICKET), save(OP_B)]),
            cx,
        );
        let row = pending_key(&shell, PENDING_ROOT, OperationAction::Close.as_str(), cx);
        // The resume stays in the library until the dialog is back: its end
        // reads the pending list again, which takes the row away.
        let held = shell.backend.hold_job(JobKind::OperationResume);
        dialog_and_back(&shell, &row, "Resume", how, cx).await;
        held.release();
    }

    // Abandon…, from the menu only: it is never a row's button. A real
    // pending save, so the dialog's read of the pending list finds it.
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_ticket(&a, TICKET, "Stranded", "open");
    fixture.strand_ticket_save(&a, TICKET, "Edited");
    let shell = Shell::open(fixture, cx);
    let row = pending_key(&shell, &key(&a), OperationAction::SaveTicket.as_str(), cx);
    dialog_and_back(&shell, &row, "Abandon", By::Menu, cx).await;
}
