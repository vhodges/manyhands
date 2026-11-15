//! The polling journey (D6 acceptance A15, Task 13; design W-4, W-6) by
//! keystrokes alone, in the shell Cycle 01 made: Settings is opened with
//! `secondary-,` and the Checking for updates page is reached by Tab; its
//! interval is changed, its switch turned off and on, and Check now pressed;
//! then Activity, the bottom dock's panel, is reached with `secondary-4`,
//! and the arrows and Tab reach Unlock… (a locked key's row) and Retire…
//! (the offer on a closed ticket), each pressed. Focus is shown at every step: the element
//! holds focus, is on screen and its region draws the one ring (`Walk::on`).
//!
//! No click and no call into the workspace. The window is the harness's,
//! resized to the default 1280 x 840, and, like every test window, inactive.
//! Every library answer a press asks for is canned or local: nothing reaches
//! a transport (ruling R11). The repositories are `a`, whose launch poll
//! finds its key locked, and `b`, paused, whose closed ticket the poll
//! reported (seeded through the schedule's own transition, since no desktop
//! test reaches SSH).

use manyhands::{
    repository::{
        AuthoringKind, ContextRetirementState as State, RetirementRemoval as Removal, SharedKeyId,
    },
    results::{Envelope, Outcome, ResultCode},
};

use gpui_kit::TestAppContext;

use super::super::Walk;
use crate::desktop::{
    model::{
        activity::rows::RowKey,
        polling::{TargetKind, attention::UNLOCK, entry::RETIRE},
    },
    services::{
        JobKind,
        requests::{Intent, Lane},
    },
    test_support::{
        fixtures::{Fixture, item_id},
        polling::unlock_request,
        retire::answer,
    },
    view::{
        activity_support::{button_focus, refresh, row as activity_row, rows, to_row},
        polling_window::{PollingWindow, reported},
        settings::Shown,
        workspace::Region,
    },
};

use crate::desktop::model::stamp::SettingsPage;

const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";

/// How long a wait of the journey may take. The whole journey takes about
/// 5 s; a wait stuck for this long says so, not after the harness's 180 s.
const BOUND: std::time::Duration = std::time::Duration::from_secs(30);

/// Draws frames, with the worker running, until `done` holds of the window,
/// for at most `BOUND`.
async fn bounded(
    shell: &crate::desktop::view::harness::Shell,
    cx: &mut TestAppContext,
    what: &str,
    done: impl Fn(&crate::desktop::view::harness::Shell, &mut TestAppContext) -> bool,
) {
    let deadline = std::time::Instant::now() + BOUND;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if done(shell, cx) {
            shell.frame(cx, |_, _| ());
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{what} did not happen"
        );
        cx.executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
}

/// The one hold the log lists: the locked key's.
fn hold_row(walk: &Walk, cx: &mut TestAppContext) -> RowKey {
    refresh(&walk.shell, cx);
    let holds: Vec<RowKey> = rows(&walk.shell, cx)
        .into_iter()
        .map(|row| row.row.key)
        .filter(|key| matches!(key, RowKey::Hold(_)))
        .collect();
    assert_eq!(holds.len(), 1, "one hold: {holds:?}");
    holds[0].clone()
}

/// The label of `key`'s button, as the log stands.
fn button_label(walk: &Walk, key: &RowKey, cx: &mut TestAppContext) -> Option<String> {
    activity_row(&walk.shell, key, cx)?
        .row
        .button
        .map(|offer| offer.label)
}

/// A Check now answered `ok`, as the library would for a repository whose
/// poll found nothing to say.
fn checked() -> Envelope<manyhands::repository::MutationDataDto> {
    Envelope::mutation(
        "poll once",
        manyhands::results::Scope::default(),
        Outcome::Success,
        ResultCode::Ok,
    )
}

/// The requests `pick` picks, on the user lane, as ids.
fn user_sent(
    walk: &Walk,
    cx: &mut TestAppContext,
    pick: impl Fn(&Intent) -> bool,
) -> Vec<crate::desktop::services::requests::DesktopRequestId> {
    walk.shell.requests(cx, |request| {
        request.lane == Lane::User && pick(&request.intent)
    })
}

