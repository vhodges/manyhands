//! The closure observation: the promotion observation over a ticket context,
//! plus pending local work (C12), the closure effect, the integration table,
//! the post-closure ticket check (D1, C10) and identity (C2). It never takes
//! the repository lease, writes Git or SQL, or opens the network.
use super::{
    ClosureAction, ClosureBlocker, ClosureExpectation, ClosureWithdrawalPreflight, NewBranchFacts,
    PlannedClosure, PreviewTicketClosureRequest, TicketClosurePreflight, TicketClosurePreview,
    checkpoint::{ClosureBuild, ClosureTarget, MergeHandle, build_closure},
};
use crate::{
    canonical::{self, CanonicalItem, Integration, ItemId, Ticket},
    repository::{
        Blocker, CommitIdentity, ConfirmedCommitIdentity, ExpectedConfiguration, PathChange,
        PlannedCleanup, PlannedIntegration, PlannedPublication, RemoteRefPlan, RemoteSnapshot,
        RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
        SynchronizationError, SynchronizationTarget,
        bridges::abandon::authoring_item,
        context_retirement::proof,
        promotion::{
            lifecycle::LifecycleKind,
            observe::{
                self as shared, ChangeEvidence, Cleanliness, ContextPresence, ObservationSeal,
                PlannedResult, PreviewFacts, PrimaryPresence, RemoteContextHistory, digest,
            },
        },
        remote::{
            state,
            sync::{self, CommittingIdentity},
        },
    },
};
use git2::Oid;
use rusqlite::OptionalExtension;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, UtcOffset};

const KIND: LifecycleKind = LifecycleKind::TicketClosure;

/// The closure digest's domain; promotion's (`manyhands promotion v1`) is
/// unchanged.
pub(in super::super) const CLOSURE_DIGEST_DOMAIN: &[u8] = b"manyhands closure v1";

/// The pull-request close's digest domain: it binds the action and the
/// head, and nothing of an integration.
const PULL_REQUEST_CLOSURE_DIGEST_DOMAIN: &[u8] = b"manyhands pull request closure v1";

/// The ticket's one canonical path.
pub(in super::super) fn ticket_path(item: &ItemId) -> PathBuf {
    PathBuf::from(format!(".manyhands/tickets/{item}/ticket.md"))
}

/// `closed_by` as the closure records it: `"<name> <<email>>"`.
pub(in super::super) fn closed_by_text(identity: &CommitIdentity) -> String {
    format!("{} <{}>", identity.name, identity.email)
}

/// Why `closed_ticket_bytes` or `withdrawn_ticket_bytes` refuses a ticket.
/// Categories only: no content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ClosureContentError {
    /// The bytes are not UTF-8 text.
    NotText,
    /// The bytes are not a conforming canonical ticket.
    NotATicket,
    /// The ticket already carries closure fields.
    AlreadyClosed,
    /// `closed_at` is not whole seconds in UTC.
    InvalidTime,
    /// `closed_by` is empty or contains NUL.
    InvalidClosedBy,
    /// Canonical re-serialization would change a field, an unknown key or
    /// the body beyond the two closure fields.
    NotPreserved,
    /// The ticket holds no closure to remove.
    NotClosed,
}

/// The closed ticket's bytes (C10): parse, set `closed_at`/`closed_by`,
/// serialize canonically, re-parse, and require every parsed field, unknown
/// key and the body to equal the original apart from the two closure fields.
/// The comparison is made after canonical normalization (R-T0-2): the
/// relationship normalization every product edit applies (sorted `deps`, a
/// dropped null `parent` or empty `deps`) is not a change. Preview and
/// execution both call this one function.
pub(in super::super) fn closed_ticket_bytes(
    original: &[u8],
    closed_at: OffsetDateTime,
    closed_by: &str,
) -> Result<Vec<u8>, ClosureContentError> {
    use ClosureContentError::*;
    let (path, ticket) = parsed_ticket(original)?;
    if ticket.closed_at.is_some() || ticket.closed_by.is_some() {
        return Err(AlreadyClosed);
    }
    if closed_at.offset() != UtcOffset::UTC || closed_at.nanosecond() != 0 {
        return Err(InvalidTime);
    }
    if closed_by.is_empty() || closed_by.contains('\0') {
        return Err(InvalidClosedBy);
    }
    with_closure(&path, &ticket, Some((closed_at, closed_by)))
}

/// A ticket's bytes with its closure fields replaced (R-30, Part G): as
/// `closed_ticket_bytes`, for a ticket that may already hold a closure. The
/// ticket is parsed, given `closed_at` and `closed_by`, and serialized
/// canonically, and every other parsed field, unknown key and the body must
/// be as they were. The closure is given as it is read from another ticket,
/// whatever it holds, so it is not validated as a new one is.
pub(in super::super) fn closure_replaced_bytes(
    original: &[u8],
    closed_at: OffsetDateTime,
    closed_by: &str,
) -> Result<Vec<u8>, ClosureContentError> {
    let (path, ticket) = parsed_ticket(original)?;
    with_closure(&path, &ticket, Some((closed_at, closed_by)))
}

/// The ticket's bytes with its closure removed (Part E): parsed, both
/// fields cleared, serialized canonically, and every other parsed field,
/// unknown key and the body as they were. `NotClosed` for an open ticket.
pub(in super::super) fn withdrawn_ticket_bytes(
    original: &[u8],
) -> Result<Vec<u8>, ClosureContentError> {
    let (path, ticket) = parsed_ticket(original)?;
    if ticket.closed_at.is_none() && ticket.closed_by.is_none() {
        return Err(ClosureContentError::NotClosed);
    }
    with_closure(&path, &ticket, None)
}

/// The ticket the bytes hold, with its canonical path.
fn parsed_ticket(original: &[u8]) -> Result<(PathBuf, Ticket), ClosureContentError> {
    use ClosureContentError::*;
    let source = std::str::from_utf8(original).map_err(|_| NotText)?;
    let id = canonical::front_matter(source)
        .and_then(|values| values.get("id")?.as_str()?.parse::<ItemId>().ok())
        .ok_or(NotATicket)?;
    let path = ticket_path(&id);
    let ticket = parse_ticket_at(&path, source)?;
    Ok((path, ticket))
}

fn parse_ticket_at(path: &Path, source: &str) -> Result<Ticket, ClosureContentError> {
    match canonical::parse_item(path, source) {
        Ok(CanonicalItem::Ticket(ticket)) => Ok(ticket),
        _ => Err(ClosureContentError::NotATicket),
    }
}

/// `ticket` normalized, given the closure (`None` clears both fields) and
/// serialized, which must read back as exactly that, and as `ticket` in
/// every other field.
fn with_closure(
    path: &Path,
    ticket: &Ticket,
    closure: Option<(OffsetDateTime, &str)>,
) -> Result<Vec<u8>, ClosureContentError> {
    use ClosureContentError::*;
    let serialize = |ticket: &Ticket| {
        canonical::serialize_item(&CanonicalItem::Ticket(ticket.clone())).map_err(|_| NotPreserved)
    };
    let normalized = parse_ticket_at(path, &serialize(ticket)?).map_err(|_| NotPreserved)?;
    if !preserved(ticket, &normalized) {
        return Err(NotPreserved);
    }
    let mut closed = normalized;
    closed.closed_at = closure.map(|(closed_at, _)| closed_at);
    closed.closed_by = closure.map(|(_, closed_by)| closed_by.to_owned());
    let bytes = serialize(&closed)?;
    if parse_ticket_at(path, &bytes).map_err(|_| NotPreserved)? != closed {
        return Err(NotPreserved);
    }
    Ok(bytes.into_bytes())
}

/// Every parsed field, the body and every unknown key equal. The
/// relationship keys may only take their canonical form (R-T0-2): `deps`
/// compares as a sorted set without repeats (an empty or null `deps` is
/// none), and `parent` compares with null as absent.
fn preserved(original: &Ticket, normalized: &Ticket) -> bool {
    use serde_yaml::Value;
    const RELATIONSHIPS: [&str; 2] = ["deps", "parent"];
    let others = |ticket: &Ticket| {
        ticket
            .unknown
            .iter()
            .filter(|(key, _)| !key.as_str().is_some_and(|key| RELATIONSHIPS.contains(&key)))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>()
    };
    let deps = |ticket: &Ticket| -> BTreeSet<String> {
        match ticket.unknown.get("deps") {
            None | Some(Value::Null) => BTreeSet::new(),
            Some(Value::Sequence(entries)) => entries.iter().map(|e| format!("{e:?}")).collect(),
            Some(other) => BTreeSet::from([format!("{other:?}")]),
        }
    };
    let parent = |ticket: &Ticket| match ticket.unknown.get("parent") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.clone()),
    };
    let mut before = others(original);
    let mut after = others(normalized);
    let order =
        |a: &(Value, Value), b: &(Value, Value)| format!("{:?}", a.0).cmp(&format!("{:?}", b.0));
    before.sort_by(order);
    after.sort_by(order);
    original.id == normalized.id
        && original.title == normalized.title
        && original.ticket_type == normalized.ticket_type
        && original.status == normalized.status
        && original.project == normalized.project
        && original.team == normalized.team
        && original.closed_at == normalized.closed_at
        && original.closed_by == normalized.closed_by
        && original.body == normalized.body
        && before == after
        && deps(original) == deps(normalized)
        && parent(original) == parent(normalized)
}

/// The verb a closure observation observes for: a close (from Task 9 its
/// situation may make it a new branch) or a withdraw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Verb {
    Close,
    Withdraw,
}

