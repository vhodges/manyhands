//! The consent test (D6 acceptance A8, Task 13): over one simulated day of
//! the scheduler, the library is called only as the design allows.
//!
//! The day runs over a recording `Backend` (`Recorder`), which logs, in
//! order and with the thread, every method the desktop calls on it, and
//! then delegates to the polling double. The clock is injected: the day
//! passes by `advance`, and the only waiting is for the real worker thread,
//! as in every scheduler test. The day:
//!
//! 1. launch: `a` (SSH) and `c` (no remote) are listed. `a`'s poll reports
//!    four items: a closed ticket that is free, a closed ticket being edited
//!    (an unkept tab), a closed ticket with a kept draft, and a promoted
//!    document that is free. Only the two free ones are retired;
//! 2. due: time passes and `a` is polled again;
//! 3. busy: a poll answers `Busy` and restarts;
//! 4. preempt: a poll is in flight, and the person's Check now cancels it,
//!    runs, and the poll comes back;
//! 5. attention: `b` joins (a window activation reads the remotes), its poll
//!    finds a key locked, and it is not polled again while it waits;
//! 6. quit: a poll is in flight when quitting is asked, and is cancelled.
//!
//! The allowed list is written here from the design, not from what the code
//! happens to call:
//!
//! - **B4:** the automatic poll calls exactly `resolve`, `polling_status`,
//!   `poll_eligibility`, `poll_automatic` and `remote_snapshot`, in that
//!   order (a poll that stops early is a prefix of it), and `after_poll`,
//!   the test hook the double reads its constructed answer by. On the
//!   worker thread, from the start of an automatic poll to the next job,
//!   nothing else is called: not `service()`, not the key store, not the
//!   draft store, and no job, so nothing can push, checkpoint, merge, clean
//!   up or retire from a poll.
//! - **B2, Part D:** `cancel_poll`, and only for a preemption or the quit,
//!   and only for an operation the desktop started.
//! - **Scheduler reads, which contact nothing:** `polling_status` and
//!   `resolve` again (C1, A4), `publication_remotes` (B6, U-2),
//!   `pending_operations` (Amendment 12, R-31), `selected_key` (E3) and the
//!   draft store's listing (E1, RT-4, RT-5). Off the worker thread nothing
//!   else is called, and `service()` and the key store only as often as
//!   start-up did before any poll existed.
//!
//! What is enforced, and what is not: every `Backend` method is forwarded
//! and logged, so a call through the trait is seen. A call the desktop makes
//! on `backend.service()` directly is seen only as a `service()` access, and
//! so is excluded off the worker thread and inside a poll window; on the
//! worker thread, between jobs, it is not checked (a job's own library call
//! is a `service()` access there by design).
//! - **Jobs:** an automatic poll (`Poll`), the person's own intents (here
//!   one Check now, `PollOnce`), and `ContextRetire` where E1 allows: one
//!   per reported `Closed` or `Promoted` item that is not being edited and
//!   has no draft, after the poll that reported it, in the order reported,
//!   never inside the automatic poll, and never `PollOnceRetiring` (which no
//!   intent builds). `ResumePolling` only for the person's Resume checking,
//!   which the day does not press.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread::{self, ThreadId},
    time::Duration,
};

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    drafts::{DraftFields, DraftTarget, ItemKind},
    repository::{
        AuthoringKind, AutomaticPollEligibility, ConflictDto, ContextRetirementState as State,
        Mutation, MutationDataDto, OperationId, PollCompletion, PollDisposition,
        PollPublicationRemoteRequest, PollingStatusDto, PrepareDataDto, ProgressStage, ReadError,
        RemoteListDto, RemotePollError, RemotePollResult, RemoteSnapshot, RepositoryError,
        RepositoryService, ResolvedRepository, RetirementRemoval as Removal, SharedKeyId,
        keys::{KeyStore, SessionCredentials},
        transport::SshTransportErrorKind,
    },
    results::{Envelope, Outcome, ResultCode, Scope},
};

