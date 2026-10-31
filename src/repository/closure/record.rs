//! Ticket closure parent storage only: no transport, Git effect, prompt or
//! lease acquisition. The record holds IDs, ref names, OIDs, timestamps
//! (`closed_at` included), fixed step names and fixed categories; never
//! `closed_by`, a title, a body, an endpoint or a secret. The table-agnostic
//! parent logic is `promotion::lifecycle_record`'s, shared with promotion.
use super::{ClosureBlocker, ClosureExpectation};
use crate::{
    canonical::ItemId,
    repository::{
        OperationId, RemoteOperationPriority, RemoteOperationTarget, RepositoryError,
        RepositoryService,
        cleanup::RetirementEvidence,
        promotion::{
            lifecycle::{LifecycleKind, LifecycleRecord, ParentStep},
            lifecycle_record::{self as parent, Blocker, ParentRecord, categories},
            record::{CleanupCategory, IntegrationKind, PublicationCategory},
        },
        remote::{reservation::RemoteReservation, state},
    },
};
use git2::Oid;
use rusqlite::{Connection, Transaction, params};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

pub(in super::super) use crate::repository::promotion::lifecycle_record::{
    ChildExecution, ChildRole, ChildState,
};

const KIND: LifecycleKind = LifecycleKind::TicketClosure;
categories!(ClosureStep {
    Confirmed=>"confirmed", ContextSynchronized=>"context_synchronized", Closing=>"closing",
    Integrating=>"integrating", Integrated=>"integrated", Published=>"published",
    PublicationPending=>"publication_pending", RemoteDeleted=>"remote_deleted",
    WorktreeRemoving=>"worktree_removing", WorktreeRemoved=>"worktree_removed",
    BranchDeleted=>"branch_deleted", Complete=>"complete", Abandoned=>"abandoned", Superseded=>"superseded"
});
categories!(ClosureKind { Record=>"record", Existing=>"existing" });

impl ClosureStep {
    pub(in super::super) fn terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Abandoned | Self::Superseded)
    }
    /// From `integrating` on: the closure has reached the primary or is
    /// reaching it.
    fn integrating_or_later(self) -> bool {
        use ClosureStep::*;
        matches!(
            self,
            Integrating
                | Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        )
    }
    /// A `record` closure passes `closing` (its checkpoint) and an `existing`
    /// one never does. `closing → abandoned` is permitted here; only the
    /// runner knows whether the checkpoint ref has moved, and it alone calls
    /// it. `closing` is never superseded.
    fn permits(self, next: Self, remote: bool, kind: Option<ClosureKind>) -> bool {
        use ClosureStep::*;
        if self.terminal() {
            return false;
        }
        self == next
            || matches!(
                (self, next),
                (Confirmed, ContextSynchronized)
                    | (Closing, Integrating)
                    | (Integrating, Integrated)
                    | (Integrated, Published | PublicationPending)
                    | (PublicationPending, Published)
                    | (Published, RemoteDeleted)
                    | (RemoteDeleted, WorktreeRemoving)
                    | (WorktreeRemoving, WorktreeRemoved)
                    | (WorktreeRemoved, BranchDeleted)
                    | (Confirmed | ContextSynchronized | Closing, Abandoned)
            )
            || kind == Some(ClosureKind::Record) && (self, next) == (ContextSynchronized, Closing)
            || kind == Some(ClosureKind::Existing)
                && (self, next) == (ContextSynchronized, Integrating)
            || (!remote && matches!((self, next), (PublicationPending, WorktreeRemoving)))
    }
}

/// The runner's shared view. `closing` reads as `context_synchronized`: the
/// runner has not integrated yet, and the closure hook owns the extra step.
impl From<ClosureStep> for ParentStep {
    fn from(step: ClosureStep) -> Self {
        match step {
            ClosureStep::Confirmed => Self::Confirmed,
            ClosureStep::ContextSynchronized | ClosureStep::Closing => Self::ContextSynchronized,
            ClosureStep::Integrating => Self::Integrating,
            ClosureStep::Integrated => Self::Integrated,
            ClosureStep::Published => Self::Published,
            ClosureStep::PublicationPending => Self::PublicationPending,
            ClosureStep::RemoteDeleted => Self::RemoteDeleted,
            ClosureStep::WorktreeRemoving => Self::WorktreeRemoving,
            ClosureStep::WorktreeRemoved => Self::WorktreeRemoved,
            ClosureStep::BranchDeleted => Self::BranchDeleted,
            ClosureStep::Complete => Self::Complete,
            ClosureStep::Abandoned => Self::Abandoned,
            ClosureStep::Superseded => Self::Superseded,
        }
    }
}
impl From<ParentStep> for ClosureStep {
    fn from(step: ParentStep) -> Self {
        match step {
            ParentStep::Confirmed => Self::Confirmed,
            ParentStep::ContextSynchronized => Self::ContextSynchronized,
            ParentStep::Integrating => Self::Integrating,
            ParentStep::Integrated => Self::Integrated,
            ParentStep::Published => Self::Published,
            ParentStep::PublicationPending => Self::PublicationPending,
            ParentStep::RemoteDeleted => Self::RemoteDeleted,
            ParentStep::WorktreeRemoving => Self::WorktreeRemoving,
            ParentStep::WorktreeRemoved => Self::WorktreeRemoved,
            ParentStep::BranchDeleted => Self::BranchDeleted,
            ParentStep::Complete => Self::Complete,
            ParentStep::Abandoned => Self::Abandoned,
            ParentStep::Superseded => Self::Superseded,
        }
    }
}

