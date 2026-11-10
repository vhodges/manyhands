//! The polling journey (D6 acceptance A15, Task 13; design W-4, W-6) by
//! keystrokes alone, in the shell Cycle 01 made: Settings is opened with
//! `secondary-,` and the Checking for updates page is reached by Tab; its
//! interval is changed, its switch turned off and on, and Check now pressed;
//! then Activity, the bottom dock's panel, is reached with `secondary-4`,
//! and Tab reaches Unlock… (a locked key's entry) and Retire… (the offer on
//! a closed ticket), each pressed. Focus is shown at every step: the element
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
    model::polling::{
        TargetKind,
        attention::UNLOCK,
        entry::{RETIRE, short_code},
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
        polling_window::{PollingWindow, reported},
        settings::Shown,
        workspace::Region,
    },
};

use crate::desktop::model::stamp::SettingsPage;

const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";

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
    // Activity at the 150 px the application launches with, so that Tab has
    // to bring an action below the fold into view for it to be on screen.
    window.shell.set_activity_height(150., cx);
    window.start(cx);
    window
        .shell
        .until_drawn(cx, "the locked key's entry", |_, cx| {
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
    // first entry, and Tab and Enter choose the fifth page.
    walk.keys("secondary-,", cx);
    walk.on(("settings-page", 0usize), Region::Content, cx);
    walk.tab_to("tab", ("settings-page", 4usize), cx);
    walk.on(("settings-page", 4usize), Region::Content, cx);
    walk.enter(cx);
    assert_eq!(
        walk.settings_shown(cx),
        Shown::Settings(SettingsPage::Polling)
    );
    walk.on(("settings-page", 4usize), Region::Content, cx);
    walk.until_drawn(("polling-name", 1usize), cx).await;
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
    walk.until_drawn(("polling-interval", row_a), cx).await;
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
    let offer = format!("attempt-{}-retire-{}", roots[1], short_code(&ticket));

    // Activity, by `secondary-4`: Tab reaches Unlock… and then Retire….
    walk.keys("secondary-4", cx);
    walk.region(Region::Operations, cx);
    walk.tab_to("tab", ("attention-action", 0usize), cx);
    walk.on(("attention-action", 0usize), Region::Operations, cx);
    assert_eq!(
        walk.shell
            .label(("attention-action", 0usize), cx)
            .as_deref(),
        Some(UNLOCK)
    );
    walk.tab_to("tab", offer.clone(), cx);
    walk.on(offer.clone(), Region::Operations, cx);
    assert_eq!(walk.shell.label(offer.clone(), cx).as_deref(), Some(RETIRE));

    // Enter on Retire… sends one retirement on the user lane, under a new
    // request, for that item.
    inner.answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, ResultCode::Ok, Removal::Observed),
    );
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the retirement to be sent", |shell, cx| {
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
    assert!(
        !walk.shell.present(offer.clone(), cx),
        "the item is retired: no offer"
    );

    // Back in Activity, Tab reaches Unlock…, and Enter sends a Check now of
    // the key's repository, on the user lane. It is answered `ok` (the
    // passphrase asked for, supplied), and the entry goes.
    // The passphrase is accepted: the poll that follows no longer finds the
    // key locked.
    backend.stop_asking_unlock();
    inner.answer_job(JobKind::PollOnce, checked());
    walk.keys("secondary-4", cx);
    walk.tab_to("tab", ("attention-action", 0usize), cx);
    walk.on(("attention-action", 0usize), Region::Operations, cx);
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
    walk.shell
        .until_drawn(cx, "the entry to go", |shell, cx| {
            !shell.present(("attention-action", 0usize), cx)
        })
        .await;
}