use super::support::World;
use crate::desktop::{
    model::{
        polling::{Clock as _, Phase, Reason},
        tabs::TabKey,
    },
    services::{
        Backend, DraftStoreUnavailable, JobKind, KeyStoreUnavailable, ReadGate, ReadKind, Services,
        edit_tabs::TabEditing,
        layout::{LayoutStore, LayoutStoreError},
        poll_convert::{Library, ReportParts, TargetParts},
        prompts::DesktopProvider,
        requests::{Intent, Lane, RequestState},
        resolutions::{ResolutionStore, ResolutionStoreError},
    },
    test_support::{
        fixtures::Fixture,
        polling::{PollingBackend, transport_error},
        retire::answer,
    },
};

/// One call the desktop made on the backend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Call {
    Service,
    KeyStore,
    Drafts,
    Resolve,
    PollingStatus,
    PublicationRemotes,
    PollEligibility,
    PollAutomatic,
    AfterPoll,
    RemoteSnapshot,
    CancelPoll,
    ResumePolling,
    SelectedKey,
    PendingOperations,
    Job(JobKind),
}

/// A backend that logs every call in order, with the thread that made it,
/// and delegates every method to the polling double (which wraps the real
/// service). Every method of the trait is forwarded: a default body would
/// reach the library without the double and without a log.
struct Recorder {
    inner: Arc<PollingBackend>,
    log: Mutex<Vec<(ThreadId, Call)>>,
}

impl Recorder {
    fn heard(&self, call: Call) {
        self.log
            .lock()
            .unwrap()
            .push((thread::current().id(), call));
    }

    fn calls(&self) -> Vec<(ThreadId, Call)> {
        self.log.lock().unwrap().clone()
    }

    fn count(&self, call: Call) -> usize {
        self.calls()
            .iter()
            .filter(|(_, heard)| *heard == call)
            .count()
    }
}

impl Backend for Recorder {
    fn service(&self) -> &RepositoryService {
        self.heard(Call::Service);
        self.inner.service()
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        self.heard(Call::KeyStore);
        self.inner.key_store()
    }

    fn drafts(&self) -> Result<&manyhands::drafts::DraftStore, DraftStoreUnavailable> {
        self.heard(Call::Drafts);
        self.inner.drafts()
    }

    fn resolutions(&self) -> Result<&ResolutionStore, ResolutionStoreError> {
        self.inner.resolutions()
    }

    fn layout(&self) -> Result<&LayoutStore, LayoutStoreError> {
        self.inner.layout()
    }

    fn read_gate(&self, kind: ReadKind) -> Option<ReadGate> {
        self.inner.read_gate(kind)
    }

    fn before_job(&self, kind: JobKind) {
        self.heard(Call::Job(kind));
        self.inner.before_job(kind);
    }

    fn instead_of_job(&self, kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        self.inner.instead_of_job(kind)
    }

    fn pending_operations(&self, root: &Path) -> Option<manyhands::repository::OperationListDto> {
        self.heard(Call::PendingOperations);
        self.inner.pending_operations(root)
    }

    fn after_job(&self, kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        self.inner.after_job(kind)
    }

    fn on_progress(&self, kind: JobKind, stage: ProgressStage) {
        self.inner.on_progress(kind, stage);
    }

    fn after_prepare(&self, mutation: &Mutation) -> Option<Envelope<PrepareDataDto>> {
        self.inner.after_prepare(mutation)
    }

    fn after_conflict_read(
        &self,
        kind: ReadKind,
        operation_id: &OperationId,
    ) -> Option<ConflictDto> {
        self.inner.after_conflict_read(kind, operation_id)
    }

    fn resolve(&self, root: &Path) -> Result<ResolvedRepository, ReadError> {
        self.heard(Call::Resolve);
        self.inner.resolve(root)
    }

    fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError> {
        self.heard(Call::PollingStatus);
        self.inner.polling_status(repo)
    }

    fn publication_remotes(&self, repo: &ResolvedRepository) -> Result<RemoteListDto, ReadError> {
        self.heard(Call::PublicationRemotes);
        self.inner.publication_remotes(repo)
    }

