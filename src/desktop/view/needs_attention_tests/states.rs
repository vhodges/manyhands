//! P23, S4 and S5: each button a repository's state row can carry, and a
//! problem item's Open, run by keyboard from the popover. Each state is made
//! the way a person meets it: the scheduler's own transitions
//! (`schedule_mut()`: a check that could not reach, a repository held until
//! recovery), the polling double's stored outcome, a folder that is away or
//! a configuration that is invalid. Nothing polls unless a case starts it, and
//! every poll that is made is answered by the double. Each press asserts
//! what the action did and that the popover closed.

use std::fs;

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, OperationId, PollingOutcome, RepositoryService},
};

use super::{actions::press_from_popover, *};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction, RowKey, Target},
        navigation::Resolution,
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind, TransportFailure,
            status::Unplaced,
        },
        repository_state::RowAction,
        stamp::SettingsPage,
        tabs::TabKey,
    },
    services::requests::Intent,
    test_support::fixtures::{Fixture, refresh_through},
    view::{
        activity_support::{refresh, row},
        harness::{Shell, key},
        polling_window::PollingWindow,
        repositories::tests::support::not_due,
        settings::{Shown, polling_support::policy},
    },
};

/// A window over `fixture`'s repositories with no polling: the scheduler
/// holds what a case puts there. Every poll that is made is not due.
fn window(fixture: Fixture, cx: &mut TestAppContext) -> PollingWindow {
    let (window, ()) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| backend.answer_unanswered_polls(not_due(clock)),
        cx,
    );
    window
        .shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    window.shell.run(cx);
    window
}

/// `root` joins the schedule: enabled, unpaused, every 300 s.
fn scheduled(window: &PollingWindow, root: &str, cx: &mut TestAppContext) {
    let root = root.to_owned();
    window.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().add(root, policy(), None);
        cx.notify();
    });
    window.shell.run(cx);
}

/// The repository state row of `root` carries the button `label`, whose
/// action is `RowAction`'s, and the log counts it.
fn state_row(
    shell: &Shell,
    root: &str,
    label: &str,
    check: impl Fn(&RowAction) -> bool,
    cx: &mut TestAppContext,
) -> RowKey {
    refresh(shell, cx);
    let at = RowKey::Repository(root.to_owned());
    let found = row(shell, &at, cx).unwrap_or_else(|| {
        panic!(
            "no state row for {root}: {:?}",
            keys_of(&crate::desktop::view::activity_support::rows(shell, cx))
        )
    });
    assert!(found.row.attention, "{label}: the state needs attention");
    let button = found.row.button.expect("the state has a button");
    assert_eq!(button.label, label);
    assert!(
        matches!(&button.action, ActivityAction::Repository(_, action) if check(action)),
        "{label}: the row's action is the state's: {:?}",
        button.action
    );
    at
}

/// The checks the popover's buttons sent.
fn checks(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .requests(cx, |request| {
            matches!(request.intent, Intent::PollOnce { .. })
        })
        .len()
}

/// P23, S4: Retry now (Can't reach) and Check now (a stored outcome), each a
/// check sent; Resume checking (a repository held, no reason in its phase:
/// the state's own `Attend`, not S3's hold); and Set up sharing, which is
/// information the popover never lists.
#[gpui_kit::test]
async fn popover_actions_states(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cant_reach(cx);
    unshared(cx).await;
    stored(cx).await;
    held(cx);
}

/// P23, S4: Reread (a repository the scheduler could not read), Retry (one
/// that cannot be opened) and Settings… (one that needs fixing).
#[gpui_kit::test]
async fn popover_actions_resolution(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    unreadable(cx).await;
    cant_open(cx);
    needs_fixing(cx);
}

