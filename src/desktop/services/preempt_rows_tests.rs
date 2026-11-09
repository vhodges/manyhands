//! The rows of `preemption_table` (D6 Task 4): one function per required
//! outcome, over the harness of the parent module.

use super::*;

/// P-1 through P-3 for one of the four ways a user job enters the worker.
#[derive(Clone, Copy, Debug)]
pub(super) enum Entry {
    Submit,
    SubmitWithRequestId,
    Requeue,
    ObserveHost,
}

/// A user job admitted while a poll runs sends the cancel, again and again
/// until the poll answers (P-1): the first cancel is early (the library has
/// no row yet and answers an error), the second reaches a row that is not
/// active and answers `Ok` without effect (Fact 30), and the third is
/// effective. The poll answers `Cancelled` as preempted, and only then does
/// the user job run (P-3).
pub(super) fn user_job_preempts(entry: Entry) {
    let mut h = Harness::new();
    let earlier = (matches!(entry, Entry::Requeue)).then(|| {
        let id = h.worker.submit(h.refresh(), Lane::User, None, None);
        h.finished(id);
        id
    });
    h.backend.script_cancels([
        CancelStep::Missing,
        CancelStep::Ineffective,
        CancelStep::Effective,
    ]);
    h.backend.answer_poll(cancelled());
    let op = OperationId::new();
    let gate = h.hold_poll(op, false);

    let job = match entry {
        Entry::Submit => h.worker.submit(h.refresh(), Lane::User, None, None),
        Entry::SubmitWithRequestId => {
            h.worker
                .submit_with_request_id(h.refresh(), Lane::User, RequestId::new())
        }
        Entry::Requeue => {
            let id = earlier.expect("an earlier request");
            assert!(h.worker.retry(id, None), "{entry:?}");
            id
        }
        Entry::ObserveHost => h.worker.observe_host(h.root.clone().into(), None),
    };
    let attempt = h.until_polled();
    h.finished_after_poll(job);
    let _ = &gate;

    assert_eq!(attempt.operation, op, "{entry:?}");
    assert_eq!(
        attempt.answer,
        Answer::Cancelled(CancelCause::Preemption),
        "{entry:?}"
    );
    assert_eq!(h.preempts(), 1, "{entry:?}: one task per poll");
    let seen = h.backend.cancels();
    assert!(seen.len() >= 3, "{entry:?}: an Ok did not end it: {seen:?}");
    assert!(seen.iter().all(|cancel| cancel.op == op), "{entry:?}");
    let answered: Vec<bool> = seen
        .iter()
        .take(3)
        .map(|cancel| cancel.answered_ok)
        .collect();
    assert_eq!(
        answered,
        [false, true, true],
        "{entry:?}: before the row exists, then Ok without effect, then effective"
    );
    // The schedule is the injected one: each resend waits the next delay.
    let waits = h.join_tasks();
    assert_eq!(waits.len(), 1, "{entry:?}");
    assert_eq!(waits[0][..3], DELAYS, "{entry:?}");
    // The user job ran after the poll answered, not before.
    let polled_at = h.position(|e| matches!(e, Heard::Polled(_)));
    assert!(
        h.log[polled_at..]
            .iter()
            .any(|e| matches!(e, Heard::Started(id) | Heard::Host(id) if *id == job)),
        "{entry:?}: the job ran after the poll answered"
    );
    assert!(
        h.log[..polled_at]
            .iter()
            .all(|e| !matches!(e, Heard::Started(id) | Heard::Host(id) if *id == job && earlier != Some(job))),
        "{entry:?}: and not before"
    );
}

/// P-2: a poll asked to stop before it reaches `poll_automatic` never makes
/// it, whatever the cancel did, as a same-ID restart between its attempts
/// would not. A restart (`restart=true`) and a first attempt alike.
pub(super) fn flag_checked_before_every_poll(restart: bool) {
    let mut h = Harness::new();
    // A trap: were `poll_automatic` reached, this would answer `Cancelled`
    // too, so the test reads the request list, not the answer.
    h.backend.answer_poll(cancelled());
    let held = h.backend.inner().hold_job(JobKind::Poll);
    let op = OperationId::new();
    h.worker
        .submit_poll(h.root.clone(), op, restart)
        .expect("the queue is open");
    held.wait_reached();
    let job = h.worker.submit(h.refresh(), Lane::User, None, None);
    held.release();
    let attempt = h.until_polled();
    h.finished(job);

    assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Preemption));
    assert_eq!(attempt.operation, op);
    assert!(
        h.backend.requests().is_empty(),
        "restart={restart}: poll_automatic was made"
    );
    assert!(!h.backend.calls().contains(&PollCall::PollAutomatic));
    h.join_tasks();
}

