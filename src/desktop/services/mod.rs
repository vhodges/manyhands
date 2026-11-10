//! The service layer views read and act through, and the `Backend` seam
//! beneath it.
//!
//! Production uses `LibraryBackend`, whose hooks do nothing. Tests use
//! `test_support::delaying::DelayingBackend`, which only delays: it never
//! fails or fabricates a call. Failures in tests come from the library's
//! own failure points and hooks.

pub(crate) mod conflicts;
pub(crate) mod created_contexts;
pub(crate) mod drafts;
pub(crate) mod edit_tabs;
pub(crate) mod images;
pub(crate) mod inflight;
pub(crate) mod layout;
pub(crate) mod links;
pub(crate) mod pending_conflicts;
pub(crate) mod poll;
pub(crate) mod poll_convert;
pub(crate) mod preempt;
pub(crate) mod prompts;
mod reads;
pub(crate) mod release;
pub(crate) mod requests;
pub(crate) mod resends;
pub(crate) mod resolutions;
pub(crate) mod scheduler;
pub(crate) mod submissions;
pub(crate) mod worker;

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use futures::StreamExt;
use gpui_kit::{App, AppContext, Entity};
use manyhands::{
    drafts::{DraftError, DraftStore},
    repository::{
        AutomaticPollEligibility, ConflictDto, ContextPrepareIntent, Mutation, MutationDataDto,
        OperationId, PollPublicationRemoteRequest, PollingStatusDto, PrepareDataDto, ProgressStage,
        ReadError, RemoteListDto, RemotePollError, RemotePollResult, RemoteSnapshot,
        RepositoryError, RepositoryService, ResolvedRepository, SharedKeyId,
        keys::{KeyStore, SessionCredentials},
    },
    results::{Envelope, Outcome, PublicationEffect, RecoveryActionKind, ResultCode},
};

use crate::desktop::model::{
    lifecycle_lock::{self as model_lock, LockKey, LockState},
    list_state::ListState,
    operations::Operations,
    polling as model_polling,
};
use created_contexts::{ContextKey, CreatedContexts};
use drafts::DraftRegistry;
use edit_tabs::EditTabs;
use layout::{LayoutStore, LayoutStoreError};
use poll::PollInbox;
use prompts::DesktopProvider;
use release::Releases;
use resolutions::{ResolutionStore, ResolutionStoreError};
use scheduler::Scheduler;
use submissions::Submissions;
use worker::{WorkerEvent, WorkerHandle};

pub(crate) use drafts::DraftStoreUnavailable;

pub(crate) use reads::{ReadFailure, ReadResult, Reader};

/// The waits before each retry of a call answered `busy`: a read, a
/// prepare, or a worker job of either lane (rulings R6, R-T7a, R-B3).
pub(crate) const BUSY_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
];

/// A read's delay: awaited inside the read's future, before the blocking
/// library call. It must never block: under `#[gpui_kit::test]` the
/// background executor runs on the test thread.
pub(crate) type ReadGate = futures::channel::oneshot::Receiver<()>;

/// What a read asks for, so the seam can hold one kind and not another.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ReadKind {
    Repositories,
    Resolve,
    Documents,
    Folders,
    Tickets,
    Item,
    Comments,
    Dependencies,
    Children,
    FindSlug,
    Operations,
    /// `inspect_repository`, of a folder the Repositories page checks.
    Inspect,
    Remotes,
    Identity,
    Keys,
    Hosts,
    Prepare,
    /// The draft store's listing and claims.
    Drafts,
    /// A draft's revision written to the store.
    DraftWrite,
    /// The registry's re-read of an item after a save, apart from `Item`,
    /// which the repository's tabs reload under at the same time.
    DraftItem,
    /// The lifecycle dialog's reads of the titles of the tickets a closure
    /// made ready (D5 A3), apart from `Item`, which the item's tabs reload
    /// under as the run ends.
    MadeReady,
    /// `observe_path`: the path a clicked link names, and any other path
    /// the desktop asks the library about.
    ObservePath,
    /// `new_id`, for a new item's ID.
    NewId,
    /// The check of a relationship a draft sets, before it is saved.
    RelationshipCheck,
    /// A local image an editor shows, read from the tab's trees and
    /// decoded (`images`): no library call, but held like one.
    Image,
    /// A conflict's reads (D5 B1): the remote `show_conflict`, or the
    /// local inspection and its content reads.
    Conflict,
    /// The resolve sheet's read of its item's work folder (D5 B4), apart
    /// from `Conflict` and from `Item`, which the tabs read under.
    ConflictFolder,
    /// Which pending rows hold a conflict to review (D5 B5), read after
    /// the rows, apart from `Operations`.
    PendingConflicts,
}

