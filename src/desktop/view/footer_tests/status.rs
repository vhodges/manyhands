//! `status_items_say_and_choose` (Cycle 03a, Part G): the footer's Update
//! check and To share items. `status_table` pins the pure functions that
//! decide what they say and choose; `journey` is the window with polling on, two
//! real repositories, every poll answered by the double and every check by
//! the delaying backend, so nothing reaches a transport.
//!
//! A tooltip cannot be read from the snapshot, so names are read through the
//! wrapper's accessible name, which is the same text. Focus after a press is
//! read in the update that causes it; where focus moves at the draw (R-38),
//! no region is among the answers, so a read after the draw cannot be
//! satisfied by `Workspace::focus_lost`'s fallback.

use gpui_kit::{FocusHandle, TestAppContext, component::WindowExt};
use manyhands::{repository::ItemDtoKind, results::Outcome};
use time::OffsetDateTime;

use super::support::*;
use crate::desktop::{
    model::{
        layout::{Layout, PanelKind},
        navigation::OpenTarget,
        polling::{PollSchedule, TargetKind, TestClock},
        stamp::{RepoKey, SettingsPage},
    },
    services::{JobKind, Services, requests::Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        footer::Control,
        harness::{Shell, key},
        polling_window::PollingWindow,
        repositories::tests::support::{
            FIRST, SECOND, THIRD, checked, completed, id, item, not_due, polled,
            press_released_and_hold,
        },
        settings::Shown,
    },
};

use PanelKind::{Repositories, Tickets};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5TC0";

fn handle(shell: &Shell, control: Control, cx: &mut TestAppContext) -> FocusHandle {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(control))
}

fn focus(shell: &Shell, control: Control, cx: &mut TestAppContext) {
    let handle = handle(shell, control, cx);
    shell.frame(cx, |window, cx| handle.focus(window, cx));
    shell.run(cx);
}

/// Tab, from the item area, to `control`.
fn tab_to(shell: &Shell, control: Control, cx: &mut TestAppContext) {
    shell.keys("secondary-3", cx);
    for _ in 0..40 {
        if focused(shell, control.id(), cx) {
            return;
        }
        shell.keys("tab", cx);
    }
    panic!("Tab never reached {control:?}");
}

/// A control as the screen reader reads it: its name.
fn said(shell: &Shell, control: Control, cx: &mut TestAppContext) -> Option<String> {
    read(shell, control, cx).map(|(name, _)| name)
}

fn chosen(shell: &Shell, cx: &mut TestAppContext) -> Option<RepoKey> {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.navigation().selected().cloned()
    })
}

fn schedule(
    shell: &Shell,
    services: &Services,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut PollSchedule),
) {
    services.scheduler().update(cx, |scheduler, cx| {
        f(scheduler.schedule_mut());
        cx.notify();
    });
    shell.run(cx);
}