/// A pull-request close, and every withdraw, takes the branch record's
/// observation (Part D, Consent and dispatch).
fn branch_observed(mode: Integration, verb: Verb) -> bool {
    mode == Integration::PullRequest || verb == Verb::Withdraw
}

/// The observation every closure stage consumes: preview, the post-child
/// re-check and the leased revalidation. It carries no authority to mutate
/// Git or the registry. It deliberately has no `Debug`: `closure` and
/// `identity` hold personal data.
pub(in super::super) struct ClosureObservation {
    pub(in super::super) item_id: ItemId,
    pub(in super::super) ticket_path: PathBuf,
    pub(in super::super) context_oid: Oid,
    /// The integration base: the tracking primary with a remote, else the
    /// local primary.
    pub(in super::super) base: Oid,
    pub(in super::super) remote: Option<String>,
    /// The pre-closure change set, with the T2-1 mode evidence.
    pub(in super::super) changes: Vec<ChangeEvidence>,
    pub(in super::super) closure: PlannedClosure,
    pub(in super::super) integration: PlannedIntegration,
    /// A context child runs: a publication remote and a context that is not
    /// contained in the base.
    pub(in super::super) needs_context_child: bool,
    pub(in super::super) cleanup: PlannedCleanup,
    /// The selected identity of a `Record` closure, a withdraw or a new
    /// branch (its commit's author); `None` for `Existing`, whose merge
    /// signer is resolved at execution, and for a withdraw's placeholder
    /// closure (ruling 35), which has no commit to sign.
    #[cfg_attr(not(test), allow(dead_code))]
    // Observed for the tests; execution re-selects under the lease.
    pub(in super::super) identity: Option<CommittingIdentity>,
    pub(in super::super) expectation: ClosureExpectation,
    pub(in super::super) context_branch: String,
    pub(in super::super) primary_branch: String,
    pub(in super::super) primary_oid: Oid,
    pub(in super::super) tracking_primary: Option<Oid>,
    pub(in super::super) tracking_context: Option<Oid>,
    /// The mode this was observed in and the action it observes. A branch
    /// observation's `base` is the tracking primary, its `changes` are empty
    /// and its `cleanup` plans nothing, and `integration` is read only as
    /// "the context child runs": nothing integrates.
    pub(in super::super) path: (Integration, ClosureAction),
    /// A new branch's old tip, retirement and unheld entries (Part F); `None`
    /// for every other path. For a new branch `context_oid` is the old tip,
    /// or the tracking primary when the ticket has no context, so this says
    /// whether there is one.
    pub(in super::super) new_branch: Option<NewBranchFacts>,
}
impl ClosureObservation {
    /// The head the consent binds: the old context's tip for a new branch
    /// (none without a context), the context's tip otherwise.
    pub(in super::super) fn consented_head(&self) -> Option<Oid> {
        match &self.new_branch {
            Some(facts) => facts.old_tip,
            None => Some(self.context_oid),
        }
    }
    pub(in super::super) fn preview(&self) -> TicketClosurePreview {
        TicketClosurePreview {
            item_id: self.item_id.clone(),
            ticket_path: self.ticket_path.clone(),
            context_branch: self.context_branch.clone(),
            context_oid: self.context_oid,
            primary_branch: self.primary_branch.clone(),
            primary_oid: self.primary_oid,
            publication_remote: self.remote.clone(),
            tracking_primary: self.tracking_primary,
            tracking_context: self.tracking_context,
            changes: self.changes.iter().map(|c| c.change.clone()).collect(),
            closure: self.closure.clone(),
            integration: self.integration,
            publication: if self.remote.is_some() {
                PlannedPublication::Publish
            } else {
                PlannedPublication::PendingNoRemote
            },
            cleanup: self.cleanup,
            expectation: self.expectation,
            path: self.path,
            new_branch: self.new_branch.clone(),
        }
    }
}

/// A refused observation: every blocker, and the identity configuration
/// exactly when the only blocker is `IdentityRequired`.
pub(in super::super) struct ClosureRefusal {
    pub(in super::super) blockers: Vec<ClosureBlocker>,
    pub(in super::super) identity_configuration: Option<ExpectedConfiguration>,
    /// Why a withdraw offers nothing to confirm; `blockers` is then empty.
    pub(in super::super) unoffered: Option<Unoffered>,
}
impl ClosureRefusal {
    fn unoffered(reason: Unoffered) -> Self {
        Self {
            blockers: Vec::new(),
            identity_configuration: None,
            unoffered: Some(reason),
        }
    }
}

/// Why a withdraw offers nothing to confirm. Neither is a blocker: a
/// close's observation never gives one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Unoffered {
    /// Neither the context's ticket nor its tracking ref's holds a closure.
    NothingToWithdraw,
    /// No publication remote: a withdraw's children publish.
    PublicationRemoteRequired,
}

fn into_preflight(observed: &Result<ClosureObservation, ClosureRefusal>) -> TicketClosurePreflight {
    match observed {
        Ok(observation) => TicketClosurePreflight::Ready(observation.preview()),
        Err(refusal) => TicketClosurePreflight::Blocked {
            blockers: refusal.blockers.clone(),
            identity_configuration: refusal.identity_configuration,
        },
    }
}

/// Discovery facts, gathered only outside the repository lease.
struct DiscoveryFacts {
    not_ticket: bool,
    ticket_path: Option<PathBuf>,
}

/// Pending local work naming the ticket (C12), read in the same registry
/// transaction as the remote snapshot.
#[derive(Clone, Default)]
struct PendingWork {
    local_operation: bool,
    comment_publication: bool,
    /// A branch observation only (a pull-request close, or a withdraw): a
    /// live synchronization of the ticket's context that no unfinished
    /// closure record of it owns.
    synchronization: Option<String>,
    /// A branch observation only: an unfinished record of the ticket that a
    /// new request could not supersede holds it (decision 33).
    record_unfinished: bool,
    /// A pull-request close with a publication remote only: what the remote
    /// history says of the ticket's branch, for a new branch (Part F).
    history: Option<NewBranchHistory>,
}

/// What a pull-request close reads of the remote history of the ticket's
/// branch, in the registry transaction of its observation: its deletion's
/// evidence, and the commit of every publication the push-absence
/// classification counts (`None` where one is not recorded).
#[derive(Clone)]
struct NewBranchHistory {
    evidence: sync::RepublishEvidence,
    counted: Vec<Option<Oid>>,
    /// A close's own context child found the branch gone at the push
    /// endpoint (see `close_found_deleted`).
    found_deleted: bool,
}

/// Prepared outside the lease (discovery), revalidated inside it without
/// discovery (Task 6 SPEC P2), as promotion's `PreparedObservation`.
pub(in super::super) struct PreparedClosureObservation {
    pub(in super::super) observed: Result<ClosureObservation, ClosureRefusal>,
    pub(in super::super) configuration: canonical::RepositoryConfig,
    /// The mode it was observed in, which a revalidation keeps.
    mode: Integration,
    /// The verb it observed for, which a revalidation keeps as it keeps the
    /// mode.
    verb: Verb,
    pub(in super::super) repository_id: i64,
    seal: ObservationSeal,
    facts: DiscoveryFacts,
    confirmation: Option<ConfirmedCommitIdentity>,
}
impl PreparedClosureObservation {
    pub(in super::super) fn preflight(&self) -> TicketClosurePreflight {
        into_preflight(&self.observed)
    }
    /// A withdraw's preflight: a close's two answers, and the two with
    /// nothing to confirm.
    fn withdrawal_preflight(&self) -> ClosureWithdrawalPreflight {
        match &self.observed {
            Ok(observation) => ClosureWithdrawalPreflight::Ready(observation.preview()),
            Err(ClosureRefusal {
                unoffered: Some(Unoffered::NothingToWithdraw),
                ..
            }) => ClosureWithdrawalPreflight::NothingToWithdraw,
            Err(ClosureRefusal {
                unoffered: Some(Unoffered::PublicationRemoteRequired),
                ..
            }) => ClosureWithdrawalPreflight::PublicationRemoteRequired,
            Err(refusal) => ClosureWithdrawalPreflight::Blocked {
                blockers: refusal.blockers.clone(),
                identity_configuration: refusal.identity_configuration,
            },
        }
    }
}

pub(super) fn preview(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
) -> Result<TicketClosurePreflight, RepositoryError> {
    Ok(prepare_observation(service, request)?.preflight())
}

pub(super) fn withdrawal_preview(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
) -> Result<ClosureWithdrawalPreflight, RepositoryError> {
    Ok(prepare_observation_for(service, request, None, Verb::Withdraw)?.withdrawal_preflight())
}

/// Outside the lease: the item's preview facts, then a full observation, in
/// the configuration's mode.
pub(in super::super) fn prepare_observation(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
) -> Result<PreparedClosureObservation, RepositoryError> {
    prepare_observation_in(service, request, None)
}

/// `prepare_observation` in `mode`, `None` being the configuration's (R25).
/// A new request observes in the configuration's mode; a request that
/// continues a record observes in the record's, so the record continues as it
/// began after a switch.
pub(in super::super) fn prepare_observation_in(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
    mode: Option<Integration>,
) -> Result<PreparedClosureObservation, RepositoryError> {
    prepare_observation_for(service, request, mode, Verb::Close)
}

/// `prepare_observation_in` for `verb`: a withdraw observes in either mode
/// as a branch record does.
pub(in super::super) fn prepare_observation_for(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
    mode: Option<Integration>,
    verb: Verb,
) -> Result<PreparedClosureObservation, RepositoryError> {
    observe_for_at(service, request, mode, verb, now())
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("zero nanoseconds are valid")
}

