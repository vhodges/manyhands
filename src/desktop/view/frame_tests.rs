//! The common frame's required outcomes (Wave 04 Cycle 05a design P1 and
//! P2): every dialog on the frame keeps its keys and its focus, and a
//! footer that puts its action last is tabbed in the order it is drawn.
//! Each test is a table over design A3's dialogs. Task 1 writes the
//! Confirmation dialog's rows; Task 2 adds the other dialogs', whose
//! set-ups are in `frame_tests/`, and Task 8 Unlock key's.
//!
//! Focus is read after two draws (design B9; Cycle 04's design A6), never
//! inside the update that moved it. A row that does not hold is kept, with
//! what was found instead, and the table fails once at its end, naming
//! every such row. A setup step that does not hold panics where it is.

use gpui_kit::{
    ElementId, FocusHandle, TestAppContext,
    component::{Theme, ThemeMode},
    test::TestWindowExt,
};

use super::theme;
use crate::desktop::{
    model::activity::rows::RowKey,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::Fixture,
    view::{
        activity_support::{list_focus, pending_key, to_row},
        harness::{Shell, key},
        operations::ABANDON,
        repositories::tests::support::highlight,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FF0";

/// How many Tab presses a row may take to reach every button it names.
const TAB_LIMIT: usize = 8;

/// The rows of one test, as they are read. Other modules' tables use it
/// too (P18, P19).
pub(in crate::desktop::view) struct Table {
    name: &'static str,
    held: Vec<String>,
    failed: Vec<String>,
}

impl Table {
    pub(in crate::desktop::view) fn new(name: &'static str) -> Self {
        Self {
            name,
            held: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// The row `row`: `held` says whether `what` is so.
    pub(in crate::desktop::view) fn fact(&mut self, row: &str, held: bool, what: &str) {
        if held {
            self.held.push(row.to_owned());
        } else {
            self.failed.push(format!("{row}: {what} does not hold"));
        }
    }

    /// Fails, naming every row that did not hold, unless none did.
    pub(in crate::desktop::view) fn done(self) {
        eprintln!("{}: rows that held: {:?}", self.name, self.held);
        assert!(
            self.failed.is_empty(),
            "{}: {} of {} rows did not hold:\n  {}",
            self.name,
            self.failed.len(),
            self.failed.len() + self.held.len(),
            self.failed.join("\n  ")
        );
    }
}

/// Draws twice, letting what each draw starts run after it.
fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

/// The innermost observed element focus is in, after two draws.
fn focus_at(shell: &Shell, cx: &mut TestAppContext) -> Option<ElementId> {
    settle(shell, cx);
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| {
                gpui_kit::base::test_support::has_observed_focus(window, element.path())
            })
            .max_by_key(|element| element.path().len())
            .and_then(|element| element.path().last().cloned())
    })
}

/// Whether `handle` holds focus, after two draws.
fn focused(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| handle.is_focused(window))
}

/// The order in which Tab, pressed from where focus is, first puts focus on
/// each of `buttons`, read after a draw each time. A button never reached
/// is missing from it.
fn tab_order(shell: &Shell, buttons: &[ElementId], cx: &mut TestAppContext) -> Vec<ElementId> {
    let mut reached = Vec::new();
    for _ in 0..TAB_LIMIT {
        if reached.len() == buttons.len() {
            break;
        }
        shell.keys("tab", cx);
        let now = shell.frame(cx, |window, _| {
            buttons
                .iter()
                .find(|id| {
                    window
                        .try_find((*id).clone())
                        .and_then(|button| button.focused())
                        == Some(true)
                })
                .cloned()
        });
        if let Some(id) = now
            && !reached.contains(&id)
        {
            reached.push(id);
        }
    }
    reached
}

/// The first dialog layer's focus trap, the innermost element holding the
/// focus the Kit gives a dialog as it opens: on the dialog, on none of its
/// controls.
fn dialog_itself() -> ElementId {
    "dialog-0".into()
}

/// P1: every dialog on the frame keeps the control it focuses on open,
/// what Enter and Escape do, and where focus goes back (design A3, A4;
/// facts 2-4).
#[gpui_kit::test]
async fn every_dialog_keeps_its_keys_and_focus(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P1");
    confirmation_keys(&mut table, cx).await;
    dialogs::asks_keys(&mut table, cx);
    forms::forms_keys(&mut table, cx).await;
    item_dialogs::item_dialog_keys(&mut table, cx).await;
    unlock::unlock_keys(&mut table, cx);
    table.done();
}

