//! The Repositories panel's rows and list states (Cycle 03a, Part C), over a
//! real window, real repositories and the polling double: every poll is
//! answered by the double (`answer_poll`) and every job by the delaying
//! backend (`answer_job`), so nothing reaches a transport.

use std::fs;

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::repository::{
    AuthoringKind, LeaseKind, OperationId, RelationshipWrite, RepositoryListDto, RepositoryService,
};

use super::{
    ADD_OR_CREATE, LIST_INCOMPLETE, LOADING_REPOSITORIES, NO_REPOSITORIES, NOT_UPDATED_BUSY,
};
use crate::desktop::{
    model::{
        layout::PanelKind,
        list_state::ListState,
        navigation::{NotReloaded, Resolution},
        polling::{Answer, Clock as _, PollAttempt},
        repository_state::{CANT_OPEN, UP_TO_DATE, to_share},
        stamp::SettingsPage,
    },
    services::{JobKind, ReadKind, poll_convert::Library, requests::Intent},
    test_support::fixtures::{enable_through, remove_registration},
    view::{
        dock::tests::support::panel_count,
        harness::{Shell, key},
        polling_window::PollingWindow,
        settings::Shown,
        sidebar::SELECTED,
    },
};

#[path = "repositories_tests/menu.rs"]
mod menu;
#[path = "repositories_tests/support.rs"]
pub(in crate::desktop::view) mod support;
#[path = "repositories_tests/titles.rs"]
mod titles;
#[path = "repositories_tests/toolbar.rs"]
mod toolbar;

use support::*;

