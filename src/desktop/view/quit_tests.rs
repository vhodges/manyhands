//! Quitting with polls about (D6 Task 10, design Part D): nothing new
//! starts once quitting is asked, a running poll is cancelled and its end
//! seen, the wait says it is still stopping after ten seconds and the worker
//! thread is joined on every graceful exit. Over the real worker and the
//! polling double; the scheduler's clock and the GPUI clock are moved by
//! hand, so nothing sleeps.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::repository::RemotePollError;

use super::{
    QuitPhase,
    support::{Rig, held_poll},
};
use crate::desktop::{
    model::{
        layout::PanelKind,
        polling::{Phase, Why},
    },
    services::{
        JobKind, ReadKind,
        requests::{Intent, Lane},
    },
    test_support::{
        drafts::{FolderMode, superuser},
        fixtures::Fixture,
    },
    view::harness::{Shell, wait_until},
};

const FINISHING_POLL: &str = "Finishing a check for updates";
const STILL_STOPPING: &str =
    "Still stopping \u{2014} waiting for Manyhands to reach a safe point. Your work is saved.";

/// The ten seconds, less and then exactly.
const TEN: Duration = Duration::from_secs(10);
const A_MOMENT: Duration = Duration::from_millis(1);

/// R-33: from the entry to `request_quit` the scheduler starts nothing: not
/// while quitting flushes a draft, nor while the dialog for a draft that
/// could not be saved asks (`QuitPhase` is back to `Running` then). Its timer
/// is dropped, and a poll that falls due waits. Cancel lets it go on. (The
/// unkept ask has its own test with the editor's helpers:
/// `edit_quit_freeze_tests`.)
#[cfg(unix)]
#[gpui_kit::test]
async fn quit_freezes_scheduler(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    let rig = Rig::new(&["a"], cx);
    rig.backend.answer_poll(rig.completed());
    rig.list(cx);
    rig.until_applied(&rig.roots[0], cx).await;
    assert!(rig.armed(cx), "a timer waits for the next poll");
    let polls = rig.polls();
    let shell = &rig.shell;

    shell.unsaved_draft(cx).await;
    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    let gate = shell.backend.hold_read(ReadKind::DraftWrite);
    assert!(!rig.request_quit(cx));
    assert_eq!(rig.phase(cx), QuitPhase::SavingDrafts);
    rig.assert_frozen(polls, cx).await;

    // The write fails: the dialog asks, with the phase back to `Running`.
    let _ = gate.send(());
    shell.run(cx);
    assert!(shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(rig.phase(cx), QuitPhase::Running);
    rig.assert_frozen(polls, cx).await;

    rig.backend.answer_poll(rig.completed());
    shell.click("drafts-not-saved-cancel", cx);
    assert_eq!(rig.phase(cx), QuitPhase::Running);
    rig.assert_resumed(polls, cx).await;
    drop(read_only);
}

/// Fact 16: a quit that waits for a poll, which is no request, says what it
/// is waiting for. The poll's end reaches the waiting quit by the notify
/// `Polled` makes of `Operations` (B1), with nothing else to wake it.
#[gpui_kit::test]
async fn quit_sees_poll_end(cx: &mut TestAppContext) {
    let rig = Rig::new(&["a"], cx);
    let gate = held_poll(&rig, cx).await;

    assert!(!rig.request_quit(cx), "the poll is running");
    assert!(matches!(rig.phase(cx), QuitPhase::Finishing(_)));
    assert_eq!(rig.text(cx).as_deref(), Some(FINISHING_POLL));
    assert_eq!(
        rig.shell.label("quit-finishing", cx).as_deref(),
        Some(FINISHING_POLL)
    );

    // The poll answers `Cancelled`: only `Polled` tells the window.
    gate.release();
    rig.shell
        .until_drawn(cx, "the quit to see the poll end", |_, cx| {
            rig.phase(cx) == QuitPhase::Quit
        })
        .await;
}

/// Decision 7: ten seconds into a poll that will not stop (a blocked
/// connect, K-1), the words change and nothing else does. The UI answers
/// (Quit now works), the poll is still the job the worker runs, and Quit now
/// does not wait for it nor join the worker. The ten seconds are the GPUI
/// clock's, moved by hand.
#[gpui_kit::test]
async fn quit_polling_still_stopping(cx: &mut TestAppContext) {
    let rig = Rig::new(&["a"], cx);
    let gate = held_poll(&rig, cx).await;
    let (op, _) = rig.backend.requests()[0];
    let watch = rig.worker().poll_watch();
    // Activity is hidden, as a person may have it: a quit that waits
    // changes no layout, and its feedback shows when Activity is opened
    // (W-2, Cycle 01 decision 7, K-18; E-5).
    rig.shell.click("footer-panel-activity", cx);
    let layout = |rig: &Rig, cx: &mut TestAppContext| {
        rig.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.layout().clone())
    };
    let saves = |rig: &Rig, cx: &mut TestAppContext| {
        rig.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.layout_saves())
    };
    let hidden = layout(&rig, cx);
    assert!(!hidden.is_displayed(PanelKind::Activity));
    let saved = saves(&rig, cx);

    assert!(!rig.request_quit(cx));
    let waiting = rig.phase(cx);
    assert!(matches!(waiting, QuitPhase::Finishing(_)));
    cx.executor().advance_clock(TEN - A_MOMENT);
    rig.shell.run(cx);
    assert_eq!(rig.text(cx).as_deref(), Some(FINISHING_POLL));

    cx.executor().advance_clock(A_MOMENT);
    rig.shell.run(cx);
    assert_eq!(rig.text(cx).as_deref(), Some(STILL_STOPPING));
    assert_eq!(layout(&rig, cx), hidden, "the layout is unchanged");
    assert_eq!(saves(&rig, cx), saved, "no save is scheduled");
    assert!(
        !rig.shell.present("quit-finishing", cx),
        "Activity is hidden"
    );
    // Opening Activity shows the same text.
    rig.shell.keys("secondary-4", cx);
    assert!(layout(&rig, cx).is_displayed(PanelKind::Activity));
    assert_eq!(
        rig.shell.label("quit-finishing", cx).as_deref(),
        Some(STILL_STOPPING)
    );
    // Nothing was cleared or reclaimed: the quit still waits, and the
    // worker still runs the poll, which the cancel could not stop.
    assert_eq!(rig.phase(cx), waiting);
    assert!(watch.running(op));
    assert!(rig.backend.cancels().iter().all(|seen| seen.op == op));
    assert!(!rig.backend.cancels().is_empty(), "the cancel was sent");
    assert!(!rig.worker().ended());

    // The UI responds: Quit now quits at once, the poll still held.
    rig.shell.click("quit-now", cx);
    assert_eq!(rig.phase(cx), QuitPhase::Quit);
    assert!(watch.running(op));
    assert!(!rig.worker().ended(), "Quit now does not join the worker");
    drop(gate);
}

