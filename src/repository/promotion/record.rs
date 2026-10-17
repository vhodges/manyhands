//! Parent storage only: no transport, Git effect, prompt or lease acquisition.
#![allow(dead_code)] // The approved Task 6 runner composes these transaction-scoped seams.
use super::{PromotionBlocker, PromotionExpectation};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationId, RemoteOperationAction, RemoteOperationPhase,
        RemoteOperationPriority, RemoteOperationTarget, RepositoryError, RepositoryErrorKind,
        RepositoryOperation, RepositoryService,
        cleanup::{self, RetirementEvidence},
        remote::{
            reservation::{self, RemoteReservation},
            state,
        },
    },
};
use git2::Oid;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

macro_rules! categories {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Clone,Copy,Debug,PartialEq,Eq)]
        pub(in super::super) enum $name { $($variant),+ }
        impl $name {
            pub(in super::super) fn name(self)-> &'static str { match self { $(Self::$variant=>$text),+ } }
            fn parse(text: &str)->Result<Self,RepositoryError> { match text { $($text=>Ok(Self::$variant),)+ _=>Err(state::recovery_required()) } }
        }
    }
}
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
    fn supersedable(self) -> bool {
        matches!(
            self,
            Self::ContextChanged | Self::IgnoredFiles | Self::RemoteContextChanged
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum Refusal {
    Blocked(Vec<PromotionBlocker>),
    ExternalChange,
    ReconfirmationRequired,
}
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
impl Refusal {
    fn encoded(&self) -> Result<(&'static str, Option<u32>), RepositoryError> {
        Ok(match self {
            Self::ExternalChange => ("external_change", None),
            Self::ReconfirmationRequired => ("reconfirmation_required", None),
            Self::Blocked(blockers) => {
                if blockers.is_empty() || !blockers.windows(2).all(|p| p[0] < p[1]) {
                    return Err(state::recovery_required());
                }
                (
                    "blocked",
                    Some(blockers.iter().fold(0, |mask, b| {
                        mask | 1
                            << BLOCKERS
                                .iter()
                                .position(|v| v == b)
                                .expect("exhaustive blockers")
                    })),
                )
            }
        })
    }
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
fn mismatch() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        "the promotion ID is bound to another operation",
    )
}
fn busy() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::RepositoryBusy,
        "another promotion is unfinished",
    )
}
fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp().max(0)
}
fn operation(text: String) -> Result<OperationId, RepositoryError> {
    let id = OperationId::parse(&text).map_err(|_| state::recovery_required())?;
    if id.to_string() != text {
        return Err(state::recovery_required());
    }
    Ok(id)
}
fn decode(row: &rusqlite::Row<'_>) -> Result<PromotionRecord, RepositoryError> {
    let text = |n| {
        row.get::<_, String>(n)
            .map_err(|_| state::recovery_required())
    };
    let opt = |n| {
        row.get::<_, Option<String>>(n)
            .map_err(|_| state::recovery_required())
    };
    let number = |n| row.get::<_, i64>(n).map_err(|_| state::recovery_required());
    let oid = |n| -> Result<Option<Oid>, RepositoryError> {
        opt(n)?
            .map(|s| {
                let o = Oid::from_str(&s).map_err(|_| state::recovery_required())?;
                if o.is_zero() || o.to_string() != s {
                    return Err(state::recovery_required());
                }
                Ok(o)
            })
            .transpose()
    };
    let item = text("item_id")?;
    let item_id: ItemId = item.parse().map_err(|_| state::recovery_required())?;
    if item_id.to_string() != item {
        return Err(state::recovery_required());
    }
    let mask: Option<u32> = row
        .get("refusal_blockers")
        .map_err(|_| state::recovery_required())?;
    let refusal = match (opt("refusal_category")?.as_deref(), mask) {
        (None, None) => None,
        (Some("external_change"), None) => Some(Refusal::ExternalChange),
        (Some("reconfirmation_required"), None) => Some(Refusal::ReconfirmationRequired),
        (Some("blocked"), Some(mask)) if mask > 0 && mask <= 65535 => Some(Refusal::Blocked(
            BLOCKERS
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, b)| *b)
                .collect(),
        )),
        _ => return Err(state::recovery_required()),
    };
    let bytes: Vec<u8> = row
        .get("expectation")
        .map_err(|_| state::recovery_required())?;
    let r = PromotionRecord {
        repository_id: number("repository_id")?,
        operation_id: operation(text("operation_id")?)?,
        item_id,
        expectation: PromotionExpectation::from_bytes(
            bytes.try_into().map_err(|_| state::recovery_required())?,
        ),
        remote_name: opt("remote_name")?,
        step: PromotionStep::parse(&text("step")?)?,
        context_attempt: number("context_attempt")?
            .try_into()
            .map_err(|_| state::recovery_required())?,
        context_sync_id: opt("context_sync_id")?.map(operation).transpose()?,
        primary_attempt: number("primary_attempt")?
            .try_into()
            .map_err(|_| state::recovery_required())?,
        primary_sync_id: opt("primary_sync_id")?.map(operation).transpose()?,
        base_oid: oid("base_oid")?,
        context_oid: oid("context_oid")?,
        integration_oid: oid("integration_oid")?,
        integration_kind: opt("integration_kind")?
            .map(|s| IntegrationKind::parse(&s))
            .transpose()?,
        remote_context_oid: oid("remote_context_oid")?,
        publication: opt("publication_category")?
            .map(|s| PublicationCategory::parse(&s))
            .transpose()?,
        cleanup: opt("cleanup_category")?
            .map(|s| CleanupCategory::parse(&s))
            .transpose()?,
        cancel_requested: match number("cancel_requested")? {
            0 => false,
            1 => true,
            _ => return Err(state::recovery_required()),
        },
        updated_at: number("updated_at")?,
        refusal,
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
    db.query_row(
        "SELECT * FROM document_promotions WHERE repository_id=?1 AND operation_id=?2",
        params![id, op.to_string()],
        |r| Ok(decode(r)),
    )
    .optional()
    .map_err(|_| state::recovery_required())?
    .transpose()
}
pub(in super::super) fn records(
    db: &Connection,
    id: i64,
) -> Result<Vec<PromotionRecord>, RepositoryError> {
    db.prepare("SELECT * FROM document_promotions WHERE repository_id=?1 ORDER BY operation_id")
        .and_then(|mut q| {
            q.query_map([id], |r| Ok(decode(r)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| state::recovery_required())?
        .into_iter()
        .collect()
}
fn existing_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
) -> Result<Option<PromotionRecord>, RepositoryError> {
    let existing = read_record(tx, id, op)?;
    let collision:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1) OR EXISTS(SELECT 1 FROM document_promotions WHERE (operation_id=?1 AND repository_id!=?2) OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND (repository_id!=?2 OR action!='promote' OR kind!='document' OR item_id!=?3))",params![op.to_string(),id,item.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if collision {
        return Err(mismatch());
    }
    if let Some(r) = existing {
        if &r.item_id != item || r.expectation != expectation {
            return Err(mismatch());
        }
        if let Some(reservation) = state::read_operation(tx, id, op)? {
            validate_remote_identity(tx, id, op, &reservation.target, reservation.priority)?;
        }
        return Ok(Some(r));
    }
    // A foreign promote row is not proof of a parent, even if its item matches.
    if tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1)",
            [op.to_string()],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|_| state::recovery_required())?
    {
        return Err(mismatch());
    }
    Ok(None)
}
fn unfinished_record(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<Option<PromotionRecord>, RepositoryError> {
    tx
        .query_row(
            "SELECT * FROM document_promotions WHERE repository_id=?1 AND item_id=?2 AND step NOT IN ('complete','abandoned','superseded')",
            params![id, item.to_string()],
            |r| Ok(decode(r)),
        )
        .optional()
        .map_err(|_| state::recovery_required())?
        .transpose()
}
/// Process-only fence for the exact predecessor observed before unleased
/// discovery. A different parent or owner cannot be silently superseded.
#[derive(PartialEq, Eq)]
pub(in super::super) struct ReplacementGuard {
    repository_id: i64,
    parent: Option<PromotionRecord>,
    owner: Option<(OperationId, i64, i64, RemoteOperationPhase, bool, bool, i64)>,
}
pub(in super::super) fn replacement_guard(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<ReplacementGuard, RepositoryError> {
    let parent = unfinished_record(tx, id, item)?;
    let owner = parent
        .as_ref()
        .map(|parent| state::read_operation(tx, id, parent.operation_id))
        .transpose()?
        .flatten()
        .map(|r| {
            (
                r.operation_id,
                r.generation,
                r.owner_epoch,
                r.phase,
                r.cancel_requested,
                r.yield_requested,
                r.updated_at,
            )
        });
    Ok(ReplacementGuard {
        repository_id: id,
        parent,
        owner,
    })
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
    if let Some(existing) = existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    let unfinished = unfinished_record(tx, id, item)?;
    if let Some(old) = unfinished {
        if !old.cleanup.is_some_and(CleanupCategory::supersedable) {
            return Err(busy());
        }
        if let Some(remote) = state::read_operation(tx, id, old.operation_id)? {
            validate_remote_identity(tx, id, old.operation_id, &remote.target, remote.priority)?;
            let epoch = remote
                .owner_epoch
                .checked_add(1)
                .ok_or_else(state::recovery_required)?;
            // Supersession is terminal for this owner. Release only its promote
            // slot, atomically with replacing the parent, never a child's slot.
            tx.execute("UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled',owner_epoch=?2,cancel_requested=0,yield_requested=0,updated_at=max(updated_at,?3) WHERE id=?1",params![remote.id,epoch,now()]).map_err(|_|state::recovery_required())?;
        }
        tx.execute("UPDATE document_promotions SET step='superseded',updated_at=max(updated_at,?2) WHERE operation_id=?1",params![old.operation_id.to_string(),now()]).map_err(|_|state::recovery_required())?;
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
    if let Some(existing) = existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if unfinished_record(tx, id, item)?
        .is_some_and(|old| !old.cleanup.is_some_and(CleanupCategory::supersedable))
    {
        return Err(busy());
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
    let (category, blockers) = refusal
        .map(Refusal::encoded)
        .transpose()?
        .map_or((None, None), |(category, blockers)| {
            (Some(category), blockers)
        });
    let step = if refusal.is_some() {
        "abandoned"
    } else {
        "confirmed"
    };
    tx.execute("INSERT INTO document_promotions(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,refusal_category,refusal_blockers) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,op.to_string(),item.to_string(),expectation.to_bytes().as_slice(),remote,step,now(),category,blockers]).map_err(|_|state::recovery_required())?;
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
        return Err(mismatch());
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
    let (refusal, mask) = r
        .refusal
        .as_ref()
        .map(Refusal::encoded)
        .transpose()?
        .map_or((None, None), |(c, m)| (Some(c), m));
    tx.execute("UPDATE document_promotions SET step=?2,base_oid=?3,context_oid=?4,integration_oid=?5,integration_kind=?6,remote_context_oid=?7,publication_category=?8,cleanup_category=?9,refusal_category=?10,refusal_blockers=?11,updated_at=max(updated_at,?12) WHERE operation_id=?1",params![r.operation_id.to_string(),r.step.name(),r.base_oid.map(|o|o.to_string()),r.context_oid.map(|o|o.to_string()),r.integration_oid.map(|o|o.to_string()),r.integration_kind.map(IntegrationKind::name),r.remote_context_oid.map(|o|o.to_string()),r.publication.map(PublicationCategory::name),r.cleanup.map(CleanupCategory::name),refusal,mask,now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ChildRole {
    Context,
    Primary,
}
/// Private, process-only identity of a parent-issued attempt. It holds no lock
/// and grants no ownership until the reservation SQL validates the current
/// durable binding. In particular, a formerly-bound no-row ID cannot fall back
/// to an ordinary synchronization when another executor replaces its attempt.
#[derive(Clone)]
pub(in super::super) struct ChildExecution {
    repository_id: i64,
    parent: OperationId,
    item: ItemId,
    expectation: PromotionExpectation,
    role: ChildRole,
    attempt: u32,
    child: OperationId,
}
pub(in super::super) fn child_execution(
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<ChildExecution, RepositoryError> {
    let (attempt, child) = match role {
        ChildRole::Context => (r.context_attempt, r.context_sync_id),
        ChildRole::Primary => (r.primary_attempt, r.primary_sync_id),
    };
    if attempt == 0 || r.remote_name.is_none() {
        return Err(mismatch());
    }
    Ok(ChildExecution {
        repository_id: r.repository_id,
        parent: r.operation_id,
        item: r.item_id.clone(),
        expectation: r.expectation,
        role,
        attempt,
        child: child.ok_or_else(mismatch)?,
    })
}
impl ChildExecution {
    pub(in super::super) fn validate_request(
        &self,
        request: &crate::repository::SynchronizeRemoteRequest,
    ) -> Result<(), RepositoryError> {
        let matches = match (&self.role, &request.target) {
            (ChildRole::Primary, crate::repository::SynchronizationTarget::Primary) => true,
            (
                ChildRole::Context,
                crate::repository::SynchronizationTarget::Context {
                    kind: AuthoringKind::Document,
                    item_id,
                },
            ) => item_id == &self.item,
            _ => false,
        };
        if request.operation_id != self.child || !matches {
            return Err(mismatch());
        }
        Ok(())
    }
    pub(in super::super) fn validate(
        &self,
        db: &Connection,
        id: i64,
        op: OperationId,
    ) -> Result<(), RepositoryError> {
        if id != self.repository_id || op != self.child {
            return Err(mismatch());
        }
        let parent = read_record(db, id, self.parent)?.ok_or_else(mismatch)?;
        let (attempt, child, allowed) = match self.role {
            ChildRole::Context => (
                parent.context_attempt,
                parent.context_sync_id,
                parent.step == PromotionStep::Confirmed,
            ),
            ChildRole::Primary => (
                parent.primary_attempt,
                parent.primary_sync_id,
                matches!(
                    parent.step,
                    PromotionStep::Integrated | PromotionStep::PublicationPending
                ),
            ),
        };
        if parent.item_id != self.item
            || parent.expectation != self.expectation
            || attempt != self.attempt
            || child != Some(self.child)
        {
            return Err(mismatch());
        }
        let record = state::read_operation(db, id, op)?;
        // Exact, current authoritative handoff remains valid after parent
        // progress/completion; this path can never create/restart remote work.
        if record.as_ref().is_some_and(|r| r.authority.is_some()) {
            return Ok(());
        }
        if !allowed || (parent.cancel_requested && record.is_none()) {
            return Err(mismatch());
        }
        Ok(())
    }
}
/// Authoritative completion is consumed first. Retained includes clean candidates,
/// installed conflicts and unreleased owned resolutions, regardless of outer error.
pub(in super::super) enum ChildState {
    Absent,
    Complete(state::StoredRemoteOperation),
    Retained(state::StoredRemoteOperation),
    Unowned(state::StoredRemoteOperation),
}
pub(in super::super) fn child_state(
    db: &Connection,
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<ChildState, RepositoryError> {
    let child = match role {
        ChildRole::Context => r.context_sync_id,
        ChildRole::Primary => r.primary_sync_id,
    };
    let Some(child) = child else {
        return Ok(ChildState::Absent);
    };
    let foreign:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![child.to_string(),r.repository_id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
    if foreign {
        return Err(state::recovery_required());
    }
    let Some(record) = state::read_operation(db, r.repository_id, child)? else {
        return Ok(ChildState::Absent);
    };
    validate_remote_identity(db, r.repository_id, child, &record.target, record.priority)?;
    if record.authority.is_some() {
        return Ok(ChildState::Complete(record));
    }
    if reservation::retains_promotion_child(db, &record)? {
        Ok(ChildState::Retained(record))
    } else {
        Ok(ChildState::Unowned(record))
    }
}
pub(in super::super) fn next_attempt(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    role: ChildRole,
) -> Result<PromotionRecord, RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    let allowed = match role {
        ChildRole::Context => r.step == PromotionStep::Confirmed,
        ChildRole::Primary => matches!(
            r.step,
            PromotionStep::Integrated | PromotionStep::PublicationPending
        ),
    };
    if !allowed {
        return Err(state::recovery_required());
    }
    match child_state(tx, &r, role)? {
        ChildState::Absent => (),
        ChildState::Unowned(child)
            if matches!(
                child.phase,
                RemoteOperationPhase::Cancelled
                    | RemoteOperationPhase::Interrupted
                    | RemoteOperationPhase::Failed
            ) => {}
        _ => return Err(state::recovery_required()),
    }
    mint_attempt(tx, &r, role)
}
fn mint_attempt(
    tx: &Transaction<'_>,
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<PromotionRecord, RepositoryError> {
    let (attempt, column, count) = match role {
        ChildRole::Context => (r.context_attempt, "context_sync_id", "context_attempt"),
        ChildRole::Primary => (r.primary_attempt, "primary_sync_id", "primary_attempt"),
    };
    let attempt = attempt
        .checked_add(1)
        .ok_or_else(state::recovery_required)?;
    let child = OperationId::new();
    let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM document_promotions WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1)",[child.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if used {
        return Err(state::recovery_required());
    }
    tx.execute(&format!("UPDATE document_promotions SET {column}=?2,{count}=?3,updated_at=max(updated_at,?4) WHERE operation_id=?1"),params![r.operation_id.to_string(),child.to_string(),attempt,now()]).map_err(|_|state::recovery_required())?;
    read_record(tx, r.repository_id, r.operation_id)?.ok_or_else(state::recovery_required)
}
/// The sole back-transition: a fresh integration preparation proved primary
/// advancement before `integrating` intent. A completed context child may be
/// replaced here; ordinary retries may not replace authoritative completion.
pub(in super::super) fn reset_context_after_primary_advance(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<PromotionRecord, RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step != PromotionStep::ContextSynchronized
        || r.remote_name.is_none()
        || matches!(
            child_state(tx, &r, ChildRole::Context)?,
            ChildState::Retained(_) | ChildState::Unowned(_)
        )
    {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE document_promotions SET step='confirmed',context_oid=NULL,remote_context_oid=NULL,base_oid=NULL,integration_kind=NULL,cleanup_category=NULL WHERE operation_id=?1",[op.to_string()]).map_err(|_|state::recovery_required())?;
    mint_attempt(tx, &r, ChildRole::Context)
}
/// Checks both parent and child identities in the same transaction as reservation.
pub(in super::super) fn validate_remote_identity(
    db: &Connection,
    id: i64,
    op: OperationId,
    target: &RemoteOperationTarget,
    priority: RemoteOperationPriority,
) -> Result<(), RepositoryError> {
    let bindings=db.prepare("SELECT * FROM document_promotions WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1").and_then(|mut q|q.query_map([op.to_string()],|row|Ok(decode(row)))?.collect::<Result<Vec<_>,_>>()).map_err(|_|state::recovery_required())?;
    for r in bindings {
        let r = r?;
        let expected = if r.operation_id == op {
            RemoteOperationAction::Promote
        } else if r.context_sync_id == Some(op) {
            RemoteOperationAction::SynchronizeContext
        } else {
            RemoteOperationAction::SynchronizePrimary
        };
        if r.repository_id != id
            || priority != RemoteOperationPriority::Manual
            || target.action() != expected
            || Some(target.remote_name()) != r.remote_name.as_deref()
            || expected != RemoteOperationAction::SynchronizePrimary
                && target.item() != Some((AuthoringKind::Document, &r.item_id))
        {
            return Err(mismatch());
        }
    }
    Ok(())
}
pub(in super::super) fn validate_local_identity(
    db: &Connection,
    root: &str,
    action: &str,
    op: &str,
    target: &str,
) -> Result<(), RepositoryError> {
    if !crate::repository::recovery::has_table(db, "document_promotions")
        .map_err(|_| state::recovery_required())?
    {
        return Ok(());
    }
    let parent: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM document_promotions WHERE operation_id=?1)",
            [op],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if parent {
        return Err(mismatch());
    }
    let child:Option<i64>=db.query_row("SELECT repository_id FROM document_promotions WHERE context_sync_id=?1 OR primary_sync_id=?1",[op],|r|r.get(0)).optional().map_err(|_|state::recovery_required())?;
    if let Some(id) = child {
        let registered: String = db
            .query_row(
                "SELECT root_path FROM repositories WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| state::recovery_required())?;
        let record = state::read_operation(db, id, operation(op.to_owned())?)?;
        if let Some(record) = &record {
            validate_remote_identity(db, id, record.operation_id, &record.target, record.priority)?;
        }
        if action != "refresh"
            || !target.is_empty()
            || registered != root
            || record.as_ref().is_none_or(|r| r.authority.is_none())
        {
            return Err(mismatch());
        }
    }
    Ok(())
}
pub(in super::super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    validate_schema(db)?;
    if db
        .prepare("PRAGMA foreign_key_check(document_promotions)")
        .and_then(|mut q| q.exists([]))
        .map_err(|_| state::recovery_required())?
    {
        return Err(state::recovery_required());
    }
    let ids = db
        .prepare("SELECT DISTINCT repository_id FROM document_promotions")
        .and_then(|mut q| {
            q.query_map([], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| state::recovery_required())?;
    for id in ids {
        for r in records(db, id)? {
            let local:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid IN (?1,?2,?3) OR synchronization_ulid IN (?1,?2,?3)) OR EXISTS(SELECT 1 FROM document_promotions p WHERE p.operation_id!=?1 AND (p.operation_id IN (?2,?3) OR p.context_sync_id IN (?1,?2,?3) OR p.primary_sync_id IN (?1,?2,?3)))",params![r.operation_id.to_string(),r.context_sync_id.map(|v|v.to_string()),r.primary_sync_id.map(|v|v.to_string())],|r|r.get(0)).map_err(|_|state::recovery_required())?;
            if local {
                return Err(state::recovery_required());
            }
            let foreign:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![r.operation_id.to_string(),id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if foreign {
                return Err(state::recovery_required());
            }
            if let Some(remote) = state::read_operation(db, id, r.operation_id)? {
                validate_remote_identity(db, id, r.operation_id, &remote.target, remote.priority)
                    .map_err(|_| state::recovery_required())?;
            }
            for role in [ChildRole::Context, ChildRole::Primary] {
                child_state(db, &r, role).map_err(|_| state::recovery_required())?;
            }
            let local_children=db.prepare("SELECT root_path,action,coalesce(target,''),operation_ulid FROM operation_records WHERE operation_ulid IN (?1,?2)").and_then(|mut q|q.query_map(params![r.context_sync_id.map(|v|v.to_string()),r.primary_sync_id.map(|v|v.to_string())],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|_|state::recovery_required())?;
            for (root, action, target, child) in local_children {
                validate_local_identity(db, &root, &action, &child, &target)
                    .map_err(|_| state::recovery_required())?;
            }
        }
    }
    Ok(())
}

pub(in super::super) fn complete_record(
    tx: &Transaction<'_>,
    service: &RepositoryService,
    id: i64,
    op: OperationId,
    evidence: &RetirementEvidence<'_>,
    owner: Option<&RemoteReservation>,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step != PromotionStep::BranchDeleted
        || !evidence.matches_promotion(
            op,
            &r.item_id,
            r.context_oid.ok_or_else(state::recovery_required)?,
        )
        || owner.is_some() && r.remote_name.is_none()
        || deletion_reservation_required(&r) && owner.is_none()
    {
        return Err(state::recovery_required());
    }
    let configured: Option<String> = tx
        .query_row(
            "SELECT remote_name FROM remote_polling_state WHERE repository_id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if configured != r.remote_name {
        return Err(state::recovery_required());
    }
    if state::read_operation(tx, id, op)?.is_some() != owner.is_some() {
        return Err(state::recovery_required());
    }
    if let Some(owner) = owner {
        if owner.operation_id() != op {
            return Err(mismatch());
        }
        let remote = reservation::owned(service, tx, id, owner)?;
        validate_remote_identity(tx, id, op, &remote.target, remote.priority)?;
    }
    cleanup::retire_context(tx, id, evidence)?;
    super::super::recovery::invalidate_index_observations(tx, id)?;
    // Completion may be followed immediately by process exit (F11). Its
    // index-only handoff must be durable in this same transaction so replay
    // cannot mistake pre-cleanup discovery for an already-refreshed outcome.
    tx.execute(
        "UPDATE repositories SET refresh_required=1 WHERE id=?1",
        [id],
    )
    .map_err(|_| state::recovery_required())?;
    tx.execute("UPDATE document_promotions SET step='complete',cleanup_category=NULL,cancel_requested=0,updated_at=max(updated_at,?2) WHERE operation_id=?1",params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    if let Some(owner) = owner {
        reservation::finish_promotion(service, tx, id, owner)?;
    }
    Ok(())
}
/// Inspect actual committed parent state after an uncertain commit result.
/// Git-ref absence is never used as completion/retirement evidence here.
pub(in super::super) fn reconcile_completion(
    db: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    let r = read_record(db, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step != PromotionStep::Complete {
        return Ok(false);
    }
    let remote = state::read_operation(db, id, op)?;
    if let Some(remote) = remote {
        if r.remote_name.is_none() {
            return Err(state::recovery_required());
        }
        validate_remote_identity(db, id, op, &remote.target, remote.priority)?;
        if remote.phase != RemoteOperationPhase::Completed {
            return Err(state::recovery_required());
        }
    } else if deletion_reservation_required(&r) {
        return Err(state::recovery_required());
    }
    // Retirement/history are mutable projections: a later first publication can
    // remove retirement and create new history. The parent's committed complete
    // row is the durable authority, not those projections or current Git absence.
    Ok(true)
}
fn deletion_reservation_required(r: &PromotionRecord) -> bool {
    r.remote_name.is_some()
        && !(r.integration_kind == Some(IntegrationKind::AlreadyContained)
            && r.remote_context_oid.is_none())
}
pub(in super::super) fn request_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    let Some(r) = read_record(tx, id, op)? else {
        return Ok(false);
    };
    if r.step.terminal() {
        return Ok(false);
    }
    tx.execute("UPDATE document_promotions SET cancel_requested=1,updated_at=max(updated_at,?2) WHERE operation_id=?1",params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    tx.execute("UPDATE remote_operation_records SET cancel_requested=1,updated_at=max(updated_at,?5) WHERE repository_id=?1 AND operation_ulid IN (?2,?3,?4) AND phase NOT IN ('completed','interrupted','cancelled','failed')",params![id,op.to_string(),r.context_sync_id.map(|v|v.to_string()),r.primary_sync_id.map(|v|v.to_string()),now()]).map_err(|_|state::recovery_required())?;
    Ok(true)
}
/// Explicit same-ID continuation consumes a parent's inter-step stop. Child
/// cancellation/ownership is reconciled independently before replacing any ID.
pub(in super::super) fn clear_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step.terminal() {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE document_promotions SET cancel_requested=0,updated_at=max(updated_at,?2) WHERE operation_id=?1",params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
