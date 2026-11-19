//! What stands in a writer's way when another operation's row does, and
//! how long the writer waits for it.
//!
//! A call that would begin an operation is refused while another
//! operation of the repository has a row that is not finished; see
//! `recovery::in_the_way`. The refusal names that operation: a `Blocker`.
//! Whether its holder still works is judged as an operation read judges
//! it, from the row the read gives (`RepositoryService::blocking_operation`,
//! `liveness::holder_liveness`), with the service's process probe.
//!
//! A writer that begins its row with `begin_lifecycle` waits for a live
//! holder: it lets go of the repository lease, and its registry
//! transaction has ended, before it sleeps, and takes the lease again
//! before it looks again. So it never holds anything a peer needs to
//! finish. Still refused once `WaitPolicy::budget` has passed, it answers
//! `busy`, naming the holder. A stale holder is not waited for: the
//! refusal stays `recovery_required` and names it. Neither leaves a row.
//!
//! A new synchronization, the publication of a comment, an `item sync`, a
//! `repo sync` or a republish, waits the same way, under the same
//! `WaitPolicy`, for another operation that is in the way of the
//! repository's remote reservation while that operation's holder is live;
//! see `RepositoryService::reserving`. Two
//! things are waited for there:
//!
//! - the holder of the reservation (another synchronization's row, family
//!   `remote`), and
//! - a live peer's unfinished local row, which refuses the new
//!   publication's reservation `recovery_required`: a peer's index
//!   hand-off, which it begins just after it lets the reservation go
//!   (family `local`, action `refresh`).
//!
//! The wait of the reservation takes a fresh budget: a comment add that
//! waited in `begin_lifecycle` may wait again here, up to the budget once
//! more, not for what was left of the first. A stale holder is answered at
//! once, as for a begin.

use std::{path::Path, sync::Arc, time::Duration};

use time::OffsetDateTime;

use super::{
    OperationFamily, OperationId, RepositoryError, RepositoryErrorKind, RepositoryService,
};

/// The operation whose unfinished row a call was refused for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocker {
    /// The journal whose row it is: `Local` or `Remote`.
    pub family: OperationFamily,
    pub operation_id: OperationId,
    /// Whether its holder still worked when the call that was refused
    /// judged it. `None` until a call judges it with the service's probe.
    pub live: Option<bool>,
    /// The row as a waiting call last read it while its holder worked:
    /// what names the operation in the answer when the answer's own read of
    /// the row fails.
    pub(crate) seen: Option<Box<super::OperationDto>>,
}

impl Blocker {
    /// The local row of `operation_id`, as its journal stores the ID. A
    /// row recorded before operations had IDs names no operation, and none
    /// is named for it.
    pub(crate) fn local(operation_id: Option<&str>) -> Option<Self> {
        Self::of(OperationFamily::Local, operation_id?)
    }

    /// The remote row of `operation_id`, as its journal stores the ID.
    pub(crate) fn remote(operation_id: &str) -> Option<Self> {
        Self::of(OperationFamily::Remote, operation_id)
    }

    fn of(family: OperationFamily, operation_id: &str) -> Option<Self> {
        Some(Self {
            family,
            operation_id: OperationId::parse(operation_id).ok()?,
            live: None,
            seen: None,
        })
    }

    /// The operation of `operation`, a row a look found live, named live.
    /// `None` for a row that names no operation.
    fn seen_live(operation: &super::OperationDto) -> Option<Self> {
        Some(Self {
            family: operation.family,
            operation_id: OperationId::parse(operation.operation_id.as_deref()?).ok()?,
            live: Some(true),
            seen: Some(Box::new(operation.clone())),
        })
    }

    fn judged(self, live: bool) -> Self {
        Self {
            live: Some(live),
            ..self
        }
    }
}

/// Who is at work in a repository, as a writer that could not take a
/// lease or the index's lock in time finds it; see
/// `RepositoryService::live_holder`.
#[derive(Debug)]
pub(crate) enum LiveHolder {
    /// The first unfinished operation whose holder still works, named live,
    /// with its row as the read found it.
    Live(Blocker),
    /// No unfinished operation has a holder at work.
    Nothing,
    /// The repository's operations could not be read.
    Unreadable,
}