    fn poll_eligibility(&self, root: &Path) -> Result<AutomaticPollEligibility, RepositoryError> {
        self.heard(Call::PollEligibility);
        self.inner.poll_eligibility(root)
    }

    fn poll_automatic(
        &self,
        request: PollPublicationRemoteRequest,
        session: &mut SessionCredentials<DesktopProvider>,
    ) -> Result<RemotePollResult, RemotePollError> {
        self.heard(Call::PollAutomatic);
        self.inner.poll_automatic(request, session)
    }

    fn remote_snapshot(&self, root: &Path) -> Result<RemoteSnapshot, RepositoryError> {
        self.heard(Call::RemoteSnapshot);
        self.inner.remote_snapshot(root)
    }

    fn after_poll(&self, operation: OperationId) -> Option<Library> {
        self.heard(Call::AfterPoll);
        self.inner.after_poll(operation)
    }

    fn resume_polling(&self, root: &Path) -> Result<(), RepositoryError> {
        self.heard(Call::ResumePolling);
        self.inner.resume_polling(root)
    }

    fn selected_key(&self) -> Option<SharedKeyId> {
        self.heard(Call::SelectedKey);
        self.inner.selected_key()
    }

    fn cancel_poll(&self, root: &Path, op: OperationId) -> Result<(), RepositoryError> {
        self.heard(Call::CancelPoll);
        self.inner.cancel_poll(root, op)
    }
}

/// How long one wait of the day may take. The whole day takes about 26 s, so
/// a wait that has not come true in this long is stuck, and says so then
/// rather than after the harness's 180 s.
const BOUND: Duration = Duration::from_secs(30);

/// B4's five calls in the order an automatic poll makes them, and `after_poll`,
/// the double's hook between the last two.
const POLL: [Call; 6] = [
    Call::Resolve,
    Call::PollingStatus,
    Call::PollEligibility,
    Call::PollAutomatic,
    Call::AfterPoll,
    Call::RemoteSnapshot,
];

/// What may be called from any thread but the worker's: the scheduler's
/// reads (`resolve` and `polling_status` again, `publication_remotes`,
/// `pending_operations`, `selected_key`, the draft listing) and the cancel.
const OFF_THE_WORKER: [Call; 7] = [
    Call::Resolve,
    Call::PollingStatus,
    Call::PublicationRemotes,
    Call::PendingOperations,
    Call::SelectedKey,
    Call::Drafts,
    Call::CancelPoll,
];

/// The jobs the day may run: an automatic poll, the person's Check now, and
/// E1's retirements. Anything else (a push, a checkpoint, a merge, a cleanup,
/// an index refresh nobody asked for) is a call the person did not agree to.
const JOBS: [JobKind; 3] = [JobKind::Poll, JobKind::PollOnce, JobKind::ContextRetire];

const IDS: [&str; 4] = [
    "01ARZ3NDEKTSV4RRFFQ69G5FA1",
    "01ARZ3NDEKTSV4RRFFQ69G5FA2",
    "01ARZ3NDEKTSV4RRFFQ69G5FA3",
    "01ARZ3NDEKTSV4RRFFQ69G5FA4",
];
/// A free closed ticket, a closed ticket being edited, a closed ticket with a
/// kept draft, and a free promoted document.
const FREE: usize = 0;
const EDITED: usize = 1;
const KEPT: usize = 2;
const DOCUMENT: usize = 3;

fn item(n: usize) -> ItemId {
    IDS[n].parse().unwrap()
}

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn completed(world: &Day, targets: Vec<TargetParts>) -> Library {
    let now = world.clock.utc();
    Library::Polled {
        parts: ReportParts {
            completion: PollCompletion::Completed,
            completed_at: now,
            next_eligible_at: Some(now + time::Duration::seconds(300)),
            targets,
            outcome: None,
        },
        index_pending: false,
    }
}

fn target(n: usize, kind: AuthoringKind, disposition: PollDisposition) -> TargetParts {
    TargetParts {
        item_id: Some(item(n)),
        kind: Some(kind),
        disposition,
        main: false,
    }
}