/// Decision 28, R-38, at the worker: a quit waits for a background job as
/// for any job, says it is still stopping at the injected wait, and quits
/// when the job returns. The job is a background `IndexRefresh`, the one
/// background job there is: `ContextRetire` arrives with Task 11a, and what
/// is left for it is that quit leaves its token alone (TODO in
/// `WorkerHandle::quit`).
#[gpui_kit::test]
async fn quit_lets_retirement_finish(cx: &mut TestAppContext) {
    let rig = Rig::new(&["a"], cx);
    let wait = Duration::from_secs(3);
    rig.shell.operations(cx).update(cx, |operations, _| {
        operations.set_still_stopping_after(wait)
    });
    let held = rig.shell.backend.hold_job(JobKind::IndexRefresh);
    let job = rig.worker().submit(
        Intent::IndexRefresh {
            root: rig.roots[0].clone().into(),
        },
        Lane::Background,
        None,
        None,
    );
    held.wait_reached();

    assert!(!rig.request_quit(cx));
    assert_eq!(rig.phase(cx), QuitPhase::Finishing(job));
    cx.executor().advance_clock(wait - A_MOMENT);
    rig.shell.run(cx);
    assert_ne!(rig.text(cx).as_deref(), Some(STILL_STOPPING));
    cx.executor().advance_clock(A_MOMENT);
    rig.shell.run(cx);
    assert_eq!(rig.text(cx).as_deref(), Some(STILL_STOPPING));
    assert_eq!(rig.phase(cx), QuitPhase::Finishing(job), "it still waits");

    held.release();
    wait_until(
        &rig.shell.visual(cx),
        &rig.shell.workspace,
        |workspace, _| workspace.quit_phase() == QuitPhase::Quit,
    )
    .await;
    assert!(rig.worker().ended());
}

/// A9, O-7, Fact 36: a graceful quit with nothing running joins the worker
/// thread before the app quits. The window does not allow parking, so a join
/// that parked the test executor would fail the test.
#[gpui_kit::test]
fn quit_joins_worker(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(!worker.ended());

    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Quit);
    assert!(worker.ended(), "joined before the app quit");
}

/// The other graceful exit: a quit that waited for a job joins when the job
/// ends. The held job is still running, so the worker is not joined while
/// the quit waits.
#[gpui_kit::test]
async fn quit_joins_worker_when_a_job_ends(cx: &mut TestAppContext) {
    let rig = Rig::new(&["a"], cx);
    let held = rig.shell.backend.hold_job(JobKind::IndexRefresh);
    let job = rig.worker().submit(
        Intent::IndexRefresh {
            root: rig.roots[0].clone().into(),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();

    assert!(!rig.request_quit(cx));
    assert_eq!(rig.phase(cx), QuitPhase::Finishing(job));
    assert!(!rig.worker().ended(), "the job is still running");

    held.release();
    wait_until(
        &rig.shell.visual(cx),
        &rig.shell.workspace,
        |workspace, _| workspace.quit_phase() == QuitPhase::Quit,
    )
    .await;
    assert!(rig.worker().ended());
}

/// R-4: the restart wait after a `busy` answer is the scheduler's own state,
/// not a job, so a quit during it has nothing to wait for and ends at once;
/// and the wait running out afterwards starts no poll.
#[gpui_kit::test]
async fn quit_during_restart_wait(cx: &mut TestAppContext) {
    let rig = Rig::new(&["a"], cx);
    rig.backend.fail_poll(RemotePollError::Busy);
    rig.list(cx);
    rig.shell
        .until_drawn(cx, "the Busy answer", |_, cx| {
            rig.services.scheduler().read_with(cx, |scheduler, _| {
                matches!(
                    scheduler
                        .schedule()
                        .repo(&rig.roots[0])
                        .map(|repo| &repo.phase),
                    Some(Phase::Waiting { why: Why::Busy, .. })
                )
            })
        })
        .await;
    assert_eq!(rig.polls(), 1);

    assert!(rig.request_quit(cx), "nothing is running");
    assert_eq!(rig.phase(cx), QuitPhase::Quit);
    assert!(rig.worker().ended());
    rig.advance(Duration::from_secs(70));
    rig.tick(cx);
    Rig::quiet(cx).await;
    assert_eq!(rig.polls(), 1, "no poll after the wait");
}