/// A15: Settings by `secondary-,`, the page's interval, switch and Check
/// now, then Unlock… and Retire… in Activity, each reached by Tab with focus
/// shown, and each sending what it says on the user lane.
#[gpui_kit::test]
async fn polling_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    for root in [&a, &b] {
        fixture.publish_over_ssh(root);
        fixture.refresh(root);
    }
    // `b` is paused, so only `a` has a launch poll.
    fixture.pause_polling(&b);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    // Every poll finds the key locked, as the transport does for a protected
    // key.
    let (window, ()) = PollingWindow::over(
        fixture,
        &[a.clone(), b.clone()],
        |backend, _| backend.ask_unlock(key.clone()),
        cx,
    );
    window.start(cx);
    bounded(&window.shell, cx, "the locked key's entry", |_, cx| {
        window.entries(cx).len() == 1
    })
    .await;
    let PollingWindow {
        shell,
        backend,
        services,
        clock,
        roots,
    } = window;
    let inner = backend.inner_shared();
    let walk = Walk::over(shell, a.clone());

    // Settings by `secondary-,`: the page list is shown with focus on its
    // first entry, and Tab and Enter choose the third.
    walk.keys("secondary-,", cx);
    walk.on(("settings-page", 0usize), Region::Content, cx);
    walk.tab_to("tab", ("settings-page", 2usize), cx);
    walk.on(("settings-page", 2usize), Region::Content, cx);
    walk.enter(cx);
    assert_eq!(
        walk.settings_shown(cx),
        Shown::Settings(SettingsPage::Polling)
    );
    walk.on(("settings-page", 2usize), Region::Content, cx);
    bounded(&walk.shell, cx, "the page's rows", |shell, cx| {
        shell.present(("polling-name", 1usize), cx)
    })
    .await;
    let rows = walk.shell.rows("polling-name", cx);
    let row = |name: &str| rows.iter().position(|shown| shown == name).unwrap();
    let (row_a, row_b) = (row("a"), row("b"));

    // The interval: Enter opens it, Down moves to Every 10 minutes, Enter
    // picks it.
    walk.tab_to("tab", ("polling-interval", row_a), cx);
    walk.on(("polling-interval", row_a), Region::Content, cx);
    walk.enter(cx);
    walk.keys("down", cx);
    walk.enter(cx);
    bounded(&walk.shell, cx, "the interval", |shell, cx| {
        shell.present(("polling-interval", row_a), cx)
    })
    .await;
    let configured = |walk: &Walk, cx: &mut TestAppContext| {
        user_sent(walk, cx, |intent| {
            matches!(
                intent,
                Intent::PollConfigure {
                    interval_seconds: 600,
                    ..
                }
            )
        })
        .len()
    };
    assert_eq!(configured(&walk, cx), 1, "Every 10 minutes was asked for");

    // The switch: Space turns checking off, which pauses, and Space again
    // turns it on.
    walk.tab_to("tab", ("polling-switch", row_a), cx);
    walk.on(("polling-switch", row_a), Region::Content, cx);
    walk.shell.press("space", cx);
    let verbs = |walk: &Walk, cx: &mut TestAppContext, resume: bool| {
        user_sent(walk, cx, |intent| {
            if resume {
                matches!(intent, Intent::PollResume { .. })
            } else {
                matches!(intent, Intent::PollPause { .. })
            }
        })
        .len()
    };
    assert_eq!(verbs(&walk, cx, false), 1, "off pauses");
    let paused = user_sent(&walk, cx, |intent| {
        matches!(intent, Intent::PollPause { .. })
    });
    walk.shell.until_ended(paused[0], cx).await;
    walk.on(("polling-switch", row_a), Region::Content, cx);
    walk.shell.press("space", cx);
    assert_eq!(verbs(&walk, cx, true), 1, "on resumes");
    let resumed = user_sent(&walk, cx, |intent| {
        matches!(intent, Intent::PollResume { .. })
    });
    walk.shell.until_ended(resumed[0], cx).await;

    // Check now, of `b`: Enter. Its answer is canned.
    inner.answer_job(JobKind::PollOnce, checked());
    walk.tab_to("tab", ("polling-check", row_b), cx);
    walk.on(("polling-check", row_b), Region::Content, cx);
    walk.enter(cx);
    let checks = user_sent(&walk, cx, |intent| {
        matches!(intent, Intent::PollOnce { .. })
    });
    assert_eq!(checks.len(), 1, "Check now was sent");
    walk.shell.until_ended(checks[0], cx).await;

    // The poll of `b` reports a closed ticket, which the loop leaves to the
    // person: an offer of Retire….
    let ticket = item_id(CLOSED);
    reported(
        &walk.shell,
        &services,
        &clock,
        &roots[1],
        vec![(ticket.clone(), TargetKind::Ticket)],
        cx,
    );
    let offer = RowKey::RetireOffer {
        root: roots[1].clone(),
        item: ticket.clone(),
    };

    // Activity, by `secondary-4`: the arrows and Tab reach Unlock… and then
    // Retire…. Settings is a sheet: Escape closes it first.
    walk.keys("escape", cx);
    let hold = hold_row(&walk, cx);
    to_row(&walk.shell, &hold, cx);
    walk.region(Region::Operations, cx);
    walk.keys("tab", cx);
    walk.on_handle(&button_focus(&walk.shell, cx), Region::Operations, cx);
    assert_eq!(button_label(&walk, &hold, cx).as_deref(), Some(UNLOCK));
    to_row(&walk.shell, &offer, cx);
    walk.keys("tab", cx);
    walk.on_handle(&button_focus(&walk.shell, cx), Region::Operations, cx);
    assert_eq!(button_label(&walk, &offer, cx).as_deref(), Some(RETIRE));
    // Enter on Retire… sends one retirement on the user lane, under a new
    // request, for that item.
    inner.answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, ResultCode::Ok, Removal::Observed),
    );
    walk.enter(cx);
    bounded(&walk.shell, cx, "the retirement to be sent", |shell, cx| {
        !shell
            .requests(cx, |request| {
                matches!(request.intent, Intent::ContextRetire { .. })
            })
            .is_empty()
    })
    .await;
    let retired = user_sent(&walk, cx, |intent| {
        matches!(intent, Intent::ContextRetire { .. })
    });
    assert_eq!(retired.len(), 1, "one retirement, on the user lane");
    walk.shell.until_ended(retired[0], cx).await;
    let sent = walk.shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(retired[0]).unwrap().intent.clone()
    });
    match sent {
        Intent::ContextRetire { root, kind, id } => {
            assert_eq!(root.to_str(), Some(roots[1].as_str()));
            assert_eq!(kind, AuthoringKind::Ticket);
            assert_eq!(id, item_id(CLOSED));
        }
        _ => panic!("not a retirement"),
    }
    refresh(&walk.shell, cx);
    assert!(
        activity_row(&walk.shell, &offer, cx).is_none(),
        "the item is retired: no offer"
    );

    // Back in Activity, Tab reaches Unlock…, and Enter sends a Check now of
    // the key's repository, on the user lane. No passphrase is asked for
    // here: the Check now is answered `ok` by a canned answer, standing for a
    // key that is now unlocked, and the entry goes. The double stops making
    // polls find the key locked, so the poll that follows does not raise the
    // entry again.
    backend.stop_asking_unlock();
    inner.answer_job(JobKind::PollOnce, checked());
    to_row(&walk.shell, &hold, cx);
    walk.keys("tab", cx);
    walk.on_handle(&button_focus(&walk.shell, cx), Region::Operations, cx);
    walk.enter(cx);
    let checks = user_sent(&walk, cx, |intent| {
        matches!(intent, Intent::PollOnce { .. })
    });
    assert_eq!(checks.len(), 2, "Unlock… sent a Check now");
    walk.shell.until_ended(checks[1], cx).await;
    let roots_checked: Vec<_> = walk.shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter_map(|request| match &request.intent {
                Intent::PollOnce { root } => root.to_str().map(str::to_owned),
                _ => None,
            })
            .collect()
    });
    assert_eq!(
        roots_checked,
        [roots[0].clone(), roots[1].clone()],
        "the newest Check now is `a`'s, the key's repository"
    );
    bounded(&walk.shell, cx, "the entry to go", |shell, cx| {
        refresh(shell, cx);
        activity_row(shell, &hold, cx).is_none()
    })
    .await;
}
