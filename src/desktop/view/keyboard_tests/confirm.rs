//! D1's confirmation dialog by keyboard: Cancel, reached by Tab and
//! pressed by Enter, cancels in every phase that shows it (Preparing,
//! Ready, Stale and Answered), where Enter used to give the dialog's own
//! default: in Ready that confirmed the command. And the Abandon control
//! the dialog gives the focus back to is pressed by Enter again.

use std::{path::PathBuf, time::Duration};

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::repository::{LeaseKind, RepositoryService};

use super::Walk;
use crate::desktop::{
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::{Fixture, resume_through},
    view::{
        harness::Shell,
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
    /// `secondary-4`, Tab to Abandon and Enter: the confirmation opens.
    fn open_abandon(&self, cx: &mut TestAppContext) {
        self.keys("secondary-4", cx);
        self.tab_to("tab", ("pending-abandon", 0usize), cx);
        self.on(("pending-abandon", 0usize), Region::Operations, cx);
        self.enter(cx);
        assert!(self.shell.present("confirm-dialog", cx));
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
    walk.on(("pending-abandon", 0usize), Region::Operations, cx);
    assert!(walk.shell.present(("pending-row", 0usize), cx));

    // The focus given back is the wrapper's: Enter presses Abandon.
    walk.enter(cx);
    assert!(walk.shell.present("confirm-dialog", cx));
    walk.keys("escape", cx);
    walk.on(("pending-abandon", 0usize), Region::Operations, cx);
    walk.enter(cx);
    assert!(walk.shell.present("confirm-dialog", cx));
    walk.keys("escape", cx);
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
