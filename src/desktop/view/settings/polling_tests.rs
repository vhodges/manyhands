//! The Checking for updates page (D6 design C1, C2, C4): its rows and
//! what its controls ask of the service. The schedule is seeded through the
//! model's own transitions; what a control sends is read from the requests
//! the service received. No test reads fonts, colours or layout.

use std::time::Duration;

use gpui_kit::{TestAppContext, test::TestWindowExt};

use manyhands::repository::OperationId;

use super::polling_support::{Rig, policy};
use crate::desktop::{
    model::{
        polling::{
            Answer, TransportFailure,
            status::{NO_SHARED_COPY, cant_reach},
        },
        stamp::SettingsPage,
    },
    services::{
        JobKind,
        requests::{Intent, Lane},
    },
    view::{
        settings::{Shown, page_title},
        workspace::Region,
    },
};

/// Tab presses enough to go round the whole window.
const TABS: usize = 40;

/// What the page says first (C2), word for word.
const INTRO: &str = "Manyhands checks your shared copies for other people's changes and brings them in when it's safe. It never shares anything. It also removes this computer's copies of tickets closed and documents promoted elsewhere, unless you have unsaved or kept work in them.";

/// One row per repository, each with its name and its status line from the
/// model (A4): a repository that checked just now, one needing its key
/// unlocked, one that cannot be reached, one waiting on another operation,
/// and one with no shared copy. The page's intro is C2's, word for word, and
/// the page is the fifth of the list.
#[gpui_kit::test]
async fn settings_polling_rows(cx: &mut TestAppContext) {
    let rig = Rig::new(
        &[
            ("a", true),
            ("b", true),
            ("c", true),
            ("d", true),
            ("e", false),
        ],
        cx,
    );
    for ix in 0..4 {
        rig.schedule(ix, cx);
    }
    rig.answer(0, Answer::Completed, cx);
    rig.answer(
        1,
        Answer::Transport(TransportFailure::KeyLocked { key: None }),
        cx,
    );
    rig.answer(2, Answer::Transport(TransportFailure::Unreachable), cx);
    rig.answer(3, Answer::Busy, cx);
    rig.open(5, cx).await;

    assert_eq!(rig.shell.label("polling-intro", cx).as_deref(), Some(INTRO));
    assert_eq!(
        rig.shell.label(("settings-page", 4usize), cx).as_deref(),
        Some(page_title(SettingsPage::Polling))
    );
    assert_eq!(page_title(SettingsPage::Polling), "Checking for updates");
    assert!(rig.shell.present("settings-polling", cx));
    let mut names = rig.shell.rows("polling-name", cx);
    assert_eq!(names.len(), 5, "one row per repository: {names:?}");
    names.sort();
    assert_eq!(names, ["a", "b", "c", "d", "e"]);

    let statuses = |cx: &mut TestAppContext| -> Vec<(String, String)> {
        let rows = rig.shell.rows("polling-name", cx);
        let mut shown: Vec<_> = rows
            .into_iter()
            .enumerate()
            .map(|(ix, name)| (name, rig.status(ix, cx).unwrap_or_default()))
            .collect();
        shown.sort();
        shown
    };
    let expected = |others: &[(&str, &str)]| -> Vec<(String, String)> {
        others
            .iter()
            .map(|(name, words)| ((*name).to_owned(), (*words).to_owned()))
            .collect()
    };
    assert_eq!(
        statuses(cx),
        expected(&[
            ("a", "Checked just now \u{b7} up to date"),
            ("b", "Choose a key"),
            ("c", &cant_reach("c")),
            ("d", "Checking soon"),
            ("e", NO_SHARED_COPY),
        ])
    );

    // The words follow the clock: five minutes on, the same check reads so.
    rig.clock.advance(Duration::from_secs(5 * 60));
    rig.services.scheduler().update(cx, |_, cx| cx.notify());
    rig.shell.run(cx);
    let a = rig.row("a", cx);
    assert_eq!(
        rig.status(a, cx).as_deref(),
        Some("Checked 5 minutes ago \u{b7} up to date")
    );
}

fn polled_once(root: &std::path::Path) -> impl Fn(&Intent) -> bool + '_ {
    move |intent| matches!(intent, Intent::PollOnce { root: asked } if asked == root)
}

/// Opens the interval select of row `ix` and picks the option `presses`
/// below the one shown, as the arrow key goes (the list wraps).
pub(super) fn choose_interval(rig: &Rig, ix: usize, presses: usize, cx: &mut TestAppContext) {
    let select = ("polling-interval-select", ix);
    rig.shell
        .frame(cx, |window, cx| window.within(select).click("input", cx));
    rig.shell.run(cx);
    for _ in 0..presses {
        rig.shell.press("down", cx);
    }
    rig.shell.press("enter", cx);
    rig.shell.run(cx);
}