/// A poll the library ends `Cancelled`, as it does when asked to stop.
fn cancelled(day: &Day) -> Library {
    Library::Polled {
        parts: ReportParts {
            completion: PollCompletion::Cancelled,
            completed_at: day.clock.utc(),
            next_eligible_at: None,
            targets: Vec::new(),
            outcome: None,
        },
        index_pending: false,
    }
}

/// A retirement the library answers `retired`.
fn retired() -> Envelope<MutationDataDto> {
    answer(State::Retired, ResultCode::Ok, Removal::Observed)
}

/// The services over the recorder over the polling double, polling on, the
/// clock in the test's hand.
struct Day {
    fixture: Fixture,
    backend: Arc<PollingBackend>,
    recorder: Arc<Recorder>,
    services: Services,
    clock: Arc<crate::desktop::model::polling::TestClock>,
    /// How many `service()` and `key_store()` calls the recorder had heard
    /// when `Services::start` returned: what start-up reads off the worker
    /// thread (I-1).
    started: (usize, usize),
}

impl Day {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let recorder = Arc::new(Recorder {
            inner: backend.clone(),
            log: Mutex::default(),
        });
        let clock = Arc::new(crate::desktop::model::polling::TestClock::at(
            time::OffsetDateTime::now_utc(),
        ));
        let services = cx.update(|cx| {
            Services::start(recorder.clone(), cx)
                .without_launch_refresh()
                .with_clock(clock.clone(), cx)
                .with_polling()
        });
        let started = (
            recorder.count(Call::Service),
            recorder.count(Call::KeyStore),
        );
        Self {
            fixture,
            backend,
            recorder,
            services,
            clock,
            started,
        }
    }

    fn polls_of(&self, root: &str) -> usize {
        self.backend
            .polled_roots()
            .iter()
            .filter(|polled| *polled == root)
            .count()
    }

    fn phase(&self, root: &str, cx: &mut TestAppContext) -> Option<Phase> {
        self.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(&root.to_owned())
                .map(|repo| repo.phase.clone())
        })
    }

    fn tick(&self, cx: &mut TestAppContext) {
        self.services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.tick(cx));
    }

    /// Time passes, and the scheduler's timer fires.
    fn pass(&self, by: Duration, cx: &mut TestAppContext) {
        self.clock.advance(by);
        self.tick(cx);
    }

    /// The retirements sent so far, as the items they name, oldest first,
    /// with their lane.
    fn retirements(&self, cx: &mut TestAppContext) -> Vec<(ItemId, Lane)> {
        self.services.operations().read_with(cx, |operations, _| {
            let mut sent: Vec<_> = operations
                .requests()
                .iter()
                .filter_map(|request| match &request.intent {
                    Intent::ContextRetire { id, .. } => Some((id.clone(), request.lane)),
                    _ => None,
                })
                .collect();
            sent.reverse();
            sent
        })
    }

    fn retirements_ended(&self, cx: &mut TestAppContext) -> bool {
        self.services.operations().read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter(|request| matches!(request.intent, Intent::ContextRetire { .. }))
                .all(|request| {
                    !matches!(
                        request.state,
                        RequestState::Queued | RequestState::Running { .. }
                    )
                })
        })
    }

    /// What the day looks like, for a wait that timed out: the polls so far
    /// (by repository name), the phases, the cancels, the jobs, and the last
    /// calls the recorder heard (the log's noisy accessors left out).
    fn dump(&self, cx: &mut TestAppContext) -> String {
        let polled = self.backend.polled_roots();
        let mut roots: Vec<&String> = polled.iter().collect();
        roots.sort();
        roots.dedup();
        let phases: Vec<_> = roots.iter().map(|root| self.phase(root, cx)).collect();
        let calls: Vec<Call> = self
            .recorder
            .calls()
            .iter()
            .map(|(_, call)| *call)
            .filter(|call| !matches!(call, Call::Service | Call::Drafts | Call::KeyStore))
            .collect();
        format!(
            "polls {}, phases {phases:?}, cancels {}, jobs {:?}, retirements {}, last calls {:?}",
            polled.len(),
            self.backend.cancels().len(),
            self.backend.inner().jobs_seen(),
            self.retirements(cx).len(),
            &calls[calls.len().saturating_sub(30)..],
        )
    }

    /// Waits, letting the executor run, until `done` holds. The scheduler
    /// and the worker are real threads and tasks; nothing here sleeps for
    /// the schedule, which is the injected clock's.
    async fn until(
        &self,
        cx: &mut TestAppContext,
        what: &str,
        done: impl Fn(&Self, &mut TestAppContext) -> bool,
    ) {
        let deadline = std::time::Instant::now() + BOUND;
        loop {
            cx.run_until_parked();
            if done(self, cx) {
                return;
            }
            if std::time::Instant::now() > deadline {
                panic!("timed out waiting for {what}: {}", self.dump(cx));
            }
            cx.executor().timer(Duration::from_millis(10)).await;
        }
    }

    /// The scheduler is waiting for nothing: idle and the worker has nothing.
    async fn idle(&self, root: &str, cx: &mut TestAppContext) {
        self.until(cx, "the repository to be idle", |day, cx| {
            day.phase(root, cx) == Some(Phase::Idle) && !day.services.worker().in_flight(|_| true)
        })
        .await;
        World::quiet(cx).await;
    }
}