/// P2: on the dialogs whose footer puts the action last (R-1, decision 15),
/// Tab from the body reaches Cancel, or Not now, before the action.
#[gpui_kit::test]
fn footer_buttons_tab_in_drawn_order(cx: &mut TestAppContext) {
    let mut table = Table::new("P2");
    confirmation_tabs(&mut table, cx);
    unlock::unlock_tabs(&mut table, cx);
    table.done();
}

/// Design A2 (R-2): the theme stays in the mode it was in, light or dark,
/// once `theme::apply` has set both modes' slots, though each config it
/// applies sets the mode to its own.
#[gpui_kit::test]
fn applying_the_theme_keeps_its_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            Theme::change(mode, None, cx);
            let before = Theme::global(cx).mode;
            theme::apply(cx);
            assert_eq!(Theme::global(cx).mode, before, "started {mode:?}");
        }
    });
}

// The Confirmation dialog, over an operation left pending.

/// A ticket save left pending in `a`, the window open over it, and its row
/// in Activity, whose menu offers Abandon….
fn stranded(cx: &mut TestAppContext) -> (Shell, RowKey) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_ticket(&a, TICKET, "Stranded", "open");
    fixture.strand_ticket_save(&a, TICKET, "Edited");
    let shell = Shell::open(fixture, cx);
    let row = pending_key(&shell, &key(&a), "save_ticket", cx);
    (shell, row)
}

/// Opens the Confirmation dialog by keyboard: the list on the pending row,
/// `shift-f10`, the arrows to Abandon… and Enter. Its preview is shown.
fn open_abandon(shell: &Shell, row: &RowKey, cx: &mut TestAppContext) {
    to_row(shell, row, cx);
    shell.keys("shift-f10", cx);
    highlight(&mut shell.visual(cx), ABANDON);
    shell.keys("enter", cx);
    assert!(
        shell.present("confirm-command-name", cx),
        "the preview is shown"
    );
}

fn abandons(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationAbandon { .. })
    })
}

/// P1's Confirmation row: on open, focus is the dialog's own, on neither
/// button. Escape closes it with nothing sent and focus back on the list it
/// was opened from. Enter confirms once, a second Enter sends nothing more,
/// and once the request is answered the dialog closes with focus back on
/// the list.
async fn confirmation_keys(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, row) = stranded(cx);
    let origin = list_focus(&shell, cx);

    open_abandon(&shell, &row, cx);
    let at = focus_at(&shell, cx);
    table.fact(
        "Confirmation, on open",
        at == Some(dialog_itself()),
        &format!("focus on the dialog itself (it is on {at:?})"),
    );
    shell.keys("escape", cx);
    table.fact(
        "Confirmation, Escape closes it",
        !shell.present("confirm-dialog", cx),
        "the dialog closed",
    );
    table.fact(
        "Confirmation, Escape sends nothing",
        shell.requests(cx, |_| true).is_empty(),
        "no request",
    );
    table.fact(
        "Confirmation, Escape gives focus back",
        focused(&shell, &origin, cx),
        "focus on the list it was opened from",
    );

    open_abandon(&shell, &row, cx);
    let held = shell.backend.hold_job(JobKind::OperationAbandon);
    shell.keys("enter", cx);
    held.wait_reached();
    shell.keys("enter", cx);
    let sent = abandons(&shell, cx);
    table.fact(
        "Confirmation, Enter confirms once",
        sent.len() == 1,
        &format!("one request (there are {})", sent.len()),
    );
    held.release();
    for id in sent {
        shell.until_ended(id, cx).await;
    }
    table.fact(
        "Confirmation, closes once answered",
        !shell.present("confirm-dialog", cx),
        "the dialog closed",
    );
    table.fact(
        "Confirmation, Confirm gives focus back",
        focused(&shell, &origin, cx),
        "focus on the list it was opened from",
    );
}

/// P2's Confirmation row: with the preview shown, Tab from the body reaches
/// Cancel, then Confirm.
fn confirmation_tabs(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, row) = stranded(cx);
    open_abandon(&shell, &row, cx);
    let cancel = ElementId::from(("confirm-cancel", 0usize));
    let ok = ElementId::from(("confirm-ok", 0usize));
    let order = tab_order(&shell, &[cancel.clone(), ok.clone()], cx);
    table.fact(
        "Confirmation",
        order == [cancel, ok],
        &format!("Tab reaching Cancel, then Confirm (it reached {order:?})"),
    );
}

// The other dialogs' set-ups and rows (Task 2), and Unlock key's (Task 8).

#[path = "frame_tests/dialogs.rs"]
mod dialogs;
#[path = "frame_tests/forms.rs"]
mod forms;
#[path = "frame_tests/item_dialogs.rs"]
mod item_dialogs;
#[path = "frame_tests/unlock.rs"]
mod unlock;