/// What a worker job does, so the seam can hold one kind and not another.
/// One per desktop intent, and the host observation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum JobKind {
    RepoCreate,
    RepoEnable,
    RepoRemove,
    RepoIdentitySet,
    RemoteAdd,
    RemoteRemove,
    RemoteSelect,
    IndexRefresh,
    IndexRebuild,
    /// Check now (D6): the person's own `poll once`, a mutation.
    PollOnce,
    /// The policy verbs (D6): single writes that never report `accepted`.
    PollConfigure,
    PollPause,
    PollResume,
    KeyGenerate,
    KeyImport,
    KeySelect,
    KeyClear,
    KeyRemove,
    KeyDelete,
    HostApprove,
    HostReplace,
    OperationResume,
    OperationAbandon,
    DocumentSave,
    TicketSave,
    ContextPrepare,
    ContextRelease,
    /// Retires a completed item's local copy (D6 Part E).
    ContextRetire,
    DocumentCreate,
    TicketCreate,
    DocumentMove,
    FolderCreate,
    DocumentRepair,
    TicketRepair,
    Comment,
    SynchronizeItem,
    SynchronizeRepo,
    ItemRepublish,
    DocumentPromote,
    TicketClose,
    ConflictResolve,
    LocalContextSync,
    LocalContextResolve,
    ObserveHost,
    /// An automatic poll (D6): no intent, so no request.
    Poll,
}

/// The key store could not be found. The Keys page shows
/// `KEY_STORE_UNAVAILABLE`; the error itself is never shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct KeyStoreUnavailable;

pub(crate) const KEY_STORE_UNAVAILABLE: &str = "Manyhands could not find your SSH key store.";