pub(in super::super) type Refusal = parent::Refusal<ClosureBlocker>;
/// The design's blocker order; bit `i` of the stored mask is entry `i`.
pub(in super::super) const CLOSURE_BLOCKERS: [ClosureBlocker; 21] = [
    ClosureBlocker::NotATicket,
    ClosureBlocker::ContextMissing,
    ClosureBlocker::ContextNonconforming,
    ClosureBlocker::ContextDirty,
    ClosureBlocker::ContextConflicted,
    ClosureBlocker::ConflictPending,
    ClosureBlocker::LocalOperationPending,
    ClosureBlocker::CommentPublicationPending,
    ClosureBlocker::PrimaryMissing,
    ClosureBlocker::PrimaryNotCheckedOut,
    ClosureBlocker::PrimaryDirty,
    ClosureBlocker::PrimaryConflicted,
    ClosureBlocker::PrimaryAheadOfRemote,
    ClosureBlocker::ContextBehindRemote,
    ClosureBlocker::IntegrationConflict,
    ClosureBlocker::RemoteContextDeleted,
    ClosureBlocker::HistoryUnknown,
    ClosureBlocker::AlreadyClosed,
    ClosureBlocker::PrimaryReopened,
    ClosureBlocker::TicketInvalidAfterClosure,
    ClosureBlocker::IdentityRequired,
];
impl Blocker for ClosureBlocker {
    const ORDER: &'static [Self] = &CLOSURE_BLOCKERS;
    const INTEGRATION_CONFLICT: Self = ClosureBlocker::IntegrationConflict;
}

/// The promotion record's fields and types, with closure's own step,
/// digest and refusal, plus the closure effect. `closure_kind` is absent
/// only on a refusal recorded at insertion, before any closure effect was
/// observed.
#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct ClosureRecord {
    pub repository_id: i64,
    pub operation_id: OperationId,
    pub item_id: ItemId,
    pub expectation: ClosureExpectation,
    pub remote_name: Option<String>,
    pub step: ClosureStep,
    pub context_attempt: u32,
    pub context_sync_id: Option<OperationId>,
    pub primary_attempt: u32,
    pub primary_sync_id: Option<OperationId>,
    pub base_oid: Option<Oid>,
    pub context_oid: Option<Oid>,
    pub integration_oid: Option<Oid>,
    pub integration_kind: Option<IntegrationKind>,
    pub remote_context_oid: Option<Oid>,
    pub publication: Option<PublicationCategory>,
    pub cleanup: Option<CleanupCategory>,
    pub cancel_requested: bool,
    pub updated_at: i64,
    pub refusal: Option<Refusal>,
    pub closure_kind: Option<ClosureKind>,
    /// Whole UTC seconds; set with `closing`.
    pub closed_at: Option<OffsetDateTime>,
    /// The checkpoint commit; set with `closing`.
    pub closure_oid: Option<Oid>,
    /// A deliberate synchronization over the checkpoint that the C9 resume
    /// adopted as the merged context: a descendant of `closure_oid`, set
    /// only while `closing` and fixed from `integrating` on.
    pub adopted_context_oid: Option<Oid>,
}
impl ClosureRecord {
    /// The context commit integration merges: an adopted synchronization
    /// over the checkpoint, else the checkpoint when one was written, else
    /// the synchronized pre-closure context.
    pub(in super::super) fn merged_context(&self) -> Option<Oid> {
        self.adopted_context_oid
            .or(self.closure_oid)
            .or(self.context_oid)
    }
}
impl std::fmt::Debug for ClosureRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosureRecord")
            .field("operation_id", &self.operation_id)
            .field("item_id", &self.item_id)
            .field("step", &self.step)
            .field("closure_kind", &self.closure_kind)
            .field("context_attempt", &self.context_attempt)
            .field("primary_attempt", &self.primary_attempt)
            .field("closure_oid", &self.closure_oid)
            .field("adopted_context_oid", &self.adopted_context_oid)
            .field("integration_oid", &self.integration_oid)
            .field("publication", &self.publication)
            .field("cleanup", &self.cleanup)
            .field("refusal", &self.refusal)
            .finish()
    }
}

