//! The scheduler entity (D6 design B6, R-16, R-27): one per `Services`,
//! shared by every clone and window (A17).
//!
//! It owns the [`PollSchedule`], the in-flight record and the clock, and
//! drives them with the pure decisions of the parent module:
//!
//! - **Which repositories it polls.** Only those whose remotes show an SSH
//!   publication remote (U-2). A repository without one is not in the
//!   schedule: nothing to check, and a poll without a remote writes state
//!   (Fact 31). Policies and remotes are read again on window activation,
//!   on a clock event and after a reconfiguration; never on the UI thread.
//! - **When.** One timer, armed to the model's `next_wake` (at most
//!   [`LONGEST_SLEEP`] ahead, so a suspend or jump is seen soon after it).
//!   Every wake, answer, listing and activation runs [`Scheduler::tick`]:
//!   it looks at the clock, reads stale policies again at once (the model's
//!   `next_wake` is `None` while one is stale) and submits at most one
//!   poll, the fairest due one. Activation polls nothing by itself.
//! - **The answer.** Each `Polled` the drain hands over is applied after a
//!   fresh `resolve` and `polling_status`, whose `active_operation_id` tells
//!   the in-flight record whether the row is still the entry's (R-31). What
//!   the poll brought in marks its repository changed (R-10); an
//!   `IndexPending` answer queues one background `IndexRefresh`.
//! - **Reconfiguration** (R-11, `reconfigure`).
//! - **Quit** (R-33): [`Scheduler::set_quitting`] stops it arming timers and
//!   submitting until Cancel.
//!
//! Nothing here holds a title, body, comment or key material.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use gpui_kit::{Context, Task, WeakEntity};
use manyhands::repository::OperationId;
use time::OffsetDateTime;

use super::{
    Holding, holding,
    reconfigure::{Heard, Moment},
    registration::{self, Read},
    start,
};
use crate::desktop::{
    model::{
        operations::Operations,
        polling::{
            Authority, Clock, ClockEvent, Now, PollAttempt, PollSchedule, Root, SystemClock,
            status::Unplaced,
        },
    },
    services::{
        Backend,
        inflight::{Acquired, InflightStore},
        poll::PollInbox,
        worker::WorkerHandle,
    },
};

/// The longest the timer sleeps, so the two clocks are compared often
/// enough to see a suspend or a jump soon after it (A3). Waking costs no
/// I/O when nothing is due.
pub(crate) const LONGEST_SLEEP: Duration = Duration::from_secs(30);

/// The wait when something is already due but cannot start yet (an answer
/// being applied, quitting): a backstop, since the end of each of those
/// ticks again.
const BLOCKED_SLEEP: Duration = Duration::from_secs(1);

pub(crate) struct Scheduler {
    pub(super) backend: Arc<dyn Backend>,
    pub(super) worker: WorkerHandle,
    pub(super) operations: WeakEntity<Operations>,
    pub(super) inbox: PollInbox,
    /// The polling switch (R-27), shared with `Services` and its drain.
    pub(super) on: Arc<AtomicBool>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) schedule: PollSchedule,
    pub(super) store: InflightStore,
    /// The repositories as the list last showed them.
    pub(super) known: BTreeSet<Root>,
    /// The newest read of each repository still to land, by sequence: an
    /// older read that lands later is dropped.
    pub(super) reading: BTreeMap<Root, u64>,
    pub(super) reads: u64,
    pub(super) launched_at: Option<OffsetDateTime>,
    /// Attempts the drain handed over and the one being applied.
    pub(super) answers: VecDeque<(Root, PollAttempt)>,
    pub(super) answering: bool,
    /// The clocks as of the last look, for clock events (A3).
    pub(super) seen: Option<Now>,
    pub(super) quitting: bool,
    pub(super) timer: Option<Task<()>>,
    pub(super) armed_for: Option<Instant>,
    /// Repositories whose last read failed. They keep their old policy and
    /// are not read again by the tick, or a stale one would be read in a
    /// loop. The next activation, clock event or reconfiguration asks for
    /// its read itself, and that read's result replaces this entry.
    pub(super) failed_reads: BTreeSet<Root>,
    /// The host each scheduled repository's publication remote is reached
    /// at, as the last read showed it (`None` when it did not say). Its
    /// keys are the scheduled repositories.
    pub(super) authorities: BTreeMap<Root, Option<Authority>>,
}

