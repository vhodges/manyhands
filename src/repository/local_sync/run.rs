//! Internal executor, reached through the F2 request-fenced mutation bindings.
use super::{
    record::{LocalContextSyncRecord, LocalSyncState},
    recover::{self, Disposition, Execution, Owner},
};
use crate::{canonical::ItemId, repository::*, results::*};
use rusqlite::params;

#[cfg(test)]
type BoundaryHook = Box<dyn FnMut(FailurePoint)>;
#[cfg(test)]
thread_local! {pub(super) static BOUNDARY_HOOK: std::cell::RefCell<Option<BoundaryHook>>=const {std::cell::RefCell::new(None)};}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Reason {
    None,
    UnrelatedHistories,
    ImmutableClosureDisagreement,
    IdentityRequired,
    IndexPending,
    ForeignChange,
    AmbiguousBackend,
    AuthorityMissing,
    UnsupportedConflict,
    CanonicalConflict,
    OriginalResolutionRequired,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Report {
    pub code: ResultCode,
    pub reason: Reason,
    pub state: Option<LocalSyncState>,
    pub effects: Effects,
    pub required_request: Option<OperationId>,
    pub cancel_acknowledged: bool,
}
impl Report {
    #[cfg(test)]
    pub fn outcome(&self) -> Outcome {
        Outcome::of_mutation(false, self.code, &self.effects, false)
    }
}
#[derive(Debug)]
pub(in super::super) struct Fault {
    pub code: ResultCode,
    pub reason: Reason,
}
impl Fault {
    pub fn foreign() -> Self {
        Self {
            code: ResultCode::ExternalChange,
            reason: Reason::ForeignChange,
        }
    }
    pub fn ambiguous() -> Self {
        Self {
            code: ResultCode::RecoveryRequired,
            reason: Reason::AmbiguousBackend,
        }
    }
    pub fn authority() -> Self {
        Self {
            code: ResultCode::RecoveryRequired,
            reason: Reason::AuthorityMissing,
        }
    }
}
impl From<ReadError> for Fault {
    fn from(e: ReadError) -> Self {
        Self {
            code: e.code(),
            reason: match e.code() {
                ResultCode::IdentityRequired => Reason::IdentityRequired,
                ResultCode::OriginalRequestRequired => Reason::OriginalResolutionRequired,
                ResultCode::ExternalChange | ResultCode::RequestMismatch => Reason::ForeignChange,
                _ => Reason::AuthorityMissing,
            },
        }
    }
}
impl From<RepositoryError> for Fault {
    fn from(e: RepositoryError) -> Self {
        if e.source
            .as_deref()
            .and_then(|e| e.downcast_ref::<std::io::Error>())
            .is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
        {
            return Self {
                code: ResultCode::RepositoryInaccessible,
                reason: Reason::AuthorityMissing,
            };
        }
        ReadError::from(e).into()
    }
}
impl From<git2::Error> for Fault {
    fn from(_: git2::Error) -> Self {
        Self::ambiguous()
    }
}
impl From<std::io::Error> for Fault {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            Self {
                code: ResultCode::RepositoryInaccessible,
                reason: Reason::AuthorityMissing,
            }
        } else {
            Self::foreign()
        }
    }
}
impl From<integration::Error> for Fault {
    fn from(_: integration::Error) -> Self {
        Self::ambiguous()
    }
}