/// A kept draft of the ticket `n` of `root`.
fn keep_draft(day: &Day, root: &str, n: usize) {
    let target = DraftTarget {
        repository: root.to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: item(n),
        comment: None,
    };
    let fields = DraftFields::Ticket {
        title: "T".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: "Words".to_owned(),
    };
    day.fixture
        .draft_store()
        .create(target, None, fields)
        .unwrap_or_else(|error| panic!("the draft must be stored: {error}"));
}

/// An open tab of the ticket `n` of `root` that says it is being edited.
fn edit_tab(day: &Day, root: &str, n: usize, cx: &mut TestAppContext) {
    let tab = TabKey::Item {
        repo: root.to_owned(),
        kind: manyhands::repository::ItemDtoKind::Ticket,
        id: IDS[n].to_owned(),
    };
    let editing = TabEditing {
        editing: true,
        unkept: true,
        ..TabEditing::default()
    };
    day.services.edit_tabs().update(cx, |tabs, _| {
        tabs.watch(tab, Box::new(move |_| Some(editing)))
    });
}

/// A Check now answered `ok`.
fn checked() -> Envelope<MutationDataDto> {
    Envelope::mutation(
        "poll once",
        Scope::default(),
        Outcome::Success,
        ResultCode::Ok,
    )
}

