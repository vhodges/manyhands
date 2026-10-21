//! The closure observation: the promotion observation over a ticket context,
//! plus pending local work (C12), the closure effect, the integration table,
//! the post-closure ticket check (D1, C10) and identity (C2). It never takes
//! the repository lease, writes Git or SQL, or opens the network.
use super::{
    ClosureBlocker, ClosureExpectation, PlannedClosure, PreviewTicketClosureRequest,
    TicketClosurePreflight, TicketClosurePreview,
    checkpoint::{ClosureBuild, ClosureTarget, MergeHandle, build_closure},
};
use crate::{
    canonical::{self, CanonicalItem, ItemId, Ticket},
    repository::{
        CommitIdentity, ConfirmedCommitIdentity, ExpectedConfiguration, PlannedCleanup,
        PlannedIntegration, PlannedPublication, RemoteSnapshot, RepositoryError, RepositoryService,
        SynchronizationError, SynchronizationTarget,
        bridges::abandon::authoring_item,
        promotion::{
            lifecycle::LifecycleKind,
            observe::{
                self as shared, ChangeEvidence, Cleanliness, ContextPresence, ObservationSeal,
                PlannedResult, PrimaryPresence, RemoteContextHistory, digest,
            },
        },
        remote::sync::{self, CommittingIdentity},
    },
};
use git2::Oid;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, UtcOffset};

const KIND: LifecycleKind = LifecycleKind::TicketClosure;

/// The closure digest's domain; promotion's (`manyhands promotion v1`) is
/// unchanged.
pub(in super::super) const CLOSURE_DIGEST_DOMAIN: &[u8] = b"manyhands closure v1";

/// The ticket's one canonical path.
pub(in super::super) fn ticket_path(item: &ItemId) -> PathBuf {
    PathBuf::from(format!(".manyhands/tickets/{item}/ticket.md"))
}

/// `closed_by` as the closure records it: `"<name> <<email>>"`.
pub(in super::super) fn closed_by_text(identity: &CommitIdentity) -> String {
    format!("{} <{}>", identity.name, identity.email)
}

/// Why `closed_ticket_bytes` refuses a ticket. Categories only: no content.
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
    let source = std::str::from_utf8(original).map_err(|_| NotText)?;
    let id = canonical::front_matter(source)
        .and_then(|values| values.get("id")?.as_str()?.parse::<ItemId>().ok())
        .ok_or(NotATicket)?;
    let path = ticket_path(&id);
    let parse = |source: &str| match canonical::parse_item(&path, source) {
        Ok(CanonicalItem::Ticket(ticket)) => Ok(ticket),
        _ => Err(NotATicket),
    };
    let ticket = parse(source)?;
    if ticket.closed_at.is_some() || ticket.closed_by.is_some() {
        return Err(AlreadyClosed);
    }
    if closed_at.offset() != UtcOffset::UTC || closed_at.nanosecond() != 0 {
        return Err(InvalidTime);
    }
    if closed_by.is_empty() || closed_by.contains('\0') {
        return Err(InvalidClosedBy);
    }
    let serialize = |ticket: &Ticket| {
        canonical::serialize_item(&CanonicalItem::Ticket(ticket.clone())).map_err(|_| NotPreserved)
    };
    let normalized = parse(&serialize(&ticket)?).map_err(|_| NotPreserved)?;
    if !preserved(&ticket, &normalized) {
        return Err(NotPreserved);
    }
    let mut closed = normalized;
    closed.closed_at = Some(closed_at);
    closed.closed_by = Some(closed_by.to_owned());
    let bytes = serialize(&closed)?;
    if parse(&bytes).map_err(|_| NotPreserved)? != closed {
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
    /// The selected identity of a `Record` closure; `None` for `Existing`,
    /// whose merge signer is resolved at execution.
    #[cfg_attr(not(test), allow(dead_code))]
    // Observed for the tests; execution re-selects under the lease.
    pub(in super::super) identity: Option<CommittingIdentity>,
    pub(in super::super) expectation: ClosureExpectation,
    pub(in super::super) context_branch: String,
    pub(in super::super) primary_branch: String,
    pub(in super::super) primary_oid: Oid,
    pub(in super::super) tracking_primary: Option<Oid>,
    pub(in super::super) tracking_context: Option<Oid>,
}
impl ClosureObservation {
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
        }
    }
}

