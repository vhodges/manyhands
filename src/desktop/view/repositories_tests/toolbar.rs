//! The Repositories toolbar's journeys (Cycle 03a, Part E): Check all over
//! four real repositories with polling on, and the More menu over two. Every
//! poll is answered by the double and every check by the delaying backend,
//! so nothing reaches a transport.
//!
//! Focus after a press or a choice is read in the update that causes it: the
//! next draw's fallback (`Workspace::focus_lost`) would put it back on the
//! last region anyway.

use std::path::Path;

use gpui_kit::{FocusHandle, TestAppContext, component::WindowExt, test::TestWindowExt};
use manyhands::repository::{ItemDtoKind, PollingOutcome, SharedKeyId};

use super::{super::check_all_roots, support::*};
use crate::desktop::{
    keys::{RepositoryMenuChoice, Shown},
    model::{
        list_view::{REBUILD_INDEX, REFRESH},
        navigation::OpenTarget,
        polling::Reason,
        repository_state::{LOCKED, MenuChoice, PAUSED, REMOVE_FROM_MANYHANDS},
        stamp::RepoKey,
    },
    services::{JobKind, requests::Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        confirm::{CONFIRM, FILES_REMAIN},
        dock::tests::support::act,
        footer::tests::support::open_modal,
        harness::{Shell, key},
        polling_window::PollingWindow,
        workspace::ToolbarButton,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5TB0";

fn button(shell: &Shell, button: ToolbarButton, cx: &mut TestAppContext) -> FocusHandle {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.toolbar_focus(button).clone())
}

fn focus(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| handle.focus(window, cx));
}

/// The chosen repository's ticket, open in a tab, so the details sheet can
/// be opened over the window.
fn open_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

/// A press of Check all, as a click or Enter reaches the Workspace.
fn press_check_all(shell: &Shell, cx: &mut TestAppContext) {
    act(shell, cx, |workspace, window, cx| {
        workspace.press_toolbar(ToolbarButton::CheckAll, window, cx)
    });
}

fn check_alls(shell: &Shell, cx: &mut TestAppContext) -> Vec<RepoKey> {
    shell.workspace.read_with(cx, check_all_roots)
}