/// What the desktop calls the library through.
pub(crate) trait Backend: Send + Sync + 'static {
    fn service(&self) -> &RepositoryService;

    /// The current user's key store in production; the fixture home's in
    /// tests.
    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable>;

    /// The draft store, opened in the application data directory in
    /// production and in the fixture's in tests; otherwise why it did not
    /// open.
    fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable>;

    /// The kept resolution sets (D5 B3), in `resolutions` beside the
    /// draft store's folder: the application data directory's in
    /// production, the fixture's in tests; otherwise why it did not open.
    fn resolutions(&self) -> Result<&ResolutionStore, ResolutionStoreError>;

    /// The saved panel layout (Wave 04 Cycle 01, Part G), in `layout`
    /// beside the draft store's folder: the application data directory's
    /// in production, the fixture's in tests; otherwise why it did not
    /// open.
    fn layout(&self) -> Result<&LayoutStore, LayoutStoreError>;

    /// Awaited inside the read's future, before the blocking library call.
    fn read_gate(&self, _kind: ReadKind) -> Option<ReadGate> {
        None
    }

    /// Called on the worker thread before a job's library call.
    fn before_job(&self, _kind: JobKind) {}

    /// Called on the worker thread after `before_job`. An answer it gives
    /// stands in place of the library's, and the library call is not made: a
    /// test's canned answer for a job whose call would reach a remote no
    /// desktop test has (D6 decision 9), or take seconds to fail.
    fn instead_of_job(&self, _kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        None
    }

    /// Called on the worker thread after a job's library call returns,
    /// before the worker decides what follows it. An answer it gives
    /// stands in place of the library's: a test's constructed answer.
    fn after_job(&self, _kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        None
    }

    /// Called on the worker thread with each progress stage a job of
    /// `kind` reports, once the stage has been forwarded to the UI and
    /// before the library goes on. A test may hold the job here; the
    /// library may hold the repository lease meanwhile (L3 reports
    /// `integrating` under it).
    fn on_progress(&self, _kind: JobKind, _stage: ProgressStage) {}

    /// Called after a prepare of `mutation` returns. An answer it gives
    /// stands in place of the library's: a test's canned prepare answer
    /// (D5 decision 9).
    fn after_prepare(&self, _mutation: &Mutation) -> Option<Envelope<PrepareDataDto>> {
        None
    }

    /// Called after a conflict read of `operation_id`, of the read kind
    /// `kind`, returns. A conflict it gives stands in place of the
    /// library's: a test's canned `ConflictDto` (D5 decision 9).
    fn after_conflict_read(
        &self,
        _kind: ReadKind,
        _operation_id: &OperationId,
    ) -> Option<ConflictDto> {
        None
    }

    // The automatic poll's calls (D6 B1, B4), one per library call so that a
    // recording double can list them. The defaults are the library's own;
    // a double records, then delegates.

    /// The first call of an automatic poll: the repository at `root`, as
    /// registered.
    #[allow(clippy::result_large_err)] // The library's `ReadError`.
    fn resolve(&self, root: &Path) -> Result<ResolvedRepository, ReadError> {
        self.service().resolve_repository(root)
    }

    /// The repository's polling policy, stored deadline and active
    /// operation.
    #[allow(clippy::result_large_err)] // The library's `ReadError`.
    fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError> {
        self.service().polling_status(repo)
    }

    /// The repository's remotes, redacted: whether one is selected for
    /// publication over SSH decides whether the scheduler polls it at all
    /// (R-27, U-2). A scheduler call, not part of an automatic poll's own
    /// call list (B4); the read contacts nothing.
    #[allow(clippy::result_large_err)] // The library's `ReadError`.
    fn publication_remotes(&self, repo: &ResolvedRepository) -> Result<RemoteListDto, ReadError> {
        self.service().list_remotes_redacted(repo)
    }

    /// Whether the policy lets an automatic poll go ahead. Contacts
    /// nothing.
    fn poll_eligibility(&self, root: &Path) -> Result<AutomaticPollEligibility, RepositoryError> {
        self.service().automatic_poll_eligibility(root)
    }

    /// The poll itself: `Automatic`, so no job prompt reaches `session`'s
    /// provider (B5).
    #[allow(clippy::result_large_err)] // The library's `RemotePollError`.
    fn poll_automatic(
        &self,
        request: PollPublicationRemoteRequest,
        session: &mut SessionCredentials<DesktopProvider>,
    ) -> Result<RemotePollResult, RemotePollError> {
        self.service().poll_publication_remote(request, session)
    }

    /// What the registry stores of the remote after the poll: its
    /// deadline.
    fn remote_snapshot(&self, root: &Path) -> Result<RemoteSnapshot, RepositoryError> {
        self.service().remote_snapshot(root)
    }

    /// A test may put a constructed answer in place of `poll_automatic`'s,
    /// as `after_job` does for a job: the library builds its poll report
    /// only inside itself, so a test that needs a `Cancelled` or an
    /// `Interrupted` report cannot make one. The call itself still runs.
    fn after_poll(&self, _operation: OperationId) -> Option<poll_convert::Library> {
        None
    }

    /// Resume checking (decision 10, C2): releases the repository's
    /// recovery suspension, and nothing else. Called off the UI thread by
    /// the scheduler.
    fn resume_polling(&self, root: &Path) -> Result<(), RepositoryError> {
        self.service().resume_remote_polling_after_recovery(root)
    }

    /// The key registered as selected, for the attention of a retirement
    /// that was refused a locked key when no key was recorded against it
    /// (D6 E3). A scheduler call, off the UI thread; `None` when no key is
    /// selected or the registry cannot be read.
    fn selected_key(&self) -> Option<SharedKeyId> {
        let keys = self.service().list_keys().ok()?;
        let selected = keys.items.iter().find(|key| key.selected)?;
        SharedKeyId::parse(&selected.id).ok()
    }

    /// Asks the poll `op` of `root` to stop at its next safe point.
    /// Called only by the cancel task (`preempt`: a user job, a
    /// reconfiguration and quit all go that way), never by a poll itself,
    /// and again until the poll answers (B2). It answers `Ok` without
    /// effect on a row that is not active (Fact 30).
    fn cancel_poll(&self, root: &Path, op: OperationId) -> Result<(), RepositoryError> {
        self.service().cancel_remote_operation(root, op)
    }
}

/// The production backend: the library, with no delays.
pub(crate) struct LibraryBackend {
    service: RepositoryService,
    drafts: Result<DraftStore, DraftStoreUnavailable>,
    resolutions: Result<ResolutionStore, ResolutionStoreError>,
    layout: Result<LayoutStore, LayoutStoreError>,
}

impl LibraryBackend {
    /// A backend over `service` and the open results of the draft store,
    /// the resolutions store and the layout store: the desktop starts
    /// whether or not they opened.
    pub(crate) fn new(
        service: RepositoryService,
        drafts: Result<DraftStore, DraftError>,
        resolutions: Result<ResolutionStore, ResolutionStoreError>,
        layout: Result<LayoutStore, LayoutStoreError>,
    ) -> Self {
        Self {
            service,
            drafts: drafts.map_err(DraftStoreUnavailable::from),
            resolutions,
            layout,
        }
    }
}

impl Backend for LibraryBackend {
    fn service(&self) -> &RepositoryService {
        &self.service
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        KeyStore::for_current_user().map_err(|_| KeyStoreUnavailable)
    }

    fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable> {
        self.drafts.as_ref().map_err(|unavailable| *unavailable)
    }

    fn resolutions(&self) -> Result<&ResolutionStore, ResolutionStoreError> {
        self.resolutions.as_ref().map_err(|error| *error)
    }

    fn layout(&self) -> Result<&LayoutStore, LayoutStoreError> {
        self.layout.as_ref().map_err(|error| *error)
    }
}