/// What a press can change: the chosen repository, the layout and its saves,
/// the reads and requests sent.
fn footprint(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> (Option<RepoKey>, Layout, usize, usize, usize) {
    (
        chosen(shell, cx),
        layout(shell, cx),
        saves(shell, cx),
        shell.backend.reads_in_order().len(),
        shell.requests(cx, |_| true).len(),
    )
}

pub(super) async fn journey(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (a, b) = (
        fixture.create_repository("a"),
        fixture.create_repository("b"),
    );
    fixture.commit_ticket(&a, TICKET, "Ticket", "");
    for root in [&a, &b] {
        fixture.publish_over_ssh(root);
        fixture.refresh(root);
    }
    let (window, ()) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| {
            // The two launch polls complete; every later poll is not due.
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
    let (update, share) = (Control::UpdateCheck, Control::ToShare);

    // Nothing scheduled: neither item is shown.
    assert_eq!(said(&shell, update, cx), None);
    assert_eq!(said(&shell, share, cx), None);

    let services = services.with_polling();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let [key_a, key_b] = [&a, &b].map(|root| key(root));
    let polled_once = |root: &RepoKey, cx: &mut TestAppContext| {
        services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(root)
                .is_some_and(|repo| repo.last.is_some() && !repo.in_flight())
        })
    };
    shell
        .until_drawn(cx, "the launch polls", |_, cx| {
            polled_once(&key_a, cx) && polled_once(&key_b, cx)
        })
        .await;
    assert_eq!(shell.rows("repo-row", cx), ["a", "b"], "list order");

    // Checks done, none running, no work to share: Update check says when,
    // To share is hidden (P15).
    let checked_text = "Checked for updates just now";
    assert_eq!(said(&shell, update, cx).as_deref(), Some(checked_text));
    assert_eq!(said(&shell, share, cx), None, "hidden at 0");

    // A check running: queued by the schedule (`in_flight`), and one the
    // user asked for (`checking_now`).
    schedule(&shell, &services, cx, |schedule| {
        schedule.mark_queued(&key_a)
    });
    let checking = "Checking for updates\u{2026}";
    assert_eq!(said(&shell, update, cx).as_deref(), Some(checking));
    // Its answer ends it: a completed check that found nothing to share.
    checked(&services, &clock, &key_a, vec![], cx);
    shell.run(cx);
    assert_eq!(said(&shell, update, cx).as_deref(), Some(checked_text));
    let held = backend.inner().hold_job(JobKind::PollOnce);
    backend.inner().answer_job(JobKind::PollOnce, polled());
    assert!(services.check_now(&a), "the check was sent");
    shell.run(cx);
    assert_eq!(said(&shell, update, cx).as_deref(), Some(checking));
    drop(held);
    let sent = shell.requests(
        cx,
        |request| matches!(&request.intent, Intent::PollOnce { root } if root == &a),
    );
    shell.until_ended(sent[0], cx).await;
    // The answer is applied a frame after its request ends.
    shell
        .until_drawn(cx, "the check's answer to end it", |shell, cx| {
            said(shell, update, cx).as_deref() == Some(checked_text)
        })
        .await;
    assert_eq!(said(&shell, update, cx).as_deref(), Some(checked_text));

    // Work in two repositories: `a` has two items, `b` one. `a` is chosen,
    // so To share names `b`.
    shell.select(&a, cx);
    checked(
        &services,
        &clock,
        &key_a,
        vec![item(FIRST), item(SECOND)],
        cx,
    );
    checked(&services, &clock, &key_b, vec![item(THIRD)], cx);
    shell.run(cx);
    assert_eq!(
        said(&shell, share, cx).as_deref(),
        Some("3 to share. Select b.")
    );
    assert_eq!(
        shell.label((share.id(), 0usize), cx).as_deref(),
        Some("3 to share")
    );

    // Tab reaches both, Update check first (P15).
    tab_to(&shell, update, cx);
    shell.keys("tab", cx);
    assert!(focused(&shell, share.id(), cx), "Tab reaches To share");

    // Enter chooses the next repository with work each time, wrapping, and
    // focus stays on the control, read in the update that presses (P15).
    let to_share = handle(&shell, share, cx);
    for (expected, then) in [
        (&key_b, "3 to share. Select a."),
        (&key_a, "3 to share. Select b."),
        (&key_b, "3 to share. Select a."),
        (&key_a, "3 to share. Select b."),
    ] {
        assert!(
            press_released_and_hold(&shell, "enter", &to_share, cx),
            "focus stays"
        );
        assert_eq!(chosen(&shell, cx).as_ref(), Some(expected));
        assert_eq!(said(&shell, share, cx).as_deref(), Some(then));
    }

    // Behind a dialog and behind the sheet both are refused (P12): the
    // chosen repository, the layout and the reads stay, and Settings does not
    // open.
    let target = OpenTarget::Item {
        repo: key_a.clone(),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    let before = footprint(&shell, cx);
    for sheet in [false, true] {
        open_modal(&shell, sheet, cx);
        for control in [update, share] {
            shell.frame(cx, |window, cx| {
                shell
                    .workspace
                    .update(cx, |workspace, cx| workspace.press(control, window, cx))
            });
            shell.run(cx);
        }
        assert_eq!(footprint(&shell, cx), before, "refused, sheet {sheet}");
        let shown = shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
        assert_eq!(shown, Shown::Tabs, "Settings did not open, sheet {sheet}");
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }

    // With Repositories hidden, choosing keeps it hidden: the layout is as
    // it was (decision 23).
    shell.click(Control::Panel(Repositories).id(), cx);
    let (hidden_layout, saved) = (layout(&shell, cx), saves(&shell, cx));
    assert!(!hidden_layout.is_displayed(Repositories));
    focus(&shell, share, cx);
    assert!(press_released_and_hold(&shell, "enter", &to_share, cx));
    assert_eq!(chosen(&shell, cx).as_ref(), Some(&key_b), "it chose b");
    assert_eq!(layout(&shell, cx), hidden_layout, "the layout is unchanged");
    assert_eq!(saves(&shell, cx), saved, "and nothing was saved");
    shell.click(Control::Panel(Repositories).id(), cx);
    shell.select(&a, cx);

    // Only the chosen repository has work: `b`'s item is shared, in the very
    // update that presses To share, before any draw: the footer still names
    // `b`, but the choice is made when pressed, so it chooses nothing (R-39).
    assert_eq!(
        said(&shell, share, cx).as_deref(),
        Some("3 to share. Select b.")
    );
    let before = footprint(&shell, cx);
    shell.frame(cx, |window, cx| {
        services.scheduler().update(cx, |scheduler, _| {
            let at = clock_now(&clock);
            scheduler
                .schedule_mut()
                .shared(&key_b, Some(&id(THIRD)), false, at);
        });
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.press(share, window, cx));
    });
    shell.run(cx);
    assert_eq!(footprint(&shell, cx), before, "it chose nothing");
    // The name now says "in a".
    assert_eq!(said(&shell, share, cx).as_deref(), Some("2 to share in a."));
    focus(&shell, share, cx);
    let before = footprint(&shell, cx);
    assert!(press_released_and_hold(&shell, "enter", &to_share, cx));
    assert_eq!(
        footprint(&shell, cx),
        before,
        "nothing changed, nothing read"
    );
    assert_eq!(said(&shell, share, cx).as_deref(), Some("2 to share in a."));

    // A share that brings the count to 0 while To share has focus: Needs
    // attention is absent, so focus moves to the right dock group's first
    // control. Read after the draw: only the move can put it on a footer
    // control (R-38).
    focus(&shell, share, cx);
    assert!(focused(&shell, share.id(), cx));
    schedule(&shell, &services, cx, |schedule| {
        for item in [FIRST, SECOND] {
            schedule.shared(&key_a, Some(&id(item)), false, clock_now(&clock));
        }
    });
    assert_eq!(said(&shell, share, cx), None, "hidden at 0");
    assert!(
        focused(&shell, Control::Panel(Tickets).id(), cx),
        "focus moved to the next shown control"
    );

    // With Needs attention shown, it is the one that takes focus.
    finish(&shell, 1, Outcome::Error, cx);
    schedule(&shell, &services, cx, |schedule| {
        schedule.saved(
            &key_a,
            id(FIRST),
            Some(TargetKind::Ticket),
            false,
            clock_now(&clock),
        );
    });
    assert_eq!(said(&shell, share, cx).as_deref(), Some("1 to share in a."));
    focus(&shell, share, cx);
    schedule(&shell, &services, cx, |schedule| {
        schedule.shared(&key_a, Some(&id(FIRST)), false, clock_now(&clock));
    });
    assert_eq!(said(&shell, share, cx), None);
    assert!(
        focused(&shell, Control::Attention.id(), cx),
        "focus moved to Needs attention"
    );

    // Two folders of the same name: the name carries the parent. `sub/a`
    // has a schedule and no completed check, and one save counts at once
    // (P41, P44).
    shell.fixture.empty_directory("sub");
    let sub = shell.fixture.create_repository("sub/a");
    shell.fixture.publish_over_ssh(&sub);
    shell.fixture.refresh(&sub);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let key_sub = key(&sub);
    shell
        .until_drawn(cx, "sub/a scheduled", |_, cx| {
            services.scheduler().read_with(cx, |scheduler, _| {
                scheduler.schedule().repo(&key_sub).is_some()
            })
        })
        .await;
    services.scheduler().read_with(cx, |scheduler, _| {
        let repo = scheduler.schedule().repo(&key_sub).expect("scheduled");
        assert!(repo.last.is_none() && repo.last_completed.is_none());
    });
    schedule(&shell, &services, cx, |schedule| {
        schedule.saved(
            &key_sub,
            id(THIRD),
            Some(TargetKind::Ticket),
            false,
            clock_now(&clock),
        );
    });
    assert_eq!(
        said(&shell, share, cx).as_deref(),
        Some("1 to share. Select a in sub.")
    );
    focus(&shell, share, cx);
    assert!(press_released_and_hold(&shell, "enter", &to_share, cx));
    assert_eq!(chosen(&shell, cx), Some(key_sub.clone()));
    assert_eq!(
        said(&shell, share, cx).as_deref(),
        Some("1 to share in a in sub.")
    );

    // Update check opens Settings, Checking for updates.
    focus(&shell, update, cx);
    shell.press("enter", cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert_eq!(shown, Shown::Settings(SettingsPage::Polling));
}

fn clock_now(clock: &TestClock) -> OffsetDateTime {
    use crate::desktop::model::polling::Clock as _;
    clock.utc()
}
