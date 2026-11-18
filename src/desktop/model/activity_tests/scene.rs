//! The fixture of the log's two tables (`attention_counts_each_thing_once`,
//! P2, and `the_log_keeps_times_and_events`, P3): a `Scene` is what the
//! Workspace would hold, a log over it, an injected clock and the facts of
//! each request. The table machinery is `table.rs`.

use std::collections::{BTreeMap, BTreeSet};

use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, OperationDto, ProgressStage},
    results::{Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode},
};
use time::{Duration, OffsetDateTime};

use super::{
    intents::{intent, lifecycle_answer, share_answer},
    states::world::World,
    support::{ITEM, TITLE, answer, facts, finish, id, queue, root, stage, start},
    utc,
};
use crate::desktop::{
    model::{
        activity::{
            Inputs, derive,
            log::{ActivityLog, pending_held},
            requests::RequestFacts,
            rows::{ActivityRow, Derived, RowKey},
            states::DraftsInput,
        },
        polling::KeyId,
        stamp::RepoKey,
    },
    services::{
        ReadResult,
        requests::{DesktopRequestId, Intent, RequestState},
        worker::{RecordState, WorkerEvent},
    },
};

/// Another operation than `OPERATION`.
pub(super) const OTHER_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";

/// What the Workspace holds at a refresh, and the log it refreshes.
pub(super) struct Scene {
    pub world: World,
    /// The facts of each request, by number; a request with none has the
    /// facts of one with nothing special about it.
    pub facts: BTreeMap<u64, RequestFacts>,
    /// Requests whose row has no attempt number (`Group.sent` is `None`),
    /// as for one the model never saw queued.
    pub unsent: BTreeSet<u64>,
    pub log: ActivityLog,
    /// The injected clock: no real time passes.
    pub now: OffsetDateTime,
    /// Numbers a step keeps for a later one to compare.
    notes: BTreeMap<&'static str, u64>,
    /// What the last refresh reported: some row changed.
    pub changed: bool,
}

/// The clock's start.
pub(super) fn start_time() -> OffsetDateTime {
    utc(2026, 11, 13, 10, 0, 0)
}

/// `minutes` after the clock's start.
pub(super) fn after(minutes: i64) -> OffsetDateTime {
    start_time() + Duration::minutes(minutes)
}

pub(super) fn req(n: u64) -> RowKey {
    RowKey::Request(id(n))
}

pub(super) fn repo(root: &str) -> RowKey {
    RowKey::Repository(root.to_owned())
}

pub(super) fn pending_row(root: &str, operation: &str) -> RowKey {
    RowKey::Pending {
        root: root.to_owned(),
        operation: operation.to_owned(),
    }
}

/// A hold's key, R-3: the variant's name and its key.
pub(super) fn key_hold(key: KeyId) -> RowKey {
    RowKey::Hold(format!("KeyLocked:{key}"))
}

pub(super) fn item() -> ItemId {
    ITEM.parse().unwrap()
}

