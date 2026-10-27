//! The internal bridge between F2 authority and the leased domain executor.
use super::{conflict, record, recover, resolve, run};
use crate::{canonical::ItemId, repository::*, results::*};
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Clone, Copy)]
pub(in super::super) struct Fence<'a> {
    pub id: RequestId,
    pub attempt: u64,
    pub cancelled: &'a dyn Fn() -> bool,
    pub progress: &'a dyn Fn(ProgressStage, Effects),
}
impl Fence<'_> {
    pub(super) fn verify(
        &self,
        db: &Connection,
        root: &std::path::Path,
        parent: OperationId,
    ) -> Result<(), ReadError> {
        mutation::local_control::audit(db)?;
        let valid: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_request_control c JOIN request_records r USING(request_ulid) WHERE c.request_ulid=?1 AND c.scope_key=?2 AND c.intended_parent=?3 AND c.execution_attempt=?4 AND r.attempt=c.execution_attempt AND r.state='accepted')",params![self.id.to_string(),root.to_str(),parent.to_string(),self.attempt],|r|r.get(0))?;
        if valid {
            authority_controls(db, root, parent)?;
            Ok(())
        } else {
            Err(ReadError::new(ResultCode::ExternalChange))
        }
    }
    pub(super) fn attach(
        &self,
        db: &Connection,
        row: &record::LocalContextSyncRecord,
    ) -> Result<(), ReadError> {
        let (role,linked):(String,bool)=db.query_row("SELECT role,parent_operation IS NOT NULL FROM local_context_sync_request_control WHERE request_ulid=?1",[self.id.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if role != "start" && !linked {
            match row.state {
                record::LocalSyncState::Complete => {
                    return Err(ReadError::new(ResultCode::ExternalChange));
                }
                record::LocalSyncState::Cancelled | record::LocalSyncState::Abandoned => {
                    return Err(ReadError::new(ResultCode::Cancelled));
                }
                _ => {}
            }
        }
        if role == "resume" && !linked && row.active_resolution.is_some() && !row.integration_proven
        {
            let git = git2::Repository::open(&row.worktree)
                .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
            let evidence = recover::Execution::read(db, row.operation_id)?;
            if !resolve::candidate_ready(&git, row, &evidence)
                .map_err(|f| ReadError::new(f.code))?
            {
                return Err(ReadError::new(ResultCode::OriginalRequestRequired));
            }
        }
        let pending_resolution:bool=db.query_row("SELECT role='resolve' AND NOT EXISTS(SELECT 1 FROM local_context_sync_resolutions a WHERE a.request_ulid=c.request_ulid AND a.operation_ulid=?2) FROM local_context_sync_request_control c WHERE c.request_ulid=?1",params![self.id.to_string(),row.operation_id.to_string()],|r|r.get(0))?;
        if pending_resolution {
            return Ok(());
        }
        let changed=db.execute("UPDATE local_context_sync_request_control SET parent_operation=?2 WHERE request_ulid=?1 AND intended_parent=?2 AND execution_attempt=?3 AND (parent_operation IS NULL OR parent_operation=?2)",params![self.id.to_string(),row.operation_id.to_string(),self.attempt])?;
        if changed != 1 {
            return Err(ReadError::new(ResultCode::RecoveryRequired));
        }
        Ok(())
    }
    pub(super) fn record(
        &self,
        db: &Connection,
        row: &record::LocalContextSyncRecord,
        e: &recover::Execution,
    ) -> Result<(), ReadError> {
        let role: String = db.query_row(
            "SELECT role FROM local_context_sync_request_control WHERE request_ulid=?1",
            [self.id.to_string()],
            |r| r.get(0),
        )?;
        let attempt:Option<String>=db.query_row("SELECT attempt_ulid FROM local_context_sync_resolutions WHERE request_ulid=?1 AND operation_ulid=?2",params![self.id.to_string(),row.operation_id.to_string()],|r|r.get(0)).optional()?;
        let mut effects = run::report(ResultCode::Ok, run::Reason::None, Some((row, e))).effects;
        if role == "resolve" {
            if attempt.is_none() {
                return Ok(());
            } else {
                let written:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_steps WHERE operation_ulid=?1 AND kind='path' AND phase='observed')",[row.operation_id.to_string()],|r|r.get(0))?;
                effects.write = if written {
                    WriteEffect::Written
                } else {
                    WriteEffect::Unchanged
                };
            }
        }
        if db.execute("UPDATE local_context_sync_request_control SET resolution_attempt=?2,owner_epoch=?3,effect_write=?4,effect_checkpoint=?5,effect_discovery=?6,effect_integration=?7,commit_oid=?8 WHERE request_ulid=?1 AND execution_attempt=?9",params![self.id.to_string(),attempt,row.owner_epoch,effects.write.as_str(),effects.checkpoint.as_str(),effects.discovery.as_str(),effects.integration.as_str(),effects.commit_oid,self.attempt])?!=1 {return Err(ReadError::new(ResultCode::ExternalChange));}
        Ok(())
    }
}

