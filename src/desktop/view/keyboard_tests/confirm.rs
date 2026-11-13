//! D1's confirmation dialog by keyboard: Cancel, reached by Tab and
//! pressed by Enter, cancels in every phase that shows it (Preparing,
//! Ready, Stale and Answered), where Enter used to give the dialog's own
//! default: in Ready that confirmed the command. And the list the dialog
//! gives the focus back to opens it again from the row's menu.

use std::{path::PathBuf, time::Duration};

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::repository::{LeaseKind, RepositoryService};

use super::Walk;
use crate::desktop::{
    model::activity::rows::RowKey,
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::{Fixture, resume_through},
    view::{
        activity_support::{highlighted, list_focused, pending_key, row as activity_row, to_row},
        harness::{Shell, key},
        operations::ABANDON,
        repositories::tests::support::highlight,
        workspace::{REGIONS, Region},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";

/// The Kit button of the dialog's Cancel (or Close).
fn cancel() -> ElementId {
    ("confirm-cancel", 0usize).into()
}

/// A ticket save left pending in `a`, the window open over it.
fn stranded(cx: &mut TestAppContext) -> (Walk, PathBuf, String) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_ticket(&a, TICKET, "Stranded", "open");
    let operation = fixture.strand_ticket_save(&a, TICKET, "Edited");
    (
        Walk::over(Shell::open(fixture, cx), a.clone()),
        a,
        operation,
    )
}

fn abandons(walk: &Walk, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationAbandon { .. })
    })
}

impl Walk {
    /// The pending save's row.
    fn row(&self, cx: &mut TestAppContext) -> RowKey {
        pending_key(&self.shell, &key(&self.a), "save_ticket", cx)
    }

    /// With the list focused on the pending row, `menu_key` opens its menu,
    /// the arrows reach Abandon and Enter chooses it: the confirmation
    /// opens.
    fn choose_abandon(&self, menu_key: &str, cx: &mut TestAppContext) {
        self.keys(menu_key, cx);
        highlight(&mut self.shell.visual(cx), ABANDON);
        self.keys("enter", cx);
        assert!(self.shell.present("confirm-dialog", cx));
    }

    /// `secondary-4`, the arrows to the pending row, `shift-f10`, the
    /// arrows to Abandon and Enter: the confirmation opens.
    fn open_abandon(&self, cx: &mut TestAppContext) {
        let row = self.row(cx);
        to_row(&self.shell, &row, cx);
        self.choose_abandon("shift-f10", cx);
    }

    /// Tab, in the open dialog, to its Cancel, which then holds focus on
    /// screen while no region does; Enter presses it.
    fn cancel_by_tab_and_enter(&self, cx: &mut TestAppContext) {
        self.tab_to("tab", cancel(), cx);
        self.shell.frame(cx, |window, _| {
            for region in REGIONS {
                assert_ne!(window.find(region.id()).focused(), Some(true));
            }
            let element = window.find(cancel());
            assert_eq!(element.focused(), Some(true), "Cancel holds focus");
            assert!(element.visible(), "Cancel is on screen");
        });
        self.enter(cx);
        assert!(
            !self.shell.present("confirm-dialog", cx),
            "Cancel closed it"
        );
    }
}

/// Ready: Tab to Cancel and Enter submits nothing and gives the focus
/// back to Abandon, which Enter presses again.
#[gpui_kit::test]
fn cancel_by_tab_and_enter_submits_nothing_and_abandon_opens_again(cx: &mut TestAppContext) {
    let (walk, _, _) = stranded(cx);
    walk.open_abandon(cx);
    assert!(walk.shell.present("confirm-ok", cx), "the preview is ready");
    walk.cancel_by_tab_and_enter(cx);
    assert!(walk.shell.requests(cx, |_| true).is_empty());
    // The focus given back is the list's, the row still highlighted.
    let row = walk.row(cx);
    walk.region(Region::Operations, cx);
    assert!(list_focused(&walk.shell, cx));
    assert_eq!(highlighted(&walk.shell, cx), Some(row.clone()));
    assert!(activity_row(&walk.shell, &row, cx).is_some());

    // Both menu keys open the row's menu again, after Cancel and after
    // Escape, and Abandon opens the confirmation each time.
    for menu_key in ["shift-f10", "menu", "shift-f10", "menu"] {
        walk.choose_abandon(menu_key, cx);
        walk.keys("escape", cx);
        walk.region(Region::Operations, cx);
        assert!(list_focused(&walk.shell, cx), "{menu_key}");
    }
    assert!(walk.shell.requests(cx, |_| true).is_empty());
}

/// Preparing, with the preview held: Tab to Cancel and Enter closes the
/// dialog, where Enter used to do nothing.
#[gpui_kit::test]
fn cancel_by_tab_and_enter_closes_a_preparing_dialog(cx: &mut TestAppContext) {
    let (walk, _, _) = stranded(cx);
    let held = walk.shell.backend.hold_read(ReadKind::Prepare);
    walk.open_abandon(cx);
    assert!(walk.shell.present("confirm-state", cx), "preparing");
    assert!(!walk.shell.present("confirm-command-name", cx));
    walk.cancel_by_tab_and_enter(cx);
    drop(held);
    walk.shell.run(cx);
    assert!(!walk.shell.present("confirm-dialog", cx));
    assert!(walk.shell.requests(cx, |_| true).is_empty());
}

/// Stale: the operation finished elsewhere after the preview; Confirm
/// (Enter, the default) is answered stale. Tab to Cancel and Enter closes
/// the dialog with no second request.
#[gpui_kit::test]
async fn cancel_by_tab_and_enter_closes_a_stale_dialog(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (walk, a, operation) = stranded(cx);
    walk.open_abandon(cx);
    assert!(walk.shell.present("confirm-ok", cx));
    let other = RepositoryService::open_at(walk.shell.fixture.data_dir()).unwrap();
    resume_through(&other, &a, &operation);

    walk.enter(cx);
    let id = abandons(&walk, cx)[0];
    walk.shell.until_ended(id, cx).await;
    assert!(walk.shell.present("confirm-prepare-again", cx), "stale");
    walk.cancel_by_tab_and_enter(cx);
    assert_eq!(abandons(&walk, cx), [id]);
}

/// Answered: a prepare that stays busy shows Retry and Close. Tab to
/// Close and Enter closes the dialog, where Enter used to do nothing.
#[gpui_kit::test]
fn cancel_by_tab_and_enter_closes_an_answered_dialog(cx: &mut TestAppContext) {
    let (walk, a, _) = stranded(cx);
    let lease = RepositoryService::hold_lease_for_testing(
        &a,
        walk.shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    walk.open_abandon(cx);
    cx.executor().advance_clock(Duration::from_secs(1));
    walk.shell.run(cx);
    assert!(walk.shell.present("confirm-retry", cx), "answered busy");
    walk.cancel_by_tab_and_enter(cx);
    assert!(walk.shell.requests(cx, |_| true).is_empty());
    drop(lease);
}