/// A refused observation: every blocker, and the identity configuration
/// exactly when the only blocker is `IdentityRequired`.
pub(in super::super) struct ClosureRefusal {
    pub(in super::super) blockers: Vec<ClosureBlocker>,
    pub(in super::super) identity_configuration: Option<ExpectedConfiguration>,
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
#[derive(Clone, Copy, Default)]
struct PendingWork {
    local_operation: bool,
    comment_publication: bool,
}

/// Prepared outside the lease (discovery), revalidated inside it without
/// discovery (Task 6 SPEC P2), as promotion's `PreparedObservation`.
pub(in super::super) struct PreparedClosureObservation {
    pub(in super::super) observed: Result<ClosureObservation, ClosureRefusal>,
    pub(in super::super) configuration: canonical::RepositoryConfig,
    pub(in super::super) repository_id: i64,
    seal: ObservationSeal,
    facts: DiscoveryFacts,
    confirmation: Option<ConfirmedCommitIdentity>,
}
impl PreparedClosureObservation {
    pub(in super::super) fn preflight(&self) -> TicketClosurePreflight {
        into_preflight(&self.observed)
    }
}

pub(super) fn preview(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
) -> Result<TicketClosurePreflight, RepositoryError> {
    Ok(prepare_observation(service, request)?.preflight())
}

/// Outside the lease: discovery, then a full observation.
pub(in super::super) fn prepare_observation(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
) -> Result<PreparedClosureObservation, RepositoryError> {
    prepare_observation_at(service, request, now())
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("zero nanoseconds are valid")
}

fn prepare_observation_at(
    service: &RepositoryService,
    request: &PreviewTicketClosureRequest,
    at: OffsetDateTime,
) -> Result<PreparedClosureObservation, RepositoryError> {
    let item = &request.item_id;
    let (repository, root, config) = shared::open_observation(KIND, service, &request.root)?;
    // One read-only registry transaction: history, conflicts and pending work.
    let (repository_id, snapshot, pending_conflict, pending) =
        shared::read_view_with(service, &root, |tx, id| pending_work(tx, id, item))?;
    let (facts, seal) = ObservationSeal::discover(KIND, &repository, &root, &config, item, || {
        let discovered = super::super::discovery::observe_root(&repository, &root);
        let active = discovered
            .active_contexts
            .iter()
            .find(|a| a.context.item_id.as_ref() == Some(item));
        let is_document = |items: &[super::super::discovery::ObservedItem]| {
            items
                .iter()
                .any(|i| i.id == *item && i.kind == super::super::AuthoringKind::Document)
        };
        DiscoveryFacts {
            not_ticket: active.is_some_and(|a| is_document(&a.items))
                || (active.is_none() && is_document(&discovered.items)),
            ticket_path: active
                .filter(|a| a.context.branch.as_deref() == Some(KIND.context_branch(item).as_str()))
                .and_then(|a| {
                    a.items
                        .iter()
                        .find(|i| i.id == *item && i.kind == KIND.authoring())
                })
                .map(|i| i.path.clone()),
        }
    })?;
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
        at,
    )?;
    Ok(PreparedClosureObservation {
        observed,
        configuration: config,
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
    let fresh = prepared.seal.reopen(repository, &prepared.configuration)?;
    let item = prepared.seal.item();
    let (id, snapshot, pending_conflict, pending) =
        shared::read_view_with(service, prepared.seal.root(), |tx, id| {
            pending_work(tx, id, item)
        })?;
    if id != prepared.repository_id {
        return Err(shared::changed());
    }
    Ok(observe_git(
        &fresh,
        prepared.seal.root(),
        &prepared.configuration,
        item,
        &snapshot,
        pending_conflict,
        pending,
        &prepared.facts,
        prepared.confirmation.as_ref(),
        now(),
    )?
    .map_err(|refusal| refusal.blockers))
}

/// C12. `operation_records.item_id` is null for lifecycle rows: a row's
/// ticket comes from its target (R-T0-4), parsed by the abandon bridge's own
/// parser. A comment binding is unfinished while its local row or its
/// publication's remote row is.
fn pending_work(
    tx: &rusqlite::Transaction<'_>,
    repository_id: i64,
    item: &ItemId,
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
        .any(|target| authoring_item(target.as_deref()).as_deref() == Some(item_text.as_str()));
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
    Ok(PendingWork {
        local_operation,
        comment_publication,
    })
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
    at: OffsetDateTime,
) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError> {
    use ClosureBlocker::*;
    let mut blockers = BTreeSet::new();
    let path = ticket_path(item);
    let context_branch = KIND.context_branch(item);
    let mut context_oid = None;
    let mut context_missing = false;
    let mut context_conforming = false;
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
            ContextPresence::Missing => context_missing = true,
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
        if remote.primary_ahead {
            blockers.insert(PrimaryAheadOfRemote);
        }
        if remote.context_behind {
            blockers.insert(ContextBehindRemote);
        }
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
        (Some(base), false) => {
            match tree_ticket(&worker, commit_tree(&worker, base, root)?, &path, root)? {
                Some(ticket) => file_closure(&ticket.bytes),
                None => FileClosure::Open,
            }
        }
        _ => FileClosure::Open,
    };
    let base_closed = !matches!(base_closure, FileClosure::Open);
    // The context's copy, from its branch tip.
    let context_ticket = match (context_oid, context_conforming) {
        (Some(context), true) => {
            match tree_ticket(&worker, commit_tree(&worker, context, root)?, &path, root)? {
                Some(ticket) if ticket.canonical_entry => Some(ticket),
                _ => {
                    blockers.insert(ContextNonconforming);
                    None
                }
            }
        }
        _ => None,
    };
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
    // D5: an absent tracking context that was published means another clone
    // deleted it. Only a context child could recreate it, and none runs for a
    // contained context.
    if config.publication_remote.is_some()
        && tracking_context.is_none()
        && !contained
        && !facts.not_ticket
    {
        match shared::remote_context_history(snapshot, KIND, item) {
            RemoteContextHistory::Deleted => {
                blockers.insert(RemoteContextDeleted);
            }
            RemoteContextHistory::Unknown => {
                blockers.insert(HistoryUnknown);
            }
            RemoteContextHistory::NeverPublished => {}
        }
    }

    // C2: identity, read-only, and only for a `Record` closure (it needs
    // `closed_by`); an `Existing` closure resolves its merge signer at
    // execution, so no selection or selection error applies to it here.
    // Effective wins; else a configuration-bound
    // confirmation; else the expected configuration for `IdentityRequired`.
    let selection = if matches!(effect, Some(Effect::Record)) {
        Some(sync::select_committing_identity(
            repository,
            confirmation,
            &SynchronizationTarget::Primary,
        ))
    } else {
        None
    };
    let mut identity_configuration = None;
    let identity = match selection {
        None => None,
        Some(Ok(identity)) => Some(identity),
        Some(Err(SynchronizationError::IdentityRequired {
            expected_configuration,
            ..
        })) => {
            identity_configuration = Some(expected_configuration);
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
                }) => identity_configuration = Some(expected_configuration),
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
    let closure = match effect {
        Some(Effect::Record) => match selected {
            Some(closed_by) => Some(PlannedClosure::Record { closed_by }),
            None => {
                // Only when nothing else blocks, and only for `Record`: it
                // needs `closed_by` for the digest.
                if blockers.is_empty() {
                    blockers.insert(IdentityRequired);
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
    };
    if !blockers.is_empty() {
        let only_identity = blockers.len() == 1 && blockers.contains(&IdentityRequired);
        return Ok(Err(ClosureRefusal {
            blockers: blockers.into_iter().collect(),
            identity_configuration: identity_configuration.filter(|_| only_identity),
        }));
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
    };
    observation.expectation = expectation(&observation);
    Ok(Ok(observation))
}

/// `blake3("manyhands closure v1" ‖ fields)`, fixed-width and
/// length-prefixed: item, ticket path, branches, remote, integration kind,
/// the sorted pre-closure change set (T2-1), the closure tag and `closed_by`
/// (and `closed_at` exactly as in the file for `existing`), the cleanup set,
/// and only without a remote the local primary and context OIDs (D6).
/// `closed_at` of a `record` closure is never bound (C6).
fn expectation(o: &ClosureObservation) -> ClosureExpectation {
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
    match &o.closure {
        PlannedClosure::Record { closed_by } => {
            hash.update(&[0]);
            digest::field(&mut hash, closed_by.as_bytes());
        }
        PlannedClosure::Existing {
            closed_at,
            closed_by,
        } => {
            hash.update(&[1]);
            digest::field(&mut hash, closed_by.as_bytes());
            digest::field(&mut hash, closed_at.as_bytes());
        }
    }
    digest::cleanup(&mut hash, o.cleanup);
    digest::local_oids(&mut hash, o.remote.as_deref(), o.primary_oid, o.context_oid);
    ClosureExpectation::from_bytes(*hash.finalize().as_bytes())
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