/// Starts the cancel task of a poll the worker was asked to stop.
struct Canceller {
    backend: Arc<dyn Backend>,
    watch: worker::PollWatch,
    executor: gpui_kit::BackgroundExecutor,
}

impl Canceller {
    /// Spawns `preempt::cancel_until_answered` for the poll `op` of `root`
    /// on the background executor, never the UI thread (P-1).
    fn start(&self, root: model_polling::Root, op: OperationId) {
        let (backend, watch, executor) = (
            self.backend.clone(),
            self.watch.clone(),
            self.executor.clone(),
        );
        self.executor
            .spawn(async move {
                preempt::cancel_until_answered(
                    backend.as_ref(),
                    &watch,
                    &root,
                    op,
                    &preempt::RESEND_DELAYS,
                    |delay| executor.timer(delay),
                )
                .await;
            })
            .detach();
    }
}

/// The handle views hold. Cloning it shares the backend, the worker
/// and the request model.
#[derive(Clone)]
pub(crate) struct Services {
    backend: Arc<dyn Backend>,
    worker: WorkerHandle,
    operations: Entity<Operations>,
    /// The drafts this session knows of, shared by every view.
    drafts: Entity<DraftRegistry>,
    /// The work contexts this process made and saved nothing into (R-75):
    /// `releases` reads them; tests read them here.
    #[cfg(all(test, feature = "desktop-test-support"))]
    created: Entity<CreatedContexts>,
    /// Release on leave (R-37), shared by every view.
    releases: Entity<Releases>,
    /// What each item tab says of its edit session (Part C fixes 1 and 2):
    /// a repository's removal and the Drafts page read it.
    edit_tabs: Entity<EditTabs>,
    /// The comments this session posted, shared by every item tab.
    submissions: Entity<Submissions>,
    /// What follows each lifecycle child's resolution once applied (R-10).
    followups: resolutions::Followups,
    /// What follows a lifecycle's resume or republish (D5 decision 10, A3).
    resends: resends::Resends,
    /// The polls that ended and no one has taken yet. The scheduler takes
    /// them from its own handle on the same inbox; tests read them here.
    #[cfg_attr(not(test), allow(dead_code))]
    polls: PollInbox,
    /// The polling scheduler (D6 B6): one entity, so every clone and window
    /// shares it (A17). `Workspace` holds none (R-16).
    scheduler: Entity<Scheduler>,
    /// The polling switch (R-27): off unless `with_polling`, shared by every
    /// clone and by the drain. Apart from `launch_refresh`, which is on by
    /// default.
    polling: Arc<AtomicBool>,
    /// Whether the workspace refreshes every listed repository once its
    /// list first loads. Tests that do not wait for the worker turn it
    /// off.
    launch_refresh: bool,
}