fn observe_for_at(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
    mode: Option<Integration>,
    verb: Verb,
    at: OffsetDateTime,
) -> Result<PreparedClosureObservation, RepositoryError> {
    let item = &request.item_id;
    let (repository, root, config) = shared::open_observation(KIND, service, &request.root)?;
    let mode = mode.unwrap_or_else(|| config.integration());
    let plan = history_plan(&config, mode, verb, &root)?;
    // One read-only registry transaction: history, conflicts and pending work.
    let (repository_id, snapshot, pending_conflict, pending) =
        shared::read_view_with(service, &root, |tx, id| {
            pending_work(
                tx,
                id,
                item,
                branch_observed(mode, verb),
                plan.as_ref().map(|plan| (root.as_path(), plan)),
            )
        })?;
    let (facts, seal) = ObservationSeal::discover(KIND, &repository, &root, &config, item, || {
        shared::item_preview_facts(KIND, service, &root, item)
    })?;
    let PreviewFacts {
        kind_mismatch,
        item_path,
    } = facts?;
    let facts = DiscoveryFacts {
        not_ticket: kind_mismatch,
        ticket_path: item_path,
    };
    let observed = observe_git(
        &repository,
        &root,
        &config,
        item,
        &snapshot,
        pending_conflict,
        pending,
        &facts,
        request.confirmed_identity.as_ref(),
        mode,
        verb,
        at,
    )?;
    Ok(PreparedClosureObservation {
        observed,
        configuration: config,
        mode,
        verb,
        repository_id,
        seal,
        facts,
        confirmation: request.confirmed_identity.clone(),
    })
}

/// Inside the lease: no discovery; the guard, the registration and a fresh
/// Git observation, including pending work and identity, all re-read.
pub(in super::super) fn revalidate_under_lease(
    prepared: &PreparedClosureObservation,
    service: &RepositoryService,
    repository: &git2::Repository,
) -> Result<Result<ClosureObservation, Vec<ClosureBlocker>>, RepositoryError> {
    Ok(
        revalidate_observation_under_lease(prepared, service, repository)?
            .map_err(|refusal| refusal.blockers),
    )
}

/// `revalidate_under_lease`, with the whole refusal: a withdraw's `ready`
/// reads why nothing was offered.
pub(in super::super) fn revalidate_observation_under_lease(
    prepared: &PreparedClosureObservation,
    service: &RepositoryService,
    repository: &git2::Repository,
) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError> {
    let fresh = prepared.seal.reopen(repository, &prepared.configuration)?;
    let item = prepared.seal.item();
    let root = prepared.seal.root();
    let plan = history_plan(&prepared.configuration, prepared.mode, prepared.verb, root)?;
    let (id, snapshot, pending_conflict, pending) =
        shared::read_view_with(service, root, |tx, id| {
            pending_work(
                tx,
                id,
                item,
                branch_observed(prepared.mode, prepared.verb),
                plan.as_ref().map(|plan| (root, plan)),
            )
        })?;
    if id != prepared.repository_id {
        return Err(shared::changed());
    }
    observe_git(
        &fresh,
        prepared.seal.root(),
        &prepared.configuration,
        item,
        &snapshot,
        pending_conflict,
        pending,
        &prepared.facts,
        prepared.confirmation.as_ref(),
        prepared.mode,
        prepared.verb,
        now(),
    )
}

/// C12. `operation_records.item_id` is null for most lifecycle rows, and for
/// every row an older build wrote: a row's ticket comes from its target
/// (R-T0-4), parsed by the abandon bridge's own parser. A comment binding is
/// unfinished while its local row or its publication's remote row is.
///
/// A branch observation (`branch`: a pull-request close, or a withdraw) also
/// reads a live synchronization of the ticket's context, which the direct
/// close meets only at its confirm. The unfinished record's own children are
/// left out: the binding's `held` answers for those, and a foreign
/// synchronization alone is `busy` (R23).
///
/// `history` is the root and the plan of a pull-request close with a
/// publication remote, whose new-branch situations read the remote history
/// of the ticket's branch in this transaction (Part F).
fn pending_work(
    tx: &rusqlite::Transaction<'_>,
    repository_id: i64,
    item: &ItemId,
    branch: bool,
    history: Option<(&Path, &RemoteRefPlan)>,
) -> Result<PendingWork, RepositoryError> {
    let item_text = item.to_string();
    let mut statement = tx
        .prepare(
            "SELECT target FROM operation_records
              WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1)
                AND (state!='completed' OR completed_step='comment_registration_pending')",
        )
        .map_err(RepositoryError::sqlite)?;
    let targets = statement
        .query_map([repository_id], |row| row.get::<_, Option<String>>(0))
        .map_err(RepositoryError::sqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(RepositoryError::sqlite)?;
    let local_operation = targets
        .iter()
        .any(|target| authoring_item(target.as_deref()).as_deref() == Some(item_text.as_str()))
        || super::super::local_sync::record::unfinished_context(tx, repository_id, item)?;
    let comment_publication = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM comment_publication_bindings b
                JOIN operation_records o ON o.id=b.local_record_id
               WHERE b.kind='ticket' AND b.item_ulid=?2
                 AND o.root_path=(SELECT root_path FROM repositories WHERE id=?1)
                 AND ((o.state!='completed' OR o.completed_step='comment_registration_pending')
                      OR EXISTS(SELECT 1 FROM remote_operation_records r
                                 WHERE r.repository_id=?1 AND r.operation_ulid=b.synchronization_ulid
                                   AND (r.phase NOT IN ('completed','interrupted','cancelled','failed')
                                        OR (r.action IN ('synchronize_context','synchronize_primary')
                                            AND r.phase IN ('interrupted','failed'))
                                        OR r.index_pending=1
                                        OR r.reconciliation_required=1))))",
            rusqlite::params![repository_id, item_text],
            |row| row.get::<_, bool>(0),
        )
        .map_err(RepositoryError::sqlite)?;
    let synchronization = if branch {
        foreign_synchronization(tx, repository_id, item)?
    } else {
        None
    };
    let history = history
        .map(|(root, plan)| {
            let (evidence, counted, newest_proven) =
                sync::republish_evidence_in(tx, repository_id, root, plan, KIND.authoring(), item)?;
            let found_deleted = close_found_deleted(tx, repository_id, item, newest_proven)?;
            Ok::<_, RepositoryError>(NewBranchHistory {
                evidence,
                counted,
                found_deleted,
            })
        })
        .transpose()?;
    Ok(PendingWork {
        local_operation,
        comment_publication,
        synchronization,
        record_unfinished: branch && super::record::holds_new_requests(tx, repository_id, item)?,
        history,
    })
}

/// Whether the push endpoint's finding, as a close recorded it, is the
/// deletion of the ticket's context (ruling 51). A published context that was
/// deleted before any fetch advertised it has no fetch to say so, so the
/// evidence is the ticket's newest closure record (the latest by row order,
/// whatever its mode and action) when all three hold:
///
/// 1. it was abandoned by its context child with the answer
///    `context_remote_branch_deleted` (D6);
/// 2. its context child's journal row is newer than the newest proven push of
///    the context (`newest_proven`, a journal row id; a context with no proven
///    push has nothing it is newer than);
/// 3. no fetch since has recorded the branch present: a fetch that did leaves
///    an `observed` row, which makes the evidence of the deletion `None`, so
///    the caller asks only when the evidence reads `Pushed`.
///
/// A read of existing records: nothing of the remote state changes.
fn close_found_deleted(
    tx: &rusqlite::Transaction<'_>,
    repository_id: i64,
    item: &ItemId,
    newest_proven: Option<i64>,
) -> Result<bool, RepositoryError> {
    tx.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM (SELECT step, answer_code, context_sync_id FROM ticket_closures
                            WHERE repository_id=?1 AND item_id=?2 ORDER BY rowid DESC LIMIT 1) c
              JOIN remote_operation_records r
                ON r.repository_id=?1 AND r.operation_ulid=c.context_sync_id
             WHERE c.step='abandoned' AND c.answer_code='context_remote_branch_deleted'
               AND r.id>?3)",
        rusqlite::params![repository_id, item.to_string(), newest_proven.unwrap_or(0)],
        |row| row.get::<_, bool>(0),
    )
    .map_err(RepositoryError::sqlite)
}

/// The plan a pull-request close reads the remote history by, when it has a
/// publication remote.
fn history_plan(
    config: &canonical::RepositoryConfig,
    mode: Integration,
    verb: Verb,
    root: &Path,
) -> Result<Option<RemoteRefPlan>, RepositoryError> {
    if mode != Integration::PullRequest || verb != Verb::Close {
        return Ok(None);
    }
    config
        .publication_remote
        .as_ref()
        .map(|remote| {
            RemoteRefPlan::from_configuration(remote, &config.primary_branch)
                .map_err(|_| shared::unavailable(root))
        })
        .transpose()
}

