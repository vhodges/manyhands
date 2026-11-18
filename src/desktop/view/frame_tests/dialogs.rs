//! P1's rows for the dialogs that are opened without a way in of their own
//! worth walking, or whose own test file opens them directly: Close a tab
//! with a draft, the quit asks, the retire asks and Discard a draft (Wave
//! 04 Cycle 05a design A3, A4). Each is opened over a window whose list
//! holds focus; the row reads the control focused on open, what Escape and
//! Enter choose, and where focus goes back. The shared set-up is here too.
//!
//! What a dialog's choice callback was told is recorded in order, so a
//! row says what Enter and Escape chose without reading a request.

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use gpui_kit::{App, FocusHandle, TestAppContext, Window, test::TestWindowExt};

use super::{Table, dialog_itself, focus_at, focused, settle};
use crate::desktop::{
    model::{navigation::Pane, polling::TargetKind},
    test_support::fixtures::Fixture,
    view::{
        close_draft::{self, Ask, CloseWithDraft, Offers},
        drafts::actions::open_discard_saying,
        harness::Shell,
        quit::{DraftsNotSaved, EditsNotKept, NotSavedChoice},
        retire_asks::{self, RetireChoice},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FF1";

/// The time the Kit's deferred close waits before it gives focus back
/// (0.25 s), and a little more.
pub(super) const PAST_THE_RESTORE: Duration = Duration::from_millis(300);

/// What a dialog's choice callback was told, oldest first.
pub(super) type Chosen = Rc<RefCell<Vec<String>>>;

/// One repository, `a`, with a ticket, refreshed, in an open window with
/// `a` selected and its ticket list shown and holding focus.
pub(super) fn listed(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Listed", "open");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.use_list(Pane::Tickets, cx);
    shell.keys("secondary-2", cx);
    (shell, root)
}

/// The handle focus is on now, which a dialog opened here gives focus
/// back to.
pub(super) fn origin(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell
        .frame(cx, |window, cx| window.focused(cx))
        .expect("something holds focus")
}

/// Whether focus is back on `origin`, once the Kit's deferred restore has
/// had its quarter second on the test clock.
pub(super) fn back_at(shell: &Shell, origin: &FocusHandle, cx: &mut TestAppContext) -> bool {
    cx.executor().advance_clock(PAST_THE_RESTORE);
    focused(shell, origin, cx)
}

/// Whether focus is on the element `id` or inside it, as the Kit's inputs
/// and buttons inside a labelled wrapper take it, after two draws.
pub(super) fn inside(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| {
        window.try_find(id).is_some_and(|wrapper| {
            gpui_kit::base::test_support::has_observed_focus(window, wrapper.path())
        })
    })
}

/// A recording callback's words, for a row to compare.
fn said(chosen: &Chosen) -> Vec<String> {
    chosen.borrow().clone()
}

/// What opens one ask over the window, telling `Chosen` what it chose.
type Opener = Box<dyn Fn(&mut Window, &mut App, Chosen)>;

/// One dialog that asks, opened by `open` over the listed window.
struct Asks {
    row: &'static str,
    /// The dialog's body element.
    dialog: &'static str,
    open: Opener,
    /// What Enter and Escape tell the callback, with nothing else focused.
    enter: &'static [&'static str],
    escape: &'static [&'static str],
}

/// P1's rows of the asks: on open, focus is the dialog's own; Escape
/// closes it with `escape` chosen and focus back on the list it was opened
/// from; Enter closes it with `enter` chosen and focus back too.
pub(super) fn asks_keys(table: &mut Table, cx: &mut TestAppContext) {
    let offers = Rc::new(|_: &App| Offers {
        keep: Some(true),
        save: Some(true),
        discard: true,
    });
    let asks = vec![
        Asks {
            row: "Close a tab with a draft",
            dialog: "close-draft-dialog",
            open: Box::new(move |window, cx, chosen| {
                let ask = Ask::restored("Listed", offers.clone(), Rc::new(|_| None));
                close_draft::open(ask, window, cx, move |choice: CloseWithDraft, _, _| {
                    chosen.borrow_mut().push(format!("{choice:?}"))
                });
            }),
            enter: &["KeepDraft"],
            escape: &["Cancel"],
        },
        Asks {
            row: "Quit, drafts not saved",
            dialog: "drafts-not-saved-dialog",
            open: Box::new(|window, cx, chosen| {
                DraftsNotSaved { count: 1 }.open(
                    window,
                    cx,
                    move |choice: NotSavedChoice, _, _| {
                        chosen.borrow_mut().push(format!("{choice:?}"))
                    },
                );
            }),
            enter: &["Cancel"],
            escape: &["Cancel"],
        },
        Asks {
            row: "Quit, edits not kept",
            dialog: "edits-not-kept-dialog",
            open: Box::new(|window, cx, chosen| {
                EditsNotKept { count: 2 }.open(window, cx, move |choice: NotSavedChoice, _, _| {
                    chosen.borrow_mut().push(format!("{choice:?}"))
                });
            }),
            enter: &["Cancel"],
            escape: &["Cancel"],
        },
        Asks {
            row: "Retire, unkept text",
            dialog: "retire-unkept-dialog",
            open: Box::new(|window, cx, chosen| {
                retire_asks::ask_unkept(2, window, cx, move |choice: RetireChoice, _, _| {
                    chosen.borrow_mut().push(format!("{choice:?}"))
                });
            }),
            enter: &["ShowTab"],
            escape: &["Cancel"],
        },
        Asks {
            row: "Retire, kept drafts",
            dialog: "retire-kept-dialog",
            open: Box::new(|window, cx, chosen| {
                retire_asks::ask_kept(
                    TargetKind::Ticket,
                    Some(2),
                    window,
                    cx,
                    move |choice: RetireChoice, _, _| {
                        chosen.borrow_mut().push(format!("{choice:?}"))
                    },
                );
            }),
            enter: &["Cancel"],
            escape: &["Cancel"],
        },
    ];
    for ask in asks {
        ask_row(table, ask, cx);
    }
    discard_draft_row(table, cx);
}