impl Scene {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            facts: BTreeMap::new(),
            unsent: BTreeSet::new(),
            log: ActivityLog::default(),
            now: start_time(),
            notes: BTreeMap::new(),
            changed: false,
        }
    }

    /// Keeps `value` under `name` for a later step.
    pub fn remember(&mut self, name: &'static str, value: u64) {
        self.notes.insert(name, value);
    }

    pub fn recall(&self, name: &'static str) -> u64 {
        self.notes[name]
    }

    /// The rows the Workspace would derive now.
    pub fn derived(&self) -> Vec<Derived> {
        let world = &self.world;
        let requests =
            |request: DesktopRequestId| self.facts.get(&request.0).cloned().unwrap_or_else(facts);
        let waits = |root: &RepoKey, operation: &str| {
            world
                .waits
                .iter()
                .any(|(held, wait)| held == root && wait == operation)
        };
        let title =
            |_: &RepoKey, _: AuthoringKind, _: &ItemId| world.titled.then(|| TITLE.to_owned());
        let primary = |_: &RepoKey| Some("trunk".to_owned());
        let unlock = |_: &KeyId| world.unlockable;
        let running = |root: &RepoKey, item: &ItemId| {
            world
                .running
                .iter()
                .any(|(held, running)| held == root && running == item)
        };
        let pending = world.operations.pending_shown();
        let mut rows = derive(&Inputs {
            operations: &world.operations,
            pending: &pending,
            schedule: &world.schedule,
            repositories: &world.repositories,
            drafts: DraftsInput {
                flush_failures: world.drafts.flush_failures.clone(),
                unavailable: world.drafts.unavailable,
                recovered: world.drafts.recovered,
            },
            requests: &requests,
            conflict_waits: &waits,
            title: &title,
            primary: &primary,
            unlock_target: &unlock,
            lifecycle_running: &running,
        });
        for row in &mut rows {
            if matches!(&row.key, RowKey::Request(request) if self.unsent.contains(&request.0)) {
                row.group.sent = None;
            }
        }
        rows
    }

    /// Derives, then refreshes the log with the listed repositories' checking
    /// set, `held` and the clock. True when the log says a row changed.
    pub fn refresh(&mut self) -> bool {
        let derived = self.derived();
        let checking: BTreeSet<RepoKey> = self
            .world
            .repositories
            .iter()
            .filter(|repository| repository.checking)
            .map(|repository| repository.root.clone())
            .collect();
        // L3b's trigger, over the listed repositories, as the Workspace builds it.
        let listed: Vec<RepoKey> = self
            .world
            .repositories
            .iter()
            .map(|repository| repository.root.clone())
            .collect();
        let held = pending_held(&self.world.operations, &listed);
        self.log.refresh(derived, &checking, &held, self.now)
    }

    /// The attention rows' keys, sorted: the footer's count and the popover's
    /// list. The log's count must be the number of them (A6).
    pub fn counted(&self) -> Vec<RowKey> {
        let mut keys: Vec<RowKey> = self
            .log
            .rows()
            .iter()
            .filter(|row| row.row.attention)
            .map(|row| row.row.key.clone())
            .collect();
        keys.sort();
        assert_eq!(
            self.log.attention_count(),
            keys.len(),
            "count and rows agree"
        );
        keys
    }

    /// Every row's key, in the log's order.
    pub fn shown(&self) -> Vec<RowKey> {
        self.log
            .rows()
            .iter()
            .map(|row| row.row.key.clone())
            .collect()
    }

    /// The row `key`, which must be in the log.
    pub fn row(&self, key: &RowKey) -> &ActivityRow {
        self.log
            .rows()
            .iter()
            .find(|row| &row.row.key == key)
            .unwrap_or_else(|| panic!("no row {key:?} in {:?}", self.shown()))
    }

    pub fn has(&self, key: &RowKey) -> bool {
        self.shown().contains(key)
    }

    /// The facts of request `n`, to change.
    pub fn facts_of(&mut self, n: u64) -> &mut RequestFacts {
        self.facts.entry(n).or_insert_with(facts)
    }

    pub fn at(&mut self, minutes: i64) {
        self.now = after(minutes);
    }
}

/// Requests, through the worker's own events.
impl Scene {
    /// Request `n` of the intent `name`, queued.
    pub fn queue(&mut self, n: u64, name: &str) {
        queue(&mut self.world.operations, n, intent(name));
    }

    pub fn queue_intent(&mut self, n: u64, intent: Intent) {
        queue(&mut self.world.operations, n, intent);
    }

    pub fn run(&mut self, n: u64) {
        start(&mut self.world.operations, n);
    }

    /// Request `n`'s first progress event, naming `operation` when it has one:
    /// it is accepted from then on.
    pub fn stage(&mut self, n: u64, at: ProgressStage, operation: Option<&str>) {
        stage(&mut self.world.operations, n, at, operation);
    }

    pub fn finish(&mut self, n: u64, envelope: Envelope<manyhands::repository::MutationDataDto>) {
        finish(&mut self.world.operations, n, envelope);
    }

    /// Request `n` of `name`, finished `outcome` under `code`.
    pub fn end(&mut self, n: u64, name: &str, outcome: Outcome, code: ResultCode) {
        self.queue(n, name);
        self.run(n);
        self.finish(n, answer(outcome, code));
    }

    /// A request that failed: `Error`, under `internal_error`.
    pub fn fail(&mut self, n: u64, name: &str) {
        self.end(n, name, Outcome::Error, ResultCode::InternalError);
    }

    /// A request that failed with `code`.
    pub fn fail_code(&mut self, n: u64, name: &str, code: ResultCode) {
        self.end(n, name, Outcome::Error, code);
    }

    /// A request that failed and whose answer offers to abandon the pending
    /// operation `operation`: it names the operation (R-23).
    pub fn fail_op(&mut self, n: u64, name: &str, operation: &str) {
        self.queue(n, name);
        self.run(n);
        self.finish_op(n, operation);
    }

    /// The running request `n` fails, naming `operation`.
    pub fn finish_op(&mut self, n: u64, operation: &str) {
        let mut envelope = answer(Outcome::Error, ResultCode::InternalError);
        envelope.recovery = vec![RecoveryAction {
            action: RecoveryActionKind::OperationAbandon,
            operation_id: Some(operation.to_owned()),
            arguments: Default::default(),
        }];
        self.finish(n, envelope);
    }