/// A live synchronization of the ticket's context that no unfinished closure
/// record of it owns: the row's operation ID.
pub(super) fn foreign_synchronization(
    db: &rusqlite::Connection,
    repository_id: i64,
    item: &ItemId,
) -> Result<Option<String>, RepositoryError> {
    db.query_row(
        "SELECT operation_ulid FROM remote_operation_records
          WHERE repository_id=?1 AND action='synchronize_context' AND kind='ticket' AND item_id=?2
            AND phase NOT IN ('completed','interrupted','cancelled','failed')
            AND operation_ulid NOT IN (
                SELECT context_sync_id FROM ticket_closures
                 WHERE repository_id=?1 AND item_id=?2
                   AND step NOT IN ('complete','abandoned','superseded') AND context_sync_id IS NOT NULL
                UNION
                SELECT primary_sync_id FROM ticket_closures
                 WHERE repository_id=?1 AND item_id=?2
                   AND step NOT IN ('complete','abandoned','superseded') AND primary_sync_id IS NOT NULL)
          LIMIT 1",
        rusqlite::params![repository_id, item.to_string()],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(RepositoryError::sqlite)
}

/// The answer to a foreign synchronization of the ticket's context: busy,
/// naming the row.
pub(super) fn foreign_synchronization_busy(root: &Path, operation: &str) -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::Read,
        Some(root.to_owned()),
        RepositoryErrorKind::RepositoryBusy,
        "another synchronization of the ticket's context is unfinished",
    )
    .with_blocker(Blocker::remote(operation))
}

/// A ticket file as one Git tree holds it.
pub(in super::super) struct TreeTicket {
    pub(in super::super) bytes: Vec<u8>,
    /// A Blob with mode `100644` or `100755`.
    canonical_entry: bool,
}
pub(in super::super) fn tree_ticket(
    worker: &git2::Repository,
    tree: Oid,
    path: &Path,
    root: &Path,
) -> Result<Option<TreeTicket>, RepositoryError> {
    let tree = worker
        .find_tree(tree)
        .map_err(|_| shared::unavailable(root))?;
    let entry = match tree.get_path(path) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(_) => return Err(shared::unavailable(root)),
    };
    let canonical_entry = entry.kind() == Some(git2::ObjectType::Blob)
        && matches!(entry.filemode(), 0o100644 | 0o100755);
    let bytes = match entry.kind() {
        Some(git2::ObjectType::Blob) => worker
            .find_blob(entry.id())
            .map_err(|_| shared::unavailable(root))?
            .content()
            .to_vec(),
        _ => Vec::new(),
    };
    Ok(Some(TreeTicket {
        bytes,
        canonical_entry,
    }))
}
fn commit_tree(
    worker: &git2::Repository,
    commit: Oid,
    root: &Path,
) -> Result<Oid, RepositoryError> {
    worker
        .find_commit(commit)
        .map(|c| c.tree_id())
        .map_err(|_| shared::unavailable(root))
}

/// The closure a ticket's front matter carries, read without asking that it
/// be a conforming ticket: a file naming either closure key is closed.
pub(in super::super) enum FileClosure {
    Open,
    /// Both fields, exactly as the file has them.
    Closed {
        closed_at: String,
        closed_by: String,
    },
    /// A closure key whose value is missing or not text.
    Malformed,
}
pub(in super::super) fn file_closure(bytes: &[u8]) -> FileClosure {
    let Some(values) = std::str::from_utf8(bytes)
        .ok()
        .and_then(canonical::front_matter)
    else {
        return FileClosure::Open;
    };
    match (values.get("closed_at"), values.get("closed_by")) {
        (None, None) => FileClosure::Open,
        (Some(at), Some(by)) => match (at.as_str(), by.as_str()) {
            (Some(at), Some(by)) => FileClosure::Closed {
                closed_at: at.to_owned(),
                closed_by: by.to_owned(),
            },
            _ => FileClosure::Malformed,
        },
        _ => FileClosure::Malformed,
    }
}