/// Complete S/active-R authority, including both F2 associations and counters.
/// It never substitutes the surviving subset when any required owner is lost.
pub(in super::super) fn authority_controls(
    db: &Connection,
    root: &std::path::Path,
    parent: OperationId,
) -> Result<Vec<(String, i64, i64)>, ReadError> {
    let missing = || ReadError::new(ResultCode::RecoveryRequired);
    let state:Option<(Option<String>,String)>=db.query_row("SELECT s.active_resolution,s.state FROM local_context_sync_records s JOIN repositories p ON p.id=s.repository_id WHERE s.operation_ulid=?1 AND p.root_path=?2",params![parent.to_string(),root.to_str()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let terminal = state
        .as_ref()
        .is_some_and(|(_, s)| matches!(s.as_str(), "complete" | "cancelled" | "abandoned"));
    let mut start=db.prepare("SELECT c.request_ulid,c.cancel_issued,c.cancel_acknowledged FROM local_context_sync_request_control c JOIN request_records r USING(request_ulid) JOIN request_operations o ON o.request_ulid=c.request_ulid AND o.family='local' AND o.operation_ulid=c.intended_parent WHERE c.intended_parent=?1 AND c.scope_key=?2 AND r.scope_key=c.scope_key AND c.role='start' AND r.command='context sync-local' AND c.execution_attempt=r.attempt AND (r.state='accepted' OR ?3) AND ((?4 AND c.parent_operation=?1) OR (NOT ?4 AND c.parent_operation IS NULL))")?;
    let mut vector: Vec<(String, i64, i64)> = start
        .query_map(
            params![parent.to_string(), root.to_str(), terminal, state.is_some()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?
        .collect::<Result<_, _>>()?;
    if vector.len() != 1 {
        return Err(missing());
    }
    if let Some((Some(active), _)) = state {
        let mut resolution=db.prepare("SELECT c.request_ulid,c.cancel_issued,c.cancel_acknowledged FROM local_context_sync_resolutions a JOIN local_context_sync_request_control c ON c.request_ulid=a.request_ulid JOIN request_records r ON r.request_ulid=c.request_ulid WHERE a.attempt_ulid=?1 AND a.operation_ulid=?2 AND c.resolution_attempt=a.attempt_ulid AND c.parent_operation=a.operation_ulid AND c.intended_parent=a.operation_ulid AND c.scope_key=?3 AND r.scope_key=c.scope_key AND c.role='resolve' AND r.command='context resolve-local' AND r.target=c.intended_parent AND c.execution_attempt=r.attempt AND (r.state='accepted' OR ?4)")?;
        let controls: Vec<(String, i64, i64)> = resolution
            .query_map(
                params![active, parent.to_string(), root.to_str(), terminal],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?
            .collect::<Result<_, _>>()?;
        if controls.len() != 1 {
            return Err(missing());
        }
        vector.extend(controls);
    }
    if vector
        .iter()
        .any(|(_, issued, ack)| *ack < 0 || issued < ack)
    {
        return Err(missing());
    }
    Ok(vector)
}

pub(in super::super) fn execute_start<'a>(
    service: &'a RepositoryService,
    repo: &'a ResolvedRepository,
    kind: AuthoringKind,
    item: &ItemId,
    id: OperationId,
    observation: &str,
    fence: Fence<'a>,
) -> run::Report {
    run::execute_request(service, repo, kind, item, id, observation, Some(fence))
}
pub(in super::super) fn execute_resolution<'a>(
    service: &'a RepositoryService,
    repo: &'a ResolvedRepository,
    id: OperationId,
    observation: &str,
    sources: &[LocalConflictResolutionInput],
    fence: Fence<'a>,
) -> run::Report {
    let sources: Vec<_> = sources
        .iter()
        .map(|s| resolve::Source {
            token: s.token.clone(),
            source: s.source.clone(),
        })
        .collect();
    resolve::execute_request(
        service,
        repo,
        id,
        OperationId::parse(&fence.id.to_string()).expect("canonical request ID"),
        observation,
        &sources,
        Some(fence),
    )
}

fn state(state: Option<record::LocalSyncState>) -> LocalContextSyncState {
    use LocalContextSyncState as D;
    use record::LocalSyncState as S;
    match state {
        None => D::Refused,
        Some(s) => match s {
            S::Prepared => D::Prepared,
            S::Applying => D::Applying,
            S::ConflictPending => D::ConflictPending,
            S::ResolutionPrepared => D::ResolutionPrepared,
            S::CommitPrepared => D::CommitPrepared,
            S::Integrated => D::Integrated,
            S::IndexPending => D::IndexPending,
            S::Complete => D::Complete,
            S::RecoveryRequired => D::RecoveryRequired,
            S::Cancelled => D::Cancelled,
            S::Abandoned => D::Abandoned,
        },
    }
}
fn reason(reason: run::Reason) -> LocalContextSyncReason {
    use LocalContextSyncReason as D;
    use run::Reason as R;
    match reason {
        R::None => D::None,
        R::UnrelatedHistories => D::UnrelatedHistories,
        R::ImmutableClosureDisagreement => D::ImmutableClosureDisagreement,
        R::IdentityRequired => D::IdentityRequired,
        R::IndexPending => D::IndexPending,
        R::ForeignChange => D::ForeignChange,
        R::AmbiguousBackend => D::AmbiguousBackend,
        R::AuthorityMissing => D::AuthorityMissing,
        R::UnsupportedConflict => D::UnsupportedConflict,
        R::CanonicalConflict => D::CanonicalConflict,
        R::OriginalResolutionRequired => D::OriginalResolutionRequired,
        R::Cancelled => D::Cancelled,
    }
}
pub(in super::super) fn data(
    report: &run::Report,
    kind: AuthoringKind,
    item: &ItemId,
    id: Option<OperationId>,
    resolution: Option<RequestId>,
    start: Option<RequestId>,
) -> LocalContextSyncMutationDto {
    use LocalContextSyncNextAction as A;
    use LocalContextSyncReason as R;
    use LocalContextSyncState as S;
    let state = state(report.state);
    let mut reason = reason(report.reason);
    if state == S::Abandoned {
        reason = R::Abandoned;
    }
    let terminal = matches!(state, S::Complete | S::Abandoned | S::Cancelled);
    let next_action = if terminal {
        A::None
    } else {
        match reason {
            R::ImmutableClosureDisagreement | R::UnrelatedHistories => A::None,
            R::IdentityRequired => A::SetIdentity,
            R::OriginalResolutionRequired => A::RetryResolution,
            R::ForeignChange
            | R::AmbiguousBackend
            | R::UnsupportedConflict
            | R::AuthorityMissing => A::ExternalRepair,
            _ => match state {
                S::ConflictPending => A::Resolve,
                S::ResolutionPrepared => A::RetryResolution,
                S::Refused => A::None,
                _ => A::Resume,
            },
        }
    };
    let required_request_id =
        report
            .required_request
            .map(|r| r.to_string())
            .or_else(|| match next_action {
                A::RetryResolution => resolution.map(|r| r.to_string()),
                A::SetIdentity => resolution.or(start).map(|r| r.to_string()),
                A::Resume => start.map(|r| r.to_string()),
                _ => None,
            });
    LocalContextSyncMutationDto {
        data_type: LocalContextSyncDataType::LocalContextSync,
        item_id: item.to_string(),
        kind: authoring_kind_segment(&kind).into(),
        operation_id: id.map(|id| id.to_string()),
        resolution_request_id: resolution.map(|r| r.to_string()),
        required_request_id,
        result_oid: report.effects.commit_oid.clone(),
        state,
        reason,
        next_action,
        eligibility: match reason {
            R::CanonicalConflict => Some(LocalContextSyncEligibility::EligibleCanonical),
            R::UnsupportedConflict | R::ImmutableClosureDisagreement => {
                Some(LocalContextSyncEligibility::ExternalResolutionRequired)
            }
            _ => None,
        },
        cancel_acknowledged: report.cancel_acknowledged,
    }
}
#[derive(PartialEq, Eq)]
struct Controls {
    start: String,
    active: Option<String>,
    issued: i64,
    ack: i64,
    r_issued: i64,
    r_ack: i64,
}
fn controls(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    row: &record::LocalContextSyncRecord,
) -> Result<Controls, ReadError> {
    service.read_session(RepositoryOperation::Read, |db| {
        mutation::local_control::audit(db)?;
        let vector = authority_controls(db, repo.root(), row.operation_id)?;
        if vector.len() != 1 + usize::from(row.active_resolution.is_some()) {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        let (start, issued, ack) = vector[0].clone();
        let (active, r_issued, r_ack) = vector
            .get(1)
            .map_or((None, 0, 0), |(id, i, a)| (Some(id.clone()), *i, *a));
        Ok(Controls {
            start,
            active,
            issued,
            ack,
            r_issued,
            r_ack,
        })
    })
}
fn display_path(path: &[u8]) -> String {
    let mut display = String::new();
    for &b in path {
        match b {
            b' '..=b'~' if b != b'\\' => display.push(char::from(b)),
            b'\\' => display.push_str("\\\\"),
            _ => {
                use std::fmt::Write;
                let _ = write!(display, "\\x{b:02x}");
            }
        }
    }
    display
}
pub(in super::super) fn inspection(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
) -> Result<LocalContextSyncInspectionDto, ReadError> {
    let (row, e) = run::read_evidence(service, repo, id)?;
    let before = controls(service, repo, &row)?;
    let facts = service.read_session(RepositoryOperation::Read, |db| {
        super::observe::facts(db, repo, row.kind, &row.item_id)
    })?;
    let git = git2::Repository::open(&row.worktree)
        .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
    let head = git.refname_to_id(&row.context_ref).ok();
    let image = conflict::image(&git).map_err(|e| ReadError::new(e.code))?;
    let candidate = if row.active_resolution.is_some() {
        resolve::candidate_ready(&git, &row, &e).map_err(|f| (f.code, f.reason))
    } else {
        Ok(false)
    };
    let backend =
        resolve::read_backend_proof(service, &git, &row, &e).map_err(|f| (f.code, f.reason));
    let mut report = run::report(ResultCode::Ok, run::Reason::None, Some((&row, &e)));
    let mut paths = Vec::new();
    let mut observation = None;
    let mut eligibility = None;
    let mut can_start_resolution = false;
    if matches!(
        row.state,
        record::LocalSyncState::ConflictPending
            | record::LocalSyncState::ResolutionPrepared
            | record::LocalSyncState::CommitPrepared
    ) && e.conflict_observed
    {
        let inspected = conflict::inspect(service, repo, id).map_err(|e| ReadError::new(e.code))?;
        can_start_resolution = inspected.can_start_resolution;
        observation = Some(inspected.observation);
        let eligible =
            inspected.eligibility == integration::validate::ConflictEligibility::EligibleCanonical;
        eligibility = Some(if eligible {
            LocalContextSyncEligibility::EligibleCanonical
        } else {
            LocalContextSyncEligibility::ExternalResolutionRequired
        });
        report.reason = if row.active_resolution.is_some() && candidate == Ok(false) {
            report.required_request = before
                .active
                .as_deref()
                .and_then(|r| OperationId::parse(r).ok());
            run::Reason::OriginalResolutionRequired
        } else if eligible && row.active_resolution.is_none() && !can_start_resolution {
            run::Reason::ForeignChange
        } else if eligible {
            run::Reason::CanonicalConflict
        } else {
            run::Reason::UnsupportedConflict
        };
        paths = inspected
            .paths
            .into_iter()
            .map(|p| LocalContextConflictEntryDto {
                display_path: display_path(&p.path),
                token: p.token,
                content_available: p.readable,
            })
            .collect();
    }
    if row.state == record::LocalSyncState::Cancelled {
        report.reason = run::Reason::Cancelled;
    }
    if row.state == record::LocalSyncState::IndexPending {
        report.reason = run::Reason::IndexPending;
    }
    if row.state == record::LocalSyncState::RecoveryRequired {
        report.reason = run::Reason::AuthorityMissing;
    }
    if row.candidate.is_none()
        && !row.integration_proven
        && backend != Ok(true)
        && e.disposition == recover::Disposition::Merge
        && !matches!(
            row.state,
            record::LocalSyncState::Complete
                | record::LocalSyncState::Cancelled
                | record::LocalSyncState::Abandoned
        )
        && run::identity(&git).is_err_and(|e| e.code == ResultCode::IdentityRequired)
    {
        report.reason = run::Reason::IdentityRequired;
        can_start_resolution = false;
    }
    if !matches!(
        row.state,
        record::LocalSyncState::Complete
            | record::LocalSyncState::Cancelled
            | record::LocalSyncState::Abandoned
    ) {
        let proof = candidate.and_then(|ready| {
            backend.and_then(|_| {
                if row.active_resolution.is_some() {
                    resolve::read_resolution_image(service, repo, &git, &row, &image, ready)
                        .map_err(|f| (f.code, f.reason))
                } else {
                    Ok(())
                }
            })
        });
        if let Err((code, reason)) = proof {
            if code == ResultCode::RepositoryInaccessible {
                return Err(ReadError::new(code));
            }
            report.reason = reason;
            report.required_request = None;
            can_start_resolution = false;
        }
    }
    super::observation_checkpoint(service);
    let (mut again, _) = run::read_evidence(service, repo, id)?;
    again.owner_epoch = row.owner_epoch;
    if row != again
        || before != controls(service, repo, &again)?
        || facts
            != service.read_session(RepositoryOperation::Read, |db| {
                super::observe::facts(db, repo, row.kind, &row.item_id)
            })?
        || head != git.refname_to_id(&row.context_ref).ok()
        || image != conflict::image(&git).map_err(|e| ReadError::new(e.code))?
        || (if row.active_resolution.is_some() {
            resolve::candidate_ready(&git, &row, &e).map_err(|f| (f.code, f.reason))
        } else {
            Ok(false)
        }) != candidate
        || resolve::read_backend_proof(service, &git, &row, &e).map_err(|f| (f.code, f.reason))
            != backend
    {
        return Err(ReadError::new(ResultCode::ExternalChange));
    }
    report.cancel_acknowledged = (before.issued > 0 || before.r_issued > 0)
        && before.issued == before.ack
        && before.r_issued == before.r_ack;
    let mut view = data(
        &report,
        row.kind,
        &row.item_id,
        Some(id),
        before
            .active
            .as_deref()
            .and_then(|r| RequestId::parse(r).ok()),
        RequestId::parse(&before.start).ok(),
    );
    if backend == Ok(true)
        && !matches!(
            row.state,
            record::LocalSyncState::Complete
                | record::LocalSyncState::Cancelled
                | record::LocalSyncState::Abandoned
        )
    {
        view.next_action = LocalContextSyncNextAction::Resume;
        view.required_request_id = Some(before.start.clone());
        if !row.integration_proven {
            view.reason = LocalContextSyncReason::None;
        }
        can_start_resolution = false;
    }
    Ok(LocalContextSyncInspectionDto {
        operation_id: id.to_string(),
        kind: view.kind,
        item_id: view.item_id,
        context_ref: row.context_ref,
        primary_ref: row.primary_ref,
        context_parent: row.context_parent.to_string(),
        primary_parent: row.primary_parent.to_string(),
        context_head: head.map(|o| o.to_string()),
        state: view.state,
        reason: view.reason,
        next_action: view.next_action,
        observation,
        eligibility,
        paths,
        can_start_resolution,
        starting_request_id: before.start.clone(),
        cancellation_request_id: before.start,
        active_resolution_request_id: before.active,
        required_request_id: view.required_request_id,
        result_oid: view.result_oid,
        cancel_requested: before.issued > before.ack || before.r_issued > before.r_ack,
        cancel_acknowledged: report.cancel_acknowledged,
    })
}
pub(in super::super) fn content(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
    token: &str,
) -> Result<LocalContextConflictContentDto, ReadError> {
    let before = inspection(service, repo, id)?;
    let sides = conflict::read(service, repo, id, token).map_err(|e| ReadError::new(e.code))?;
    if before != inspection(service, repo, id)? {
        return Err(ReadError::new(ResultCode::ExternalChange));
    }
    Ok(LocalContextConflictContentDto {
        operation_id: id.to_string(),
        token: token.into(),
        base: sides.base,
        local: sides.local,
        incoming: sides.incoming,
        historical: sides.historical,
    })
}

#[derive(PartialEq, Eq)]
struct ResolutionHistory {
    row: record::LocalContextSyncRecord,
    scope: String,
    attempt: String,
    execution_attempt: i64,
    paths: Vec<(i64, Vec<u8>, Vec<u8>, String)>,
}
fn resolution_history(
    service: &RepositoryService,
    id: RequestId,
) -> Result<Option<ResolutionHistory>, ReadError> {
    service.read_session(RepositoryOperation::Read,|db|{
        record::audit(db)?;
        mutation::local_control::audit(db)?;
        let control:Option<(String,String,String,i64)>=db.query_row("SELECT c.scope_key,c.parent_operation,c.resolution_attempt,c.execution_attempt FROM local_context_sync_request_control c JOIN request_records r USING(request_ulid) WHERE c.request_ulid=?1 AND c.role='resolve' AND c.parent_operation IS NOT NULL AND c.resolution_attempt IS NOT NULL AND r.scope_key=c.scope_key AND r.attempt=c.execution_attempt",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        let Some((scope,parent,attempt,execution_attempt))=control else{return Ok(None)};
        let row=db.query_row("SELECT p.* FROM local_context_sync_records p JOIN repositories repo ON repo.id=p.repository_id JOIN local_context_sync_resolutions a ON a.operation_ulid=p.operation_ulid AND a.attempt_ulid=p.active_resolution WHERE p.operation_ulid=?1 AND repo.root_path=?2 AND a.attempt_ulid=?3 AND a.request_ulid=?4",params![parent,scope,attempt,id.to_string()],record::decode)?;
        let paths=db.prepare("SELECT ordinal,baseline_metadata_digest,expected_metadata_digest,phase FROM local_context_sync_steps WHERE operation_ulid=?1 AND kind='path' ORDER BY ordinal")?.query_map([parent],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?.collect::<Result<_,_>>()?;
        Ok(Some(ResolutionHistory{row,scope,attempt,execution_attempt,paths}))
    })
}

/// Historical effect evidence only. Exact durable path intents plus a proven
/// output image can account for a killed writer before its SQL observation.
/// This never takes a lease, advances a request, changes a phase or writes cache.
pub(in super::super) fn resolution_written(
    service: &RepositoryService,
    id: RequestId,
) -> Result<bool, ReadError> {
    let Some(before) = resolution_history(service, id)? else {
        return Ok(false);
    };
    let missing = || ReadError::new(ResultCode::RecoveryRequired);
    let repo = service.resolve_repository(std::path::Path::new(&before.scope))?;
    let facts = service.read_session(RepositoryOperation::Read, |db| {
        super::observe::facts(db, &repo, before.row.kind, &before.row.item_id)
    })?;
    if facts.worktree != before.row.worktree || facts.branch != before.row.context_ref {
        return Err(missing());
    }
    let git = git2::Repository::open(&before.row.worktree)
        .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
    let (entries, _) =
        conflict::enumerate(&repo, &git, &before.row).map_err(|f| ReadError::new(f.code))?;
    if entries.len() != before.paths.len() {
        return Err(missing());
    }
    let mut proven = Vec::new();
    for (i, (entry, (ordinal, pre, post, _))) in entries.iter().zip(&before.paths).enumerate() {
        if *ordinal != 100 + i as i64 || pre.len() != 32 || post.len() != 32 {
            return Err(missing());
        }
        let path =
            std::path::PathBuf::from(std::str::from_utf8(&entry.path).map_err(|_| missing())?);
        if pre != post
            && let Some(image) = super::file_image::read(&before.row.worktree, &path)
                .map_err(|e| ReadError::new(run::Fault::from(e).code))?
            && image.mode.is_none_or(|mode| mode == 0o100644)
            && blake3::hash(&image.bytes).as_bytes().as_slice() == post
        {
            proven.push((path, post));
        }
    }
    let Some(mut after) = resolution_history(service, id)? else {
        return Err(missing());
    };
    after.row.owner_epoch = before.row.owner_epoch;
    if after != before {
        return Err(ReadError::new(ResultCode::ExternalChange));
    }
    for (path, post) in proven {
        if let Some(image) = super::file_image::read(&before.row.worktree, &path)
            .map_err(|e| ReadError::new(run::Fault::from(e).code))?
            && image.mode.is_none_or(|mode| mode == 0o100644)
            && blake3::hash(&image.bytes).as_bytes().as_slice() == post
        {
            return Ok(true);
        }
    }
    Ok(false)
}