/// Retry now on a repository that could not be reached.
fn cant_reach(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let window = window(fixture, cx);
    let key_a = key(&a);
    scheduled(&window, &key_a, cx);
    let now = window.clock.now();
    let attempt = PollAttempt {
        root: key_a.clone(),
        operation: OperationId::new(),
        answer: Answer::Transport(TransportFailure::Unreachable),
        completed_at: None,
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: Vec::new(),
        index_pending: false,
    };
    window.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    let shell = &window.shell;
    shell.run(cx);
    let at = state_row(
        shell,
        &key_a,
        "Retry now",
        |a| *a == RowAction::RetryNow,
        cx,
    );
    let before = checks(shell, cx);
    press_from_popover(shell, &at, "Retry now", cx);
    assert_eq!(checks(shell, cx), before + 1, "Retry now sent a check");
}

/// A repository with no shared copy has a state row whose Set up sharing… is
/// information, no need for attention: the popover never lists it, though
/// another row needs the person and opens it.
async fn unshared(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let b = fixture.create_repository("b");
    fixture.refresh(&b);
    let window = window(fixture, cx);
    let _services = window.services.clone().with_polling();
    let shell = &window.shell;
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let at = RowKey::Repository(key(&b));
    shell
        .until_drawn(cx, "its state row", |shell, cx| {
            refresh(shell, cx);
            row(shell, &at, cx).is_some()
        })
        .await;
    let info = row(shell, &at, cx).expect("b's state row");
    assert!(!info.row.attention, "no shared copy is information");
    assert!(
        info.row
            .button
            .as_ref()
            .is_some_and(|button| button.label.starts_with("Set up sharing")),
        "it offers Set up sharing: {:?}",
        info.row.button
    );
    crate::desktop::view::footer::tests::support::finish(
        shell,
        1,
        manyhands::results::Outcome::Error,
        cx,
    );
    refresh(shell, cx);
    open_by_click(shell, cx);
    assert!(is_open(shell, cx), "a failed request opens it");
    assert!(
        !keys_of(&applied(shell, cx)).contains(&at),
        "and it does not list the state row"
    );
}

/// Check now, on a repository whose stored outcome says a key is locked.
async fn stored(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let (window, ()) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| {
            backend.store_outcome(&key(&a), PollingOutcome::UnlockRequired);
            backend.answer_unanswered_polls(not_due(clock));
        },
        cx,
    );
    let shell = &window.shell;
    // Polling on: the scheduler reads the stored outcome, and every poll
    // it makes is not due.
    let _services = window.services.clone().with_polling();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let wanted = RowKey::Repository(key(&a));
    shell
        .until_drawn(cx, "the stored outcome's row", |shell, cx| {
            refresh(shell, cx);
            row(shell, &wanted, cx).is_some()
        })
        .await;
    let at = state_row(
        shell,
        &key(&a),
        "Check now",
        |a| *a == RowAction::CheckNow,
        cx,
    );
    let before = checks(shell, cx);
    press_from_popover(shell, &at, "Check now", cx);
    assert_eq!(checks(shell, cx), before + 1, "Check now sent a check");
}

/// Resume checking, on a repository held until recovery with no reason in
/// its phase: the state row's own `Attend`, which releases the hold.
fn held(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let window = window(fixture, cx);
    let root = key(&a);
    window.services.scheduler().update(cx, |scheduler, cx| {
        let held = crate::desktop::model::polling::Policy {
            recovery_suspended: true,
            ..policy()
        };
        scheduler.schedule_mut().add(root.clone(), held, None);
        cx.notify();
    });
    let shell = &window.shell;
    shell.run(cx);
    let at = state_row(
        shell,
        &root,
        "Resume checking",
        |action| matches!(action, RowAction::Attend(..)),
        cx,
    );
    press_from_popover(shell, &at, "Resume checking", cx);
    assert_eq!(window.backend.resumed(), [root], "the hold was released");
}

