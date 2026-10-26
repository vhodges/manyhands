//! Parent-record storage shared by every lifecycle table (`document_promotions`
//! and `ticket_closures`): the category macro, row decoding, the refusal
//! mask, child attempts and their fence, cancellation, the identity audits
//! and completion. Each table's own module keeps its schema, step graph,
//! invariants, insert and save. A statement here differs between the tables
//! only in the table name and the lifecycle's own action and kind.
use super::lifecycle::{LifecycleKind, LifecycleRecord, LifecycleRefusal, ParentStep};
use crate::{
    canonical::ItemId,
    repository::{
        OperationId, RemoteOperationAction, RemoteOperationPhase, RemoteOperationPriority,
        RemoteOperationTarget, RepositoryError, RepositoryErrorKind, RepositoryOperation,
        RepositoryService,
        cleanup::{self, RetirementEvidence},
        closure::record::ClosureRecord,
        promotion::record::PromotionRecord,
        remote::{
            reservation::{self, RemoteReservation},
            state,
        },
    },
};
use git2::Oid;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

/// A closed set of stored names: `name` writes one, `parse` refuses any
/// other text as recovery.
macro_rules! categories {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Clone,Copy,Debug,PartialEq,Eq)]
        pub(in super::super) enum $name { $($variant),+ }
        impl $name {
            pub(in super::super) fn name(self)-> &'static str { match self { $(Self::$variant=>$text),+ } }
            pub(in super::super) fn parse(text: &str)->Result<Self,RepositoryError> { match text { $($text=>Ok(Self::$variant),)+ _=>Err(state::recovery_required()) } }
        }
    }
}
pub(in super::super) use categories;

impl LifecycleKind {
    /// The parent table.
    pub(in super::super) fn table(self) -> &'static str {
        match self {
            Self::DocumentPromotion => "document_promotions",
            Self::TicketClosure => "ticket_closures",
        }
    }
    fn other(self) -> Self {
        match self {
            Self::DocumentPromotion => Self::TicketClosure,
            Self::TicketClosure => Self::DocumentPromotion,
        }
    }
    /// The parent reservation's stored `action` and `kind`.
    fn stored_action(self) -> (&'static str, &'static str) {
        match self {
            Self::DocumentPromotion => ("promote", "document"),
            Self::TicketClosure => ("close", "ticket"),
        }
    }
}
impl ParentStep {
    pub(in super::super) fn terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Abandoned | Self::Superseded)
    }
}

pub(in super::super) fn mismatch(kind: LifecycleKind) -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        match kind {
            LifecycleKind::DocumentPromotion => "the promotion ID is bound to another operation",
            LifecycleKind::TicketClosure => "the closure ID is bound to another operation",
        },
    )
}
pub(in super::super) fn busy(kind: LifecycleKind) -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::RepositoryBusy,
        match kind {
            LifecycleKind::DocumentPromotion => "another promotion is unfinished",
            LifecycleKind::TicketClosure => "another closure of the ticket is unfinished",
        },
    )
}
pub(in super::super) fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp().max(0)
}
fn operation(text: String) -> Result<OperationId, RepositoryError> {
    let id = OperationId::parse(&text).map_err(|_| state::recovery_required())?;
    if id.to_string() != text {
        return Err(state::recovery_required());
    }
    Ok(id)
}

