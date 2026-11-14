//! The journey of `the_menu_by_pointer_and_keyboard` (Cycle 03a, Part D):
//! four real repositories with polling on, every poll answered by the
//! double and every check and share by the delaying backend, and a row's
//! menu opened by a right click and by `shift-f10` and `menu`. Items are
//! read and chosen by label: a separator takes an index (fact 43), and the
//! snapshot cannot read an item's enablement, which `menu_items`' own table
//! pins; here a disabled choice is checked to send nothing. A poll nothing
//! is queued for gets the double's stand-in, never the library's call.
//!
//! Focus after Esc or a choice is read in the update that causes it: the
//! next draw's fallback (`Workspace::focus_lost`) would put it back on the
//! last region anyway.

use std::{fs, path::Path};

use gpui_kit::{TestAppContext, component::WindowExt, point, px, test::TestWindowExt};
use manyhands::repository::{ItemDtoKind, OperationId, RepositoryService, SharedKeyId};

use super::{super::menu::MenuAnchor, support::*};
use crate::desktop::{
    keys::{OpenRepositoryMenu, RepositoryMenuChoice, Shown},
    model::{
        list_view::RETRY,
        navigation::OpenTarget,
        polling::{
            Answer, Clock as _, PollAttempt, PollSchedule, Reason, TransportFailure,
            attention::{RESUME_CHECKING, UNLOCK},
            status::Unplaced,
        },
        repository_state::{
            CANT_OPEN, CANT_REACH, CHECK_NOW, LOCKED, MenuChoice, NEEDS_ATTENTION, OPEN_FOLDER,
            REMOVE_FROM_MANYHANDS, RETRY_NOW, SETTINGS, kept_sentence, sentence_line,
        },
        stamp::{RepoKey, SettingsPage},
    },
    services::{JobKind, requests::Intent},
    test_support::fixtures::{Fixture, item_id, remove_registration},
    view::{
        confirm::{CONFIRM, FILES_REMAIN},
        footer::Control,
        footer::tests::support::{modal_open, open_modal},
        harness::{Shell, key},
        item::OpenedFolders,
        polling_window::PollingWindow,
        settings::Shown as Page,
        sidebar::{SHARE_MAIN, SHARE_MAIN_SET_UP},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5MN0";

pub(super) async fn journey(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (a, b, c, d) = (
        fixture.create_repository("a"),
        fixture.create_repository("b"),
        fixture.create_repository("c"),
        fixture.create_repository("d"),
    );
    fixture.commit_ticket(&a, TICKET, "Ticket", "");
    for root in [&a, &b, &c] {
        fixture.publish_over_ssh(root);
    }
    for root in [&a, &b, &c, &d] {
        fixture.refresh(root);
    }
    // `c`'s folder is away while the scheduler reads it: it cannot.
    let away = c.with_file_name("c-away");
    fs::rename(&c, &away).unwrap();
    let (window, ()) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| {
            // `a`'s and `b`'s launch polls; every later poll, however many
            // a test answer's rearm or backoff starts, is not due.
            backend.answer_poll(completed(clock));
            backend.answer_poll(completed(clock));
            backend.answer_unanswered_polls(not_due(clock));
        },
        cx,
    );
    let PollingWindow {
        shell,
        services,
        backend,
        clock,
        ..
    } = window;
    let services = services.with_polling();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let [key_a, key_b, key_c, key_d] = [&a, &b, &c, &d].map(|root| key(root));
    let polled_once = |root: &RepoKey, cx: &mut TestAppContext| {
        services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(root)
                .is_some_and(|repo| repo.last.is_some() && !repo.in_flight())
        })
    };
    let unplaced = |root: &RepoKey, cx: &mut TestAppContext| {
        services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.unplaced(root))
    };
    shell
        .until_drawn(cx, "the launch polls, and c unread", |_, cx| {
            polled_once(&key_a, cx)
                && polled_once(&key_b, cx)
                && unplaced(&key_c, cx) == Some(Unplaced::Unreadable)
        })
        .await;
    let schedule = |cx: &mut TestAppContext, f: &dyn Fn(&mut PollSchedule)| {
        services.scheduler().update(cx, |scheduler, cx| {
            f(scheduler.schedule_mut());
            cx.notify();
        });
        shell.run(cx);
    };

    // `a` is chosen, its ticket open in a tab whose details sheet the
    // guards open.
    shell.select(&a, cx);
    let target = OpenTarget::Item {
        repo: key_a.clone(),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    let chosen = |cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            workspace.navigation().selected().cloned()
        })
    };
    let list = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.repositories_list_focus(cx));
    let sent = |cx: &mut TestAppContext| shell.requests(cx, |_| true).len();

    // A right click on `b` while `a` is chosen: `b`'s menu at the pointer,
    // the choice unchanged. Its items are `menu_items`', the state's
    // sentence first. Opening it, moving in it and Esc read nothing and
    // send nothing (P22); Esc gives focus back to what held it.
    // Focus is on the footer's Settings, which the draw's fallback never
    // picks, so only the menu's own restore puts it back there.
    let settings = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(Control::Settings));
    shell.frame(cx, |window, cx| settings.focus(window, cx));
    let (reads, requests, polls) = (all_reads(&shell), sent(cx), backend.polled_roots().len());
    right_click(&shell, "b", cx);
    assert_eq!(open_root(&shell, cx), Some(key_b.clone()), "b's menu");
    assert_eq!(chosen(cx), Some(key_a.clone()), "the choice stays");
    let b_items = items_of(&shell, "b", cx);
    assert_eq!(labels(&shell, cx), b_items);
    assert_eq!(
        b_items[0],
        sentence_line(&row_of(&shell, "b", cx).state),
        "first, the state's sentence (P21)"
    );
    shell.keys("down", cx);
    shell.keys("escape", cx);
    assert_eq!(open_root(&shell, cx), None, "Esc closes it");
    assert!(focused(&shell, &settings, cx), "Esc: back where focus was");
    assert_eq!(all_reads(&shell), reads, "no read");
    assert_eq!(sent(cx), requests, "no request");
    assert_eq!(backend.polled_roots().len(), polls, "no poll");

    // By keyboard: the list, the Kit cursor moved to `b` (it starts on the
    // chosen row), then `shift-f10` and `menu` open the highlighted row's
    // menu; Esc gives focus back to the list.
    shell.keys("secondary-1", cx);
    assert!(focused(&shell, &list, cx), "the list has focus");
    let (at_a, at_b) = (index(&shell, "a", cx), index(&shell, "b", cx));
    assert!(at_b > at_a);
    for _ in at_a..at_b {
        shell.keys("down", cx);
    }
    assert_eq!(chosen(cx), Some(key_a.clone()));
    shell.keys("shift-f10", cx);
    assert_eq!(open_root(&shell, cx), Some(key_b.clone()), "shift-f10");
    assert_eq!(labels(&shell, cx), b_items);
    let anchored = shell.workspace.read_with(cx, |workspace, _| {
        let row = workspace.repositories_pane().row_bounds(&key_b);
        (
            workspace.repository_menu_at(),
            row.map(|row| row.bottom_left()),
        )
    });
    assert!(anchored.1.is_some(), "b's row was drawn");
    assert_eq!(anchored.0, anchored.1, "below b's row (R-42)");
    shell.keys("down", cx);
    assert!(press_and_hold(&shell, "escape", &list, cx), "Esc: the list");
    assert_eq!(open_root(&shell, cx), None);
    assert_eq!(all_reads(&shell), reads, "no read by keyboard");
    assert_eq!(sent(cx), requests, "no request by keyboard");
    shell.keys("menu", cx);
    assert_eq!(open_root(&shell, cx), Some(key_b.clone()), "menu");

    // Check now: one `PollOnce` of `b` on the user lane, and focus back on
    // the list it was opened from.
    assert!(b_items.iter().any(|label| label == CHECK_NOW));
    backend.inner().answer_job(JobKind::PollOnce, polled());
    let held = choose_and_hold(&shell, CHECK_NOW, &list, cx);
    assert!(held, "the list it was opened from");
    assert_eq!(open_root(&shell, cx), None);
    let checked = checks(&shell, &b, cx);
    assert_eq!(checked.len(), 1, "one Check now of b");
    shell.until_ended(checked[0], cx).await;
    shell.run(cx);

    // Behind a dialog and behind the item details sheet: an open menu
    // closes when the modal opens, and the keys' action, the right press's
    // opener and a dispatched choice (Check now, and Remove from
    // Manyhands…) do nothing (P12).
    for sheet in [false, true] {
        right_click(&shell, "b", cx);
        assert_eq!(open_root(&shell, cx), Some(key_b.clone()));
        open_modal(&shell, sheet, cx);
        assert_eq!(open_root(&shell, cx), None, "sheet {sheet}: it closed");
        let requests = sent(cx);
        let choice = |choice: MenuChoice, label: &str| RepositoryMenuChoice {
            root: key_b.clone(),
            choice,
            shown: Shown::Label(label.to_owned()),
        };
        shell.frame(cx, |window, cx| {
            window.dispatch_action(Box::new(OpenRepositoryMenu), cx);
            window.dispatch_action(Box::new(choice(MenuChoice::CheckNow, CHECK_NOW)), cx);
            let remove = choice(MenuChoice::Remove, REMOVE_FROM_MANYHANDS);
            window.dispatch_action(Box::new(remove), cx);
        });
        shell.run(cx);
        shell.frame(cx, |window, cx| {
            shell.workspace.update(cx, |workspace, cx| {
                let at = MenuAnchor::Pointer(point(px(10.), px(10.)));
                workspace.open_repository_menu(key_b.clone(), at, window, cx)
            })
        });
        shell.run(cx);
        assert_eq!(open_root(&shell, cx), None, "sheet {sheet}: none opens");
        assert_eq!(sent(cx), requests, "sheet {sheet}: nothing sent");
        assert!(!shell.present("confirm-dialog", cx), "sheet {sheet}");
        assert!(modal_open(&shell, cx), "sheet {sheet}: the modal stays");
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }

    // The sentence, always disabled, chosen by pointer and as a dispatched
    // choice: nothing is sent.
    let requests = sent(cx);
    right_click(&shell, "b", cx);
    let sentence = labels(&shell, cx)[0].clone();
    choose(&shell, &sentence, cx);
    let read = RepositoryMenuChoice {
        root: key_b.clone(),
        choice: MenuChoice::Sentence,
        shown: Shown::Label(sentence),
    };
    shell.frame(cx, |window, cx| window.dispatch_action(Box::new(read), cx));
    shell.run(cx);
    assert_eq!(sent(cx), requests, "the sentence sends nothing");
    close_menu(&shell, cx);

    // Open folder records `b`'s folder.
    right_click(&shell, "b", cx);
    choose(&shell, OPEN_FOLDER, cx);
    let opened = shell.frame(cx, |_, cx| {
        cx.try_global::<OpenedFolders>()
            .map(|opened| opened.0.clone())
            .unwrap_or_default()
    });
    assert_eq!(opened, [Path::new(&key_b).to_owned()]);

    // Share main: `b` has a publication remote, so `repo sync`.
    backend
        .inner()
        .answer_job(JobKind::SynchronizeRepo, synced());
    right_click(&shell, "b", cx);
    choose(&shell, SHARE_MAIN, cx);
    let shares = shell.requests(
        cx,
        |request| matches!(&request.intent, Intent::RepoSync { root } if *root == b),
    );
    assert_eq!(shares.len(), 1, "one share of b's main");
    shell.until_ended(shares[0], cx).await;
    shell.run(cx);

    // Locked: Unlock… reaches `Scheduler::unlock`, which clears the key's
    // Not now mark and sends a Check now (D6 C3).
    let locked = SharedKeyId::new();
    services.worker().decline_for_testing(locked);
    schedule(cx, &|schedule| {
        assert!(schedule.raise(&key_b, Reason::KeyLocked(locked)));
    });
    assert_eq!(state(&shell, "b", cx).as_deref(), Some(LOCKED));
    backend.inner().answer_job(JobKind::PollOnce, polled());
    let before = checks(&shell, &b, cx).len();
    right_click(&shell, "b", cx);
    choose(&shell, UNLOCK, cx);
    assert!(!services.worker().is_declined(locked), "unlocked");
    let unlocked = checks(&shell, &b, cx);
    assert_eq!(unlocked.len(), before + 1, "its Check now");
    shell.until_ended(unlocked[0], cx).await;
    // The Check's end reads `b`'s registration, and an automatic poll may
    // still be running: wait for both before acting on the schedule by hand.
    shell
        .until_drawn(cx, "b to be quiet after the Unlock", |_, cx| {
            services.scheduler().read_with(cx, |scheduler, _| {
                scheduler
                    .schedule()
                    .repo(&key_b)
                    .is_some_and(|repo| !repo.in_flight())
                    && !scheduler.refreshing(&key_b)
            })
        })
        .await;
    schedule(cx, &|schedule| {
        schedule.clear_attention(&Reason::KeyLocked(locked));
    });

    // A menu opened on Can't reach, whose action is Retry now; then a live
    // attention makes the row Needs attention with Resume checking. The
    // Retry now it showed, chosen, sends nothing and resumes nothing: it is
    // no longer what it showed (P40, R-35).
    let unreachable = PollAttempt {
        root: key_b.clone(),
        operation: OperationId::new(),
        answer: Answer::Transport(TransportFailure::Unreachable),
        completed_at: Some(clock.now().utc),
        next_eligible_at: Some(clock.now().utc + time::Duration::seconds(300)),
        targets: Vec::new(),
        index_pending: false,
    };
    let now = clock.now();
    schedule(cx, &|schedule| {
        schedule.after(&now, &unreachable);
    });
    assert_eq!(state(&shell, "b", cx).as_deref(), Some(CANT_REACH));
    right_click(&shell, "b", cx);
    assert_eq!(labels(&shell, cx)[1], RETRY_NOW, "its action");
    schedule(cx, &|schedule| {
        assert!(schedule.raise(&key_b, Reason::RecoveryHeld(key_b.clone())));
    });
    assert_eq!(state(&shell, "b", cx).as_deref(), Some(NEEDS_ATTENTION));
    assert_eq!(items_of(&shell, "b", cx)[1], RESUME_CHECKING);
    let before = checks(&shell, &b, cx).len();
    choose(&shell, RETRY_NOW, cx);
    assert_eq!(checks(&shell, &b, cx).len(), before, "no check");
    assert!(backend.resumed().is_empty(), "nothing resumed");
    schedule(cx, &|schedule| {
        schedule.clear_attention(&Reason::RecoveryHeld(key_b.clone()));
    });

    // A menu opened on `a`, then a check of it starts: its Check now, now
    // disabled, sends nothing (P40).
    right_click(&shell, "a", cx);
    assert!(labels(&shell, cx).iter().any(|label| label == CHECK_NOW));
    schedule(cx, &|schedule| schedule.mark_queued(&key_a));
    let before = checks(&shell, &a, cx).len();
    choose(&shell, CHECK_NOW, cx);
    assert_eq!(checks(&shell, &a, cx).len(), before, "it was disabled");
    // Opened while the check runs, its Check now is built disabled.
    right_click(&shell, "a", cx);
    choose(&shell, CHECK_NOW, cx);
    assert_eq!(checks(&shell, &a, cx).len(), before, "built disabled");
    close_menu(&shell, cx);
    schedule(cx, &|schedule| schedule.poll_ended(&key_a));

    // Retry on `c`, which the scheduler could not read and which is not
    // chosen: its registration is read again (P26, R-19).
    assert_eq!(state(&shell, "c", cx).as_deref(), Some(NEEDS_ATTENTION));
    fs::rename(&away, &c).unwrap();
    let remote_reads = |cx: &mut TestAppContext| {
        shell.run(cx);
        backend
            .remote_read_roots()
            .iter()
            .filter(|root| **root == key_c)
            .count()
    };
    let before = remote_reads(cx);
    right_click(&shell, "c", cx);
    assert_eq!(labels(&shell, cx)[1], RETRY);
    choose(&shell, RETRY, cx);
    let scheduled = |cx: &mut TestAppContext| {
        services.scheduler().read_with(cx, |scheduler, _| {
            scheduler.schedule().repo(&key_c).is_some()
        })
    };
    shell
        .until_drawn(cx, "c read again", |_, cx| scheduled(cx))
        .await;
    assert!(remote_reads(cx) > before, "read again");
    assert_eq!(chosen(cx), Some(key_a.clone()));

    // "Share main…" on `d`, which has no publication remote: Settings ›
    // Remotes, on `d`.
    right_click(&shell, "d", cx);
    choose(&shell, SHARE_MAIN_SET_UP, cx);
    assert_eq!(page(&shell, cx), Page::Settings(SettingsPage::Remotes));
    assert_eq!(
        shell.label("remotes-root", cx),
        Some(format!("Repository: {key_d}"))
    );

    // Settings… on `b`, not the chosen one: Settings opens on `b` (P27).
    right_click(&shell, "b", cx);
    choose(&shell, SETTINGS, cx);
    assert_eq!(page(&shell, cx), Page::Settings(SettingsPage::Repositories));
    assert_eq!(shell.label("managed-root", cx), Some(key_b.clone()));
    assert_eq!(chosen(cx), Some(key_a.clone()));

    // Remove from Manyhands… on `d`: the dialog with its note; Cancel gives
    // focus back to the list the menu was opened from (P11, R-12).
    let ready = |shell: &Shell, cx: &mut TestAppContext| {
        shell.label("confirm-ok", cx).as_deref() == Some(CONFIRM)
            && shell.present("confirm-expires", cx)
    };
    right_click(&shell, "d", cx);
    choose(&shell, REMOVE_FROM_MANYHANDS, cx);
    shell.until_drawn(cx, "the removal's preview", ready).await;
    assert_eq!(
        shell.label("confirm-note", cx).as_deref(),
        Some(FILES_REMAIN)
    );
    let back = shell.frame(cx, |window, cx| {
        window.click("confirm-cancel", cx);
        list.is_focused(window)
    });
    assert!(back, "Cancel: the list");
    shell.run(cx);
    assert!(!shell.present("confirm-dialog", cx));

    // OK removes it, and its row goes, with the bounds it was drawn at.
    right_click(&shell, "d", cx);
    choose(&shell, REMOVE_FROM_MANYHANDS, cx);
    shell.until_drawn(cx, "the removal's preview", ready).await;
    shell.click("confirm-ok", cx);
    shell
        .until_drawn(cx, "d's row gone", |shell, cx| {
            !shell.rows("repo-row", cx).contains(&"d".to_owned())
        })
        .await;
    let bounds = shell.workspace.read_with(cx, |workspace, _| {
        workspace.repositories_pane().row_bounds(&key_d)
    });
    assert_eq!(bounds, None, "no stale bounds");

    // A reload that drops the repository of the open menu closes it (P29).
    let second = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    right_click(&shell, "c", cx);
    assert_eq!(open_root(&shell, cx), Some(key_c.clone()));
    remove_registration(&second, &c);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    assert_eq!(open_root(&shell, cx), None, "it closed with its row");
    assert!(!shell.present("popup-menu", cx));

    // The kept row alone: the list takes focus, and `shift-f10` opens its
    // menu, with Retry and Remove from Manyhands… (decision 22).
    remove_registration(&second, &a);
    remove_registration(&second, &b);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    assert_eq!(shell.rows("repo-row", cx), ["a"]);
    assert_eq!(state(&shell, "a", cx).as_deref(), Some(CANT_OPEN));
    shell.keys("secondary-1", cx);
    assert!(focused(&shell, &list, cx), "the kept row's list has focus");
    shell.keys("shift-f10", cx);
    assert_eq!(open_root(&shell, cx), Some(key_a.clone()));
    let kept = labels(&shell, cx);
    assert_eq!(kept[0], kept_sentence("a"));
    assert!(kept.iter().any(|label| label == RETRY));
    assert!(kept.iter().any(|label| label == REMOVE_FROM_MANYHANDS));
    assert!(press_and_hold(&shell, "escape", &list, cx), "Esc: the list");

    // Every poll after the launch polls got the stand-in, never the
    // library's call. They are `b`'s, which the test's own answers to `b`
    // start (two in every run seen); at least one, so the stand-in is
    // shown to be used, and bounded, so a slower interleaving still passes.
    let stood_in = backend.unanswered_roots();
    assert!(
        !stood_in.is_empty() && stood_in.len() <= 4 && stood_in.iter().all(|root| *root == key_b),
        "stand-ins: {stood_in:?}"
    );
}