pub(super) fn identity(git: &git2::Repository) -> Result<git2::Signature<'static>, Fault> {
    let config = git.config()?;
    let identity = CommitIdentity {
        name: config.get_string("user.name").unwrap_or_default(),
        email: config.get_string("user.email").unwrap_or_default(),
    };
    validate_identity(&identity, RepositoryOperation::ContextSyncLocal).map_err(|_| Fault {
        code: ResultCode::IdentityRequired,
        reason: Reason::IdentityRequired,
    })?;
    git2::Signature::now(&identity.name, &identity.email).map_err(|_| Fault {
        code: ResultCode::IdentityRequired,
        reason: Reason::IdentityRequired,
    })
}
pub(super) fn classify(
    git: &git2::Repository,
    local: git2::Oid,
    incoming: git2::Oid,
) -> Result<Disposition, Fault> {
    use integration::prepare::{
        IntegrationClassificationError as E, IntegrationDisposition as D, classify_integration,
    };
    match classify_integration(
        local,
        incoming,
        |a, b| git.graph_descendant_of(b, a),
        |a, b| match git.merge_base(a, b) {
            Ok(_) => Ok(true),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(e) => Err(e),
        },
    ) {
        Ok(D::Equal | D::IncomingAlreadyIntegrated) => Ok(Disposition::Noop),
        Ok(D::FastForward) => Ok(Disposition::FastForward),
        Ok(D::MergeRequired) => Ok(Disposition::Merge),
        Err(E::UnrelatedHistories) => Err(Fault {
            code: ResultCode::RecoveryRequired,
            reason: Reason::UnrelatedHistories,
        }),
        Err(E::Ancestry(_)) => Err(Fault::ambiguous()),
    }
}
fn prepare(
    repo: &ResolvedRepository,
    observation: &LocalContextSyncObservationDto,
) -> Result<Execution, Fault> {
    let git = git2::Repository::open(&observation.context_worktree)?;
    let local = git2::Oid::from_str(&observation.context_head)?;
    let incoming = git2::Oid::from_str(&observation.primary_head)?;
    let disposition = classify(&git, local, incoming)?;
    if disposition != Disposition::Noop {
        recover::require_logging(&git)?;
    }
    recover::clean_metadata(&git, &observation.context_ref)?;
    if !recover::exact_image(&git, local)? {
        return Err(Fault {
            code: ResultCode::WorktreeNotClean,
            reason: Reason::ForeignChange,
        });
    }
    if disposition == Disposition::Merge {
        let index = integration::prepare::prepare_recorded_merge_index(&git, local, incoming)?;
        if index.has_conflicts() {
            let incompatible = index.conflicts()?.any(|c| {
                c.is_ok_and(|c| {
                    integration::validate::classify_conflict_entry(&git, &c).structure
                        == integration::validate::ConflictStructure::IncompatibleClosures
                })
            });
            if incompatible {
                return Err(Fault {
                    code: ResultCode::RecoveryRequired,
                    reason: Reason::ImmutableClosureDisagreement,
                });
            }
            // Conflicts still need the same complete prospective representation
            // checks below, including stage-zero additions and every stage mode.
            // Canonical eligibility alone is not checkout/image authority.
        } else {
            identity(&git)?;
        }
    }
    if disposition != Disposition::Noop {
        let prospective = if disposition == Disposition::Merge {
            integration::prepare::prepare_recorded_merge_index(&git, local, incoming)?
        } else {
            let mut index = git2::Index::new()?;
            index.read_tree(&git.find_commit(incoming)?.tree()?)?;
            index
        };
        let baseline = git.find_commit(local)?.tree()?;
        for entry in prospective.iter() {
            // The bounded executor proves regular-file checkout images only.
            // Unsupported structural input must not become retained ownership
            // whose prospective result can never satisfy that proof.
            if !matches!(entry.mode, 0o100644 | 0o100755) {
                return Err(Fault {
                    code: ResultCode::RecoveryRequired,
                    reason: Reason::UnsupportedConflict,
                });
            }
            let path = std::str::from_utf8(&entry.path).map_err(|_| Fault::foreign())?;
            if baseline.get_path(std::path::Path::new(path)).is_err() {
                let root = git.workdir().ok_or_else(Fault::foreign)?;
                match owned_file_bytes(
                    root,
                    std::path::Path::new(path),
                    RepositoryOperation::ContextSyncLocal,
                    root,
                ) {
                    Ok(None) => {}
                    _ => {
                        return Err(Fault {
                            code: ResultCode::WorktreeNotClean,
                            reason: Reason::ForeignChange,
                        });
                    }
                }
            }
        }
    }
    Execution::prepare(
        repo,
        &git,
        &observation.context_ref,
        disposition,
        (local, incoming),
    )
}