const SCHEMA:&str="CREATE TABLE ticket_closures (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    operation_id TEXT NOT NULL UNIQUE CHECK(length(operation_id)=26),
    item_id TEXT NOT NULL CHECK(length(item_id)=26),
    expectation BLOB NOT NULL CHECK(typeof(expectation)='blob' AND length(expectation)=32),
    remote_name TEXT,
    step TEXT NOT NULL CHECK(step IN ('confirmed','context_synchronized','closing','integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete','abandoned','superseded')),
    context_attempt INTEGER NOT NULL DEFAULT 0 CHECK(context_attempt BETWEEN 0 AND 4294967295),
    context_sync_id TEXT UNIQUE CHECK(length(context_sync_id)=26),
    primary_attempt INTEGER NOT NULL DEFAULT 0 CHECK(primary_attempt BETWEEN 0 AND 4294967295),
    primary_sync_id TEXT UNIQUE CHECK(length(primary_sync_id)=26),
    base_oid TEXT CHECK(length(base_oid)=40 AND base_oid NOT GLOB '*[^0-9a-f]*' AND base_oid!='0000000000000000000000000000000000000000'),
    context_oid TEXT CHECK(length(context_oid)=40 AND context_oid NOT GLOB '*[^0-9a-f]*' AND context_oid!='0000000000000000000000000000000000000000'),
    integration_oid TEXT CHECK(length(integration_oid)=40 AND integration_oid NOT GLOB '*[^0-9a-f]*' AND integration_oid!='0000000000000000000000000000000000000000'),
    integration_kind TEXT CHECK(integration_kind IN ('merged','already_contained')),
    remote_context_oid TEXT CHECK(length(remote_context_oid)=40 AND remote_context_oid NOT GLOB '*[^0-9a-f]*' AND remote_context_oid!='0000000000000000000000000000000000000000'),
    publication_category TEXT CHECK(publication_category IN ('published','already_current','no_publication_remote','conflict','rejected','transport','identity','busy','yielding','cancelled','recovery_required')),
    cleanup_category TEXT CHECK(cleanup_category IN ('awaiting_publication','context_changed','ignored_files','remote_context_changed','remote_deletion_transport','remote_deletion_rejected','remote_deletion_configuration','remote_deletion_key','remote_deletion_unlock','remote_deletion_host','recovery_required','cancelled')),
    cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1)),
    updated_at INTEGER NOT NULL CHECK(updated_at>=0),
    refusal_category TEXT CHECK(refusal_category IN ('blocked','external_change','reconfirmation_required')),
    refusal_blockers INTEGER CHECK(refusal_blockers BETWEEN 1 AND 2097151),
    closure_kind TEXT CHECK(closure_kind IN ('record','existing')),
    closed_at TEXT CHECK(closed_at IS NULL OR (typeof(closed_at)='text' AND length(closed_at)=20 AND closed_at GLOB '[0-9][0-9][0-9][0-9]-[0-1][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-6][0-9]Z')),
    closure_oid TEXT CHECK(length(closure_oid)=40 AND closure_oid NOT GLOB '*[^0-9a-f]*' AND closure_oid!='0000000000000000000000000000000000000000'),
    adopted_context_oid TEXT CHECK(length(adopted_context_oid)=40 AND adopted_context_oid NOT GLOB '*[^0-9a-f]*' AND adopted_context_oid!='0000000000000000000000000000000000000000'),
    CHECK((context_attempt=0)=(context_sync_id IS NULL)),
    CHECK((primary_attempt=0)=(primary_sync_id IS NULL)),
    CHECK(context_sync_id IS NULL OR context_sync_id!=operation_id),
    CHECK(primary_sync_id IS NULL OR (primary_sync_id!=operation_id AND primary_sync_id IS NOT context_sync_id)),
    CHECK((step='abandoned')=(refusal_category IS NOT NULL)),
    CHECK((refusal_category IS 'blocked')=(refusal_blockers IS NOT NULL)),
    CHECK(closure_kind IS NOT NULL OR step='abandoned'),
    CHECK((closed_at IS NULL)=(closure_oid IS NULL)),
    CHECK(closed_at IS NULL OR closure_kind IS 'record'),
    CHECK(adopted_context_oid IS NULL OR (closure_oid IS NOT NULL AND adopted_context_oid!=closure_oid AND closure_kind IS 'record')),
    CHECK(step!='closing' OR (closure_kind IS 'record' AND closed_at IS NOT NULL AND base_oid IS NOT NULL AND context_oid IS NOT NULL))
)";
const INDEX: &str = "CREATE UNIQUE INDEX ticket_closures_unfinished ON ticket_closures(repository_id,item_id) WHERE step NOT IN ('complete','abandoned','superseded')";
const MARKER: &str = "cycle_10_ticket_closures";

