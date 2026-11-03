//! Parent storage only: no transport, Git effect, prompt or lease acquisition.
//! The table-agnostic parent logic is `lifecycle_record`'s, shared with the
//! ticket closure table; this module keeps the promotion schema, step graph,
//! invariants, insert and save.
#![allow(dead_code)] // The approved Task 6 runner composes these transaction-scoped seams.
use super::lifecycle::LifecycleKind;
use super::lifecycle_record::{self as parent, Blocker, ParentRecord, categories};
use super::{PromotionBlocker, PromotionExpectation};
use crate::{
    canonical::ItemId,
    repository::{
        OperationId, RepositoryError, RepositoryService,
        cleanup::RetirementEvidence,
        remote::{reservation::RemoteReservation, state},
    },
};
use git2::Oid;
use rusqlite::{Connection, Transaction, params};

pub(in super::super) use super::lifecycle_record::{
    ChildExecution, ChildRole, ChildState, validate_remote_identity,
};

const KIND: LifecycleKind = LifecycleKind::DocumentPromotion;
categories!(PromotionStep {
    Confirmed=>"confirmed", ContextSynchronized=>"context_synchronized", Integrating=>"integrating",
    Integrated=>"integrated", Published=>"published", PublicationPending=>"publication_pending",
    RemoteDeleted=>"remote_deleted", WorktreeRemoving=>"worktree_removing", WorktreeRemoved=>"worktree_removed",
    BranchDeleted=>"branch_deleted", Complete=>"complete", Abandoned=>"abandoned", Superseded=>"superseded"
});
impl PromotionStep {
    pub(in super::super) fn terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Abandoned | Self::Superseded)
    }
    fn permits(self, next: Self, remote: bool) -> bool {
        use PromotionStep::*;
        if self.terminal() {
            return false;
        }
        self == next
            || matches!(
                (self, next),
                (Confirmed, ContextSynchronized)
                    | (ContextSynchronized, Integrating)
                    | (Integrating, Integrated)
                    | (Integrated, Published | PublicationPending)
                    | (PublicationPending, Published)
                    | (Published, RemoteDeleted)
                    | (RemoteDeleted, WorktreeRemoving)
                    | (WorktreeRemoving, WorktreeRemoved)
                    | (WorktreeRemoved, BranchDeleted)
                    | (Confirmed | ContextSynchronized, Abandoned)
            )
            || (!remote && matches!((self, next), (PublicationPending, WorktreeRemoving)))
    }
}
categories!(IntegrationKind { Merged=>"merged", AlreadyContained=>"already_contained" });
categories!(PublicationCategory {
    Published=>"published", AlreadyCurrent=>"already_current", NoPublicationRemote=>"no_publication_remote",
    Conflict=>"conflict", Rejected=>"rejected", Transport=>"transport", Identity=>"identity", Busy=>"busy", Yielding=>"yielding", Cancelled=>"cancelled", RecoveryRequired=>"recovery_required"
});
categories!(CleanupCategory {
    AwaitingPublication=>"awaiting_publication", ContextChanged=>"context_changed", IgnoredFiles=>"ignored_files", RemoteContextChanged=>"remote_context_changed",
    RemoteDeletionTransport=>"remote_deletion_transport", RemoteDeletionRejected=>"remote_deletion_rejected",
    RemoteDeletionConfiguration=>"remote_deletion_configuration", RemoteDeletionKey=>"remote_deletion_key", RemoteDeletionUnlock=>"remote_deletion_unlock", RemoteDeletionHost=>"remote_deletion_host",
    RecoveryRequired=>"recovery_required", Cancelled=>"cancelled"
});
impl CleanupCategory {
    pub(in super::super) fn supersedable(self) -> bool {
        matches!(
            self,
            Self::ContextChanged | Self::IgnoredFiles | Self::RemoteContextChanged
        )
    }
}