#[cfg(test)]
pub(super) fn execute(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    kind: AuthoringKind,
    item: &ItemId,
    id: OperationId,
    observation: &str,
) -> Report {
    execute_request(service, repo, kind, item, id, observation, None)
}
pub(super) fn execute_request<'a>(
    service: &'a RepositoryService,
    repo: &'a ResolvedRepository,
    kind: AuthoringKind,
    item: &ItemId,
    id: OperationId,
    observation: &str,
    request: Option<super::Fence<'a>>,
) -> Report {
    let mut preparation_reason = None;
    // This store looks for accepted intent BEFORE evaluating live observations.
    // The preparation callback runs under the common lease before acceptance.
    let row = service.begin_local_context_sync_with_request(
        repo,
        (id, kind, item, observation),
        request,
        |_, observation| {
            prepare(repo, observation).map(Some).map_err(|fault| {
                preparation_reason = Some(fault.reason);
                ReadError::new(fault.code)
            })
        },
    );
    let row = match row {
        Ok(row) => row,
        Err(error) => {
            let fault = Fault::from(error);
            // Input mismatch must keep this operation's historical effects.
            let evidence = read_evidence(service, repo, id).ok();
            return report(
                fault.code,
                preparation_reason.unwrap_or(fault.reason),
                evidence.as_ref().map(|(r, e)| (r, e)),
            );
        }
    };
    #[cfg(test)]
    BOUNDARY_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().as_mut() {
            hook(FailurePoint::LocalSyncBeforeExecuteLease);
        }
    });
    let mut owner = match (|| -> Result<Owner<'_>, Fault> {
        let git = git2::Repository::open(&row.worktree)?;
        let lease = repository_lease(&git, repo.root(), RepositoryOperation::ContextSyncLocal)?;
        Ok(Owner {
            request,
            service,
            repo,
            git,
            lease,
            row: row.clone(),
            known_execution: None,
            performed: recover::Performed::default(),
        })
    })() {
        Ok(owner) => owner,
        Err(fault) => {
            let evidence = read_evidence(service, repo, id).ok();
            return report(
                fault.code,
                fault.reason,
                evidence.as_ref().map(|(row, e)| (row, e)),
            );
        }
    };
    let result = drive(&mut owner);
    if result.is_err() {
        let _ = observe_object_progress(&mut owner);
        let _ = observe_checkout_progress(&mut owner);
    }
    // Loss of execution authority does not erase previously proven effects.
    // This read grants no takeover/write/settlement permission.
    let evidence = read_evidence(service, repo, id);
    let reply = match (result, evidence) {
        (Ok(code), Ok((row, e))) => report(code, Reason::None, Some((&row, &e))),
        (Err(fault), Ok((row, e))) => report(fault.code, fault.reason, Some((&row, &e))),
        (result, Err(error)) => {
            // A secondary evidence-read failure must not replace the error that
            // actually stopped execution or erase already known durable work.
            let fault = result.err().unwrap_or_else(|| Fault::from(error));
            report(
                fault.code,
                fault.reason,
                owner.known_execution.as_ref().map(|e| (&owner.row, e)),
            )
        }
    };
    let mut reply = with_performed(reply, &owner);
    if reply.code == ResultCode::OriginalRequestRequired {
        reply.required_request = super::resolve::required_request(service, id);
    }
    reply
}

pub(super) fn with_performed(mut reply: Report, owner: &Owner<'_>) -> Report {
    if owner.performed.wrote {
        reply.effects.write = WriteEffect::Written;
    }
    if owner.performed.committed {
        reply.effects.checkpoint = CheckpointEffect::Committed;
    }
    if owner.performed.integrated {
        reply.effects.integration = IntegrationEffect::Complete;
    }
    if let Some(oid) = owner.performed.oid {
        reply.effects.commit_oid = Some(oid.to_string());
    }
    reply
}

pub(super) fn read_evidence(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
) -> Result<(LocalContextSyncRecord, Execution), ReadError> {
    super::record::store_access(true)?;
    let (row,mut evidence)=service.read_session(RepositoryOperation::Read,|db| {
        super::record::audit(db).map_err(|_|ReadError::new(ResultCode::RecoveryRequired))?;
        let row=db.query_row("SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.id=?2 AND r.root_path=?3",params![id.to_string(),repo.registration_id(),repo.root().to_str()],super::record::decode)?;
        Ok((row,Execution::read(db,id)?))
    })?;
    let git = git2::Repository::open(repo.root())
        .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
    evidence
        .validate_graph(&git, &row)
        .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
    // Reporting only, never partial-install reconciliation authority. A changed
    // index/file image after recorded backend intent is retained as durable work.
    if evidence.conflict_started
        && !evidence.conflict_observed
        && let Ok(context) = git2::Repository::open(&row.worktree)
        && evidence.images.as_ref().is_some_and(|p| {
            p.matches(&context, row.context_parent)
                .is_ok_and(|same| !same)
        })
    {
        evidence.conflict_observed = true;
    }
    Ok((row, evidence))
}