/// P5, P8, P9, P16, P18, P33, P38: the list states of the repositories
/// read, each with its action; rows with states from the schedule, which a
/// finished share and save move at once; the kept row, and the same
/// repository registered again resolving afresh; choosing by click and
/// Enter; the rows after the frame.
#[gpui_kit::test]
async fn rows_and_list_states(cx: &mut TestAppContext) {
    let fixture = crate::desktop::test_support::fixtures::Fixture::new();
    let degraded = fixture.empty_directory("degraded-data");
    fs::write(degraded.join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let (window, held) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| {
            backend
                .inner()
                .replace_service(RepositoryService::open_at(&degraded).unwrap());
            // Every poll no answer is queued for (the re-registered `a`'s
            // restart after its busy answer) is not due.
            backend.answer_unanswered_polls(not_due(clock));
            backend.inner().hold_read(ReadKind::Repositories)
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
    let restore = |shell: &Shell| {
        shell
            .backend
            .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap())
    };
    let alert = || child("repositories-alert", "action");

    // The first load: skeleton rows, and no alert.
    assert_eq!(
        shell.label("repositories-loading", cx).as_deref(),
        Some(LOADING_REPOSITORIES)
    );
    assert!(!shell.present("repositories-alert", cx));

    // It fails, with no rows: the failure's message, and Retry.
    let _ = held.send(());
    shell.run(cx);
    let failed = shell.workspace.read_with(cx, |workspace, _| {
        match workspace.navigation().repositories() {
            ListState::Failed(failure) => failure.code.message(),
            other => panic!("the first load did not fail: {:?}", other.label()),
        }
    });
    assert_eq!(
        shell.label("repositories-alert", cx).as_deref(),
        Some(failed)
    );
    assert!(!shell.present("repositories-loading", cx));
    assert!(!shell.present(("repo-row", 0usize), cx));

    // Retry reads again, and the first load stays busy: the busy alert.
    let a = shell.fixture.create_repository("a");
    restore(&shell);
    let lease = RepositoryService::hold_lease_for_testing(
        &a,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    let before = reads(&shell, ReadKind::Repositories);
    shell.click(alert(), cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    shell.run(cx);
    assert_eq!(reads(&shell, ReadKind::Repositories), before + 1);
    assert_eq!(
        shell.label("repositories-alert", cx).as_deref(),
        Some(NOT_UPDATED_BUSY)
    );
    assert!(!shell.present(("repo-row", 0usize), cx));

    // With no repositories: the empty state, whose action opens Settings ›
    // Repositories.
    drop(lease);
    let second = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    remove_registration(&second, &a);
    shell.click(alert(), cx);
    assert_eq!(
        shell.label("repositories-empty", cx).as_deref(),
        Some(NO_REPOSITORIES)
    );
    assert!(!shell.present("repositories-alert", cx));
    assert_eq!(
        shell
            .label(child("repositories-empty", "action"), cx)
            .as_deref(),
        Some(ADD_OR_CREATE)
    );
    shell.click(child("repositories-empty", "action"), cx);
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx)),
        Shown::Settings(SettingsPage::Repositories)
    );

    // Two repositories with a shared copy, and polling on: each launch poll
    // is answered, and sets no list.
    enable_through(&second, &a);
    let b = shell.fixture.create_repository("b");
    for root in [&a, &b] {
        shell.fixture.publish_over_ssh(root);
        shell.fixture.refresh(root);
    }
    backend.answer_poll(completed(&clock));
    backend.answer_poll(completed(&clock));
    let services = services.with_polling();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let (key_a, key_b) = (key(&a), key(&b));
    let polled = |cx: &mut TestAppContext| {
        services.scheduler().read_with(cx, |scheduler, _| {
            [&key_a, &key_b].into_iter().all(|root| {
                scheduler
                    .schedule()
                    .repo(root)
                    .is_some_and(|repo| repo.last.is_some() && !repo.in_flight())
            })
        })
    };
    shell
        .until_drawn(cx, "both launch polls", |_, cx| polled(cx))
        .await;
    assert_eq!(shell.rows("repo-row", cx).len(), 2);
    assert_eq!(
        panel_count(&shell, PanelKind::Repositories, cx).as_deref(),
        Some("2")
    );

    // A completed check of each: two items to share in `a`; main only,
    // shared, in `b`. The pane follows once the frame has run (P16).
    let now = clock.now();
    for (root, targets) in [
        (&key_a, vec![main_current(), item(FIRST), item(SECOND)]),
        (&key_b, vec![main_current()]),
    ] {
        let attempt = PollAttempt {
            root: root.clone(),
            operation: OperationId::new(),
            answer: Answer::Completed,
            completed_at: Some(now.utc),
            next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
            targets,
            index_pending: false,
        };
        services.scheduler().update(cx, |scheduler, cx| {
            scheduler.schedule_mut().after(&now, &attempt);
            cx.notify();
        });
    }
    shell.run(cx);
    assert!(pane_follows(&shell, cx), "the rows follow the model");
    assert_eq!(state(&shell, "a", cx), Some(to_share(2)));
    assert_eq!(state(&shell, "b", cx), Some(UP_TO_DATE.to_owned()));

    // A click chooses a row, which carries the mark; the row claims no
    // selection. The arrows move the Kit cursor only; Enter chooses.
    let (at_a, at_b) = (index(&shell, "a", cx), index(&shell, "b", cx));
    let claims = |shell: &Shell, at: usize, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| window.find(("repo-row", at)).selected())
    };
    shell.click(("repo-row", at_a), cx);
    let chosen = |cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            workspace.navigation().selected().cloned()
        })
    };
    assert_eq!(chosen(cx), Some(key_a.clone()));
    assert_eq!(
        shell.label(("repo-selected", at_a), cx).as_deref(),
        Some(SELECTED)
    );
    assert!(!shell.present(("repo-selected", at_b), cx));
    assert_eq!(claims(&shell, at_a, cx), None);
    shell.keys("secondary-1", cx);
    let arrow = if at_b > at_a { "down" } else { "up" };
    shell.keys(arrow, cx);
    assert_eq!(chosen(cx), Some(key_a.clone()), "moving chooses nothing");
    assert_eq!(claims(&shell, at_b, cx), None, "the cursor's row");
    shell.keys("enter", cx);
    assert_eq!(chosen(cx), Some(key_b.clone()));
    assert_eq!(
        shell.label(("repo-selected", at_b), cx).as_deref(),
        Some(SELECTED)
    );
    assert!(!shell.present(("repo-selected", at_a), cx));
    assert_eq!(claims(&shell, at_b, cx), None, "the chosen row");

    // A finished share that published one item drops the count (P5); a
    // save of it that made a checkpoint and did not publish adds it back
    // (P33). Each finished request starts one list read: it is held, and
    // the row already reads the new count while it is, so the count comes
    // from the request, not from a read; once the read lands the count is
    // the same (it moved once, and the reload does not reset it). No check
    // runs meanwhile.
    let polls = backend.polled_roots().len();
    let listed = reads(&shell, ReadKind::Repositories);
    let held = shell.backend.hold_read(ReadKind::Repositories);
    backend
        .inner()
        .answer_job(JobKind::SynchronizeItem, finished("item sync", FIRST, true));
    let sync = Intent::ItemSync {
        root: a.clone(),
        kind: AuthoringKind::Ticket,
        id: id(FIRST),
    };
    finish(&shell, sync, cx).await;
    assert_eq!(
        reads(&shell, ReadKind::Repositories),
        listed + 1,
        "the share's one list read, held"
    );
    assert_eq!(state(&shell, "a", cx), Some(to_share(1)), "while held");
    let _ = held.send(());
    shell.run(cx);
    assert_eq!(state(&shell, "a", cx), Some(to_share(1)), "after the read");
    let listed = reads(&shell, ReadKind::Repositories);
    let held = shell.backend.hold_read(ReadKind::Repositories);
    backend
        .inner()
        .answer_job(JobKind::TicketSave, finished("ticket save", FIRST, false));
    let save = Intent::TicketSave {
        root: a.clone(),
        id: id(FIRST),
        title: "First".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        body: String::new(),
        deps: RelationshipWrite::Unchanged,
        parent: RelationshipWrite::Unchanged,
        observation: String::new(),
    };
    finish(&shell, save, cx).await;
    assert_eq!(
        reads(&shell, ReadKind::Repositories),
        listed + 1,
        "the save's one list read, held"
    );
    assert_eq!(state(&shell, "a", cx), Some(to_share(2)), "while held");
    let _ = held.send(());
    shell.run(cx);
    assert_eq!(state(&shell, "a", cx), Some(to_share(2)), "after the read");
    assert_eq!(backend.polled_roots().len(), polls, "no check");

    // An incomplete list keeps its rows under its alert; Retry reads again.
    let incomplete = shell
        .workspace
        .read_with(cx, |workspace, _| RepositoryListDto {
            items: workspace
                .navigation()
                .repositories()
                .rows()
                .unwrap()
                .iter()
                .map(|entry| entry.summary.clone())
                .collect(),
            complete: false,
        });
    shell.workspace.update(cx, |workspace, cx| {
        workspace.apply_repository_list(incomplete, cx)
    });
    shell.run(cx);
    assert_eq!(
        shell.label("repositories-alert", cx).as_deref(),
        Some(LIST_INCOMPLETE)
    );
    assert_eq!(shell.rows("repo-row", cx).len(), 2);
    let before = reads(&shell, ReadKind::Repositories);
    shell.click(alert(), cx);
    assert_eq!(reads(&shell, ReadKind::Repositories), before + 1);
    assert!(!shell.present("repositories-alert", cx));

    // A reload that stays busy keeps the rows under the busy alert.
    let lease = RepositoryService::hold_lease_for_testing(
        &b,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    shell.run(cx);
    assert_eq!(
        shell.label("repositories-alert", cx).as_deref(),
        Some(NOT_UPDATED_BUSY)
    );
    assert_eq!(shell.rows("repo-row", cx).len(), 2);
    drop(lease);
    shell.click(alert(), cx);
    assert!(!shell.present("repositories-alert", cx));

    // A reload that fails keeps the rows under the failure's message.
    shell
        .backend
        .replace_service(RepositoryService::open_at(&degraded).unwrap());
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    let failed = shell.workspace.read_with(cx, |workspace, _| {
        match workspace.navigation().not_reloaded() {
            Some(NotReloaded::Failed(failure)) => failure.code.message(),
            other => panic!("the reload did not fail: {other:?}"),
        }
    });
    assert_eq!(
        shell.label("repositories-alert", cx).as_deref(),
        Some(failed)
    );
    assert_eq!(shell.rows("repo-row", cx).len(), 2);
    restore(&shell);
    shell.click(alert(), cx);
    assert!(!shell.present("repositories-alert", cx));

    // The chosen repository's registration is removed by a second actor:
    // the reload keeps it as a row of its own, last, reading Can't open,
    // and the header counts it (P18, R-20).
    shell.click(("repo-row", index(&shell, "a", cx)), cx);
    assert_eq!(chosen(cx), Some(key_a.clone()));
    remove_registration(&second, &a);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    assert_eq!(shell.rows("repo-row", cx), ["b", "a"]);
    assert_eq!(
        shell.label(("repo-state", 1usize), cx).as_deref(),
        Some(CANT_OPEN)
    );
    assert_eq!(
        shell.label(("repo-selected", 1usize), cx).as_deref(),
        Some(SELECTED)
    );
    assert_eq!(
        panel_count(&shell, PanelKind::Repositories, cx).as_deref(),
        Some("2")
    );

    // Registered again: the reload sends a resolve read, and Can't open
    // goes once it lands (P38). Its schedule's poll is answered as busy, so
    // it has no completed check.
    backend.answer_poll(Library::Busy);
    enable_through(&second, &a);
    let resolves = reads(&shell, ReadKind::Resolve);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    assert!(
        reads(&shell, ReadKind::Resolve) > resolves,
        "resolved again"
    );
    assert!(matches!(
        shell.workspace.read_with(cx, |workspace, _| workspace
            .navigation()
            .selected_entry()
            .map(|entry| entry.resolution.clone())),
        Some(Resolution::Resolved(_))
    ));
    assert_ne!(state(&shell, "a", cx).as_deref(), Some(CANT_OPEN));
    assert_eq!(shell.rows("repo-row", cx).len(), 2);
    assert_eq!(
        panel_count(&shell, PanelKind::Repositories, cx).as_deref(),
        Some("2")
    );

    // A save in a scheduled repository with no completed check yet counts
    // at once (P44).
    let scheduled = |cx: &mut TestAppContext| {
        services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(&key_a)
                .is_some_and(|repo| repo.unshared.as_of.is_none() && !repo.in_flight())
        })
    };
    shell
        .until_drawn(cx, "a's new schedule", |_, cx| scheduled(cx))
        .await;
    backend.inner().answer_job(
        JobKind::DocumentSave,
        finished("document save", THIRD, false),
    );
    let save = Intent::DocumentSave {
        root: a.clone(),
        id: id(THIRD),
        title: "Third".to_owned(),
        body: String::new(),
        observation: None,
    };
    finish(&shell, save, cx).await;
    assert_eq!(state(&shell, "a", cx), Some(to_share(1)));
}