impl Scheduler {
    pub(crate) fn new(
        backend: Arc<dyn Backend>,
        worker: WorkerHandle,
        operations: WeakEntity<Operations>,
        inbox: PollInbox,
        on: Arc<AtomicBool>,
    ) -> Self {
        let store = InflightStore::open(backend.service().data_directory());
        Self {
            backend,
            worker,
            operations,
            inbox,
            on,
            clock: Arc::new(SystemClock),
            schedule: PollSchedule::default(),
            store,
            known: BTreeSet::new(),
            reading: BTreeMap::new(),
            reads: 0,
            launched_at: None,
            answers: VecDeque::new(),
            answering: false,
            seen: None,
            quitting: false,
            timer: None,
            armed_for: None,
            failed_reads: BTreeSet::new(),
            authorities: BTreeMap::new(),
        }
    }

    /// The schedule: what each polled repository's status and attention
    /// read (Tasks 7 and 8).
    pub(crate) fn schedule(&self) -> &PollSchedule {
        &self.schedule
    }

    /// Why `root`'s stale row is not taken over yet (L-2), for the status
    /// text.
    pub(crate) fn holding(&self, root: &Root) -> Option<Holding> {
        holding(&self.schedule, &self.store, root, &self.clock.now())
    }

    /// Why `root` has no schedule yet, when that is not because it has no
    /// SSH publication remote: its remotes are not read yet, or the read
    /// failed. `None` once placed or while polling is off (R-27), so a
    /// repository without a schedule then is one with nothing to check.
    pub(crate) fn unplaced(&self, root: &Root) -> Option<Unplaced> {
        if !self.on() || self.schedule.repo(root).is_some() {
            return None;
        }
        if self.reading.contains_key(root) || !self.known.contains(root) {
            Some(Unplaced::Reading)
        } else if self.failed_reads.contains(root) {
            Some(Unplaced::Unreadable)
        } else {
            None
        }
    }

    /// Whether `root`'s policy is about to change: it is being read again
    /// or is known to be out of date, so a view of it should not trust what
    /// it shows.
    pub(crate) fn refreshing(&self, root: &Root) -> bool {
        self.reading.contains_key(root) || self.schedule.repo(root).is_some_and(|repo| repo.stale)
    }

    /// The wall time by the scheduler's clock, for a status line's "ago".
    pub(crate) fn utc(&self) -> OffsetDateTime {
        self.clock.utc()
    }

    #[cfg(test)]
    pub(crate) fn use_clock(&mut self, clock: Arc<dyn Clock>) {
        self.clock = clock;
        self.seen = None;
    }

    #[cfg(test)]
    pub(crate) fn schedule_mut(&mut self) -> &mut PollSchedule {
        &mut self.schedule
    }

    /// The monotonic instant the timer is armed for, if it is.
    #[cfg(test)]
    pub(crate) fn armed_for(&self) -> Option<Instant> {
        self.armed_for
    }

    #[cfg(test)]
    pub(crate) fn store(&self) -> &InflightStore {
        &self.store
    }

    pub(super) fn on(&self) -> bool {
        self.on.load(Ordering::Relaxed)
    }

    // The repository list.

    /// The repository list loaded or changed (the one call beside the
    /// launch refresh). The first call is the launch: the in-flight lock is
    /// tried once, every repository is read, and each with an SSH
    /// publication remote and a due stored deadline is polled, fairest
    /// first and one at a time (decision 23, A11). A repository the list no
    /// longer shows loses its schedule (R-11).
    pub(crate) fn listed(&mut self, roots: &[Root], cx: &mut Context<Self>) {
        if !self.on() {
            return;
        }
        let launching = self.launched_at.is_none();
        if launching {
            self.launched_at = Some(self.clock.utc());
            self.acquire(cx);
        }
        let now: BTreeSet<Root> = roots.iter().cloned().collect();
        let gone: Vec<Root> = self.known.difference(&now).cloned().collect();
        for root in gone {
            self.forget(&root);
        }
        let added: Vec<Root> = now.difference(&self.known).cloned().collect();
        self.known = now;
        // All at once, so the first poll is the fairest of every repository
        // the list shows, not of the first to be read (R-7).
        self.read_registrations(added, cx);
        self.publish(cx);
        self.tick(cx);
    }