    /// A Resume of `operation`, answered `outcome` under `code`.
    pub fn resume(&mut self, n: u64, operation: &str, outcome: Outcome, code: ResultCode) {
        self.queue_intent(
            n,
            Intent::OperationResume {
                root: root(),
                operation_id: manyhands::repository::OperationId::parse(operation).unwrap(),
            },
        );
        self.run(n);
        self.finish(n, answer(outcome, code));
    }

    /// The retirement of `item`, answered `envelope`, the worker's record at
    /// `record`.
    pub fn retire(
        &mut self,
        n: u64,
        item: ItemId,
        record: RecordState,
        envelope: Envelope<manyhands::repository::MutationDataDto>,
    ) {
        let kind = if item == "01ARZ3NDEKTSV4RRFFQ69G5FD0".parse().unwrap() {
            AuthoringKind::Document
        } else {
            AuthoringKind::Ticket
        };
        self.queue_intent(
            n,
            Intent::ContextRetire {
                root: root(),
                kind,
                id: item,
            },
        );
        self.run(n);
        self.world.operations.record_stood(id(n), record);
        self.finish(n, envelope);
    }

    /// A retirement running: it has named `OPERATION` in its first progress.
    pub fn queue_retire(&mut self, n: u64) {
        use manyhands::repository::ProgressStage;
        self.queue_intent(
            n,
            Intent::ContextRetire {
                root: root(),
                kind: AuthoringKind::Ticket,
                id: item(),
            },
        );
        self.run(n);
        self.stage(
            n,
            ProgressStage::Observing,
            Some("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
        );
    }

    /// One of this app's automatic polls runs `operation`.
    pub fn poll_runs(&mut self, operation: &str) {
        let poll = manyhands::repository::OperationId::parse(operation).unwrap();
        self.world
            .operations
            .set_running_polls(std::collections::HashSet::from([poll]));
    }

    /// No automatic poll runs.
    pub fn poll_runs_none(&mut self) {
        self.world
            .operations
            .set_running_polls(std::collections::HashSet::new());
    }

    /// A request that succeeded.
    pub fn succeed(&mut self, n: u64, name: &str) {
        self.queue(n, name);
        self.run(n);
        let envelope = match name {
            "TicketClose" | "DocumentPromote" => lifecycle_answer(Outcome::Success),
            // A share that succeeded published (B3).
            "ItemSync" | "RepoSync" | "ItemRepublish" => share_answer("Published"),
            _ => answer(Outcome::Success, ResultCode::Ok),
        };
        self.finish(n, envelope);
    }

    /// A share of `name` stopped, its record at `record` (B3's Stopped).
    pub fn stop_share(&mut self, n: u64, name: &str, record: RecordState) {
        self.queue(n, name);
        self.run(n);
        self.world
            .operations
            .note_share_attempt(id(n), record, false);
        self.finish(n, share_answer("Stopped"));
    }

    /// A lifecycle (close) cancelled: its row offers Open….
    pub fn cancel(&mut self, n: u64, name: &str) {
        self.end(n, name, Outcome::Cancelled, ResultCode::Cancelled);
    }

    /// Request `n` is sent again under the same ID: a new attempt, queued.
    pub fn retry(&mut self, n: u64) {
        let mut request = self
            .world
            .operations
            .request(id(n))
            .expect("the request")
            .clone();
        request.state = RequestState::Queued;
        request.attempt += 1;
        self.world.operations.apply(WorkerEvent::Queued(request));
    }

    /// The person dismisses request `n` (`dismiss_request`).
    pub fn dismiss(&mut self, n: u64) {
        self.world.operations.dismiss(id(n));
    }

    /// Request `n` ends cancelled while it waits in the queue.
    pub fn remove(&mut self, n: u64) {
        self.world.operations.apply(WorkerEvent::Removed(id(n)));
    }
}

/// Pending operations.
impl Scene {
    /// `root`'s pending list, read: `rows` listed.
    pub fn listed(&mut self, root: &str, rows: Vec<OperationDto>) {
        self.world.pending(root, rows);
    }

    /// `root`'s pending list, read again with `result`.
    pub fn read(
        &mut self,
        root: &str,
        result: ReadResult<manyhands::repository::OperationListDto>,
    ) {
        let operations = &mut self.world.operations;
        let stamp = operations.load_pending(root.to_owned());
        assert!(operations.apply_pending(stamp, result));
    }

    /// `root` is listed and has an automatic answer applied.
    pub fn answered(&mut self, root: &str, answer: crate::desktop::model::polling::Answer) {
        self.world.schedule_root(root);
        self.world.automatic(root, answer, &[]);
    }
}

pub(super) use super::table::{Row, Step, play, row, step};