pub(super) fn report(
    code: ResultCode,
    reason: Reason,
    evidence: Option<(&LocalContextSyncRecord, &Execution)>,
) -> Report {
    let mut effects = Effects::not_requested();
    if let Some((row, e)) = evidence {
        effects.write = if e.checkout_observed || e.conflict_observed {
            WriteEffect::Written
        } else {
            WriteEffect::Unchanged
        };
        effects.checkpoint = if e.commit_observed {
            CheckpointEffect::Committed
        } else if e.conflict_observed && !row.integration_proven {
            CheckpointEffect::Pending
        } else {
            CheckpointEffect::Unchanged
        };
        effects.integration = if e.disposition == Disposition::Noop {
            IntegrationEffect::NotRequested
        } else if row.integration_proven {
            IntegrationEffect::Complete
        } else {
            IntegrationEffect::Pending
        };
        effects.discovery = if row.index_complete {
            DiscoveryEffect::Current
        } else {
            DiscoveryEffect::Pending
        };
        // A planned merge OID may not have been imported yet. Its intent is
        // recovery authority, not evidence that a commit was actually made.
        effects.commit_oid = row
            .result
            .or_else(|| {
                if e.disposition == Disposition::Merge && !e.commit_observed {
                    None
                } else {
                    row.candidate
                }
            })
            .map(|o| o.to_string());
    }
    Report {
        code,
        reason,
        state: evidence.map(|(r, _)| r.state),
        effects,
        required_request: None,
        cancel_acknowledged: code == ResultCode::Cancelled,
    }
}
pub(super) fn boundary(owner: &Owner<'_>, point: FailurePoint) -> Result<(), Fault> {
    #[cfg(test)]
    BOUNDARY_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().as_mut() {
            hook(point);
        }
    });
    owner
        .service
        .check_failure(
            point,
            RepositoryOperation::ContextSyncLocal,
            owner.repo.root(),
        )
        .map_err(|_| Fault::ambiguous())
}

fn observe_checkout_progress(owner: &mut Owner<'_>) -> Result<(), Fault> {
    owner.authorize()?;
    let e = owner.evidence()?;
    if e.checkout_started && !e.checkout_observed && recover::checkout_progress(owner)? {
        owner.flag("checkout_observed")?;
    }
    Ok(())
}
fn observe_object_progress(owner: &mut Owner<'_>) -> Result<(), Fault> {
    // The shared authority audit also checks active-R candidate ownership here;
    // error reporting must not promote an intent that resolution just refused.
    owner.authorize()?;
    let e = owner.evidence()?;
    if e.disposition == Disposition::Merge
        && !e.commit_observed
        && let Some(oid) = owner.row.candidate
        && owner.git.find_commit(oid).is_ok()
    {
        validate_candidate(owner, &e)?;
        owner.step(0, "objects", true, Some(oid))?;
    }
    Ok(())
}