/// P23, P24: an ordinary, a paused, a locked and a stored-`UnlockRequired`
/// repository. Check all sends `PollOnce` for the first two, in list order,
/// and nothing for the others; nothing else is sent; while those checks run
/// it takes nothing and sends nothing, and focus has moved to the list; it
/// is refused behind a dialog and a sheet.
pub(super) async fn check_all(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (a, b, c, d, e) = (
        fixture.create_repository("a"),
        fixture.create_repository("b"),
        fixture.create_repository("c"),
        fixture.create_repository("d"),
        fixture.create_repository("e"),
    );
    fixture.commit_ticket(&a, TICKET, "Ticket", "");
    for root in [&a, &b, &c, &d, &e] {
        fixture.publish_over_ssh(root);
        fixture.refresh(root);
    }
    // `e` is paused and its stored outcome says a key is locked (owner
    // ruling 2026-11-12): Check all takes it for no check, so no prompt.
    fixture.pause_polling(&b);
    fixture.pause_polling(&e);
    let [key_a, key_b, key_c] = [&a, &b, &c].map(|root| key(root));
    let (window, ()) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| {
            backend.store_outcome(&key(&d), PollingOutcome::UnlockRequired);
            backend.store_outcome(&key(&e), PollingOutcome::UnlockRequired);
            // Every launch poll, and every poll a check's end starts, is
            // not due: nothing reaches the library.
            backend.answer_unanswered_polls(not_due(clock));
        },
        cx,
    );
    let PollingWindow {
        shell,
        services,
        backend,
        ..
    } = window;
    let services = services.with_polling();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell
        .until_drawn(cx, "d's stored outcome to count", |shell, cx| {
            state(shell, "d", cx).as_deref() == Some(LOCKED)
        })
        .await;
    assert_eq!(state(&shell, "b", cx).as_deref(), Some(PAUSED));
    // The launch polls have ended: Check all skips a repository whose
    // check is still running.
    shell
        .until_drawn(cx, "the launch polls to end", |_, cx| {
            [&key_a, &key_b].iter().all(|root| {
                services.scheduler().read_with(cx, |scheduler, _| {
                    scheduler
                        .schedule()
                        .repo(root)
                        .is_none_or(|repo| !repo.in_flight())
                })
            })
        })
        .await;
    // `raise` refuses an unscheduled repository and one whose check is
    // running: `c` is scheduled, and its launch poll has ended.
    shell
        .until_drawn(cx, "c to be scheduled and quiet", |_, cx| {
            services.scheduler().read_with(cx, |scheduler, _| {
                scheduler
                    .schedule()
                    .repo(&key_c)
                    .is_some_and(|repo| !repo.in_flight())
            })
        })
        .await;

    // `c`'s key is locked, as the scheduler says it live.
    let locked = SharedKeyId::new();
    services.scheduler().update(cx, |scheduler, cx| {
        assert!(
            scheduler
                .schedule_mut()
                .raise(&key_c, Reason::KeyLocked(locked))
        );
        cx.notify();
    });
    shell.run(cx);
    assert_eq!(state(&shell, "c", cx).as_deref(), Some(LOCKED));

    // Check all takes the paused and the ordinary one, in list order.
    let order = shell.rows("repo-row", cx);
    let listed = |wanted: &[&str]| -> Vec<RepoKey> {
        order
            .iter()
            .filter(|name| wanted.contains(&name.as_str()))
            .map(|name| match name.as_str() {
                "a" => key_a.clone(),
                "b" => key_b.clone(),
                other => panic!("no key for {other}"),
            })
            .collect()
    };
    let taken = listed(&["a", "b"]);
    assert_eq!(check_alls(&shell, cx), taken);
    open_ticket(&shell, &a, cx);
    shell.select(&a, cx);

    // Behind a dialog and behind the sheet it sends nothing (P12).
    let sent = |cx: &mut TestAppContext| shell.requests(cx, |_| true).len();
    for sheet in [false, true] {
        open_modal(&shell, sheet, cx);
        let before = sent(cx);
        press_check_all(&shell, cx);
        assert_eq!(sent(cx), before, "sheet {sheet}: refused");
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }

    // By Enter on the button: one `PollOnce` each for the two, nothing for
    // the others, and focus on the list, as Check all disabled itself.
    let (check_all_button, list) = (
        button(&shell, ToolbarButton::CheckAll, cx),
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.repositories_list_focus(cx)),
    );
    let _held = [
        backend.inner().hold_job(JobKind::PollOnce),
        backend.inner().hold_job(JobKind::PollOnce),
    ];
    backend.inner().answer_job(JobKind::PollOnce, polled());
    backend.inner().answer_job(JobKind::PollOnce, polled());
    focus(&shell, &check_all_button, cx);
    let before = sent(cx);
    assert!(
        press_released_and_hold(&shell, "enter", &list, cx),
        "focus is on the list once Check all disables itself"
    );
    assert_eq!(sent(cx), before + 2, "two requests, nothing else");
    let sent_to = |root: &Path, cx: &mut TestAppContext| checks(&shell, root, cx);
    let (first, second) = (sent_to(&a, cx), sent_to(&b, cx));
    assert_eq!((first.len(), second.len()), (1, 1), "one check each");
    assert!(sent_to(&c, cx).is_empty(), "locked: none");
    assert!(sent_to(&d, cx).is_empty(), "stored unlock required: none");
    assert!(sent_to(&e, cx).is_empty(), "paused, stored unlock required");
    let mut by_time = [(first[0], key_a.clone()), (second[0], key_b.clone())];
    by_time.sort_by_key(|(id, _)| *id);
    assert_eq!(
        by_time.map(|(_, root)| root).to_vec(),
        taken,
        "in list order"
    );

    // While they run it takes nothing, and a press sends nothing.
    assert_eq!(check_alls(&shell, cx), Vec::<RepoKey>::new());
    press_check_all(&shell, cx);
    assert_eq!(sent(cx), before + 2, "nothing while they run");
    drop(_held);
    for id in [first[0], second[0]] {
        shell.until_ended(id, cx).await;
    }
    shell.run(cx);
    assert_eq!(check_alls(&shell, cx), taken, "enabled again");
    assert!(backend.asked().is_empty(), "no passphrase was asked for");
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.passphrase().is_none()),
        "no passphrase dialog"
    );
}