/// A8: one day of the scheduler, over a recording backend, calls only what
/// the design allows (the module's list), and the cancels, the retirements
/// and the polls of each phase are the ones its cause allows.
#[gpui_kit::test]
async fn polling_consent(cx: &mut TestAppContext) {
    let day = Day::new(cx);
    let a = day.fixture.create_repository("a");
    day.fixture.publish_over_ssh(&a);
    let a = a.to_str().unwrap().to_owned();
    let plain = day
        .fixture
        .create_repository("c")
        .to_str()
        .unwrap()
        .to_owned();
    // `b` has no remote yet: it joins the day when one is added.
    let b = day.fixture.create_repository("b");
    let b = b.to_str().unwrap().to_owned();
    keep_draft(&day, &a, KEPT);
    // The draft listing lands before the first poll completes (RT-5).
    World::quiet(cx).await;
    edit_tab(&day, &a, EDITED, cx);

    // 1. Launch. The poll reports the four items; the two free ones are
    // retired, in the order reported, and the retirement's `Changed` makes
    // the repository due again at once (the library stores no deadline in
    // a test), so one more poll answers.
    let ticket = AuthoringKind::Ticket;
    day.backend.answer_poll(completed(
        &day,
        vec![
            target(FREE, ticket, PollDisposition::Closed),
            target(EDITED, ticket, PollDisposition::Closed),
            target(KEPT, ticket, PollDisposition::Closed),
            target(DOCUMENT, AuthoringKind::Document, PollDisposition::Promoted),
        ],
    ));
    day.backend.answer_poll(completed(&day, Vec::new()));
    for _ in 0..2 {
        day.backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }
    let services = day.services.clone();
    let roots = [a.clone(), plain.clone(), b.clone()];
    cx.update(|cx| services.repositories_listed(&roots, cx));
    day.until(
        cx,
        "the launch poll, its retirements and the next poll",
        |day, cx| {
            day.polls_of(&a) >= 2 && day.retirements(cx).len() >= 2 && day.retirements_ended(cx)
        },
    )
    .await;
    day.idle(&a, cx).await;
    assert_eq!(day.polls_of(&a), 2, "the launch poll and the one after it");
    assert_eq!(day.polls_of(&plain), 0, "no remote, no poll (B6)");
    assert_eq!(day.polls_of(&b), 0, "no remote yet, no poll (B6)");
    assert_eq!(
        day.retirements(cx),
        [
            (item(FREE), Lane::Background),
            (item(DOCUMENT), Lane::Background)
        ],
        "E1: the free ones, in the order reported; not the edited, not the kept"
    );
    assert_eq!(day.backend.cancels().len(), 0, "launch cancels nothing");
    let polls = day.polls_of(&a);

    // 2. Due: the poll the launch set up comes round, and no sooner. (An
    // answer carries its own completion time, so it is made after the clock
    // moves.)
    day.pass(secs(100), cx);
    World::quiet(cx).await;
    assert_eq!(day.polls_of(&a), polls, "not due yet");
    day.clock.advance(secs(201));
    day.backend.answer_poll(completed(&day, Vec::new()));
    day.tick(cx);
    day.until(cx, "the due poll", |day, _| day.polls_of(&a) > polls)
        .await;
    day.idle(&a, cx).await;
    let polls = day.polls_of(&a);
    assert_eq!(polls, 3, "one poll for one due time");

    // 3. Busy: the answer restarts the same operation after the schedule's
    // delay (5 s), and the restart completes. (A queued constructed answer
    // would stand in for the busy poll's own, so it is queued afterwards.)
    day.clock.advance(secs(301));
    day.backend.fail_poll(RemotePollError::Busy);
    day.tick(cx);
    day.until(cx, "the busy poll", |day, _| day.polls_of(&a) > polls)
        .await;
    World::quiet(cx).await;
    day.clock.advance(secs(6));
    day.backend.answer_poll(completed(&day, Vec::new()));
    day.tick(cx);
    day.until(cx, "the restart", |day, _| day.polls_of(&a) > polls + 1)
        .await;
    day.idle(&a, cx).await;
    assert_eq!(day.backend.cancels().len(), 0, "busy cancels nothing");
    let polls = day.polls_of(&a);

    // 4. Preempt: a poll in flight, held; the person's Check now cancels it
    // (B2) and runs first; the poll returns once the user lane is empty.
    day.clock.advance(secs(301));
    let gate = day.backend.hold_poll();
    // The held poll ends `Cancelled`, as the library ends one it is asked to
    // stop; the poll that comes back completes.
    day.backend.answer_poll(cancelled(&day));
    day.backend.answer_poll(completed(&day, Vec::new()));
    day.backend.inner().answer_job(JobKind::PollOnce, checked());
    day.tick(cx);
    day.until(cx, "the poll to be in flight", |day, _| {
        day.polls_of(&a) > polls
    })
    .await;
    day.services.worker().submit(
        Intent::PollOnce {
            root: a.clone().into(),
        },
        Lane::User,
        None,
        None,
    );
    day.until(cx, "the cancel", |day, _| !day.backend.cancels().is_empty())
        .await;
    drop(gate);
    day.until(cx, "the Check now", |day, _| {
        day.backend.inner().jobs_seen().contains(&JobKind::PollOnce)
    })
    .await;
    day.until(cx, "the poll to come back", |day, cx| {
        day.tick(cx);
        day.polls_of(&a) > polls + 1
    })
    .await;
    day.idle(&a, cx).await;
    let cancelled_by_preemption = day.backend.cancels().len();
    assert!(cancelled_by_preemption > 0, "the preemption cancelled");
    // 5. Attention: `b` gets its remote and the window is activated, which
    // reads every policy and remote again, so both repositories are due.
    // The first poll finds a key locked: attention and no prompt. The second
    // completes. (The polls are held so that each is given its answer: an
    // answer made ready for one poll would be taken by the other.) The
    // repository in attention is not polled again, and the other is.
    let before = day.backend.polled_roots().len();
    let first_gate = day.backend.hold_poll();
    let second_gate = day.backend.hold_poll();
    day.backend.fail_poll(transport_error(
        &b,
        SshTransportErrorKind::UnlockCancelled,
        Some(SharedKeyId::new()),
        None,
    ));
    day.fixture.publish_over_ssh(Path::new(&b));
    let services = day.services.clone();
    cx.update(|cx| services.window_activated(cx));
    day.until(cx, "the first poll", |day, _| {
        day.backend.polled_roots().len() > before
    })
    .await;
    let held = day.backend.polled_roots()[before].clone();
    drop(first_gate);
    day.until(cx, "the attention", |day, cx| {
        matches!(
            day.phase(&held, cx),
            Some(Phase::Attention(Reason::KeyLocked(_)))
        )
    })
    .await;
    day.until(cx, "the second poll", |day, _| {
        day.backend.polled_roots().len() > before + 1
    })
    .await;
    let free = day.backend.polled_roots()[before + 1].clone();
    assert_ne!(free, held, "the other repository");
    day.backend.answer_poll(completed(&day, Vec::new()));
    drop(second_gate);
    day.idle(&free, cx).await;
    assert_eq!(
        day.backend.cancels().len(),
        cancelled_by_preemption,
        "attention cancels nothing"
    );
    let (held_polls, free_polls) = (day.polls_of(&held), day.polls_of(&free));
    day.clock.advance(secs(4000));
    day.backend.answer_poll(completed(&day, Vec::new()));
    day.tick(cx);
    day.until(cx, "the free repository to be polled", |day, _| {
        day.polls_of(&free) > free_polls
    })
    .await;
    day.idle(&free, cx).await;
    World::quiet(cx).await;
    assert_eq!(
        day.polls_of(&held),
        held_polls,
        "a repository in attention is not polled"
    );

    // 6. Quit: a poll in flight is cancelled, once quitting is asked.
    let polls = day.polls_of(&free);
    day.clock.advance(secs(4000));
    let gate = day.backend.hold_poll();
    day.backend.answer_poll(cancelled(&day));
    day.tick(cx);
    day.until(cx, "a poll in flight", |day, _| day.polls_of(&free) > polls)
        .await;
    day.services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.set_quitting(true, cx));
    day.services.worker().quit();
    day.until(cx, "the quit's cancel", |day, _| {
        day.backend.cancels().len() > cancelled_by_preemption
    })
    .await;
    drop(gate);
    World::quiet(cx).await;
    let polls = day.polls_of(&free);
    day.clock.advance(secs(4000));
    day.tick(cx);
    World::quiet(cx).await;
    assert_eq!(day.polls_of(&free), polls, "nothing starts after quit");

    verify(&day, &a, cx);
}