fn ask_row(table: &mut Table, ask: Asks, cx: &mut TestAppContext) {
    let (shell, _root) = listed(cx);
    let origin = origin(&shell, cx);
    let row = ask.row;
    let chosen: Chosen = Rc::default();
    let open = |shell: &Shell, cx: &mut TestAppContext| {
        let chosen = chosen.clone();
        shell.frame(cx, |window, cx| (ask.open)(window, cx, chosen));
        shell.run(cx);
        assert!(shell.present(ask.dialog, cx), "{row}: the dialog opened");
    };

    open(&shell, cx);
    let at = focus_at(&shell, cx);
    table.fact(
        &format!("{row}, on open"),
        at == Some(dialog_itself()),
        &format!("focus on the dialog itself (it is on {at:?})"),
    );
    shell.keys("escape", cx);
    table.fact(
        &format!("{row}, Escape closes it"),
        !shell.present(ask.dialog, cx),
        "the dialog closed",
    );
    table.fact(
        &format!("{row}, Escape chooses"),
        said(&chosen) == ask.escape,
        &format!("{:?} chosen (it was {:?})", ask.escape, said(&chosen)),
    );
    table.fact(
        &format!("{row}, Escape gives focus back"),
        back_at(&shell, &origin, cx),
        "focus on the list it was opened from",
    );

    chosen.borrow_mut().clear();
    open(&shell, cx);
    shell.keys("enter", cx);
    table.fact(
        &format!("{row}, Enter closes it"),
        !shell.present(ask.dialog, cx),
        "the dialog closed",
    );
    table.fact(
        &format!("{row}, Enter chooses"),
        said(&chosen) == ask.enter,
        &format!("{:?} chosen (it was {:?})", ask.enter, said(&chosen)),
    );
    table.fact(
        &format!("{row}, Enter gives focus back"),
        back_at(&shell, &origin, cx),
        "focus on the list it was opened from",
    );
}

/// Discard a draft: its default is Cancel, so Enter and Escape each close
/// it with nothing confirmed, and focus goes back to the origin it was
/// given.
fn discard_draft_row(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _root) = listed(cx);
    let origin = origin(&shell, cx);
    let chosen: Chosen = Rc::default();
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let open = |shell: &Shell, cx: &mut TestAppContext| {
        let (chosen, services, origin) = (chosen.clone(), services.clone(), origin.clone());
        shell.frame(cx, |window, cx| {
            open_discard_saying(
                "Discard this draft?".to_owned(),
                origin,
                &services,
                window,
                cx,
                move |_, _| chosen.borrow_mut().push("confirmed".to_owned()),
            )
        });
        shell.run(cx);
        assert!(
            shell.present("discard-draft-dialog", cx),
            "Discard a draft: the dialog opened"
        );
    };

    open(&shell, cx);
    let at = focus_at(&shell, cx);
    table.fact(
        "Discard a draft, on open",
        at == Some(dialog_itself()),
        &format!("focus on the dialog itself (it is on {at:?})"),
    );
    shell.keys("escape", cx);
    table.fact(
        "Discard a draft, Escape closes it",
        !shell.present("discard-draft-dialog", cx),
        "the dialog closed",
    );
    table.fact(
        "Discard a draft, Escape confirms nothing",
        said(&chosen).is_empty(),
        "nothing confirmed",
    );
    table.fact(
        "Discard a draft, Escape gives focus back",
        back_at(&shell, &origin, cx),
        "focus on the origin it was given",
    );

    open(&shell, cx);
    shell.keys("enter", cx);
    table.fact(
        "Discard a draft, Enter closes it",
        !shell.present("discard-draft-dialog", cx),
        "the dialog closed",
    );
    table.fact(
        "Discard a draft, Enter confirms nothing",
        said(&chosen).is_empty(),
        "nothing confirmed",
    );
    table.fact(
        "Discard a draft, Enter gives focus back",
        back_at(&shell, &origin, cx),
        "focus on the origin it was given",
    );
}