    /// The window became active: the in-flight lock is tried again (once
    /// held, the stale rows of the previous holder are finished, L-1), and
    /// every policy and remote is read again, so a remote added through the
    /// CLI is seen. Nothing is polled by this alone.
    pub(crate) fn activated(&mut self, cx: &mut Context<Self>) {
        if !self.on() {
            return;
        }
        self.acquire(cx);
        let roots = self.known.iter().cloned().collect();
        self.read_registrations(roots, cx);
        self.tick(cx);
    }

    fn acquire(&mut self, cx: &mut Context<Self>) {
        if self.store.try_acquire() == Acquired::Now {
            // The entries the previous holder left are now ours to finish:
            // `plan` takes them over when each repository is due (L-1), and
            // their rows are not offered Resume.
            self.publish(cx);
        }
    }

    /// Reads `roots`: policy and remotes, together, off the UI thread.
    pub(super) fn read_registrations(&mut self, roots: Vec<Root>, cx: &mut Context<Self>) {
        if roots.is_empty() {
            return;
        }
        let mut asked = Vec::with_capacity(roots.len());
        for root in roots {
            self.reads += 1;
            self.reading.insert(root.clone(), self.reads);
            asked.push((root, self.reads));
        }
        let backend = self.backend.clone();
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_executor()
                .spawn(async move {
                    asked
                        .into_iter()
                        .map(|(root, sequence)| {
                            let read = registration::read(backend.as_ref(), &root);
                            (root, sequence, read)
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            // The scheduler lives as long as the services.
            let _ = this.update(cx, |scheduler, cx| scheduler.registered(read, cx));
        })
        .detach();
    }

    fn registered(&mut self, reads: Vec<(Root, u64, Read)>, cx: &mut Context<Self>) {
        for (root, sequence, read) in reads {
            if self.reading.get(&root) != Some(&sequence) {
                // A newer read was asked since, or the repository is gone.
                continue;
            }
            self.reading.remove(&root);
            if matches!(read, Read::Failed) {
                self.failed_reads.insert(root.clone());
            } else {
                self.failed_reads.remove(&root);
            }
            match read {
                Read::Polled {
                    policy,
                    last_completed,
                    authority,
                } => {
                    self.authorities.insert(root.clone(), authority);
                    if self.schedule.repo(&root).is_some() {
                        self.schedule.set_policy(&root, policy);
                    } else {
                        self.schedule.add(root, policy, last_completed);
                    }
                }
                Read::NotPolled => self.forget_schedule(&root),
                Read::Failed => {}
            }
        }
        self.publish(cx);
        self.tick(cx);
    }

    /// The repository leaves the list or the registry.
    pub(super) fn forget(&mut self, root: &Root) {
        self.reading.remove(root);
        self.failed_reads.remove(root);
        self.forget_schedule(root);
    }

    fn forget_schedule(&mut self, root: &Root) {
        self.schedule.remove(root);
        self.authorities.remove(root);
        if let Some(op) = self.store.poll_entry(root) {
            self.store.poll_withdrawn(root, op);
        }
    }

    // The tick.

    /// Looks at the clocks, reads stale policies again, submits at most one
    /// due poll, and arms the timer. Everything that can change what is due
    /// ends here.
    pub(crate) fn tick(&mut self, cx: &mut Context<Self>) {
        if !self.on() {
            return;
        }
        let now = self.clock.now();
        self.watch_clock(&now, cx);
        if !self.worker.in_flight(|_| true) {
            self.schedule.user_lane_empty(&now);
        }
        // The model's `next_wake` is `None` while a policy is stale: it is
        // read again now, never waited for.
        let stale: Vec<Root> = self
            .schedule
            .stale()
            .into_iter()
            .filter(|root| !self.reading.contains_key(root) && !self.failed_reads.contains(root))
            .collect();
        self.read_registrations(stale, cx);
        let free = !self.quitting && self.answers.is_empty() && !self.answering;
        if free && let Some(root) = self.schedule.due(&now).into_iter().next() {
            self.send(root, cx);
        }
        self.arm(&now, cx);
        // The Settings page reads "Checked 5 minutes ago" from the clock.
        cx.notify();
    }

    /// A suspend or a jump: the clocks disagree, so every policy and remote
    /// is read again (A3, R-6).
    fn watch_clock(&mut self, now: &Now, cx: &mut Context<Self>) {
        let Some(seen) = self.seen.replace(*now) else {
            return;
        };
        let utc = now.utc - seen.utc;
        let mono = now.mono.saturating_duration_since(seen.mono);
        if self.schedule.clock_event(utc, mono) == ClockEvent::Jumped {
            let roots = self.known.iter().cloned().collect();
            self.read_registrations(roots, cx);
        }
    }

    /// Sends the poll of the due repository `root`.
    fn send(&mut self, root: Root, cx: &mut Context<Self>) {
        let worker = self.worker.clone();
        let target = root.clone();
        // `start` writes the in-flight entry, marks the model queued and
        // only then submits. A queue that quitting closed gives the entry
        // and the mark back.
        start(
            &mut self.schedule,
            &mut self.store,
            &root,
            |operation, restart| worker.submit_poll(target, operation, restart),
        );
        self.publish(cx);
    }

    fn arm(&mut self, now: &Now, cx: &mut Context<Self>) {
        self.timer = None;
        self.armed_for = None;
        if !self.on() || self.quitting || self.known.is_empty() {
            return;
        }
        let wait = match self.schedule.next_wake(now) {
            Some(at) if at > now.mono => (at - now.mono).min(LONGEST_SLEEP),
            Some(_) => BLOCKED_SLEEP,
            None => LONGEST_SLEEP,
        };
        self.armed_for = Some(now.mono + wait);
        let executor = cx.background_executor().clone();
        self.timer = Some(cx.spawn(async move |this, cx| {
            executor.timer(wait).await;
            let _ = this.update(cx, |scheduler, cx| scheduler.tick(cx));
        }));
    }

    // Answers.

    /// The drain heard something: requests that reconfigure polling, polls
    /// that ended. Polls nothing by itself except through the tick.
    pub(crate) fn heard(&mut self, heard: Vec<Heard>, cx: &mut Context<Self>) {
        if !self.on() {
            return;
        }
        for heard in heard {
            match heard {
                Heard::Reconfiguration(Moment::Submitted, scope) => {
                    let reached = self.reached(&scope);
                    self.drop_queued_retirements(&reached);
                }
                Heard::Reconfiguration(Moment::Finished, scope) => self.reconfigured(scope, cx),
                Heard::PolicyMoved(root) => self.read_registrations(vec![root], cx),
            }
        }
        self.answers.extend(self.inbox.take());
        self.next_answer(cx);
        self.tick(cx);
    }

    // Quit (R-33).

    /// From the entry to `request_quit` the scheduler starts nothing and
    /// arms no timer; Cancel on either ask resumes it.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 10.
    pub(crate) fn set_quitting(&mut self, quitting: bool, cx: &mut Context<Self>) {
        self.quitting = quitting;
        if !quitting {
            self.schedule.resume_stopped();
        }
        self.tick(cx);
    }

    // The desktop's own polls (Task 4 review I-2).

    /// Tells the request model which operations are this desktop's own
    /// automatic polls, so their pending rows offer no Resume.
    pub(super) fn publish(&mut self, cx: &mut Context<Self>) {
        let mut own: HashSet<OperationId> =
            self.store.polls().into_iter().map(|(_, op)| op).collect();
        own.extend(
            self.known
                .iter()
                .filter_map(|root| self.schedule.restart_of(root)),
        );
        // The record's own problem is heard once, wherever it arose (R-31).
        let report = self.store.take_report();
        let _ = self.operations.update(cx, |operations, cx| {
            let mut changed = operations.set_automatic_polls(own);
            if let Some(report) = report {
                changed |= operations.note_record(report);
            }
            if changed {
                cx.notify();
            }
        });
        // The Settings page reads the schedule this follows.
        cx.notify();
    }
}