impl Services {
    /// Starts the mutation worker over `backend`, and the foreground task
    /// that applies its events to the request model. Every event waiting
    /// is applied in one update with one notify, so the views read what a
    /// batch changed once, not once per event. The task ends when the
    /// worker does, after the last handle is dropped. The draft store is
    /// listed once, at launch.
    ///
    /// Each finished request is then passed to the draft registry, after
    /// the request model has it: a save's or a new item's create's answer
    /// applies to its draft, an abandoned operation ends the pending save
    /// it held, and a `context prepare` with the Create intent gives the
    /// worktree its create's `occupied_path` is read in. The registry
    /// outlives tabs, so a save that finishes after its tab closed still
    /// retires its revision.
    ///
    /// A finished `context prepare` is first passed to `CreatedContexts`,
    /// in the same update, so a context is counted before any view hears
    /// of its answer, and whether or not its tab is still open (R-37).
    /// Once the request model has it, release on leave hears of it too, as
    /// of a prepare removed from the queue or not sent: a tab that closed
    /// while it prepared releases the context it made.
    ///
    /// A comment's end is passed to `Submissions` after the request model
    /// has it, so a post's answer lands whether or not a tab is open (P9),
    /// and so is a comment's request queued again, a retry or a Give up
    /// sent from anywhere, the operation area among them, so the post is
    /// posting again and the attempt is a re-send (P10). Each in the order
    /// the worker sent them.
    ///
    /// A resolution's end has its kept set deleted once `show_request`
    /// says its request finished (R-12), and the launch sweep does the same
    /// for every set kept from before (`resolutions`).
    ///
    /// A lifecycle's resume answered `original_request_required` sends the
    /// request it names again, and a republish done from a lifecycle's way
    /// out sends that lifecycle on, before the request model has the answer
    /// (`resends`).
    pub(crate) fn start(backend: Arc<dyn Backend>, cx: &mut App) -> Self {
        let (worker, mut events) = WorkerHandle::spawn(backend.clone());
        resolutions::sweep_at_launch(&backend, cx);
        let settling = backend.clone();
        let operations = cx.new(|_| Operations::default());
        let model = operations.downgrade();
        let drafts = cx.new(|_| DraftRegistry::new(backend.clone(), worker.clone()));
        let registry = drafts.downgrade();
        let created = cx.new(|_| CreatedContexts::default());
        let made = created.downgrade();
        let releases = cx.new(|cx| Releases::new(worker.clone(), created.clone(), &drafts, cx));
        let releasing = releases.downgrade();
        let submissions = cx.new(|_| Submissions::new(worker.clone()));
        let posts = submissions.downgrade();
        let followups = resolutions::Followups::default();
        let following = followups.clone();
        let settled_followups = followups.clone();
        let resending = worker.clone();
        let resends = resends::Resends::default();
        let resent = resends.clone();
        let polls = PollInbox::default();
        let polled = polls.clone();
        let polling = Arc::new(AtomicBool::new(false));
        let edit_tabs = cx.new(|_| EditTabs::default());
        let scheduler = cx.new(|_| {
            Scheduler::new(
                backend.clone(),
                worker.clone(),
                operations.downgrade(),
                polls.clone(),
                polling.clone(),
                edit_tabs.downgrade(),
                drafts.downgrade(),
            )
        });
        let scheduling = scheduler.downgrade();
        let switch = polling.clone();
        // The cancel task of a poll the worker was asked to stop (B2) runs
        // on a background executor: the library's cancel is a write.
        let canceller = Canceller {
            backend: backend.clone(),
            watch: worker.poll_watch(),
            executor: cx.background_executor().clone(),
        };
        cx.spawn(async move |cx| {
            while let Some(event) = events.next().await {
                let mut batch = vec![event];
                while let Ok(event) = events.try_recv() {
                    batch.push(event);
                }
                let applied = model.update(cx, |operations, cx| {
                    let mut finished = Vec::new();
                    let mut prepared = Vec::new();
                    let mut posted = Vec::new();
                    let mut settled = Vec::new();
                    let mut scheduled = Vec::new();
                    for event in batch {
                        // A poll's end is the scheduler's, not a request's
                        // (D6 B1).
                        let event = match event {
                            WorkerEvent::Polled(root, attempt) => {
                                polled.push(root, attempt);
                                continue;
                            }
                            WorkerEvent::PreemptPoll { root, op } => {
                                canceller.start(root, op);
                                continue;
                            }
                            event => event,
                        };
                        // A retirement's answer is read from where the worker's
                        // record of the request stood (E3); `Finished` has none.
                        scheduler::note_record(&event, operations, |id| resending.record_state(id));
                        // A request that reconfigures polling (R-11).
                        scheduled.extend(scheduler::heard(&event, operations));
                        let ended = match &event {
                            WorkerEvent::Finished(id, _)
                            | WorkerEvent::Removed(id)
                            | WorkerEvent::NotSent(id, _) => operations.request(*id),
                            _ => None,
                        };
                        // A comment's end, for `Submissions`: its answer, or
                        // that it never ran.
                        if let Some(request) = ended
                            && matches!(request.intent, requests::Intent::CommentAdd { .. })
                        {
                            posted.push(match &event {
                                WorkerEvent::Finished(_, envelope) => {
                                    PostEvent::Finished(request.id, envelope.clone())
                                }
                                _ => PostEvent::Ended(request.id),
                            });
                        }
                        // A share of an item that published it, whatever
                        // sent it (a Share, a Republish…, a close or an
                        // approval that publishes): its saved comments'
                        // chips clear (decision 14, K-19).
                        if let WorkerEvent::Finished(id, envelope) = &event
                            && let Some(request) = operations.request(*id)
                            && let requests::Intent::ItemSync { root, id: item, .. }
                            | requests::Intent::ItemRepublish { root, id: item, .. }
                            | requests::Intent::TicketClose { root, id: item }
                            | requests::Intent::DocumentPromote { root, id: item } =
                                &request.intent
                            && matches!(
                                envelope.effects.publication,
                                PublicationEffect::Published | PublicationEffect::Current
                            )
                        {
                            posted.push(PostEvent::ItemShared(root.clone(), item.clone()));
                        }
                        // A resolution's end, for its kept set (R-12,
                        // R-25): its answer, or that it never reached the
                        // library.
                        if let Some(request) = ended
                            && let Some(source) = resolutions::source_of(&request.intent)
                        {
                            // A lifecycle child's applied resolution sends
                            // the lifecycle on, sheet open or not (R-10).
                            if let WorkerEvent::Finished(_, envelope) = &event {
                                following.ended(
                                    request.request_id,
                                    envelope.code,
                                    operations,
                                    &resending,
                                );
                            }
                            settled.push(resolutions::Ended {
                                request: request.request_id,
                                source,
                                answer: match &event {
                                    WorkerEvent::Finished(_, envelope) => {
                                        Some((envelope.code, envelope.effects.checkpoint))
                                    }
                                    _ => None,
                                },
                                retry: matches!(
                                    &event,
                                    WorkerEvent::Finished(_, envelope)
                                        if envelope.recovery.iter().any(|action| {
                                            action.action == RecoveryActionKind::RequestRetry
                                        })
                                ),
                            });
                        }
                        // A run queued again leads on from here afresh.
                        if let WorkerEvent::Queued(request) = &event
                            && request.attempt > 1
                        {
                            resent.requeued(request.id);
                        }
                        // A comment queued again under its own request.
                        if let WorkerEvent::Queued(request) = &event
                            && request.attempt > 1
                            && matches!(request.intent, requests::Intent::CommentAdd { .. })
                        {
                            posted.push(PostEvent::Resent(request.id));
                        }
                        // A prepare's end, for release on leave: what it
                        // made, if anything. One removed from the queue, or
                        // not sent, made nothing.
                        let prepare = ended.and_then(|request| {
                            matches!(request.intent, requests::Intent::ContextPrepare { .. })
                                .then(|| (request.id, ContextKey::of(&request.intent)))
                        });
                        // A republish that never ran will not be done.
                        if let WorkerEvent::Removed(id) | WorkerEvent::NotSent(id, _) = &event {
                            resent.not_run(*id);
                        }
                        let mut made_here = false;
                        if let WorkerEvent::Finished(id, envelope) = &event {
                            resent.ended(*id, envelope, operations, &resending);
                            if let Some(request) = operations.request(*id) {
                                made_here = made
                                    .update(cx, |created, _| {
                                        created.finished(&request.intent, envelope)
                                    })
                                    .unwrap_or(false);
                            }
                            if let Some(forward) = for_drafts(operations, *id, envelope) {
                                finished.push(forward);
                            }
                        }
                        if let Some((id, item)) = prepare {
                            prepared.push((id, item, made_here));
                        }
                        operations.apply(event);
                    }
                    cx.notify();
                    (finished, prepared, posted, settled, scheduled)
                });
                let Ok((finished, prepared, posted, settled, scheduled)) = applied else {
                    break;
                };
                // Every batch pokes the scheduler: a poll may have ended and
                // the user lane may have emptied. It does nothing while
                // polling is off (R-27).
                if switch.load(Ordering::Relaxed) {
                    let _ = scheduling.update(cx, |scheduler, cx| scheduler.heard(scheduled, cx));
                }
                if !settled.is_empty() {
                    resolutions::settle_finished(&settling, &settled_followups, settled, cx);
                }
                if !posted.is_empty() {
                    let retire = posts
                        .update(cx, |submissions, cx| {
                            let mut retire = Vec::new();
                            for event in posted {
                                match event {
                                    PostEvent::Resent(id) => {
                                        submissions.resent(id);
                                    }
                                    // A saved comment's draft retires here,
                                    // whether or not a tab is open (P9).
                                    PostEvent::Finished(id, envelope) => {
                                        let left = submissions.finished(id, envelope);
                                        if left
                                            .is_some_and(|left| left.saved && left.draft.is_some())
                                        {
                                            retire.extend(submissions.to_retire(id));
                                        }
                                    }
                                    PostEvent::Ended(id) => {
                                        submissions.ended(id);
                                    }
                                    PostEvent::ItemShared(root, item) => {
                                        submissions.item_shared(&root, &item);
                                    }
                                }
                            }
                            cx.notify();
                            retire
                        })
                        .unwrap_or_default();
                    for (root, comment, draft) in retire {
                        submissions::retire_posted(&registry, &posts, root, comment, draft, cx);
                    }
                }
                if !prepared.is_empty() {
                    let _ = releasing.update(cx, |releases, cx| {
                        for (id, item, made) in prepared {
                            releases.prepared(id, item, made, cx);
                        }
                    });
                }
                if finished.is_empty() {
                    continue;
                }
                // The registry lives as long as the request model.
                let _ = registry.update(cx, |registry, cx| {
                    for forward in finished {
                        match forward {
                            ForDrafts::Save(id, envelope) => registry.finished(id, &envelope, cx),
                            ForDrafts::Abandoned(operation) => {
                                registry.operation_abandoned(&operation, cx)
                            }
                            ForDrafts::Prepared(key, worktree) => {
                                registry.context_prepared(key, worktree)
                            }
                        }
                    }
                });
            }
        })
        .detach();
        let services = Self {
            backend,
            worker,
            operations,
            drafts: drafts.clone(),
            #[cfg(all(test, feature = "desktop-test-support"))]
            created,
            releases,
            edit_tabs,
            submissions,
            followups,
            resends,
            polls,
            scheduler,
            polling,
            launch_refresh: true,
        };
        drafts.update(cx, |registry, cx| {
            registry.reload_listing(cx);
        });
        services
    }