/// P11, P13, R-35, R-40: the More menu, for the chosen repository whatever
/// row the Kit cursor rests on. Rebuild sends `IndexRebuild` for it and
/// focus stays on the button; Esc returns to the button; a Refresh chosen
/// for a repository that is not the chosen one sends nothing; Remove from
/// Manyhands… asks, and Cancel gives focus back to the button, OK removes
/// the repository.
pub(super) async fn more_menu(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let (a, b) = (
        fixture.create_repository("a"),
        fixture.create_repository("b"),
    );
    for root in [&a, &b] {
        fixture.refresh(root);
    }
    let shell = Shell::open(fixture, cx);
    let (key_a, key_b) = (key(&a), key(&b));
    shell.select(&a, cx);
    let more = button(&shell, ToolbarButton::RepositoriesMore, cx);

    // The Kit cursor on the other row.
    shell.keys("secondary-1", cx);
    shell.keys("down", cx);
    let cursor = |cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, cx| {
            workspace.repositories_pane().highlighted(cx)
        })
    };
    if cursor(cx) == Some(key_a.clone()) {
        shell.keys("up", cx);
    }
    assert_ne!(cursor(cx), Some(key_a.clone()));

    let intents = |cx: &mut TestAppContext, root: &Path, rebuild: bool| {
        shell
            .requests(cx, |request| match &request.intent {
                Intent::IndexRebuild { root: asked } => rebuild && asked == root,
                Intent::IndexRefresh { root: asked } => !rebuild && asked == root,
                _ => false,
            })
            .len()
    };
    let total = |cx: &mut TestAppContext| shell.requests(cx, |_| true).len();

    // Esc returns focus to the button.
    focus(&shell, &more, cx);
    press_released_and_hold(&shell, "enter", &more, cx);
    assert_eq!(open_root(&shell, cx), Some(key_a.clone()), "a's");
    let before = total(cx);
    assert!(press_and_hold(&shell, "escape", &more, cx), "Esc: More");
    assert_eq!(open_root(&shell, cx), None);
    assert_eq!(total(cx), before, "opening and Esc send nothing");

    // Rebuild the search index: `IndexRebuild` for `a`, focus on More.
    focus(&shell, &more, cx);
    press_released_and_hold(&shell, "enter", &more, cx);
    assert_eq!(open_root(&shell, cx), Some(key_a.clone()), "a's");
    let before = intents(cx, &a, true);
    assert!(choose_and_hold(&shell, REBUILD_INDEX, &more, cx), "focus");
    assert_eq!(intents(cx, &a, true), before + 1, "IndexRebuild of a");
    assert_eq!(intents(cx, &b, true), 0, "not b");

    // A Refresh the menu showed for `b`, while `a` is chosen, does nothing.
    let stale = RepositoryMenuChoice {
        root: key_b.clone(),
        choice: MenuChoice::Refresh,
        shown: Shown::Label(REFRESH.to_owned()),
    };
    let before = total(cx);
    shell.frame(cx, |window, cx| window.dispatch_action(Box::new(stale), cx));
    shell.run(cx);
    assert_eq!(total(cx), before, "no request for a repository not chosen");

    // Remove from Manyhands…: the dialog with its note; Cancel gives focus
    // back to More; OK removes `a`.
    let ready = |shell: &Shell, cx: &mut TestAppContext| {
        shell.label("confirm-ok", cx).as_deref() == Some(CONFIRM)
            && shell.present("confirm-expires", cx)
    };
    focus(&shell, &more, cx);
    press_released_and_hold(&shell, "enter", &more, cx);
    choose(&shell, REMOVE_FROM_MANYHANDS, cx);
    shell.until_drawn(cx, "the removal's preview", ready).await;
    assert_eq!(
        shell.label("confirm-note", cx).as_deref(),
        Some(FILES_REMAIN)
    );
    let back = shell.frame(cx, |window, cx| {
        window.click("confirm-cancel", cx);
        more.is_focused(window)
    });
    assert!(back, "Cancel: the More button");
    shell.run(cx);
    assert!(!shell.present("confirm-dialog", cx));
    focus(&shell, &more, cx);
    press_released_and_hold(&shell, "enter", &more, cx);
    choose(&shell, REMOVE_FROM_MANYHANDS, cx);
    shell.until_drawn(cx, "the removal's preview", ready).await;
    shell.click("confirm-ok", cx);
    shell
        .until_drawn(cx, "a's row gone", |shell, cx| {
            !shell.rows("repo-row", cx).contains(&"a".to_owned())
        })
        .await;
}