/// All or nothing inside the registry-schema transaction, marked once.
pub(in super::super) fn migrate(tx: &Transaction<'_>) -> Result<(), RepositoryError> {
    let marked: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if !marked {
        let partial:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name IN ('ticket_closures','ticket_closures_unfinished'))",[],|r|r.get(0)).map_err(|_|state::recovery_required())?;
        if partial {
            return Err(state::recovery_required());
        }
        tx.execute_batch(SCHEMA)
            .and_then(|()| tx.execute_batch(INDEX))
            .map_err(|_| state::recovery_required())?;
        tx.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MARKER])
            .map_err(|_| state::recovery_required())?;
    }
    validate_schema(tx)
}
fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
    state::validate_schema_object(db, "TABLE", "ticket_closures", SCHEMA)?;
    state::validate_schema_object(db, "INDEX", "ticket_closures_unfinished", INDEX)
}
/// `closed_at` exactly as stored: RFC 3339, UTC `Z`, whole seconds.
fn closed_at_text(at: OffsetDateTime) -> Result<String, RepositoryError> {
    let at = at.to_offset(UtcOffset::UTC);
    if at.nanosecond() != 0 {
        return Err(state::recovery_required());
    }
    at.format(&Rfc3339).map_err(|_| state::recovery_required())
}
fn parse_closed_at(text: &str) -> Result<OffsetDateTime, RepositoryError> {
    let at = OffsetDateTime::parse(text, &Rfc3339).map_err(|_| state::recovery_required())?;
    if at.offset() != UtcOffset::UTC || closed_at_text(at)? != text {
        return Err(state::recovery_required());
    }
    Ok(at)
}
fn decode(row: &rusqlite::Row<'_>) -> Result<ClosureRecord, RepositoryError> {
    let row = parent::Row(row);
    let r = ClosureRecord {
        repository_id: row.number("repository_id")?,
        operation_id: row.operation("operation_id")?,
        item_id: row.item()?,
        expectation: ClosureExpectation::from_bytes(row.expectation()?),
        remote_name: row.opt("remote_name")?,
        step: ClosureStep::parse(&row.text("step")?)?,
        context_attempt: row.attempt("context_attempt")?,
        context_sync_id: row.sync_id("context_sync_id")?,
        primary_attempt: row.attempt("primary_attempt")?,
        primary_sync_id: row.sync_id("primary_sync_id")?,
        base_oid: row.oid("base_oid")?,
        context_oid: row.oid("context_oid")?,
        integration_oid: row.oid("integration_oid")?,
        integration_kind: row.category("integration_kind", IntegrationKind::parse)?,
        remote_context_oid: row.oid("remote_context_oid")?,
        publication: row.category("publication_category", PublicationCategory::parse)?,
        cleanup: row.category("cleanup_category", CleanupCategory::parse)?,
        cancel_requested: row.flag("cancel_requested")?,
        updated_at: row.number("updated_at")?,
        refusal: row.refusal()?,
        closure_kind: row.category("closure_kind", ClosureKind::parse)?,
        closed_at: row.category("closed_at", parse_closed_at)?,
        closure_oid: row.oid("closure_oid")?,
        adopted_context_oid: row.oid("adopted_context_oid")?,
    };
    validate(&r)?;
    Ok(r)
}
/// The design's step invariants (Records), plus promotion's own.
fn validate(r: &ClosureRecord) -> Result<(), RepositoryError> {
    use ClosureStep::*;
    let record = r.closure_kind == Some(ClosureKind::Record);
    let synchronized =
        matches!(r.step, ContextSynchronized | Closing) || r.step.integrating_or_later();
    if r.repository_id <= 0
        || OffsetDateTime::from_unix_timestamp(r.updated_at).is_err()
        || r.updated_at < 0
        || (r.context_attempt == 0) != r.context_sync_id.is_none()
        || (r.primary_attempt == 0) != r.primary_sync_id.is_none()
        || r.context_sync_id == Some(r.operation_id)
        || r.primary_sync_id == Some(r.operation_id)
        || r.context_sync_id.is_some() && r.context_sync_id == r.primary_sync_id
        || (r.step == Abandoned) != r.refusal.is_some()
        || [
            r.base_oid,
            r.context_oid,
            r.integration_oid,
            r.remote_context_oid,
            r.closure_oid,
            r.adopted_context_oid,
        ]
        .into_iter()
        .flatten()
        .any(|o| o.is_zero())
        // confirmed: the closure effect is known from the observation.
        || r.closure_kind.is_none() && r.step != Abandoned
        // context_synchronized: the synchronized pre-closure context.
        || synchronized && r.context_oid.is_none()
        // closing: a record closure's checkpoint over a recorded base.
        || r.step == Closing
            && (!record || r.base_oid.is_none() || r.closed_at.is_none() || r.closure_oid.is_none())
        // integrating: the recorded base, for every kind.
        || r.step.integrating_or_later() && (r.base_oid.is_none() || r.integration_kind.is_none())
        // A record closure reaches integration only through its checkpoint.
        || record && r.step.integrating_or_later() && r.closure_oid.is_none()
        // integrated onward: the integration itself.
        || r.step.integrating_or_later() && r.step != Integrating && r.integration_oid.is_none()
        // closed_at and closure_oid are written together, for record only,
        // and never before closing.
        // An adopted context descends from a record closure's checkpoint
        // (ancestry is checked when it is written) and exists from closing on.
        || r.adopted_context_oid.is_some()
            && (r.closure_oid.is_none()
                || r.adopted_context_oid == r.closure_oid
                || !record
                || matches!(r.step, Confirmed | ContextSynchronized))
        || r.closed_at.is_some() != r.closure_oid.is_some()
        || r.closed_at.is_some() && !record
        || r.closed_at.is_some() && matches!(r.step, Confirmed | ContextSynchronized)
        || r.closed_at.is_some_and(|at| at.offset() != UtcOffset::UTC || at.nanosecond() != 0)
        || matches!(
            r.step,
            Published | RemoteDeleted | WorktreeRemoving | WorktreeRemoved | BranchDeleted | Complete
        ) && r.remote_name.is_some()
            && !matches!(
                r.publication,
                Some(PublicationCategory::Published | PublicationCategory::AlreadyCurrent)
            )
        || r.step == PublicationPending
            && (r.publication.is_none()
                || matches!(
                    r.publication,
                    Some(PublicationCategory::Published | PublicationCategory::AlreadyCurrent)
                ))
        || r.remote_name.is_none() && (r.remote_context_oid.is_some() || r.step == RemoteDeleted)
    {
        return Err(state::recovery_required());
    }
    if let Some(name) = &r.remote_name {
        crate::repository::RemoteRefPlan::from_configuration(name, "main")
            .map_err(|_| state::recovery_required())?;
    }
    if let Some(refusal) = &r.refusal {
        refusal.encoded()?;
    }
    Ok(())
}