    /// The same services, with no refresh at launch: for a test that
    /// does not wait for the worker.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn without_launch_refresh(mut self) -> Self {
        self.launch_refresh = false;
        self
    }

    pub(crate) fn launch_refresh(&self) -> bool {
        self.launch_refresh
    }

    /// The same services, with polling switched on (R-27): the repository
    /// list starts the launch poll and the scheduler polls what is due. Off
    /// unless this is called; production `run()` calls it. The switch is
    /// shared with every clone.
    pub(crate) fn with_polling(self) -> Self {
        self.polling.store(true, Ordering::Relaxed);
        self
    }

    /// Whether polling is switched on.
    pub(crate) fn polling(&self) -> bool {
        self.polling.load(Ordering::Relaxed)
    }

    /// The polling scheduler, shared by every clone (A17).
    pub(crate) fn scheduler(&self) -> &Entity<Scheduler> {
        &self.scheduler
    }

    /// The same services with the scheduler reading `clock`, for a test
    /// that moves the time by hand.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn with_clock(self, clock: Arc<dyn model_polling::Clock>, cx: &mut App) -> Self {
        self.scheduler
            .update(cx, |scheduler, _| scheduler.use_clock(clock));
        self
    }

    /// The key store a key command is built with.
    pub(crate) fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        self.backend.key_store()
    }

    pub(crate) fn worker(&self) -> &WorkerHandle {
        &self.worker
    }

    /// This session's requests, as the worker's events left them.
    pub(crate) fn operations(&self) -> &Entity<Operations> {
        &self.operations
    }

    /// Where the item `id` of the repository at `root` stands under the
    /// lifecycle lock (D5 A2; R-21 to R-23): an open lifecycle dialog of
    /// it, its lifecycle in the worker, or its L2 row.
    pub(crate) fn lifecycle_lock(&self, root: &str, id: &str, cx: &App) -> LockState {
        let operations = self.operations.read(cx);
        let key = LockKey::new(root, id);
        let held = operations.locks().operations(&key);
        let in_flight = self
            .worker
            .in_flight(|intent| lifecycle_of(intent, &key, &held));
        let loading = matches!(operations.pending().get(root), Some(ListState::Loading));
        operations.locks().state(&key, in_flight, loading)
    }

    /// Whether the local copy of the item `id` of `kind` in `root` is being
    /// retired (D6 decision 19, E1): a retirement of it is queued or
    /// running in the worker, of any lane, a mark holds it for the loop or a
    /// Retire… not yet handed over, or the in-flight record holds one that
    /// is pending, after a relaunch too. Keyed by all three, and counted per
    /// request: one ending clears nothing of another's.
    pub(crate) fn retiring(
        &self,
        root: &str,
        kind: manyhands::repository::ItemDtoKind,
        id: &str,
        cx: &App,
    ) -> bool {
        self.operations.read(cx).is_retiring(root, kind, id)
            || self.worker.in_flight(|intent| {
                matches!(
                    intent,
                    requests::Intent::ContextRetire { root: asked, kind: asked_kind, id: asked_id }
                        if asked.to_str() == Some(root)
                            && retire_kind(*asked_kind) == kind
                            && asked_id.to_string() == id
                )
            })
    }

    /// Where the item `id` of `kind` in `root` stands under every lock: the
    /// lifecycle's (`lifecycle_lock`), then "Retiring…" (D6 decision 19). A
    /// lifecycle that holds the item says so first.
    pub(crate) fn lock(
        &self,
        root: &str,
        kind: manyhands::repository::ItemDtoKind,
        id: &str,
        cx: &App,
    ) -> LockState {
        let state = self.lifecycle_lock(root, id, cx);
        if matches!(state, LockState::Locked(_)) || !self.retiring(root, kind, id, cx) {
            return state;
        }
        LockState::Locked(model_lock::LockSource::Retiring)
    }

    /// Whether a republish of the item `id` of `root` is queued or running.
    pub(crate) fn republish_running(
        &self,
        root: &std::path::Path,
        id: &manyhands::canonical::ItemId,
    ) -> bool {
        self.worker.in_flight(|intent| {
            matches!(intent, requests::Intent::ItemRepublish { root: asked, id: asked_id, .. }
                if asked == root && asked_id == id)
        })
    }

    /// Whether a republish of the item `id` of `root` is queued or running,
    /// or one confirmed from a lifecycle's way out waits to be done (D5
    /// Part C; review I-2, R-30): no other Republish… of it is offered
    /// meanwhile.
    pub(crate) fn republishing(
        &self,
        root: &std::path::Path,
        id: &manyhands::canonical::ItemId,
    ) -> bool {
        self.republish_running(root, id) || self.resends.waiting_for(root, id)
    }

    /// The drafts this session knows of.
    pub(crate) fn drafts(&self) -> &Entity<DraftRegistry> {
        &self.drafts
    }

    /// The work contexts this process made and saved nothing into.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn created_contexts(&self) -> &Entity<CreatedContexts> {
        &self.created
    }

    /// Release on leave: what each item's tab and its drafts' discards
    /// hand over (R-37).
    pub(crate) fn releases(&self) -> &Entity<Releases> {
        &self.releases
    }

    /// What each item tab says of its edit session.
    pub(crate) fn edit_tabs(&self) -> &Entity<EditTabs> {
        &self.edit_tabs
    }

    /// The comments this session posted.
    pub(crate) fn submissions(&self) -> &Entity<Submissions> {
        &self.submissions
    }

    /// What follows each lifecycle child's resolution once applied.
    pub(crate) fn followups(&self) -> &resolutions::Followups {
        &self.followups
    }

    /// What follows a lifecycle's resume or republish.
    pub(crate) fn resends(&self) -> &resends::Resends {
        &self.resends
    }

    pub(crate) fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }
}