/// What the read of a blocker's row found.
#[derive(Debug)]
pub(crate) enum BlockerRow {
    /// Its row, and whether that row is still unfinished.
    Found {
        operation: Box<super::OperationDto>,
        unfinished: bool,
    },
    /// No row of the operation is there.
    Gone,
    /// The row could not be read, in every try.
    Unreadable,
}

/// How a waiting writer finds the operation in its way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Judged {
    /// Its row is unfinished and its holder still works.
    Live,
    /// Its row is unfinished and nothing works under it.
    Stale,
    /// Its row has finished, or is gone, since the refusal: the next look
    /// finds the way clear, or another operation in it.
    Away,
    /// Its row could not be read. That says nothing of its holder, so it is
    /// waited for as a live one is: a writer never goes past a row it
    /// could not judge.
    Unreadable,
}

/// What one look at a blocker found: how it stands, and, for a live one,
/// its row as the look read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Judgment {
    pub(crate) judged: Judged,
    pub(crate) seen: Option<Box<super::OperationDto>>,
}

/// How long a writer waits for a live holder, and how long it sleeps
/// between two looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitPolicy {
    pub budget: Duration,
    pub poll: Duration,
}

impl Default for WaitPolicy {
    fn default() -> Self {
        Self {
            budget: Duration::from_secs(5),
            poll: Duration::from_millis(50),
        }
    }
}

/// What a waiting writer lets time pass with. The service sleeps the
/// thread; a test passes one that moves its own clock instead.
pub trait Sleeper: Send + Sync {
    fn sleep(&self, duration: Duration);
}

/// The thread's own sleep.
pub(crate) struct SystemSleeper;

impl Sleeper for SystemSleeper {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// One writer's wait: when it began by the service's clock, and how many
/// times it has slept. It ends at the budget by either measure, so a clock
/// that does not move cannot make it endless.
pub(crate) struct Wait {
    started: OffsetDateTime,
    polls: u128,
    /// The blocker's row as the last look that could read it found it,
    /// live: `None` once a look found it stale, gone or finished.
    seen_live: Option<Box<super::OperationDto>>,
}

/// What a writer that was refused does next.
pub(crate) enum Waited {
    /// It slept with nothing held, and looks again.
    Again,
    /// It is answered with this.
    Refused(RepositoryError),
}

impl RepositoryService {
    /// The same service, waiting for a live holder as `policy` says.
    #[doc(hidden)]
    pub fn with_wait_policy_for_testing(mut self, policy: WaitPolicy) -> Self {
        self.wait_policy = policy;
        self
    }

    /// The same service, letting the time of a wait pass with `sleeper`.
    #[doc(hidden)]
    pub fn with_sleeper_for_testing(mut self, sleeper: Arc<dyn Sleeper>) -> Self {
        self.sleeper = sleeper;
        self
    }

    /// A wait that begins now.
    pub(crate) fn start_wait(&self) -> Wait {
        Wait {
            started: self.clock.now(),
            polls: 0,
            seen_live: None,
        }
    }

    /// A wait that is spent before it begins: a writer that may not wait
    /// is answered at once, `busy` naming a live blocker and
    /// `recovery_required` naming a stale one, as a wait whose budget is
    /// spent answers.
    pub(crate) fn no_wait(&self) -> Wait {
        Wait {
            started: self.clock.now(),
            polls: u128::MAX,
            seen_live: None,
        }
    }

    /// How the operation `blocker` names is found by the read of its row at
    /// `root`; see `Judged`.
    pub(crate) fn judge_blocker(&self, root: &Path, blocker: &Blocker) -> Judgment {
        let judged = |judged| Judgment { judged, seen: None };
        match self.blocking_operation(root, blocker) {
            BlockerRow::Found {
                operation,
                unfinished: true,
            } => {
                if operation.attribution.live == Some(true) {
                    Judgment {
                        judged: Judged::Live,
                        seen: Some(operation),
                    }
                } else {
                    judged(Judged::Stale)
                }
            }
            BlockerRow::Found {
                operation: _,
                unfinished: false,
            }
            | BlockerRow::Gone => judged(Judged::Away),
            BlockerRow::Unreadable => judged(Judged::Unreadable),
        }
    }