fn drive(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
    let e = owner.evidence()?;
    if matches!(
        owner.row.state,
        LocalSyncState::Cancelled | LocalSyncState::Abandoned
    ) {
        return Err(Fault {
            code: ResultCode::Cancelled,
            reason: Reason::Cancelled,
        });
    }
    if owner.row.state == LocalSyncState::Complete {
        return Ok(if e.disposition == Disposition::Noop {
            ResultCode::AlreadyApplied
        } else {
            ResultCode::Ok
        });
    }
    owner.authorize()?;
    boundary(owner, FailurePoint::LocalSyncAfterIntent)?;
    super::control::safe_point(owner)?;
    if e.disposition == Disposition::Merge && super::conflict::has_conflicts(owner)? {
        if owner.row.active_resolution.is_some()
            || owner.row.integration_proven
            || owner.git.refname_to_id(&owner.row.context_ref)? != owner.row.context_parent
        {
            return super::resolve::continue_without_source(owner);
        }
        return super::conflict::install(owner);
    }
    if owner.row.active_resolution.is_some()
        || !matches!(
            owner.row.state,
            LocalSyncState::Prepared
                | LocalSyncState::CommitPrepared
                | LocalSyncState::Applying
                | LocalSyncState::Integrated
                | LocalSyncState::IndexPending
        )
    {
        return Err(Fault::authority());
    }
    owner.authorize()?;
    owner.transaction(|tx,row| {
        let next=row.owner_epoch.checked_add(1).ok_or_else(||ReadError::new(ResultCode::RecoveryRequired))?;
        if tx.execute("UPDATE local_context_sync_records SET owner_epoch=?2 WHERE operation_ulid=?1 AND owner_epoch=?3 AND generation=?4",params![row.operation_id.to_string(),next,row.owner_epoch,row.generation])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
        Ok(())
    })?;
    boundary(owner, FailurePoint::LocalSyncAfterIntent)?;
    observe_checkout_progress(owner)?;
    if !owner.row.integration_proven {
        integrate(owner)?;
    }
    super::control::safe_point(owner)?;
    owner.authorize()?;
    let expected = owner.row.result.unwrap_or(owner.row.context_parent);
    recover::clean_metadata(&owner.git, &owner.row.context_ref)?;
    if owner.git.refname_to_id(&owner.row.context_ref)? != expected
        || !recover::owned_image(owner, expected)?
    {
        return Err(Fault::foreign());
    }
    owner.phase("index_pending")?;
    super::control::safe_point(owner)?;
    boundary(owner, FailurePoint::LocalSyncBeforeIndex).map_err(|_| Fault {
        code: ResultCode::DiscoveryPending,
        reason: Reason::IndexPending,
    })?;
    index(owner).map_err(|fault| {
        if fault.code == ResultCode::Cancelled {
            fault
        } else {
            Fault {
                code: ResultCode::DiscoveryPending,
                reason: Reason::IndexPending,
            }
        }
    })?;
    boundary(owner, FailurePoint::LocalSyncAfterIndex)?;
    Ok(if e.disposition == Disposition::Noop {
        ResultCode::AlreadyApplied
    } else {
        ResultCode::Ok
    })
}