pub(in super::super) type Refusal = parent::Refusal<PromotionBlocker>;
const BLOCKERS: [PromotionBlocker; 16] = [
    PromotionBlocker::NotADocument,
    PromotionBlocker::ContextMissing,
    PromotionBlocker::ContextNonconforming,
    PromotionBlocker::ContextDirty,
    PromotionBlocker::ContextConflicted,
    PromotionBlocker::ConflictPending,
    PromotionBlocker::PrimaryMissing,
    PromotionBlocker::PrimaryNotCheckedOut,
    PromotionBlocker::PrimaryDirty,
    PromotionBlocker::PrimaryConflicted,
    PromotionBlocker::PrimaryAheadOfRemote,
    PromotionBlocker::ContextBehindRemote,
    PromotionBlocker::IntegrationConflict,
    PromotionBlocker::RemoteContextDeleted,
    PromotionBlocker::HistoryUnknown,
    PromotionBlocker::DocumentInvalidAfterIntegration,
];
impl Blocker for PromotionBlocker {
    const ORDER: &'static [Self] = &BLOCKERS;
    const INTEGRATION_CONFLICT: Self = PromotionBlocker::IntegrationConflict;
}
#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct PromotionRecord {
    pub repository_id: i64,
    pub operation_id: OperationId,
    pub item_id: ItemId,
    pub expectation: PromotionExpectation,
    pub remote_name: Option<String>,
    pub step: PromotionStep,
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
}
impl std::fmt::Debug for PromotionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromotionRecord")
            .field("operation_id", &self.operation_id)
            .field("item_id", &self.item_id)
            .field("step", &self.step)
            .field("context_attempt", &self.context_attempt)
            .field("primary_attempt", &self.primary_attempt)
            .field("integration_oid", &self.integration_oid)
            .field("publication", &self.publication)
            .field("cleanup", &self.cleanup)
            .field("refusal", &self.refusal)
            .finish()
    }
}
const SCHEMA:&str="CREATE TABLE document_promotions (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    operation_id TEXT NOT NULL UNIQUE CHECK(length(operation_id)=26),
    item_id TEXT NOT NULL CHECK(length(item_id)=26),
    expectation BLOB NOT NULL CHECK(typeof(expectation)='blob' AND length(expectation)=32),
    remote_name TEXT,
    step TEXT NOT NULL CHECK(step IN ('confirmed','context_synchronized','integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete','abandoned','superseded')),
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
    refusal_blockers INTEGER CHECK(refusal_blockers BETWEEN 1 AND 65535),
    CHECK((context_attempt=0)=(context_sync_id IS NULL)),
    CHECK((primary_attempt=0)=(primary_sync_id IS NULL)),
    CHECK(context_sync_id IS NULL OR context_sync_id!=operation_id),
    CHECK(primary_sync_id IS NULL OR (primary_sync_id!=operation_id AND primary_sync_id IS NOT context_sync_id)),
    CHECK((step='abandoned')=(refusal_category IS NOT NULL)),
    CHECK((refusal_category IS 'blocked')=(refusal_blockers IS NOT NULL))
)";
const INDEX: &str = "CREATE UNIQUE INDEX document_promotions_unfinished ON document_promotions(repository_id,item_id) WHERE step NOT IN ('complete','abandoned','superseded')";
pub(in super::super) fn migrate(tx: &Transaction<'_>) -> Result<(), RepositoryError> {
    let marked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name='cycle_09_document_promotions')",[],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if !marked {
        let partial:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name IN ('document_promotions','document_promotions_unfinished'))",[],|r|r.get(0)).map_err(|_|state::recovery_required())?;
        if partial {
            return Err(state::recovery_required());
        }
        tx.execute_batch(SCHEMA)
            .and_then(|()| tx.execute_batch(INDEX))
            .map_err(|_| state::recovery_required())?;
        tx.execute(
            "INSERT INTO registry_migrations(name) VALUES('cycle_09_document_promotions')",
            [],
        )
        .map_err(|_| state::recovery_required())?;
    }
    validate_schema(tx)
}
fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
    state::validate_schema_object(db, "TABLE", "document_promotions", SCHEMA)?;
    state::validate_schema_object(db, "INDEX", "document_promotions_unfinished", INDEX)
}
fn decode(row: &rusqlite::Row<'_>) -> Result<PromotionRecord, RepositoryError> {
    let row = parent::Row(row);
    let r = PromotionRecord {
        repository_id: row.number("repository_id")?,
        operation_id: row.operation("operation_id")?,
        item_id: row.item()?,
        expectation: PromotionExpectation::from_bytes(row.expectation()?),
        remote_name: row.opt("remote_name")?,
        step: PromotionStep::parse(&row.text("step")?)?,
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
    };
    validate(&r)?;
    Ok(r)
}
fn validate(r: &PromotionRecord) -> Result<(), RepositoryError> {
    use PromotionStep::*;
    if r.repository_id <= 0
        || time::OffsetDateTime::from_unix_timestamp(r.updated_at).is_err()
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
        ]
        .into_iter()
        .flatten()
        .any(|o| o.is_zero())
        || matches!(
            r.step,
            ContextSynchronized
                | Integrating
                | Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && r.context_oid.is_none()
        || matches!(
            r.step,
            Integrating
                | Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && (r.base_oid.is_none() || r.integration_kind.is_none())
        || matches!(
            r.step,
            Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && r.integration_oid.is_none()
        || matches!(
            r.step,
            Published
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
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
) -> Result<Option<PromotionRecord>, RepositoryError> {
    parent::read_record(db, id, op)
}
pub(in super::super) fn records(
    db: &Connection,
    id: i64,
) -> Result<Vec<PromotionRecord>, RepositoryError> {
    parent::records(db, id)
}
pub(in super::super) type ReplacementGuard = parent::ReplacementGuard<PromotionRecord>;
pub(in super::super) fn replacement_guard(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<ReplacementGuard, RepositoryError> {
    parent::replacement_guard(tx, id, item)
}
/// Caller has validated new consent before this atomic supersession/insert.
pub(in super::super) fn begin_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
) -> Result<PromotionRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if let Some(old) = parent::unfinished_record::<PromotionRecord>(tx, id, item)? {
        if !old.cleanup.is_some_and(CleanupCategory::supersedable) {
            return Err(parent::busy(KIND));
        }
        parent::supersede(tx, id, &old)?;
    }
    insert_record(tx, id, op, item, expectation, remote, None)
}
/// A refused new request is terminal from insertion. It can coexist with the
/// original unfinished parent without superseding/fencing that parent's owner.
pub(in super::super) fn refuse_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
    refusal: &Refusal,
) -> Result<PromotionRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if parent::unfinished_record::<PromotionRecord>(tx, id, item)?
        .is_some_and(|old| !old.cleanup.is_some_and(CleanupCategory::supersedable))
    {
        return Err(parent::busy(KIND));
    }
    insert_record(tx, id, op, item, expectation, remote, Some(refusal))
}
fn insert_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
    refusal: Option<&Refusal>,
) -> Result<PromotionRecord, RepositoryError> {
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
    tx.execute("INSERT INTO document_promotions(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,refusal_category,refusal_blockers) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,op.to_string(),item.to_string(),expectation.to_bytes().as_slice(),remote,step,parent::now(),category,blockers]).map_err(|_|state::recovery_required())?;
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)
}
pub(in super::super) fn save_record(
    tx: &Transaction<'_>,
    id: i64,
    r: &PromotionRecord,
) -> Result<(), RepositoryError> {
    let old = read_record(tx, id, r.operation_id)?.ok_or_else(state::recovery_required)?;
    if old.repository_id != r.repository_id
        || old.item_id != r.item_id
        || old.expectation != r.expectation
        || old.remote_name != r.remote_name
        || old.context_attempt != r.context_attempt
        || old.context_sync_id != r.context_sync_id
        || old.primary_attempt != r.primary_attempt
        || old.primary_sync_id != r.primary_sync_id
    {
        return Err(parent::mismatch(KIND));
    }
    if !old.step.permits(r.step, r.remote_name.is_some())
        || r.cancel_requested != old.cancel_requested
        || old.base_oid.is_some() && old.base_oid != r.base_oid
        || old.context_oid.is_some() && old.context_oid != r.context_oid
        || old.integration_oid.is_some() && old.integration_oid != r.integration_oid
        || old.integration_kind.is_some() && old.integration_kind != r.integration_kind
        || old.remote_context_oid.is_some() && old.remote_context_oid != r.remote_context_oid
    {
        return Err(state::recovery_required());
    }
    validate(r)?;
    let (refusal, mask) = Refusal::encoded_columns(r.refusal.as_ref())?;
    tx.execute("UPDATE document_promotions SET step=?2,base_oid=?3,context_oid=?4,integration_oid=?5,integration_kind=?6,remote_context_oid=?7,publication_category=?8,cleanup_category=?9,refusal_category=?10,refusal_blockers=?11,updated_at=max(updated_at,?12) WHERE operation_id=?1",params![r.operation_id.to_string(),r.step.name(),r.base_oid.map(|o|o.to_string()),r.context_oid.map(|o|o.to_string()),r.integration_oid.map(|o|o.to_string()),r.integration_kind.map(IntegrationKind::name),r.remote_context_oid.map(|o|o.to_string()),r.publication.map(PublicationCategory::name),r.cleanup.map(CleanupCategory::name),refusal,mask,parent::now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

impl ParentRecord for PromotionRecord {
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
pub(in super::super) fn child_execution(
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<ChildExecution, RepositoryError> {
    parent::child_execution(r, role)
}
pub(in super::super) fn child_state(
    db: &Connection,
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<ChildState, RepositoryError> {
    parent::child_state(db, r, role)
}
pub(in super::super) fn next_attempt(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    role: ChildRole,
) -> Result<PromotionRecord, RepositoryError> {
    parent::next_attempt(tx, id, op, role)
}
/// The sole back-transition: a fresh integration preparation proved primary
/// advancement before `integrating` intent. A completed context child may be
/// replaced here; ordinary retries may not replace authoritative completion.
pub(in super::super) fn reset_context_after_primary_advance(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<PromotionRecord, RepositoryError> {
    parent::reset_context_after_primary_advance(tx, id, op, |r: &PromotionRecord| {
        r.step == PromotionStep::ContextSynchronized
    })
}
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
    parent::audit::<PromotionRecord>(db)
}
pub(in super::super) fn complete_record(
    tx: &Transaction<'_>,
    service: &RepositoryService,
    id: i64,
    op: OperationId,
    evidence: &RetirementEvidence<'_>,
    owner: Option<&RemoteReservation>,
) -> Result<(), RepositoryError> {
    parent::complete_record::<PromotionRecord>(tx, service, id, op, evidence, owner)
}
/// Inspect actual committed parent state after an uncertain commit result.
pub(in super::super) fn reconcile_completion(
    db: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    parent::reconcile_completion::<PromotionRecord>(db, id, op)
}
pub(in super::super) fn request_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    parent::request_cancellation::<PromotionRecord>(tx, id, op)
}
pub(in super::super) fn clear_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    parent::clear_cancellation::<PromotionRecord>(tx, id, op)
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