    /// What a writer that was refused with `error` does next. The caller
    /// holds nothing: its lease is let go and its transaction ended.
    ///
    /// A `busy` that names nothing is a lease or the index's lock that could
    /// not be taken in time. While an unfinished operation's holder works,
    /// that holder may be what holds it, as an index owner's heartbeat holds
    /// the index's lock for a moment: it is waited for, and named, as a live
    /// blocker is; see `waited_holding`.
    pub(crate) fn waited(&self, root: &Path, wait: &mut Wait, error: RepositoryError) -> Waited {
        if error.kind == RepositoryErrorKind::RepositoryBusy && error.blocker().is_none() {
            return self.waited_holding(wait, error, || self.live_holder(root));
        }
        self.waited_judging(wait, error, |blocker| self.judge_blocker(root, blocker))
    }

    /// `waited`, for a `busy` that names nothing, with who is at work found
    /// by `find`. With no unfinished operation at work it is answered as it
    /// is, at once: nothing is waited for. With one, it is slept on as a
    /// live blocker is, and the budget then ends in `busy` naming it. One
    /// that cannot be read is slept on too, and is then named by the row
    /// the last look that could read one found live, if any.
    fn waited_holding(
        &self,
        wait: &mut Wait,
        error: RepositoryError,
        find: impl FnOnce() -> LiveHolder,
    ) -> Waited {
        let blocker = match find() {
            LiveHolder::Nothing => return Waited::Refused(error),
            LiveHolder::Live(blocker) => {
                wait.seen_live.clone_from(&blocker.seen);
                Some(blocker)
            }
            LiveHolder::Unreadable => wait.seen_live.as_deref().and_then(Blocker::seen_live),
        };
        if !self.slept(wait) {
            return Waited::Refused(match blocker {
                Some(blocker) => busy(error, blocker),
                None => error,
            });
        }
        Waited::Again
    }

    /// `waited`, with the blocker judged by `judge`.
    ///
    /// An error that names no blocker, or that is not `recovery_required`,
    /// is answered as it is. A stale blocker is named, stale, and the
    /// answer stays `recovery_required`, with no wait. Any other, live, gone
    /// since or unreadable, is slept on, `WaitPolicy::poll` at a time (at
    /// least `SHORTEST_POLL`), and looked at again, until the budget is spent
    /// by the service's clock or by the count of looks, so that a clock that
    /// does not move cannot make it endless. Then the answer is `busy`,
    /// naming the blocker live, with its row as last read, when the last
    /// look that could read it found it live, and with no judgment
    /// otherwise.
    fn waited_judging(
        &self,
        wait: &mut Wait,
        error: RepositoryError,
        judge: impl FnOnce(&Blocker) -> Judgment,
    ) -> Waited {
        let Some(blocker) = error.blocker().cloned() else {
            return Waited::Refused(error);
        };
        if error.kind != RepositoryErrorKind::RecoveryRequired {
            return Waited::Refused(error);
        }
        let Judgment { judged, seen } = judge(&blocker);
        match judged {
            Judged::Stale => {
                return Waited::Refused(error.with_blocker(Some(blocker.judged(false))));
            }
            Judged::Live => wait.seen_live = seen,
            Judged::Away => wait.seen_live = None,
            // What the last look that could read the row found stands.
            Judged::Unreadable => {}
        }
        let live =
            judged == Judged::Live || judged == Judged::Unreadable && wait.seen_live.is_some();
        let blocker = Blocker {
            live: live.then_some(true),
            seen: if live { wait.seen_live.clone() } else { None },
            ..blocker
        };
        if !self.slept(wait) {
            return Waited::Refused(busy(error, blocker));
        }
        Waited::Again
    }