/// What the select of row `ix` shows.
pub(super) fn shown_interval(rig: &Rig, ix: usize, cx: &mut TestAppContext) -> Option<String> {
    rig.shell.frame(cx, |window, _| {
        window
            .find(("polling-interval-select", ix))
            .value()
            .map(str::to_owned)
    })
}

/// Picks as `choose_interval` does, and gives the interval the page sent
/// to the repository at `root`, once the request ended.
async fn pick_interval(
    rig: &Rig,
    (ix, root): (usize, &std::path::Path),
    presses: usize,
    cx: &mut TestAppContext,
) -> u64 {
    choose_interval(rig, ix, presses, cx);
    let sent = rig.sent(
        cx,
        |intent| matches!(intent, Intent::PollConfigure { root: asked, .. } if asked == root),
    );
    let request = sent.last().expect("the pick sent a request");
    assert_eq!(request.lane, Lane::User);
    let Intent::PollConfigure {
        interval_seconds, ..
    } = request.intent
    else {
        unreachable!("picked above")
    };
    rig.shell.until_ended(request.id, cx).await;
    interval_seconds
}

/// Check now sends `poll once` on the user lane, whatever the row is doing,
/// an automatic poll of it running included (B2: a user job runs first), and
/// reads Retry now after a failure. It is disabled only while this
/// process's own Check now of that repository is queued or running (C1):
/// another repository's is not, and it is back once the check ended. The
/// switch is on while unpaused, off is Pause and on is Resume.
#[gpui_kit::test]
async fn settings_polling_controls(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true), ("b", true), ("c", true)], cx);
    for ix in 0..3 {
        rig.schedule(ix, cx);
    }
    // `b` failed; `c` has an automatic poll running.
    rig.answer(1, Answer::Transport(TransportFailure::Unreachable), cx);
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler
            .schedule_mut()
            .mark_running(&rig.key(2), OperationId::new());
        cx.notify();
    });
    rig.open(3, cx).await;
    let (a, b, c) = (rig.row("a", cx), rig.row("b", cx), rig.row("c", cx));
    let label = |ix: usize, cx: &mut TestAppContext| rig.shell.label(("polling-check", ix), cx);
    assert_eq!(label(a, cx).as_deref(), Some("Check now"));
    assert_eq!(label(b, cx).as_deref(), Some("Retry now"));
    assert_eq!(label(c, cx).as_deref(), Some("Check now"));
    assert_eq!(rig.status(c, cx).as_deref(), Some("Checking\u{2026}"));

    // Check now while an automatic poll runs: a user job, so the worker
    // cancels that poll and runs it (B2, Task 4).
    assert!(
        rig.shell.tab_reaches(("polling-check", c), TABS, cx),
        "an automatic poll does not disable it"
    );
    let gate = rig.shell.backend.hold_job(JobKind::PollOnce);
    rig.shell.click(("polling-check", c), cx);
    let sent = rig.sent(cx, polled_once(&rig.roots[2]));
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].lane, Lane::User);
    gate.wait_reached();
    // Its own Check now is running: disabled, so pressing it again sends
    // nothing. Another repository's is not.
    // The Kit's disabled button takes no focus, so Tab passes it by; the
    // others' buttons, and Check now of `c` before and after, are reached.
    assert!(!rig.shell.tab_reaches(("polling-check", c), TABS, cx));
    assert!(rig.shell.tab_reaches(("polling-check", a), TABS, cx));
    assert!(rig.shell.tab_reaches(("polling-check", b), TABS, cx));
    rig.shell.click(("polling-check", c), cx);
    assert_eq!(rig.sent(cx, polled_once(&rig.roots[2])).len(), 1);
    rig.shell.click(("polling-check", a), cx);
    assert_eq!(rig.sent(cx, polled_once(&rig.roots[0])).len(), 1);
    gate.release();
    rig.shell.until_ended(sent[0].id, cx).await;
    // It ended: Tab reaches it again, and pressed it sends.
    rig.shell.run(cx);
    assert!(rig.shell.tab_reaches(("polling-check", c), TABS, cx));
    rig.shell.click(("polling-check", c), cx);
    assert_eq!(rig.sent(cx, polled_once(&rig.roots[2])).len(), 2);

    // Retry now is the same request.
    rig.shell.click(("polling-check", b), cx);
    assert_eq!(rig.sent(cx, polled_once(&rig.roots[1])).len(), 1);

    // The interval offers Every 1, 5, 10, 15, 30 and 60 minutes and nothing
    // else: from the one shown, six presses go round the whole list (it
    // wraps), and each sends a `poll configure` inside 60 to 3600 s.
    assert_eq!(
        shown_interval(&rig, a, cx).as_deref(),
        Some("Every 5 minutes")
    );
    let mut offered = std::collections::BTreeSet::new();
    for presses in 1..=6 {
        offered.insert(pick_interval(&rig, (a, &rig.roots[0]), presses, cx).await);
    }
    assert_eq!(
        offered,
        std::collections::BTreeSet::from([60, 300, 600, 900, 1800, 3600])
    );

    // The pick stays on show while its request runs, and once it ended the
    // select follows the policy, which did not change here.
    let gate = rig.shell.backend.hold_job(JobKind::PollConfigure);
    choose_interval(&rig, a, 1, cx);
    gate.wait_reached();
    assert_eq!(
        shown_interval(&rig, a, cx).as_deref(),
        Some("Every 10 minutes")
    );
    gate.release();
    let held = rig.sent(cx, |intent| matches!(intent, Intent::PollConfigure { .. }));
    rig.shell.until_ended(held.last().unwrap().id, cx).await;
    rig.shell.run(cx);
    assert_eq!(
        shown_interval(&rig, a, cx).as_deref(),
        Some("Every 5 minutes")
    );

    // The switch is on while unpaused: turning it off pauses, and a paused
    // repository's turns it on again.
    let toggles = |cx: &mut TestAppContext| {
        (
            rig.sent(
                cx,
                |i| matches!(i, Intent::PollPause { root } if *root == rig.roots[0]),
            ),
            rig.sent(
                cx,
                |i| matches!(i, Intent::PollResume { root } if *root == rig.roots[0]),
            ),
        )
    };
    assert_eq!(
        rig.shell.checked(("polling-switch-toggle", a), cx),
        Some(true)
    );
    rig.shell.click(("polling-switch", a), cx);
    let (paused, resumed) = toggles(cx);
    assert_eq!((paused.len(), resumed.len()), (1, 0));
    assert_eq!(paused[0].lane, Lane::User);
    let mut stored = policy();
    stored.paused = true;
    rig.policy(0, stored, cx);
    assert_eq!(
        rig.shell.checked(("polling-switch-toggle", a), cx),
        Some(false)
    );
    assert_eq!(rig.status(a, cx).as_deref(), Some("Paused"));
    rig.shell.click(("polling-switch", a), cx);
    let (paused, resumed) = toggles(cx);
    assert_eq!((paused.len(), resumed.len()), (1, 1));
}