/// The kind of item a retirement names, as the tabs know it.
fn retire_kind(kind: manyhands::repository::AuthoringKind) -> manyhands::repository::ItemDtoKind {
    match kind {
        manyhands::repository::AuthoringKind::Ticket => manyhands::repository::ItemDtoKind::Ticket,
        manyhands::repository::AuthoringKind::Document => {
            manyhands::repository::ItemDtoKind::Document
        }
    }
}

/// Whether `intent` is the lifecycle of `key`: its close or merge, or a
/// resume of one of `operations`, its lifecycle operations.
pub(crate) fn lifecycle_of(
    intent: &requests::Intent,
    key: &LockKey,
    operations: &[String],
) -> bool {
    use requests::Intent;
    let same_root = |root: &std::path::Path| root.to_str() == Some(key.root.as_str());
    match intent {
        Intent::TicketClose { root, id } | Intent::DocumentPromote { root, id } => {
            same_root(root) && id.to_string() == key.id
        }
        Intent::OperationResume { root, operation_id } => {
            same_root(root) && operations.contains(&operation_id.to_string())
        }
        _ => false,
    }
}

/// What the drain hands `Submissions` of a comment's request.
enum PostEvent {
    /// Queued again under its own request: a retry or a Give up.
    Resent(requests::DesktopRequestId),
    Finished(requests::DesktopRequestId, worker::FinishedEnvelope),
    /// Removed from the queue, or not sent: that attempt never ran.
    Ended(requests::DesktopRequestId),
    /// A share of the item, by its repository and ID, published it or
    /// found it current.
    ItemShared(PathBuf, manyhands::canonical::ItemId),
}