/// A lifecycle's preflight blockers in declaration order: bit `i` of the
/// stored refusal mask is entry `i`.
pub(in super::super) trait Blocker: Copy + Ord + 'static {
    const ORDER: &'static [Self];
    /// Under the integration lease this blocker is `ReconfirmationRequired`.
    const INTEGRATION_CONFLICT: Self;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum Refusal<B> {
    Blocked(Vec<B>),
    ExternalChange,
    ReconfirmationRequired,
}
impl<B: Blocker> Refusal<B> {
    pub(in super::super) fn encoded(&self) -> Result<(&'static str, Option<u32>), RepositoryError> {
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
                            << B::ORDER
                                .iter()
                                .position(|v| v == b)
                                .expect("exhaustive blockers")
                    })),
                )
            }
        })
    }
    /// `(category, mask)` as encoded; `None` is no refusal.
    pub(in super::super) fn encoded_columns(
        refusal: Option<&Self>,
    ) -> Result<(Option<&'static str>, Option<u32>), RepositoryError> {
        Ok(refusal
            .map(Self::encoded)
            .transpose()?
            .map_or((None, None), |(category, mask)| (Some(category), mask)))
    }
    fn decoded(category: Option<&str>, mask: Option<u32>) -> Result<Option<Self>, RepositoryError> {
        let limit = (1 << B::ORDER.len()) - 1;
        Ok(match (category, mask) {
            (None, None) => None,
            (Some("external_change"), None) => Some(Self::ExternalChange),
            (Some("reconfirmation_required"), None) => Some(Self::ReconfirmationRequired),
            (Some("blocked"), Some(mask)) if mask > 0 && mask <= limit => Some(Self::Blocked(
                B::ORDER
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| mask & (1 << i) != 0)
                    .map(|(_, b)| *b)
                    .collect(),
            )),
            _ => return Err(state::recovery_required()),
        })
    }
}
impl<B: Blocker> LifecycleRefusal for Refusal<B> {
    fn external_change() -> Self {
        Self::ExternalChange
    }
    fn reconfirmation_required() -> Self {
        Self::ReconfirmationRequired
    }
    fn integration_conflict(&self) -> bool {
        matches!(self, Self::Blocked(b) if b.contains(&B::INTEGRATION_CONFLICT))
    }
}

/// One stored parent row's columns, each refused as recovery when it is not
/// exactly its canonical spelling.
pub(in super::super) struct Row<'a, 'b>(pub(in super::super) &'a rusqlite::Row<'b>);
impl Row<'_, '_> {
    pub(in super::super) fn text(&self, name: &str) -> Result<String, RepositoryError> {
        self.0
            .get::<_, String>(name)
            .map_err(|_| state::recovery_required())
    }
    pub(in super::super) fn opt(&self, name: &str) -> Result<Option<String>, RepositoryError> {
        self.0
            .get::<_, Option<String>>(name)
            .map_err(|_| state::recovery_required())
    }
    pub(in super::super) fn number(&self, name: &str) -> Result<i64, RepositoryError> {
        self.0
            .get::<_, i64>(name)
            .map_err(|_| state::recovery_required())
    }
    pub(in super::super) fn attempt(&self, name: &str) -> Result<u32, RepositoryError> {
        self.number(name)?
            .try_into()
            .map_err(|_| state::recovery_required())
    }
    pub(in super::super) fn flag(&self, name: &str) -> Result<bool, RepositoryError> {
        match self.number(name)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(state::recovery_required()),
        }
    }
    pub(in super::super) fn oid(&self, name: &str) -> Result<Option<Oid>, RepositoryError> {
        self.opt(name)?
            .map(|s| {
                let o = Oid::from_str(&s).map_err(|_| state::recovery_required())?;
                if o.is_zero() || o.to_string() != s {
                    return Err(state::recovery_required());
                }
                Ok(o)
            })
            .transpose()
    }
    pub(in super::super) fn operation(&self, name: &str) -> Result<OperationId, RepositoryError> {
        operation(self.text(name)?)
    }
    pub(in super::super) fn sync_id(
        &self,
        name: &str,
    ) -> Result<Option<OperationId>, RepositoryError> {
        self.opt(name)?.map(operation).transpose()
    }
    pub(in super::super) fn item(&self) -> Result<ItemId, RepositoryError> {
        let item = self.text("item_id")?;
        let item_id: ItemId = item.parse().map_err(|_| state::recovery_required())?;
        if item_id.to_string() != item {
            return Err(state::recovery_required());
        }
        Ok(item_id)
    }
    pub(in super::super) fn expectation(&self) -> Result<[u8; 32], RepositoryError> {
        let bytes: Vec<u8> = self
            .0
            .get("expectation")
            .map_err(|_| state::recovery_required())?;
        bytes.try_into().map_err(|_| state::recovery_required())
    }
    pub(in super::super) fn category<T>(
        &self,
        name: &str,
        parse: fn(&str) -> Result<T, RepositoryError>,
    ) -> Result<Option<T>, RepositoryError> {
        self.opt(name)?.map(|s| parse(&s)).transpose()
    }
    pub(in super::super) fn refusal<B: Blocker>(
        &self,
    ) -> Result<Option<Refusal<B>>, RepositoryError> {
        let mask: Option<u32> = self
            .0
            .get("refusal_blockers")
            .map_err(|_| state::recovery_required())?;
        Refusal::decoded(self.opt("refusal_category")?.as_deref(), mask)
    }
}