/// What the recording backend heard over the day, against the list.
fn verify(day: &Day, a: &str, cx: &mut TestAppContext) {
    let log = day.recorder.calls();

    // Every job is on the list, and the worker's jobs are the day's.
    let jobs: Vec<JobKind> = log
        .iter()
        .filter_map(|(_, call)| match call {
            Call::Job(kind) => Some(*kind),
            _ => None,
        })
        .collect();
    assert!(
        jobs.iter().all(|kind| JOBS.contains(kind)),
        "a job the person did not agree to: {jobs:?}"
    );
    let count = |kind: JobKind| jobs.iter().filter(|seen| **seen == kind).count();
    assert_eq!(count(JobKind::PollOnce), 1, "the person's own Check now");
    assert_eq!(count(JobKind::ContextRetire), 2, "E1: the two free items");
    // B4: from the start of an automatic poll to the next job, the worker
    // thread calls B4's methods in order, and nothing else.
    let mut windows = 0;
    for (at, (thread, call)) in log.iter().enumerate() {
        if *call != Call::Job(JobKind::Poll) {
            continue;
        }
        windows += 1;
        let made: Vec<Call> = log[at + 1..]
            .iter()
            .filter(|(other, _)| other == thread)
            .map(|(_, call)| *call)
            .take_while(|call| !matches!(call, Call::Job(_)))
            .collect();
        assert!(
            POLL.starts_with(&made),
            "an automatic poll called {made:?}, which is not a prefix of B4's {POLL:?}"
        );
    }
    // Every poll the double was asked for ran in a window of its own, and
    // each stage of B4 ran once per poll, no more (a call from anywhere
    // else would add to the counts).
    assert_eq!(windows, count(JobKind::Poll));
    assert_eq!(windows, day.backend.polled_roots().len());
    for stage in [
        Call::PollAutomatic,
        Call::PollEligibility,
        Call::RemoteSnapshot,
    ] {
        assert_eq!(
            day.recorder.count(stage),
            windows,
            "{stage:?} once per poll"
        );
    }

    // Outside the worker thread nothing but the scheduler's reads and the
    // cancel is called (I-1). `service()` and the key store are the
    // library's whole surface, so none may be reached off the worker beyond
    // what start-up did before any poll or job existed.
    let worker = log
        .iter()
        .find(|(_, call)| *call == Call::Job(JobKind::Poll))
        .map(|(thread, _)| *thread)
        .expect("a poll ran");
    let (mut services, mut key_stores) = (0, 0);
    for (thread, call) in log.iter().filter(|(thread, _)| *thread != worker) {
        match call {
            Call::Service => services += 1,
            Call::KeyStore => key_stores += 1,
            other => assert!(
                OFF_THE_WORKER.contains(other),
                "{other:?} was called off the worker thread {thread:?}: not on the list"
            ),
        }
    }
    assert!(
        services <= day.started.0 && key_stores <= day.started.1,
        "service() was reached {services} times and the key store {key_stores} times off the \
         worker; start-up made {:?}",
        day.started
    );

    // A cancel is for an operation the desktop started (a poll that reached
    // `poll_automatic`), never for another's row (M-1).
    let started: Vec<OperationId> = day.backend.requests().iter().map(|(op, _)| *op).collect();
    for cancel in day.backend.cancels() {
        assert!(started.contains(&cancel.op), "a cancel for another's row");
    }

    // ResumePolling is the person's Resume checking, which was not pressed.
    assert_eq!(day.recorder.count(Call::ResumePolling), 0);

    // E1: each retirement is a background request for an item a completed
    // attempt reported, after that attempt's poll, and every one is built
    // as `ContextRetire`, never as `PollOnceRetiring`.
    let first_poll = log
        .iter()
        .position(|(_, call)| *call == Call::AfterPoll)
        .expect("a poll");
    let first_retire = log
        .iter()
        .position(|(_, call)| *call == Call::Job(JobKind::ContextRetire))
        .expect("a retirement");
    assert!(first_retire > first_poll, "after the poll that reported it");
    let requests = day.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .map(|request| (request.intent.clone(), request.lane))
            .collect::<Vec<_>>()
    });
    for (intent, lane) in &requests {
        let mutation = intent
            .build(None, || Err(KeyStoreUnavailable))
            .unwrap_or_else(|_| panic!("the intent builds"));
        assert!(
            !matches!(mutation, Mutation::PollOnceRetiring(_)),
            "no intent builds PollOnceRetiring"
        );
        match intent {
            Intent::PollOnce { root } => {
                assert_eq!(*lane, Lane::User);
                assert_eq!(root, Path::new(a));
                assert!(matches!(mutation, Mutation::PollOnce(_)));
            }
            Intent::ContextRetire { root, kind, .. } => {
                assert_eq!(*lane, Lane::Background);
                assert_eq!(root, Path::new(a));
                assert!(matches!(mutation, Mutation::ContextRetire(_)));
                assert!(matches!(
                    kind,
                    AuthoringKind::Ticket | AuthoringKind::Document
                ));
            }
            _ => panic!("a request nobody asked for"),
        }
    }
    assert_eq!(requests.len(), 3, "the Check now and two retirements");
}