/// What a finished request means to the draft registry.
enum ForDrafts {
    /// A save or a new item's create finished: its answer applies to its
    /// draft.
    Save(requests::DesktopRequestId, Box<Envelope<MutationDataDto>>),
    /// An `operation abandon` ended this operation: a pending save that
    /// held it is over (ruling R22).
    Abandoned(String),
    /// A `context prepare` with the Create intent made the item's context
    /// ready, at this worktree: where its create's `occupied_path` is read.
    Prepared(ContextKey, PathBuf),
}

fn for_drafts(
    operations: &Operations,
    id: requests::DesktopRequestId,
    envelope: &Envelope<MutationDataDto>,
) -> Option<ForDrafts> {
    let intent = &operations.request(id)?.intent;
    match intent {
        requests::Intent::DocumentSave { .. }
        | requests::Intent::TicketSave { .. }
        | requests::Intent::DocumentCreate { .. }
        | requests::Intent::TicketCreate { .. } => {
            Some(ForDrafts::Save(id, Box::new(envelope.clone())))
        }
        requests::Intent::OperationAbandon { operation_id, .. }
            if matches!(envelope.outcome, Outcome::Success | Outcome::Noop) =>
        {
            Some(ForDrafts::Abandoned(operation_id.to_string()))
        }
        requests::Intent::ContextPrepare {
            intent: ContextPrepareIntent::Create,
            ..
        } if envelope.code == ResultCode::Ok => match &envelope.data {
            Some(MutationDataDto::Context(context)) => Some(ForDrafts::Prepared(
                ContextKey::of(intent)?,
                PathBuf::from(&context.worktree),
            )),
            _ => None,
        },
        _ => None,
    }
}
