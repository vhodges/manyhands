//! Ticket closure parent storage only: no transport, Git effect, prompt or
//! lease acquisition. The record holds IDs, ref names, OIDs, timestamps
//! (`closed_at` included), fixed step names and fixed categories; never
//! `closed_by`, a title, a body, an endpoint or a secret.
#![allow(dead_code)] // The closure lifecycle (Tasks 4–6) composes these transaction-scoped seams.
use super::{ClosureBlocker, ClosureExpectation};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationId, RemoteOperationAction, RemoteOperationPhase,
        RemoteOperationPriority, RemoteOperationTarget, RepositoryError, RepositoryErrorKind,
        RepositoryOperation, RepositoryService,
        cleanup::{self, RetirementEvidence},
        promotion::{
            lifecycle::{LifecycleKind, LifecycleRecord, LifecycleRefusal, ParentStep},
            record::{
                self as promotion, ChildExecution, ChildRole, ChildState, CleanupCategory,
                IntegrationKind, PublicationCategory,
            },
        },
        remote::{
            reservation::{self, RemoteReservation},
            state,
        },
    },
};
use git2::Oid;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum Refusal {
    Blocked(Vec<ClosureBlocker>),
    ExternalChange,
    ReconfirmationRequired,
}
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
const MASK_LIMIT: u32 = (1 << CLOSURE_BLOCKERS.len()) - 1;
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
                            << CLOSURE_BLOCKERS
                                .iter()
                                .position(|v| v == b)
                                .expect("exhaustive blockers")
                    })),
                )
            }
        })
    }
}
impl LifecycleRefusal for Refusal {
    fn external_change() -> Self {
        Self::ExternalChange
    }
    fn reconfirmation_required() -> Self {
        Self::ReconfirmationRequired
    }
    fn integration_conflict(&self) -> bool {
        matches!(self, Self::Blocked(b) if b.contains(&ClosureBlocker::IntegrationConflict))
    }
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
}
impl ClosureRecord {
    /// The context commit integration merges: the checkpoint when one was
    /// written, else the synchronized pre-closure context.
    pub(in super::super) fn merged_context(&self) -> Option<Oid> {
        self.closure_oid.or(self.context_oid)
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
    CHECK((context_attempt=0)=(context_sync_id IS NULL)),
    CHECK((primary_attempt=0)=(primary_sync_id IS NULL)),
    CHECK(context_sync_id IS NULL OR context_sync_id!=operation_id),
    CHECK(primary_sync_id IS NULL OR (primary_sync_id!=operation_id AND primary_sync_id IS NOT context_sync_id)),
    CHECK((step='abandoned')=(refusal_category IS NOT NULL)),
    CHECK((refusal_category IS 'blocked')=(refusal_blockers IS NOT NULL)),
    CHECK(closure_kind IS NOT NULL OR step='abandoned'),
    CHECK((closed_at IS NULL)=(closure_oid IS NULL)),
    CHECK(closed_at IS NULL OR closure_kind IS 'record'),
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
fn mismatch() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        "the closure ID is bound to another operation",
    )
}
fn busy() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::RepositoryBusy,
        "another closure of the ticket is unfinished",
    )
}
fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp().max(0)
}
fn operation(text: String) -> Result<OperationId, RepositoryError> {
    let id = OperationId::parse(&text).map_err(|_| state::recovery_required())?;
    if id.to_string() != text {
        return Err(state::recovery_required());
    }
    Ok(id)
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
        (Some("blocked"), Some(mask)) if mask > 0 && mask <= MASK_LIMIT => Some(Refusal::Blocked(
            CLOSURE_BLOCKERS
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
    let r = ClosureRecord {
        repository_id: number("repository_id")?,
        operation_id: operation(text("operation_id")?)?,
        item_id,
        expectation: ClosureExpectation::from_bytes(
            bytes.try_into().map_err(|_| state::recovery_required())?,
        ),
        remote_name: opt("remote_name")?,
        step: ClosureStep::parse(&text("step")?)?,
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
        closure_kind: opt("closure_kind")?
            .map(|s| ClosureKind::parse(&s))
            .transpose()?,
        closed_at: opt("closed_at")?.map(|s| parse_closed_at(&s)).transpose()?,
        closure_oid: oid("closure_oid")?,
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
    db.query_row(
        "SELECT * FROM ticket_closures WHERE repository_id=?1 AND operation_id=?2",
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
) -> Result<Vec<ClosureRecord>, RepositoryError> {
    db.prepare("SELECT * FROM ticket_closures WHERE repository_id=?1 ORDER BY operation_id")
        .and_then(|mut q| {
            q.query_map([id], |r| Ok(decode(r)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| state::recovery_required())?
        .into_iter()
        .collect()
}
/// Remote identity across both parent tables: a reservation under any
/// parent or child ID must be exactly that binding's.
fn remote_identity(
    db: &Connection,
    id: i64,
    op: OperationId,
    target: &RemoteOperationTarget,
    priority: RemoteOperationPriority,
) -> Result<(), RepositoryError> {
    promotion::validate_remote_identity(db, id, op, target, priority)
}
fn existing_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: ClosureExpectation,
) -> Result<Option<ClosureRecord>, RepositoryError> {
    let existing = read_record(tx, id, op)?;
    let collision:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1) OR EXISTS(SELECT 1 FROM document_promotions WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM ticket_closures WHERE (operation_id=?1 AND repository_id!=?2) OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND (repository_id!=?2 OR action!='close' OR kind!='ticket' OR item_id!=?3))",params![op.to_string(),id,item.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if collision {
        return Err(mismatch());
    }
    if let Some(r) = existing {
        if &r.item_id != item || r.expectation != expectation {
            return Err(mismatch());
        }
        if let Some(reservation) = state::read_operation(tx, id, op)? {
            remote_identity(tx, id, op, &reservation.target, reservation.priority)?;
        }
        return Ok(Some(r));
    }
    // A foreign close row is not proof of a parent, even if its item matches.
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
) -> Result<Option<ClosureRecord>, RepositoryError> {
    tx
        .query_row(
            "SELECT * FROM ticket_closures WHERE repository_id=?1 AND item_id=?2 AND step NOT IN ('complete','abandoned','superseded')",
            params![id, item.to_string()],
            |r| Ok(decode(r)),
        )
        .optional()
        .map_err(|_| state::recovery_required())?
        .transpose()
}
/// Only a cleanup stop after integration may be superseded. A written
/// closure checkpoint (`closing`, `integrating`) never is: only its own ID
/// finishes it.
fn supersedable(old: &ClosureRecord) -> bool {
    !matches!(old.step, ClosureStep::Closing | ClosureStep::Integrating)
        && old.cleanup.is_some_and(CleanupCategory::supersedable)
}
/// Process-only fence for the exact predecessor observed before unleased
/// discovery. A different parent or owner cannot be silently superseded.
#[derive(PartialEq, Eq)]
pub(in super::super) struct ReplacementGuard {
    repository_id: i64,
    parent: Option<ClosureRecord>,
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
    if let Some(existing) = existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if let Some(old) = unfinished_record(tx, id, item)? {
        if !supersedable(&old) {
            return Err(busy());
        }
        if let Some(remote) = state::read_operation(tx, id, old.operation_id)? {
            remote_identity(tx, id, old.operation_id, &remote.target, remote.priority)?;
            let epoch = remote
                .owner_epoch
                .checked_add(1)
                .ok_or_else(state::recovery_required)?;
            // Supersession is terminal for this owner. Release only its close
            // slot, atomically with replacing the parent, never a child's slot.
            tx.execute("UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled',owner_epoch=?2,cancel_requested=0,yield_requested=0,updated_at=max(updated_at,?3) WHERE id=?1",params![remote.id,epoch,now()]).map_err(|_|state::recovery_required())?;
        }
        tx.execute("UPDATE ticket_closures SET step='superseded',updated_at=max(updated_at,?2) WHERE operation_id=?1",params![old.operation_id.to_string(),now()]).map_err(|_|state::recovery_required())?;
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
    if let Some(existing) = existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if unfinished_record(tx, id, item)?.is_some_and(|old| !supersedable(&old)) {
        return Err(busy());
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
    tx.execute("INSERT INTO ticket_closures(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,refusal_category,refusal_blockers,closure_kind) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,op.to_string(),item.to_string(),expectation.to_bytes().as_slice(),remote,step,now(),category,blockers,kind.map(ClosureKind::name)]).map_err(|_|state::recovery_required())?;
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
        return Err(mismatch());
    }
    if !old
        .step
        .permits(r.step, r.remote_name.is_some(), old.closure_kind)
        || r.cancel_requested != old.cancel_requested
        || old.closed_at != r.closed_at
        || old.closure_oid != r.closure_oid
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
    tx.execute("UPDATE ticket_closures SET step=?2,base_oid=?3,context_oid=?4,integration_oid=?5,integration_kind=?6,remote_context_oid=?7,publication_category=?8,cleanup_category=?9,refusal_category=?10,refusal_blockers=?11,updated_at=max(updated_at,?12) WHERE operation_id=?1",params![r.operation_id.to_string(),r.step.name(),r.base_oid.map(|o|o.to_string()),r.context_oid.map(|o|o.to_string()),r.integration_oid.map(|o|o.to_string()),r.integration_kind.map(IntegrationKind::name),r.remote_context_oid.map(|o|o.to_string()),r.publication.map(PublicationCategory::name),r.cleanup.map(CleanupCategory::name),refusal,mask,now()]).map_err(|_|state::recovery_required())?;
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
    tx.execute("UPDATE ticket_closures SET step='closing',base_oid=?2,closed_at=?3,closure_oid=?4,updated_at=max(updated_at,?5) WHERE operation_id=?1",params![operation.to_string(),base.to_string(),closed_at,closure.to_string(),now()]).map_err(|_|state::recovery_required())?;
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
    tx.execute("UPDATE ticket_closures SET closure_oid=?2,updated_at=max(updated_at,?3) WHERE operation_id=?1",params![operation.to_string(),closure.to_string(),now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

/// The C9 resume of a written checkpoint, while `closing`: the base the merge
/// uses now (the current tracking or local primary) and the merged context
/// (the checkpoint, or an adoptable descendant of it). `closed_at` is kept.
/// The caller has proved, under the lease, that the context ref has moved to
/// `merged`.
pub(in super::super) fn adopt_written_closure(
    tx: &Transaction<'_>,
    repository_id: i64,
    operation: OperationId,
    base: Oid,
    merged: Oid,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, repository_id, operation)?.ok_or_else(state::recovery_required)?;
    if r.step != ClosureStep::Closing || base.is_zero() || merged.is_zero() {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE ticket_closures SET base_oid=?2,closure_oid=?3,updated_at=max(updated_at,?4) WHERE operation_id=?1",params![operation.to_string(),base.to_string(),merged.to_string(),now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

pub(in super::super) fn child_execution(
    r: &ClosureRecord,
    role: ChildRole,
) -> Result<ChildExecution, RepositoryError> {
    let (attempt, child) = match role {
        ChildRole::Context => (r.context_attempt, r.context_sync_id),
        ChildRole::Primary => (r.primary_attempt, r.primary_sync_id),
    };
    if attempt == 0 || r.remote_name.is_none() {
        return Err(mismatch());
    }
    Ok(ChildExecution::new(
        LifecycleKind::TicketClosure,
        r.repository_id,
        r.operation_id,
        r.item_id.clone(),
        r.expectation.to_bytes(),
        role,
        attempt,
        child.ok_or_else(mismatch)?,
    ))
}
/// Authoritative completion is consumed first. Retained includes clean
/// candidates, installed conflicts and unreleased owned resolutions.
pub(in super::super) fn child_state(
    db: &Connection,
    r: &ClosureRecord,
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
    remote_identity(db, r.repository_id, child, &record.target, record.priority)?;
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
) -> Result<ClosureRecord, RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    let allowed = match role {
        ChildRole::Context => r.step == ClosureStep::Confirmed,
        ChildRole::Primary => matches!(
            r.step,
            ClosureStep::Integrated | ClosureStep::PublicationPending
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
    r: &ClosureRecord,
    role: ChildRole,
) -> Result<ClosureRecord, RepositoryError> {
    let (attempt, column, count) = match role {
        ChildRole::Context => (r.context_attempt, "context_sync_id", "context_attempt"),
        ChildRole::Primary => (r.primary_attempt, "primary_sync_id", "primary_attempt"),
    };
    let attempt = attempt
        .checked_add(1)
        .ok_or_else(state::recovery_required)?;
    let child = OperationId::new();
    let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM ticket_closures WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM document_promotions WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1)",[child.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if used {
        return Err(state::recovery_required());
    }
    tx.execute(&format!("UPDATE ticket_closures SET {column}=?2,{count}=?3,updated_at=max(updated_at,?4) WHERE operation_id=?1"),params![r.operation_id.to_string(),child.to_string(),attempt,now()]).map_err(|_|state::recovery_required())?;
    read_record(tx, r.repository_id, r.operation_id)?.ok_or_else(state::recovery_required)
}
/// The sole back-transition: a fresh observation proved primary advancement
/// before `closing` (design step 3). Nothing has been written; a completed
/// context child may be replaced here.
pub(in super::super) fn reset_context_after_primary_advance(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<ClosureRecord, RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step != ClosureStep::ContextSynchronized
        || r.remote_name.is_none()
        || matches!(
            child_state(tx, &r, ChildRole::Context)?,
            ChildState::Retained(_) | ChildState::Unowned(_)
        )
    {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE ticket_closures SET step='confirmed',context_oid=NULL,remote_context_oid=NULL,base_oid=NULL,integration_kind=NULL,cleanup_category=NULL WHERE operation_id=?1",[op.to_string()]).map_err(|_|state::recovery_required())?;
    mint_attempt(tx, &r, ChildRole::Context)
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
    let bindings=db.prepare("SELECT * FROM ticket_closures WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1").and_then(|mut q|q.query_map([op.to_string()],|row|Ok(decode(row)))?.collect::<Result<Vec<_>,_>>()).map_err(|_|state::recovery_required())?;
    for r in bindings {
        let r = r?;
        let expected = if r.operation_id == op {
            RemoteOperationAction::Close
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
                && target.item() != Some((AuthoringKind::Ticket, &r.item_id))
        {
            return Err(mismatch());
        }
    }
    Ok(())
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
    if !crate::repository::recovery::has_table(db, "ticket_closures")
        .map_err(|_| state::recovery_required())?
    {
        return Ok(());
    }
    let parent: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM ticket_closures WHERE operation_id=?1)",
            [op],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if parent {
        return Err(mismatch());
    }
    let child:Option<i64>=db.query_row("SELECT repository_id FROM ticket_closures WHERE context_sync_id=?1 OR primary_sync_id=?1",[op],|r|r.get(0)).optional().map_err(|_|state::recovery_required())?;
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
            remote_identity(db, id, record.operation_id, &record.target, record.priority)?;
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
        .prepare("PRAGMA foreign_key_check(ticket_closures)")
        .and_then(|mut q| q.exists([]))
        .map_err(|_| state::recovery_required())?
    {
        return Err(state::recovery_required());
    }
    let ids = db
        .prepare("SELECT DISTINCT repository_id FROM ticket_closures")
        .and_then(|mut q| {
            q.query_map([], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| state::recovery_required())?;
    for id in ids {
        for r in records(db, id)? {
            let local:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid IN (?1,?2,?3) OR synchronization_ulid IN (?1,?2,?3)) OR EXISTS(SELECT 1 FROM ticket_closures c WHERE c.operation_id!=?1 AND (c.operation_id IN (?2,?3) OR c.context_sync_id IN (?1,?2,?3) OR c.primary_sync_id IN (?1,?2,?3))) OR EXISTS(SELECT 1 FROM document_promotions p WHERE p.operation_id IN (?1,?2,?3) OR p.context_sync_id IN (?1,?2,?3) OR p.primary_sync_id IN (?1,?2,?3))",params![r.operation_id.to_string(),r.context_sync_id.map(|v|v.to_string()),r.primary_sync_id.map(|v|v.to_string())],|r|r.get(0)).map_err(|_|state::recovery_required())?;
            if local {
                return Err(state::recovery_required());
            }
            let foreign:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![r.operation_id.to_string(),id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if foreign {
                return Err(state::recovery_required());
            }
            if let Some(remote) = state::read_operation(db, id, r.operation_id)? {
                remote_identity(db, id, r.operation_id, &remote.target, remote.priority)
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
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step != ClosureStep::BranchDeleted
        || !retirement_matches(
            evidence,
            op,
            &r.item_id,
            r.merged_context().ok_or_else(state::recovery_required)?,
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
        remote_identity(tx, id, op, &remote.target, remote.priority)?;
    }
    cleanup::retire_context(tx, id, evidence)?;
    crate::repository::recovery::invalidate_index_observations(tx, id)?;
    // Completion may be followed immediately by process exit. Its index-only
    // hand-off must be durable in this same transaction.
    tx.execute(
        "UPDATE repositories SET refresh_required=1 WHERE id=?1",
        [id],
    )
    .map_err(|_| state::recovery_required())?;
    tx.execute("UPDATE ticket_closures SET step='complete',cleanup_category=NULL,cancel_requested=0,updated_at=max(updated_at,?2) WHERE operation_id=?1",params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    if let Some(owner) = owner {
        reservation::finish_parent(LifecycleKind::TicketClosure, service, tx, id, owner)?;
    }
    Ok(())
}
/// The ticket retirement proof. `RetirementEvidence` binds documents only
/// until Task 5 adds its kind-checked `matches`; until then a closure never
/// completes (fail closed).
fn retirement_matches(
    _evidence: &RetirementEvidence<'_>,
    _operation: OperationId,
    _item: &ItemId,
    _merged_context: Oid,
) -> bool {
    false
}
/// Inspect actual committed parent state after an uncertain commit result.
pub(in super::super) fn reconcile_completion(
    db: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    let r = read_record(db, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step != ClosureStep::Complete {
        return Ok(false);
    }
    let remote = state::read_operation(db, id, op)?;
    if let Some(remote) = remote {
        if r.remote_name.is_none() {
            return Err(state::recovery_required());
        }
        remote_identity(db, id, op, &remote.target, remote.priority)?;
        if remote.phase != RemoteOperationPhase::Completed {
            return Err(state::recovery_required());
        }
    } else if deletion_reservation_required(&r) {
        return Err(state::recovery_required());
    }
    Ok(true)
}
fn deletion_reservation_required(r: &ClosureRecord) -> bool {
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
    tx.execute("UPDATE ticket_closures SET cancel_requested=1,updated_at=max(updated_at,?2) WHERE operation_id=?1",params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    tx.execute("UPDATE remote_operation_records SET cancel_requested=1,updated_at=max(updated_at,?5) WHERE repository_id=?1 AND operation_ulid IN (?2,?3,?4) AND phase NOT IN ('completed','interrupted','cancelled','failed')",params![id,op.to_string(),r.context_sync_id.map(|v|v.to_string()),r.primary_sync_id.map(|v|v.to_string()),now()]).map_err(|_|state::recovery_required())?;
    Ok(true)
}
pub(in super::super) fn clear_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step.terminal() {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE ticket_closures SET cancel_requested=0,updated_at=max(updated_at,?2) WHERE operation_id=?1",params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    Ok(())
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