/// D1: the pre-closure result holds exactly one conforming ticket at its
/// canonical path, as a regular Blob, and no `.manyhands/worktrees/` entry.
fn result_ticket(
    worker: &git2::Repository,
    tree: Oid,
    item: &ItemId,
    root: &Path,
) -> Result<Option<Vec<u8>>, RepositoryError> {
    let tree = worker
        .find_tree(tree)
        .map_err(|_| shared::unavailable(root))?;
    let mut forbidden = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |parent, entry| {
        let mut path = parent.as_bytes().to_vec();
        path.extend_from_slice(entry.name_bytes());
        if path == b".manyhands/worktrees" || path.starts_with(b".manyhands/worktrees/") {
            forbidden = true;
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|_| shared::unavailable(root))?;
    if forbidden {
        return Ok(None);
    }
    let snapshot = RepositoryService::canonical_tree_sources(worker, &tree)
        .map_err(|_| shared::unavailable(root))?;
    let sources = snapshot
        .sources
        .iter()
        .filter_map(|(path, bytes)| {
            Some((
                PathBuf::from(std::str::from_utf8(path).ok()?),
                std::str::from_utf8(bytes).ok()?.to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    let validation = canonical::validate_context(sources.clone());
    if !validation
        .items
        .iter()
        .any(|found| matches!(found, CanonicalItem::Ticket(ticket) if ticket.id == *item))
    {
        return Ok(None);
    }
    let paths = sources
        .iter()
        .filter(|(path, source)| {
            matches!(canonical::parse_item(path, source), Ok(CanonicalItem::Ticket(t)) if t.id == *item)
        })
        .collect::<Vec<_>>();
    let [(path, source)] = paths.as_slice() else {
        return Ok(None);
    };
    if *path != ticket_path(item)
        || !matches!(
            snapshot.entries.get(path.as_os_str().as_encoded_bytes()),
            Some((_, 0o100644 | 0o100755, Some(git2::ObjectType::Blob)))
        )
    {
        return Ok(None);
    }
    Ok(Some(source.as_bytes().to_vec()))
}

/// What the integration table decides for a present context.
enum Effect {
    Record,
    Existing {
        closed_at: String,
        closed_by: String,
    },
}

/// Observe actual Git afresh with a coherently read registry view. The same
/// function runs at preview, after the context child and under the lease.
#[allow(clippy::too_many_arguments)] // One observation's coherent inputs.
fn observe_git(
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
    snapshot: &RemoteSnapshot,
    pending_conflict: bool,
    pending: PendingWork,
    facts: &DiscoveryFacts,
    confirmation: Option<&ConfirmedCommitIdentity>,
    mode: Integration,
    verb: Verb,
    at: OffsetDateTime,
) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError> {
    use ClosureBlocker::*;
    let branch = branch_observed(mode, verb);
    let mut blockers = BTreeSet::new();
    let path = ticket_path(item);
    let context_branch = KIND.context_branch(item);
    let mut context_oid = None;
    let mut context_missing = false;
    let mut context_conforming = false;
    // No worktree and no local branch: nothing of a context is left. A
    // worktree path that something else holds is `worktree_occupied`.
    let mut context_absent = false;
    let mut worktree_occupied = false;
    if facts.not_ticket {
        blockers.insert(NotATicket);
    } else {
        match shared::context_presence(KIND, repository, root, config, item)? {
            ContextPresence::Materialized { oid, cleanliness } => {
                context_oid = oid;
                if oid.is_none() {
                    // The branch can disappear between materialization and this
                    // unleased read; an incomplete observation is never ready.
                    context_missing = true;
                }
                match cleanliness {
                    Cleanliness::Clean => {}
                    Cleanliness::Dirty => {
                        blockers.insert(ContextDirty);
                    }
                    Cleanliness::Conflicted => {
                        blockers.insert(ContextConflicted);
                    }
                }
                context_conforming = facts.ticket_path.as_deref() == Some(path.as_path());
                if !context_conforming {
                    blockers.insert(ContextNonconforming);
                }
            }
            ContextPresence::Missing => {
                context_missing = true;
                // Only a pull-request close reads them: the direct close and
                // the withdraw keep today's answer for a ref that cannot be
                // read.
                if branch && verb == Verb::Close {
                    context_absent =
                        shared::reference_oid(repository, &KIND.context_ref(item), root)?.is_none();
                    let name = item.to_string();
                    worktree_occupied = root
                        .join(".manyhands/worktrees")
                        .join(&name)
                        .symlink_metadata()
                        .is_ok()
                        || repository.find_worktree(&name).is_ok();
                }
            }
            ContextPresence::Nonconforming => {
                blockers.insert(ContextNonconforming);
            }
        }
    }
    if pending_conflict {
        blockers.insert(ConflictPending);
    }
    if pending.local_operation {
        blockers.insert(LocalOperationPending);
    }
    if pending.comment_publication {
        blockers.insert(CommentPublicationPending);
    }
    let primary = shared::primary_presence(repository, root, config)?;
    let primary_oid = primary.oid();
    match primary {
        PrimaryPresence::Missing => {
            blockers.insert(PrimaryMissing);
        }
        // Nothing of the primary's checkout is touched by a pull-request
        // close, so nothing of it blocks one.
        _ if branch => {}
        PrimaryPresence::NotCheckedOut(_) => {
            blockers.insert(PrimaryNotCheckedOut);
        }
        PrimaryPresence::CheckedOut(_, Cleanliness::Clean) => {}
        PrimaryPresence::CheckedOut(_, Cleanliness::Dirty) => {
            blockers.insert(PrimaryDirty);
        }
        PrimaryPresence::CheckedOut(_, Cleanliness::Conflicted) => {
            blockers.insert(PrimaryConflicted);
        }
    }
    // A merge, rebase or cherry-pick in progress at the root, whichever
    // branch the root is on, is what refuses a synchronization (ruling 6).
    if branch && repository.state() != git2::RepositoryState::Clean {
        blockers.insert(PrimaryConflicted);
    }
    let mut tracking_primary = None;
    let mut tracking_context = None;
    if let Some(remote) = shared::remote_facts(
        KIND,
        repository,
        root,
        config,
        item,
        primary_oid,
        context_oid,
    )? {
        tracking_primary = remote.tracking_primary;
        tracking_context = remote.tracking_context;
        if tracking_primary.is_none() {
            blockers.insert(PrimaryMissing);
        }
        if remote.primary_ahead && !branch {
            blockers.insert(PrimaryAheadOfRemote);
        }
        if remote.context_behind && !branch {
            blockers.insert(ContextBehindRemote);
        }
    }
    if branch {
        let gathered = Gathered {
            blockers,
            context_oid,
            context_missing,
            context_conforming,
            context_absent,
            worktree_occupied,
            primary_oid,
            tracking_primary,
            tracking_context,
        };
        return match verb {
            Verb::Close => branch_observation(
                repository,
                root,
                config,
                item,
                snapshot,
                facts,
                confirmation,
                at,
                pending,
                gathered,
            ),
            Verb::Withdraw => withdraw_observation(
                repository,
                root,
                config,
                item,
                snapshot,
                facts,
                confirmation,
                mode,
                pending,
                gathered,
            ),
        };
    }
    let base = if config.publication_remote.is_some() {
        tracking_primary
    } else {
        primary_oid
    };
    // A memory-backed handle: in-memory merges and canonical tree reads
    // write no ODB object.
    let worker = git2::Repository::open(root).map_err(|_| shared::unavailable(root))?;
    let odb = worker.odb().map_err(|_| shared::unavailable(root))?;
    let _memory = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| shared::unavailable(root))?;

    // The primary's copy is read from Git, never the working tree.
    let base_closure = match (base, facts.not_ticket) {
        (Some(base), false) => closure_at(&worker, base, &path, root)?,
        _ => FileClosure::Open,
    };
    let base_closed = !matches!(base_closure, FileClosure::Open);
    // The context's copy, from its branch tip.
    let context_ticket = context_ticket_at(
        &worker,
        context_oid.filter(|_| context_conforming),
        &path,
        root,
        &mut blockers,
    )?;
    let context_closure = context_ticket.as_ref().map(|t| file_closure(&t.bytes));

    // The integration table (Observation item 4).
    let mut contained = false;
    let mut integration = None;
    let mut result = None;
    let mut effect = None;
    if context_missing {
        blockers.insert(if base_closed {
            AlreadyClosed
        } else {
            ContextMissing
        });
    } else if let (Some(base), Some(context)) = (base, context_oid) {
        contained = shared::ancestor(repository, context, base, root)?;
        if !contained && base_closed {
            // Never merge further work into a closed ticket; the context is kept.
            blockers.insert(AlreadyClosed);
        } else if let Some(context_closure) = &context_closure {
            let planned = shared::planned_result(repository, &worker, root, base, context)?;
            match (planned, base_closed, context_closure) {
                (PlannedResult::Contained(tree), true, _) => {
                    integration = Some(PlannedIntegration::AlreadyContained);
                    result = Some(tree);
                    effect = match &base_closure {
                        FileClosure::Closed {
                            closed_at,
                            closed_by,
                        } => Some(Effect::Existing {
                            closed_at: closed_at.clone(),
                            closed_by: closed_by.clone(),
                        }),
                        _ => {
                            blockers.insert(TicketInvalidAfterClosure);
                            None
                        }
                    };
                }
                (PlannedResult::Contained(_), false, FileClosure::Open) => {
                    // Merged by hand without closure fields: close it now.
                    integration = Some(PlannedIntegration::MergeCommit);
                    result = Some(commit_tree(&worker, base, root)?);
                    effect = Some(Effect::Record);
                }
                (PlannedResult::Contained(_), false, _) => {
                    // The closure was removed on the primary by hand.
                    blockers.insert(PrimaryReopened);
                }
                (PlannedResult::Merge(tree), _, closure) => {
                    integration = Some(PlannedIntegration::MergeCommit);
                    result = Some(tree);
                    effect = match closure {
                        FileClosure::Open => Some(Effect::Record),
                        FileClosure::Closed {
                            closed_at,
                            closed_by,
                        } => Some(Effect::Existing {
                            closed_at: closed_at.clone(),
                            closed_by: closed_by.clone(),
                        }),
                        FileClosure::Malformed => {
                            blockers.insert(ContextNonconforming);
                            None
                        }
                    };
                }
                (PlannedResult::Conflict, _, _) => {
                    integration = Some(PlannedIntegration::MergeCommit);
                    blockers.insert(IntegrationConflict);
                }
            }
        }
    }
    // D5: an absent tracking context that was published, and that a fetch has
    // seen gone, means another clone deleted it; published and not seen gone
    // is unknown (decision 35). Only a context child could recreate it, and
    // none runs for a contained context.
    if config.publication_remote.is_some()
        && tracking_context.is_none()
        && !contained
        && !facts.not_ticket
    {
        blockers.extend(remote_history_blocker(snapshot, item));
    }

    // C2: identity, read-only, and only for a `Record` closure (it needs
    // `closed_by`); an `Existing` closure resolves its merge signer at
    // execution, so no selection or selection error applies to it here.
    let IdentityChoice {
        identity,
        configuration: identity_configuration,
        selected,
    } = select_identity(
        repository,
        root,
        confirmation,
        matches!(effect, Some(Effect::Record)),
    )?;

    // D1, C10: the result's ticket, and the same closed-ticket function the
    // checkpoint runs, with the preview time standing in for `closed_at`.
    if let Some(result) = result {
        match result_ticket(&worker, result, item, root)? {
            None => {
                blockers.insert(TicketInvalidAfterClosure);
            }
            Some(result_bytes) => match &effect {
                Some(Effect::Record) => {
                    // Any valid text stands in for a not yet selected identity.
                    let closed_by = selected.as_deref().unwrap_or("Manyhands <closure@invalid>");
                    let context_bytes = &context_ticket.as_ref().expect("record context").bytes;
                    if closed_ticket_bytes(context_bytes, at, closed_by).is_err()
                        || closed_ticket_bytes(&result_bytes, at, closed_by).is_err()
                    {
                        blockers.insert(TicketInvalidAfterClosure);
                    } else if let (Some(base), Some(context)) = (base, context_oid)
                        && (config.publication_remote.is_none() || contained)
                    {
                        // Final review I-1: the checkpoint and its merge into
                        // the base, exactly as execution builds them, in the
                        // memory-backed `worker`. The closure fields can sit
                        // next to a primary-side edit the pre-closure merge
                        // never sees. `at` stands in for `closed_at`: each
                        // field is one line whatever its value, so whether a
                        // hunk conflicts never depends on it.
                        // Only where execution merges the checkpoint against
                        // this context (R-F-3): with a remote and a context
                        // that is not contained, the context child merges
                        // the base in first, and the checkpoint then
                        // contains the base, so no checkpoint merge runs.
                        let stand_in = CommitIdentity {
                            name: "Manyhands".into(),
                            email: "closure@invalid".into(),
                        };
                        let identity = match &identity {
                            Some(
                                CommittingIdentity::Effective(identity)
                                | CommittingIdentity::Confirmed(identity),
                            ) => identity,
                            None => &stand_in,
                        };
                        let target = ClosureTarget {
                            item,
                            path: &path,
                            context,
                            base,
                        };
                        match build_closure(&worker, MergeHandle::Memory, target, at, identity)? {
                            ClosureBuild::Built(_) => {}
                            ClosureBuild::Conflict => {
                                blockers.insert(IntegrationConflict);
                            }
                            ClosureBuild::Invalid => {
                                blockers.insert(TicketInvalidAfterClosure);
                            }
                        }
                    }
                }
                // The result must carry exactly the planned closure.
                Some(Effect::Existing {
                    closed_at,
                    closed_by,
                }) if !matches!(
                    file_closure(&result_bytes),
                    FileClosure::Closed { closed_at: a, closed_by: b }
                        if a == *closed_at && b == *closed_by
                ) =>
                {
                    blockers.insert(TicketInvalidAfterClosure);
                }
                Some(Effect::Existing { .. }) | None => {}
            },
        }
    }
    let closure = planned_closure(effect, selected, &mut blockers);
    if !blockers.is_empty() {
        return Ok(Err(refusal_of(blockers, identity_configuration)));
    }
    let (
        Some(base),
        Some(context_oid),
        Some(primary_oid),
        Some(result),
        Some(closure),
        Some(integration),
    ) = (base, context_oid, primary_oid, result, closure, integration)
    else {
        // An unblocked observation always has every fact above.
        return Err(shared::unavailable(root));
    };
    let base_tree = worker
        .find_tree(commit_tree(&worker, base, root)?)
        .map_err(|_| shared::unavailable(root))?;
    let result_tree = worker
        .find_tree(result)
        .map_err(|_| shared::unavailable(root))?;
    let changes = shared::change_evidence(&worker, &base_tree, &result_tree, root)?;
    let remote = config.publication_remote.clone();
    let cleanup = PlannedCleanup {
        worktree: true,
        local_branch: true,
        remote_branch: remote.is_some() && !(contained && tracking_context.is_none()),
    };
    let mut observation = ClosureObservation {
        item_id: item.clone(),
        ticket_path: path,
        context_oid,
        base,
        remote,
        changes,
        closure,
        integration,
        needs_context_child: config.publication_remote.is_some() && !contained,
        cleanup,
        identity,
        expectation: ClosureExpectation::from_bytes([0; 32]),
        context_branch,
        primary_branch: config.primary_branch.clone(),
        primary_oid,
        tracking_primary,
        tracking_context,
        path: (Integration::Direct, ClosureAction::Close),
        new_branch: None,
    };
    observation.expectation = expectation(&observation);
    Ok(Ok(observation))
}

/// The closure the ticket at `path` holds in `commit`'s tree.
fn closure_at(
    worker: &git2::Repository,
    commit: Oid,
    path: &Path,
    root: &Path,
) -> Result<FileClosure, RepositoryError> {
    Ok(
        match tree_ticket(worker, commit_tree(worker, commit, root)?, path, root)? {
            Some(ticket) => file_closure(&ticket.bytes),
            None => FileClosure::Open,
        },
    )
}

/// The context's copy of the ticket, from its branch tip. One that is not a
/// canonical regular file is a nonconforming context.
fn context_ticket_at(
    worker: &git2::Repository,
    context: Option<Oid>,
    path: &Path,
    root: &Path,
    blockers: &mut BTreeSet<ClosureBlocker>,
) -> Result<Option<TreeTicket>, RepositoryError> {
    let Some(context) = context else {
        return Ok(None);
    };
    Ok(
        match tree_ticket(worker, commit_tree(worker, context, root)?, path, root)? {
            Some(ticket) if ticket.canonical_entry => Some(ticket),
            _ => {
                blockers.insert(ClosureBlocker::ContextNonconforming);
                None
            }
        },
    )
}

/// What an absent tracking context says of the remote (D5, decision 35), as
/// a blocker.
fn remote_history_blocker(snapshot: &RemoteSnapshot, item: &ItemId) -> Option<ClosureBlocker> {
    match shared::remote_context_history(snapshot, KIND, item) {
        RemoteContextHistory::Deleted => Some(ClosureBlocker::RemoteContextDeleted),
        RemoteContextHistory::Unknown => Some(ClosureBlocker::HistoryUnknown),
        RemoteContextHistory::Unresolved => Some(ClosureBlocker::HistoryUnresolved),
        RemoteContextHistory::NeverPublished => None,
    }
}

/// The committing identity a `Record` closure selects, read-only.
struct IdentityChoice {
    identity: Option<CommittingIdentity>,
    /// The configuration a missing identity is asked for.
    configuration: Option<ExpectedConfiguration>,
    /// `closed_by`, as the closure records it.
    selected: Option<String>,
}

/// Effective wins; else a configuration-bound confirmation; else the
/// expected configuration for `IdentityRequired`. Only for a `Record`
/// closure (`needed`): it needs `closed_by`.
fn select_identity(
    repository: &git2::Repository,
    root: &Path,
    confirmation: Option<&ConfirmedCommitIdentity>,
    needed: bool,
) -> Result<IdentityChoice, RepositoryError> {
    let selection = if needed {
        Some(sync::select_committing_identity(
            repository,
            confirmation,
            &SynchronizationTarget::Primary,
        ))
    } else {
        None
    };
    let mut configuration = None;
    let identity = match selection {
        None => None,
        Some(Ok(identity)) => Some(identity),
        Some(Err(SynchronizationError::IdentityRequired {
            expected_configuration,
            ..
        })) => {
            configuration = Some(expected_configuration);
            None
        }
        // A confirmation for another configuration is no confirmation.
        Some(Err(SynchronizationError::ExternalChange)) => {
            match sync::select_committing_identity(
                repository,
                None,
                &SynchronizationTarget::Primary,
            ) {
                Err(SynchronizationError::IdentityRequired {
                    expected_configuration,
                    ..
                }) => configuration = Some(expected_configuration),
                _ => return Err(shared::unavailable(root)),
            }
            None
        }
        Some(Err(_)) => return Err(shared::unavailable(root)),
    };
    let selected = identity.as_ref().map(|identity| match identity {
        CommittingIdentity::Effective(identity) | CommittingIdentity::Confirmed(identity) => {
            closed_by_text(identity)
        }
    });
    Ok(IdentityChoice {
        identity,
        configuration,
        selected,
    })
}

/// The closure the observation plans. A `Record` without an identity is
/// `IdentityRequired`, only when nothing else blocks: it needs `closed_by`
/// for the digest.
fn planned_closure(
    effect: Option<Effect>,
    selected: Option<String>,
    blockers: &mut BTreeSet<ClosureBlocker>,
) -> Option<PlannedClosure> {
    match effect {
        Some(Effect::Record) => match selected {
            Some(closed_by) => Some(PlannedClosure::Record { closed_by }),
            None => {
                if blockers.is_empty() {
                    blockers.insert(ClosureBlocker::IdentityRequired);
                }
                None
            }
        },
        Some(Effect::Existing {
            closed_at,
            closed_by,
        }) => Some(PlannedClosure::Existing {
            closed_at,
            closed_by,
        }),
        None => None,
    }
}

/// The refusal of a blocked observation: the identity configuration exactly
/// when the only blocker is `IdentityRequired`.
fn refusal_of(
    blockers: BTreeSet<ClosureBlocker>,
    identity_configuration: Option<ExpectedConfiguration>,
) -> ClosureRefusal {
    let only_identity = blockers.len() == 1 && blockers.contains(&ClosureBlocker::IdentityRequired);
    ClosureRefusal {
        blockers: blockers.into_iter().collect(),
        identity_configuration: identity_configuration.filter(|_| only_identity),
        unoffered: None,
    }
}

/// What `observe_git` has read when a pull-request close takes over.
struct Gathered {
    blockers: BTreeSet<ClosureBlocker>,
    context_oid: Option<Oid>,
    context_missing: bool,
    context_conforming: bool,
    /// No context is materialized and its local branch is gone.
    context_absent: bool,
    /// No context is materialized, and its worktree path is held by
    /// something else, or a worktree of that name is registered.
    worktree_occupied: bool,
    primary_oid: Option<Oid>,
    tracking_primary: Option<Oid>,
    tracking_context: Option<Oid>,
}

/// The pull-request close's observation. Nothing integrates, so there is no
/// change set, no planned cleanup and no result tree to check: the situation
/// (Part D) is read from the primaries' and the context's tickets, the remote
/// history and the tracking refs, and the closure the context's tip would be
/// given is checked as the checkpoint will build it. The situations, in
/// order:
///
/// 1. the local or the tracking primary's ticket is not open: `AlreadyClosed`;
/// 2. a context whose remote branch's deletion is evidenced: a new branch,
///    which keeps a provisional closure the context's ticket holds;
/// 3. no context:
///    1. the remote branch's deletion is evidenced: a new branch, retiring
///       the remote history;
///    2. a tracking ref of the branch that the tracking primary holds: a new
///       branch with nothing to retire, which its publication fast-forwards;
///    3. a tracking ref otherwise: `RemoteContextUnmaterialized`;
///    4. no tracking ref: a new branch with nothing to retire;
/// 4. the context's ticket holds a closure: `Existing`;
/// 5. otherwise `Record`.
#[allow(clippy::too_many_arguments)] // One observation's coherent inputs.
fn branch_observation(
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
    snapshot: &RemoteSnapshot,
    facts: &DiscoveryFacts,
    confirmation: Option<&ConfirmedCommitIdentity>,
    at: OffsetDateTime,
    pending: PendingWork,
    gathered: Gathered,
) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError> {
    use ClosureBlocker::*;
    let Gathered {
        mut blockers,
        context_oid,
        context_missing,
        context_conforming,
        context_absent,
        worktree_occupied,
        primary_oid,
        tracking_primary,
        tracking_context,
    } = gathered;
    let path = ticket_path(item);
    let base = tracking_primary;
    if base.is_none() {
        blockers.insert(PrimaryMissing);
    }
    let closure_of = |commit: Option<Oid>| match commit {
        Some(commit) if !facts.not_ticket => closure_at(repository, commit, &path, root),
        _ => Ok(FileClosure::Open),
    };
    // Situation 1 decides alone: a ticket a primary has closed is
    // `already_closed` and nothing else, whatever the context, the remote or
    // an earlier read say.
    if !matches!(closure_of(primary_oid)?, FileClosure::Open)
        || !matches!(closure_of(base)?, FileClosure::Open)
    {
        return Ok(Err(refusal_of(BTreeSet::from([AlreadyClosed]), None)));
    }
    // A synchronization of the context that is not an unfinished record's
    // own is `busy`, and names the row. While such a record holds the ticket
    // the hold is the binding's to answer (decision 33), not this read's.
    if let Some(operation) = pending
        .synchronization
        .as_deref()
        .filter(|_| !pending.record_unfinished)
    {
        return Err(foreign_synchronization_busy(root, operation));
    }
    // Situations 2 and 3. A new branch is `(old tip, retired)`.
    let history = pending.history.as_ref().filter(|_| !facts.not_ticket);
    let deleted = history.is_some_and(|history| history.evidence.deleted());
    // A context that was published is not deleted for want of a fetch: its
    // proven push alone reads as `Pushed` until the next poll. A context is a
    // deleted one when a fetch saw its branch gone, or when a close's own
    // push found it gone (`close_found_deleted`).
    let context_deleted = history.is_some_and(|history| {
        matches!(history.evidence.deletion, sync::DeletionEvidence::Fetched)
            || (matches!(history.evidence.deletion, sync::DeletionEvidence::Pushed)
                && history.found_deleted)
    });
    let mut new_branch: Option<(Option<Oid>, Option<Oid>)> = None;
    let mut unmaterialized = false;
    if let Some(history) = history {
        if let Some(tip) = context_oid
            && context_deleted
        {
            new_branch = Some((Some(tip), Some(tip)));
        } else if context_absent {
            if deleted {
                // The stale tracking ref, else what the newest proven push
                // or the fetch last found published: a commit that is here.
                let retired = tracking_context
                    .or(history.evidence.last_published)
                    .filter(|oid| repository.find_commit(*oid).is_ok());
                new_branch = Some((None, retired));
            } else {
                match (tracking_context, base) {
                    (None, _) => new_branch = Some((None, None)),
                    (Some(tip), Some(base)) if shared::ancestor(repository, tip, base, root)? => {
                        new_branch = Some((None, None));
                    }
                    (Some(tip), Some(_)) => {
                        unmaterialized = true;
                        blockers.insert(RemoteContextUnmaterialized);
                        if let Some(primary) = primary_oid
                            && shared::ancestor(repository, tip, primary, root)?
                        {
                            blockers.insert(PrimaryAheadOfRemote);
                        }
                    }
                    (Some(_), None) => {}
                }
            }
        }
        // Every publication the classification counts must be the retired
        // tip or its ancestor, or the boundary after the retirement could
        // still read the branch as deleted, ambiguous or unknown.
        if let Some((_, retired)) = new_branch.filter(|_| deleted) {
            let ancestors = retired.is_some_and(|retired| {
                history.counted.iter().all(|published| {
                    published.is_some_and(|published| {
                        published == retired
                            || repository
                                .graph_descendant_of(retired, published)
                                .unwrap_or(false)
                    })
                })
            });
            if !ancestors {
                blockers.insert(HistoryUnresolved);
            }
        }
    }
    if config.publication_remote.is_some() && tracking_context.is_none() && !facts.not_ticket {
        blockers.extend(
            remote_history_blocker(snapshot, item)
                // The deletion the new branch retires is not a blocker.
                .filter(|blocker| {
                    *blocker != RemoteContextDeleted || !deleted || new_branch.is_none()
                }),
        );
    }
    if context_missing && new_branch.is_none() && !unmaterialized {
        blockers.insert(ContextMissing);
    }
    let context_ticket = context_ticket_at(
        repository,
        context_oid.filter(|_| context_conforming),
        &path,
        root,
        &mut blockers,
    )?;
    // F7: the tracking primary must hold the ticket for a new branch to be
    // built on it.
    let mut base_ticket = None;
    if let (Some((old_tip, _)), Some(base)) = (new_branch, base) {
        match tree_ticket(
            repository,
            commit_tree(repository, base, root)?,
            &path,
            root,
        )? {
            None => {
                blockers.insert(if old_tip.is_some() {
                    RemoteContextDeleted
                } else {
                    ContextMissing
                });
            }
            Some(ticket) if !ticket.canonical_entry => {
                blockers.insert(TicketInvalidAfterClosure);
            }
            Some(ticket) => base_ticket = Some(ticket),
        }
    }
    // Situations 4 and 5, and for a new branch with no context 5.
    let effect = match context_ticket.as_ref().map(|t| file_closure(&t.bytes)) {
        Some(FileClosure::Open) => Some(Effect::Record),
        Some(FileClosure::Closed {
            closed_at,
            closed_by,
        }) => Some(Effect::Existing {
            closed_at,
            closed_by,
        }),
        Some(FileClosure::Malformed) => {
            blockers.insert(TicketInvalidAfterClosure);
            None
        }
        None if matches!(new_branch, Some((None, _))) => Some(Effect::Record),
        None => None,
    };
    // A new branch's commit is authored by the selected identity, whatever
    // the closure it carries.
    let IdentityChoice {
        identity,
        configuration: identity_configuration,
        selected,
    } = select_identity(
        repository,
        root,
        confirmation,
        matches!(effect, Some(Effect::Record)) || (new_branch.is_some() && effect.is_some()),
    )?;
    if new_branch.is_some() {
        // The closure is written on the tracking primary's ticket, so it is
        // checked against that one.
        if let (Some(base_ticket), Some(effect)) = (&base_ticket, &effect) {
            let refused = match effect {
                Effect::Record => {
                    // Any valid text stands in for a not yet selected identity.
                    let closed_by = selected.as_deref().unwrap_or("Manyhands <closure@invalid>");
                    closed_ticket_bytes(&base_ticket.bytes, at, closed_by).is_err()
                }
                Effect::Existing { .. } => {
                    let kept = context_ticket
                        .as_ref()
                        .and_then(|ticket| parsed_ticket(&ticket.bytes).ok())
                        .and_then(|(_, ticket)| Some((ticket.closed_at?, ticket.closed_by?)));
                    kept.is_none_or(|(kept_at, kept_by)| {
                        closure_replaced_bytes(&base_ticket.bytes, kept_at, &kept_by).is_err()
                    })
                }
            };
            if refused {
                blockers.insert(TicketInvalidAfterClosure);
            }
        }
        // A kept closure needs the identity too, and only when nothing else
        // blocks, as a `Record` one does.
        if matches!(effect, Some(Effect::Existing { .. }))
            && selected.is_none()
            && blockers.is_empty()
        {
            blockers.insert(IdentityRequired);
        }
    } else if let (Some(Effect::Record), Some(context_ticket)) = (&effect, &context_ticket) {
        // Any valid text stands in for a not yet selected identity.
        let closed_by = selected.as_deref().unwrap_or("Manyhands <closure@invalid>");
        if closed_ticket_bytes(&context_ticket.bytes, at, closed_by).is_err() {
            blockers.insert(TicketInvalidAfterClosure);
        }
    }
    let closure = planned_closure(effect, selected, &mut blockers);
    if !blockers.is_empty() {
        return Ok(Err(refusal_of(blockers, identity_configuration)));
    }
    let (Some(base), Some(primary_oid), Some(closure)) = (base, primary_oid, closure) else {
        // An unblocked observation always has every fact above.
        return Err(shared::unavailable(root));
    };
    let (context_oid, cleanup, facts_of_new_branch) = match new_branch {
        Some((old_tip, retired_oid)) => {
            // What the old tip holds that the tracking primary lacks. While
            // an unfinished record holds the ticket, a history that cannot be
            // compared is left to the hold, and the record's own step 1 still
            // binds the entries it can read.
            let unheld = match old_tip {
                Some(tip) => match proof::unheld_entries(repository, tip, base) {
                    Ok(entries) => entries,
                    Err(_) if pending.record_unfinished => Vec::new(),
                    Err(_) => return Err(state::recovery_required()),
                },
                None => Vec::new(),
            };
            // The new context's worktree path must be free once the old
            // context is counted out.
            if old_tip.is_none() && worktree_occupied && !pending.record_unfinished {
                return Err(RepositoryError::new(
                    RepositoryOperation::Read,
                    Some(root.to_owned()),
                    RepositoryErrorKind::OccupiedItemPath,
                    "the ticket's worktree path is occupied",
                ));
            }
            (
                old_tip.unwrap_or(base),
                PlannedCleanup {
                    worktree: old_tip.is_some(),
                    local_branch: old_tip.is_some(),
                    remote_branch: false,
                },
                Some(NewBranchFacts {
                    old_tip,
                    retired_oid,
                    unheld,
                }),
            )
        }
        None => {
            let Some(context_oid) = context_oid else {
                return Err(shared::unavailable(root));
            };
            (
                context_oid,
                PlannedCleanup {
                    worktree: false,
                    local_branch: false,
                    remote_branch: false,
                },
                None,
            )
        }
    };
    let mut observation = ClosureObservation {
        item_id: item.clone(),
        ticket_path: path,
        context_oid,
        base,
        remote: config.publication_remote.clone(),
        changes: Vec::new(),
        closure,
        integration: PlannedIntegration::MergeCommit,
        needs_context_child: facts_of_new_branch.is_none(),
        cleanup,
        identity,
        expectation: ClosureExpectation::from_bytes([0; 32]),
        context_branch: KIND.context_branch(item),
        primary_branch: config.primary_branch.clone(),
        primary_oid,
        tracking_primary,
        tracking_context,
        path: (
            Integration::PullRequest,
            if facts_of_new_branch.is_some() {
                ClosureAction::NewBranch
            } else {
                ClosureAction::Close
            },
        ),
        new_branch: facts_of_new_branch,
    };
    observation.expectation = expectation(&observation);
    Ok(Ok(observation))
}

/// A withdraw's observation (Part E), in either mode: the branch record's
/// road, ending in the closure being withdrawn. Nothing integrates, and the
/// withdraw's commit is built at step 3 from the context's head. In order:
///
/// 1. no publication remote: nothing to confirm, a withdraw's children
///    publish (`Unoffered::PublicationRemoteRequired`);
/// 2. the local or the tracking primary's ticket is not open: `AlreadyClosed`
///    alone, as for the close (withdrawing it would be a reopen);
/// 3. a synchronization of the context that no unfinished record owns: busy;
/// 4. the blockers Part D keeps, `ContextMissing` for no context, and the
///    remote history of a context whose tracking ref is absent;
/// 5. the closure to withdraw: the context's, else the tracking context's,
///    which the context child brings in; one that is malformed, or that
///    cannot be removed, is `TicketInvalidAfterClosure`;
/// 6. any blocker so far refuses, before there is nothing to withdraw;
/// 7. none: nothing to confirm (`Unoffered::NothingToWithdraw`), unless an
///    unfinished record that a new request could not supersede holds the
///    ticket. Then the observation goes on, with a closure that is only a
///    placeholder: the binding observes before its hold (decision 33), which
///    answers `recovery_required` naming the holder, and a request that
///    confirms it meets the same record's `busy` at insertion (ruling 35);
/// 8. the identity of the commit's author, which is not consented.
#[allow(clippy::too_many_arguments)] // One observation's coherent inputs.
fn withdraw_observation(
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
    snapshot: &RemoteSnapshot,
    facts: &DiscoveryFacts,
    confirmation: Option<&ConfirmedCommitIdentity>,
    mode: Integration,
    pending: PendingWork,
    gathered: Gathered,
) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError> {
    use ClosureBlocker::*;
    let Gathered {
        mut blockers,
        context_oid,
        context_missing,
        context_conforming,
        primary_oid,
        tracking_primary,
        tracking_context,
        ..
    } = gathered;
    if config.publication_remote.is_none() {
        return Ok(Err(ClosureRefusal::unoffered(
            Unoffered::PublicationRemoteRequired,
        )));
    }
    let path = ticket_path(item);
    let closure_of = |commit: Option<Oid>| match commit {
        Some(commit) if !facts.not_ticket => closure_at(repository, commit, &path, root),
        _ => Ok(FileClosure::Open),
    };
    // Situation 1 decides alone, as for the close.
    if !matches!(closure_of(primary_oid)?, FileClosure::Open)
        || !matches!(closure_of(tracking_primary)?, FileClosure::Open)
    {
        return Ok(Err(refusal_of(BTreeSet::from([AlreadyClosed]), None)));
    }
    if let Some(operation) = pending
        .synchronization
        .as_deref()
        .filter(|_| !pending.record_unfinished)
    {
        return Err(foreign_synchronization_busy(root, operation));
    }
    if tracking_primary.is_none() {
        blockers.insert(PrimaryMissing);
    }
    if context_missing {
        blockers.insert(ContextMissing);
    } else if tracking_context.is_none() && !facts.not_ticket {
        blockers.extend(remote_history_blocker(snapshot, item));
    }
    let context_ticket = context_ticket_at(
        repository,
        context_oid.filter(|_| context_conforming),
        &path,
        root,
        &mut blockers,
    )?;
    let tracking_ticket = match tracking_context.filter(|_| !facts.not_ticket) {
        Some(tip) => tree_ticket(repository, commit_tree(repository, tip, root)?, &path, root)?,
        None => None,
    };
    // The context's closure is the one withdrawn; failing it, the tracking
    // context's, which the context child brings in.
    let mut chosen = None;
    for ticket in [context_ticket.as_ref(), tracking_ticket.as_ref()]
        .into_iter()
        .flatten()
    {
        match file_closure(&ticket.bytes) {
            FileClosure::Open => {}
            FileClosure::Malformed => {
                blockers.insert(TicketInvalidAfterClosure);
            }
            FileClosure::Closed {
                closed_at,
                closed_by,
            } => {
                chosen.get_or_insert((closed_at, closed_by, &ticket.bytes));
            }
        }
    }
    if chosen
        .as_ref()
        .is_some_and(|(_, _, bytes)| withdrawn_ticket_bytes(bytes).is_err())
    {
        blockers.insert(TicketInvalidAfterClosure);
    }
    if !blockers.is_empty() {
        return Ok(Err(refusal_of(blockers, None)));
    }
    let (closure, identity) = match chosen {
        Some((closed_at, closed_by, _)) => {
            let IdentityChoice {
                identity,
                configuration,
                selected,
            } = select_identity(repository, root, confirmation, true)?;
            if selected.is_none() {
                return Ok(Err(refusal_of(
                    BTreeSet::from([IdentityRequired]),
                    configuration,
                )));
            }
            (
                PlannedClosure::Existing {
                    closed_at,
                    closed_by,
                },
                identity,
            )
        }
        None if pending.record_unfinished => (
            PlannedClosure::Existing {
                closed_at: String::new(),
                closed_by: String::new(),
            },
            None,
        ),
        None => {
            return Ok(Err(ClosureRefusal::unoffered(Unoffered::NothingToWithdraw)));
        }
    };
    let (Some(base), Some(context_oid), Some(primary_oid)) =
        (tracking_primary, context_oid, primary_oid)
    else {
        // An unblocked observation always has every fact above.
        return Err(shared::unavailable(root));
    };
    let mut observation = ClosureObservation {
        item_id: item.clone(),
        ticket_path: path,
        context_oid,
        base,
        remote: config.publication_remote.clone(),
        changes: Vec::new(),
        closure,
        integration: PlannedIntegration::MergeCommit,
        needs_context_child: true,
        cleanup: PlannedCleanup {
            worktree: false,
            local_branch: false,
            remote_branch: false,
        },
        identity,
        expectation: ClosureExpectation::from_bytes([0; 32]),
        context_branch: KIND.context_branch(item),
        primary_branch: config.primary_branch.clone(),
        primary_oid,
        tracking_primary,
        tracking_context,
        path: (mode, ClosureAction::Withdraw),
        new_branch: None,
    };
    observation.expectation = expectation(&observation);
    Ok(Ok(observation))
}

/// The consent digest of the observation's path: the direct close's, or the
/// branch record's, which is a pull-request close's and every withdraw's, in
/// either mode.
fn expectation(o: &ClosureObservation) -> ClosureExpectation {
    match o.path {
        (Integration::Direct, ClosureAction::Close) => direct_expectation(o),
        _ => branch_expectation(o),
    }
}

/// `blake3("manyhands closure v1" ‖ fields)`, fixed-width and
/// length-prefixed: item, ticket path, branches, remote, integration kind,
/// the sorted pre-closure change set (T2-1), the closure tag and `closed_by`
/// (and `closed_at` exactly as in the file for `existing`), the cleanup set,
/// and only without a remote the local primary and context OIDs (D6).
/// `closed_at` of a `record` closure is never bound (C6).
fn direct_expectation(o: &ClosureObservation) -> ClosureExpectation {
    let mut hash = blake3::Hasher::new();
    hash.update(CLOSURE_DIGEST_DOMAIN);
    digest::field(&mut hash, o.item_id.to_string().as_bytes());
    digest::field(&mut hash, o.ticket_path.as_os_str().as_encoded_bytes());
    digest::field(&mut hash, o.context_branch.as_bytes());
    digest::field(&mut hash, o.primary_branch.as_bytes());
    digest::remote(&mut hash, o.remote.as_deref());
    digest::integration(&mut hash, o.integration);
    let (changes, entries): (Vec<_>, Vec<_>) = o
        .changes
        .iter()
        .map(|c| (c.change.clone(), c.entry))
        .unzip();
    digest::changes(&mut hash, &changes, &entries);
    closure_effect(&mut hash, &o.closure);
    digest::cleanup(&mut hash, o.cleanup);
    digest::local_oids(&mut hash, o.remote.as_deref(), o.primary_oid, o.context_oid);
    ClosureExpectation::from_bytes(*hash.finalize().as_bytes())
}

/// The closure's tag and what it binds: `closed_by` of a `record`; both
/// fields, exactly as in the file, of an `existing` one.
fn closure_effect(hash: &mut blake3::Hasher, closure: &PlannedClosure) {
    match closure {
        PlannedClosure::Record { closed_by } => {
            hash.update(&[0]);
            digest::field(hash, closed_by.as_bytes());
        }
        PlannedClosure::Existing {
            closed_at,
            closed_by,
        } => {
            hash.update(&[1]);
            digest::field(hash, closed_by.as_bytes());
            digest::field(hash, closed_at.as_bytes());
        }
    }
}

/// The pull-request close's digest, which binds the head the consent
/// reads (`consented_head`).
fn branch_expectation(o: &ClosureObservation) -> ClosureExpectation {
    branch_expectation_with_head(o, o.consented_head())
}

/// `blake3("manyhands pull request closure v1" ‖ fields)`, framed as the
/// direct digest is: the action tag (`close` 0, `withdraw` 1, `new_branch`
/// 2), the item, the context branch, the primary branch, the remote, the
/// context's head (or none) and the closure effect. For a new branch they
/// are followed by the tracking primary, what step 2 retires and the
/// tracking context's tip (each framed as the head is), and the unheld
/// entries: their count, then each path and its change (`added` 0,
/// `modified` 1, `deleted` 2). A `record` closure's `closed_at` is never
/// bound (C6). Nothing of the change set or the cleanup is bound: nothing
/// integrates.
pub(super) fn branch_expectation_with_head(
    o: &ClosureObservation,
    head: Option<Oid>,
) -> ClosureExpectation {
    fn optional(hash: &mut blake3::Hasher, oid: Option<Oid>) {
        match oid {
            Some(oid) => {
                hash.update(&[1]);
                hash.update(oid.as_bytes());
            }
            None => {
                hash.update(&[0]);
            }
        }
    }
    let mut hash = blake3::Hasher::new();
    hash.update(PULL_REQUEST_CLOSURE_DIGEST_DOMAIN);
    hash.update(&[match o.path.1 {
        ClosureAction::Close => 0,
        ClosureAction::Withdraw => 1,
        ClosureAction::NewBranch => 2,
    }]);
    digest::field(&mut hash, o.item_id.to_string().as_bytes());
    digest::field(&mut hash, o.context_branch.as_bytes());
    digest::field(&mut hash, o.primary_branch.as_bytes());
    digest::remote(&mut hash, o.remote.as_deref());
    optional(&mut hash, head);
    closure_effect(&mut hash, &o.closure);
    if let Some(new_branch) = &o.new_branch {
        hash.update(o.base.as_bytes());
        optional(&mut hash, new_branch.retired_oid);
        optional(&mut hash, o.tracking_context);
        hash.update(&(new_branch.unheld.len() as u64).to_be_bytes());
        for entry in &new_branch.unheld {
            digest::field(&mut hash, &entry.path);
            hash.update(&[match entry.change {
                PathChange::Added => 0,
                PathChange::Modified => 1,
                PathChange::Deleted => 2,
            }]);
        }
    }
    ClosureExpectation::from_bytes(*hash.finalize().as_bytes())
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
