//! Canonical complete-set resolution, with source-free durable path intents.
//! Internal execution carries F2 association/attempt fencing through its owner.
use super::{
    conflict,
    record::LocalSyncState,
    recover::{self, Owner},
    run::{self, Fault, Reason, Report},
};
use crate::{repository::*, results::*};
use rusqlite::{OptionalExtension, params};
use std::{collections::BTreeMap, path::Path};

// Source deliberately has no formatting or serialization implementation.
pub(super) struct Source {
    pub token: String,
    pub source: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    InstallReturned,
    Accepted,
    Path(usize),
    CandidateIntent,
    CandidateImported,
    Candidate,
    Index,
    Ref,
    Retired(usize),
}
#[cfg(test)]
type Hook = Box<dyn FnMut(Point) -> Result<(), std::io::Error>>;
#[cfg(test)]
thread_local! { pub(super) static HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
thread_local! {
    pub(super) static WRITE_FAULT: std::cell::Cell<Option<std::io::ErrorKind>>=const {std::cell::Cell::new(None)};
    pub(super) static WRITE_ATTEMPTS: std::cell::Cell<usize>=const {std::cell::Cell::new(0)};
}
fn write_path(owner: &Owner<'_>, path: &Path, bytes: &[u8], before: [u8; 32]) -> Result<(), Fault> {
    // Typed fault at the actual owned-file I/O seam, after authority validation.
    #[cfg(test)]
    WRITE_ATTEMPTS.with(|n| n.set(n.get() + 1));
    #[cfg(test)]
    if let Some(kind) = WRITE_FAULT.with(|f| f.take()) {
        let error = std::io::Error::from(kind);
        return Err(Fault {
            code: if error.kind() == std::io::ErrorKind::PermissionDenied {
                ResultCode::RepositoryInaccessible
            } else {
                ResultCode::RecoveryRequired
            },
            reason: Reason::AuthorityMissing,
        });
    }
    write_owned_document_if_prewrite_digest(
        &owner.row.worktree,
        path,
        bytes,
        before,
        RepositoryOperation::ContextSyncLocal,
        owner.repo.root(),
    )
    .map_err(Fault::from)
}
pub(super) fn boundary(point: Point) -> Result<(), Fault> {
    #[cfg(not(test))]
    {
        let _ = point;
        Ok(())
    }
    #[cfg(test)]
    HOOK.with(|h| h.borrow_mut().as_mut().map_or(Ok(()), |h| h(point)))
        .map_err(|e| Fault {
            code: if e.kind() == std::io::ErrorKind::PermissionDenied {
                ResultCode::RepositoryInaccessible
            } else {
                ResultCode::RecoveryRequired
            },
            reason: Reason::AuthorityMissing,
        })
}
fn invalid() -> Fault {
    Fault {
        code: ResultCode::InvalidInput,
        reason: Reason::ForeignChange,
    }
}
fn digest(
    repo: &ResolvedRepository,
    id: OperationId,
    observation: &str,
    sources: &[Source],
) -> Result<[u8; 32], Fault> {
    let mut pairs = BTreeMap::new();
    for source in sources {
        if pairs.insert(&source.token, &source.source).is_some() {
            return Err(invalid());
        }
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands-local-resolution-input-v1");
    for field in [
        b"context resolve-local".as_slice(),
        repo.root().as_os_str().as_encoded_bytes(),
        id.to_string().as_bytes(),
        observation.as_bytes(),
    ] {
        super::observe::field(&mut hash, field);
    }
    for (token, source) in pairs {
        super::observe::field(&mut hash, token.as_bytes());
        super::observe::field(&mut hash, source.as_bytes());
    }
    Ok(*hash.finalize().as_bytes())
}
#[cfg(test)]
pub(super) fn owner<'a>(
    service: &'a RepositoryService,
    repo: &'a ResolvedRepository,
    id: OperationId,
) -> Result<Owner<'a>, Fault> {
    owner_request(service, repo, id, None)
}
pub(super) fn owner_request<'a>(
    service: &'a RepositoryService,
    repo: &'a ResolvedRepository,
    id: OperationId,
    request: Option<super::Fence<'a>>,
) -> Result<Owner<'a>, Fault> {
    let row = run::read_evidence(service, repo, id)?.0;
    let git = git2::Repository::open(&row.worktree)?;
    let lease = repository_lease(&git, repo.root(), RepositoryOperation::ContextSyncLocal)?;
    let mut owner = Owner {
        request,
        service,
        repo,
        git,
        lease,
        row,
        known_execution: None,
        performed: recover::Performed::default(),
    };
    owner.authorize()?;
    Ok(owner)
}
#[cfg(test)]
pub(super) fn execute(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
    request: OperationId,
    observation: &str,
    sources: &[Source],
) -> Report {
    execute_request(service, repo, id, request, observation, sources, None)
}
pub(super) fn execute_request<'a>(
    service: &'a RepositoryService,
    repo: &'a ResolvedRepository,
    id: OperationId,
    request: OperationId,
    observation: &str,
    sources: &[Source],
    fence: Option<super::Fence<'a>>,
) -> Report {
    let digest = match digest(repo, id, observation, sources) {
        Ok(digest) => digest,
        Err(fault) => return reply(service, repo, id, Err(fault)),
    };
    let mut owner = match owner_request(service, repo, id, fence) {
        Ok(owner) => owner,
        Err(fault) => return reply(service, repo, id, Err(fault)),
    };
    let result = apply(&mut owner, request, observation, sources, digest);
    owned_reply(&owner, result)
}
fn owned_reply(owner: &Owner<'_>, result: Result<ResultCode, Fault>) -> Report {
    let mut report = reply(owner.service, owner.repo, owner.row.operation_id, result);
    if report.state.is_none() {
        report = run::report(
            report.code,
            report.reason,
            owner.known_execution.as_ref().map(|e| (&owner.row, e)),
        );
    }
    if let Some(request) = owner.request {
        let written=owner.service.read_session(RepositoryOperation::Read,|db|Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_resolutions a JOIN local_context_sync_steps p ON p.operation_ulid=a.operation_ulid WHERE a.request_ulid=?1 AND p.kind='path' AND p.phase='observed')",[request.id.to_string()],|r|r.get::<_,bool>(0))?)).unwrap_or(false);
        report.effects.write = if written || owner.performed.wrote {
            WriteEffect::Written
        } else {
            WriteEffect::Unchanged
        };
    }
    run::with_performed(report, owner)
}
pub(super) fn reply(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
    result: Result<ResultCode, Fault>,
) -> Report {
    let evidence = run::read_evidence(service, repo, id).ok();
    let (code, reason) = match result {
        Ok(code) => (code, Reason::None),
        Err(e) => (e.code, e.reason),
    };
    let mut reply = run::report(code, reason, evidence.as_ref().map(|(r, e)| (r, e)));
    if code == ResultCode::OriginalRequestRequired {
        reply.required_request = required_request(service, id);
    }
    reply
}
pub(super) fn required_request(
    service: &RepositoryService,
    id: OperationId,
) -> Option<OperationId> {
    service.read_session(RepositoryOperation::Read,|db| {
            let text:String=db.query_row("SELECT request_ulid FROM local_context_sync_resolutions WHERE attempt_ulid=(SELECT active_resolution FROM local_context_sync_records WHERE operation_ulid=?1)",[id.to_string()],|r|r.get(0))?;
            OperationId::parse(&text).map_err(|_|ReadError::new(ResultCode::RecoveryRequired))
        }).ok()
}
#[cfg(test)]
pub(super) fn resume(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
) -> Report {
    let mut owner = match owner(service, repo, id) {
        Ok(owner) => owner,
        Err(fault) => return reply(service, repo, id, Err(fault)),
    };
    let result = continue_without_source(&mut owner);
    owned_reply(&owner, result)
}
pub(super) fn continue_without_source(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
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
        return Ok(ResultCode::Ok);
    }
    if owner.row.active_resolution.is_some()
        && !owner.row.integration_proven
        && owner
            .row
            .candidate
            .is_none_or(|id| owner.git.find_commit(id).is_err())
    {
        return Err(Fault {
            code: ResultCode::OriginalRequestRequired,
            reason: Reason::OriginalResolutionRequired,
        });
    }
    if owner.request.is_some() {
        if owner.row.integration_proven && owner.row.artifacts_released {
            recover::verify_released_integration(owner)?;
        } else {
            verify_backend(owner)?;
        }
        owner.transaction(|db,row|{
        let epoch=row.owner_epoch.checked_add(1).ok_or_else(||ReadError::new(ResultCode::RecoveryRequired))?;
        if db.execute("UPDATE local_context_sync_records SET owner_epoch=?2 WHERE operation_ulid=?1 AND owner_epoch=?3",params![row.operation_id.to_string(),epoch,row.owner_epoch])?!=1{return Err(ReadError::new(ResultCode::RecoveryRequired));}
        if let Some(attempt)=row.active_resolution && db.execute("UPDATE local_context_sync_resolutions SET owner_epoch=?2 WHERE attempt_ulid=?1",params![attempt.to_string(),epoch])?!=1{return Err(ReadError::new(ResultCode::RecoveryRequired));}
        Ok(())
        })?;
    }
    super::control::safe_point(owner)?;
    if owner.row.integration_proven {
        return if owner.row.active_resolution.is_some() {
            finish(owner)
        } else {
            finish_external(owner)
        };
    }
    if owner.row.active_resolution.is_none() {
        if owner.git.refname_to_id(&owner.row.context_ref)? != owner.row.context_parent {
            return adopt_external(owner);
        }
        return conflict::install(owner);
    }
    if owner
        .row
        .candidate
        .is_some_and(|id| owner.git.find_commit(id).is_ok())
    {
        return complete_candidate(owner);
    }
    Err(Fault {
        code: ResultCode::OriginalRequestRequired,
        reason: Reason::OriginalResolutionRequired,
    })
}
fn apply(
    owner: &mut Owner<'_>,
    request: OperationId,
    observation: &str,
    sources: &[Source],
    digest: [u8; 32],
) -> Result<ResultCode, Fault> {
    let existing: Option<(String,Vec<u8>)> = owner.transaction(|db,row|Ok(db.query_row("SELECT attempt_ulid,input_digest FROM local_context_sync_resolutions WHERE operation_ulid=?1 AND request_ulid=?2",params![row.operation_id.to_string(),request.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?))?;
    if let Some((attempt, expected)) = existing {
        if expected != digest
            || owner
                .row
                .active_resolution
                .map(|id| id.to_string())
                .as_deref()
                != Some(attempt.as_str())
        {
            return Err(Fault {
                code: ResultCode::RequestMismatch,
                reason: Reason::ForeignChange,
            });
        }
        if owner.row.state == LocalSyncState::Complete {
            return Ok(ResultCode::Ok);
        }
        if matches!(
            owner.row.state,
            LocalSyncState::Cancelled | LocalSyncState::Abandoned
        ) {
            return Err(Fault {
                code: ResultCode::Cancelled,
                reason: Reason::Cancelled,
            });
        }
    } else {
        if owner.row.active_resolution.is_some()
            || owner.row.state != LocalSyncState::ConflictPending
        {
            return Err(Fault::foreign());
        }
        let inspection = conflict::inspect(owner.service, owner.repo, owner.row.operation_id)?;
        if !inspection.can_start_resolution || inspection.observation != observation {
            return Err(Fault::foreign());
        }
        verify_backend(owner)?;
        let ordered = validate(owner, sources)?;
        run::identity(&owner.git)?;
        let baseline = recover::index_digest(&owner.git)?;
        let mut intents = Vec::new();
        for (entry, source) in &ordered {
            let path = Path::new(std::str::from_utf8(&entry.path).map_err(|_| invalid())?);
            let before =
                integration::resolution::conflict_worktree_digest(&owner.row.worktree, path)?;
            intents.push((before, *blake3::hash(source.source.as_bytes()).as_bytes()));
        }
        let attempt = OperationId::new();
        owner.transaction(|db,row| {
            db.execute("INSERT INTO local_context_sync_resolutions(attempt_ulid,operation_ulid,request_ulid,generation,owner_epoch,observation,input_digest,state) VALUES(?1,?2,?3,?4,?5,?6,?7,'prepared')",params![attempt.to_string(),row.operation_id.to_string(),request.to_string(),row.generation,row.owner_epoch,observation,digest.as_slice()])?;
            db.execute("INSERT INTO local_context_sync_resolution_cancellation(operation_ulid,attempt_ulid) VALUES(?1,?2)",params![row.operation_id.to_string(),attempt.to_string()])?;
            for (i,(before,result)) in intents.iter().enumerate() {
                db.execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest,expected_metadata_digest) VALUES(?1,?2,?3,?4,'path','prepared',?5,?6,?7)",params![row.operation_id.to_string(),100+i as i64,row.generation,row.owner_epoch,baseline.as_slice(),before.as_slice(),result.as_slice()])?;
            }
            db.execute("UPDATE local_context_sync_records SET active_resolution=?2,state='resolution_prepared',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),attempt.to_string()])?;
            Ok(())
        })?;
        boundary(Point::Accepted)?;
    }
    let epoch = owner
        .row
        .owner_epoch
        .checked_add(1)
        .ok_or_else(Fault::authority)?;
    owner.transaction(|db,row| {
        if db.execute("UPDATE local_context_sync_records SET owner_epoch=?2 WHERE operation_ulid=?1 AND owner_epoch=?3 AND generation=?4",params![row.operation_id.to_string(),epoch,row.owner_epoch,row.generation])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
        db.execute("UPDATE local_context_sync_resolutions SET owner_epoch=?2,state=CASE WHEN state='prepared' THEN 'applying' ELSE state END WHERE attempt_ulid=?1",params![row.active_resolution.map(|id|id.to_string()),epoch])?;
        Ok(())
    })?;
    if owner.row.owner_epoch != epoch {
        return Err(Fault::authority());
    }
    super::control::safe_point(owner)?;
    if owner.row.integration_proven {
        return finish(owner);
    }
    if owner
        .row
        .candidate
        .is_none_or(|oid| owner.git.find_commit(oid).is_err())
    {
        let ordered = validate(owner, sources)?;
        verify_paths(owner, &ordered)?;
        for (i, (entry, source)) in ordered.iter().enumerate() {
            owner.authorize()?;
            let (before, result, applied) = path_intent(owner, i)?;
            let path = Path::new(std::str::from_utf8(&entry.path).map_err(|_| invalid())?);
            if integration::resolution::path_needs_write(
                &owner.row.worktree,
                path,
                source.source.as_bytes(),
                result,
                before,
                applied,
            )? {
                // Typed permission injection sits immediately at the owned write.
                boundary(Point::Path(i))?;
                owner.authorize()?;
                verify_backend(owner)?;
                write_path(owner, path, source.source.as_bytes(), before)?;
                owner.performed.wrote = true;
            }
            owner.transaction(|db,row| {db.execute("UPDATE local_context_sync_steps SET phase='observed',owner_epoch=?3 WHERE operation_ulid=?1 AND ordinal=?2",params![row.operation_id.to_string(),100+i as i64,row.owner_epoch])?;Ok(())})?;
            run::boundary(owner, FailurePoint::LocalSyncAfterResolutionPath)?;
            super::control::safe_point(owner)?;
        }
        make_candidate(owner, &ordered)?;
        boundary(Point::Candidate)?;
        run::boundary(owner, FailurePoint::LocalSyncAfterCandidate)?;
    }
    complete_candidate(owner)
}
fn validate<'a>(
    owner: &Owner<'_>,
    sources: &'a [Source],
) -> Result<Vec<(conflict::Entry, &'a Source)>, Fault> {
    if sources.iter().any(|s| !conflict::valid_token(&s.token)) {
        return Err(invalid());
    }
    let (entries, eligibility) = conflict::enumerate(owner.repo, &owner.git, &owner.row)?;
    if eligibility != integration::validate::ConflictEligibility::EligibleCanonical {
        return Err(Fault::foreign());
    }
    if entries.len() != sources.len() {
        return Err(invalid());
    }
    let mut ordered = Vec::new();
    for entry in entries {
        let source = sources
            .iter()
            .find(|s| s.token == entry.token)
            .ok_or_else(Fault::foreign)?;
        if !integration::validate::validates_resolution_sides(
            &owner.git,
            &entry.path,
            entry.sides,
            source.source.as_bytes(),
        )? {
            return Err(invalid());
        }
        ordered.push((entry, source));
    }
    let replacements = ordered
        .iter()
        .map(|(e, s)| integration::validate::ResolutionSource {
            path: &e.path,
            sides: e.sides,
            bytes: s.source.as_bytes(),
        })
        .collect::<Vec<_>>();
    if !integration::validate::validates_prospective_context(&owner.git, &replacements)? {
        return Err(invalid());
    }
    // Stage-zero canonical entries are part of the result too. Validate a full
    // prospective tree against all three frozen trees BEFORE binding R or writing
    // paths. All generated source blobs/tree/commit objects stay in this overlay.
    let worker = git2::Repository::open(owner.git.path())?;
    let odb = worker.odb()?;
    let _memory = odb.add_new_mempack_backend(1000)?;
    let preview = git2::Signature::new(
        "Manyhands",
        "preview@manyhands.invalid",
        &git2::Time::new(0, 0),
    )?;
    prepared_candidate(&worker, &owner.row, &ordered, &preview)?;
    Ok(ordered)
}
fn path_intent(owner: &mut Owner<'_>, i: usize) -> Result<([u8; 32], [u8; 32], bool), Fault> {
    owner.transaction(|db,row| {
        db.query_row("SELECT baseline_metadata_digest,expected_metadata_digest,phase='observed' FROM local_context_sync_steps WHERE operation_ulid=?1 AND ordinal=?2 AND kind='path'",params![row.operation_id.to_string(),100+i as i64],|r| {
            let hash=|i|r.get::<_,Vec<u8>>(i)?.try_into().map_err(|_|rusqlite::Error::InvalidQuery);
            Ok((hash(0)?,hash(1)?,r.get(2)?))
        }).map_err(ReadError::from)
    })
}
fn verify_paths(
    owner: &mut Owner<'_>,
    ordered: &[(conflict::Entry, &Source)],
) -> Result<(), Fault> {
    verify_backend(owner)?;
    let expected = conflict::stored(owner)?
        .flatten()
        .ok_or_else(Fault::authority)?;
    let actual = conflict::image(&owner.git)?;
    if actual.index != expected.index
        || actual.metadata != expected.metadata
        || owner.git.refname_to_id(&owner.row.context_ref)? != owner.row.context_parent
    {
        return Err(Fault::foreign());
    }
    for (i, (entry, source)) in ordered.iter().enumerate() {
        let (before, result, applied) = path_intent(owner, i)?;
        integration::resolution::path_needs_write(
            &owner.row.worktree,
            Path::new(std::str::from_utf8(&entry.path).map_err(|_| invalid())?),
            source.source.as_bytes(),
            result,
            before,
            applied,
        )?;
    }
    for path in &expected.paths {
        if !ordered.iter().any(|(e, _)| e.path == path.0) && !actual.paths.contains(path) {
            return Err(Fault::foreign());
        }
    }
    let allowed = ordered.iter().map(|(e, _)| e.path.clone()).collect();
    if !integration::resolution::only_paths_are_dirty(&owner.git, &allowed)? {
        return Err(Fault::foreign());
    }
    Ok(())
}
fn make_candidate(
    owner: &mut Owner<'_>,
    ordered: &[(conflict::Entry, &Source)],
) -> Result<(), Fault> {
    let signature = match owner.evidence()?.commit_identity {
        Some((name, email, seconds, offset)) => {
            git2::Signature::new(&name, &email, &git2::Time::new(seconds, offset))?
        }
        None => run::identity(&owner.git)?,
    };
    let worker = git2::Repository::open(owner.git.path())?;
    let odb = worker.odb()?;
    let _memory = odb.add_new_mempack_backend(1000)?;
    let candidate = prepared_candidate(&worker, &owner.row, ordered, &signature)?;
    let tree = worker.find_commit(candidate)?.tree_id();
    let baseline = recover::index_digest(&owner.git)?;
    if let Some(recorded) = owner.row.candidate {
        if recorded != candidate {
            return Err(Fault::authority());
        }
    } else {
        owner.transaction(|db,row| {
        db.execute("UPDATE local_context_sync_records SET candidate_oid=?2,state='commit_prepared',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),candidate.to_string()])?;
        db.execute("UPDATE local_context_sync_resolutions SET candidate_oid=?2,state='commit_prepared' WHERE attempt_ulid=?1",params![row.active_resolution.map(|id|id.to_string()),candidate.to_string()])?;
        db.execute("UPDATE local_context_sync_execution SET commit_name=?2,commit_email=?3,commit_time=?4,commit_offset=?5 WHERE operation_ulid=?1",params![row.operation_id.to_string(),signature.name(),signature.email(),signature.when().seconds(),signature.when().offset_minutes()])?;
        db.execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest,expected_old_oid,expected_new_oid) VALUES(?1,0,?2,?3,'objects','prepared',?4,?4,?5,?6)",params![row.operation_id.to_string(),row.generation,row.owner_epoch,baseline.as_slice(),row.context_parent.to_string(),candidate.to_string()])?;
        Ok(())
    })?;
    }
    boundary(Point::CandidateIntent)?;
    owner.authorize()?;
    verify_backend(owner)?;
    run::boundary(owner, FailurePoint::LocalSyncBeforeObjectImport)?;
    let target = owner.git.odb()?;
    let mut objects = vec![tree];
    while let Some(oid) = objects.pop() {
        let object = odb.read(oid)?;
        if object.kind() == git2::ObjectType::Tree {
            for e in worker.find_tree(oid)?.iter() {
                objects.push(e.id());
            }
        }
        if target.write(object.kind(), object.data())? != oid {
            return Err(Fault::authority());
        }
    }
    // Publish the commit object only after every referenced object exists. A
    // crash during import still needs exact R rather than a fabricated source.
    let commit = odb.read(candidate)?;
    if target.write(git2::ObjectType::Commit, commit.data())? != candidate {
        return Err(Fault::authority());
    }
    drop(target);
    owner.performed.committed = true;
    owner.performed.oid = Some(candidate);
    boundary(Point::CandidateImported)?;
    owner.step(0, "objects", true, Some(candidate))?;
    Ok(())
}

/// Caller owns a memory-backed ODB. This routine cannot authorize persistent
/// effects; both preacceptance validation and candidate preparation use it.
fn prepared_candidate(
    worker: &git2::Repository,
    row: &super::record::LocalContextSyncRecord,
    ordered: &[(conflict::Entry, &Source)],
    signature: &git2::Signature<'_>,
) -> Result<git2::Oid, Fault> {
    let mut index = conflict::prepared(worker, row)?;
    for (entry, source) in ordered {
        integration::resolution::stage_result(
            worker,
            &mut index,
            &entry.path,
            source.source.as_bytes(),
        )?;
    }
    let tree = index.write_tree_to(worker)?;
    let candidate = worker.commit(
        None,
        signature,
        signature,
        "Resolve local context conflicts",
        &worker.find_tree(tree)?,
        &[
            &worker.find_commit(row.context_parent)?,
            &worker.find_commit(row.primary_parent)?,
        ],
    )?;
    if !integration::validate::validates_external_integration(
        worker,
        row.context_parent,
        row.primary_parent,
        candidate,
    )? {
        return Err(invalid());
    }
    Ok(candidate)
}
pub(super) fn complete_candidate(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
    owner.authorize()?;
    verify_backend(owner)?;
    let evidence = owner.evidence()?;
    run::validate_candidate(owner, &evidence)?;
    conflict::backend_unlocked(owner)?;
    let oid = owner.row.candidate.ok_or_else(Fault::authority)?;
    let tree = owner.git.find_commit(oid)?.tree()?;
    let mut expected = git2::Index::new()?;
    expected.read_tree(&tree)?;
    drop(tree);
    if !integration::validate::validates_external_integration(
        &owner.git,
        owner.row.context_parent,
        owner.row.primary_parent,
        oid,
    )? {
        return Err(Fault::foreign());
    }
    verify_result_files(owner, &expected)?;
    let snapshot = conflict::stored(owner)?
        .flatten()
        .ok_or_else(Fault::authority)?;
    let current = conflict::image(&owner.git)?;
    if current.metadata != snapshot.metadata {
        return Err(Fault::foreign());
    }
    if owner.git.find_commit(oid)?.parent_ids().collect::<Vec<_>>()
        != [owner.row.context_parent, owner.row.primary_parent]
    {
        return Err(Fault::foreign());
    }
    let head = owner.git.refname_to_id(&owner.row.context_ref)?;
    // The process may have died after the commit object reached the ODB but
    // before observing its objects intent. Signature/parents/all-side invariants,
    // full result files and accepted path digests have now been checked above;
    // the shared authority audit requires the same active-R candidate binding
    // unconditionally, including when the objects step was already observed.
    if !evidence.commit_observed {
        owner.step(0, "objects", true, Some(oid))?;
    }
    if current.index != conflict::entries(&expected) {
        if current.index != snapshot.index || head != owner.row.context_parent {
            return Err(Fault::foreign());
        }
        owner.step(11, "index", false, Some(oid))?;
        owner.authorize()?;
        let mut index = owner.git.index()?;
        index.read_tree(&owner.git.find_commit(oid)?.tree()?)?;
        index.write()?;
    }
    owner.step(11, "index", true, Some(oid))?;
    boundary(Point::Index)?;
    verify_backend(owner)?;
    let evidence = owner.evidence()?;
    if head == owner.row.context_parent {
        if !recover::ref_proof(owner, &evidence, false)? {
            return Err(Fault::foreign());
        }
        owner.step(2, "reference", false, Some(oid))?;
        owner.flag("ref_started")?;
        let git = git2::Repository::open(owner.git.path())?;
        let branch = owner.row.context_ref.clone();
        let signature = git2::Signature::new(
            "Manyhands",
            "local-sync@manyhands.invalid",
            &git2::Time::new(evidence.ref_time, 0),
        )?;
        integration::artifacts::local_reference(
            &git,
            (&branch, owner.row.context_parent, oid),
            &signature,
            &format!("manyhands local sync {}", owner.row.operation_id),
            || owner.authorize(),
            || Ok::<_, Fault>(()),
        )?;
        owner.performed.integrated = true;
        owner.performed.oid = Some(oid);
    } else if head != oid || !evidence.ref_started || !recover::ref_proof(owner, &evidence, true)? {
        return Err(Fault::foreign());
    }
    owner.flag("ref_observed")?;
    owner.step(2, "reference", true, Some(oid))?;
    owner.transaction(|db,row| {
        db.execute("UPDATE local_context_sync_records SET integration_proven=1,result_oid=candidate_oid,state='integrated',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",[row.operation_id.to_string()])?;
        db.execute("UPDATE local_context_sync_resolutions SET state='integrated' WHERE attempt_ulid=?1",[row.active_resolution.map(|id|id.to_string())])?;
        Ok(())
    })?;
    boundary(Point::Ref)?;
    super::control::safe_point(owner)?;
    finish(owner)
}
/// A same-OID ref is not proof that its logs or backend namespace are unchanged.
/// Called outside owned backend primitives (which hold their own transient locks)
/// before acceptance and before each next persistent Git/path effect.
fn verify_backend(owner: &mut Owner<'_>) -> Result<(), Fault> {
    conflict::backend_unlocked(owner)?;
    let evidence = owner.evidence()?;
    let head = owner.git.refname_to_id(&owner.row.context_ref)?;
    let after = head != owner.row.context_parent;
    if after && (Some(head) != owner.row.candidate || !evidence.ref_started) {
        return Err(Fault::foreign());
    }
    if !recover::ref_proof(owner, &evidence, after)? {
        return Err(Fault::foreign());
    }
    Ok(())
}
fn verify_result_files(owner: &mut Owner<'_>, index: &git2::Index) -> Result<(), Fault> {
    let original = conflict::stored(owner)?
        .flatten()
        .ok_or_else(Fault::authority)?;
    let (paths, _) = conflict::enumerate(owner.repo, &owner.git, &owner.row)?;
    let prepared = conflict::prepared(&owner.git, &owner.row)?;
    if !integration::validate::prepared_merge_index_matches_index(&prepared, index)? {
        return Err(Fault::foreign());
    }
    let allowed = paths.iter().map(|e| e.path.clone()).collect();
    if !integration::resolution::only_paths_are_dirty(&owner.git, &allowed)? {
        return Err(Fault::foreign());
    }
    for entry in index.iter() {
        let path = Path::new(std::str::from_utf8(&entry.path).map_err(|_| Fault::foreign())?);
        let actual = super::file_image::read(&owner.row.worktree, path)
            .map_err(|_| Fault::foreign())?
            .ok_or_else(Fault::foreign)?;
        if actual.mode.is_some_and(|mode| mode != entry.mode) {
            return Err(Fault::foreign());
        }
        let digest = *blake3::hash(&actual.bytes).as_bytes();
        if let Some(i) = paths.iter().position(|p| p.path == entry.path) {
            let (_, expected, _) = path_intent(owner, i)?;
            if digest != expected
                || git2::Oid::hash_object(git2::ObjectType::Blob, &actual.bytes)? != entry.id
            {
                return Err(Fault::foreign());
            }
        } else if !original
            .paths
            .iter()
            .any(|p| p.0 == entry.path && p.1 == Some(digest) && p.2 == actual.mode)
        {
            return Err(Fault::foreign());
        }
    }
    Ok(())
}
pub(super) fn finish(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
    owner.authorize()?;
    let snapshot = conflict::stored(owner)?
        .flatten()
        .ok_or_else(Fault::authority)?;
    let metadata = integration::artifacts::MergeMetadataDirectory::open(&owner.git)?;
    release_preflight(owner, &metadata, snapshot.metadata)?;
    owner.step(12, "metadata_release", false, owner.row.candidate)?;
    for (i, (member, expected)) in conflict::MEMBERS.iter().zip(snapshot.metadata).enumerate() {
        // A previous member's retirement is a safe boundary, not permission to
        // consume the rest after foreign ref/log/lock/member changes arrive.
        release_preflight(owner, &metadata, snapshot.metadata)?;
        if let (Some(actual), Some(expected)) = (metadata.digest(member)?, expected) {
            if actual != expected {
                return Err(Fault::foreign());
            }
            metadata.retire(member, expected)?;
            boundary(Point::Retired(i))?;
        }
    }
    owner.step(12, "metadata_release", true, owner.row.candidate)?;
    recover::clean_metadata(&owner.git, &owner.row.context_ref)?;
    let mut index = git2::Index::new()?;
    let oid = owner.row.result.ok_or_else(Fault::authority)?;
    index.read_tree(&owner.git.find_commit(oid)?.tree()?)?;
    verify_result_files(owner, &index)?;
    let evidence = owner.evidence()?;
    if conflict::entries(&owner.git.index()?) != conflict::entries(&index)
        || owner.git.refname_to_id(&owner.row.context_ref)? != oid
        || !recover::ref_proof(owner, &evidence, true)?
    {
        return Err(Fault::foreign());
    }
    owner.transaction(|db,row| {db.execute("UPDATE local_context_sync_records SET artifacts_released=1,state='index_pending',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",[row.operation_id.to_string()])?;Ok(())})?;
    super::control::safe_point(owner)?;
    run::boundary(owner, FailurePoint::LocalSyncBeforeIndex).map_err(|_| Fault {
        code: ResultCode::DiscoveryPending,
        reason: Reason::IndexPending,
    })?;
    super::control::safe_point(owner)?;
    finish_index(owner)?;
    Ok(ResultCode::Ok)
}
fn release_preflight(
    owner: &mut Owner<'_>,
    metadata: &integration::artifacts::MergeMetadataDirectory,
    expected: [Option<[u8; 32]>; 3],
) -> Result<(), Fault> {
    owner.authorize()?;
    conflict::backend_unlocked(owner)?;
    verify_integrated(owner)?;
    // Inspect the whole remaining set before any next unlink. Also reject a
    // foreign reappearance at an already-retired name rather than ignoring it.
    for (member, expected) in conflict::MEMBERS.iter().zip(expected) {
        let actual = metadata.digest(member)?;
        if actual.is_some() && actual != expected {
            return Err(Fault::foreign());
        }
    }
    Ok(())
}
pub(super) fn verify_integrated(owner: &mut Owner<'_>) -> Result<(), Fault> {
    let oid = owner.row.result.ok_or_else(Fault::authority)?;
    let mut index = git2::Index::new()?;
    index.read_tree(&owner.git.find_commit(oid)?.tree()?)?;
    verify_result_files(owner, &index)?;
    let evidence = owner.evidence()?;
    if conflict::entries(&owner.git.index()?) != conflict::entries(&index)
        || owner.git.refname_to_id(&owner.row.context_ref)? != oid
        || !recover::ref_proof(owner, &evidence, true)?
    {
        return Err(Fault::foreign());
    }
    Ok(())
}
fn external_proof(owner: &mut Owner<'_>, oid: git2::Oid) -> Result<(), Fault> {
    owner.authorize()?;
    if owner.git.find_commit(oid)?.parent_ids().collect::<Vec<_>>()
        != [owner.row.context_parent, owner.row.primary_parent]
    {
        return Err(Fault::foreign());
    }
    if owner.git.refname_to_id(&owner.row.context_ref)? != oid
        || !recover::exact_image(&owner.git, oid)?
        || !integration::validate::validates_external_integration(
            &owner.git,
            owner.row.context_parent,
            owner.row.primary_parent,
            oid,
        )?
    {
        return Err(Fault::foreign());
    }
    // External Git resolves its own backend artifacts. Do not reinterpret an
    // index lock or incomplete metadata cleanup as successful external repair.
    recover::clean_metadata(&owner.git, &owner.row.context_ref)
}
pub(super) fn adopt_external(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
    let oid = owner.git.refname_to_id(&owner.row.context_ref)?;
    external_proof(owner, oid)?;
    owner.step(13, "reference", false, Some(oid))?;
    bind_external_logs(owner)?;
    owner.step(13, "reference", true, Some(oid))?;
    owner.transaction(|db,row| {
        db.execute("UPDATE local_context_sync_records SET state='index_pending',integration_proven=1,artifacts_released=1,result_oid=?2,semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),oid.to_string()])?;
        Ok(())
    })?;
    finish_external(owner)
}
fn finish_external(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
    verify_external_integrated(owner)?;
    super::control::safe_point(owner)?;
    run::boundary(owner, FailurePoint::LocalSyncBeforeIndex).map_err(|_| Fault {
        code: ResultCode::DiscoveryPending,
        reason: Reason::IndexPending,
    })?;
    super::control::safe_point(owner)?;
    finish_index(owner)?;
    Ok(ResultCode::Ok)
}
pub(super) fn verify_external_integrated(owner: &mut Owner<'_>) -> Result<(), Fault> {
    let result = owner.row.result.ok_or_else(Fault::authority)?;
    if !external_integration_recorded(owner)? {
        return Err(Fault::authority());
    }
    external_proof(owner, result)?;
    let logs = recover::current_log_digests(&owner.git, &owner.row.context_ref)?;
    owner.transaction(|db,row| {
        let (branch,head):(Vec<u8>,Option<Vec<u8>>)=db.query_row("SELECT baseline_metadata_digest,expected_metadata_digest FROM local_context_sync_steps WHERE operation_ulid=?1 AND ordinal=13",[row.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let head=head.ok_or_else(||ReadError::new(ResultCode::RecoveryRequired))?;
        if branch!=logs[0] || head!=logs[1] {return Err(ReadError::new(ResultCode::ExternalChange));}
        Ok(())
    })
}
/// External adoption has no library-made reflog append. Its reference step stores
/// the validated post-adoption branch/HEAD log digests in the two metadata digest
/// fields. Once bound they cannot be refreshed from a later foreign observation.
fn bind_external_logs(owner: &mut Owner<'_>) -> Result<(), Fault> {
    let logs = recover::current_log_digests(&owner.git, &owner.row.context_ref)?;
    owner.transaction(|db,row| {
        let (branch,head,phase):(Vec<u8>,Option<Vec<u8>>,String)=db.query_row("SELECT baseline_metadata_digest,expected_metadata_digest,phase FROM local_context_sync_steps WHERE operation_ulid=?1 AND ordinal=13",[row.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if let Some(head)=head {
            if branch!=logs[0] || head!=logs[1] {return Err(ReadError::new(ResultCode::ExternalChange));}
        } else {
            let unbound=blake3::hash(b"local-sync-no-merge-metadata-v1");
            if phase!="prepared" || branch.as_slice()!=unbound.as_bytes() {return Err(ReadError::new(ResultCode::RecoveryRequired));}
            if db.execute("UPDATE local_context_sync_steps SET baseline_metadata_digest=?2,expected_metadata_digest=?3 WHERE operation_ulid=?1 AND ordinal=13 AND phase='prepared' AND expected_metadata_digest IS NULL",params![row.operation_id.to_string(),logs[0].as_slice(),logs[1].as_slice()])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
        }
        Ok(())
    })
}
pub(super) fn external_integration_recorded(owner: &mut Owner<'_>) -> Result<bool, Fault> {
    owner.transaction(|db,row| {
        Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_steps WHERE operation_ulid=?1 AND ordinal=13 AND kind='reference' AND phase='observed' AND generation=?2 AND expected_old_oid=?3 AND expected_new_oid=?4)",params![row.operation_id.to_string(),row.generation,row.context_parent.to_string(),row.result.map(|id|id.to_string())],|r|r.get(0))?)
    })
}
fn finish_index(owner: &mut Owner<'_>) -> Result<(), Fault> {
    run::index(owner).map_err(|fault| {
        if fault.code == ResultCode::Cancelled {
            fault
        } else {
            Fault {
                code: ResultCode::DiscoveryPending,
                reason: Reason::IndexPending,
            }
        }
    })
}