/// Reread, on a repository whose folder was away when the scheduler read
/// it: the registration is read again once the folder is back.
async fn unreadable(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let away = a.with_file_name("a-away");
    fs::rename(&a, &away).unwrap();
    let window = window(fixture, cx);
    let services = window.services.clone().with_polling();
    let shell = &window.shell;
    let root = key(&a);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell
        .until_drawn(cx, "the repository unread", |_, cx| {
            services
                .scheduler()
                .read_with(cx, |scheduler, _| scheduler.unplaced(&root))
                == Some(Unplaced::Unreadable)
        })
        .await;
    let at = state_row(shell, &root, "Retry", |a| *a == RowAction::Reread, cx);
    fs::rename(&away, &a).unwrap();
    let reads = |cx: &mut TestAppContext| {
        shell.run(cx);
        window
            .backend
            .remote_read_roots()
            .iter()
            .filter(|read| **read == root)
            .count()
    };
    let before = reads(cx);
    press_from_popover(shell, &at, "Retry", cx);
    shell
        .until_drawn(cx, "its registration read again", |_, cx| {
            services.scheduler().read_with(cx, |scheduler, _| {
                scheduler.schedule().repo(&root).is_some()
            })
        })
        .await;
    assert!(reads(cx) > before, "Reread read the registration again");
}

/// Retry, on a repository whose folder is away: it is resolved again once
/// the folder is back.
fn cant_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.refresh(&a);
    let window = window(fixture, cx);
    let shell = &window.shell;
    let root = key(&a);
    shell.select(&a, cx);
    let away = a.with_file_name("a-away");
    fs::rename(&a, &away).unwrap();
    shell.workspace.update(cx, |workspace, cx| {
        workspace.retry_resolution(root.clone(), cx)
    });
    shell.run(cx);
    let resolved = |shell: &Shell, cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            workspace
                .navigation()
                .entry(&key(&a))
                .is_some_and(|entry| matches!(entry.resolution, Resolution::Resolved(_)))
        })
    };
    assert!(!resolved(shell, cx), "the folder is away");
    let at = state_row(shell, &root, "Retry", |a| *a == RowAction::Retry, cx);
    fs::rename(&away, &a).unwrap();
    press_from_popover(shell, &at, "Retry", cx);
    shell.run(cx);
    assert!(resolved(shell, cx), "Retry resolved the repository again");
}

/// Settings…, on a repository whose configuration is invalid: Settings opens
/// on that repository.
fn needs_fixing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.refresh(&a);
    let second = RepositoryService::open_at(fixture.data_dir()).unwrap();
    fixture.commit_file(&a, ".manyhands/config.toml", b"format_version = 99\n");
    refresh_through(&second, &a);
    let window = window(fixture, cx);
    let shell = &window.shell;
    let root = key(&a);
    let at = state_row(
        shell,
        &root,
        "Settings\u{2026}",
        |a| *a == RowAction::Settings,
        cx,
    );
    press_from_popover(shell, &at, "Settings\u{2026}", cx);
    assert_eq!(
        crate::desktop::view::activity::tests::cases::settings(shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert_eq!(shell.label("managed-root", cx), Some(root));
}

/// S5: a problem item's Open takes the person to it.
#[gpui_kit::test]
fn popover_actions_problem(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let window = window(fixture, cx);
    let root = key(&a);
    scheduled(&window, &root, cx);
    let problem = ItemId::generate();
    let now = window.clock.now();
    let attempt = PollAttempt {
        root: root.clone(),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: vec![AttemptTarget {
            item_id: Some(problem.clone()),
            kind: Some(TargetKind::Ticket),
            disposition: Disposition::Dirty,
            main: false,
        }],
        index_pending: false,
    };
    window.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    let shell = &window.shell;
    shell.run(cx);
    let at = RowKey::Problem {
        root: root.clone(),
        target: Target::Item(problem.clone()),
    };
    assert!(
        keys_of(&attention_rows(shell, cx)).contains(&at),
        "a problem item needs attention"
    );
    press_from_popover(shell, &at, "Open", cx);
    let tab = TabKey::Item {
        repo: root,
        kind: ItemDtoKind::Ticket,
        id: problem.to_string(),
    };
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys());
    assert!(tabs.contains(&tab), "its tab opened");
}