pub(in super::super) fn read_record(
    db: &Connection,
    id: i64,
    op: OperationId,
) -> Result<Option<ClosureRecord>, RepositoryError> {
    parent::read_record(db, id, op)
}
pub(in super::super) fn records(
    db: &Connection,
    id: i64,
) -> Result<Vec<ClosureRecord>, RepositoryError> {
    parent::records(db, id)
}
/// Only a cleanup stop after integration may be superseded. A written
/// closure checkpoint (`closing`, `integrating`) never is: only its own ID
/// finishes it.
fn supersedable(old: &ClosureRecord) -> bool {
    !matches!(old.step, ClosureStep::Closing | ClosureStep::Integrating)
        && old.cleanup.is_some_and(CleanupCategory::supersedable)
}
pub(in super::super) type ReplacementGuard = parent::ReplacementGuard<ClosureRecord>;
pub(in super::super) fn replacement_guard(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<ReplacementGuard, RepositoryError> {
    parent::replacement_guard(tx, id, item)
}
/// Caller has validated new consent before this atomic supersession/insert.
/// `kind` is the observed closure effect, fixed for the record's life.
pub(in super::super) fn begin_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: ClosureExpectation,
    remote: Option<&str>,
    kind: ClosureKind,
) -> Result<ClosureRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if let Some(old) = parent::unfinished_record::<ClosureRecord>(tx, id, item)? {
        if !supersedable(&old) {
            return Err(parent::busy(KIND));
        }
        parent::supersede(tx, id, &old)?;
    }
    insert_record(tx, id, op, item, expectation, remote, Some(kind), None)
}
/// A refused new request is terminal from insertion. It can coexist with the
/// original unfinished parent without superseding/fencing that parent's owner.
/// `kind` is absent when the refusal came before any closure effect was seen.
#[allow(clippy::too_many_arguments)] // Promotion's refusal shape plus the closure effect.
pub(in super::super) fn refuse_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: ClosureExpectation,
    remote: Option<&str>,
    kind: Option<ClosureKind>,
    refusal: &Refusal,
) -> Result<ClosureRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if parent::unfinished_record::<ClosureRecord>(tx, id, item)?
        .is_some_and(|old| !supersedable(&old))
    {
        return Err(parent::busy(KIND));
    }
    insert_record(tx, id, op, item, expectation, remote, kind, Some(refusal))
}
#[allow(clippy::too_many_arguments)] // The two insert shapes above, in one place.
fn insert_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: ClosureExpectation,
    remote: Option<&str>,
    kind: Option<ClosureKind>,
    refusal: Option<&Refusal>,
) -> Result<ClosureRecord, RepositoryError> {
    if let Some(remote) = remote {
        crate::repository::RemoteRefPlan::from_configuration(remote, "main")
            .map_err(|_| state::recovery_required())?;
    }
    let (category, blockers) = Refusal::encoded_columns(refusal)?;
    let step = if refusal.is_some() {
        "abandoned"
    } else {
        "confirmed"
    };
    tx.execute("INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,refusal_category,refusal_blockers,closure_kind) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,op.to_string(),item.to_string(),expectation.to_bytes().as_slice(),remote,step,parent::now(),category,blockers,kind.map(ClosureKind::name)]).map_err(|_|state::recovery_required())?;
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)
}
/// Saves every runner-owned field. The closure effect (`closure_kind`,
/// `closed_at`, `closure_oid`) changes only through `begin_closing` and
/// `replace_unmoved_closure`.
pub(in super::super) fn save_record(
    tx: &Transaction<'_>,
    id: i64,
    r: &ClosureRecord,
) -> Result<(), RepositoryError> {
    let old = read_record(tx, id, r.operation_id)?.ok_or_else(state::recovery_required)?;
    if r.step != old.step
        && super::super::local_sync::record::unfinished_context(tx, id, &r.item_id)?
    {
        return Err(state::recovery_required());
    }
    if old.repository_id != r.repository_id
        || old.item_id != r.item_id
        || old.expectation != r.expectation
        || old.remote_name != r.remote_name
        || old.context_attempt != r.context_attempt
        || old.context_sync_id != r.context_sync_id
        || old.primary_attempt != r.primary_attempt
        || old.primary_sync_id != r.primary_sync_id
        || old.closure_kind != r.closure_kind
    {
        return Err(parent::mismatch(KIND));
    }
    if !old
        .step
        .permits(r.step, r.remote_name.is_some(), old.closure_kind)
        || r.cancel_requested != old.cancel_requested
        || old.closed_at != r.closed_at
        || old.closure_oid != r.closure_oid
        || old.adopted_context_oid != r.adopted_context_oid
        || old.base_oid.is_some() && old.base_oid != r.base_oid
        || old.context_oid.is_some() && old.context_oid != r.context_oid
        || old.integration_oid.is_some() && old.integration_oid != r.integration_oid
        || old.integration_kind.is_some() && old.integration_kind != r.integration_kind
        || old.remote_context_oid.is_some() && old.remote_context_oid != r.remote_context_oid
        // R-T5-3: the tracking OID is observed with the synchronization only.
        || old.remote_context_oid.is_none()
            && r.remote_context_oid.is_some()
            && (old.step, r.step) != (ClosureStep::Confirmed, ClosureStep::ContextSynchronized)
    {
        return Err(state::recovery_required());
    }
    validate(r)?;
    let (refusal, mask) = Refusal::encoded_columns(r.refusal.as_ref())?;
    tx.execute("UPDATE ticket_closures SET step=?2,base_oid=?3,context_oid=?4,integration_oid=?5,integration_kind=?6,remote_context_oid=?7,publication_category=?8,cleanup_category=?9,refusal_category=?10,refusal_blockers=?11,updated_at=max(updated_at,?12) WHERE operation_id=?1",params![r.operation_id.to_string(),r.step.name(),r.base_oid.map(|o|o.to_string()),r.context_oid.map(|o|o.to_string()),r.integration_oid.map(|o|o.to_string()),r.integration_kind.map(IntegrationKind::name),r.remote_context_oid.map(|o|o.to_string()),r.publication.map(PublicationCategory::name),r.cleanup.map(CleanupCategory::name),refusal,mask,parent::now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}
/// `context_synchronized → closing` for a `record` closure, in one SQL write
/// with its base, `closed_at` (whole UTC seconds) and the checkpoint commit
/// just built and not yet referenced.
pub(in super::super) fn begin_closing(
    tx: &Transaction<'_>,
    repository_id: i64,
    operation: OperationId,
    base: Oid,
    closed_at: OffsetDateTime,
    closure: Oid,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, repository_id, operation)?.ok_or_else(state::recovery_required)?;
    if r.step != ClosureStep::ContextSynchronized
        || r.closure_kind != Some(ClosureKind::Record)
        || base.is_zero()
        || closure.is_zero()
        || r.base_oid.is_some_and(|old| old != base)
    {
        return Err(state::recovery_required());
    }
    let closed_at = closed_at_text(closed_at)?;
    tx.execute("UPDATE ticket_closures SET step='closing',base_oid=?2,closed_at=?3,closure_oid=?4,updated_at=max(updated_at,?5) WHERE operation_id=?1",params![operation.to_string(),base.to_string(),closed_at,closure.to_string(),parent::now()]).map_err(|_|state::recovery_required())?;
    read_record(tx, repository_id, operation)?.ok_or_else(state::recovery_required)?;
    Ok(())
}
/// Rebuilds the checkpoint's record while `closing`. The caller has proved
/// the context ref has not moved to the stored commit; `closed_at` is kept.
pub(in super::super) fn replace_unmoved_closure(
    tx: &Transaction<'_>,
    repository_id: i64,
    operation: OperationId,
    closure: Oid,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, repository_id, operation)?.ok_or_else(state::recovery_required)?;
    if r.step != ClosureStep::Closing || closure.is_zero() {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE ticket_closures SET closure_oid=?2,updated_at=max(updated_at,?3) WHERE operation_id=?1",params![operation.to_string(),closure.to_string(),parent::now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

/// The C9 resume of a written checkpoint. While `closing`: the base the
/// merge uses now (the current tracking or local primary) and, when the
/// context tip is a deliberate synchronization over the checkpoint, that tip
/// as the adopted context (`None`: the checkpoint itself). It must descend
/// from `closure_oid`, checked here in Git. `closure_oid` and `closed_at`
/// never change. From `integrating` both are fixed: only the recorded values
/// are accepted, and nothing is written.
pub(in super::super) fn adopt_written_closure(
    tx: &Transaction<'_>,
    repository_id: i64,
    operation: OperationId,
    repository: &git2::Repository,
    base: Oid,
    adopted: Option<Oid>,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, repository_id, operation)?.ok_or_else(state::recovery_required)?;
    let Some(closure) = r.closure_oid else {
        return Err(state::recovery_required());
    };
    if super::super::local_sync::record::unfinished_context(tx, repository_id, &r.item_id)? {
        return Err(state::recovery_required());
    }
    if base.is_zero() || r.closure_kind != Some(ClosureKind::Record) {
        return Err(state::recovery_required());
    }
    match r.step {
        ClosureStep::Closing => {}
        ClosureStep::Integrating
            if r.base_oid == Some(base) && r.adopted_context_oid == adopted =>
        {
            return Ok(());
        }
        _ => return Err(state::recovery_required()),
    }
    if let Some(adopted) = adopted
        && (adopted.is_zero()
            || adopted == closure
            || !repository
                .graph_descendant_of(adopted, closure)
                .unwrap_or(false))
    {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE ticket_closures SET base_oid=?2,adopted_context_oid=?3,updated_at=max(updated_at,?4) WHERE operation_id=?1",params![operation.to_string(),base.to_string(),adopted.map(|o|o.to_string()),parent::now()]).map_err(|_|state::recovery_required())?;
    read_record(tx, repository_id, operation)?.ok_or_else(state::recovery_required)?;
    Ok(())
}

pub(in super::super) fn child_execution(
    r: &ClosureRecord,
    role: ChildRole,
) -> Result<ChildExecution, RepositoryError> {
    parent::child_execution(r, role)
}
pub(in super::super) fn child_state(
    db: &Connection,
    r: &ClosureRecord,
    role: ChildRole,
) -> Result<ChildState, RepositoryError> {
    parent::child_state(db, r, role)
}
pub(in super::super) fn next_attempt(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    role: ChildRole,
) -> Result<ClosureRecord, RepositoryError> {
    parent::next_attempt(tx, id, op, role)
}
/// The sole back-transition: a fresh observation proved primary advancement
/// before `closing` (design step 3). Nothing has been written; a completed
/// context child may be replaced here.
pub(in super::super) fn reset_context_after_primary_advance(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<ClosureRecord, RepositoryError> {
    parent::reset_context_after_primary_advance(tx, id, op, |r: &ClosureRecord| {
        r.step == ClosureStep::ContextSynchronized
    })
}
/// Checks closure parent and child bindings only; callers reach both parent
/// tables through `promotion::record::validate_remote_identity`.
pub(in super::super) fn validate_remote_identity(
    db: &Connection,
    id: i64,
    op: OperationId,
    target: &RemoteOperationTarget,
    priority: RemoteOperationPriority,
) -> Result<(), RepositoryError> {
    if !crate::repository::recovery::has_table(db, "ticket_closures")
        .map_err(|_| state::recovery_required())?
    {
        return Ok(());
    }
    parent::remote_identity_of::<ClosureRecord>(db, id, op, target, priority)
}
/// The local journal may never begin under a closure parent ID, and under a
/// child ID only as that child's exact authoritative refresh hand-off.
pub(in super::super) fn validate_local_identity(
    db: &Connection,
    root: &str,
    action: &str,
    op: &str,
    target: &str,
) -> Result<(), RepositoryError> {
    parent::validate_local_identity(KIND, db, root, action, op, target)
}
pub(in super::super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    parent::audit::<ClosureRecord>(db)
}
/// Completion retires the merged context: the checkpoint when written,
/// else the synchronized context.
pub(in super::super) fn complete_record(
    tx: &Transaction<'_>,
    service: &RepositoryService,
    id: i64,
    op: OperationId,
    evidence: &RetirementEvidence<'_>,
    owner: Option<&RemoteReservation>,
) -> Result<(), RepositoryError> {
    parent::complete_record::<ClosureRecord>(tx, service, id, op, evidence, owner)
}
/// Inspect actual committed parent state after an uncertain commit result.
pub(in super::super) fn reconcile_completion(
    db: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    parent::reconcile_completion::<ClosureRecord>(db, id, op)
}
/// The design's cleanup row: with a remote, the remote context branch is
/// deleted unless no context child ran (a contained closure) and no
/// tracking context was observed. A contained `record` closure integrates
/// as a merge, so promotion's `already_contained` test cannot decide it.
fn deletion_reservation_required(r: &ClosureRecord) -> bool {
    r.remote_name.is_some() && !(r.context_attempt == 0 && r.remote_context_oid.is_none())
}
/// C1, C9: the remote context branch never receives a closure checkpoint.
/// From `closing` until retirement the ticket context may hold one, so a
/// synchronization of that context, which would push it, is refused.
pub(in super::super) fn written_closure_unfinished(
    db: &Connection,
    id: i64,
    item: &ItemId,
) -> Result<bool, RepositoryError> {
    if !crate::repository::recovery::has_table(db, "ticket_closures")
        .map_err(|_| state::recovery_required())?
    {
        return Ok(false);
    }
    db.query_row("SELECT EXISTS(SELECT 1 FROM ticket_closures WHERE repository_id=?1 AND item_id=?2 AND step IN ('closing','integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted'))",params![id,item.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())
}
/// The other order of the same rule: an unfinished synchronization of the
/// ticket context, reserved before the checkpoint, holds the checkpoint off.
pub(in super::super) fn context_synchronization_unfinished(
    db: &Connection,
    id: i64,
    item: &ItemId,
) -> Result<bool, RepositoryError> {
    if super::super::local_sync::record::unfinished_context(db, id, item)? {
        return Ok(true);
    }
    db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND action='synchronize_context' AND kind='ticket' AND item_id=?2 AND phase NOT IN ('completed','interrupted','cancelled','failed'))",params![id,item.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())
}
pub(in super::super) fn request_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    parent::request_cancellation::<ClosureRecord>(tx, id, op)
}
pub(in super::super) fn clear_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    parent::clear_cancellation::<ClosureRecord>(tx, id, op)
}

impl ParentRecord for ClosureRecord {
    const KIND: LifecycleKind = KIND;
    fn decode(row: &rusqlite::Row<'_>) -> Result<Self, RepositoryError> {
        decode(row)
    }
    fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
        validate_schema(db)
    }
    fn context_attempt(&self) -> u32 {
        self.context_attempt
    }
    fn primary_attempt(&self) -> u32 {
        self.primary_attempt
    }
    fn expectation_bytes(&self) -> [u8; 32] {
        self.expectation.to_bytes()
    }
}