/// P-3 and P-4 through the model: a preempted poll goes again under a new
/// operation ID with no failure backoff, three preemptions in a row wait for
/// the interval (R-34), and the in-flight record drops each cancelled
/// operation's entry.
pub(super) fn preempted_polls_are_rescheduled() {
    let mut h = Harness::new();
    let clock = TestClock::new();
    let mut schedule = PollSchedule::default();
    let mut store = InflightStore::in_memory();
    let root = h.root.clone();
    schedule.add(
        root.clone(),
        Policy {
            enabled: true,
            paused: false,
            interval: Duration::from_secs(300),
            backoff: None,
            recovery_suspended: false,
            next_eligible_at: None,
            active_operation: None,
        },
        None,
    );
    let mut used = Vec::new();
    for round in 1..=3 {
        assert_eq!(
            schedule.due(&clock.now()),
            std::slice::from_ref(&root),
            "round {round}"
        );
        h.backend.answer_poll(cancelled());
        let gate = h.backend.hold_poll();
        let sent = scheduler::start(&mut schedule, &mut store, &root, |op, restart| {
            assert!(!restart, "round {round}: a new operation");
            assert!(!used.contains(&op), "round {round}: a new operation ID");
            used.push(op);
            h.worker.submit_poll(root.clone(), op, restart)
        });
        assert!(sent.is_some());
        gate.wait_reached();
        let job = h.worker.submit(h.refresh(), Lane::User, None, None);
        let attempt = h.until_polled();
        h.finished(job);
        h.log.clear();
        assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Preemption));
        scheduler::apply(
            &mut schedule,
            &mut store,
            &clock.now(),
            &attempt,
            ActiveRow::Nothing,
            // The reports carry the wall clock's time, not the test clock's.
            OffsetDateTime::now_utc() - time::Duration::hours(1),
        );
        assert_eq!(store.poll_entry(&root), None, "round {round}: terminal");
        assert_eq!(schedule.repo(&root).unwrap().phase, Phase::Preempted);
        assert!(
            schedule.due(&clock.now()).is_empty(),
            "until the lane empties"
        );
        schedule.user_lane_empty(&clock.now());
        h.join_tasks();
    }
    // The third in a row waits for the interval, to the second.
    assert!(schedule.due(&clock.now()).is_empty());
    clock.advance(Duration::from_secs(299));
    assert!(schedule.due(&clock.now()).is_empty());
    clock.advance(Duration::from_secs(1));
    assert_eq!(schedule.due(&clock.now()), [root]);
}

/// P-5: a background job submitted while a poll runs cancels nothing.
pub(super) fn background_job_cancels_nothing() {
    let mut h = Harness::new();
    h.backend.answer_poll(polled(PollCompletion::Completed));
    let op = OperationId::new();
    let gate = h.hold_poll(op, false);
    let job = h.worker.submit(h.refresh(), Lane::Background, None, None);
    gate.release();
    let attempt = h.until_polled();
    h.finished(job);
    assert_eq!(attempt.answer, Answer::Completed);
    assert_eq!(h.preempts(), 0);
    assert!(h.backend.cancels().is_empty());
}

/// P-6: a background job that is running when a user job arrives is not
/// cancelled, and neither is anything else (decision 20). A background
/// `IndexRefresh` stands for the retirement, `Intent::ContextRetire` coming
/// with Task 11a: both are background jobs the admission step ignores.
pub(super) fn running_background_job_is_left_alone() {
    let mut h = Harness::new();
    let held = h.backend.inner().hold_job(JobKind::IndexRefresh);
    let background = h.worker.submit(h.refresh(), Lane::Background, None, None);
    held.wait_reached();
    let user = h.worker.submit(h.refresh(), Lane::User, None, None);
    let running = h.worker.running_job().expect("the background job runs");
    assert_eq!(running.id, background);
    assert!(!running.cancelled, "its token is not cancelled");
    held.release();
    h.finished(background);
    h.finished(user);
    assert_eq!(h.preempts(), 0);
    assert!(h.backend.cancels().is_empty());
    assert!(
        h.position(|e| matches!(e, Heard::Finished(id) if *id == background))
            < h.position(|e| matches!(e, Heard::Finished(id) if *id == user)),
        "it ran to its end first"
    );
}

/// P-7: quit and a reconfiguration use the same task and flag. They ask
/// through `ask_poll_cancel`; a user job already asked, so the second ask
/// sends no second event and only names the firmer cause. Quit's own
/// `quit()` closes the queue, and a poll not yet at `poll_automatic` does
/// not make it.
pub(super) fn quit_and_reconfiguration_share_the_task() {
    // Quit.
    let mut h = Harness::new();
    h.backend.answer_poll(cancelled());
    let op = OperationId::new();
    let gate = h.hold_poll(op, false);
    assert!(h.worker.ask_poll_cancel(CancelCause::Quit));
    let attempt = h.until_polled();
    let _ = &gate;
    assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Quit));
    assert_eq!(h.preempts(), 1);
    assert!(!h.worker.ask_poll_cancel(CancelCause::Quit), "no poll runs");
    h.join_tasks();

    // A user job, then a reconfiguration: one task, the firmer cause.
    let mut h = Harness::new();
    h.backend.answer_poll(cancelled());
    let gate = h.hold_poll(OperationId::new(), false);
    let job = h.worker.submit(h.refresh(), Lane::User, None, None);
    assert!(h.worker.ask_poll_cancel(CancelCause::Reconfiguration));
    assert!(h.worker.ask_poll_cancel(CancelCause::Preemption));
    let attempt = h.until_polled();
    h.finished(job);
    let _ = &gate;
    assert_eq!(
        attempt.answer,
        Answer::Cancelled(CancelCause::Reconfiguration)
    );
    assert_eq!(h.preempts(), 1, "the same task");
    assert_eq!(h.join_tasks().len(), 1);

    // Quit while the poll has not reached `poll_automatic`.
    let mut h = Harness::new();
    h.backend.answer_poll(cancelled());
    let held = h.backend.inner().hold_job(JobKind::Poll);
    h.worker
        .submit_poll(h.root.clone(), OperationId::new(), false)
        .expect("the queue is open");
    held.wait_reached();
    h.worker.quit();
    held.release();
    let attempt = h.until_polled();
    assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Quit));
    assert!(h.backend.requests().is_empty());

    // Nobody asked, the library cancelled: a launch takeover's stored
    // cancel.
    let mut h = Harness::new();
    h.backend.answer_poll(cancelled());
    h.worker
        .submit_poll(h.root.clone(), OperationId::new(), true)
        .expect("the queue is open");
    let attempt = h.until_polled();
    assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Takeover));
    assert_eq!(h.preempts(), 0);
}
