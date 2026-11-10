//! The attempt entry in the operation area (D6 Task 9, A2, A4, R-20, R-23,
//! W-6): one entry per repository for its latest attempt, replaced by the
//! next, absent for a quiet one; each disposition listed with its text and
//! calling no mutation; and Share… on `Unpublished` alone, reachable by a
//! click at the default window and by Tab. The attempts reach the schedule
//! as the scheduler hands them (`PollSchedule::after`).

use gpui_kit::{TestAppContext, px, size};
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, OperationId, PollDisposition},
};

use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind,
        entry::{SHARE, UNPUBLISHED},
    },
    services::{JobKind, requests::Intent},
    view::settings::polling_support::Rig,
};

const TABS: usize = 60;

/// An attempt of `ix`'s repository that completed over `dispositions`, one
/// item each.
fn attempt(
    rig: &Rig,
    ix: usize,
    dispositions: &[Disposition],
    cx: &mut TestAppContext,
) -> Vec<ItemId> {
    let ids: Vec<ItemId> = dispositions.iter().map(|_| ItemId::generate()).collect();
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(ix),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: dispositions
            .iter()
            .zip(&ids)
            .map(|(&disposition, id)| AttemptTarget {
                item_id: Some(id.clone()),
                kind: Some(TargetKind::Ticket),
                disposition,
            })
            .collect(),
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
    ids
}

fn line(rig: &Rig, n: usize, i: usize, cx: &mut TestAppContext) -> Option<String> {
    rig.shell.label(format!("attempt-{n}-line-{i}"), cx)
}

#[gpui_kit::test]
async fn dispositions_listed(cx: &mut TestAppContext) {
    use Disposition::*;
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    rig.schedule(0, cx);
    rig.schedule(1, cx);
    assert!(
        !rig.shell.present("attempt-0-repository", cx),
        "no attempt, no entry"
    );

    // A quiet attempt shows no entry.
    attempt(&rig, 0, &[Current, LocalAhead], cx);
    assert!(
        !rig.shell.present("attempt-0-repository", cx),
        "quiet: no entry"
    );

    // Everything the poll leaves alone is listed, in the poll's order, and
    // only the unpublished one offers Share….
    let left_alone = [
        Dirty,
        Diverged,
        Conflicted,
        Inaccessible,
        BlockedByLocalState,
        MaterializationIncomplete,
        Nonconforming,
        Malformed,
        RemotelyDeleted,
        HistoryUnknown,
        Unpublished,
        NotReached,
    ];
    attempt(&rig, 0, &left_alone, cx);
    assert!(rig.shell.present("attempt-0-repository", cx));
    assert!(
        !rig.shell.present("attempt-1-repository", cx),
        "one entry per repository"
    );
    let expected = [
        PollDisposition::Dirty.guidance(),
        PollDisposition::Diverged.guidance(),
        PollDisposition::Conflicted.guidance(),
        PollDisposition::Inaccessible.guidance(),
        PollDisposition::BlockedByLocalState.guidance(),
        PollDisposition::MaterializationIncomplete.guidance(),
        PollDisposition::Nonconforming.guidance(),
        PollDisposition::Malformed.guidance(),
        PollDisposition::RemotelyDeleted.guidance(),
        PollDisposition::HistoryUnknown.guidance(),
        UNPUBLISHED,
        PollDisposition::NotReached.guidance(),
    ];
    for (i, words) in expected.into_iter().enumerate() {
        assert_eq!(line(&rig, 0, i, cx).as_deref(), Some(words), "line {i}");
        let action = rig.shell.label(format!("attempt-0-action-{i}"), cx);
        match i {
            10 => assert_eq!(action.as_deref(), Some(SHARE)),
            _ => assert_eq!(action, None, "line {i} offers nothing"),
        }
    }
    assert!(!rig.shell.present("attempt-0-line-12", cx));
    assert_eq!(rig.shell.label("attempt-0-brought-in", cx), None);

    // A disposition is only text: nothing was sent.
    assert!(rig.sent(cx, |_| true).is_empty(), "no request, no mutation");

    // The next attempt replaces the entry; a quiet one removes it.
    attempt(&rig, 0, &[Dirty, FastForwarded], cx);
    assert_eq!(
        rig.shell.label("attempt-0-brought-in", cx).as_deref(),
        Some("Brought in 1 change")
    );
    assert_eq!(
        line(&rig, 0, 0, cx).as_deref(),
        Some(PollDisposition::Dirty.guidance())
    );
    assert!(!rig.shell.present("attempt-0-line-1", cx), "replaced");
    attempt(&rig, 0, &[Current], cx);
    assert!(
        !rig.shell.present("attempt-0-repository", cx),
        "quiet again: no entry"
    );

    // Another repository's entry is its own.
    attempt(&rig, 1, &[Malformed], cx);
    assert_eq!(
        line(&rig, 0, 0, cx).as_deref(),
        Some(PollDisposition::Malformed.guidance())
    );
    assert!(rig.sent(cx, |_| true).is_empty(), "still no mutation");
}

/// W-6, E-2: Share… is reachable by Tab, where it scrolls into view, is
/// then where a click reaches it at the default window, and Enter presses
/// it. Each sends the share of its own item, as the item header's Share
/// does, and nothing else; while a share runs, its line says "Sharing…" and
/// offers no second press. Thirty lines come first, so the actions are below
/// the fold of the 260 px area until focus reaches them.
#[gpui_kit::test]
async fn attempt_entry_actions_hit(cx: &mut TestAppContext) {
    use Disposition::*;
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.shell
        .visual(cx)
        .simulate_resize(size(px(1280.), px(840.)));
    let mut dispositions = vec![Dirty; 30];
    dispositions.extend([Unpublished, Unpublished]);
    let ids = attempt(&rig, 0, &dispositions, cx);
    let shares = |rig: &Rig, cx: &mut TestAppContext| -> Vec<ItemId> {
        rig.sent(cx, |_| true)
            .into_iter()
            .map(|request| match request.intent {
                Intent::ItemSync { root, kind, id } => {
                    assert_eq!(root, rig.roots[0]);
                    assert_eq!(kind, AuthoringKind::Ticket);
                    id
                }
                _ => panic!("not a share"),
            })
            .collect()
    };
    assert!(shares(&rig, cx).is_empty());
    assert_eq!(
        rig.shell.label("attempt-0-action-30", cx).as_deref(),
        Some(SHARE)
    );

    let held = rig.shell.backend.hold_job(JobKind::SynchronizeItem);
    assert!(rig.shell.tab_reaches("attempt-0-action-30", TABS, cx));
    rig.shell.click("attempt-0-action-30", cx);
    assert_eq!(shares(&rig, cx), [ids[30].clone()]);
    held.wait_reached();
    rig.shell.run(cx);
    assert_eq!(
        rig.shell.label("attempt-0-action-30", cx).as_deref(),
        Some("Sharing\u{2026}"),
        "no second press while it runs"
    );

    assert!(rig.shell.tab_reaches("attempt-0-action-31", TABS, cx));
    rig.shell.press("enter", cx);
    assert_eq!(shares(&rig, cx), [ids[30].clone(), ids[31].clone()]);
    held.release();
}