impl LifecycleRecord for ClosureRecord {
    type Expectation = ClosureExpectation;
    type Refusal = Refusal;
    fn repository_id(&self) -> i64 {
        self.repository_id
    }
    fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    fn item_id(&self) -> &ItemId {
        &self.item_id
    }
    fn expectation(&self) -> ClosureExpectation {
        self.expectation
    }
    fn remote_name(&self) -> Option<&str> {
        self.remote_name.as_deref()
    }
    fn step(&self) -> ParentStep {
        self.step.into()
    }
    /// `closing` is what the runner sees as `context_synchronized`; writing
    /// that view back keeps `closing`. Any other step is taken as given, and
    /// `save_record` refuses all but `integrating` and `abandoned` from it.
    fn set_step(&mut self, step: ParentStep) {
        if self.step == ClosureStep::Closing && step == ParentStep::ContextSynchronized {
            return;
        }
        self.step = step.into();
    }
    fn context_sync_id(&self) -> Option<OperationId> {
        self.context_sync_id
    }
    fn primary_sync_id(&self) -> Option<OperationId> {
        self.primary_sync_id
    }
    fn base_oid(&self) -> Option<Oid> {
        self.base_oid
    }
    fn set_base_oid(&mut self, oid: Option<Oid>) {
        self.base_oid = oid;
    }
    fn context_oid(&self) -> Option<Oid> {
        self.context_oid
    }
    fn set_context_oid(&mut self, oid: Option<Oid>) {
        self.context_oid = oid;
    }
    fn merged_context(&self) -> Option<Oid> {
        ClosureRecord::merged_context(self)
    }
    /// A checkpoint is recorded from `closing` on: whether its ref moved is
    /// decided under the lease by the closure's `resume_written`.
    fn effect_written(&self) -> bool {
        self.closure_oid.is_some()
    }
    fn integration_oid(&self) -> Option<Oid> {
        self.integration_oid
    }
    fn set_integration_oid(&mut self, oid: Option<Oid>) {
        self.integration_oid = oid;
    }
    fn integration_kind(&self) -> Option<IntegrationKind> {
        self.integration_kind
    }
    fn set_integration_kind(&mut self, kind: Option<IntegrationKind>) {
        self.integration_kind = kind;
    }
    fn remote_context_oid(&self) -> Option<Oid> {
        self.remote_context_oid
    }
    fn set_remote_context_oid(&mut self, oid: Option<Oid>) {
        self.remote_context_oid = oid;
    }
    /// R-T3-3: a contained closure runs no context child, so the delete is
    /// guarded by the pre-checkpoint tracking OID its observation recorded,
    /// never by an absent publication. A child that ran guards it with its
    /// verified publication (T7-1). R-T5-2: "no child ran" is no context
    /// attempt, the proxy the required-delete rule uses; a minted attempt
    /// that never reserved has no guard, and cleanup stops.
    fn remote_delete_guard(&self, published: Option<Oid>) -> Option<Oid> {
        if self.context_attempt == 0 {
            published.or(self.remote_context_oid)
        } else {
            published
        }
    }
    fn remote_deletion_required(&self) -> bool {
        deletion_reservation_required(self)
    }
    fn publication(&self) -> Option<PublicationCategory> {
        self.publication
    }
    fn set_publication(&mut self, category: Option<PublicationCategory>) {
        self.publication = category;
    }
    fn cleanup(&self) -> Option<CleanupCategory> {
        self.cleanup
    }
    fn set_cleanup(&mut self, category: Option<CleanupCategory>) {
        self.cleanup = category;
    }
    fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }
    fn set_cancel_requested(&mut self, requested: bool) {
        self.cancel_requested = requested;
    }
    fn refusal(&self) -> Option<&Refusal> {
        self.refusal.as_ref()
    }
    fn set_refusal(&mut self, refusal: Option<Refusal>) {
        self.refusal = refusal;
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