/// The stored parent of one lifecycle: its kind, row decoding and schema
/// check, plus the attempt counters the runner's view does not carry.
pub(in super::super) trait ParentRecord: LifecycleRecord + Sized {
    const KIND: LifecycleKind;
    fn decode(row: &rusqlite::Row<'_>) -> Result<Self, RepositoryError>;
    fn validate_schema(db: &Connection) -> Result<(), RepositoryError>;
    fn context_attempt(&self) -> u32;
    fn primary_attempt(&self) -> u32;
    fn expectation_bytes(&self) -> [u8; 32];
}

pub(in super::super) fn read_record<R: ParentRecord>(
    db: &Connection,
    id: i64,
    op: OperationId,
) -> Result<Option<R>, RepositoryError> {
    db.query_row(
        &format!(
            "SELECT * FROM {} WHERE repository_id=?1 AND operation_id=?2",
            R::KIND.table()
        ),
        params![id, op.to_string()],
        |r| Ok(R::decode(r)),
    )
    .optional()
    .map_err(|_| state::recovery_required())?
    .transpose()
}
pub(in super::super) fn records<R: ParentRecord>(
    db: &Connection,
    id: i64,
) -> Result<Vec<R>, RepositoryError> {
    db.prepare(&format!(
        "SELECT * FROM {} WHERE repository_id=?1 ORDER BY operation_id",
        R::KIND.table()
    ))
    .and_then(|mut q| {
        q.query_map([id], |r| Ok(R::decode(r)))?
            .collect::<Result<Vec<_>, _>>()
    })
    .map_err(|_| state::recovery_required())?
    .into_iter()
    .collect()
}
/// The ID's binding as parent or as either child, in any repository.
fn bindings<R: ParentRecord>(db: &Connection, op: OperationId) -> Result<Vec<R>, RepositoryError> {
    db.prepare(&format!(
        "SELECT * FROM {} WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1",
        R::KIND.table()
    ))
    .and_then(|mut q| {
        q.query_map([op.to_string()], |row| Ok(R::decode(row)))?
            .collect::<Result<Vec<_>, _>>()
    })
    .map_err(|_| state::recovery_required())?
    .into_iter()
    .collect()
}
pub(in super::super) fn unfinished_record<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<Option<R>, RepositoryError> {
    tx
        .query_row(
            &format!("SELECT * FROM {} WHERE repository_id=?1 AND item_id=?2 AND step NOT IN ('complete','abandoned','superseded')",R::KIND.table()),
            params![id, item.to_string()],
            |r| Ok(R::decode(r)),
        )
        .optional()
        .map_err(|_| state::recovery_required())?
        .transpose()
}
/// The same-ID record, after refusing an ID bound anywhere else: another
/// table, another repository, a local journal, a comment binding or a
/// foreign reservation.
pub(in super::super) fn existing_record<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: R::Expectation,
) -> Result<Option<R>, RepositoryError>
where
    R::Expectation: PartialEq,
{
    let existing = read_record::<R>(tx, id, op)?;
    let (own, other) = (R::KIND.table(), R::KIND.other().table());
    let (action, kind) = R::KIND.stored_action();
    let collision:bool=tx.query_row(&format!("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1) OR EXISTS(SELECT 1 FROM {own} WHERE (operation_id=?1 AND repository_id!=?2) OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM {other} WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND (repository_id!=?2 OR action!='{action}' OR kind!='{kind}' OR item_id!=?3))"),params![op.to_string(),id,item.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if collision {
        return Err(mismatch(R::KIND));
    }
    if existing.as_ref().is_none_or(|r| !r.step().terminal()) {
        crate::repository::local_sync::record::require_no_pending(tx, id)?;
    }
    if let Some(r) = existing {
        if r.item_id() != item || r.expectation() != expectation {
            return Err(mismatch(R::KIND));
        }
        if let Some(reservation) = state::read_operation(tx, id, op)? {
            validate_remote_identity(tx, id, op, &reservation.target, reservation.priority)?;
        }
        return Ok(Some(r));
    }
    // A foreign parent reservation is not proof of a parent, even if its item
    // matches.
    if tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1)",
            [op.to_string()],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|_| state::recovery_required())?
    {
        return Err(mismatch(R::KIND));
    }
    Ok(None)
}
/// Process-only fence for the exact predecessor observed before unleased
/// discovery. A different parent or owner cannot be silently superseded.
#[derive(PartialEq, Eq)]
pub(in super::super) struct ReplacementGuard<R> {
    repository_id: i64,
    parent: Option<R>,
    owner: Option<(OperationId, i64, i64, RemoteOperationPhase, bool, bool, i64)>,
}
pub(in super::super) fn replacement_guard<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<ReplacementGuard<R>, RepositoryError> {
    let parent = unfinished_record::<R>(tx, id, item)?;
    let owner = parent
        .as_ref()
        .map(|parent| state::read_operation(tx, id, parent.operation_id()))
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
/// Supersedes `old`, which the caller has proved supersedable. Supersession
/// is terminal for its owner: only the parent's own slot is released,
/// atomically with replacing the parent, never a child's slot.
pub(in super::super) fn supersede<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    old: &R,
) -> Result<(), RepositoryError> {
    if let Some(remote) = state::read_operation(tx, id, old.operation_id())? {
        validate_remote_identity(tx, id, old.operation_id(), &remote.target, remote.priority)?;
        let epoch = remote
            .owner_epoch
            .checked_add(1)
            .ok_or_else(state::recovery_required)?;
        tx.execute("UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled',owner_epoch=?2,cancel_requested=0,yield_requested=0,updated_at=max(updated_at,?3) WHERE id=?1",params![remote.id,epoch,now()]).map_err(|_|state::recovery_required())?;
    }
    tx.execute(
        &format!(
            "UPDATE {} SET step='superseded',updated_at=max(updated_at,?2) WHERE operation_id=?1",
            R::KIND.table()
        ),
        params![old.operation_id().to_string(), now()],
    )
    .map_err(|_| state::recovery_required())?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ChildRole {
    Context,
    Primary,
}
/// A context child only from `confirmed`, a primary child only from
/// `integrated` or `publication_pending`, for every parent kind.
fn child_allowed(step: ParentStep, role: ChildRole) -> bool {
    match role {
        ChildRole::Context => step == ParentStep::Confirmed,
        ChildRole::Primary => matches!(
            step,
            ParentStep::Integrated | ParentStep::PublicationPending
        ),
    }
}
fn attempt_of<R: ParentRecord>(r: &R, role: ChildRole) -> (u32, Option<OperationId>) {
    match role {
        ChildRole::Context => (r.context_attempt(), r.context_sync_id()),
        ChildRole::Primary => (r.primary_attempt(), r.primary_sync_id()),
    }
}

/// The parent facts the child fence and a parent restart compare, read
/// from the parent's own table.
pub(in super::super) struct ParentFacts {
    item_id: ItemId,
    expectation: [u8; 32],
    context: (u32, Option<OperationId>),
    primary: (u32, Option<OperationId>),
    step: ParentStep,
    cancel_requested: bool,
}
impl ParentFacts {
    fn of<R: ParentRecord>(r: R) -> Self {
        Self {
            expectation: r.expectation_bytes(),
            context: attempt_of(&r, ChildRole::Context),
            primary: attempt_of(&r, ChildRole::Primary),
            step: r.step(),
            cancel_requested: r.cancel_requested(),
            item_id: r.item_id().clone(),
        }
    }
    pub(in super::super) fn read(
        db: &Connection,
        kind: LifecycleKind,
        id: i64,
        parent: OperationId,
    ) -> Result<Option<Self>, RepositoryError> {
        Ok(match kind {
            LifecycleKind::DocumentPromotion => {
                read_record::<PromotionRecord>(db, id, parent)?.map(Self::of)
            }
            LifecycleKind::TicketClosure => {
                read_record::<ClosureRecord>(db, id, parent)?.map(Self::of)
            }
        })
    }
    /// A refused or replaced parent.
    pub(in super::super) fn ended(&self) -> bool {
        matches!(self.step, ParentStep::Abandoned | ParentStep::Superseded)
    }
}

/// Private, process-only identity of a parent-issued attempt. It holds no lock
/// and grants no ownership until the reservation SQL validates the current
/// durable binding. In particular, a formerly-bound no-row ID cannot fall back
/// to an ordinary synchronization when another executor replaces its attempt.
#[derive(Clone)]
pub(in super::super) struct ChildExecution {
    kind: LifecycleKind,
    repository_id: i64,
    parent: OperationId,
    item: ItemId,
    expectation: [u8; 32],
    role: ChildRole,
    attempt: u32,
    child: OperationId,
}
pub(in super::super) fn child_execution<R: ParentRecord>(
    r: &R,
    role: ChildRole,
) -> Result<ChildExecution, RepositoryError> {
    let (attempt, child) = attempt_of(r, role);
    if attempt == 0 || r.remote_name().is_none() {
        return Err(mismatch(R::KIND));
    }
    Ok(ChildExecution {
        kind: R::KIND,
        repository_id: r.repository_id(),
        parent: r.operation_id(),
        item: r.item_id().clone(),
        expectation: r.expectation_bytes(),
        role,
        attempt,
        child: child.ok_or_else(|| mismatch(R::KIND))?,
    })
}
/// The fence's refusal names the lifecycle that issued the child attempt.
fn fence_mismatch(kind: LifecycleKind) -> RepositoryError {
    mismatch(kind)
}
impl ChildExecution {
    pub(in super::super) fn validate_request(
        &self,
        request: &crate::repository::SynchronizeRemoteRequest,
    ) -> Result<(), RepositoryError> {
        // The context target's kind is the parent's: a document context for
        // a promotion parent, a ticket context for a closure parent.
        let matches = match (&self.role, &request.target) {
            (ChildRole::Primary, crate::repository::SynchronizationTarget::Primary) => true,
            (
                ChildRole::Context,
                crate::repository::SynchronizationTarget::Context { kind, item_id },
            ) => *kind == self.kind.authoring() && item_id == &self.item,
            _ => false,
        };
        if request.operation_id != self.child || !matches {
            return Err(fence_mismatch(self.kind));
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
            return Err(fence_mismatch(self.kind));
        }
        let parent = ParentFacts::read(db, self.kind, id, self.parent)?
            .ok_or_else(|| fence_mismatch(self.kind))?;
        let (attempt, child) = match self.role {
            ChildRole::Context => parent.context,
            ChildRole::Primary => parent.primary,
        };
        if parent.item_id != self.item
            || parent.expectation != self.expectation
            || attempt != self.attempt
            || child != Some(self.child)
        {
            return Err(fence_mismatch(self.kind));
        }
        let record = state::read_operation(db, id, op)?;
        // Exact, current authoritative handoff remains valid after parent
        // progress/completion; this path can never create/restart remote work.
        if record.as_ref().is_some_and(|r| r.authority.is_some()) {
            return Ok(());
        }
        if !child_allowed(parent.step, self.role) || (parent.cancel_requested && record.is_none()) {
            return Err(fence_mismatch(self.kind));
        }
        Ok(())
    }
}

/// Authoritative completion is consumed first. Retained includes clean candidates,
/// installed conflicts and unreleased owned resolutions, regardless of outer error.
#[allow(dead_code)] // Every variant carries its row; callers read Complete's only.
pub(in super::super) enum ChildState {
    Absent,
    Complete(state::StoredRemoteOperation),
    Retained(state::StoredRemoteOperation),
    Unowned(state::StoredRemoteOperation),
}
pub(in super::super) fn child_state<R: ParentRecord>(
    db: &Connection,
    r: &R,
    role: ChildRole,
) -> Result<ChildState, RepositoryError> {
    let Some(child) = attempt_of(r, role).1 else {
        return Ok(ChildState::Absent);
    };
    let foreign:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![child.to_string(),r.repository_id()],|row|row.get(0)).map_err(|_|state::recovery_required())?;
    if foreign {
        return Err(state::recovery_required());
    }
    let Some(record) = state::read_operation(db, r.repository_id(), child)? else {
        return Ok(ChildState::Absent);
    };
    validate_remote_identity(
        db,
        r.repository_id(),
        child,
        &record.target,
        record.priority,
    )?;
    if record.authority.is_some() {
        return Ok(ChildState::Complete(record));
    }
    if reservation::retains_promotion_child(db, &record)? {
        Ok(ChildState::Retained(record))
    } else {
        Ok(ChildState::Unowned(record))
    }
}
pub(in super::super) fn next_attempt<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    role: ChildRole,
) -> Result<R, RepositoryError> {
    let r = read_record::<R>(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if !child_allowed(r.step(), role) {
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
fn mint_attempt<R: ParentRecord>(
    tx: &Transaction<'_>,
    r: &R,
    role: ChildRole,
) -> Result<R, RepositoryError> {
    let (column, count) = match role {
        ChildRole::Context => ("context_sync_id", "context_attempt"),
        ChildRole::Primary => ("primary_sync_id", "primary_attempt"),
    };
    let attempt = attempt_of(r, role)
        .0
        .checked_add(1)
        .ok_or_else(state::recovery_required)?;
    let child = OperationId::new();
    let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM document_promotions WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM ticket_closures WHERE operation_id=?1 OR context_sync_id=?1 OR primary_sync_id=?1) OR EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1)",[child.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if used {
        return Err(state::recovery_required());
    }
    tx.execute(&format!("UPDATE {} SET {column}=?2,{count}=?3,updated_at=max(updated_at,?4) WHERE operation_id=?1",R::KIND.table()),params![r.operation_id().to_string(),child.to_string(),attempt,now()]).map_err(|_|state::recovery_required())?;
    read_record::<R>(tx, r.repository_id(), r.operation_id())?.ok_or_else(state::recovery_required)
}
/// The sole back-transition, to `confirmed` with a fresh context attempt,
/// from the lifecycle's own `context_synchronized` (`synchronized`). A
/// completed context child may be replaced here; an unfinished one may not.
pub(in super::super) fn reset_context_after_primary_advance<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    synchronized: fn(&R) -> bool,
) -> Result<R, RepositoryError> {
    let r = read_record::<R>(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if !synchronized(&r)
        || r.remote_name().is_none()
        || matches!(
            child_state(tx, &r, ChildRole::Context)?,
            ChildState::Retained(_) | ChildState::Unowned(_)
        )
    {
        return Err(state::recovery_required());
    }
    tx.execute(&format!("UPDATE {} SET step='confirmed',context_oid=NULL,remote_context_oid=NULL,base_oid=NULL,integration_kind=NULL,cleanup_category=NULL WHERE operation_id=?1",R::KIND.table()),[op.to_string()]).map_err(|_|state::recovery_required())?;
    mint_attempt(tx, &r, ChildRole::Context)
}

/// Checks both parent and child identities in the same transaction as
/// reservation, across both parent tables: promotion's and closure's.
pub(in super::super) fn validate_remote_identity(
    db: &Connection,
    id: i64,
    op: OperationId,
    target: &RemoteOperationTarget,
    priority: RemoteOperationPriority,
) -> Result<(), RepositoryError> {
    remote_identity_of::<PromotionRecord>(db, id, op, target, priority)?;
    crate::repository::closure::record::validate_remote_identity(db, id, op, target, priority)
}
/// A reservation under any of one table's parent or child IDs must be
/// exactly that binding's.
pub(in super::super) fn remote_identity_of<R: ParentRecord>(
    db: &Connection,
    id: i64,
    op: OperationId,
    target: &RemoteOperationTarget,
    priority: RemoteOperationPriority,
) -> Result<(), RepositoryError> {
    for r in bindings::<R>(db, op)? {
        let expected = if r.operation_id() == op {
            R::KIND.action()
        } else if r.context_sync_id() == Some(op) {
            RemoteOperationAction::SynchronizeContext
        } else {
            RemoteOperationAction::SynchronizePrimary
        };
        if r.repository_id() != id
            || priority != RemoteOperationPriority::Manual
            || target.action() != expected
            || Some(target.remote_name()) != r.remote_name()
            || expected != RemoteOperationAction::SynchronizePrimary
                && target.item() != Some((R::KIND.authoring(), r.item_id()))
        {
            return Err(mismatch(R::KIND));
        }
    }
    Ok(())
}
/// The local journal may never begin under a parent ID, and under a child
/// ID only as that child's exact authoritative refresh hand-off.
pub(in super::super) fn validate_local_identity(
    kind: LifecycleKind,
    db: &Connection,
    root: &str,
    action: &str,
    op: &str,
    target: &str,
) -> Result<(), RepositoryError> {
    let table = kind.table();
    if !crate::repository::recovery::has_table(db, table).map_err(|_| state::recovery_required())? {
        return Ok(());
    }
    let parent: bool = db
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE operation_id=?1)"),
            [op],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if parent {
        return Err(mismatch(kind));
    }
    let child: Option<i64> = db
        .query_row(
            &format!(
                "SELECT repository_id FROM {table} WHERE context_sync_id=?1 OR primary_sync_id=?1"
            ),
            [op],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| state::recovery_required())?;
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
            return Err(mismatch(kind));
        }
    }
    Ok(())
}
pub(in super::super) fn audit<R: ParentRecord>(db: &Connection) -> Result<(), RepositoryError> {
    R::validate_schema(db)?;
    let (own, other) = (R::KIND.table(), R::KIND.other().table());
    if db
        .prepare(&format!("PRAGMA foreign_key_check({own})"))
        .and_then(|mut q| q.exists([]))
        .map_err(|_| state::recovery_required())?
    {
        return Err(state::recovery_required());
    }
    let ids = db
        .prepare(&format!("SELECT DISTINCT repository_id FROM {own}"))
        .and_then(|mut q| {
            q.query_map([], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| state::recovery_required())?;
    for id in ids {
        for r in records::<R>(db, id)? {
            let (context, primary) = (
                r.context_sync_id().map(|v| v.to_string()),
                r.primary_sync_id().map(|v| v.to_string()),
            );
            let local:bool=db.query_row(&format!("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid IN (?1,?2,?3) OR synchronization_ulid IN (?1,?2,?3)) OR EXISTS(SELECT 1 FROM {own} s WHERE s.operation_id!=?1 AND (s.operation_id IN (?2,?3) OR s.context_sync_id IN (?1,?2,?3) OR s.primary_sync_id IN (?1,?2,?3))) OR EXISTS(SELECT 1 FROM {other} o WHERE o.operation_id IN (?1,?2,?3) OR o.context_sync_id IN (?1,?2,?3) OR o.primary_sync_id IN (?1,?2,?3))"),params![r.operation_id().to_string(),context,primary],|r|r.get(0)).map_err(|_|state::recovery_required())?;
            if local {
                return Err(state::recovery_required());
            }
            let foreign:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![r.operation_id().to_string(),id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if foreign {
                return Err(state::recovery_required());
            }
            if let Some(remote) = state::read_operation(db, id, r.operation_id())? {
                validate_remote_identity(db, id, r.operation_id(), &remote.target, remote.priority)
                    .map_err(|_| state::recovery_required())?;
            }
            for role in [ChildRole::Context, ChildRole::Primary] {
                child_state(db, &r, role).map_err(|_| state::recovery_required())?;
            }
            let local_children=db.prepare("SELECT root_path,action,coalesce(target,''),operation_ulid FROM operation_records WHERE operation_ulid IN (?1,?2)").and_then(|mut q|q.query_map(params![context,primary],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|_|state::recovery_required())?;
            for (root, action, target, child) in local_children {
                validate_local_identity(R::KIND, db, &root, &action, &child, &target)
                    .map_err(|_| state::recovery_required())?;
            }
        }
    }
    Ok(())
}

/// Completion retires the merged context (promotion's synchronized context;
/// a closure's checkpoint, or the adopted synchronization over it) in one
/// transaction with the parent's own reservation.
pub(in super::super) fn complete_record<R: ParentRecord>(
    tx: &Transaction<'_>,
    service: &RepositoryService,
    id: i64,
    op: OperationId,
    evidence: &RetirementEvidence<'_>,
    owner: Option<&RemoteReservation>,
) -> Result<(), RepositoryError> {
    let r = read_record::<R>(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step() != ParentStep::BranchDeleted
        || !evidence.matches(
            R::KIND.authoring(),
            op,
            r.item_id(),
            r.merged_context().ok_or_else(state::recovery_required)?,
        )
        || owner.is_some() && r.remote_name().is_none()
        || r.remote_deletion_required() && owner.is_none()
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
    if configured.as_deref() != r.remote_name() {
        return Err(state::recovery_required());
    }
    if state::read_operation(tx, id, op)?.is_some() != owner.is_some() {
        return Err(state::recovery_required());
    }
    if let Some(owner) = owner {
        if owner.operation_id() != op {
            return Err(mismatch(R::KIND));
        }
        let remote = reservation::owned(service, tx, id, owner)?;
        validate_remote_identity(tx, id, op, &remote.target, remote.priority)?;
    }
    cleanup::retire_context(tx, id, evidence)?;
    crate::repository::recovery::invalidate_index_observations(tx, id)?;
    // Completion may be followed immediately by process exit (F11). Its
    // index-only handoff must be durable in this same transaction so replay
    // cannot mistake pre-cleanup discovery for an already-refreshed outcome.
    tx.execute(
        "UPDATE repositories SET refresh_required=1 WHERE id=?1",
        [id],
    )
    .map_err(|_| state::recovery_required())?;
    tx.execute(&format!("UPDATE {} SET step='complete',cleanup_category=NULL,cancel_requested=0,updated_at=max(updated_at,?2) WHERE operation_id=?1",R::KIND.table()),params![op.to_string(),now()]).map_err(|_|state::recovery_required())?;
    if let Some(owner) = owner {
        reservation::finish_parent(R::KIND, service, tx, id, owner)?;
    }
    Ok(())
}
/// Inspect actual committed parent state after an uncertain commit result.
/// Git-ref absence is never used as completion/retirement evidence here.
pub(in super::super) fn reconcile_completion<R: ParentRecord>(
    db: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    let r = read_record::<R>(db, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step() != ParentStep::Complete {
        return Ok(false);
    }
    let remote = state::read_operation(db, id, op)?;
    if let Some(remote) = remote {
        if r.remote_name().is_none() {
            return Err(state::recovery_required());
        }
        validate_remote_identity(db, id, op, &remote.target, remote.priority)?;
        if remote.phase != RemoteOperationPhase::Completed {
            return Err(state::recovery_required());
        }
    } else if r.remote_deletion_required() {
        return Err(state::recovery_required());
    }
    // Retirement/history are mutable projections: a later first publication can
    // remove retirement and create new history. The parent's committed complete
    // row is the durable authority, not those projections or current Git absence.
    Ok(true)
}
pub(in super::super) fn request_cancellation<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    let Some(r) = read_record::<R>(tx, id, op)? else {
        return Ok(false);
    };
    if r.step().terminal() {
        return Ok(false);
    }
    tx.execute(
        &format!(
            "UPDATE {} SET cancel_requested=1,updated_at=max(updated_at,?2) WHERE operation_id=?1",
            R::KIND.table()
        ),
        params![op.to_string(), now()],
    )
    .map_err(|_| state::recovery_required())?;
    tx.execute("UPDATE remote_operation_records SET cancel_requested=1,updated_at=max(updated_at,?5) WHERE repository_id=?1 AND operation_ulid IN (?2,?3,?4) AND phase NOT IN ('completed','interrupted','cancelled','failed')",params![id,op.to_string(),r.context_sync_id().map(|v|v.to_string()),r.primary_sync_id().map(|v|v.to_string()),now()]).map_err(|_|state::recovery_required())?;
    Ok(true)
}
/// Explicit same-ID continuation consumes a parent's inter-step stop. Child
/// cancellation/ownership is reconciled independently before replacing any ID.
pub(in super::super) fn clear_cancellation<R: ParentRecord>(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    let r = read_record::<R>(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.step().terminal() {
        return Err(state::recovery_required());
    }
    tx.execute(
        &format!(
            "UPDATE {} SET cancel_requested=0,updated_at=max(updated_at,?2) WHERE operation_id=?1",
            R::KIND.table()
        ),
        params![op.to_string(), now()],
    )
    .map_err(|_| state::recovery_required())?;
    Ok(())
}
