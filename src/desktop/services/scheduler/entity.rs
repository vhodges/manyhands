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
use manyhands::repository::{OperationId, PollingOutcome};
use time::OffsetDateTime;

use super::{
    Holding, holding,
    reconfigure::{Heard, Moment},
    registration::{self, Read, SharedCopy},
    start,
};
use crate::desktop::{
    model::{
        operations::{Entered, Operations},
        polling::{
            Authority, Clock, ClockEvent, Now, PollAttempt, PollSchedule, Root, SystemClock,
            retire_loop::RetireLoops, status::Unplaced,
        },
    },
    services::{
        Backend,
        drafts::DraftRegistry,
        edit_tabs::EditTabs,
        inflight::{Acquired, InflightStore},
        poll::PollInbox,
        requests::DesktopRequestId,
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
    /// How the library's latest check of each scheduled repository ended,
    /// as the last registration read or answer's status showed it. It
    /// speaks for a repository this process has not checked (Part A).
    pub(super) stored: BTreeMap<Root, PollingOutcome>,
    /// The publication remote each scheduled repository is checked
    /// against, from the same read as `authorities`.
    pub(super) shared_copies: BTreeMap<Root, SharedCopy>,
    /// Repositories whose Check now waits for the change the person was
    /// sent to make (Choose a key…, Approve host…, Open remote settings…):
    /// it is sent when a reconfiguration reaching them finishes (C3).
    pub(super) check_after: BTreeSet<Root>,
    /// The open item tabs, which the retirement loop asks whether an item
    /// is being edited (RT-3).
    pub(super) edit_tabs: WeakEntity<EditTabs>,
    /// The draft registry, which it asks what the store holds (RT-4, RT-5)
    /// and to list the store again (RT-6).
    pub(super) drafts: WeakEntity<DraftRegistry>,
    /// Each repository's retirement loop (R-32).
    pub(super) loops: RetireLoops,
    /// The mark ("Retiring…") of the item each loop is checking.
    pub(super) marked: BTreeMap<Root, u64>,
    /// The retirement requests the loops sent and have not heard end, by
    /// repository.
    pub(super) sent: BTreeMap<DesktopRequestId, Root>,
    /// Moves whenever a retirement's entry begins, answers or settles: a
    /// pending-operations read that began before the last move may predate
    /// it (`reconcile`).
    pub(super) entry_epoch: u64,
    /// The failed Check nows whose answer `check_failed` applied to the
    /// schedule (Activity N4; R-58). A request is added when its answer
    /// moved the repository, never computed from the code.
    pub(super) checks_applied: BTreeSet<DesktopRequestId>,
    /// Each failed Check now `check_failed` heard, with its repository and the
    /// repository's completed attempts at that moment (N22 clause 4).
    pub(super) checks_heard: BTreeMap<DesktopRequestId, (Root, u64)>,
}

impl Scheduler {
    pub(crate) fn new(
        backend: Arc<dyn Backend>,
        worker: WorkerHandle,
        operations: WeakEntity<Operations>,
        inbox: PollInbox,
        on: Arc<AtomicBool>,
        edit_tabs: WeakEntity<EditTabs>,
        drafts: WeakEntity<DraftRegistry>,
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
            stored: BTreeMap::new(),
            shared_copies: BTreeMap::new(),
            check_after: BTreeSet::new(),
            edit_tabs,
            drafts,
            loops: RetireLoops::default(),
            marked: BTreeMap::new(),
            sent: BTreeMap::new(),
            entry_epoch: 0,
            checks_applied: BTreeSet::new(),
            checks_heard: BTreeMap::new(),
        }
    }

    /// Whether `check_failed` applied the failed Check now `id`'s answer to
    /// the schedule (Activity's `RequestFacts.check_applied`; N4, R-58).
    pub(crate) fn check_applied(&self, id: DesktopRequestId) -> bool {
        self.checks_applied.contains(&id)
    }

    /// Whether the repository of the failed Check now `id` has applied a
    /// completed attempt since `check_failed` heard it (N22 clause 4).
    pub(crate) fn check_superseded(&self, id: DesktopRequestId) -> bool {
        self.checks_heard
            .get(&id)
            .is_some_and(|(root, heard)| self.schedule.completed_attempts(root) > *heard)
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
    /// it shows. Not when its last read failed: nothing is on the way, and
    /// a view follows the last policy it knows until a later activation or
    /// clock event reads again.
    pub(crate) fn refreshing(&self, root: &Root) -> bool {
        self.reading.contains_key(root)
            || (!self.failed_reads.contains(root)
                && self.schedule.repo(root).is_some_and(|repo| repo.stale))
    }

    /// How the library's latest check of `root` ended, as stored.
    pub(crate) fn stored(&self, root: &Root) -> Option<PollingOutcome> {
        self.stored.get(root).copied()
    }

    /// The publication remote `root` is checked against.
    pub(crate) fn shared_copy(&self, root: &Root) -> Option<&SharedCopy> {
        self.shared_copies.get(root)
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
            self.forget(&root, cx);
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
        // A row settled while the window was away ends its entry: taking the
        // lock reads the list itself, so only a lock already held reads it
        // here.
        if !self.acquire(cx) {
            self.reconcile_entries(cx);
        }
        let roots = self.known.iter().cloned().collect();
        self.read_registrations(roots, cx);
        self.tick(cx);
    }

    /// Reads `root`'s registration again, for Retry on a repository the
    /// scheduler could not read (R-19): it leaves the failed ones, which the
    /// tick does not read, and asks for its own read. A root the scheduler
    /// never listed is ignored (R-41).
    pub(crate) fn reread(&mut self, root: Root, cx: &mut Context<Self>) {
        if !self.known.contains(&root) {
            return;
        }
        self.failed_reads.remove(&root);
        // The row's facts changed: it is no longer a failed read.
        cx.notify();
        self.read_registrations(vec![root], cx);
    }

    /// Tries the in-flight lock. True when it was taken now, which also
    /// reconciles the entries (it reads their lists).
    fn acquire(&mut self, cx: &mut Context<Self>) -> bool {
        if self.store.try_acquire() == Acquired::Now {
            // The entries the previous holder left are now ours to finish:
            // `plan` takes them over when each repository is due (L-1), and
            // their rows are not offered Resume.
            self.publish(cx);
            self.reconcile_entries(cx);
            return true;
        }
        false
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
                    outcome,
                    shared_copy,
                } => {
                    self.authorities.insert(root.clone(), authority);
                    self.keep_stored(&root, outcome);
                    match shared_copy {
                        Some(copy) => self.shared_copies.insert(root.clone(), copy),
                        None => self.shared_copies.remove(&root),
                    };
                    if self.schedule.repo(&root).is_some() {
                        self.schedule.set_policy(&root, policy);
                    } else {
                        self.schedule.add(root.clone(), policy, last_completed);
                    }
                    // A repository the library holds until recovery is
                    // listed as attention, like any other (decision 10).
                    self.schedule.sync_recovery(&root);
                }
                Read::NotPolled => self.forget_schedule(&root, cx),
                Read::Failed => {}
            }
        }
        self.publish(cx);
        self.tick(cx);
    }

    /// The latest stored outcome a status read showed for `root`.
    pub(super) fn keep_stored(&mut self, root: &Root, outcome: Option<PollingOutcome>) {
        match outcome {
            Some(outcome) => self.stored.insert(root.clone(), outcome),
            None => self.stored.remove(root),
        };
    }

    /// The repository leaves the list or the registry.
    pub(super) fn forget(&mut self, root: &Root, cx: &mut Context<Self>) {
        self.reading.remove(root);
        self.failed_reads.remove(root);
        self.forget_schedule(root, cx);
    }

    fn forget_schedule(&mut self, root: &Root, cx: &mut Context<Self>) {
        // Its loop has nothing left to poll after (R-32).
        self.end_loop(root, cx);
        self.schedule.remove(root);
        self.check_after.remove(root);
        self.authorities.remove(root);
        self.stored.remove(root);
        self.shared_copies.remove(root);
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
        self.prune_arms();
        self.watch_clock(&now, cx);
        if !self.worker.user_lane_busy() {
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
        let mut changed = false;
        for heard in heard {
            match heard {
                // A finished share or save changes the unshared list at
                // once, timed when heard, in the order the drain heard them
                // (rules 1 and 2), before any older answer applied later.
                Heard::Shared { root, item, main } => {
                    let at = self.clock.utc();
                    changed |= self.schedule.shared(&root, item.as_ref(), main, at);
                }
                Heard::Saved {
                    root,
                    item,
                    kind,
                    new,
                } => {
                    let at = self.clock.utc();
                    changed |= self.schedule.saved(&root, item, kind, new, at);
                }
                Heard::MainAhead { root } => {
                    let at = self.clock.utc();
                    changed |= self.schedule.main_ahead(&root, at);
                }
                Heard::Retirement(retiring) => self.retiring(retiring, cx),
                Heard::Reconfiguration(Moment::Submitted, scope) => {
                    let reached = self.reached(&scope);
                    self.drop_queued_retirements(&reached, cx);
                }
                Heard::Reconfiguration(moment @ (Moment::Finished | Moment::Succeeded), scope) => {
                    self.reconfigured(scope, moment == Moment::Succeeded, cx)
                }
                Heard::PolicyMoved(root) => self.read_registrations(vec![root], cx),
                Heard::ResendRetirement {
                    resume,
                    request,
                    root,
                    kind,
                    item,
                } => self.resend_resumed(resume, request, &root, kind, &item, cx),
                Heard::PendingEnded {
                    root,
                    operation,
                    finished,
                } => self.pending_ended(&root, operation, finished, cx),
                Heard::CheckedNow(root, attempt) => self.checked_now(root, attempt, cx),
                Heard::CheckFailed {
                    root,
                    request,
                    code,
                    authority,
                } => self.check_failed(&root, request, code, authority, cx),
            }
        }
        if changed {
            cx.notify();
        }
        self.answers.extend(self.inbox.take());
        self.next_answer(cx);
        self.tick(cx);
    }

    // Quit (R-33).

    /// From the entry to `request_quit` the scheduler starts nothing and
    /// arms no timer; Cancel on either ask resumes it.
    ///
    /// Asking to quit drops every retirement loop: it started nothing more
    /// from here, and Cancel must not find a repository left in
    /// `Phase::Retiring`. What the loops had left waits for the next
    /// completed attempt. A retirement already running finishes (decision
    /// 28) and finds no loop.
    pub(crate) fn set_quitting(&mut self, quitting: bool, cx: &mut Context<Self>) {
        self.quitting = quitting;
        if quitting {
            self.end_loops(cx);
        } else {
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
        // The polls queued or running now: their rows are in progress, not
        // pending (R-23).
        let running: HashSet<OperationId> = self
            .known
            .iter()
            .filter(|root| {
                self.schedule
                    .repo(root)
                    .is_some_and(|repo| repo.in_flight())
            })
            .filter_map(|root| self.store.poll_entry(root))
            .collect();
        // The items whose retirement the record holds, queued, running or
        // pending, are "Retiring…" (decision 19, RT-1).
        let entered: Vec<_> = self
            .store
            .retirements()
            .iter()
            .filter_map(|entry| {
                let (kind, item) = entry.item()?;
                Some(Entered {
                    root: entry.root().clone(),
                    kind,
                    item,
                    request: entry.request(),
                    operation: entry.operation(),
                })
            })
            .collect();
        // The record's own problem is heard once, wherever it arose (R-31).
        let report = self.store.take_report();
        let _ = self.operations.update(cx, |operations, cx| {
            let mut changed = operations.set_automatic_polls(own);
            changed |= operations.set_running_polls(running);
            changed |= operations.set_retirement_entries(entered);
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