    /// One more sleep of the writer waiting `wait`, while its budget lasts:
    /// it sleeps `WaitPolicy::poll` (at least `SHORTEST_POLL`), with nothing
    /// held, counts it, and is told to look again. Once the budget is spent,
    /// by the service's clock or by the count of looks, so that a clock that
    /// does not move cannot make it endless, it does not sleep and is told
    /// to answer.
    pub(crate) fn slept(&self, wait: &mut Wait) -> bool {
        let policy = self.wait_policy;
        let poll = policy.poll.max(SHORTEST_POLL);
        let polls = policy.budget.as_nanos() / poll.as_nanos();
        let elapsed = (self.clock.now() - wait.started).whole_nanoseconds();
        if wait.polls >= polls
            || elapsed >= i128::try_from(policy.budget.as_nanos()).unwrap_or(i128::MAX)
        {
            return false;
        }
        self.sleeper.sleep(poll);
        wait.polls += 1;
        true
    }
}

/// The shortest sleep between two looks, whatever a policy asks: a writer
/// does not spin on the index.
const SHORTEST_POLL: Duration = Duration::from_millis(1);

/// `busy`, for the operation and root `error` names, naming `blocker`.
fn busy(error: RepositoryError, blocker: Blocker) -> RepositoryError {
    RepositoryError::new(
        error.operation,
        error.root.clone(),
        RepositoryErrorKind::RepositoryBusy,
        "another operation of the repository is still at work; retry",
    )
    .with_blocker(Some(blocker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A clock that moves, by `step` for each sleep, only when the writer
    /// sleeps on it; `step` zero is a clock that never moves.
    struct TestTime {
        nanos: AtomicU64,
        step: Duration,
        slept: AtomicU64,
    }

    impl super::super::mutation::Clock for TestTime {
        fn now(&self) -> OffsetDateTime {
            OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
                + time::Duration::nanoseconds(
                    i64::try_from(self.nanos.load(Ordering::SeqCst)).unwrap(),
                )
        }
    }

    impl Sleeper for TestTime {
        fn sleep(&self, _duration: Duration) {
            self.nanos.fetch_add(
                u64::try_from(self.step.as_nanos()).unwrap(),
                Ordering::SeqCst,
            );
            self.slept.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn blocker() -> Blocker {
        Blocker {
            family: OperationFamily::Local,
            operation_id: OperationId::new(),
            live: None,
            seen: None,
        }
    }

    /// A row as a look that found its holder live read it.
    fn seen() -> Box<super::super::OperationDto> {
        Box::new(super::super::OperationDto {
            operation_id: None,
            family: OperationFamily::Local,
            owner: super::super::OperationOwner::Repository,
            action: super::super::OperationAction::SaveTicket,
            state: "indexing".to_owned(),
            completed_step: None,
            next_action: None,
            item_id: None,
            key_id: None,
            worktree: None,
            updated_at: None,
            failure_code: None,
            attribution: Default::default(),
        })
    }

    fn refused(kind: RepositoryErrorKind, blocker: Option<Blocker>) -> RepositoryError {
        RepositoryError::new(
            super::super::RepositoryOperation::SaveTicket,
            None,
            kind,
            "refused",
        )
        .with_blocker(blocker)
    }

    /// What a writer refused with `error` comes to, its blocker judged in
    /// turn as `judgments` says (the last one again once they run out): the
    /// error it is answered with, how often it slept and how often it judged.
    fn wait_out(
        policy: WaitPolicy,
        step: Duration,
        error: impl Fn() -> RepositoryError,
        judgments: &[Judged],
    ) -> (RepositoryError, u64, usize) {
        let data = tempfile::tempdir().unwrap();
        let time = Arc::new(TestTime {
            nanos: AtomicU64::new(0),
            step,
            slept: AtomicU64::new(0),
        });
        let service = RepositoryService::open_at(data.path())
            .unwrap()
            .with_clock_for_testing(time.clone())
            .with_sleeper_for_testing(time.clone())
            .with_wait_policy_for_testing(policy);
        let mut wait = service.start_wait();
        let judged = std::cell::Cell::new(0_usize);
        for _ in 0..10_000 {
            let judge = |_: &Blocker| {
                let at = judged.get();
                judged.set(at + 1);
                let judged = judgments[at.min(judgments.len() - 1)];
                Judgment {
                    judged,
                    seen: (judged == Judged::Live).then(seen),
                }
            };
            match service.waited_judging(&mut wait, error(), judge) {
                Waited::Again => {}
                Waited::Refused(error) => {
                    return (error, time.slept.load(Ordering::SeqCst), judged.get());
                }
            }
        }
        panic!("the wait did not end");
    }

    #[test]
    fn a_waiting_writer_looks_again_until_its_budget_is_spent_and_only_for_a_blocker_not_stale() {
        use RepositoryErrorKind::{RecoveryRequired, RepositoryBusy};
        let short = WaitPolicy {
            budget: Duration::from_millis(250),
            poll: Duration::from_millis(50),
        };
        let frozen = Duration::ZERO;
        let named = blocker();
        let blocked = || refused(RecoveryRequired, Some(named.clone()));
        // (case, policy, clock step per sleep, error, judgments, answered
        // kind, its blocker's `live` (`None` inside for no judgment, outer
        // `None` for no blocker), sleeps, judgments made)
        type Row<'a> = (
            &'a str,
            WaitPolicy,
            Duration,
            &'a dyn Fn() -> RepositoryError,
            &'a [Judged],
            RepositoryErrorKind,
            Option<Option<bool>>,
            u64,
            usize,
        );
        let rows: [Row<'_>; 9] = [
            (
                "live, with a clock that never moves: budget / poll looks",
                short,
                frozen,
                &blocked,
                &[Judged::Live],
                RepositoryBusy,
                Some(Some(true)),
                5,
                6,
            ),
            (
                "live, the clock moving 100 ms a sleep: the clock ends it",
                short,
                Duration::from_millis(100),
                &blocked,
                &[Judged::Live],
                RepositoryBusy,
                Some(Some(true)),
                3,
                4,
            ),
            (
                "stale: no sleep, recovery_required naming it stale",
                short,
                frozen,
                &blocked,
                &[Judged::Stale],
                RecoveryRequired,
                Some(Some(false)),
                0,
                1,
            ),
            (
                "unreadable: looked at again, then busy with no judgment",
                short,
                frozen,
                &blocked,
                &[Judged::Unreadable],
                RepositoryBusy,
                Some(None),
                5,
                6,
            ),
            (
                "live, then unreadable: busy, live by the last read that worked",
                short,
                frozen,
                &blocked,
                &[Judged::Live, Judged::Unreadable],
                RepositoryBusy,
                Some(Some(true)),
                5,
                6,
            ),
            (
                "gone since, then stale: one sleep, then recovery_required",
                short,
                frozen,
                &blocked,
                &[Judged::Away, Judged::Stale],
                RecoveryRequired,
                Some(Some(false)),
                1,
                2,
            ),
            (
                "a poll of zero sleeps a millisecond a look",
                WaitPolicy {
                    budget: Duration::from_millis(3),
                    poll: Duration::ZERO,
                },
                frozen,
                &blocked,
                &[Judged::Live],
                RepositoryBusy,
                Some(Some(true)),
                3,
                4,
            ),
            (
                "an error other than recovery_required: as it is",
                short,
                frozen,
                &|| refused(RepositoryBusy, Some(named.clone())),
                &[Judged::Live],
                RepositoryBusy,
                Some(None),
                0,
                0,
            ),
            (
                "no blocker: as it is",
                short,
                frozen,
                &|| refused(RecoveryRequired, None),
                &[Judged::Live],
                RecoveryRequired,
                None,
                0,
                0,
            ),
        ];
        for (case, policy, step, error, judgments, kind, live, slept, judged) in rows {
            let (answered, sleeps, judgments_made) = wait_out(policy, step, error, judgments);
            assert_eq!(answered.kind, kind, "{case}");
            assert_eq!(
                answered.blocker().map(|blocker| blocker.live),
                live,
                "{case}"
            );
            if let Some(blocker) = answered.blocker() {
                assert_eq!(blocker.operation_id, named.operation_id, "{case}");
                // A blocker named live carries its row as the wait read it.
                assert_eq!(blocker.seen.is_some(), blocker.live == Some(true), "{case}");
            }
            assert_eq!((sleeps, judgments_made), (slept, judged), "{case}");
        }
    }

    /// What a writer refused with a `busy` that names nothing comes to,
    /// who is at work found in turn as `finds` says (the last one again
    /// once they run out): the error it is answered with and how often it
    /// slept. `Some(true)` in `finds` is a live holder, `Some(false)` none,
    /// `None` an unreadable repository.
    fn hold_out(finds: &[Option<bool>], named: &Blocker) -> (RepositoryError, u64) {
        let data = tempfile::tempdir().unwrap();
        let time = Arc::new(TestTime {
            nanos: AtomicU64::new(0),
            step: Duration::ZERO,
            slept: AtomicU64::new(0),
        });
        let service = RepositoryService::open_at(data.path())
            .unwrap()
            .with_clock_for_testing(time.clone())
            .with_sleeper_for_testing(time.clone())
            .with_wait_policy_for_testing(WaitPolicy {
                budget: Duration::from_millis(250),
                poll: Duration::from_millis(50),
            });
        let mut wait = service.start_wait();
        for look in 0..10_000 {
            let find = || match finds[look.min(finds.len() - 1)] {
                Some(true) => LiveHolder::Live(named.clone()),
                Some(false) => LiveHolder::Nothing,
                None => LiveHolder::Unreadable,
            };
            let error = refused(RepositoryErrorKind::RepositoryBusy, None);
            match service.waited_holding(&mut wait, error, find) {
                Waited::Again => {}
                Waited::Refused(error) => return (error, time.slept.load(Ordering::SeqCst)),
            }
        }
        panic!("the wait did not end");
    }

    /// A `busy` that names nothing, a lease or the index's lock not taken
    /// in time, is waited on while an unfinished operation's holder works,
    /// and the budget then ends in `busy` naming that operation, live, with
    /// its row. With none at work it is answered as it is, at once.
    #[test]
    fn a_busy_that_names_nothing_waits_for_a_holder_at_work_and_names_it() {
        let named = Blocker {
            live: Some(true),
            seen: Some(Box::new(super::super::OperationDto {
                operation_id: Some(OperationId::new().to_string()),
                ..*seen()
            })),
            ..blocker()
        };
        let named = Blocker {
            operation_id: OperationId::parse(
                named
                    .seen
                    .as_ref()
                    .unwrap()
                    .operation_id
                    .as_deref()
                    .unwrap(),
            )
            .unwrap(),
            ..named
        };
        // (case, what each look finds, the blocker named (`None` for
        // none), sleeps)
        let rows: [(&str, &[Option<bool>], Option<&Blocker>, u64); 5] = [
            (
                "nothing at work: as it is, at once",
                &[Some(false)],
                None,
                0,
            ),
            (
                "a holder at work: the budget, then named",
                &[Some(true)],
                Some(&named),
                5,
            ),
            (
                "at work, then unreadable: named by the last read that worked",
                &[Some(true), None],
                Some(&named),
                5,
            ),
            (
                "unreadable throughout: the budget, then as it is",
                &[None],
                None,
                5,
            ),
            (
                "at work, then nothing: one sleep, then as it is",
                &[Some(true), Some(false)],
                None,
                1,
            ),
        ];
        for (case, finds, expected, sleeps) in rows {
            let (answered, slept) = hold_out(finds, &named);
            assert_eq!(answered.kind, RepositoryErrorKind::RepositoryBusy, "{case}");
            assert_eq!(answered.blocker(), expected, "{case}");
            assert_eq!(slept, sleeps, "{case}");
        }
    }

    #[test]
    fn a_writer_waits_five_seconds_looking_again_every_fifty_milliseconds() {
        assert_eq!(
            WaitPolicy::default(),
            WaitPolicy {
                budget: Duration::from_secs(5),
                poll: Duration::from_millis(50),
            }
        );
    }
}