/// P6, P36: what a row names of what is not shared. A title the loaded
/// Tickets or Documents list gives costs no read; any other is read once
/// per repository, held, answered, and named after; a busy read is read
/// again at the next notify and, after three, the item is named by its ID
/// for good; notifies while a read is held start nothing and use no tries;
/// an item with no kind is never read; of seven, five are read.
#[gpui_kit::test]
async fn titles_of_what_is_not_shared(cx: &mut TestAppContext) {
    titles::journey(cx).await;
}

/// P10-P12, P21, P22, P26, P27, P29, P40: a row's menu by a right click,
/// for that row and not the chosen one, and by `shift-f10` and `menu`, for
/// the Kit cursor's row, the kept row alone included; its first line;
/// what each item sends or opens, by label; a choice made after its row
/// changed sends nothing; Remove's dialog and focus; refused behind a
/// dialog and the details sheet; nothing read by opening, moving or Esc;
/// the menu closing when its row goes.
#[gpui_kit::test]
async fn the_menu_by_pointer_and_keyboard(cx: &mut TestAppContext) {
    menu::journey(cx).await;
}

/// P23, P24: Check all sends `PollOnce` for the ordinary and the paused
/// repository, in list order, and none for a locked one or one whose stored
/// outcome is `UnlockRequired`; nothing else is sent; it takes nothing while
/// those run; focus moves to the list once it disables itself; refused
/// behind a dialog and the sheet.
#[gpui_kit::test]
async fn check_all_checks_each_repository(cx: &mut TestAppContext) {
    toolbar::check_all(cx).await;
}

/// P11, P13, R-35, R-40: the More menu opens for the chosen repository
/// whatever row the Kit cursor rests on; Rebuild the search index, Esc and
/// Remove from Manyhands… leave focus on the More button; a choice made for
/// another repository sends nothing.
#[gpui_kit::test]
async fn the_more_menu_acts_on_the_chosen_repository(cx: &mut TestAppContext) {
    toolbar::more_menu(cx).await;
}