/// A repository with no SSH publication remote says "No shared copy,
/// nothing to check" and offers Set up sharing… instead of any control,
/// which opens the Remotes page on it (C2). One whose `enabled` is off says
/// "Checking is off" and has no control at all: no verb sets `enabled`
/// (K-5).
#[gpui_kit::test]
async fn settings_polling_states(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", false), ("b", true)], cx);
    let mut off = policy();
    off.enabled = false;
    rig.policy(1, off, cx);
    rig.open(2, cx).await;
    let (a, b) = (rig.row("a", cx), rig.row("b", cx));
    let controls = |ix: usize, cx: &mut TestAppContext| {
        ["polling-interval", "polling-check", "polling-switch"]
            .map(|id| rig.shell.present((id, ix), cx))
    };

    assert_eq!(rig.status(a, cx).as_deref(), Some(NO_SHARED_COPY));
    assert_eq!(controls(a, cx), [false; 3]);
    assert_eq!(
        rig.shell.label(("polling-setup", a), cx).as_deref(),
        Some("Set up sharing\u{2026}")
    );
    assert_eq!(rig.status(b, cx).as_deref(), Some("Checking is off"));
    assert_eq!(controls(b, cx), [false; 3]);
    assert!(!rig.shell.present(("polling-setup", b), cx));

    rig.shell.click(("polling-setup", a), cx);
    assert_eq!(
        rig.shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx)),
        Shown::Settings(SettingsPage::Remotes)
    );
    assert!(rig.shell.present("settings-remotes", cx));
    let chosen = rig.shell.rows("remotes-registration", cx);
    let ix = chosen
        .iter()
        .position(|root| *root == rig.key(0))
        .expect("the repository is listed");
    assert!(rig.shell.present(("remotes-chosen", ix), cx), "worked on");
}

/// A repository waiting for an unfinished operation links to its pending
/// row: Show in Activity moves focus to the operation area, which lists it
/// (A4). No other row offers it.
#[gpui_kit::test]
async fn settings_polling_pending_link(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    rig.schedule(0, cx);
    rig.schedule(1, cx);
    rig.answer(
        0,
        Answer::Repository {
            recovery_required: true,
        },
        cx,
    );
    rig.open(2, cx).await;
    let (a, b) = (rig.row("a", cx), rig.row("b", cx));
    assert_eq!(
        rig.status(a, cx).as_deref(),
        Some("Waiting for an unfinished operation")
    );
    assert_eq!(
        rig.shell.label(("polling-pending", a), cx).as_deref(),
        Some("Show in Activity")
    );
    assert!(!rig.shell.present(("polling-pending", b), cx));
    assert!(!rig.shell.present(Region::Operations.ring_id(), cx));
    rig.shell.click(("polling-pending", a), cx);
    assert!(rig.shell.present(Region::Operations.ring_id(), cx));
}