fn integrate(owner: &mut Owner<'_>) -> Result<(), Fault> {
    owner.authorize()?;
    let e = owner.evidence()?;
    recover::clean_metadata(&owner.git, &owner.row.context_ref)?;
    let head = owner.git.refname_to_id(&owner.row.context_ref)?;
    if e.disposition == Disposition::Noop {
        if head != owner.row.context_parent
            || !recover::owned_image(owner, head)?
            || !recover::ref_proof(owner, &e, false)?
        {
            return Err(Fault::foreign());
        }
        return integrated(owner, None);
    }
    if owner.row.candidate.is_none() || e.disposition == Disposition::Merge && !e.commit_observed {
        if head != owner.row.context_parent
            || !recover::owned_image(owner, head)?
            || !recover::ref_proof(owner, &e, false)?
        {
            return Err(Fault::foreign());
        }
        let candidate = match e.disposition {
            Disposition::FastForward => owner.row.primary_parent,
            Disposition::Merge => candidate(owner)?,
            Disposition::Noop => unreachable!(),
        };
        owner.transaction(|tx,row| {tx.execute("UPDATE local_context_sync_records SET candidate_oid=?2,state='commit_prepared',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),candidate.to_string()])?;Ok(())})?;
        boundary(owner, FailurePoint::LocalSyncAfterCandidate)?;
    }
    let candidate = owner.row.candidate.ok_or_else(Fault::authority)?;
    let e = owner.evidence()?;
    validate_candidate(owner, &e)?;
    if head == candidate {
        if !e.ref_started
            || !recover::owned_image(owner, candidate)?
            || !recover::ref_proof(owner, &e, true)?
        {
            return Err(recover::mismatch_reason(&e));
        }
        owner.flag("ref_observed")?;
        owner.step(2, "reference", true, Some(candidate))?;
        return integrated(owner, Some(candidate));
    }
    if head != owner.row.context_parent || !recover::ref_proof(owner, &e, false)? {
        return Err(recover::mismatch_reason(&e));
    }
    let baseline = recover::owned_image(owner, owner.row.context_parent)?;
    let result = recover::owned_image(owner, candidate)?;
    if !baseline && !(e.checkout_started && result) {
        return Err(recover::mismatch_reason(&e));
    }
    if !result {
        if e.checkout_baseline.is_none() {
            let baseline = recover::checkout_image(owner)?;
            owner.transaction(|tx,row| {
                if tx.execute("UPDATE local_context_sync_execution SET baseline_worktree=?2 WHERE operation_ulid=?1 AND baseline_worktree IS NULL",params![row.operation_id.to_string(),baseline.as_slice()])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
                Ok(())
            })?;
        }
        if e.images
            .as_ref()
            .is_some_and(|p| p.coarse_version == 2 && p.baseline_v2.is_none())
        {
            let baseline = recover::checkout_image_v2(owner)?;
            owner.transaction(|tx,row| {
                if tx.execute("UPDATE local_context_sync_images SET baseline_v2=?2 WHERE operation_ulid=?1 AND baseline_v2 IS NULL",params![row.operation_id.to_string(),baseline.as_slice()])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}Ok(())
            })?;
        }
        owner.step(1, "checkout", false, Some(candidate))?;
        owner.flag("checkout_started")?;
        owner.phase("applying")?;
        owner.authorize()?;
        checkout(owner, candidate)?;
    }
    // Equal trees still need the ref transition, but no checkout or file-write
    // effect. A previously started checkout may instead be reconciling its
    // already-complete image and retains that historical write evidence.
    if !result || e.checkout_started {
        owner.flag("checkout_observed")?;
        owner.step(1, "checkout", true, Some(candidate))?;
    }
    boundary(owner, FailurePoint::LocalSyncAfterCheckout)?;
    super::control::safe_point(owner)?;
    owner.step(2, "reference", false, Some(candidate))?;
    owner.flag("ref_started")?;
    owner.authorize()?;
    transition(owner, candidate, &e)?;
    boundary(owner, FailurePoint::LocalSyncAfterRef)?;
    owner.flag("ref_observed")?;
    owner.step(2, "reference", true, Some(candidate))?;
    integrated(owner, Some(candidate))
}

fn candidate(owner: &mut Owner<'_>) -> Result<git2::Oid, Fault> {
    let evidence = owner.evidence()?;
    let sig = match evidence.commit_identity {
        Some((name, email, seconds, offset)) => {
            git2::Signature::new(&name, &email, &git2::Time::new(seconds, offset))?
        }
        None => identity(&owner.git)?,
    };
    let worker = git2::Repository::open(owner.git.path())?;
    let odb = worker.odb()?;
    let _memory = odb.add_new_mempack_backend(1000)?;
    let prepared = match integration::prepare::prepare_clean_merge_in(
        &worker,
        owner.git.path(),
        owner.row.context_parent,
        owner.row.primary_parent,
    )? {
        integration::prepare::MergePreparation::Clean(tree) => tree,
        integration::prepare::MergePreparation::Conflict => return Err(Fault::authority()),
    };
    let tree_id = integration::prepare::import_prepared_tree(&worker, &prepared)?;
    let tree = worker.find_tree(tree_id)?;
    let local = worker.find_commit(owner.row.context_parent)?;
    let incoming = worker.find_commit(owner.row.primary_parent)?;
    let oid = worker.commit(
        None,
        &sig,
        &sig,
        "Synchronize local primary into editing context",
        &tree,
        &[&local, &incoming],
    )?;
    if !integration::validate::validates_external_integration(
        &worker,
        local.id(),
        incoming.id(),
        oid,
    )? {
        return Err(Fault::authority());
    }
    if !evidence
        .images
        .as_ref()
        .ok_or_else(Fault::authority)?
        .filters_match(&worker, oid)?
    {
        return Err(Fault::foreign());
    }
    if let Some(recorded) = owner.row.candidate {
        if recorded != oid {
            return Err(Fault::authority());
        }
    } else {
        let index = recover::index_digest(&owner.git)?;
        owner.transaction(|tx,row| {
            tx.execute("UPDATE local_context_sync_records SET candidate_oid=?2,state='commit_prepared',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),oid.to_string()])?;
            tx.execute("UPDATE local_context_sync_execution SET commit_name=?2,commit_email=?3,commit_time=?4,commit_offset=?5 WHERE operation_ulid=?1",params![row.operation_id.to_string(),sig.name(),sig.email(),sig.when().seconds(),sig.when().offset_minutes()])?;
            let empty=blake3::hash(b"local-sync-no-merge-metadata-v1");
            tx.execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest,expected_old_oid,expected_new_oid) VALUES(?1,0,?2,?3,'objects','prepared',?4,?5,?6,?7)",params![row.operation_id.to_string(),row.generation,row.owner_epoch,index.as_slice(),empty.as_bytes().as_slice(),row.context_parent.to_string(),oid.to_string()])?;
            Ok(())
        })?;
    }
    boundary(owner, FailurePoint::LocalSyncBeforeObjectImport)?;
    owner.authorize()?;
    // Recheck real worktree attribute sources after the interruption boundary,
    // before importing even the owned candidate objects.
    evidence
        .images
        .as_ref()
        .ok_or_else(Fault::authority)?
        .require_worktree_attributes(&owner.git)?;
    integration::prepare::import_owned_tree::<Fault>(&owner.git, &prepared, || Ok(()))?;
    let object = odb.read(oid)?;
    owner.authorize()?;
    if owner
        .git
        .odb()?
        .write(git2::ObjectType::Commit, object.data())?
        != oid
    {
        return Err(Fault::authority());
    }
    owner.performed.committed = true;
    owner.performed.oid = Some(oid);
    boundary(owner, FailurePoint::LocalSyncAfterObjectImport)?;
    owner.step(0, "objects", true, Some(oid))?;
    Ok(oid)
}
pub(super) fn validate_candidate(owner: &Owner<'_>, e: &Execution) -> Result<(), Fault> {
    validate_candidate_in(&owner.git, &owner.row, e)
}
pub(super) fn validate_candidate_in(
    git: &git2::Repository,
    row: &LocalContextSyncRecord,
    e: &Execution,
) -> Result<(), Fault> {
    let oid = row.candidate.ok_or_else(Fault::authority)?;
    let commit = git.find_commit(oid)?;
    match e.disposition {
        Disposition::FastForward if oid == row.primary_parent => Ok(()),
        Disposition::Merge => {
            let (name, email, seconds, offset) =
                e.commit_identity.as_ref().ok_or_else(Fault::authority)?;
            let signature_matches = |s: git2::Signature<'_>| {
                s.name() == Some(name.as_str())
                    && s.email() == Some(email.as_str())
                    && s.when().seconds() == *seconds
                    && s.when().offset_minutes() == *offset
            };
            if commit.parent_ids().collect::<Vec<_>>() != [row.context_parent, row.primary_parent]
                || !signature_matches(commit.author())
                || !signature_matches(commit.committer())
                || !integration::validate::validates_external_integration(
                    git,
                    row.context_parent,
                    row.primary_parent,
                    oid,
                )?
            {
                return Err(Fault::authority());
            }
            Ok(())
        }
        _ => Err(Fault::authority()),
    }
}
fn checkout(owner: &mut Owner<'_>, candidate: git2::Oid) -> Result<(), Fault> {
    let git = git2::Repository::open(owner.git.path())?;
    let (service, root) = (owner.service, owner.repo.root());
    let result = integration::artifacts::local_checkout(
        &git,
        candidate,
        || {
            owner.authorize()?;
            recover::frozen_filters(owner, candidate)?;
            boundary(owner, FailurePoint::LocalSyncBeforeCheckoutBackend)
        },
        || {
            #[cfg(test)]
            BOUNDARY_HOOK.with(|slot| {
                if let Some(hook) = slot.borrow_mut().as_mut() {
                    hook(FailurePoint::LocalSyncDuringCheckout);
                }
            });
            service
                .check_failure(
                    FailurePoint::LocalSyncDuringCheckout,
                    RepositoryOperation::ContextSyncLocal,
                    root,
                )
                .map_err(|_| Fault::ambiguous())
        },
    );
    // This test seam also restores child-only OS write limits on returned errors.
    let after = boundary(owner, FailurePoint::LocalSyncAfterCheckoutBackend);
    if result.is_ok() || recover::checkout_progress(owner).unwrap_or(false) {
        owner.performed.wrote = true;
        owner.performed.oid = Some(candidate);
    }
    result.and(after)
}
fn transition(owner: &mut Owner<'_>, candidate: git2::Oid, e: &Execution) -> Result<(), Fault> {
    if !recover::owned_image(owner, candidate)? {
        return Err(Fault::foreign());
    }
    recover::frozen_filters(owner, candidate)?;
    let git = git2::Repository::open(owner.git.path())?;
    let branch = owner.row.context_ref.clone();
    let old = owner.row.context_parent;
    let signature = git2::Signature::new(
        "Manyhands",
        "local-sync@manyhands.invalid",
        &git2::Time::new(e.ref_time, 0),
    )?;
    let message = format!("manyhands local sync {}", owner.row.operation_id);
    let (service, root) = (owner.service, owner.repo.root());
    integration::artifacts::local_reference(
        &git,
        (&branch, old, candidate),
        &signature,
        &message,
        || owner.authorize(),
        || {
            service
                .check_failure(
                    FailurePoint::LocalSyncInsideRefWindow,
                    RepositoryOperation::ContextSyncLocal,
                    root,
                )
                .map_err(|_| Fault::ambiguous())
        },
    )?;
    owner.performed.integrated = true;
    owner.performed.oid = Some(candidate);
    Ok(())
}
fn integrated(owner: &mut Owner<'_>, result: Option<git2::Oid>) -> Result<(), Fault> {
    owner.authorize()?;
    let e = owner.evidence()?;
    if result.is_some() && (!e.ref_observed || !recover::ref_proof(owner, &e, true)?) {
        return Err(Fault::ambiguous());
    }
    owner.transaction(|tx,row| {tx.execute("UPDATE local_context_sync_records SET state='integrated',result_oid=?2,integration_proven=1,artifacts_released=1,semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),result.map(|o|o.to_string())])?;Ok(())})
}
pub(super) fn index(owner: &mut Owner<'_>) -> Result<(), Fault> {
    owner.authorize()?;
    let git = git2::Repository::open(owner.repo.root())?;
    let observed_from = time::OffsetDateTime::now_utc().unix_timestamp();
    let before = observe_root(&git, owner.repo.root());
    boundary(owner, FailurePoint::BeforeIndexTransactionCommit)?;
    super::control::safe_point(owner)?;
    let after = observe_root(&git, owner.repo.root());
    if !changed_contexts(&before, &after, owner.repo.root()).is_empty() {
        return Err(Fault::foreign());
    }
    // Matching discovery snapshots do not establish integration authority: both
    // may observe the same foreign ref/log/lock state. Recheck the actual frozen
    // disposition/release proof after all index boundaries, before terminal SQL.
    recover::verify_released_integration(owner)?;
    let root = owner.repo.root();
    let authoritative = owner.request.is_some();
    let cancelled=owner.transaction(|tx,row| {
        if super::control::acknowledge_pending(tx,row,authoritative)? {return Ok(true);}
        persist_observation(tx,row.repository_id,&before,root)?;
        tx.execute("UPDATE repositories SET accessibility='accessible',refresh_required=0,refreshed_at=?2 WHERE id=?1",params![row.repository_id,observed_from])?;
        tx.execute("UPDATE local_context_sync_records SET state='complete',index_complete=1,semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",[row.operation_id.to_string()])?;
        if let Some(attempt)=row.active_resolution
            && tx.execute("UPDATE local_context_sync_resolutions SET state='complete' WHERE attempt_ulid=?1 AND operation_ulid=?2",params![attempt.to_string(),row.operation_id.to_string()])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
        tx.execute("UPDATE operation_records SET state='completed',completed_step='completed' WHERE id=?1",[row.local_record_id])?;
        Ok(false)
    })?;
    if cancelled {
        Err(Fault {
            code: ResultCode::Cancelled,
            reason: Reason::Cancelled,
        })
    } else {
        Ok(())
    }
}
